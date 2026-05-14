//! `ConnectionProfile` — kullanıcı tanımlı kalıcı bağlantı şablonu (Bölüm 25).
//!
//! Bir profil tek bir backend hedefini tarif eder: protokol türü + bağlantı
//! parametreleri (host, port, kullanıcı adı, kök dizinler) + auth metodu. Sır
//! değerler (parola, private-key passphrase) **buraya yazılmaz** — OS keystore
//! üzerinden `CredentialVault` yönetir (Bölüm 25.1). Bu sınır:
//!
//! 1. JSON dump/backup ihracında sırların yanlışlıkla diske yazılmasını engeller.
//! 2. Profil DB'sinin tehlikeye atıldığı senaryoda kimlik bilgileri OS keystore
//!    katmanında izole kalır.
//! 3. UI'ın "şifremi göster" ile keystore arasında temiz bir bağlam ayrımı yapar.
//!
//! `options_json` protokole özgü ekstra anahtarlar için serbest JSON tutar
//! (örn. SFTP host-key strategy, S3 region/endpoint, WebDAV TLS skip). Şu an
//! validate edilmiyor; protocol adapter Faz 4'te kendi şemasını uygulayacak.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileProtocol {
    Local,
    Sftp,
    S3,
    Webdav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    /// Anonim / parolasız (örn. local FS, public WebDAV).
    None,
    Password,
    PublicKey,
}

/// Kalıcı bağlantı profili. `created_at`/`updated_at` UTC; tüm DateTime kolonları
/// RFC3339 string olarak SQLite'a yazılır.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: Uuid,
    pub name: String,
    pub protocol: ProfileProtocol,
    /// Local FS profili için `None`. SFTP/S3/WebDAV'de zorunlu.
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    /// SFTP/WebDAV başlangıç dizini (`/var/www`, `/`, …). Opsiyonel — boş kalırsa
    /// adapter kendi varsayılanını kullanır.
    pub remote_root: Option<String>,
    /// Local protocol için root path; uzak protokoller için `None`.
    pub local_root: Option<PathBuf>,
    pub auth_method: AuthMethod,
    /// Protokole özgü serbest JSON. `{}` legaldir.
    pub options_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProfileProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileProtocol::Local => "local",
            ProfileProtocol::Sftp => "sftp",
            ProfileProtocol::S3 => "s3",
            ProfileProtocol::Webdav => "webdav",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "local" => ProfileProtocol::Local,
            "sftp" => ProfileProtocol::Sftp,
            "s3" => ProfileProtocol::S3,
            "webdav" => ProfileProtocol::Webdav,
            _ => return None,
        })
    }
}

impl AuthMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthMethod::None => "none",
            AuthMethod::Password => "password",
            AuthMethod::PublicKey => "publicKey",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "none" => AuthMethod::None,
            "password" => AuthMethod::Password,
            "publicKey" => AuthMethod::PublicKey,
            _ => return None,
        })
    }
}
