//! MaskingEngine — Bölüm 17.3.
//!
//! Audit log'a yazılırken alanlara uygulanacak granüler maske politikası.
//! Kullanıcı UI'dan tek tek toggle eder; presigned URL **her zaman** redact
//! edilir (credential içerir) — bu kural `redact_presigned_url` field'ı
//! UI'da kapatılabilir görünse bile audit writer override eder.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskingEngine {
    pub mask_ip: bool,
    pub mask_path: bool,
    pub mask_filename: bool,
    pub mask_username: bool,
    /// Spec kuralı: her zaman `true`. UI'da görünse bile audit writer
    /// `mask_url_if_presigned()` çağrısında bu field'ı zorlar.
    pub redact_presigned_url: bool,
}

impl Default for MaskingEngine {
    fn default() -> Self {
        Self {
            // Konservatif defaults — opt-in audit zaten compliance odaklı.
            mask_ip: true,
            mask_path: true,
            mask_filename: false,
            mask_username: false,
            redact_presigned_url: true,
        }
    }
}

impl MaskingEngine {
    /// IPv4 → `a.b.*.*`, IPv6 → ilk iki segment + `::`.
    pub fn mask_ip(&self, ip: &str) -> String {
        if !self.mask_ip {
            return ip.to_string();
        }
        if ip.contains(':') {
            // IPv6 — ilk iki segment + `::`.
            let mut parts = ip.split(':');
            let a = parts.next().unwrap_or_default();
            let b = parts.next().unwrap_or_default();
            return format!("{a}:{b}::");
        }
        let octets: Vec<&str> = ip.split('.').collect();
        if octets.len() != 4 {
            return ip.to_string();
        }
        format!("{}.{}.*.*", octets[0], octets[1])
    }

    /// `/home/john/secret/file.txt` → `/home/<user>/<masked>/<masked>`. Tam
    /// path kayboluyor ama depth korunuyor (debug için bilgi).
    pub fn mask_path(&self, path: &str) -> String {
        if !self.mask_path {
            return path.to_string();
        }
        let trim = path.trim_start_matches('/');
        let parts: Vec<&str> = trim.split('/').collect();
        let mut out = String::from(if path.starts_with('/') { "/" } else { "" });
        for (i, _) in parts.iter().enumerate() {
            if i > 0 {
                out.push('/');
            }
            if i == 0 && (parts[0] == "home" || parts[0] == "Users") {
                out.push_str(parts[0]);
                continue;
            }
            out.push_str("<masked>");
        }
        out
    }

    /// `report-2026.pdf` → `<masked>.pdf` (extension korunur).
    pub fn mask_filename(&self, name: &str) -> String {
        if !self.mask_filename {
            return name.to_string();
        }
        match name.rfind('.') {
            Some(dot) if dot > 0 => format!("<masked>{}", &name[dot..]),
            _ => "<masked>".into(),
        }
    }

    pub fn mask_username(&self, user: &str) -> String {
        if !self.mask_username {
            return user.to_string();
        }
        match user.find('@') {
            Some(at) => format!("<masked>{}", &user[at..]),
            None => "<masked>".into(),
        }
    }

    /// **Override kuralı:** URL `?` veya `&` query-string'i olan ve içinde
    /// signature anahtarı görünen tipik AWS / Azure / GCS presigned formdaysa
    /// her durumda `[PRESIGNED_URL_REDACTED]` döner.
    pub fn mask_url_if_presigned(&self, url: &str) -> String {
        if looks_like_presigned(url) || self.redact_presigned_url && url.contains('?') {
            return "[PRESIGNED_URL_REDACTED]".into();
        }
        url.to_string()
    }
}

/// Heuristic: presigned URL'lerde tipik `Signature=` / `X-Amz-Signature=` /
/// `sig=` / `sv=` / `se=` (Azure SAS) query parametreleri görülür.
fn looks_like_presigned(url: &str) -> bool {
    const MARKERS: &[&str] = &[
        "X-Amz-Signature",
        "Signature=",
        "X-Goog-Signature",
        "sig=",
        "&se=",
        "?se=",
        "X-Amz-SignedHeaders",
    ];
    MARKERS.iter().any(|m| url.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_ipv4_to_two_octets() {
        let m = MaskingEngine::default();
        assert_eq!(m.mask_ip("192.168.1.10"), "192.168.*.*");
    }

    #[test]
    fn masks_ipv6_to_two_segments() {
        let m = MaskingEngine::default();
        assert_eq!(m.mask_ip("2001:db8::1"), "2001:db8::");
    }

    #[test]
    fn mask_path_preserves_home_root() {
        let m = MaskingEngine::default();
        assert_eq!(
            m.mask_path("/home/john/secret/file.txt"),
            "/home/<masked>/<masked>/<masked>"
        );
    }

    #[test]
    fn mask_filename_preserves_extension() {
        let mut m = MaskingEngine::default();
        m.mask_filename = true;
        assert_eq!(m.mask_filename("report-2026.pdf"), "<masked>.pdf");
        assert_eq!(m.mask_filename("noext"), "<masked>");
    }

    #[test]
    fn mask_username_keeps_email_domain() {
        let mut m = MaskingEngine::default();
        m.mask_username = true;
        assert_eq!(
            m.mask_username("john.doe@example.com"),
            "<masked>@example.com"
        );
        assert_eq!(m.mask_username("alice"), "<masked>");
    }

    #[test]
    fn presigned_url_always_redacted_regardless_of_flag() {
        let m = MaskingEngine {
            redact_presigned_url: false,
            ..Default::default()
        };
        let url =
            "https://s3.amazonaws.com/bucket/key?X-Amz-Signature=deadbeef&X-Amz-SignedHeaders=host";
        // `redact_presigned_url` false olsa bile heuristik yakalar — spec
        // 17.3 "her zaman true" davranışını override.
        assert_eq!(m.mask_url_if_presigned(url), "[PRESIGNED_URL_REDACTED]");
    }

    #[test]
    fn plain_url_passes_through() {
        let m = MaskingEngine {
            redact_presigned_url: true,
            ..Default::default()
        };
        // Query-string'i olmayan URL presigned değil — geçer.
        assert_eq!(
            m.mask_url_if_presigned("https://example.com/file.txt"),
            "https://example.com/file.txt"
        );
    }

    #[test]
    fn disabling_each_mask_returns_input() {
        let m = MaskingEngine {
            mask_ip: false,
            mask_path: false,
            mask_filename: false,
            mask_username: false,
            redact_presigned_url: false,
        };
        assert_eq!(m.mask_ip("1.2.3.4"), "1.2.3.4");
        assert_eq!(m.mask_path("/foo/bar"), "/foo/bar");
        assert_eq!(m.mask_filename("x.zip"), "x.zip");
        assert_eq!(m.mask_username("user@x"), "user@x");
    }
}
