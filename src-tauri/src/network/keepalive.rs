//! SSH keepalive + ConnectStrategy — Bölüm 38.1 + 38.4.
//!
//! `SshKeepalive` russh client::Config'a geçilen iki parametreyi tek tipte
//! toplar; ileride per-profile override için settings.json'da serialize edilir.
//!
//! `ConnectStrategy` aynı host'a N paralel bağlantı isteğini staggered hale
//! getirir — kurumsal firewall'lar SYN flood / Fail2Ban tetikleyicilerini
//! aşmasın diye 250-500ms jitter ile sırayla açılır.

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeepalive {
    /// SSH application-level keepalive ping aralığı (saniye). russh
    /// `client::Config::keepalive_interval`.
    pub server_alive_interval_secs: u64,
    /// Bu kadar ping ardışık yanıtsız kalırsa bağlantı drop. russh
    /// `client::Config::keepalive_max`.
    pub server_alive_count_max: u8,
    /// Transport-level `SO_KEEPALIVE` aktif mi.
    pub tcp_keepalive: bool,
    /// TCP keepalive idle (saniye) — soket sessizken bu kadar sonra
    /// kernel probe başlar.
    pub tcp_keepalive_idle_secs: u64,
}

impl Default for SshKeepalive {
    fn default() -> Self {
        Self {
            server_alive_interval_secs: 30,
            server_alive_count_max: 3,
            tcp_keepalive: true,
            tcp_keepalive_idle_secs: 60,
        }
    }
}

impl SshKeepalive {
    pub fn server_alive_interval(&self) -> Duration {
        Duration::from_secs(self.server_alive_interval_secs)
    }

    pub fn tcp_keepalive_idle(&self) -> Duration {
        Duration::from_secs(self.tcp_keepalive_idle_secs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectStrategy {
    /// İlk bağlantı sonrası kaç ms bekle, sonraki açılır.
    pub stagger_initial_ms: u64,
    /// Sonraki her bağlantı için jitter aralığı [min, max] ms.
    pub stagger_jitter_min_ms: u64,
    pub stagger_jitter_max_ms: u64,
    /// `true` ise bir önceki bağlantı auth tamamlayana kadar bir sonrakini
    /// açma. Spec 38.4 default.
    pub wait_for_auth: bool,
    /// Aggressive override (intranet/güvenli ortam) — staggering kapalı.
    pub aggressive: bool,
}

impl Default for ConnectStrategy {
    fn default() -> Self {
        Self {
            stagger_initial_ms: 250,
            stagger_jitter_min_ms: 250,
            stagger_jitter_max_ms: 500,
            wait_for_auth: true,
            aggressive: false,
        }
    }
}

impl ConnectStrategy {
    /// `[min, max]` range deterministik test edilebilsin diye `unit` 0..1
    /// arası bir oran kabul eder. Production'da caller `fastrand::f64()` veya
    /// `rand::random::<f64>()` geçer.
    pub fn next_delay_ms(&self, unit: f64) -> u64 {
        if self.aggressive {
            return 0;
        }
        let min = self.stagger_jitter_min_ms;
        let max = self.stagger_jitter_max_ms.max(min);
        let span = (max - min) as f64;
        min + (span * unit.clamp(0.0, 1.0)) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keepalive_defaults_match_spec() {
        let k = SshKeepalive::default();
        assert_eq!(k.server_alive_interval_secs, 30);
        assert_eq!(k.server_alive_count_max, 3);
        assert!(k.tcp_keepalive);
        assert_eq!(k.tcp_keepalive_idle_secs, 60);
    }

    #[test]
    fn keepalive_helper_durations() {
        let k = SshKeepalive::default();
        assert_eq!(k.server_alive_interval(), Duration::from_secs(30));
        assert_eq!(k.tcp_keepalive_idle(), Duration::from_secs(60));
    }

    #[test]
    fn connect_strategy_stays_in_jitter_band() {
        let s = ConnectStrategy::default();
        assert_eq!(s.next_delay_ms(0.0), 250);
        assert_eq!(s.next_delay_ms(1.0), 500);
        let mid = s.next_delay_ms(0.5);
        assert!((250..=500).contains(&mid));
    }

    #[test]
    fn aggressive_mode_disables_stagger() {
        let s = ConnectStrategy {
            aggressive: true,
            ..Default::default()
        };
        assert_eq!(s.next_delay_ms(0.5), 0);
    }

    #[test]
    fn jitter_clamps_out_of_range_unit() {
        let s = ConnectStrategy::default();
        assert_eq!(s.next_delay_ms(-1.0), 250);
        assert_eq!(s.next_delay_ms(99.0), 500);
    }
}
