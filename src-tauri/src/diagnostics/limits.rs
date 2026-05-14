//! RuntimeLimits + LimitProfile — Bölüm 30.2-30.4.
//!
//! `RuntimeLimits` davranışsal değil sayısal kontrat — scheduler / connection
//! pool / DbActor bu değerleri okuyup *backpressure* uygular. Limit ihlali
//! her zaman *pause veya queue*, asla *error değil* (30.2 davranış tablosu).
//!
//! `LimitProfile` adaptive heuristics — startup'ta `detect()` çağrılır, sonuç
//! `Custom(...)` ile UI üzerinden override edilebilir.

use serde::{Deserialize, Serialize};

/// Spec'in 30.2 tablosundaki değerlerin tip-güvenli karşılığı.
///
/// Field sıralaması spec'le bire bir; `Serialize` UI export'una uygun, `clone`
/// ucuz çünkü hepsi `Copy` skalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLimits {
    /// Toplam aynı anda açık dosya descriptor sınırı. POSIX'te `setrlimit`
    /// ile büyütülmeye çalışılır; başarısızsa OS ulimit'in %75'ine clamp.
    pub max_open_files: usize,
    /// Aynı anda inflight chunk sayısı (tüm transferler toplam).
    pub max_inflight_chunks: usize,
    /// Toplam inflight byte (memory pressure).
    pub max_inflight_bytes: u64,
    /// Aynı anda aktif transfer sayısı (scheduler concurrency).
    pub max_concurrent_transfers: u32,
    /// Aynı host'a açık eş zamanlı bağlantı (per-profile semaphore).
    pub max_connections_per_host: u32,
    /// DbActor mpsc backlog (dolulukta producer bekler).
    pub max_db_command_backlog: usize,
    /// Soft memory cap — bu değere ulaşınca yeni transfer kabul edilmez,
    /// mevcutler tamamlanmaya bırakılır.
    pub soft_memory_cap_mb: u64,
    /// Hard memory cap — graceful pause trigger, son çare.
    pub hard_memory_cap_mb: u64,
    /// Tokio blocking pool sınırı (30.4 — profile-aware sizing).
    pub max_blocking_threads: usize,
    /// S3 reqwest pool — Bölüm 11.5; idle connection cap per host.
    pub s3_pool_max_idle_per_host: u32,
}

/// Adaptive limit profile. `detect()` startup'ta sistem RAM/CPU/headless
/// durumuna bakıp ayırır; `Custom(...)` UI override için açık kapı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LimitProfile {
    /// 4GB RAM altı VPS / eski laptop / Raspberry Pi.
    LowMemory,
    /// 8-16GB RAM, tipik laptop / desktop (varsayılan).
    Desktop,
    /// 32GB+ RAM, developer/enterprise workstation.
    Workstation,
    /// 64GB+ RAM, headless / batch transfer server.
    Server,
    /// Kullanıcı manuel ayarladı.
    Custom { limits: RuntimeLimits },
}

impl LimitProfile {
    /// Sistem profilini probe et — RAM toplamı + headless durumu birincil
    /// belirleyici. Başarısız probe → `Desktop` (güvenli orta yol).
    pub fn detect() -> Self {
        let ram_gb = system_total_ram_mb().map(|mb| mb / 1024).unwrap_or(8);
        let headless = is_headless();
        match (ram_gb, headless) {
            (r, _) if r < 4 => Self::LowMemory,
            (r, true) if r >= 32 => Self::Server,
            (r, _) if r >= 32 => Self::Workstation,
            _ => Self::Desktop,
        }
    }

    /// Profil → somut limit tablosu. Spec 30.3'teki dört preset bire bir
    /// kodlandı; `Custom` doğrudan kullanıcı değerini geri verir.
    pub fn to_limits(&self) -> RuntimeLimits {
        match self {
            Self::LowMemory => RuntimeLimits {
                max_open_files: 512,
                max_inflight_chunks: 32,
                max_inflight_bytes: 64 * 1024 * 1024,
                max_concurrent_transfers: 4,
                max_connections_per_host: 2,
                max_db_command_backlog: 128,
                soft_memory_cap_mb: 150,
                hard_memory_cap_mb: 400,
                max_blocking_threads: 64,
                s3_pool_max_idle_per_host: 4,
            },
            Self::Desktop => RuntimeLimits {
                max_open_files: 4096,
                max_inflight_chunks: 256,
                max_inflight_bytes: 512 * 1024 * 1024,
                max_concurrent_transfers: 16,
                max_connections_per_host: 8,
                max_db_command_backlog: 1024,
                soft_memory_cap_mb: 600,
                hard_memory_cap_mb: 1500,
                max_blocking_threads: 256,
                s3_pool_max_idle_per_host: 16,
            },
            Self::Workstation => RuntimeLimits {
                max_open_files: 8192,
                max_inflight_chunks: 1024,
                max_inflight_bytes: 2 * 1024 * 1024 * 1024,
                max_concurrent_transfers: 64,
                max_connections_per_host: 16,
                max_db_command_backlog: 4096,
                soft_memory_cap_mb: 2400,
                hard_memory_cap_mb: 6000,
                max_blocking_threads: 512,
                s3_pool_max_idle_per_host: 32,
            },
            Self::Server => RuntimeLimits {
                max_open_files: 16384,
                max_inflight_chunks: 4096,
                max_inflight_bytes: 8 * 1024 * 1024 * 1024,
                max_concurrent_transfers: 256,
                max_connections_per_host: 32,
                max_db_command_backlog: 16384,
                soft_memory_cap_mb: 8000,
                hard_memory_cap_mb: 20000,
                max_blocking_threads: 1024,
                s3_pool_max_idle_per_host: 64,
            },
            Self::Custom { limits } => *limits,
        }
    }
}

/// `/proc/meminfo` (Linux) veya OS API (Windows fallback) üzerinden toplam
/// fiziksel RAM (MB). Hata = `None`; caller default kullanır.
fn system_total_ram_mb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let raw = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let kb: u64 = rest
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()?;
                return Some(kb / 1024);
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Windows / macOS: doğrudan probe için dep gerekiyor (sysinfo veya
        // windows-sys feature). Spec'teki fallback "auto-detect başarısızsa
        // Desktop" — None döner, caller `Desktop`'a düşer.
        None
    }
}

/// Headless ortam tespiti (Bölüm 30.3): Linux'ta DISPLAY ve WAYLAND_DISPLAY
/// yoksa headless; Windows'ta her zaman GUI varsayıyoruz (Server Core nadir
/// senaryo, ayrı bir telemetry alanı olur).
fn is_headless() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_profile_uses_512mb_inflight() {
        let l = LimitProfile::Desktop.to_limits();
        assert_eq!(l.max_inflight_bytes, 512 * 1024 * 1024);
        assert_eq!(l.max_concurrent_transfers, 16);
    }

    #[test]
    fn low_memory_profile_scales_down_blocking_pool() {
        let l = LimitProfile::LowMemory.to_limits();
        // 30.4: LowMemory blocking_threads 64.
        assert_eq!(l.max_blocking_threads, 64);
        assert!(l.soft_memory_cap_mb < l.hard_memory_cap_mb);
    }

    #[test]
    fn custom_profile_round_trips() {
        let original = RuntimeLimits {
            max_open_files: 999,
            max_inflight_chunks: 7,
            max_inflight_bytes: 1234,
            max_concurrent_transfers: 1,
            max_connections_per_host: 1,
            max_db_command_backlog: 8,
            soft_memory_cap_mb: 100,
            hard_memory_cap_mb: 200,
            max_blocking_threads: 16,
            s3_pool_max_idle_per_host: 2,
        };
        let p = LimitProfile::Custom { limits: original };
        assert_eq!(p.to_limits(), original);
    }

    #[test]
    fn detect_never_panics() {
        // Probe başarısız olsa bile bir profile dönmeli.
        let _ = LimitProfile::detect();
    }
}
