//! AuditEngine — Bölüm 17.2 batch writer.
//!
//! Mimari:
//! - Producer'lar `AuditEngine::emit(event).await` çağırır — mpsc kanalına
//!   gönderir, blocking yok (kanal kapasitesi 1024, fire-and-forget).
//! - Background worker `recv` ile besler ve `Vec<AuditEvent>` buffer'ı
//!   biriktirir. İki tetikleyici:
//!   - Buffer ≥ 64 entry → hemen flush.
//!   - 500ms tick → buffer doluysa flush.
//! - Flush: tek `Connection::transaction()` ile batch insert. DbActor pattern
//!   queue.db'de var; audit DB tek thread'de bu worker tarafından kullanıldığı
//!   için ekstra actor gerekmez — direkt `Mutex<Connection>`.
//!
//! Hata davranışı: DB yazımı başarısız olursa worker olayı log'lar ama
//! buffer'ı *düşürmez* — bir sonraki tick'te tekrar dener. Worker shutdown
//! sırasında buffer'ı son kez flush eder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::schema;

const CHANNEL_CAPACITY: usize = 1024;
const FLUSH_BATCH_SIZE: usize = 64;
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub enum AuditEngineError {
    #[error("audit db open failed: {0}")]
    Open(#[from] rusqlite::Error),
    #[error("audit db io: {0}")]
    Io(#[from] std::io::Error),
}

/// Audit log'a düşen event türleri. Yeni tip eklendiğinde
/// `kind_str()` ve discriminant aynı kalır — Rust enum reordering
/// DB string'ini bozmaz.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuditEventKind {
    /// Audit'i etkinleştirme onayı — KVKK/GDPR rıza kaydı.
    ConsentGiven { ip: String, user_agent: String },
    ConsentRevoked,
    ProfileCreated { profile_id: Uuid, name: String },
    ProfileDeleted { profile_id: Uuid },
    TransferStarted {
        transfer_id: Uuid,
        profile_id: Uuid,
        direction: String,
        local_path: String,
        remote_path: String,
        bytes_total: u64,
    },
    TransferCompleted {
        transfer_id: Uuid,
        bytes: u64,
        duration_ms: u64,
        checksum: String,
    },
    TransferFailed {
        transfer_id: Uuid,
        reason: String,
        category: String,
    },
    TransferCancelled { transfer_id: Uuid },
    /// Diagnostics bundle export'u (kim, nereye, hangi mask seti).
    DiagnosticsExported { destination: PathBuf, masks_applied: bool },
}

impl AuditEventKind {
    /// SQL'e yazılan stabil string ad — `serde(tag)` ile aynı discriminant.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::ConsentGiven { .. } => "consentGiven",
            Self::ConsentRevoked => "consentRevoked",
            Self::ProfileCreated { .. } => "profileCreated",
            Self::ProfileDeleted { .. } => "profileDeleted",
            Self::TransferStarted { .. } => "transferStarted",
            Self::TransferCompleted { .. } => "transferCompleted",
            Self::TransferFailed { .. } => "transferFailed",
            Self::TransferCancelled { .. } => "transferCancelled",
            Self::DiagnosticsExported { .. } => "diagnosticsExported",
        }
    }

    /// İlişkili profil id'si (varsa) — DB kolonu için.
    pub fn profile_id(&self) -> Option<Uuid> {
        match self {
            Self::ProfileCreated { profile_id, .. }
            | Self::ProfileDeleted { profile_id }
            | Self::TransferStarted { profile_id, .. } => Some(*profile_id),
            _ => None,
        }
    }

    /// İlişkili transfer id'si (varsa).
    pub fn transfer_id(&self) -> Option<Uuid> {
        match self {
            Self::TransferStarted { transfer_id, .. }
            | Self::TransferCompleted { transfer_id, .. }
            | Self::TransferFailed { transfer_id, .. }
            | Self::TransferCancelled { transfer_id } => Some(*transfer_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub occurred_at: DateTime<Utc>,
    pub kind: AuditEventKind,
    /// `MaskingEngine.apply()` pre-pass uygulandı mı? (false → ham payload,
    /// audit reader tarafında uyarı gösterilir.)
    #[serde(default = "default_masked")]
    pub masked: bool,
}

fn default_masked() -> bool {
    true
}

impl AuditEvent {
    pub fn now(kind: AuditEventKind) -> Self {
        Self {
            occurred_at: Utc::now(),
            kind,
            masked: true,
        }
    }
}

#[derive(Clone)]
pub struct AuditEngine {
    tx: mpsc::Sender<AuditEvent>,
}

impl AuditEngine {
    /// `audit.db` aç, schema'yı uygula, worker spawn et.
    ///
    /// `cancel` token ile worker shutdown sinyali — root cancellation'a
    /// bağlanır, app kapanırken worker buffer'ı son kez flush eder.
    pub fn spawn(
        db_path: PathBuf,
        cancel: CancellationToken,
    ) -> Result<Self, AuditEngineError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(&db_path)?;
        schema::initialize(&mut conn)?;
        info!(?db_path, "audit_db opened");

        let (tx, rx) = mpsc::channel::<AuditEvent>(CHANNEL_CAPACITY);
        let conn = Arc::new(Mutex::new(conn));
        tokio::spawn(run_worker(rx, conn, cancel));
        Ok(Self { tx })
    }

    /// Fire-and-forget emit. Kanal doluysa `send` await edilir (back-pressure
    /// — producer kısa süreli bekler); kanal kapalıysa hata yutulur (audit
    /// shutdown sonrası emit pratikte no-op).
    pub async fn emit(&self, kind: AuditEventKind) {
        let event = AuditEvent::now(kind);
        if self.tx.send(event).await.is_err() {
            debug!("audit emit dropped — worker closed");
        }
    }

    /// `try_send` versiyonu — kanal doluysa yutar. Hot path'lerde await
    /// edilemeyen yerler için.
    pub fn try_emit(&self, kind: AuditEventKind) {
        let event = AuditEvent::now(kind);
        if let Err(e) = self.tx.try_send(event) {
            warn!(?e, "audit try_emit dropped");
        }
    }
}

async fn run_worker(
    mut rx: mpsc::Receiver<AuditEvent>,
    conn: Arc<Mutex<Connection>>,
    cancel: CancellationToken,
) {
    let mut buffer: Vec<AuditEvent> = Vec::with_capacity(FLUSH_BATCH_SIZE);
    let mut ticker = tokio::time::interval(FLUSH_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                info!("audit worker shutdown requested — draining");
                while let Ok(ev) = rx.try_recv() {
                    buffer.push(ev);
                }
                if !buffer.is_empty() {
                    if let Err(e) = flush_to_db(&conn, &mut buffer).await {
                        error!(?e, "final audit flush failed");
                    }
                }
                break;
            }
            maybe = rx.recv() => {
                match maybe {
                    Some(event) => {
                        buffer.push(event);
                        if buffer.len() >= FLUSH_BATCH_SIZE {
                            if let Err(e) = flush_to_db(&conn, &mut buffer).await {
                                error!(?e, "audit batch flush failed (will retry next tick)");
                            }
                        }
                    }
                    None => {
                        if !buffer.is_empty() {
                            let _ = flush_to_db(&conn, &mut buffer).await;
                        }
                        break;
                    }
                }
            }
            _ = ticker.tick() => {
                if !buffer.is_empty() {
                    if let Err(e) = flush_to_db(&conn, &mut buffer).await {
                        error!(?e, "audit tick flush failed");
                    }
                }
            }
        }
    }
}

async fn flush_to_db(
    conn: &Arc<Mutex<Connection>>,
    buffer: &mut Vec<AuditEvent>,
) -> Result<(), rusqlite::Error> {
    if buffer.is_empty() {
        return Ok(());
    }
    let drained: Vec<AuditEvent> = buffer.drain(..).collect();
    let conn_arc = Arc::clone(conn);
    let count = drained.len();
    tokio::task::spawn_blocking(move || -> Result<(), rusqlite::Error> {
        let mut guard = conn_arc.blocking_lock();
        let tx = guard.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO audit_events
                    (occurred_at, kind, profile_id, transfer_id, payload_json, masked_flag)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for ev in &drained {
                let payload = serde_json::to_string(&ev.kind).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                })?;
                stmt.execute(rusqlite::params![
                    ev.occurred_at.to_rfc3339(),
                    ev.kind.kind_str(),
                    ev.kind.profile_id().map(|u| u.to_string()),
                    ev.kind.transfer_id().map(|u| u.to_string()),
                    payload,
                    ev.masked as i64,
                ])?;
            }
        }
        tx.commit()?;
        debug!(rows = count, "audit batch committed");
        Ok(())
    })
    .await
    .expect("spawn_blocking joined")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::time::sleep;

    #[tokio::test]
    async fn batches_flush_to_db() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("audit.db");
        let cancel = CancellationToken::new();
        let engine = AuditEngine::spawn(path.clone(), cancel.clone()).unwrap();

        for i in 0..10 {
            engine
                .emit(AuditEventKind::ProfileCreated {
                    profile_id: Uuid::new_v4(),
                    name: format!("p{i}"),
                })
                .await;
        }

        // 500ms tick'i bekle, sonra DB'yi kontrol et.
        sleep(Duration::from_millis(800)).await;

        let conn = Connection::open(&path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 10);

        cancel.cancel();
        sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn shutdown_drains_remaining_buffer() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("audit.db");
        let cancel = CancellationToken::new();
        let engine = AuditEngine::spawn(path.clone(), cancel.clone()).unwrap();

        for _ in 0..5 {
            engine.emit(AuditEventKind::ConsentRevoked).await;
        }
        cancel.cancel();
        sleep(Duration::from_millis(200)).await;

        let conn = Connection::open(&path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn batch_threshold_flushes_immediately() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("audit.db");
        let cancel = CancellationToken::new();
        let engine = AuditEngine::spawn(path.clone(), cancel.clone()).unwrap();

        // 64 entry threshold — 65 yollarsak en az 64'ü hemen yazılır.
        for i in 0..65 {
            engine
                .emit(AuditEventKind::TransferCancelled {
                    transfer_id: Uuid::from_u128(i as u128),
                })
                .await;
        }
        // Threshold flush async spawn_blocking içinde; kısa bir sleep yeter.
        sleep(Duration::from_millis(150)).await;

        let conn = Connection::open(&path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert!(count >= 64, "beklenen >=64, alınan {count}");

        cancel.cancel();
    }

    #[test]
    fn kind_str_is_stable() {
        assert_eq!(AuditEventKind::ConsentRevoked.kind_str(), "consentRevoked");
        assert_eq!(
            AuditEventKind::TransferCancelled {
                transfer_id: Uuid::nil()
            }
            .kind_str(),
            "transferCancelled"
        );
    }
}
