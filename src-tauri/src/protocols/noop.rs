//! `NoopAdapter` — derleme zamanı bağlı placeholder.
//!
//! Hiçbir backend implementasyonu olmadan trait'in derlenip derlenmediğini
//! doğrular ve testlerde stub olarak kullanılır. Tüm I/O metodları
//! `CapabilityNotSupported` döner.

use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::{self, Stream};

use crate::errors::TransferError;

use super::adapter::{ConnectionProfile, ProtocolAdapter};
use super::types::{
    AdapterCapabilities, ListOpts, LocalPath, ProgressSender, ProtocolInfo, RemoteEntry,
    RemotePath, TransferOptions, TransferResult,
};

#[derive(Default)]
pub struct NoopAdapter;

impl NoopAdapter {
    pub fn new() -> Self {
        Self
    }

    fn unsupported(capability: &str) -> TransferError {
        TransferError::CapabilityNotSupported {
            capability: capability.to_string(),
        }
    }
}

#[async_trait]
impl ProtocolAdapter for NoopAdapter {
    async fn connect(&mut self, _profile: &ConnectionProfile) -> Result<(), TransferError> {
        Err(Self::unsupported("connect"))
    }

    async fn disconnect(&mut self) -> Result<(), TransferError> {
        Ok(())
    }

    fn list_dir(
        &self,
        _path: &RemotePath,
        _opts: ListOpts,
    ) -> Pin<Box<dyn Stream<Item = Result<RemoteEntry, TransferError>> + Send + '_>> {
        Box::pin(stream::iter(std::iter::once(Err(Self::unsupported("list_dir")))))
    }

    async fn stat(&self, _path: &RemotePath) -> Result<RemoteEntry, TransferError> {
        Err(Self::unsupported("stat"))
    }

    async fn upload(
        &self,
        _local: &LocalPath,
        _remote: &RemotePath,
        _opts: &TransferOptions,
        _tx: ProgressSender,
    ) -> Result<TransferResult, TransferError> {
        Err(Self::unsupported("upload"))
    }

    async fn download(
        &self,
        _remote: &RemotePath,
        _local: &LocalPath,
        _opts: &TransferOptions,
        _tx: ProgressSender,
    ) -> Result<TransferResult, TransferError> {
        Err(Self::unsupported("download"))
    }

    async fn delete(&self, _path: &RemotePath) -> Result<(), TransferError> {
        Err(Self::unsupported("delete"))
    }

    async fn mkdir(&self, _path: &RemotePath) -> Result<(), TransferError> {
        Err(Self::unsupported("mkdir"))
    }

    async fn rename(
        &self,
        _from: &RemotePath,
        _to: &RemotePath,
    ) -> Result<(), TransferError> {
        Err(Self::unsupported("rename"))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities::default()
    }

    fn protocol_info(&self) -> ProtocolInfo {
        ProtocolInfo::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn noop_returns_unsupported() {
        let mut adapter = NoopAdapter::new();
        let err = adapter
            .connect(&serde_json::json!({}))
            .await
            .expect_err("expected unsupported");
        assert!(matches!(err, TransferError::CapabilityNotSupported { .. }));
    }

    #[tokio::test]
    async fn noop_list_dir_emits_single_error() {
        let adapter = NoopAdapter::new();
        let mut stream = adapter.list_dir(&RemotePath::new("/"), ListOpts::default());
        let first = stream.next().await.expect("stream should yield once");
        assert!(matches!(
            first,
            Err(TransferError::CapabilityNotSupported { .. })
        ));
        assert!(stream.next().await.is_none(), "stream should be exhausted");
    }
}
