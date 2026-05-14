//! DbActor — yazma serileştirmesi (Bölüm 15.4).
//!
//! Tüm transfer state mutasyonları tek bir owned `Connection` üzerinden,
//! tek bir blocking thread içinde, sıralı işlenir. 16+ paralel transfer
//! worker'ı `mpsc::Sender<DbCommand>` klonu üzerinden command yollar; actor
//! `recv` sırasına göre işler. Sonuç: SQLITE_BUSY pratikte sıfır, lock
//! contention sıfır, backpressure built-in (channel 1024).
//!
//! Read-only sorgular da actor'dan geçer — sıralılığı garantilemek ve test
//! API yüzeyini sade tutmak için. WAL reader/writer ayrımı v2'de gerekirse
//! eklenecek (Bölüm 15.4 trade-off notu).
//!
//! Plain rusqlite + `spawn_blocking` (tokio-rusqlite yerine): bağımlılık
//! yüzeyini daraltır, connection ownership'i açıkça actor thread'inde tutar,
//! WAL pragma'larını her bağlantı için tek noktada uygulayabiliriz.

use std::path::Path;

use rusqlite::{params, Connection};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::events::TransferState;
use crate::profiles::ConnectionProfile;

use super::recovery::{run_recovery, RecoveryReport};
use super::schema::{apply_migrations, configure_connection};
use super::state_machine::can_transition_to;
use super::task::{
    direction_as_str, parse_state, state_as_str, PersistedTransferTask,
};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid state transition: {from:?} → {to:?}")]
    InvalidTransition {
        from: TransferState,
        to: TransferState,
    },
    #[error("task not found: {0}")]
    NotFound(Uuid),
    #[error("actor channel closed")]
    ActorClosed,
}

/// `mpsc` üzerinden actor'a yollanan komutlar.
///
/// Her komut `oneshot` ack ile döner — fire-and-forget yok. (Spec 15.4'te
/// `BatchProgress` fire-and-forget; Faz 3 scope'unda batched progress yok,
/// her progress update ack'le döner. Batched flush Faz 4'te eklenir.)
pub enum DbCommand {
    Insert {
        task: Box<PersistedTransferTask>,
        ack: oneshot::Sender<Result<(), DbError>>,
    },
    UpdateState {
        id: Uuid,
        new_state: TransferState,
        ack: oneshot::Sender<Result<(), DbError>>,
    },
    UpdateProgress {
        id: Uuid,
        bytes_done: u64,
        ack: oneshot::Sender<Result<(), DbError>>,
    },
    Get {
        id: Uuid,
        ack: oneshot::Sender<Result<Option<PersistedTransferTask>, DbError>>,
    },
    ListByState {
        state: TransferState,
        ack: oneshot::Sender<Result<Vec<PersistedTransferTask>, DbError>>,
    },
    Recover {
        ack: oneshot::Sender<Result<RecoveryReport, DbError>>,
    },
    // ---------- ConnectionProfile CRUD (Bölüm 25) ----------
    ProfileInsert {
        profile: Box<ConnectionProfile>,
        ack: oneshot::Sender<Result<(), DbError>>,
    },
    ProfileGet {
        id: Uuid,
        ack: oneshot::Sender<Result<Option<ConnectionProfile>, DbError>>,
    },
    ProfileList {
        ack: oneshot::Sender<Result<Vec<ConnectionProfile>, DbError>>,
    },
    ProfileUpdate {
        profile: Box<ConnectionProfile>,
        ack: oneshot::Sender<Result<(), DbError>>,
    },
    ProfileDelete {
        id: Uuid,
        ack: oneshot::Sender<Result<(), DbError>>,
    },
    Shutdown,
}

/// Actor handle'ı. `Clone` ucuz (mpsc Sender internal Arc).
#[derive(Clone)]
pub struct DbActorHandle {
    tx: mpsc::Sender<DbCommand>,
}

impl DbActorHandle {
    pub async fn insert(&self, task: PersistedTransferTask) -> Result<(), DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::Insert {
                task: Box::new(task),
                ack: ack_tx,
            })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn update_state(
        &self,
        id: Uuid,
        new_state: TransferState,
    ) -> Result<(), DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::UpdateState {
                id,
                new_state,
                ack: ack_tx,
            })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn update_progress(&self, id: Uuid, bytes_done: u64) -> Result<(), DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::UpdateProgress {
                id,
                bytes_done,
                ack: ack_tx,
            })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<PersistedTransferTask>, DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::Get { id, ack: ack_tx })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn list_by_state(
        &self,
        state: TransferState,
    ) -> Result<Vec<PersistedTransferTask>, DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::ListByState {
                state,
                ack: ack_tx,
            })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn recover(&self) -> Result<RecoveryReport, DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::Recover { ack: ack_tx })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    // ---------- ConnectionProfile CRUD ----------

    pub async fn profile_insert(&self, profile: ConnectionProfile) -> Result<(), DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::ProfileInsert {
                profile: Box::new(profile),
                ack: ack_tx,
            })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn profile_get(
        &self,
        id: Uuid,
    ) -> Result<Option<ConnectionProfile>, DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::ProfileGet { id, ack: ack_tx })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn profile_list(&self) -> Result<Vec<ConnectionProfile>, DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::ProfileList { ack: ack_tx })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn profile_update(&self, profile: ConnectionProfile) -> Result<(), DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::ProfileUpdate {
                profile: Box::new(profile),
                ack: ack_tx,
            })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn profile_delete(&self, id: Uuid) -> Result<(), DbError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(DbCommand::ProfileDelete { id, ack: ack_tx })
            .await
            .map_err(|_| DbError::ActorClosed)?;
        ack_rx.await.map_err(|_| DbError::ActorClosed)?
    }

    /// Best-effort shutdown sinyali. Actor mevcut komutu bitirir, sonra döner.
    /// Hata sessizce yutulur — graceful kapanışta channel zaten kapanmış olabilir.
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(DbCommand::Shutdown);
    }
}

/// DB dosyasını açar, migration'ları uygular, actor thread'ini spawn eder.
///
/// **Önemli**: bu fonksiyon recovery çağırmaz — caller `handle.recover()`
/// ile actor üzerinden tetikler (veya `run_recovery` ile actor başlamadan
/// önce sync olarak yapar; Bölüm 15.1).
pub fn spawn_db_actor(db_path: &Path) -> Result<DbActorHandle, DbError> {
    // İlk açılışta migration'ları sync uygula — actor başlamadan şema hazır
    // olmalı.
    let mut conn = Connection::open(db_path)?;
    apply_migrations(&mut conn)?;

    // Channel kapasitesi 1024: Bölüm 15.4 — 16 transfer × 64 chunk burst yeter,
    // dolduğunda producer `await`'te bekler (backpressure).
    let (tx, mut rx) = mpsc::channel::<DbCommand>(1024);

    // Actor blocking thread'inde yaşar çünkü rusqlite sync API'dir;
    // `blocking_recv` ile mpsc'i tüketir. Tokio worker'ını bloklamaz.
    tokio::task::spawn_blocking(move || {
        // Connection bu thread'e bind. Pragmalar yeniden uygulanmaz —
        // `apply_migrations` zaten yaptı; ama defensive:
        if let Err(e) = configure_connection(&conn) {
            tracing::error!(?e, "db_actor: configure_connection failed");
        }

        while let Some(cmd) = rx.blocking_recv() {
            if matches!(cmd, DbCommand::Shutdown) {
                tracing::info!("db_actor: shutdown received");
                break;
            }
            dispatch(&mut conn, cmd);
        }
        tracing::info!("db_actor: receiver closed, exiting");
    });

    Ok(DbActorHandle { tx })
}

/// Tek komutu işler. Ack hataları yutulur (caller drop etmiş olabilir).
fn dispatch(conn: &mut Connection, cmd: DbCommand) {
    match cmd {
        DbCommand::Insert { task, ack } => {
            let _ = ack.send(insert_task(conn, &task));
        }
        DbCommand::UpdateState {
            id,
            new_state,
            ack,
        } => {
            let _ = ack.send(update_state(conn, id, new_state));
        }
        DbCommand::UpdateProgress {
            id,
            bytes_done,
            ack,
        } => {
            let _ = ack.send(update_progress(conn, id, bytes_done));
        }
        DbCommand::Get { id, ack } => {
            let _ = ack.send(get_task(conn, id));
        }
        DbCommand::ListByState { state, ack } => {
            let _ = ack.send(list_by_state(conn, state));
        }
        DbCommand::Recover { ack } => {
            let _ = ack.send(run_recovery(conn));
        }
        DbCommand::ProfileInsert { profile, ack } => {
            let _ = ack.send(insert_profile(conn, &profile));
        }
        DbCommand::ProfileGet { id, ack } => {
            let _ = ack.send(get_profile(conn, id));
        }
        DbCommand::ProfileList { ack } => {
            let _ = ack.send(list_profiles(conn));
        }
        DbCommand::ProfileUpdate { profile, ack } => {
            let _ = ack.send(update_profile(conn, &profile));
        }
        DbCommand::ProfileDelete { id, ack } => {
            let _ = ack.send(delete_profile(conn, id));
        }
        DbCommand::Shutdown => {} // outer loop'ta yakalanıyor
    }
}

// ---------- CRUD impls (actor-internal) ----------

fn insert_task(conn: &Connection, task: &PersistedTransferTask) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO transfers (
            id, profile_id, direction, state, priority,
            local_path, remote_path, bytes_total, bytes_done, chunk_size,
            retry_count, last_error, schema_version,
            created_at, updated_at, started_at, completed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13,
            ?14, ?15, ?16, ?17
         )",
        params![
            task.id.to_string(),
            task.profile_id.to_string(),
            direction_as_str(task.direction),
            state_as_str(task.state),
            task.priority,
            task.local_path.to_string_lossy().to_string(),
            task.remote_path,
            task.bytes_total as i64,
            task.bytes_done as i64,
            task.chunk_size as i64,
            task.retry_count as i64,
            task.last_error,
            task.schema_version as i64,
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            task.started_at.map(|d| d.to_rfc3339()),
            task.completed_at.map(|d| d.to_rfc3339()),
        ],
    )?;
    Ok(())
}

fn update_state(
    conn: &mut Connection,
    id: Uuid,
    new_state: TransferState,
) -> Result<(), DbError> {
    // Önce mevcut state'i oku — state machine validasyonu için. Tek
    // transaction içinde read+write yaparak aradaki TOCTOU'yu kapatıyoruz;
    // DbActor zaten tek writer ama defensif.
    let tx = conn.transaction()?;
    let current_state_str: String = match tx.query_row(
        "SELECT state FROM transfers WHERE id = ?1",
        params![id.to_string()],
        |row| row.get(0),
    ) {
        Ok(s) => s,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Err(DbError::NotFound(id)),
        Err(e) => return Err(e.into()),
    };
    let current = parse_state(&current_state_str)?;

    if !can_transition_to(current, new_state) {
        return Err(DbError::InvalidTransition {
            from: current,
            to: new_state,
        });
    }

    let now = chrono::Utc::now().to_rfc3339();
    // `started_at` / `completed_at` zarif şekilde set edilir:
    // - İlk Active'e geçişte started_at NULL ise doldurulur.
    // - Terminal state'e (Completed/Failed/Cancelled/Skipped) geçişte
    //   completed_at doldurulur.
    let set_started =
        new_state == TransferState::Active && current != TransferState::Paused;
    let is_terminal = matches!(
        new_state,
        TransferState::Completed
            | TransferState::Failed
            | TransferState::Cancelled
            | TransferState::Skipped
    );

    tx.execute(
        "UPDATE transfers
         SET state = ?1,
             updated_at = ?2,
             started_at = COALESCE(started_at, CASE WHEN ?3 THEN ?2 ELSE NULL END),
             completed_at = CASE WHEN ?4 THEN ?2 ELSE completed_at END
         WHERE id = ?5",
        params![
            state_as_str(new_state),
            now,
            set_started,
            is_terminal,
            id.to_string(),
        ],
    )?;
    tx.commit()?;
    Ok(())
}

fn update_progress(conn: &Connection, id: Uuid, bytes_done: u64) -> Result<(), DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    let updated = conn.execute(
        "UPDATE transfers SET bytes_done = ?1, updated_at = ?2 WHERE id = ?3",
        params![bytes_done as i64, now, id.to_string()],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound(id));
    }
    Ok(())
}

fn get_task(conn: &Connection, id: Uuid) -> Result<Option<PersistedTransferTask>, DbError> {
    let mut stmt = conn.prepare(SELECT_ALL_BY_ID)?;
    let mut rows = stmt.query(params![id.to_string()])?;
    match rows.next()? {
        Some(row) => Ok(Some(PersistedTransferTask::from_row(row)?)),
        None => Ok(None),
    }
}

fn list_by_state(
    conn: &Connection,
    state: TransferState,
) -> Result<Vec<PersistedTransferTask>, DbError> {
    let mut stmt = conn.prepare(LIST_BY_STATE)?;
    let mut rows = stmt.query(params![state_as_str(state)])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(PersistedTransferTask::from_row(row)?);
    }
    Ok(out)
}

// SELECT'ler explicit kolon listesi ile — `from_row` kolon adıyla okuduğu için
// SELECT * de çalışırdı, ama gerek olmayan I/O'yu önlemek ve şema değişimi
// sırasında stabilite için açık liste.
const SELECT_ALL_BY_ID: &str = "SELECT id, profile_id, direction, state, priority,
    local_path, remote_path, bytes_total, bytes_done, chunk_size,
    retry_count, last_error, schema_version,
    created_at, updated_at, started_at, completed_at
    FROM transfers WHERE id = ?1";

const LIST_BY_STATE: &str = "SELECT id, profile_id, direction, state, priority,
    local_path, remote_path, bytes_total, bytes_done, chunk_size,
    retry_count, last_error, schema_version,
    created_at, updated_at, started_at, completed_at
    FROM transfers WHERE state = ?1
    ORDER BY priority DESC, created_at ASC";

// ---------- ConnectionProfile CRUD (actor-internal) ----------

const SELECT_PROFILE_BY_ID: &str =
    "SELECT id, name, protocol, host, port, username, remote_root, local_root,
            auth_method, options_json, created_at, updated_at
       FROM profiles WHERE id = ?1";

const SELECT_PROFILES_ALL: &str =
    "SELECT id, name, protocol, host, port, username, remote_root, local_root,
            auth_method, options_json, created_at, updated_at
       FROM profiles ORDER BY name COLLATE NOCASE ASC";

fn insert_profile(conn: &Connection, profile: &ConnectionProfile) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO profiles (
            id, name, protocol, host, port, username,
            remote_root, local_root, auth_method, options_json,
            created_at, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10,
            ?11, ?12
         )",
        params![
            profile.id.to_string(),
            profile.name,
            profile.protocol.as_str(),
            profile.host,
            profile.port.map(|p| p as i64),
            profile.username,
            profile.remote_root,
            profile
                .local_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            profile.auth_method.as_str(),
            profile.options_json,
            profile.created_at.to_rfc3339(),
            profile.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn get_profile(conn: &Connection, id: Uuid) -> Result<Option<ConnectionProfile>, DbError> {
    let mut stmt = conn.prepare(SELECT_PROFILE_BY_ID)?;
    let mut rows = stmt.query(params![id.to_string()])?;
    match rows.next()? {
        Some(row) => Ok(Some(ConnectionProfile::from_row(row)?)),
        None => Ok(None),
    }
}

fn list_profiles(conn: &Connection) -> Result<Vec<ConnectionProfile>, DbError> {
    let mut stmt = conn.prepare(SELECT_PROFILES_ALL)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(ConnectionProfile::from_row(row)?);
    }
    Ok(out)
}

fn update_profile(conn: &Connection, profile: &ConnectionProfile) -> Result<(), DbError> {
    // Full overwrite by id. `updated_at` caller tarafından set edilmiş olur;
    // burada ek bir "şu an" mührü basmıyoruz — caller ile DB time'ı arasında
    // gizli drift yaratmasın.
    let affected = conn.execute(
        "UPDATE profiles SET
            name = ?1,
            protocol = ?2,
            host = ?3,
            port = ?4,
            username = ?5,
            remote_root = ?6,
            local_root = ?7,
            auth_method = ?8,
            options_json = ?9,
            updated_at = ?10
         WHERE id = ?11",
        params![
            profile.name,
            profile.protocol.as_str(),
            profile.host,
            profile.port.map(|p| p as i64),
            profile.username,
            profile.remote_root,
            profile
                .local_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            profile.auth_method.as_str(),
            profile.options_json,
            profile.updated_at.to_rfc3339(),
            profile.id.to_string(),
        ],
    )?;
    if affected == 0 {
        return Err(DbError::NotFound(profile.id));
    }
    Ok(())
}

fn delete_profile(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    let affected = conn.execute(
        "DELETE FROM profiles WHERE id = ?1",
        params![id.to_string()],
    )?;
    if affected == 0 {
        return Err(DbError::NotFound(id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::TransferDirection;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn sample_task(state: TransferState) -> PersistedTransferTask {
        PersistedTransferTask {
            id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
            direction: TransferDirection::Upload,
            state,
            priority: 0,
            local_path: PathBuf::from("/tmp/source.bin"),
            remote_path: "/remote/dest.bin".to_string(),
            bytes_total: 1024 * 1024,
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

    async fn fresh_actor() -> (tempfile::TempDir, DbActorHandle) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("queue.db");
        let handle = spawn_db_actor(&db_path).expect("spawn actor");
        (dir, handle)
    }

    #[tokio::test]
    async fn insert_and_get_roundtrip() {
        let (_dir, handle) = fresh_actor().await;
        let task = sample_task(TransferState::Queued);
        let id = task.id;
        let profile = task.profile_id;
        handle.insert(task.clone()).await.unwrap();

        let fetched = handle.get(id).await.unwrap().expect("task exists");
        assert_eq!(fetched.id, id);
        assert_eq!(fetched.profile_id, profile);
        assert_eq!(fetched.state, TransferState::Queued);
        assert_eq!(fetched.bytes_total, 1024 * 1024);
        assert_eq!(fetched.chunk_size, 64 * 1024);
        assert_eq!(fetched.direction, TransferDirection::Upload);
        // DateTime round-trip: rfc3339 → DateTime → rfc3339 idempotent
        assert_eq!(
            fetched.created_at.timestamp_millis(),
            task.created_at.timestamp_millis()
        );
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_id() {
        let (_dir, handle) = fresh_actor().await;
        let result = handle.get(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_state_valid_transition() {
        let (_dir, handle) = fresh_actor().await;
        let task = sample_task(TransferState::Queued);
        let id = task.id;
        handle.insert(task).await.unwrap();

        handle.update_state(id, TransferState::Active).await.unwrap();

        let fetched = handle.get(id).await.unwrap().unwrap();
        assert_eq!(fetched.state, TransferState::Active);
        assert!(fetched.started_at.is_some(), "started_at set on Active");
    }

    #[tokio::test]
    async fn update_state_invalid_transition_errors() {
        let (_dir, handle) = fresh_actor().await;
        // Completed terminal — geri dönüş yok
        let mut task = sample_task(TransferState::Completed);
        task.completed_at = Some(Utc::now());
        let id = task.id;
        handle.insert(task).await.unwrap();

        let err = handle
            .update_state(id, TransferState::Active)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                DbError::InvalidTransition {
                    from: TransferState::Completed,
                    to: TransferState::Active
                }
            ),
            "expected InvalidTransition, got: {err:?}"
        );

        // State değişmedi mi?
        let fetched = handle.get(id).await.unwrap().unwrap();
        assert_eq!(fetched.state, TransferState::Completed);
    }

    #[tokio::test]
    async fn update_state_not_found() {
        let (_dir, handle) = fresh_actor().await;
        let err = handle
            .update_state(Uuid::new_v4(), TransferState::Active)
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[tokio::test]
    async fn update_progress_updates_bytes_done() {
        let (_dir, handle) = fresh_actor().await;
        let task = sample_task(TransferState::Active);
        let id = task.id;
        handle.insert(task).await.unwrap();

        handle.update_progress(id, 512 * 1024).await.unwrap();

        let fetched = handle.get(id).await.unwrap().unwrap();
        assert_eq!(fetched.bytes_done, 512 * 1024);
    }

    #[tokio::test]
    async fn update_progress_not_found() {
        let (_dir, handle) = fresh_actor().await;
        let err = handle
            .update_progress(Uuid::new_v4(), 100)
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[tokio::test]
    async fn list_by_state_filters_correctly() {
        let (_dir, handle) = fresh_actor().await;
        for _ in 0..3 {
            handle.insert(sample_task(TransferState::Queued)).await.unwrap();
        }
        handle.insert(sample_task(TransferState::Active)).await.unwrap();
        handle.insert(sample_task(TransferState::Completed)).await.unwrap();

        let queued = handle.list_by_state(TransferState::Queued).await.unwrap();
        assert_eq!(queued.len(), 3);
        for task in &queued {
            assert_eq!(task.state, TransferState::Queued);
        }

        let active = handle.list_by_state(TransferState::Active).await.unwrap();
        assert_eq!(active.len(), 1);
    }

    // ---------- ConnectionProfile CRUD round-trip ----------

    fn sample_profile(name: &str) -> ConnectionProfile {
        use crate::profiles::{AuthMethod, ProfileProtocol};
        let now = Utc::now();
        ConnectionProfile {
            id: Uuid::new_v4(),
            name: name.to_string(),
            protocol: ProfileProtocol::Sftp,
            host: Some("example.com".into()),
            port: Some(22),
            username: Some("alice".into()),
            remote_root: Some("/srv".into()),
            local_root: None,
            auth_method: AuthMethod::Password,
            options_json: "{}".into(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn profile_insert_and_get_roundtrip() {
        let (_dir, handle) = fresh_actor().await;
        let profile = sample_profile("prod");
        let id = profile.id;
        handle.profile_insert(profile.clone()).await.unwrap();

        let fetched = handle.profile_get(id).await.unwrap().expect("exists");
        assert_eq!(fetched.id, id);
        assert_eq!(fetched.name, "prod");
        assert_eq!(fetched.host.as_deref(), Some("example.com"));
        assert_eq!(fetched.port, Some(22));
        assert_eq!(fetched.username.as_deref(), Some("alice"));
        assert_eq!(fetched.remote_root.as_deref(), Some("/srv"));
        assert!(fetched.local_root.is_none());
        assert_eq!(fetched.options_json, "{}");
        assert_eq!(
            fetched.created_at.timestamp_millis(),
            profile.created_at.timestamp_millis()
        );
    }

    #[tokio::test]
    async fn profile_get_missing_returns_none() {
        let (_dir, handle) = fresh_actor().await;
        let result = handle.profile_get(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn profile_list_orders_by_name_case_insensitive() {
        let (_dir, handle) = fresh_actor().await;
        handle.profile_insert(sample_profile("zeta")).await.unwrap();
        handle.profile_insert(sample_profile("Alpha")).await.unwrap();
        handle.profile_insert(sample_profile("middle")).await.unwrap();

        let listed = handle.profile_list().await.unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].name, "Alpha");
        assert_eq!(listed[1].name, "middle");
        assert_eq!(listed[2].name, "zeta");
    }

    #[tokio::test]
    async fn profile_update_overwrites_fields() {
        let (_dir, handle) = fresh_actor().await;
        let mut profile = sample_profile("draft");
        let id = profile.id;
        handle.profile_insert(profile.clone()).await.unwrap();

        profile.name = "final".into();
        profile.host = Some("new.example.com".into());
        profile.port = Some(2222);
        profile.updated_at = Utc::now();
        handle.profile_update(profile.clone()).await.unwrap();

        let fetched = handle.profile_get(id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "final");
        assert_eq!(fetched.host.as_deref(), Some("new.example.com"));
        assert_eq!(fetched.port, Some(2222));
    }

    #[tokio::test]
    async fn profile_update_not_found_errors() {
        let (_dir, handle) = fresh_actor().await;
        let profile = sample_profile("orphan");
        let err = handle.profile_update(profile).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[tokio::test]
    async fn profile_delete_removes_row() {
        let (_dir, handle) = fresh_actor().await;
        let profile = sample_profile("doomed");
        let id = profile.id;
        handle.profile_insert(profile).await.unwrap();

        handle.profile_delete(id).await.unwrap();

        let fetched = handle.profile_get(id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn profile_delete_not_found_errors() {
        let (_dir, handle) = fresh_actor().await;
        let err = handle.profile_delete(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[tokio::test]
    async fn profile_local_roundtrip_with_path() {
        use crate::profiles::{AuthMethod, ProfileProtocol};
        use std::path::PathBuf;
        let (_dir, handle) = fresh_actor().await;
        let now = Utc::now();
        let profile = ConnectionProfile {
            id: Uuid::new_v4(),
            name: "local-fs".into(),
            protocol: ProfileProtocol::Local,
            host: None,
            port: None,
            username: None,
            remote_root: None,
            local_root: Some(PathBuf::from("/home/user/data")),
            auth_method: AuthMethod::None,
            options_json: "{}".into(),
            created_at: now,
            updated_at: now,
        };
        let id = profile.id;
        handle.profile_insert(profile).await.unwrap();
        let fetched = handle.profile_get(id).await.unwrap().unwrap();
        assert_eq!(fetched.protocol, ProfileProtocol::Local);
        assert_eq!(fetched.local_root.as_deref(), Some(std::path::Path::new("/home/user/data")));
        assert!(fetched.host.is_none());
    }

    #[tokio::test]
    async fn concurrent_producers_are_serialized() {
        // 16 producer × 50 update — DbActor sırayla işler, hiç SQLITE_BUSY yok.
        let (_dir, handle) = fresh_actor().await;
        let task = sample_task(TransferState::Active);
        let id = task.id;
        handle.insert(task).await.unwrap();

        let handle = Arc::new(handle);
        let mut joins = Vec::new();
        for producer in 0..16u64 {
            let h = handle.clone();
            joins.push(tokio::spawn(async move {
                for i in 0..50u64 {
                    let bytes = producer * 1000 + i;
                    h.update_progress(id, bytes).await.expect("no SQLITE_BUSY");
                }
            }));
        }
        for j in joins {
            j.await.unwrap();
        }
        // Tüm 800 update başarılı; son okunan değer 16 producer'dan birinin son
        // yazdığı — spesifik değer önemli değil, panic/error olmaması yeterli.
        let fetched = handle.get(id).await.unwrap().unwrap();
        assert!(fetched.bytes_done <= 16_000); // sanity
    }
}
