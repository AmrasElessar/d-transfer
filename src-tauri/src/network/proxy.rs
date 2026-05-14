//! ProxyConfig — Bölüm 37.
//!
//! HTTP/HTTPS/SOCKS5 proxy tarifi. Adapter'lar (S3 reqwest, SFTP CONNECT
//! tüneli) bu tipi okuyup transport'a uygular. Bu modül **type + bypass
//! matcher** kapsamında; gerçek tunnel/transport wiring ilgili adapter'da
//! yapılır.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxyKind {
    Http,
    Https,
    Socks5,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    /// Basic/bearer auth varsa keystore key referansı (`profile_id` +
    /// `kind`). Sırlar bu yapıda **plaintext değildir**.
    pub credential_ref: Option<ProxyCredentialRef>,
    /// Bypass glob pattern listesi. Match olan host proxy'siz çıkar.
    pub bypass_hosts: Vec<String>,
    /// SOCKS5: remote DNS resolve aktif mi (privacy / split-DNS).
    pub dns_through_proxy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyCredentialRef {
    /// Keystore'da hangi profile'a ait.
    pub profile_id: Uuid,
    /// `kind` tipik olarak `proxyBasic` / `proxyBearer` — credentials modülü
    /// ile aynı `KIND_*` paterni; ayrı sabit eklemek için bu modül credential
    /// modülüne dokunmuyor, caller string'i hazırlar.
    pub kind: String,
}

/// `ProxySource` — UI'da kullanıcının seçtiği proxy kaynağı. Adapter
/// `apply_for(host)` çağırdığında bu kaynak `Option<ProxyConfig>`'a inhise
/// eder (bypass match veya `None` proxy kullanma).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "source")]
pub enum ProxySource {
    None,
    /// Windows WinHTTP veya Linux env `HTTP_PROXY` / `HTTPS_PROXY` okuma.
    /// Faz 5 wiring; şu an placeholder.
    System,
    Custom(ProxyConfig),
    /// Sadece belirli profile'a uygula.
    PerProfile {
        profile_id: Uuid,
        config: ProxyConfig,
    },
}

impl Default for ProxySource {
    fn default() -> Self {
        Self::None
    }
}

impl ProxySource {
    /// Verilen `target_host` için kullanılacak `Option<ProxyConfig>`.
    /// Bypass match veya `None`/`System` durumlarında `None` döner.
    ///
    /// `System` şimdilik `None` — gerçek WinHTTP/env okuma Faz 5'te bu
    /// fonksiyonun içine girer. Çağrı semantiği aynı kalır.
    pub fn resolve_for(&self, target_host: &str, target_profile: Option<Uuid>) -> Option<&ProxyConfig> {
        let cfg = match self {
            Self::None | Self::System => return None,
            Self::Custom(c) => c,
            Self::PerProfile { profile_id, config } => {
                if Some(*profile_id) != target_profile {
                    return None;
                }
                config
            }
        };
        if cfg
            .bypass_hosts
            .iter()
            .any(|pattern| bypass_matches(pattern, target_host))
        {
            return None;
        }
        Some(cfg)
    }
}

/// Glob pattern eşleştir — `*` yıldız tek segment veya çok karakter
/// (Windows'taki proxy bypass listesi de aynı semantiği kullanır). Pattern
/// dot-segment akıllıdır: `*.local` `foo.local`'ı yakalar ama
/// `bar.foo.local`'ı yakalamak için pattern başında `*.` gerekir.
/// Spec 37.3 örnekleri:
/// - `*.local`     → `foo.local` ✓, `local` ✗
/// - `192.168.*`   → `192.168.1.10` ✓
/// - `10.*`        → `10.0.0.1` ✓
/// - `localhost`   → `localhost` ✓
pub fn bypass_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim().to_ascii_lowercase();
    let host = host.trim().to_ascii_lowercase();
    if pattern.is_empty() {
        return false;
    }
    if pattern == host {
        return true;
    }
    if !pattern.contains('*') {
        return false;
    }
    // Pattern'i `*` ile ayır; her parçanın sırayla `host` içinde bulunması
    // gerekir, ilk parça başta, son parça sonda.
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut cursor = 0;
    let host_bytes = host.as_bytes();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !host.starts_with(part) {
                return false;
            }
            cursor = part.len();
        } else if i == parts.len() - 1 {
            if !host.ends_with(part) {
                return false;
            }
            // Last part conflict check: cursor'a hâlâ yer var mı.
            if host.len() < cursor + part.len() {
                return false;
            }
            return true;
        } else {
            // Orta parça — cursor'dan itibaren ara.
            if let Some(pos) = find_from(host_bytes, part.as_bytes(), cursor) {
                cursor = pos + part.len();
            } else {
                return false;
            }
        }
    }
    // Hepsi `*` ise (pattern = "*"), bütün host'lar yakalanır.
    true
}

fn find_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(bypass_matches("localhost", "localhost"));
        assert!(bypass_matches("localhost", "LocalHost"));
        assert!(!bypass_matches("localhost", "localhost.example"));
    }

    #[test]
    fn dot_local_pattern() {
        assert!(bypass_matches("*.local", "foo.local"));
        assert!(bypass_matches("*.local", "bar.foo.local"));
        assert!(!bypass_matches("*.local", "local"));
    }

    #[test]
    fn prefix_glob() {
        assert!(bypass_matches("192.168.*", "192.168.1.10"));
        assert!(!bypass_matches("192.168.*", "10.0.0.1"));
    }

    #[test]
    fn middle_glob() {
        assert!(bypass_matches("api.*.example.com", "api.eu.example.com"));
        assert!(!bypass_matches("api.*.example.com", "web.eu.example.com"));
    }

    #[test]
    fn wildcard_only() {
        assert!(bypass_matches("*", "anything"));
    }

    #[test]
    fn empty_pattern_never_matches() {
        assert!(!bypass_matches("", "host"));
    }

    #[test]
    fn proxy_source_resolves_bypass_to_none() {
        let cfg = ProxyConfig {
            kind: ProxyKind::Http,
            host: "proxy.local".into(),
            port: 8080,
            credential_ref: None,
            bypass_hosts: vec!["*.local".into(), "localhost".into()],
            dns_through_proxy: false,
        };
        let src = ProxySource::Custom(cfg);
        assert!(src.resolve_for("foo.local", None).is_none());
        assert!(src.resolve_for("localhost", None).is_none());
        assert!(src.resolve_for("api.dropbox.com", None).is_some());
    }

    #[test]
    fn proxy_source_per_profile_only_matches_target() {
        let pid = Uuid::new_v4();
        let cfg = ProxyConfig {
            kind: ProxyKind::Socks5,
            host: "proxy.example".into(),
            port: 1080,
            credential_ref: None,
            bypass_hosts: vec![],
            dns_through_proxy: true,
        };
        let src = ProxySource::PerProfile {
            profile_id: pid,
            config: cfg,
        };
        assert!(src.resolve_for("api.dropbox.com", Some(pid)).is_some());
        assert!(src.resolve_for("api.dropbox.com", Some(Uuid::new_v4())).is_none());
        assert!(src.resolve_for("api.dropbox.com", None).is_none());
    }

    #[test]
    fn system_source_currently_returns_none() {
        let src = ProxySource::System;
        assert!(src.resolve_for("any", None).is_none());
    }
}
