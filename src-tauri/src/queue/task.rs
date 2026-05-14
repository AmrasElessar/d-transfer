//! `PersistedTransferTask` — queue.db `transfers` tablosunun Rust temsili.
//!
//! Sadece struct + row parser; CRUD operasyonları DbActor üzerinden geçer
//! (Bölüm 15.4 — yazma serileştirmesi).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rusqlite::Row;
use uuid::Uuid;

use crate::engine::TransferDirection;
use crate::events::TransferState;

/// Bir transfer task'inin DB'ye yansıyan tam durumu.
///
/// Bölüm 15.1 `PersistedTransferTask` ile aynı amaca hizmet eder; bazı alanlar
/// (chunk_size, schema_version, started_at, completed_at) Faz 3 ihtiyaçlarına
/// göre eklendi. `source/destination` `TransferEndpoint`'i Faz 4 multi-profil
/// adım'ında eklenecek; şimdilik tek profile_id + local/remote path yeterli
/// çünkü local-to-local + tek adapter senaryosu destekleniyor.
#[derive(Debug, Clone)]
pub struct PersistedTransferTask {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub priority: i32,
    pub local_path: PathBuf,
    pub remote_path: String,
    pub bytes_total: u64,
    pub bytes_done: u64,
    /// **Immutable**: task oluşturulurken sabitlenir, sonradan değişmez
    /// (Bölüm 15.2 v1.13 errata — chunk_size snapshot).
    pub chunk_size: usize,
    pub retry_count: u32,
    /// `Option<String>` = JSON-serialized `WireError` (Bölüm 10).
    pub last_error: Option<String>,
    pub schema_version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl PersistedTransferTask {
    /// `transfers` tablosundan tek satırı tipize çevirir.
    ///
    /// Sıralama `SELECT *` değil — sütun yerine alan adıyla çekmek API
    /// kararlılığı için tercih edilir.
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let id_str: String = row.get("id")?;
        let profile_id_str: String = row.get("profile_id")?;
        let direction_str: String = row.get("direction")?;
        let state_str: String = row.get("state")?;
        let last_error: Option<String> = row.get("last_error")?;
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        let started_at_str: Option<String> = row.get("started_at")?;
        let completed_at_str: Option<String> = row.get("completed_at")?;
        let local_path_str: String = row.get("local_path")?;
        let bytes_total_i: i64 = row.get("bytes_total")?;
        let bytes_done_i: i64 = row.get("bytes_done")?;
        let chunk_size_i: i64 = row.get("chunk_size")?;
        let retry_count_i: i64 = row.get("retry_count")?;
        let schema_version_i: i64 = row.get("schema_version")?;
        let priority_i: i64 = row.get("priority")?;

        Ok(Self {
            id: parse_uuid(&id_str)?,
            profile_id: parse_uuid(&profile_id_str)?,
            direction: parse_direction(&direction_str)?,
            state: parse_state(&state_str)?,
            priority: priority_i as i32,
            local_path: PathBuf::from(local_path_str),
            remote_path: row.get("remote_path")?,
            bytes_total: bytes_total_i.max(0) as u64,
            bytes_done: bytes_done_i.max(0) as u64,
            chunk_size: chunk_size_i.max(0) as usize,
            retry_count: retry_count_i.max(0) as u32,
            last_error,
            schema_version: schema_version_i.max(0) as u32,
            created_at: parse_dt(&created_at_str)?,
            updated_at: parse_dt(&updated_at_str)?,
            started_at: started_at_str.as_deref().map(parse_dt).transpose()?,
            completed_at: completed_at_str.as_deref().map(parse_dt).transpose()?,
        })
    }
}

// ---------- Enum string codec'leri ----------
//
// Bunlar `pub(super)` çünkü db_actor'ın INSERT/UPDATE bind'leri de aynı
// gösterimi kullanmak zorunda — kanonik tek noktada tutuluyor.

pub(super) fn state_as_str(s: TransferState) -> &'static str {
    use TransferState::*;
    match s {
        Queued => "queued",
        Active => "active",
        Verifying => "verifying",
        Finalizing => "finalizing",
        Paused => "paused",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
        Skipped => "skipped",
    }
}

pub(super) fn parse_state(s: &str) -> rusqlite::Result<TransferState> {
    use TransferState::*;
    Ok(match s {
        "queued" => Queued,
        "active" => Active,
        "verifying" => Verifying,
        "finalizing" => Finalizing,
        "paused" => Paused,
        "completed" => Completed,
        "failed" => Failed,
        "cancelled" => Cancelled,
        "skipped" => Skipped,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown transfer state '{other}'"),
                )),
            ))
        }
    })
}

pub(super) fn direction_as_str(d: TransferDirection) -> &'static str {
    match d {
        TransferDirection::Upload => "upload",
        TransferDirection::Download => "download",
    }
}

pub(super) fn parse_direction(s: &str) -> rusqlite::Result<TransferDirection> {
    Ok(match s {
        "upload" => TransferDirection::Upload,
        "download" => TransferDirection::Download,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown direction '{other}'"),
                )),
            ))
        }
    })
}

pub(super) fn parse_uuid(s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })
}

pub(super) fn parse_dt(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })
}

// ---------- ConnectionProfile row codec ----------
//
// `transfers` ile aynı kanonik tasarımı uygular: Uuid ↔ TEXT, DateTime ↔ RFC3339,
// enum'lar `as_str/parse` üzerinden geçer. Codec'i task.rs içinde tutmak
// db_actor.rs'in tek bir yardımcı modülünden bind yapabilmesini sağlar.

use crate::profiles::{AuthMethod, ConnectionProfile, ProfileProtocol};

impl ConnectionProfile {
    /// `profiles` tablosundan tek satırı tipize çevirir.
    pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let id_str: String = row.get("id")?;
        let protocol_str: String = row.get("protocol")?;
        let auth_str: String = row.get("auth_method")?;
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        let port_i: Option<i64> = row.get("port")?;
        let local_root_str: Option<String> = row.get("local_root")?;

        Ok(Self {
            id: parse_uuid(&id_str)?,
            name: row.get("name")?,
            protocol: parse_profile_protocol(&protocol_str)?,
            host: row.get("host")?,
            port: port_i.map(|p| p as u16),
            username: row.get("username")?,
            remote_root: row.get("remote_root")?,
            local_root: local_root_str.map(PathBuf::from),
            auth_method: parse_auth_method(&auth_str)?,
            options_json: row.get("options_json")?,
            created_at: parse_dt(&created_at_str)?,
            updated_at: parse_dt(&updated_at_str)?,
        })
    }
}

pub(super) fn parse_profile_protocol(s: &str) -> rusqlite::Result<ProfileProtocol> {
    ProfileProtocol::parse(s).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown profile protocol '{s}'"),
            )),
        )
    })
}

pub(super) fn parse_auth_method(s: &str) -> rusqlite::Result<AuthMethod> {
    AuthMethod::parse(s).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown auth method '{s}'"),
            )),
        )
    })
}
