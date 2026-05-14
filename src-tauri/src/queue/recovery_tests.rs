//! Recovery integration tests — DB'ye orphan kayıtlar yaz, `run_recovery`
//! çağır, sonuçları doğrula. Tests aktarıldıkları için ayrı dosyada
//! (recovery.rs minimal kalsın, test runner DbActor `recover()` üzerinden de
//! gider).

use std::path::PathBuf;

use chrono::Utc;
use tempfile::tempdir;
use uuid::Uuid;

use super::{spawn_db_actor, PersistedTransferTask};
use crate::engine::TransferDirection;
use crate::events::TransferState;

fn task(state: TransferState) -> PersistedTransferTask {
    PersistedTransferTask {
        id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        direction: TransferDirection::Upload,
        state,
        priority: 0,
        local_path: PathBuf::from("/tmp/x"),
        remote_path: "/remote/x".into(),
        bytes_total: 100,
        bytes_done: 0,
        chunk_size: 64 * 1024,
        retry_count: 0,
        last_error: None,
        schema_version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: None,
        completed_at: None,
    }
}

#[tokio::test]
async fn recovery_resurrects_active_and_abandons_verifying() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("queue.db");
    let handle = spawn_db_actor(&db_path).unwrap();

    // Hazırlık: 3 task farklı state'lerde.
    // Recovery şeması: insert direkt state ile yapıyoruz çünkü state machine
    // dışından oluyor (test fixture). Real path'te scheduler insert sırasında
    // Queued kullanır, sonra geçişler validate edilir.
    handle.insert(task(TransferState::Queued)).await.unwrap();
    handle.insert(task(TransferState::Active)).await.unwrap();
    handle.insert(task(TransferState::Verifying)).await.unwrap();

    let report = handle.recover().await.unwrap();
    assert_eq!(report.resurrected_count, 1, "1 Active resurrected to Queued");
    assert_eq!(report.abandoned_count, 1, "1 Verifying abandoned to Failed");
    assert_eq!(report.orphan_tmp_files, 0, "Faz 3 tmp cleanup pasif");

    let queued = handle.list_by_state(TransferState::Queued).await.unwrap();
    assert_eq!(queued.len(), 2, "original Queued + resurrected Active");

    let failed = handle.list_by_state(TransferState::Failed).await.unwrap();
    assert_eq!(failed.len(), 1);
    assert!(
        failed[0].last_error.as_deref().unwrap_or("").contains("abandoned"),
        "abandoned reason recorded"
    );

    let active = handle.list_by_state(TransferState::Active).await.unwrap();
    assert!(active.is_empty(), "no Active should remain after recovery");
}

#[tokio::test]
async fn recovery_finalizing_also_abandoned() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("queue.db");
    let handle = spawn_db_actor(&db_path).unwrap();

    handle.insert(task(TransferState::Finalizing)).await.unwrap();
    handle.insert(task(TransferState::Verifying)).await.unwrap();

    let report = handle.recover().await.unwrap();
    assert_eq!(report.abandoned_count, 2);

    let failed = handle.list_by_state(TransferState::Failed).await.unwrap();
    assert_eq!(failed.len(), 2);
}

#[tokio::test]
async fn recovery_on_empty_db_is_noop() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("queue.db");
    let handle = spawn_db_actor(&db_path).unwrap();

    let report = handle.recover().await.unwrap();
    assert_eq!(report.resurrected_count, 0);
    assert_eq!(report.abandoned_count, 0);
}

#[tokio::test]
async fn recovery_leaves_terminal_states_untouched() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("queue.db");
    let handle = spawn_db_actor(&db_path).unwrap();

    handle.insert(task(TransferState::Completed)).await.unwrap();
    handle.insert(task(TransferState::Cancelled)).await.unwrap();
    handle.insert(task(TransferState::Skipped)).await.unwrap();
    handle.insert(task(TransferState::Failed)).await.unwrap();
    handle.insert(task(TransferState::Paused)).await.unwrap();

    let report = handle.recover().await.unwrap();
    assert_eq!(report.resurrected_count, 0);
    assert_eq!(report.abandoned_count, 0);

    // Terminal/paused state'ler korunmuş olmalı
    assert_eq!(
        handle.list_by_state(TransferState::Completed).await.unwrap().len(),
        1
    );
    assert_eq!(
        handle.list_by_state(TransferState::Paused).await.unwrap().len(),
        1
    );
}
