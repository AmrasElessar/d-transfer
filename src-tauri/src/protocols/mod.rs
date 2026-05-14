//! Protocol adapter katmanı — Bölüm 9.1, 11.
//!
//! `ProtocolAdapter` trait'i tüm backend'lerin (SFTP, S3, WebDAV, Local FS)
//! uyduğu kontrattır. v1.0'da compile-time bağlı tek monolitik binary; v2+'da
//! third-party adapter ekosistemi (sandboxed) buradan beslenecek.
//!
//! Faz 1 yalnızca trait + tipler + `NoopAdapter` placeholder içerir. Asıl SFTP,
//! S3, WebDAV adapter'ları Faz 2'de eklenir.

pub mod adapter;
pub mod local;
pub mod noop;
pub mod sftp;
pub mod types;

pub use adapter::ProtocolAdapter;
pub use local::LocalAdapter;
pub use noop::NoopAdapter;
pub use sftp::SftpAdapter;
pub use types::{
    AdapterCapabilities, ChecksumAlgo, FsyncPolicy, ListOpts, LocalPath, OverwritePolicy,
    ProgressSender, ProtocolInfo, RemoteEntry, RemoteEntryKind, RemotePath, TransferOptions,
    TransferResult, TransferStats,
};
