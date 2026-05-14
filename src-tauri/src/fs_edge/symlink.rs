//! Symlink politikası ve target sanitization — Bölüm 12.1.
//!
//! ### Kötü niyetli sunucu senaryosu (CVE-class)
//!
//! Remote sunucu transfer manifest'inde şu symlink'i gönderir:
//! ```text
//! remote: /var/www/data/passwd_link -> /etc/passwd
//! ```
//! İstemci `Preserve` modunda bunu olduğu gibi yazarsa:
//! ```text
//! local:  ~/Downloads/data/passwd_link -> /etc/passwd
//! ```
//! Kullanıcı dosyaya tıklayınca **kendi sisteminin** `/etc/passwd`'ına yönelir;
//! Windows'ta benzeri `-> C:\Windows\System32\config\SAM`. Bu pasif bir
//! information-disclosure açığıdır.
//!
//! Bu sebeple default politika `SanitizeOrSkip`'tir: absolute target'lar
//! reddedilir; relative target'lar transfer root'undan dışarı çıkarsa
//! (`../../../etc/passwd`) yine reddedilir. `PreserveAsIs` yalnızca güvenilir
//! kaynak için bilinçli override'dır (rsync benzeri full backup senaryosu).

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Symlink target nasıl ele alınacak?
///
/// Default = `SanitizeOrSkip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkPolicy {
    /// Symlink target'ı olduğu gibi yaz; güvenilir kaynak override'ı.
    /// Absolute target'lar dahil hiçbir sanitization yapılmaz.
    PreserveAsIs,
    /// Default: relative + root-içi target → preserve; absolute veya root-dışı → skip.
    SanitizeOrSkip,
    /// Symlink yerine hedefin içeriğini kopyala (cycle detection ile).
    FollowAndCopy,
}

impl Default for SymlinkPolicy {
    fn default() -> Self {
        SymlinkPolicy::SanitizeOrSkip
    }
}

/// Sanitize sonucu — adapter ne yapacağını bu enum'dan anlar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymlinkAction {
    /// Bu target ile symlink'i olduğu gibi yaz.
    PreserveTarget(PathBuf),
    /// Bu link'i transfer etme; sebep `~/.dtransfer/skipped.log`'a yazılacak.
    SkipWithReason(String),
    /// Symlink değil, hedefin bytes'ını kopyala.
    FollowToContent,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SymlinkError {
    /// `SanitizeOrSkip` modunda absolute target → reddedilir.
    #[error("absolute symlink target blocked by SanitizeOrSkip policy")]
    AbsoluteLinkBlocked,
    /// Relative target normalize edildiğinde root dışına çıkıyor.
    #[error("symlink target escapes transfer root")]
    EscapesRoot,
    /// `FollowAndCopy` modunda hedef zinciri kendine dönüyor (a -> b -> c -> a).
    #[error("symlink cycle detected starting at {start}")]
    CycleDetected { start: String },
    /// Path UTF-8 değil ve target OS bunu kabul edemiyor.
    #[error("invalid encoding in symlink target")]
    InvalidEncoding,
}

/// Tek bir symlink target'ını politikaya göre değerlendirir.
///
/// `link_path` symlink dosyasının kendisi (lokasyon için kullanılır);
/// `target` symlink'in işaret ettiği path (absolute veya relative);
/// `root` transfer kök dizini — relative target'ların buradan dışarı çıkıp
/// çıkmadığını anlamak için kullanılır.
pub fn sanitize_symlink_target(
    link_path: &Path,
    target: &Path,
    policy: SymlinkPolicy,
    root: &Path,
) -> Result<SymlinkAction, SymlinkError> {
    match policy {
        SymlinkPolicy::PreserveAsIs => {
            // Override: kullanıcı bilinçli olarak güvenilir kaynak diyor.
            Ok(SymlinkAction::PreserveTarget(target.to_path_buf()))
        }
        SymlinkPolicy::FollowAndCopy => {
            // Burada gerçek cycle detection için adapter chain'i resolve eder;
            // pure fonksiyon olarak yapabileceğimiz tek şey: link kendine
            // direkt veya logical olarak dönüyorsa cycle.
            // FollowToContent + cycle kontrolü ayrıca `follow_with_cycle_check`
            // helper'ında yapılır.
            Ok(SymlinkAction::FollowToContent)
        }
        SymlinkPolicy::SanitizeOrSkip => sanitize_or_skip(link_path, target, root),
    }
}

fn sanitize_or_skip(
    link_path: &Path,
    target: &Path,
    root: &Path,
) -> Result<SymlinkAction, SymlinkError> {
    if is_absolute_or_unc(target) {
        // CVE-class: /etc/passwd, C:\Windows\System32\... veya UNC \\server\share.
        return Err(SymlinkError::AbsoluteLinkBlocked);
    }

    // Relative target — link'in dizinine göre çözüp normalize et, sonra
    // root'a sığıp sığmadığını kontrol et.
    let link_dir = link_path.parent().unwrap_or_else(|| Path::new(""));
    let logical = link_dir.join(target);

    if !stays_within_root(&logical, root) {
        return Err(SymlinkError::EscapesRoot);
    }

    Ok(SymlinkAction::PreserveTarget(target.to_path_buf()))
}

/// `..` bileşenlerini soyutlayarak normalize eder ve sonucun `root` altında
/// kalıp kalmadığını kontrol eder. Disk I/O yapmaz (canonicalize değil) —
/// transfer öncesi statik analiz.
fn stays_within_root(logical: &Path, root: &Path) -> bool {
    let mut depth: i64 = 0;
    let root_depth = count_normal_components(root) as i64;

    for comp in logical.components() {
        match comp {
            Component::Normal(_) => depth += 1,
            Component::ParentDir => depth -= 1,
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir => {
                // Absolute leaked through; reject defensively.
                return false;
            }
        }
        // Root depth'inden daha geriye gittiysek root dışına çıktık.
        if depth < root_depth {
            return false;
        }
    }
    true
}

fn count_normal_components(p: &Path) -> usize {
    p.components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count()
}

/// Windows UNC (`\\server\share\...`), Windows drive letter (`C:\...`) veya
/// POSIX absolute (`/...`) target'ları reddeder.
fn is_absolute_or_unc(target: &Path) -> bool {
    if target.is_absolute() {
        return true;
    }
    // Defansif: `is_absolute` sadece host OS'e göre karar verir. Cross-platform
    // manifest'te POSIX absolute (`/etc/passwd`) Windows host'unda
    // `is_absolute() == false` döner — manuel kontrol ekle.
    if let Some(s) = target.to_str() {
        if s.starts_with('/') {
            return true;
        }
        if s.starts_with("\\\\") {
            // UNC — `\\server\share\foo` veya `\\?\C:\...`.
            return true;
        }
        // Windows drive letter: `C:\foo` veya `C:foo`.
        let bytes = s.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return true;
        }
    }
    false
}

/// `FollowAndCopy` modunda chain resolution. Verilen `resolver` callback
/// bir symlink'in hedefini döndürür (None = düz dosya / chain bitti).
/// `start_link`'ten başlayarak hedefi resolve eder; cycle gördüğü an hata.
///
/// Pure fonksiyon (resolver caller'ın sağladığı; testlerde stub).
pub fn follow_with_cycle_check<F>(
    start_link: &Path,
    mut resolver: F,
    max_depth: usize,
) -> Result<PathBuf, SymlinkError>
where
    F: FnMut(&Path) -> Option<PathBuf>,
{
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut current = start_link.to_path_buf();

    for _ in 0..max_depth {
        if !seen.insert(current.clone()) {
            return Err(SymlinkError::CycleDetected {
                start: start_link.display().to_string(),
            });
        }
        match resolver(&current) {
            Some(next) => current = next,
            None => return Ok(current),
        }
    }
    // max_depth aşıldı — büyük olasılıkla loop ama emin değiliz; cycle olarak rapor.
    Err(SymlinkError::CycleDetected {
        start: start_link.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// CVE-class senaryo: kötü niyetli sunucu absolute target gönderir.
    /// Default `SanitizeOrSkip` bunu engellemeli.
    #[test]
    fn sanitize_or_skip_blocks_posix_absolute_target() {
        let res = sanitize_symlink_target(
            &p("downloads/data/passwd_link"),
            &p("/etc/passwd"),
            SymlinkPolicy::SanitizeOrSkip,
            &p("downloads"),
        );
        assert_eq!(res, Err(SymlinkError::AbsoluteLinkBlocked));
    }

    #[test]
    fn sanitize_or_skip_blocks_windows_absolute_target() {
        let res = sanitize_symlink_target(
            &p("downloads/data/sam_link"),
            &p(r"C:\Windows\System32\config\SAM"),
            SymlinkPolicy::SanitizeOrSkip,
            &p("downloads"),
        );
        assert_eq!(res, Err(SymlinkError::AbsoluteLinkBlocked));
    }

    #[test]
    fn sanitize_or_skip_blocks_unc_target() {
        let res = sanitize_symlink_target(
            &p("downloads/data/share_link"),
            &p(r"\\evil-server\share\payload"),
            SymlinkPolicy::SanitizeOrSkip,
            &p("downloads"),
        );
        assert_eq!(res, Err(SymlinkError::AbsoluteLinkBlocked));
    }

    /// Relative target root dışına çıkıyorsa reddet (../../../etc/passwd).
    #[test]
    fn sanitize_or_skip_blocks_relative_escape() {
        let res = sanitize_symlink_target(
            &p("downloads/data/sneaky"),
            &p("../../../etc/passwd"),
            SymlinkPolicy::SanitizeOrSkip,
            &p("downloads"),
        );
        assert_eq!(res, Err(SymlinkError::EscapesRoot));
    }

    /// Root içinde kalan relative target → preserve.
    #[test]
    fn sanitize_or_skip_allows_in_root_relative() {
        let res = sanitize_symlink_target(
            &p("downloads/data/ok_link"),
            &p("../README.md"),
            SymlinkPolicy::SanitizeOrSkip,
            &p("downloads"),
        );
        assert_eq!(
            res,
            Ok(SymlinkAction::PreserveTarget(p("../README.md")))
        );
    }

    /// Power user override: PreserveAsIs absolute target'a izin verir.
    /// Test "override gerçekten çalışıyor mu?" diye doğrular — kullanıcının
    /// bilinçli kararı.
    #[test]
    fn preserve_as_is_passes_absolute_target() {
        let res = sanitize_symlink_target(
            &p("downloads/data/passwd_link"),
            &p("/etc/passwd"),
            SymlinkPolicy::PreserveAsIs,
            &p("downloads"),
        );
        assert_eq!(res, Ok(SymlinkAction::PreserveTarget(p("/etc/passwd"))));
    }

    /// FollowAndCopy + cycle: resolver `a -> b -> a` döngüsü kuruyor.
    #[test]
    fn follow_and_copy_detects_cycle() {
        // a -> b -> a
        let resolver = |path: &Path| -> Option<PathBuf> {
            match path.to_str() {
                Some("a") => Some(p("b")),
                Some("b") => Some(p("a")),
                _ => None,
            }
        };
        let res = follow_with_cycle_check(&p("a"), resolver, 32);
        assert!(matches!(res, Err(SymlinkError::CycleDetected { .. })));
    }

    /// FollowAndCopy normal zinciri çözer.
    #[test]
    fn follow_and_copy_resolves_chain() {
        // a -> b -> c -> (no link)
        let resolver = |path: &Path| -> Option<PathBuf> {
            match path.to_str() {
                Some("a") => Some(p("b")),
                Some("b") => Some(p("c")),
                _ => None,
            }
        };
        let res = follow_with_cycle_check(&p("a"), resolver, 32).unwrap();
        assert_eq!(res, p("c"));
    }

    /// max_depth aşılırsa cycle olarak raporla (defansif).
    #[test]
    fn follow_and_copy_max_depth_exhaustion() {
        let resolver = |path: &Path| -> Option<PathBuf> {
            // Sonsuz zincir: a -> a1 -> a2 -> ...
            let s = path.to_str().unwrap_or("");
            Some(PathBuf::from(format!("{s}x")))
        };
        let res = follow_with_cycle_check(&p("a"), resolver, 4);
        assert!(matches!(res, Err(SymlinkError::CycleDetected { .. })));
    }

    /// FollowAndCopy policy direkt → FollowToContent.
    #[test]
    fn policy_follow_and_copy_returns_follow_action() {
        let res = sanitize_symlink_target(
            &p("downloads/link"),
            &p("../target"),
            SymlinkPolicy::FollowAndCopy,
            &p("downloads"),
        );
        assert_eq!(res, Ok(SymlinkAction::FollowToContent));
    }

    #[test]
    fn default_policy_is_sanitize_or_skip() {
        assert_eq!(SymlinkPolicy::default(), SymlinkPolicy::SanitizeOrSkip);
    }
}
