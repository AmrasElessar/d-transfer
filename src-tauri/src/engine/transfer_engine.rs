//! `TransferEngine` — Faz 2 dispatch orchestrator.
//!
//! Sorumlulukları:
//! 1. `TransferRequest`'i alır, child `TransferCancellation` üretir.
//! 2. `ProgressAggregator`'ı spawn eder, adapter'a `ProgressSender` verir.
//! 3. Adapter `upload`/`download` çağrısını cancellation token'ı ile race eder.
//! 4. Lifecycle event'lerini (`TransferStateChanged`, `TransferProgress`,
//!    `TransferCompleted`, `TransferFailed`) `EventBus`'a yayınlar.
//! 5. Caller'a `TransferHandle` döner — cancel + wait için.
//!
//! Asıl Multipart/Retry/RateLimiter Faz 3+'ta bu modülün etrafına eklenecek;
//! şu an "adapter'ı çağır, sonucu yayınla" minimum sözleşmesi var.

use std::sync::Arc;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::cancellation::AppCancellation;
use crate::errors::{TransferError, WireError};
use crate::events::{EngineEvent, EventBus, TransferState};
use crate::protocols::types::ProgressTick;
use crate::protocols::{ProtocolAdapter, TransferResult};

use super::progress::ProgressAggregator;
use super::types::{TransferDirection, TransferHandle, TransferRequest};

pub struct TransferEngine {
    events: Arc<EventBus>,
    root_cancel: AppCancellation,
}

impl TransferEngine {
    pub fn new(events: Arc<EventBus>, root_cancel: AppCancellation) -> Self {
        Self {
            events,
            root_cancel,
        }
    }

    /// Yeni bir transfer dispatch et. Background task spawn edilir, çağırana
    /// `TransferHandle` döner.
    pub fn submit(&self, request: TransferRequest) -> TransferHandle {
        let transfer_id = request.id;
        // Faz 2'de profile_id == transfer_id; gerçek profile kavramı Faz 3'te
        // ConnectionProfile struct'ı ile gelir. Hiyerarşi root → transfer
        // şimdilik tek katlı.
        let profile_scope = self.root_cancel.child_profile(Uuid::nil());
        let cancellation = profile_scope.child_transfer(transfer_id);

        let (progress_tx, progress_rx) = mpsc::channel::<ProgressTick>(64);
        let events = Arc::clone(&self.events);

        // Aggregator paralel task — progress_tx kapandığında kendiliğinden biter.
        let aggregator_events = Arc::clone(&events);
        let aggregator_handle = tokio::spawn(async move {
            ProgressAggregator::run(transfer_id, progress_rx, aggregator_events).await;
        });

        let cancel_token = cancellation.token().clone();
        let join = tokio::spawn(async move {
            events.emit(EngineEvent::TransferStateChanged {
                transfer_id,
                old_state: TransferState::Queued,
                new_state: TransferState::Active,
            });

            let started = std::time::Instant::now();
            let result = tokio::select! {
                r = run_transfer(request, progress_tx) => r,
                _ = cancel_token.cancelled() => Err(TransferError::Cancelled),
            };

            // Aggregator'ın drain etmesini bekle — tx Drop oldu, rx None alacak.
            let _ = aggregator_handle.await;

            match &result {
                Ok(transfer_result) => {
                    events.emit(EngineEvent::TransferStateChanged {
                        transfer_id,
                        old_state: TransferState::Active,
                        new_state: TransferState::Completed,
                    });
                    events.emit(EngineEvent::TransferCompleted {
                        transfer_id,
                        checksum: transfer_result
                            .stats
                            .checksum
                            .clone()
                            .unwrap_or_default(),
                        duration_ms: started.elapsed().as_millis() as u64,
                    });
                }
                Err(TransferError::Cancelled) => {
                    events.emit(EngineEvent::TransferStateChanged {
                        transfer_id,
                        old_state: TransferState::Active,
                        new_state: TransferState::Cancelled,
                    });
                }
                Err(other) => {
                    events.emit(EngineEvent::TransferStateChanged {
                        transfer_id,
                        old_state: TransferState::Active,
                        new_state: TransferState::Failed,
                    });
                    events.emit(EngineEvent::TransferFailed {
                        transfer_id,
                        error: WireError::from(other),
                        retry_in_ms: None,
                    });
                }
            }

            result
        });

        TransferHandle::new(transfer_id, cancellation, join)
    }
}

async fn run_transfer(
    request: TransferRequest,
    progress_tx: mpsc::Sender<ProgressTick>,
) -> Result<TransferResult, TransferError> {
    let adapter: &dyn ProtocolAdapter = &*request.adapter;
    match request.direction {
        TransferDirection::Upload => {
            adapter
                .upload(&request.local, &request.remote, &request.options, progress_tx)
                .await
        }
        TransferDirection::Download => {
            adapter
                .download(&request.remote, &request.local, &request.options, progress_tx)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EngineEvent;
    use crate::protocols::{LocalAdapter, LocalPath, RemotePath};
    use serde_json::json;
    use std::path::Path;
    use tempfile::tempdir;
    use tokio::sync::broadcast;

    async fn drain_events(
        rx: &mut broadcast::Receiver<std::sync::Arc<EngineEvent>>,
    ) -> Vec<std::sync::Arc<EngineEvent>> {
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            out.push(ev);
        }
        out
    }

    async fn make_adapter(root: &Path) -> Arc<dyn ProtocolAdapter> {
        let mut adapter = LocalAdapter::new();
        adapter
            .connect(&json!({ "root": root.to_str().unwrap() }))
            .await
            .unwrap();
        Arc::new(adapter)
    }

    #[tokio::test]
    async fn upload_completes_with_lifecycle_events() {
        let bus = Arc::new(EventBus::new(128));
        let mut events_rx = bus.subscribe_ui();
        let engine = TransferEngine::new(Arc::clone(&bus), AppCancellation::new());

        let root = tempdir().unwrap();
        let src_dir = tempdir().unwrap();
        let src_file = src_dir.path().join("src.bin");
        std::fs::write(&src_file, vec![7u8; 8192]).unwrap();

        let adapter = make_adapter(root.path()).await;
        let request = TransferRequest::new(
            TransferDirection::Upload,
            LocalPath::new(&src_file),
            RemotePath::new("dst.bin"),
            adapter,
        );
        let id = request.id;

        let handle = engine.submit(request);
        let result = handle.wait().await.unwrap();
        assert_eq!(result.stats.bytes_transferred, 8192);

        let written = std::fs::read(root.path().join("dst.bin")).unwrap();
        assert_eq!(written.len(), 8192);

        let events = drain_events(&mut events_rx).await;
        assert!(
            events.iter().any(|e| matches!(
                **e,
                EngineEvent::TransferStateChanged {
                    transfer_id, new_state: TransferState::Active, ..
                } if transfer_id == id
            )),
            "expected Queued→Active transition"
        );
        assert!(
            events.iter().any(|e| matches!(
                **e,
                EngineEvent::TransferCompleted { transfer_id, .. } if transfer_id == id
            )),
            "expected TransferCompleted"
        );
    }

    #[tokio::test]
    async fn cancel_marks_transfer_cancelled() {
        let bus = Arc::new(EventBus::new(128));
        let mut events_rx = bus.subscribe_ui();
        let engine = TransferEngine::new(Arc::clone(&bus), AppCancellation::new());

        let root = tempdir().unwrap();
        let src_dir = tempdir().unwrap();
        let src_file = src_dir.path().join("big.bin");
        // 64 MiB — birden fazla chunk olacak kadar büyük, cancel race penceresi açar.
        std::fs::write(&src_file, vec![0xCDu8; 64 * 1024 * 1024]).unwrap();

        let adapter = make_adapter(root.path()).await;
        let request = TransferRequest::new(
            TransferDirection::Upload,
            LocalPath::new(&src_file),
            RemotePath::new("big-copy.bin"),
            adapter,
        );
        let id = request.id;

        let handle = engine.submit(request);
        handle.cancel();
        let result = handle.wait().await;
        assert!(matches!(result, Err(TransferError::Cancelled)));

        let events = drain_events(&mut events_rx).await;
        assert!(
            events.iter().any(|e| matches!(
                **e,
                EngineEvent::TransferStateChanged {
                    transfer_id, new_state: TransferState::Cancelled, ..
                } if transfer_id == id
            )),
            "expected Active→Cancelled transition"
        );
    }
}
