//! Profile registry — adapter factory katmanı (Bölüm 9 control plane).
//!
//! `QueueScheduler` task'leri DB'den çekerken `profile_id` görür ama o id'ye
//! karşılık gelen adapter yapılandırmasını bilmez. Bu modül id → adapter
//! eşlemesini sağlar.
//!
//! `LocalAdapterFactory` Faz 3'ten beri **in-memory** map; Faz 4'te
//! `ConnectionProfile` (bu modülde) ile birlikte kalıcı DB-destekli factory
//! eklenecek. Şimdilik iki katman yan yana yaşıyor — local debug transferleri
//! eski yolu kullanmaya devam ediyor, UI'dan kurulan profiller yeni
//! `connection_profile`/`credentials` katmanından yürür.

pub mod connection_manager;
pub mod connection_profile;
pub mod credentials;

pub use connection_manager::ConnectionManager;
pub use connection_profile::{AuthMethod, ConnectionProfile, ProfileProtocol};
pub use credentials::{
    CredentialError, CredentialVault, KIND_PASSWORD, KIND_PRIVATE_KEY_PASSPHRASE,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use crate::errors::TransferError;
use crate::protocols::{LocalAdapter, ProtocolAdapter};

/// QueueScheduler'ın dispatch sırasında adapter alabilmesi için trait.
///
/// Çoklu protokol desteği geldiğinde implementasyon switch eder (Local + SFTP +
/// S3 + WebDAV → tek registry).
#[async_trait]
pub trait AdapterFactory: Send + Sync {
    async fn build(&self, profile_id: Uuid) -> Result<Arc<dyn ProtocolAdapter>, TransferError>;
}

#[derive(Default)]
pub struct LocalAdapterFactory {
    profiles: Mutex<HashMap<Uuid, PathBuf>>,
}

impl LocalAdapterFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_local(&self, root: PathBuf) -> Uuid {
        let id = Uuid::new_v4();
        self.profiles
            .lock()
            .expect("local-profiles mutex poisoned")
            .insert(id, root);
        id
    }

    pub fn unregister(&self, id: Uuid) {
        self.profiles
            .lock()
            .expect("local-profiles mutex poisoned")
            .remove(&id);
    }

    pub fn profile_count(&self) -> usize {
        self.profiles
            .lock()
            .expect("local-profiles mutex poisoned")
            .len()
    }
}

#[async_trait]
impl AdapterFactory for LocalAdapterFactory {
    async fn build(&self, profile_id: Uuid) -> Result<Arc<dyn ProtocolAdapter>, TransferError> {
        let root = {
            let guard = self
                .profiles
                .lock()
                .expect("local-profiles mutex poisoned");
            guard.get(&profile_id).cloned()
        }
        .ok_or_else(|| TransferError::Protocol {
            message: format!("local profile not found: {profile_id}"),
        })?;

        let mut adapter = LocalAdapter::new();
        adapter
            .connect(&json!({ "root": root.to_string_lossy().into_owned() }))
            .await?;
        Ok(Arc::new(adapter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn register_then_build_yields_connected_adapter() {
        let dir = tempdir().unwrap();
        let factory = LocalAdapterFactory::new();
        let id = factory.register_local(dir.path().to_path_buf());
        assert_eq!(factory.profile_count(), 1);
        let adapter = factory.build(id).await.expect("build");
        // Capability flags doğru — adapter gerçekten connect oldu.
        let caps = adapter.capabilities();
        assert!(caps.supports_resume);
    }

    #[tokio::test]
    async fn unregister_then_build_returns_protocol_error() {
        let dir = tempdir().unwrap();
        let factory = LocalAdapterFactory::new();
        let id = factory.register_local(dir.path().to_path_buf());
        factory.unregister(id);
        match factory.build(id).await {
            Err(TransferError::Protocol { .. }) => {}
            Ok(_) => panic!("expected Protocol error after unregister"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn build_unknown_profile_id_errors() {
        let factory = LocalAdapterFactory::new();
        match factory.build(Uuid::new_v4()).await {
            Err(TransferError::Protocol { .. }) => {}
            Ok(_) => panic!("expected Protocol error for unknown profile"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
}
