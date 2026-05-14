//! Case + Unicode normalization conflict detection — Bölüm 12.3.
//!
//! Linux'tan iki dosya gelir: `README.md` + `readme.md`. Windows ve macOS
//! (default) case-insensitive — ikinci dosya birinciyi overwrite eder, kullanıcı
//! sessizce veri kaybeder. Detector incoming her path için lowercase + NFC
//! key'i HashMap'te tutar; çakışma görürse `CaseConflict` raporlar.
//!
//! NFC + lowercase fold sayesinde aynı zamanda NFC vs NFD farkıyla aynı isme
//! düşen iki dosya da yakalanır (`şehir` NFC + `şehir` NFD).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseConflict {
    /// Önce kayıt edilmiş path.
    pub existing: PathBuf,
    /// Yeni gelen ve çakışan path.
    pub incoming: PathBuf,
}

#[derive(Debug, Default)]
pub struct CaseConflictDetector {
    /// lowercase(NFC(path)) → ilk kayıtta görülen path.
    seen: HashMap<String, PathBuf>,
}

impl CaseConflictDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Path'i kaydet; çakışma varsa `Some(CaseConflict)` döner ve **yeni** path
    /// kayıtta tutulmaz (önce gelen kazanır — bu UI'a "incoming bu existing ile
    /// çakışıyor" demek için netlik sağlar).
    ///
    /// Tamamen aynı path (case ve normalize dahil) iki kez register edilirse
    /// conflict üretmez (idempotent — aynı manifest entry'sinin iki kez işlenmesi
    /// tipik bir bug değil, koruma).
    pub fn register(&mut self, path: &Path) -> Option<CaseConflict> {
        let key = fold_key(path);
        match self.seen.get(&key) {
            Some(existing) if existing == path => None,
            Some(existing) => Some(CaseConflict {
                existing: existing.clone(),
                incoming: path.to_path_buf(),
            }),
            None => {
                self.seen.insert(key, path.to_path_buf());
                None
            }
        }
    }

    /// Şu ana kadar kaç farklı bucket görüldü (test/diagnostik).
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

fn fold_key(path: &Path) -> String {
    // NFC normalize sonrası ASCII lowercase fold. NFC öncesi/sonrası
    // case-mapping farkı (örn. Türkçe `İ` -> `i̇` NFD) sürpriz çıkarabilir;
    // bu yüzden önce NFC, sonra ASCII lowercase fold (filename collision
    // pratikleri için yeterli — full Unicode case fold v2'de eklenebilir).
    let lossy = path.to_string_lossy();
    let nfc: String = lossy.nfc().collect();
    nfc.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// Linux'tan gelen `README.md` + `readme.md` Windows'ta tek dosyaya overwrite
    /// olur — detector bunu yakalamalı.
    #[test]
    fn detects_case_only_conflict() {
        let mut d = CaseConflictDetector::new();
        assert!(d.register(&pb("README.md")).is_none());
        let conflict = d.register(&pb("readme.md")).unwrap();
        assert_eq!(conflict.existing, pb("README.md"));
        assert_eq!(conflict.incoming, pb("readme.md"));
    }

    /// Aynı path iki kez gelirse conflict raporlanmamalı (idempotent).
    #[test]
    fn same_path_twice_is_not_conflict() {
        let mut d = CaseConflictDetector::new();
        assert!(d.register(&pb("README.md")).is_none());
        assert!(d.register(&pb("README.md")).is_none());
    }

    /// NFC vs NFD aynı isim → conflict (case + normalize birlikte).
    #[test]
    fn nfc_vs_nfd_conflict_with_case() {
        let mut d = CaseConflictDetector::new();
        // Türkçe 'ş' NFC tek codepoint
        let nfc = pb("\u{015F}EHIR.txt");
        // Aynısı NFD + farklı case
        let nfd = pb("s\u{0327}ehir.txt");
        assert!(d.register(&nfc).is_none());
        let conflict = d.register(&nfd).unwrap();
        assert_eq!(conflict.existing, nfc);
        assert_eq!(conflict.incoming, nfd);
    }

    /// Tamamen farklı dosyalar çakışmaz.
    #[test]
    fn distinct_paths_no_conflict() {
        let mut d = CaseConflictDetector::new();
        assert!(d.register(&pb("a.txt")).is_none());
        assert!(d.register(&pb("b.txt")).is_none());
        assert_eq!(d.len(), 2);
    }

    /// Subdir prefix farkı korunur: `foo/README.md` ve `bar/README.md` çakışmaz.
    #[test]
    fn different_dirs_no_conflict() {
        let mut d = CaseConflictDetector::new();
        assert!(d.register(&pb("foo/README.md")).is_none());
        assert!(d.register(&pb("bar/README.md")).is_none());
    }

    /// Aynı dir, farklı case + farklı normalize → conflict.
    #[test]
    fn same_dir_case_difference_only() {
        let mut d = CaseConflictDetector::new();
        assert!(d.register(&pb("docs/Notes.md")).is_none());
        let c = d.register(&pb("docs/notes.md")).unwrap();
        assert_eq!(c.existing, pb("docs/Notes.md"));
    }

    #[test]
    fn empty_detector_is_empty() {
        let d = CaseConflictDetector::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
    }
}
