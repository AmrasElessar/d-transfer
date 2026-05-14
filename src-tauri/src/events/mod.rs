//! Unified EngineEvent Bus — Bölüm 33.
//!
//! Progress, queue state, adapter state, log — hepsi tek `EngineEvent` enum
//! üzerinden akar. Diagnostics bundle, UI ve internal tracing aynı bus'tan
//! beslenir.
//!
//! Event'ler `Arc<EngineEvent>` olarak yayınlanır (Bölüm 33 — v1.14 allocation
//! reduction): broadcast subscriber'larda refcount++ kullanılır, deep clone yok.
//!
//! ## İki kanal
//!
//! - **UI broadcast (lossy):** yavaş subscriber eski event'leri kaçırabilir.
//!   `tokio::sync::broadcast` overflow policy = drop-oldest.
//! - **Diagnostics mpsc (lossless):** event kaybı diagnostics doğruluğunu
//!   bozar; unbounded channel + disk writer.
//!
//! Asıl Per-event TransferError, TransferState gibi domain enum'lar Faz 2'de
//! eklenecek; bu modül yalnızca bus altyapısını barındırır.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::errors::WireError;

/// Engine'in dış dünyaya yayınladığı her olay bu enum üzerinden geçer.
///
/// `Clone` türetilmez — bus `Arc<EngineEvent>` üzerinden yayınlar (Bölüm 33
/// v1.14 allocation reduction). `TransferError` içindeki `std::io::Error` zaten
/// `Clone` olmadığından bunu zorlamak `Io` variant'ını wrapper'a çevirmemizi
/// gerektirirdi — yerine `Arc<…>` tercih.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EngineEvent {
    // ---- Transfer ----
    TransferProgress {
        transfer_id: Uuid,
        bytes_done: u64,
        bytes_total: u64,
        speed_bps: f64,
        eta_secs: Option<u64>,
    },
    TransferStateChanged {
        transfer_id: Uuid,
        old_state: TransferState,
        new_state: TransferState,
    },
    TransferCompleted {
        transfer_id: Uuid,
        checksum: String,
        duration_ms: u64,
    },
    TransferFailed {
        transfer_id: Uuid,
        error: WireError,
        retry_in_ms: Option<u64>,
    },

    // ---- Ağ / API ----
    RateLimited {
        profile_id: Uuid,
        retry_after_secs: u64,
    },
    ConnectionLost {
        profile_id: Uuid,
    },
    ConnectionRestored {
        profile_id: Uuid,
    },

    // ---- Queue ----
    QueueRecovered {
        restored_count: usize,
    },
    QueueDrained,

    // ---- Sistem ----
    AppShutdownInitiated,
    DiagnosticsFlushed {
        path: PathBuf,
    },
}

/// Domain TransferState — UI Pinia store'undaki tipin Rust eşi.
/// `String` yerine enum tutmak DbActor / serializer'ı sıkılaştırır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferState {
    Queued,
    Active,
    Verifying,
    Finalizing,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

pub struct EventBus {
    ui_tx: broadcast::Sender<Arc<EngineEvent>>,
    diagnostics_tx: mpsc::UnboundedSender<Arc<EngineEvent>>,
    // Diagnostics receiver burada saklanır; gerçek disk writer Faz 5'te
    // ResolverTask olarak buraya bağlanacak. Faz 1'de bus iskeletini sızdırmamak
    // için tutucu (option) tutulur.
    diagnostics_rx: Mutex<Option<mpsc::UnboundedReceiver<Arc<EngineEvent>>>>,
}

impl EventBus {
    pub fn new(broadcast_capacity: usize) -> Self {
        let (ui_tx, _ui_rx_drop) = broadcast::channel(broadcast_capacity);
        let (diagnostics_tx, diagnostics_rx) = mpsc::unbounded_channel();
        Self {
            ui_tx,
            diagnostics_tx,
            diagnostics_rx: Mutex::new(Some(diagnostics_rx)),
        }
    }

    pub fn emit(&self, event: EngineEvent) {
        let event = Arc::new(event);
        // Diagnostics — kaybı kabul edilemez, ama receiver henüz drain edilmiyor
        // olabilir (Faz 1). `send` hatasını yutuyoruz çünkü unbounded sender'ın
        // tek hata case'i receiver dropped — bu Faz 5'te disk writer bağlanınca
        // anlamlı hale gelir.
        let _ = self.diagnostics_tx.send(Arc::clone(&event));
        tracing::debug!(event = ?event, "engine_event");
        // UI broadcast — subscriber yoksa sessiz drop (normal).
        let _ = self.ui_tx.send(event);
    }

    pub fn subscribe_ui(&self) -> broadcast::Receiver<Arc<EngineEvent>> {
        self.ui_tx.subscribe()
    }

    /// Diagnostics tüketici handle'ı — yalnızca BIR kere alınabilir. Faz 5'te
    /// DiagnosticsWriter bunu sahiplenir; Faz 1'de boş bırakmak için option.
    pub fn take_diagnostics_receiver(
        &self,
    ) -> Option<mpsc::UnboundedReceiver<Arc<EngineEvent>>> {
        self.diagnostics_rx
            .lock()
            .expect("diagnostics_rx mutex poisoned")
            .take()
    }

    pub fn subscriber_count(&self) -> usize {
        self.ui_tx.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn emit_reaches_ui_subscriber() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe_ui();

        bus.emit(EngineEvent::QueueDrained);

        let received = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("event timed out")
            .expect("event channel closed");
        assert!(matches!(*received, EngineEvent::QueueDrained));
    }

    #[tokio::test]
    async fn diagnostics_receiver_takeable_once() {
        let bus = EventBus::new(16);
        assert!(bus.take_diagnostics_receiver().is_some());
        assert!(
            bus.take_diagnostics_receiver().is_none(),
            "second take must yield None"
        );
    }

    #[tokio::test]
    async fn arc_event_avoids_deep_clone() {
        let bus = EventBus::new(16);
        let mut a = bus.subscribe_ui();
        let mut b = bus.subscribe_ui();

        bus.emit(EngineEvent::QueueRecovered { restored_count: 7 });

        let ev_a = a.recv().await.unwrap();
        let ev_b = b.recv().await.unwrap();
        // Aynı Arc instance — Arc::ptr_eq doğrularsa deep clone yok demektir.
        assert!(Arc::ptr_eq(&ev_a, &ev_b));
    }
}
