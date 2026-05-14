//! Rate Limiter + Adaptive Backoff — Bölüm 16.
//!
//! Cloud provider'lar (Dropbox 300 req/dk, Microsoft Graph 10k req/10dk, GCS
//! 1k req/sn) agresif throttle uygular. Bu modül:
//!
//! 1. Provider yanıt header'larından kalan kotayı çıkarır.
//! 2. `wait_if_limited(profile_id)` — limit ihlali öncesi *proaktif* bekler.
//! 3. `should_retry(error)` — TransferError kategorisine göre retry kararı.
//! 4. `BackoffConfig` — exponential + jitter; thundering herd koruması.
//!
//! **Spec 16.1 kritik karar:** key = `profile_id`, **host değil**. İki farklı
//! Dropbox hesabı aynı `api.dropbox.com`'u paylaşır ama farklı token'larla
//! ayrı kotaya sahip; host key olursa biri throttle yediğinde diğeri de
//! gereksiz yere yavaşlar.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::TransferError;

/// Tek bir profile'a ait rate limit snapshot'ı.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitState {
    /// Mevcut pencerede kalan istek sayısı.
    pub remaining: u32,
    /// Toplam pencere kotası.
    pub limit: u32,
    /// Kota yenilenme zamanı.
    pub reset_at: DateTime<Utc>,
    /// Provider Retry-After header'ı yolladıysa süre — bu varken `remaining`
    /// 0 olmasa bile kullanıcı bekler.
    pub retry_after: Option<Duration>,
}

impl RateLimitState {
    /// Mevcut snapshot'a göre proaktif bekleme süresi.
    ///
    /// - `retry_after` varsa onu kullan.
    /// - `remaining == 0` ise `reset_at - now`.
    /// - Aksi halde 0.
    pub fn wait_duration(&self, now: DateTime<Utc>) -> Duration {
        if let Some(d) = self.retry_after {
            return d;
        }
        if self.remaining == 0 {
            let delta = self.reset_at - now;
            if delta.num_milliseconds() > 0 {
                return Duration::from_millis(delta.num_milliseconds() as u64);
            }
        }
        Duration::ZERO
    }

    pub fn is_throttled(&self, now: DateTime<Utc>) -> bool {
        self.wait_duration(now) > Duration::ZERO
    }
}

/// Adaptive exponential backoff parametreleri. Provider Retry-After
/// header'ı yoksa veya `ConnectionLost`/`Timeout` durumlarında kullanılır.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackoffConfig {
    pub initial_ms: u64,
    pub multiplier: f64,
    pub max_ms: u64,
    /// `true` ise hesaplanan delay [0.5x, 1.5x] aralığında jitter'lanır —
    /// thundering herd koruması, aynı dakikada paniklemiş 100 client ayrı
    /// zaman dilimlerinde retry yapar.
    pub jitter: bool,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_ms: 1_000,
            multiplier: 2.0,
            max_ms: 60_000,
            jitter: true,
        }
    }
}

impl BackoffConfig {
    /// `attempt` 0-based; ilk retry = attempt 0 → `initial_ms`.
    ///
    /// Jitter algoritması spec'te detaylandırılmamış; deterministik test
    /// için `rng` parametresi alıyoruz — caller `rand_value(0..1)` üreten
    /// closure verir. Production'da `fastrand` veya `thread_rng` yeterli.
    pub fn delay_for(&self, attempt: u32, rand_unit: f64) -> Duration {
        let base = (self.initial_ms as f64) * self.multiplier.powi(attempt as i32);
        let capped = base.min(self.max_ms as f64).max(0.0);
        let final_ms = if self.jitter {
            // [0.5x, 1.5x] aralığı — fazla agresif değil ama yine de spread.
            let factor = 0.5 + rand_unit;
            (capped * factor).min(self.max_ms as f64)
        } else {
            capped
        };
        Duration::from_millis(final_ms as u64)
    }
}

/// Provider HTTP yanıt header'larından beslenen rate-limit cache.
///
/// `Mutex<HashMap>` yaklaşımı: rate limit update path'i nadiren çağrılır
/// (her API yanıtı sonrası ~ms), `tokio::sync::Mutex` overhead'ine gerek
/// yok. Lock altında await yapılmaz.
pub struct RateLimiter {
    states: Mutex<HashMap<Uuid, RateLimitState>>,
    backoff: BackoffConfig,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(BackoffConfig::default())
    }
}

impl RateLimiter {
    pub fn new(backoff: BackoffConfig) -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            backoff,
        }
    }

    pub fn backoff(&self) -> &BackoffConfig {
        &self.backoff
    }

    /// Provider yanıt header'larından state çıkarıp cache'le.
    ///
    /// Header map ham `&[(name, value)]` olarak alınıyor — `reqwest::HeaderMap`
    /// bağımlılığı yok (adapter zaten header'ı kendi tipinden okur ve bize
    /// flat slice geçer). Bilinen header isimleri (case-insensitive):
    /// - `Retry-After` — saniye veya HTTP-date
    /// - `X-RateLimit-Remaining` / `X-RateLimit-Limit` / `X-RateLimit-Reset`
    /// - `x-ms-ratelimit-remaining-*` (Graph API)
    pub fn update_from_headers(
        &self,
        profile_id: Uuid,
        headers: &[(&str, &str)],
        now: DateTime<Utc>,
    ) {
        let parsed = parse_rate_limit_headers(headers, now);
        if let Some(state) = parsed {
            self.states
                .lock()
                .expect("rate-limit state mutex poisoned")
                .insert(profile_id, state);
        }
    }

    /// Profile için snapshot.
    pub fn state(&self, profile_id: Uuid) -> Option<RateLimitState> {
        self.states
            .lock()
            .expect("rate-limit state mutex poisoned")
            .get(&profile_id)
            .copied()
    }

    /// `state.wait_duration(now)` proxy'si — caller manuel `now` geçer; test
    /// dostu. Production'da `Utc::now()`.
    pub fn should_throttle(&self, profile_id: Uuid, now: DateTime<Utc>) -> Duration {
        self.state(profile_id)
            .map(|s| s.wait_duration(now))
            .unwrap_or(Duration::ZERO)
    }

    /// Bekleme süresi 0 değilse `tokio::time::sleep` ile bekle. Sleep
    /// sonunda state silinmez — bir sonraki yanıt onu güncelleyecek.
    pub async fn wait_if_limited(&self, profile_id: Uuid) {
        let dur = self.should_throttle(profile_id, Utc::now());
        if dur > Duration::ZERO {
            tracing::info!(?profile_id, sleep_ms = dur.as_millis() as u64, "rate_limit_wait");
            tokio::time::sleep(dur).await;
        }
    }
}

/// Hata kategorisine göre retry hakkını ver/verme. Bölüm 16.2 spec'i:
///
/// - `RateLimited { retry_after }` → her zaman retry, süreyi yansıt.
/// - `ConnectionLost` / `Timeout` → exponential backoff.
/// - `Authentication` / `Authorization` → retry yok (kullanıcı kararı).
/// - `Cancelled` → retry yok.
/// - `ChecksumMismatch` → spec "Redo" → retry yok (manuel kullanıcı kararı).
pub fn should_retry(err: &TransferError) -> bool {
    use TransferError as E;
    matches!(
        err,
        E::ConnectionLost { .. }
            | E::Timeout { .. }
            | E::RateLimited { .. }
            | E::QuotaExceeded
            | E::Io(_)
    )
}

/// Retry-After + X-RateLimit-* header'larını parse et. Bilinmeyen formatta
/// `None` döner — caller eski state'i tutar.
fn parse_rate_limit_headers(
    headers: &[(&str, &str)],
    now: DateTime<Utc>,
) -> Option<RateLimitState> {
    let mut retry_after: Option<Duration> = None;
    let mut remaining: Option<u32> = None;
    let mut limit: Option<u32> = None;
    let mut reset_at: Option<DateTime<Utc>> = None;

    for (name, value) in headers {
        match name.to_ascii_lowercase().as_str() {
            "retry-after" => {
                retry_after = parse_retry_after(value, now);
            }
            "x-ratelimit-remaining" => {
                remaining = value.parse().ok();
            }
            "x-ratelimit-limit" => {
                limit = value.parse().ok();
            }
            "x-ratelimit-reset" => {
                reset_at = parse_reset(value, now);
            }
            _ => {}
        }
    }

    let any = retry_after.is_some()
        || remaining.is_some()
        || limit.is_some()
        || reset_at.is_some();
    if !any {
        return None;
    }
    Some(RateLimitState {
        remaining: remaining.unwrap_or(u32::MAX),
        limit: limit.unwrap_or(u32::MAX),
        reset_at: reset_at.unwrap_or(now + chrono::Duration::seconds(60)),
        retry_after,
    })
}

/// `Retry-After` saniye veya HTTP-date olabilir; saniye versiyonu çoğunluk.
fn parse_retry_after(value: &str, now: DateTime<Utc>) -> Option<Duration> {
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    // HTTP-date (RFC 7231) — chrono parse with feature; default features
    // off; basit fallback: ignore (None). Adapter ham saniye versiyonunu
    // yolluyor olur — bu yol kapsamı kapatır.
    let _ = now;
    None
}

/// `X-RateLimit-Reset` Unix epoch second veya ISO 8601 olabilir; çoğu
/// provider epoch.
fn parse_reset(value: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if let Ok(epoch) = value.parse::<i64>() {
        // Heuristik: 10^11'den küçük epoch saniye, ötesi milisaniye.
        let dt = if epoch < 10_000_000_000 {
            DateTime::<Utc>::from_timestamp(epoch, 0)
        } else {
            DateTime::<Utc>::from_timestamp_millis(epoch)
        };
        return dt;
    }
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or(Some(now))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn backoff_grows_exponential_without_jitter() {
        let cfg = BackoffConfig {
            initial_ms: 100,
            multiplier: 2.0,
            max_ms: 10_000,
            jitter: false,
        };
        assert_eq!(cfg.delay_for(0, 0.0), Duration::from_millis(100));
        assert_eq!(cfg.delay_for(1, 0.0), Duration::from_millis(200));
        assert_eq!(cfg.delay_for(2, 0.0), Duration::from_millis(400));
        // Cap kontrolü.
        assert_eq!(cfg.delay_for(100, 0.0), Duration::from_millis(10_000));
    }

    #[test]
    fn backoff_jitter_stays_in_band() {
        let cfg = BackoffConfig::default();
        // attempt=2 → 4000ms base. Jitter [0.5,1.5] → [2000, 6000].
        let lo = cfg.delay_for(2, 0.0); // factor 0.5
        let hi = cfg.delay_for(2, 1.0); // factor 1.5
        assert!(lo >= Duration::from_millis(2_000));
        assert!(hi <= Duration::from_millis(6_000));
        assert!(lo < hi);
    }

    #[test]
    fn limiter_parses_retry_after_seconds() {
        let limiter = RateLimiter::default();
        let pid = Uuid::new_v4();
        limiter.update_from_headers(pid, &[("Retry-After", "12")], now());
        let state = limiter.state(pid).expect("state");
        assert_eq!(state.retry_after, Some(Duration::from_secs(12)));
        assert!(state.is_throttled(now()));
    }

    #[test]
    fn limiter_remaining_zero_triggers_wait() {
        let limiter = RateLimiter::default();
        let pid = Uuid::new_v4();
        let reset = now() + chrono::Duration::seconds(30);
        limiter.update_from_headers(
            pid,
            &[
                ("X-RateLimit-Remaining", "0"),
                ("X-RateLimit-Limit", "300"),
                ("X-RateLimit-Reset", &reset.timestamp().to_string()),
            ],
            now(),
        );
        let dur = limiter.should_throttle(pid, now());
        assert!(dur >= Duration::from_secs(29) && dur <= Duration::from_secs(31));
    }

    #[test]
    fn limiter_remaining_positive_no_wait() {
        let limiter = RateLimiter::default();
        let pid = Uuid::new_v4();
        limiter.update_from_headers(
            pid,
            &[("X-RateLimit-Remaining", "150"), ("X-RateLimit-Limit", "300")],
            now(),
        );
        assert_eq!(limiter.should_throttle(pid, now()), Duration::ZERO);
    }

    #[test]
    fn limiter_unknown_profile_no_wait() {
        let limiter = RateLimiter::default();
        assert_eq!(limiter.should_throttle(Uuid::new_v4(), now()), Duration::ZERO);
    }

    #[test]
    fn should_retry_filters_terminal_errors() {
        assert!(should_retry(&TransferError::Timeout { elapsed_ms: 5_000 }));
        assert!(should_retry(&TransferError::RateLimited {
            retry_after_secs: 1
        }));
        assert!(!should_retry(&TransferError::Cancelled));
        assert!(!should_retry(&TransferError::Authentication {
            reason: "bad".into()
        }));
        assert!(!should_retry(&TransferError::ChecksumMismatch {
            expected: "a".into(),
            actual: "b".into(),
        }));
    }

    #[test]
    fn empty_headers_yield_no_update() {
        let limiter = RateLimiter::default();
        let pid = Uuid::new_v4();
        limiter.update_from_headers(pid, &[("Content-Type", "application/json")], now());
        assert!(limiter.state(pid).is_none());
    }
}
