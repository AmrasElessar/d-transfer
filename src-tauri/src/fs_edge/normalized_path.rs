//! NFC-normalized path newtype — Bölüm 12.2.
//!
//! macOS APFS NFD üretir, Windows/Linux çoğu zaman NFC. Aynı görsel filename iki
//! farklı byte sequence ile (`ş` = U+015F vs `s` + U+0327) gelirse, byte-exact
//! karşılaştırma sync engine'i bozar: duplicate detection patlar, conflict
//! resolution iki ayrı dosya görür. Bu sebeple **internal compare daima NFC**
//! üzerinden yapılır; ham byte sequence disk I/O için saklanır (server'a
//! NFC'ye çevrilmiş halde yazarsak server kafası karışır).

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

/// Disk I/O için ham path + internal compare için NFC normalize form.
///
/// `PartialEq`/`Eq`/`Hash` daima NFC üzerinden çalışır — bu sayede macOS NFD
/// "şehir" ile Windows NFC "şehir" aynı kovaya düşer.
#[derive(Debug, Clone)]
pub struct NormalizedPath {
    /// Orijinal byte sequence; Linux'ta non-UTF-8 olabilir, bu durumda
    /// `to_string_lossy()` placeholder içeren bir NFC üretir (raw bytes yine
    /// `raw` üzerinden korunur, Bölüm 12.7).
    raw: PathBuf,
    /// NFC normalize form; karşılaştırma + hash burada gerçekleşir.
    nfc: String,
}

impl NormalizedPath {
    /// Verilen path'ten NFC normalize form üretir.
    ///
    /// Non-UTF-8 Linux path'leri `to_string_lossy()` ile UTF-8'e geçirilir
    /// (NFC compare için); raw bytes değişmeden korunur.
    pub fn new(path: impl AsRef<Path>) -> Self {
        let raw = path.as_ref().to_path_buf();
        let lossy = raw.to_string_lossy();
        let nfc: String = lossy.nfc().collect();
        Self { raw, nfc }
    }

    /// Internal compare / display için NFC form.
    pub fn nfc(&self) -> &str {
        &self.nfc
    }

    /// Disk I/O için ham path (NFC'ye çevrilmemiş, server'ın verdiği gibi).
    pub fn raw(&self) -> &Path {
        &self.raw
    }

    /// Tüketerek ham path'i çıkar.
    pub fn into_raw(self) -> PathBuf {
        self.raw
    }
}

impl PartialEq for NormalizedPath {
    fn eq(&self, other: &Self) -> bool {
        self.nfc == other.nfc
    }
}

impl Eq for NormalizedPath {}

impl Hash for NormalizedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.nfc.hash(state);
    }
}

impl From<&Path> for NormalizedPath {
    fn from(p: &Path) -> Self {
        Self::new(p)
    }
}

impl From<PathBuf> for NormalizedPath {
    fn from(p: PathBuf) -> Self {
        Self::new(p)
    }
}

impl From<&str> for NormalizedPath {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::collections::hash_map::DefaultHasher;

    fn hash_of(p: &NormalizedPath) -> u64 {
        let mut h = DefaultHasher::new();
        p.hash(&mut h);
        h.finish()
    }

    /// Türkçe `ş` — NFC tek codepoint vs NFD iki codepoint. Aynı görsel,
    /// farklı byte; NormalizedPath ikisini eşit görmeli.
    #[test]
    fn turkish_nfc_vs_nfd_equality() {
        // NFC: U+015F (LATIN SMALL LETTER S WITH CEDILLA)
        let nfc = NormalizedPath::new("\u{015F}ehir");
        // NFD: U+0073 + U+0327 (s + COMBINING CEDILLA)
        let nfd = NormalizedPath::new("s\u{0327}ehir");

        assert_eq!(nfc, nfd, "NFC and NFD forms of 'şehir' must compare equal");
        // Ham byte sequence farkı korunmalı.
        assert_ne!(nfc.raw(), nfd.raw(), "raw bytes preserved as-is");
    }

    /// Hash invariance: aynı dosya NFC ve NFD ile gelse de HashMap'te tek
    /// kovaya düşmeli, aksi halde dedup patlar.
    #[test]
    fn hash_invariance_nfc_nfd() {
        let nfc = NormalizedPath::new("\u{015F}ehir.txt");
        let nfd = NormalizedPath::new("s\u{0327}ehir.txt");
        assert_eq!(hash_of(&nfc), hash_of(&nfd));

        let mut map: HashMap<NormalizedPath, u32> = HashMap::new();
        map.insert(nfc.clone(), 1);
        // NFD insert üzerine yazmalı (aynı kova).
        map.insert(nfd, 2);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&nfc), Some(&2));
    }

    /// Diğer Türkçe karakterler: `ğ` (U+011F) ve `ı` (U+0131).
    /// `ı` (dotless i) NFC ve NFD'de aynıdır (combining yok), ama
    /// regression için kontrol edelim.
    #[test]
    fn turkish_other_chars_dotless_i_and_g_breve() {
        let g_nfc = NormalizedPath::new("a\u{011F}");
        let g_nfd = NormalizedPath::new("ag\u{0306}");
        assert_eq!(g_nfc, g_nfd, "'ğ' NFC vs NFD");

        let i_a = NormalizedPath::new("kar\u{0131}\u{015F}\u{0131}k");
        let i_b = NormalizedPath::new("kar\u{0131}s\u{0327}\u{0131}k");
        assert_eq!(i_a, i_b, "'karışık' mixed NFC/NFD");
    }

    /// ASCII path: NFC/NFD eşdeğer (no-op normalize), ham == nfc.
    #[test]
    fn ascii_pure_path_is_pass_through() {
        let p = NormalizedPath::new("/var/log/app.log");
        assert_eq!(p.nfc(), "/var/log/app.log");
        assert_eq!(p.raw().to_str().unwrap(), "/var/log/app.log");
    }

    /// Farklı dosyalar farklı kovaya düşmeli (false-positive regression).
    #[test]
    fn distinct_paths_remain_distinct() {
        let a = NormalizedPath::new("README.md");
        let b = NormalizedPath::new("readme.md");
        // NFC compare case-sensitive — case_conflict.rs lowercase fold yapar.
        assert_ne!(a, b);
        assert_ne!(hash_of(&a), hash_of(&b));
    }

    /// PathBuf, &str, &Path conversion'larının hepsi aynı NFC üretmeli.
    #[test]
    fn from_conversions_consistent() {
        let s = "\u{015F}ehir";
        let from_str: NormalizedPath = s.into();
        let from_path: NormalizedPath = Path::new(s).into();
        let from_buf: NormalizedPath = PathBuf::from(s).into();
        assert_eq!(from_str, from_path);
        assert_eq!(from_path, from_buf);
    }
}
