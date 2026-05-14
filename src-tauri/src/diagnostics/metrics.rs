//! RuntimeMetrics snapshot — Bölüm 30.1.
//!
//! Engine periyodik olarak (5sn) `RuntimeMetrics::snapshot()` üretir; ring
//! buffer'da son 1 saatlik veri tutulur. UI "About → Runtime Metrics" paneli
//! ve diagnostics bundle export'u bunu okur.
//!
//! Faz 5 öncesi tüm field'lar yer tutucu — gerçek tokio runtime probe'ları
//! (`tokio::runtime::Handle::current().metrics()`) ileride bağlanır. Şu an
//! struct + Default + serializer hazır olsun ki UI tip-güvenli build edilsin.

use serde::Serialize;

/// Engine'in dış dünyaya gösterdiği tek snapshot. Spec 30.1 field
/// sıralamasıyla bire bir tutuldu; UI grafik ekleyince bu sıralamaya göre
/// sütun seçecek.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMetrics {
    // ---- Transfer ----
    pub active_transfers: u32,
    pub queued_transfers: u32,
    pub completed_last_hour: u32,
    pub failed_last_hour: u32,
    pub inflight_bytes: u64,
    pub bytes_transferred_total: u64,

    // ---- Per-connection ----
    pub avg_chunk_latency_ms: u64,
    pub p50_chunk_latency_ms: u64,
    pub p99_chunk_latency_ms: u64,
    pub active_connections: u32,
    pub connections_in_pool: u32,

    // ---- DB ----
    pub db_queue_depth: usize,
    pub db_avg_write_latency_ms: u64,
    pub db_size_mb: u64,
    pub db_wal_size_mb: u64,

    // ---- Tokio runtime ----
    pub tokio_workers_busy: u32,
    pub tokio_blocking_threads: u32,
    pub tokio_tasks_alive: u32,
    pub stall_events_24h: u32,

    // ---- System ----
    pub memory_rss_mb: u64,
    pub open_file_descriptors: u32,
    pub thread_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_zero() {
        let m = RuntimeMetrics::default();
        assert_eq!(m.active_transfers, 0);
        assert_eq!(m.memory_rss_mb, 0);
    }

    #[test]
    fn serializes_to_camel_case_json() {
        let m = RuntimeMetrics {
            active_transfers: 3,
            inflight_bytes: 1024,
            ..Default::default()
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"activeTransfers\":3"));
        assert!(json.contains("\"inflightBytes\":1024"));
    }
}
