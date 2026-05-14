//! `CredentialVault` — OS keystore aracılığıyla sır yönetimi (Bölüm 25.1).
//!
//! Profilin kendisi (host, port, username, options) `queue.db` içinde plaintext
//! durur — paylaşılabilir, backup'lanabilir. Sır değerleri (parola, private-key
//! passphrase) ise platforma özgü güvenli storage'a yazılır:
//!
//! - **Windows**: Credential Manager (DPAPI ile şifrelenir).
//! - **macOS**: Keychain.
//! - **Linux**: Secret Service API (D-Bus, GNOME Keyring / KWallet).
//!
//! Account key her sır için `"{profile_id}:{kind}"` formundadır; bu sayede aynı
//! profile birden fazla sır (örn. `password` + `private-key-passphrase`)
//! bağlanabilir, silinirken tek tek hedeflenir.
//!
//! ## Linux fallback (Bölüm 25.1.3)
//!
//! D-Bus erişimi olmayan ortamlarda (Alpine, headless WSL2, container)
//! `keyring::Error::PlatformFailure` döner. Faz 1'de bu hatayı upstream'e
//! propagate ediyoruz; UI kullanıcıyı uyarır. **TODO (Faz 5)**: Argon2id + XChaCha20
//! ile şifrelenmiş file-backed fallback, master parola UI ile birlikte gelecek.

use keyring::Entry;
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

const SERVICE_NAME: &str = "DTransfer";

#[derive(Debug, Error)]
pub enum CredentialError {
    /// OS keystore erişilemedi (D-Bus yok, locked keychain, vb.). Faz 5'te file
    /// backend ile yumuşatılacak; şimdilik hard fail.
    #[error("OS keystore unavailable: {0}")]
    PlatformUnavailable(String),

    /// Keystore'a yazma/okuma sırasında diğer hata. Genelde permission denied
    /// veya keyring kilidi (kullanıcı keyring şifresini girmemiş).
    #[error("keystore error: {0}")]
    Backend(String),
}

impl From<keyring::Error> for CredentialError {
    fn from(err: keyring::Error) -> Self {
        match err {
            keyring::Error::PlatformFailure(e) => {
                CredentialError::PlatformUnavailable(e.to_string())
            }
            keyring::Error::NoStorageAccess(e) => {
                CredentialError::PlatformUnavailable(e.to_string())
            }
            other => CredentialError::Backend(other.to_string()),
        }
    }
}

/// `Clone` ucuz — vault stateless, `Entry`'ler her çağrıda yeniden oluşturulur.
/// Bu seçimin nedeni: keyring `Entry` thread-safe değil (Linux'ta D-Bus bağlamı
/// per-thread), bu yüzden vault'u Arc'lemek yerine fonksiyon başına yeni Entry
/// kuruyoruz. Maliyet düşük; yalnızca profil CRUD anında çağrılır.
#[derive(Debug, Clone, Default)]
pub struct CredentialVault;

impl CredentialVault {
    pub fn new() -> Self {
        Self
    }

    /// Sırrı keystore'a yazar. Mevcut değer üzerine yazılır.
    pub fn store(
        &self,
        profile_id: Uuid,
        kind: &str,
        value: &str,
    ) -> Result<(), CredentialError> {
        let account = account_key(profile_id, kind);
        let entry = Entry::new(SERVICE_NAME, &account)?;
        entry.set_password(value)?;
        Ok(())
    }

    /// Sırrı keystore'dan okur. Kayıt yoksa `Ok(None)` döner — `Err` yalnızca
    /// gerçek bir backend arızasında (D-Bus down, permission denied) gelir.
    pub fn fetch(
        &self,
        profile_id: Uuid,
        kind: &str,
    ) -> Result<Option<String>, CredentialError> {
        let account = account_key(profile_id, kind);
        let entry = Entry::new(SERVICE_NAME, &account)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(other) => Err(other.into()),
        }
    }

    /// Sırrı keystore'dan siler. Kayıt yoksa sessizce başarılı sayar — caller
    /// "profil silindi, sırlar da gitsin" akışında race'leri tolere edebilir.
    pub fn delete(&self, profile_id: Uuid, kind: &str) -> Result<(), CredentialError> {
        let account = account_key(profile_id, kind);
        let entry = Entry::new(SERVICE_NAME, &account)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(other) => Err(other.into()),
        }
    }

    /// Profil silinirken bilinen tüm sır türlerini best-effort temizler.
    /// Hata olursa warn log basıp devam eder; UI akışını bloklamaz.
    pub fn purge_all_known_kinds(&self, profile_id: Uuid) {
        for kind in KNOWN_SECRET_KINDS {
            if let Err(e) = self.delete(profile_id, kind) {
                warn!(?profile_id, kind, ?e, "credential purge failed");
            }
        }
    }
}

fn account_key(profile_id: Uuid, kind: &str) -> String {
    format!("{profile_id}:{kind}")
}

/// UI/IPC katmanında kullanılan sır türleri için kanonik isim listesi.
/// Yeni türler buraya eklenir → `purge_all_known_kinds` otomatik toplar.
pub const KIND_PASSWORD: &str = "password";
pub const KIND_PRIVATE_KEY_PASSPHRASE: &str = "private-key-passphrase";

const KNOWN_SECRET_KINDS: &[&str] = &[KIND_PASSWORD, KIND_PRIVATE_KEY_PASSPHRASE];

#[cfg(test)]
mod tests {
    use super::*;

    // Bu testler gerçek OS keystore'a yazar; CI'da D-Bus / Keychain
    // muhtemelen yok. Lokal ortamda `cargo test -- --ignored` ile çalıştırılır.
    // Mock keyring backend Faz 5'te (file fallback ile birlikte) gelecek.

    #[test]
    #[ignore = "requires OS keystore access — manual run only"]
    fn store_fetch_delete_roundtrip() {
        let vault = CredentialVault::new();
        let id = Uuid::new_v4();
        vault.store(id, KIND_PASSWORD, "secret-value").unwrap();
        let got = vault.fetch(id, KIND_PASSWORD).unwrap();
        assert_eq!(got.as_deref(), Some("secret-value"));
        vault.delete(id, KIND_PASSWORD).unwrap();
        let after = vault.fetch(id, KIND_PASSWORD).unwrap();
        assert!(after.is_none());
    }

    #[test]
    #[ignore = "requires OS keystore access — manual run only"]
    fn fetch_missing_returns_none() {
        let vault = CredentialVault::new();
        let id = Uuid::new_v4();
        let got = vault.fetch(id, KIND_PASSWORD).unwrap();
        assert!(got.is_none());
    }

    #[test]
    #[ignore = "requires OS keystore access — manual run only"]
    fn delete_missing_is_ok() {
        let vault = CredentialVault::new();
        let id = Uuid::new_v4();
        vault.delete(id, KIND_PASSWORD).unwrap();
    }
}
