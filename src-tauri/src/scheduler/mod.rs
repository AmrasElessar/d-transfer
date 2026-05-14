//! `QueueScheduler` — Bölüm 15.3.
//!
//! DB'deki `Queued` task'leri sıralı dispatch eden background worker.
//!
//! ## Sorumlulukları
//!
//! 1. `Queued` task'leri eski-önce sırasıyla seçer (FIFO; öncelikli olanlar
//!    `priority DESC` ile öne alınır — Bölüm 15.2 index).
//! 2. `AdapterFactory` üzerinden profile_id'den adapter inşa eder.
//! 3. DB state machine'i ile `Queued → Active` geçişini commit eder.
//! 4. `TransferEngine.submit()` ile transferi başlatır, sonucu bekler.
//! 5. Final state'i (`Completed` / `Failed` / `Cancelled`) DB'ye yazar,
//!    `bytes_done`'ı günceller.
//! 6. IPC tarafından `submit()` ile gönderilmiş `oneshot` waiter'a sonucu iletir.
//!
//! ## Faz 3 kısıtları
//!
//! - **`max_concurrent = 1`** — paralel dispatch yok (sıralı çalışır). Faz 4'te
//!   semaphore + per-profile concurrency limit eklenir.
//! - **Priority tiebreak** DB index'inde var ama burada henüz farklı priority
//!   tiebreak stratejisi yok — FIFO sade ve doğru.
//! - **Retry yok** — `Failed` terminal. Retry policy (Bölüm 15) Faz 4'te.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::engine::{TransferEngine, TransferRequest};
use crate::errors::{TransferError, WireError};
use crate::events::TransferState;
use crate::profiles::AdapterFactory;
use crate::protocols::{LocalPath, RemotePath};
use crate::queue::{DbActorHandle, DbError, PersistedTransferTask};

/// Per-transfer outcome (scheduler tamamladığında IPC waiter'a gönderilir).
#[derive(Debug, Clone)]
pub struct TransferOutcome {
    pub transfer_id: Uuid,
    pub final_state: TransferState,
    pub bytes_transferred: u64,
    pub duration_ms: u64,
    pub avg_speed_bps: f64,
    pub error: Option<WireError>,
}

/// Caller'ın eline geçen scheduler handle'ı. `Clone` ucuz — paylaşılabilir.
#[derive(Clone)]
pub struct QueueScheduler {
    queue: Arc<DbActorHandle>,
    waiters: Arc<Mutex<HashMap<Uuid, oneshot::Sender<TransferOutcome>>>>,
    notify_tx: mpsc::Sender<()>,
}

/// Worker tarafı — `run()` ile loop'ta tüketilir.
pub struct QueueSchedulerWorker {
    queue: Arc<DbActorHandle>,
    engine: Arc<TransferEngine>,
    factory: Arc<dyn AdapterFactory>,
    waiters: Arc<Mutex<HashMap<Uuid, oneshot::Sender<TransferOutcome>>>>,
    notify_rx: mpsc::Receiver<()>,
    cancel: CancellationToken,
}

/// Scheduler'ı kur. Worker ayrı bir task'te `run()` ile spawn edilir.
pub fn new_scheduler(
    queue: Arc<DbActorHandle>,
    engine: Arc<TransferEngine>,
    factory: Arc<dyn AdapterFactory>,
    cancel: CancellationToken,
) -> (QueueScheduler, QueueSchedulerWorker) {
    let (notify_tx, notify_rx) = mpsc::channel::<()>(8);
    let waiters = Arc::new(Mutex::new(HashMap::new()));
    let scheduler = QueueScheduler {
        queue: Arc::clone(&queue),
        waiters: Arc::clone(&waiters),
        notify_tx,
    };
    let worker = QueueSchedulerWorker {
        queue,
        engine,
        factory,
        waiters,
        notify_rx,
        cancel,
    };
    (scheduler, worker)
}

impl QueueScheduler {
    /// Task'i kuyruğa ekle, scheduler'ı uyandır, completion için oneshot döner.
    pub async fn submit(
        &self,
        task: PersistedTransferTask,
    ) -> Result<oneshot::Receiver<TransferOutcome>, TransferError> {
        let id = task.id;
        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.waiters.lock().expect("waiters mutex poisoned");
            guard.insert(id, tx);
        }
        self.queue
            .insert(task)
            .await
            .map_err(db_err_to_transfer_err)?;
        // Best-effort wake — channel doluysa scheduler zaten 5sn poll'a düşer.
        let _ = self.notify_tx.try_send(());
        Ok(rx)
    }

    /// Manuel uyandırma (örn. eski abandoned task'leri restart için).
    pub fn notify(&self) {
        let _ = self.notify_tx.try_send(());
    }
}

impl QueueSchedulerWorker {
    pub async fn run(mut self) {
        let mut poll = interval(Duration::from_secs(5));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // İlk tick'i hemen yutuyoruz ki 5sn beklemeden ilk poll başlasın.
        poll.tick().await;

        tracing::info!("queue scheduler started");

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    tracing::info!("queue scheduler shutdown");
                    return;
                }
                _ = self.notify_rx.recv() => {}
                _ = poll.tick() => {}
            }

            // Kuyrukta ne varsa hepsini drain et (Faz 3'te sıralı, max_concurrent=1).
            loop {
                if self.cancel.is_cancelled() {
                    return;
                }
                let next = self.pick_next().await;
                let Some(task) = next else { break };
                if let Err(e) = self.dispatch(task).await {
                    tracing::error!(?e, "scheduler dispatch error");
                }
            }
        }
    }

    async fn pick_next(&self) -> Option<PersistedTransferTask> {
        match self.queue.list_by_state(TransferState::Queued).await {
            Ok(tasks) => tasks.into_iter().next(),
            Err(e) => {
                tracing::error!(?e, "scheduler list_by_state failed");
                None
            }
        }
    }

    async fn dispatch(&self, task: PersistedTransferTask) -> Result<(), TransferError> {
        let transfer_id = task.id;
        let direction = task.direction;
        let local_path = task.local_path.clone();
        let remote_path = task.remote_path.clone();

        let started = std::time::Instant::now();

        // Adapter inşası başarısız → task Failed, waiter'a hata gönder.
        let adapter = match self.factory.build(task.profile_id).await {
            Ok(a) => a,
            Err(err) => {
                let wire = WireError::from(&err);
                self.finalize(transfer_id, TransferState::Failed, 0, 0, 0.0, Some(wire))
                    .await;
                return Err(err);
            }
        };

        // Queued → Active commit. Hata olursa scheduler-level log; task hâlâ
        // Queued olduğu için bir sonraki tick'te yeniden denenir.
        if let Err(e) = self
            .queue
            .update_state(transfer_id, TransferState::Active)
            .await
        {
            tracing::error!(?e, ?transfer_id, "Queued→Active transition failed");
            return Err(db_err_to_transfer_err(e));
        }

        // task.id'yi engine'e geçirmek için TransferRequest::new() sonrası id
        // override ediyoruz (pub field — Faz 3 sade yaklaşım).
        let mut request = TransferRequest::new(
            direction,
            LocalPath::new(local_path),
            RemotePath::new(remote_path),
            adapter,
        );
        request.id = transfer_id;

        let handle = self.engine.submit(request);
        let result = handle.wait().await;
        let duration_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(transfer_result) => {
                let bytes = transfer_result.stats.bytes_transferred;
                let speed = transfer_result.stats.avg_speed_bps;
                if let Err(e) = self.queue.update_progress(transfer_id, bytes).await {
                    tracing::warn!(?e, ?transfer_id, "final update_progress failed");
                }
                self.finalize(
                    transfer_id,
                    TransferState::Completed,
                    bytes,
                    duration_ms,
                    speed,
                    None,
                )
                .await;
            }
            Err(TransferError::Cancelled) => {
                self.finalize(
                    transfer_id,
                    TransferState::Cancelled,
                    0,
                    duration_ms,
                    0.0,
                    None,
                )
                .await;
            }
            Err(err) => {
                let wire = WireError::from(&err);
                self.finalize(
                    transfer_id,
                    TransferState::Failed,
                    0,
                    duration_ms,
                    0.0,
                    Some(wire),
                )
                .await;
            }
        }

        Ok(())
    }

    async fn finalize(
        &self,
        transfer_id: Uuid,
        final_state: TransferState,
        bytes_transferred: u64,
        duration_ms: u64,
        avg_speed_bps: f64,
        error: Option<WireError>,
    ) {
        if let Err(e) = self.queue.update_state(transfer_id, final_state).await {
            tracing::error!(?e, ?transfer_id, ?final_state, "final state update failed");
        }

        let outcome = TransferOutcome {
            transfer_id,
            final_state,
            bytes_transferred,
            duration_ms,
            avg_speed_bps,
            error,
        };

        let waiter = {
            let mut guard = self.waiters.lock().expect("waiters mutex poisoned");
            guard.remove(&transfer_id)
        };
        if let Some(tx) = waiter {
            let _ = tx.send(outcome);
        }
    }
}

fn db_err_to_transfer_err(e: DbError) -> TransferError {
    TransferError::Protocol {
        message: format!("queue db error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancellation::AppCancellation;
    use crate::engine::TransferDirection;
    use crate::events::EventBus;
    use crate::profiles::LocalAdapterFactory;
    use crate::queue::spawn_db_actor;
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn build_task(profile_id: Uuid, source: &std::path::Path, dest: &str) -> PersistedTransferTask {
        let now = Utc::now();
        PersistedTransferTask {
            id: Uuid::new_v4(),
            profile_id,
            direction: TransferDirection::Upload,
            state: TransferState::Queued,
            priority: 0,
            local_path: source.to_path_buf(),
            remote_path: dest.to_string(),
            bytes_total: 0,
            bytes_done: 0,
            chunk_size: 8 * 1024 * 1024,
            retry_count: 0,
            last_error: None,
            schema_version: 1,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    #[tokio::test]
    async fn submit_dispatches_through_engine_and_updates_db() {
        // Setup stack.
        let db_dir = tempdir().unwrap();
        let db_path = db_dir.path().join("queue.db");
        let queue = Arc::new(spawn_db_actor(&db_path).unwrap());

        let bus = Arc::new(EventBus::new(64));
        let app_cancel = AppCancellation::new();
        let engine = Arc::new(TransferEngine::new(Arc::clone(&bus), app_cancel.clone()));

        let factory = Arc::new(LocalAdapterFactory::new());
        let factory_dyn: Arc<dyn AdapterFactory> = Arc::clone(&factory) as Arc<dyn AdapterFactory>;

        let cancel_token = app_cancel.token().clone();
        let (scheduler, worker) = new_scheduler(
            Arc::clone(&queue),
            Arc::clone(&engine),
            factory_dyn,
            cancel_token,
        );
        let worker_handle = tokio::spawn(worker.run());

        // Hazırla: profil + dosya.
        let root_dir = tempdir().unwrap();
        let profile_id = factory.register_local(root_dir.path().to_path_buf());

        let src_dir = tempdir().unwrap();
        let src = src_dir.path().join("data.bin");
        std::fs::write(&src, vec![1u8; 4096]).unwrap();

        let task = build_task(profile_id, &src, "out.bin");
        let task_id = task.id;
        let rx = scheduler.submit(task).await.expect("submit");

        let outcome = tokio::time::timeout(Duration::from_secs(10), rx)
            .await
            .expect("outcome timeout")
            .expect("outcome channel");

        assert_eq!(outcome.final_state, TransferState::Completed);
        assert_eq!(outcome.bytes_transferred, 4096);
        assert!(outcome.error.is_none());

        // DB doğrula: state=Completed, bytes_done=4096.
        let final_task = queue.get(task_id).await.unwrap().unwrap();
        assert_eq!(final_task.state, TransferState::Completed);
        assert_eq!(final_task.bytes_done, 4096);

        // Dosya gerçekten yazıldı.
        let dst = root_dir.path().join("out.bin");
        assert!(dst.exists());
        assert_eq!(std::fs::metadata(&dst).unwrap().len(), 4096);

        // Cleanup
        app_cancel.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(2), worker_handle).await;
    }

    #[tokio::test]
    async fn missing_profile_yields_failed_state() {
        let db_dir = tempdir().unwrap();
        let queue = Arc::new(spawn_db_actor(&db_dir.path().join("q.db")).unwrap());
        let bus = Arc::new(EventBus::new(64));
        let cancel = AppCancellation::new();
        let engine = Arc::new(TransferEngine::new(Arc::clone(&bus), cancel.clone()));
        let factory: Arc<dyn AdapterFactory> = Arc::new(LocalAdapterFactory::new());

        let (scheduler, worker) =
            new_scheduler(Arc::clone(&queue), engine, factory, cancel.token().clone());
        let worker_handle = tokio::spawn(worker.run());

        // Profile registry'ye eklenmeyen bir id ile task.
        let task = build_task(Uuid::new_v4(), &PathBuf::from("noop"), "dst.bin");
        let task_id = task.id;
        let rx = scheduler.submit(task).await.expect("submit");

        let outcome = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("outcome timeout")
            .expect("outcome channel");
        assert_eq!(outcome.final_state, TransferState::Failed);
        assert!(outcome.error.is_some());

        let final_task = queue.get(task_id).await.unwrap().unwrap();
        assert_eq!(final_task.state, TransferState::Failed);

        cancel.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(2), worker_handle).await;
    }
}
