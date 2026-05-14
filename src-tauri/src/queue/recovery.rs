//! Startup recovery — Bölüm 15.1 + Bölüm 28.
//!
//! Crash sonrası iki ayrı tutarsızlık temizliği:
//!
//! 1. **DB tarafı** ([`run_recovery`]): `Active` task'lar process ile birlikte
//!    öldü → `Queued` (resurrect). `Verifying`/`Finalizing` commit fazında
//!    öldüler → `Failed` (abandoned) — partial-rename gibi yan etkiler
//!    olabileceğinden otomatik retry yapmıyoruz.
//! 2. **Filesystem tarafı** ([`cleanup_orphan_tmps`]): `*.dtransfer_tmp`
//!    dosyaları atomic-finalization sırasında ölmüş transferler — `mtime`
//!    eşiğinden eski olanlar silinir. Yarım yazılmış dosya kuyrukla
//!    eşleşmiyorsa orphan; eşleşiyorsa scheduler yeniden başlattığında üzerine
//!    yeni `.tmp` açar.
//!
//! Atomik tek transaction (DB tarafı) + best-effort filesystem walk; iki
//! taraf birbirinden bağımsız.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::Connection;
use serde::Serialize;
use tracing::{debug, warn};

use super::db_actor::DbError;

const TMP_SUFFIX: &str = ".dtransfer_tmp";

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct RecoveryReport {
    /// Active → Queued reset edilen task sayısı.
    pub resurrected_count: usize,
    /// Verifying/Finalizing → Failed işaretlenen task sayısı.
    pub abandoned_count: usize,
    /// `.dtransfer_tmp` orphan dosya temizlik sayısı. Sadece `lib.rs` startup
    /// path'inde [`cleanup_orphan_tmps`] çağrılırsa dolar; DB-only recovery
    /// 0 bırakır.
    pub orphan_tmp_files: usize,
}

/// DB recovery query'lerini sırayla çalıştırır. **Atomik tek transaction**:
/// yarı-kurtarılmış DB durumu olamaz.
pub fn run_recovery(conn: &mut Connection) -> Result<RecoveryReport, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    let tx = conn.transaction()?;

    let resurrected = tx.execute(
        "UPDATE transfers SET state='queued', updated_at=?1
         WHERE state='active'",
        rusqlite::params![now],
    )?;

    let abandoned_err = r#"{"category":"unknown","suggestedAction":"userDecision","i18nKey":"unknown","message":"abandoned during commit phase"}"#;
    let abandoned = tx.execute(
        "UPDATE transfers SET state='failed', last_error=?1, updated_at=?2
         WHERE state IN ('verifying', 'finalizing')",
        rusqlite::params![abandoned_err, now],
    )?;

    tx.commit()?;

    Ok(RecoveryReport {
        resurrected_count: resurrected,
        abandoned_count: abandoned,
        orphan_tmp_files: 0,
    })
}

/// Verilen kök dizinleri (non-recursive **değil** — alt dizinleri de tarar)
/// gez, `*.dtransfer_tmp` dosyaları topla; `mtime` `min_age`'den eski olanları
/// sil. Toplam silinen dosya sayısını döner.
///
/// "Eski" eşiğinin amacı: az önce yazılmaya başlamış bir tmp'yi (yeni transfer
/// başlattı) silmemek. Default `min_age = 24h` startup'ta makul; daha agresif
/// temizlik için caller daha kısa süre verebilir.
///
/// I/O hataları sayıma dahil edilmez (best-effort) — `warn!` ile log'a düşer
/// ve cleanup devam eder.
pub fn cleanup_orphan_tmps(roots: &[PathBuf], min_age: Duration) -> usize {
    let cutoff = SystemTime::now().checked_sub(min_age).unwrap_or(SystemTime::UNIX_EPOCH);
    let mut deleted = 0usize;
    for root in roots {
        deleted += walk_and_clean(root, cutoff);
    }
    deleted
}

fn walk_and_clean(dir: &Path, cutoff: SystemTime) -> usize {
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(e) => {
            // Dizin yok / izin yok — sessizce geç, recovery'i bloklamayız.
            debug!(?dir, ?e, "tmp cleanup skip dir");
            return 0;
        }
    };
    let mut deleted = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                debug!(?path, ?e, "tmp cleanup metadata fail");
                continue;
            }
        };
        if meta.is_dir() {
            // Symlink dir'leri takip etmiyoruz — sonsuz döngü riskli ve
            // genelde başka transfer profile'ının root'una sıçramak istemiyoruz.
            if !meta.file_type().is_symlink() {
                deleted += walk_and_clean(&path, cutoff);
            }
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !name.ends_with(TMP_SUFFIX) {
            continue;
        }
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if mtime > cutoff {
            // Henüz çok yeni — yazımı süren bir transfer olabilir.
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                deleted += 1;
                debug!(?path, "orphan tmp removed");
            }
            Err(e) => warn!(?path, ?e, "orphan tmp remove failed"),
        }
    }
    deleted
}

#[cfg(test)]
mod fs_tests {
    use super::*;
    use std::fs::{self, File};
    use std::time::Duration;
    use tempfile::tempdir;

    fn touch(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    /// `File::set_modified` (std 1.75+) ile mtime'ı geriye al. Hem POSIX hem
    /// Windows'ta çalışır, ekstra dep gerekmez.
    fn set_old_mtime(path: &Path, secs_ago: u64) {
        let target = SystemTime::now() - Duration::from_secs(secs_ago);
        let f = File::options().write(true).open(path).unwrap();
        f.set_modified(target).unwrap();
    }

    #[test]
    fn cleanup_removes_old_tmps_only() {
        let dir = tempdir().unwrap();
        let old_tmp = dir.path().join("a.dtransfer_tmp");
        let fresh_tmp = dir.path().join("b.dtransfer_tmp");
        let plain = dir.path().join("normal.txt");
        touch(&old_tmp, b"x");
        touch(&fresh_tmp, b"y");
        touch(&plain, b"z");
        set_old_mtime(&old_tmp, 48 * 3600);

        let count = cleanup_orphan_tmps(
            &[dir.path().to_path_buf()],
            Duration::from_secs(24 * 3600),
        );
        assert_eq!(count, 1);
        assert!(!old_tmp.exists());
        assert!(fresh_tmp.exists());
        assert!(plain.exists());
    }

    #[test]
    fn cleanup_walks_subdirs() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("nested/deep");
        let tmp = sub.join("file.dtransfer_tmp");
        touch(&tmp, b"x");
        set_old_mtime(&tmp, 48 * 3600);

        let count = cleanup_orphan_tmps(
            &[dir.path().to_path_buf()],
            Duration::from_secs(24 * 3600),
        );
        assert_eq!(count, 1);
        assert!(!tmp.exists());
    }

    #[test]
    fn missing_root_is_noop() {
        let bogus = if cfg!(windows) {
            PathBuf::from(r"Z:\nope\nope\nope")
        } else {
            PathBuf::from("/definitely/not/a/real/path/xyz123")
        };
        let count = cleanup_orphan_tmps(&[bogus], Duration::from_secs(60));
        assert_eq!(count, 0);
    }

    /// Filtre `.dtransfer_tmp` suffix'inde — diğer dosyalar dokunulmaz.
    #[test]
    fn non_tmp_suffix_ignored() {
        let dir = tempdir().unwrap();
        let f = dir.path().join("scratch.tmp"); // farklı suffix
        touch(&f, b"x");
        set_old_mtime(&f, 48 * 3600);
        let count = cleanup_orphan_tmps(
            &[dir.path().to_path_buf()],
            Duration::from_secs(24 * 3600),
        );
        assert_eq!(count, 0);
        assert!(f.exists());
    }
}
