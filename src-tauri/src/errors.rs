//! Structured Error Taxonomy — Bölüm 10.
//!
//! `anyhow` transport layer'da kalır; UI kararları için domain error modeli şart.
//! UI bu enum'a göre retry/refresh/pause kararı verir (Bölüm 10 tablo).

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    // ---------------- Kimlik doğrulama ----------------
    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    #[error("Authorization denied: {path}")]
    Authorization { path: String },

    // ---------------- Ağ ----------------
    #[error("Connection lost after {bytes_sent} bytes")]
    ConnectionLost { bytes_sent: u64 },

    #[error("Connection timeout after {elapsed_ms}ms")]
    Timeout { elapsed_ms: u64 },

    // ---------------- Dosya sistemi ----------------
    #[error("Disk full: {available_bytes} bytes available")]
    DiskFull { available_bytes: u64 },

    #[error("Remote file locked: {path}")]
    RemoteLocked { path: String },

    #[error("Not found: {path}")]
    NotFound { path: String },

    // ---------------- Transfer bütünlüğü ----------------
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Encryption failure: {reason}")]
    EncryptionFailure { reason: String },

    // ---------------- API / Rate limit ----------------
    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("API quota exceeded")]
    QuotaExceeded,

    #[error("Presigned URL expired")]
    UrlExpired,

    // ---------------- Adapter ----------------
    #[error("Adapter capability not supported: {capability}")]
    CapabilityNotSupported { capability: String },

    #[error("Protocol error: {message}")]
    Protocol { message: String },

    // ---------------- Lifecycle ----------------
    #[error("Cancelled by user or parent token")]
    Cancelled,

    // ---------------- Sistem ----------------
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Yüksek-seviye kategori — i18n anahtarlarıyla 1:1 eşleşir (`errors.network`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCategory {
    Network,
    Auth,
    Permission,
    NotFound,
    Conflict,
    RateLimit,
    ServerError,
    Integrity,
    Cancelled,
    Unknown,
}

/// UI tarafına önerilen aksiyon — Bölüm 10 tablosundan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestedAction {
    /// Kullanıcıya kimlik bilgisi sor.
    PromptCredentials,
    /// Otomatik retry — backoff ile.
    AutoRetry,
    /// Belirtilen süreyi bekle, sonra otomatik devam.
    WaitAndRetry { seconds: u64 },
    /// Presigned URL yenile (refresh) ve transferi tekrar başlat.
    RefreshUrl,
    /// Transferi duraklat, kullanıcıya bildir.
    PauseAndNotify,
    /// Yeniden indir/yükle.
    Redo,
    /// UI bilgi göster, kullanıcı kararı bekle.
    UserDecision,
    /// İçe gömülü iptal — başka aksiyon yok.
    None,
}

impl TransferError {
    pub fn category(&self) -> ErrorCategory {
        use TransferError as E;
        match self {
            E::Authentication { .. } => ErrorCategory::Auth,
            E::Authorization { .. } => ErrorCategory::Permission,
            E::ConnectionLost { .. } | E::Timeout { .. } => ErrorCategory::Network,
            E::DiskFull { .. } => ErrorCategory::ServerError,
            E::RemoteLocked { .. } => ErrorCategory::Conflict,
            E::NotFound { .. } => ErrorCategory::NotFound,
            E::ChecksumMismatch { .. } | E::EncryptionFailure { .. } => ErrorCategory::Integrity,
            E::RateLimited { .. } | E::QuotaExceeded => ErrorCategory::RateLimit,
            E::UrlExpired => ErrorCategory::Auth,
            E::CapabilityNotSupported { .. } | E::Protocol { .. } => ErrorCategory::ServerError,
            E::Cancelled => ErrorCategory::Cancelled,
            E::Io(_) => ErrorCategory::Unknown,
        }
    }

    pub fn suggested_action(&self) -> SuggestedAction {
        use TransferError as E;
        match self {
            E::Authentication { .. } => SuggestedAction::PromptCredentials,
            E::Authorization { .. } => SuggestedAction::UserDecision,
            E::ConnectionLost { .. } | E::Timeout { .. } => SuggestedAction::AutoRetry,
            E::DiskFull { .. } | E::RemoteLocked { .. } => SuggestedAction::PauseAndNotify,
            E::NotFound { .. } => SuggestedAction::UserDecision,
            E::ChecksumMismatch { .. } | E::EncryptionFailure { .. } => SuggestedAction::Redo,
            E::RateLimited { retry_after_secs } => SuggestedAction::WaitAndRetry {
                seconds: *retry_after_secs,
            },
            E::QuotaExceeded => SuggestedAction::PauseAndNotify,
            E::UrlExpired => SuggestedAction::RefreshUrl,
            E::CapabilityNotSupported { .. } | E::Protocol { .. } => SuggestedAction::UserDecision,
            E::Cancelled => SuggestedAction::None,
            E::Io(_) => SuggestedAction::AutoRetry,
        }
    }

    /// i18n key suffix — `errors.{this}` template'i için.
    pub fn i18n_key(&self) -> &'static str {
        match self.category() {
            ErrorCategory::Network => "network",
            ErrorCategory::Auth => "auth",
            ErrorCategory::Permission => "permission",
            ErrorCategory::NotFound => "notFound",
            ErrorCategory::Conflict => "conflict",
            ErrorCategory::RateLimit => "rateLimit",
            ErrorCategory::ServerError => "serverError",
            ErrorCategory::Integrity => "integrity",
            ErrorCategory::Cancelled => "cancelled",
            ErrorCategory::Unknown => "unknown",
        }
    }
}

/// Tauri IPC üzerinden UI'a serialize edilebilir form.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireError {
    pub category: ErrorCategory,
    pub suggested_action: SuggestedAction,
    pub i18n_key: &'static str,
    pub message: String,
}

impl From<&TransferError> for WireError {
    fn from(err: &TransferError) -> Self {
        WireError {
            category: err.category(),
            suggested_action: err.suggested_action(),
            i18n_key: err.i18n_key(),
            message: err.to_string(),
        }
    }
}

impl serde::Serialize for TransferError {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        WireError::from(self).serialize(ser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_categorizes_correctly() {
        let err = TransferError::Authentication {
            reason: "bad password".into(),
        };
        assert_eq!(err.category(), ErrorCategory::Auth);
        assert_eq!(err.suggested_action(), SuggestedAction::PromptCredentials);
        assert_eq!(err.i18n_key(), "auth");
    }

    #[test]
    fn rate_limit_propagates_retry_after() {
        let err = TransferError::RateLimited {
            retry_after_secs: 30,
        };
        match err.suggested_action() {
            SuggestedAction::WaitAndRetry { seconds } => assert_eq!(seconds, 30),
            other => panic!("unexpected action: {:?}", other),
        }
    }

    #[test]
    fn wire_error_serializes_camel_case() {
        let err = TransferError::Timeout { elapsed_ms: 5_000 };
        let wire = WireError::from(&err);
        let json = serde_json::to_string(&wire).unwrap();
        assert!(json.contains("\"category\":\"network\""));
        assert!(json.contains("\"suggestedAction\":\"autoRetry\""));
        assert!(json.contains("\"i18nKey\":\"network\""));
    }
}
