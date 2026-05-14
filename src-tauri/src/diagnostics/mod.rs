//! Diagnostics & Resource Limits — Bölüm 30.
//!
//! İki ana sorumluluk:
//!
//! 1. **[`RuntimeMetrics`]** (30.1) — observability snapshot'ı. Engine her 5 sn'de
//!    bir günceller; UI "About → Runtime Metrics" panelinde ve diagnostics
//!    bundle export'unda kullanılır.
//! 2. **[`RuntimeLimits`] + [`LimitProfile`]** (30.2-30.4) — kaynak sınırları.
//!    8GB laptop ile 128GB workstation aynı default'la çalışmasın diye
//!    `LimitProfile::detect()` startup'ta sistem profilini belirler, ondan
//!    türeyen `RuntimeLimits` scheduler/connection pool tarafından okunur.
//!
//! ## Memory probe
//!
//! Faz 5 sonu hedefi: cross-platform RSS okuma + soft/hard cap enforcement.
//! Şu an `LimitProfile::detect()` Linux'ta `/proc/meminfo`'dan toplam RAM'i
//! okuyabiliyor; Windows fallback'i Desktop. Bu **bilinçli minimum**: dep
//! ekleme maliyetinden kaçındık, gerekirse `sysinfo` crate'i veya doğrudan
//! `windows-sys::Win32_System_SystemInformation::GlobalMemoryStatusEx`
//! ileride bağlanır.

pub mod limits;
pub mod metrics;
pub mod retention;

pub use limits::{LimitProfile, RuntimeLimits};
pub use metrics::RuntimeMetrics;
pub use retention::{CrashLoopDedup, RotationPolicy};
