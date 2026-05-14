//! Log retention + crash-loop dedup — Bölüm 30.5.
//!
//! İki ayrı problem:
//!
//! 1. **Disk birikimi:** Bus event'leri tracing'e gider, diagnostics writer
//!    diske yazar; rotation policy olmadan `AppData` GB'larca log biriktirir.
//!    `RotationPolicy` rotation eşiklerini taşır.
//! 2. **Crash-loop spam:** Aynı bug 100 kez aynı stack trace üretirse log
//!    dosyası şişer, bilgi artmaz. `CrashLoopDedup` `xxh64(stack)` ile aynı
//!    panic'i tek `CrashSummary` satırına özetler.
//!
//! Faz 5 hedefi: rotation policy'i `DiagnosticsBuffer`'a wire et + crash
//! handler kayıtlarını dedup'a yönlendir. Şu an tipler + saf logic var,
//! cron veya file I/O henüz bağlı değil.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Diagnostics dosya rotasyon eşikleri. Hepsi inclusive — değer aşıldığında
/// yeni dosya açılır veya eski silinir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationPolicy {
    /// Tek dosya max boyutu (byte). Aşıldığında yeni dosya açılır.
    pub max_file_size_bytes: u64,
    /// Toplam dosya sayısı. Aşıldığında en eski silinir.
    pub max_file_count: u32,
    /// Tek dosya max yaşı (gün). Eskiyse silinir.
    pub max_file_age_days: u32,
    /// Toplam dizin cap'i (byte). Hard limit — aşılınca en eskiler %90'a
    /// inene kadar silinir (30.5 enforce_total_cap).
    pub total_cap_bytes: u64,
}

impl Default for RotationPolicy {
    /// Spec 30.5 default'ları: 50MB/file, 10 file, 30 gün, 1GB cap.
    fn default() -> Self {
        Self {
            max_file_size_bytes: 50 * 1024 * 1024,
            max_file_count: 10,
            max_file_age_days: 30,
            total_cap_bytes: 1024 * 1024 * 1024,
        }
    }
}

/// Aynı stack trace'in tekrar tekrar düşmesini özetleyen ring buffer.
///
/// Bölüm 30.5 spec'inde algoritma: panic handler `record_crash(stack, ctx)`
/// çağırır; struct hash'e göre count'u arttırır. Diagnostics writer her N
/// saniyede bir `flush_to_log()` ile özet satırları üretir — orijinal stack
/// **tek satır** olarak korunur, count + zaman aralığı eklenir.
#[derive(Debug, Default)]
pub struct CrashLoopDedup {
    seen: HashMap<u64, CrashInstance>,
}

#[derive(Debug, Clone)]
pub struct CrashInstance {
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub count: u32,
    pub stack_trace: String,
    pub context: String,
}

/// `flush_to_log()` çıktısı — diagnostics writer NDJSON satırı olarak basar.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashSummaryEvent {
    pub stack_trace: String,
    pub count: u32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub last_context: String,
}

impl CrashLoopDedup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Yeni bir panic kaydet. Aynı stack daha önce görüldüyse count++, son
    /// görülme zamanı ve context güncellenir.
    pub fn record_crash(&mut self, stack_trace: &str, context: &str) {
        let hash = fnv1a_64(stack_trace.as_bytes());
        let now = Utc::now();
        self.seen
            .entry(hash)
            .and_modify(|inst| {
                inst.count += 1;
                inst.last_seen = now;
                inst.context = context.to_string();
            })
            .or_insert_with(|| CrashInstance {
                first_seen: now,
                last_seen: now,
                count: 1,
                stack_trace: stack_trace.to_string(),
                context: context.to_string(),
            });
    }

    /// İçerikleri özet satırlarına dök ve buffer'ı temizle.
    pub fn flush_to_log(&mut self) -> Vec<CrashSummaryEvent> {
        self.seen
            .drain()
            .map(|(_, inst)| CrashSummaryEvent {
                stack_trace: inst.stack_trace,
                count: inst.count,
                first_seen: inst.first_seen,
                last_seen: inst.last_seen,
                last_context: inst.context,
            })
            .collect()
    }

    pub fn distinct_stack_count(&self) -> usize {
        self.seen.len()
    }
}

/// FNV-1a 64-bit. `xxhash` kalitesine ihtiyaç yok — stack string'leri tipik
/// olarak yüzlerce byte, collision prob çoğu cep yarımkürede 1/2^32+; üretim
/// koşulu için yeterli, dep eklemeden gelir.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_defaults_match_spec() {
        let p = RotationPolicy::default();
        assert_eq!(p.max_file_size_bytes, 50 * 1024 * 1024);
        assert_eq!(p.max_file_count, 10);
        assert_eq!(p.total_cap_bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn dedup_collapses_repeated_stack() {
        let mut d = CrashLoopDedup::new();
        for _ in 0..47 {
            d.record_crash("thread 'main' panicked at src/foo.rs:12", "ctx-a");
        }
        d.record_crash("thread 'main' panicked at src/bar.rs:3", "ctx-b");

        assert_eq!(d.distinct_stack_count(), 2);
        let mut events = d.flush_to_log();
        events.sort_by_key(|e| e.count);
        assert_eq!(events[0].count, 1);
        assert_eq!(events[1].count, 47);
        assert_eq!(events[1].last_context, "ctx-a");
        assert_eq!(d.distinct_stack_count(), 0, "flush boşaltmalı");
    }

    #[test]
    fn dedup_keeps_last_context() {
        let mut d = CrashLoopDedup::new();
        d.record_crash("stack-x", "first");
        d.record_crash("stack-x", "second");
        let events = d.flush_to_log();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].count, 2);
        assert_eq!(events[0].last_context, "second");
    }

    #[test]
    fn fnv1a_distinct_for_distinct_inputs() {
        assert_ne!(fnv1a_64(b"abc"), fnv1a_64(b"abd"));
        assert_eq!(fnv1a_64(b"hello"), fnv1a_64(b"hello"));
    }
}
