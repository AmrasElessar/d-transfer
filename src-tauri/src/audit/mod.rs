//! Audit Trail + KVKK/GDPR — Bölüm 17.
//!
//! **Opt-in feature.** `AppSettings::audit_enabled` (varsayılan kapalı) ile
//! etkinleştirilir; KVKK/GDPR rıza onayı UI tarafında zorunlu.
//!
//! - [`AuditEngine`] (17.2) — mpsc + 500ms tick + 64-batch flush, `audit.db`'ye
//!   yazar. Fire-and-forget; emit'in başarısızlığı transfer akışını bozmaz.
//! - [`MaskingEngine`] (17.3) — IP/path/filename/username/presigned URL'i
//!   istenirse maske altına alır. Presigned URL **her zaman** redact edilir.
//! - [`Redacted<T>`] (17.3.1) — sensitive field wrapper; Debug impl
//!   `<redacted>` basar, log/diagnostics PII sızdırmaz.

pub mod engine;
pub mod masking;
pub mod redacted;
pub mod schema;

pub use engine::{AuditEngine, AuditEngineError, AuditEvent, AuditEventKind};
pub use masking::MaskingEngine;
pub use redacted::Redacted;
