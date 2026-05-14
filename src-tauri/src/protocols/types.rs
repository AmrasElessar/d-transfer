//! ProtocolAdapter tarafından paylaşılan veri tipleri (Bölüm 9.1, 9.3).
//!
//! `LocalPath` / `RemotePath` ayrı newtype'lar çünkü:
//! 1. Tip sisteminde "remote yolu local yerine geçti" sınıfı bug'ları engelle.
//! 2. Remote yollar invalid UTF-8 olabilir (Linux raw bytes) — Bölüm 12.7
//!    için temel; `PathTransport` enum'una ileride evrilir.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalPath(pub PathBuf);

impl LocalPath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemotePath(pub String);

impl RemotePath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEntry {
    pub path: RemotePath,
    pub name: String,
    pub kind: RemoteEntryKind,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    /// Backend-specific content hash. Bölüm 5.3 — SFTP yok, S3 ETag (NOT MD5!),
    /// WebDAV ETag, Local FS local re-read sonucu. Identity proof DEĞİL.
    pub remote_hash: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ListOpts {
    /// SFTP default 1024, S3 default 1000 (Bölüm 9.1).
    pub page_size: u32,
    pub recursive: bool,
    pub include_hidden: bool,
}

impl Default for ListOpts {
    fn default() -> Self {
        Self {
            page_size: 1000,
            recursive: false,
            include_hidden: false,
        }
    }
}

/// Adapter'ın startup probe sonrası dönen capability bayrakları (Bölüm 5.3).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterCapabilities {
    pub supports_byte_range: bool,
    pub supports_remote_checksum: bool,
    pub supports_server_side_rename: bool,
    pub supports_symlinks: bool,
    pub supports_resume: bool,
    pub supports_multipart: bool,
    pub max_parallel_sessions: u8,
}

impl Default for AdapterCapabilities {
    fn default() -> Self {
        Self {
            supports_byte_range: false,
            supports_remote_checksum: false,
            supports_server_side_rename: false,
            supports_symlinks: false,
            supports_resume: false,
            supports_multipart: false,
            max_parallel_sessions: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProtocolInfo {
    Sftp { host: String, port: u16 },
    S3 { endpoint: String, bucket: String },
    Webdav { url: String },
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverwritePolicy {
    Ask,
    OverwriteAll,
    SkipExisting,
    ResumeIfPossible,
    KeepBoth,
}

impl Default for OverwritePolicy {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChecksumAlgo {
    None,
    Sha256,
    XxHash3,
}

impl Default for ChecksumAlgo {
    fn default() -> Self {
        Self::Sha256
    }
}

/// fsync politikası — adapter dosya finalize ederken hangi seviyede senkron
/// yapsın? Bölüm 14.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FsyncPolicy {
    /// Hiç fsync yok — hızlı, power-cut'ta veri kaybı kabul.
    None,
    /// `sync_data()` + (POSIX'te) parent dir `sync_all()`. Default.
    DataOnly,
    /// `sync_all()` — data + metadata.
    Full,
}

impl Default for FsyncPolicy {
    fn default() -> Self {
        Self::DataOnly
    }
}

/// Transfer çağrısının runtime parametreleri (Bölüm 9.3).
#[derive(Debug, Clone)]
pub struct TransferOptions {
    pub chunk_size: usize,
    pub parallel_streams: u8,
    pub max_inflight_bytes: usize,
    pub retry_max: u8,
    pub retry_backoff_ms: u64,
    pub speed_limit_bps: Option<u64>,
    pub delta_enabled: bool,
    pub verify_checksum: ChecksumAlgo,
    pub encrypt_at_rest: bool,
    pub overwrite_policy: OverwritePolicy,
    pub preserve_mtime: bool,
    pub max_buffered_chunks: usize,
    /// Adapter dosya finalize aşamasında bu politikaya uyar (Bölüm 14.6).
    pub fsync_policy: FsyncPolicy,
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            chunk_size: 8 * 1024 * 1024,           // 8 MiB
            parallel_streams: 4,
            max_inflight_bytes: 64 * 1024 * 1024,  // 64 MiB (Bölüm 9.2)
            retry_max: 5,
            retry_backoff_ms: 1_000,
            speed_limit_bps: None,
            delta_enabled: false,
            verify_checksum: ChecksumAlgo::Sha256,
            encrypt_at_rest: false,
            overwrite_policy: OverwritePolicy::Ask,
            preserve_mtime: true,
            max_buffered_chunks: 8,
            fsync_policy: FsyncPolicy::DataOnly,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferStats {
    pub bytes_transferred: u64,
    pub duration_ms: u64,
    pub avg_speed_bps: f64,
    pub checksum: Option<String>,
    pub chunks_retried: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferResult {
    pub stats: TransferStats,
    /// Backend tarafından bilinen son state — eg. S3 ETag, SFTP mtime.
    pub remote_metadata: Option<RemoteEntry>,
}

/// Per-chunk progress mesajını ProgressAggregator'a aktaran kanal (Bölüm 9.4).
/// Faz 1'de raw byte+state payload; Faz 2'de struct'a evrilir.
pub type ProgressSender = mpsc::Sender<ProgressTick>;

#[derive(Debug, Clone, Copy)]
pub struct ProgressTick {
    pub chunk_index: u32,
    pub bytes_done: u64,
    pub bytes_total: u64,
}
