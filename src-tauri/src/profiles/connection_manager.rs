//! `ConnectionManager` — persistent adapter havuzu (Faz 4).
//!
//! `AdapterFactory` Faz 3'te transfer dispatch path'i için `register_local()`'a
//! bağlı, transient bir kayıt sistemi (her transfer yeni profile_id alır,
//! sonunda unregister). Faz 4'te UI doğrudan remote browse istiyor:
//! "active profile seçildiğinde anında bağlan, navigation sırasında bağlantıyı
//! koru, kullanıcı profile değiştirince eskisini bırak". Her listing'de
//! yeniden connect = SSH TCP+auth handshake = saniyeler / pahalı; cache şart.
//!
//! Bu modül `AdapterFactory` katmanından bilinçli olarak ayrı yaşar:
//! - Factory: profile_id (transient) → in-memory LocalAdapter config; transfer
//!   scheduler tarafından.
//! - Manager: profile_id (kalıcı DB row) → connect-edilmiş adapter; UI tarafından.
//!
//! Faz 5'te ikisi birleşip ortak bir "profile-scoped adapter pool" haline gelir;
//! şimdilik scope ayrımı temiz tutuyor.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::errors::TransferError;
use crate::protocols::{LocalAdapter, ProtocolAdapter, SftpAdapter};

use super::connection_profile::{ConnectionProfile, ProfileProtocol};
use super::credentials::{CredentialVault, KIND_PASSWORD};

/// Aktif bağlantı havuzu — `profile_id` → connect edilmiş adapter.
///
/// Faz 4 davranışı:
/// - Connect **lazy**: ilk `get_or_connect` çağrısında kurulur, sonraki çağrılar
///   cache'lenen `Arc<dyn ProtocolAdapter>`'ı paylaşır.
/// - Disconnect **explicit**: UI çağırırsa (`disconnect_profile` IPC) veya app
///   shutdown ile drop. Idle timeout yok (Faz 5'te configurable).
/// - **Race**: aynı profile_id'ye paralel iki çağrı varsa, dış mutex'i kısa
///   tutmak için `build_and_connect`'i lock dışında çalıştırıyoruz. İkinci
///   çağrı geldiğinde lock yeniden alındığında `or_insert` victim'i seçer —
///   loser tarafın connect'i drop olur (connection sızıntısı yok çünkü Arc
///   sadece yerel `adapter`'a tutuluyor; map'e konulan winner versiyonu).
pub struct ConnectionManager {
    inner: Mutex<HashMap<Uuid, Arc<dyn ProtocolAdapter>>>,
    vault: Arc<CredentialVault>,
}

impl ConnectionManager {
    pub fn new(vault: Arc<CredentialVault>) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            vault,
        }
    }

    /// Profile için cached adapter döndür; yoksa connect edip cache'le.
    pub async fn get_or_connect(
        &self,
        profile: &ConnectionProfile,
    ) -> Result<Arc<dyn ProtocolAdapter>, TransferError> {
        // Hızlı yol: zaten bağlıysa lock'u serbest bırakıp dön.
        {
            let guard = self.inner.lock().await;
            if let Some(adapter) = guard.get(&profile.id) {
                return Ok(Arc::clone(adapter));
            }
        }

        // Connect SSH/TCP'ye dokunabilir — saniyeler sürebilir. Mutex'i bırakıp
        // çalış, sonra winner-takes-all ile cache'e koy.
        let adapter = build_and_connect(profile, &self.vault).await?;

        let mut guard = self.inner.lock().await;
        // Eğer aramızdaki yarış sırasında başka bir çağrı bizden önce kayıt
        // attıysa onun adapter'ını kullan — bizimkini drop et (RAII close).
        let entry = guard.entry(profile.id).or_insert(adapter);
        Ok(Arc::clone(entry))
    }

    /// Cache'den çıkar — son referans düşünce adapter drop olur ve TCP/SSH
    /// kapanır. Halen list_dir streaming'de olan future'lar Arc'a sahip olduğu
    /// için işlerini bitirir; gerçek socket close `Arc::strong_count == 1`
    /// hâline gelince RAII üzerinden yürür. Bu Faz 4 için kabul edilen yarış
    /// davranışı; Faz 5'te explicit cancellation token threading gelecek.
    pub async fn disconnect(&self, profile_id: Uuid) -> Result<(), TransferError> {
        let removed = {
            let mut guard = self.inner.lock().await;
            guard.remove(&profile_id)
        };
        // Adapter trait `disconnect` `&mut self` ister; `Arc<dyn>` hold ediyoruz
        // bu yüzden çağırma — drop yeterli. russh client connection cleanup
        // Drop impl'i içinde async runtime'a spawn ediliyor; LocalAdapter için
        // sadece root path düşer.
        drop(removed);
        Ok(())
    }

    /// Debug/diagnostics: o anki bağlı profil id listesi.
    pub async fn connected_profile_ids(&self) -> Vec<Uuid> {
        self.inner.lock().await.keys().copied().collect()
    }
}

async fn build_and_connect(
    profile: &ConnectionProfile,
    vault: &CredentialVault,
) -> Result<Arc<dyn ProtocolAdapter>, TransferError> {
    match profile.protocol {
        ProfileProtocol::Local => {
            let root = profile.local_root.clone().ok_or_else(|| TransferError::Protocol {
                message: "local profile missing localRoot".into(),
            })?;
            let mut adapter = LocalAdapter::new();
            adapter
                .connect(&json!({ "root": root.to_string_lossy().into_owned() }))
                .await?;
            Ok(Arc::new(adapter))
        }
        ProfileProtocol::Sftp => {
            let host = profile.host.clone().ok_or_else(|| TransferError::Protocol {
                message: "sftp profile missing host".into(),
            })?;
            let username = profile.username.clone().ok_or_else(|| TransferError::Protocol {
                message: "sftp profile missing username".into(),
            })?;
            // Vault read başarısız olursa Protocol değil Authentication kategori
            // tercih edilebilirdi, ama keystore arızası (D-Bus down) saf kimlik
            // hatası değil — operasyonel hata. Protocol message'da reason taşıyor.
            let password = vault
                .fetch(profile.id, KIND_PASSWORD)
                .map_err(|e| TransferError::Protocol {
                    message: format!("vault read failed: {e}"),
                })?;

            let mut adapter = SftpAdapter::new();
            // Faz 5: private_key_pem, private_key_passphrase, known_host_fingerprint
            // options_json üzerinden okunup payload'a eklenecek. Şimdilik password-only.
            adapter
                .connect(&json!({
                    "host": host,
                    "port": profile.port.unwrap_or(22),
                    "username": username,
                    "password": password,
                    "remote_root": profile.remote_root.clone().unwrap_or_else(|| ".".into()),
                }))
                .await?;
            Ok(Arc::new(adapter))
        }
        other => Err(TransferError::CapabilityNotSupported {
            capability: format!("{} protocol not yet wired in ConnectionManager", other.as_str()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_local_profile(root: std::path::PathBuf) -> ConnectionProfile {
        ConnectionProfile {
            id: Uuid::new_v4(),
            name: "test-local".into(),
            protocol: ProfileProtocol::Local,
            host: None,
            port: None,
            username: None,
            remote_root: None,
            local_root: Some(root),
            auth_method: super::super::AuthMethod::None,
            options_json: "{}".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn local_profile_connects_and_caches() {
        let dir = tempdir().unwrap();
        let profile = make_local_profile(dir.path().to_path_buf());
        let vault = Arc::new(CredentialVault::new());
        let manager = ConnectionManager::new(vault);

        let first = manager.get_or_connect(&profile).await.expect("first connect");
        let second = manager.get_or_connect(&profile).await.expect("second connect");

        // Cache HIT — aynı Arc adresi.
        assert!(
            Arc::ptr_eq(&first, &second),
            "second call should return cached Arc"
        );

        // Capability sağlam — adapter gerçekten connect oldu (yoksa root None'da kalırdı).
        let caps = first.capabilities();
        assert!(caps.supports_resume);

        let ids = manager.connected_profile_ids().await;
        assert_eq!(ids, vec![profile.id]);
    }

    #[tokio::test]
    async fn disconnect_clears_cache() {
        let dir = tempdir().unwrap();
        let profile = make_local_profile(dir.path().to_path_buf());
        let vault = Arc::new(CredentialVault::new());
        let manager = ConnectionManager::new(vault);

        let first = manager.get_or_connect(&profile).await.expect("connect");
        manager.disconnect(profile.id).await.expect("disconnect");

        assert!(manager.connected_profile_ids().await.is_empty());

        let third = manager.get_or_connect(&profile).await.expect("reconnect");
        // disconnect sonrası fresh adapter — eski Arc ile aynı olmamalı.
        assert!(
            !Arc::ptr_eq(&first, &third),
            "after disconnect, new connect must yield a fresh adapter"
        );
    }

    #[tokio::test]
    async fn missing_local_root_errors() {
        let mut profile = make_local_profile(std::path::PathBuf::from("/tmp"));
        profile.local_root = None;
        let vault = Arc::new(CredentialVault::new());
        let manager = ConnectionManager::new(vault);

        match manager.get_or_connect(&profile).await {
            Err(TransferError::Protocol { message }) => {
                assert!(message.contains("localRoot"), "got: {message}");
            }
            Err(other) => panic!("expected Protocol error, got {other}"),
            Ok(_) => panic!("expected Protocol error, got connected adapter"),
        }
    }

    // SFTP testleri gerçek bir server gerektirir; ignored bırakıyoruz —
    // local dev'de `cargo test -- --ignored` ile çalıştırılabilir.
    #[tokio::test]
    #[ignore = "requires reachable SFTP server"]
    async fn sftp_profile_connects() {
        // Placeholder — manuel manual smoke test için iskelet.
    }
}
