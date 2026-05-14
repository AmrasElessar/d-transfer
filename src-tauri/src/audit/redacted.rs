//! `Redacted<T>` — sensitive field wrapper (Bölüm 17.3.1).
//!
//! `tracing::debug!(event = ?event, ...)` her event'in `Debug` impl'ini
//! stringify eder. Diagnostics bundle export'unda log dosyaları kullanıcıya /
//! support'a açılır; bu yol üzerinden parola/path/email gibi PII sızar.
//!
//! `Redacted<T>` aynı `Clone` / `PartialEq` semantik özellikleri korur, ama
//! `Debug` impl'i hiçbir zaman içeriği basmaz. Inner değere erişmek için
//! `.expose()` çağırılır — call site kasıtlı olarak görünür kalır.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Sensitive bir değeri saran wrapper. Sadece `Debug` impl'i overrride
/// edilir — `Serialize` raw değeri yansıtır çünkü IPC payload'ları
/// tarafında redact kararı caller'a aittir.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// İçerikteki ham değere kasıtlı erişim.
    pub fn expose(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<redacted>")
    }
}

impl<T> From<T> for Redacted<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_leaks_inner() {
        let r: Redacted<String> = "supersecret".to_string().into();
        let dbg = format!("{:?}", r);
        assert_eq!(dbg, "<redacted>");
        assert!(!dbg.contains("supersecret"));
    }

    #[test]
    fn expose_returns_inner_reference() {
        let r = Redacted::new(42_u32);
        assert_eq!(*r.expose(), 42);
    }

    #[test]
    fn serialize_transparent() {
        let r = Redacted::new("hello".to_string());
        let json = serde_json::to_string(&r).unwrap();
        // `transparent` → outer wrapper yok, içerik plain string.
        assert_eq!(json, "\"hello\"");
    }
}
