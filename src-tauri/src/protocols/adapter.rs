//! `ProtocolAdapter` trait — Bölüm 9.1.
//!
//! Tüm backend'lerin (SFTP, S3, WebDAV, Local FS) uyması gereken kontrat.
//!
//! ## Tasarım notları
//!
//! - `list_dir` **stream** döndürür, `Vec<RemoteEntry>` değil. Geniş listingler
//!   (2M dosyalı S3 bucket, `/var/log/`) `Vec` ile RAM patlatır — Bölüm 9.1
//!   paginated streaming kuralı.
//! - `connect` / `disconnect` exclusive mutable referans alır (`&mut self`)
//!   çünkü adapter durumu (channel handle, auth context) içerir.
//! - Diğer metodlar `&self` — paralel istekler aynı adapter instance'ı üzerinden
//!   yürür (multiplexed SFTP channel, HTTP/2 connection pool).

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::errors::TransferError;

use super::types::{
    AdapterCapabilities, ListOpts, LocalPath, ProgressSender, ProtocolInfo, RemoteEntry,
    RemotePath, TransferOptions, TransferResult,
};

pub type ConnectionProfile = serde_json::Value;
// Faz 2'de gerçek `ConnectionProfile` struct'ı `crate::profiles` modülünde
// tanımlanacak; Faz 1'de placeholder olarak serde_json::Value.

#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    async fn connect(&mut self, profile: &ConnectionProfile) -> Result<(), TransferError>;

    async fn disconnect(&mut self) -> Result<(), TransferError>;

    fn list_dir(
        &self,
        path: &RemotePath,
        opts: ListOpts,
    ) -> Pin<Box<dyn Stream<Item = Result<RemoteEntry, TransferError>> + Send + '_>>;

    async fn stat(&self, path: &RemotePath) -> Result<RemoteEntry, TransferError>;

    async fn upload(
        &self,
        local: &LocalPath,
        remote: &RemotePath,
        opts: &TransferOptions,
        tx: ProgressSender,
    ) -> Result<TransferResult, TransferError>;

    async fn download(
        &self,
        remote: &RemotePath,
        local: &LocalPath,
        opts: &TransferOptions,
        tx: ProgressSender,
    ) -> Result<TransferResult, TransferError>;

    async fn delete(&self, path: &RemotePath) -> Result<(), TransferError>;

    async fn mkdir(&self, path: &RemotePath) -> Result<(), TransferError>;

    async fn rename(
        &self,
        from: &RemotePath,
        to: &RemotePath,
    ) -> Result<(), TransferError>;

    fn supports_byte_range(&self) -> bool {
        self.capabilities().supports_byte_range
    }

    fn supports_remote_checksum(&self) -> bool {
        self.capabilities().supports_remote_checksum
    }

    fn capabilities(&self) -> AdapterCapabilities;

    fn protocol_info(&self) -> ProtocolInfo;
}
