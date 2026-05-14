//! Path length classifier — Bölüm 12.5.
//!
//! Windows MAX_PATH = 260 karakter (legacy API). Üzeri için `\\?\` UNC prefix
//! ile 32_767'ye kadar çıkılabilir (Win 10+ opt-in). 32_760'ın üstü pratikte
//! her şeyi kırar — Pathological. Bu modül runtime'da bir path'in hangi
//! kategoriye düştüğünü söyler; karar UI/policy katmanına bırakılır.

use std::path::Path;

const WIN_LONG_PATH_PREFIX: &str = r"\\?\";
const WIN_UNC_LONG_PREFIX: &str = r"\\?\UNC\";

/// Path uzunluk sınıfı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathLengthClass {
    /// < 240 byte — hiçbir endişe.
    Safe,
    /// 240 ≤ len < 260 — sınırda, gelecek değişiklikler riskli.
    NearLimit,
    /// ≥ 260 ve `\\?\` prefix yok — Windows legacy API başarısız olur.
    OverLimitWithoutUnc,
    /// ≥ 260 ve UNC prefix var — çalışır ama Explorer/AV compat. uyarısı.
    OverLimitWithUnc,
    /// ≥ 32_760 byte — Windows hard limit, her şey kırılır.
    Pathological,
}

const NEAR_LIMIT_THRESHOLD: usize = 240;
const MAX_PATH: usize = 260;
const WIN_HARD_LIMIT: usize = 32_760;

/// Path uzunluğunu sınıflandırır. Byte uzunluğu kullanılır — UTF-8 multi-byte
/// karakterler (Türkçe `ş`, `ı`, vb.) gerçek depolama maliyetiyle hesaplanır.
pub fn classify_path_length(path: &Path) -> PathLengthClass {
    let s = path.to_string_lossy();
    let len = s.len();
    let has_unc = s.starts_with(WIN_LONG_PATH_PREFIX) || s.starts_with(WIN_UNC_LONG_PREFIX);

    if len >= WIN_HARD_LIMIT {
        return PathLengthClass::Pathological;
    }

    if len >= MAX_PATH {
        if has_unc {
            return PathLengthClass::OverLimitWithUnc;
        }
        return PathLengthClass::OverLimitWithoutUnc;
    }

    if len >= NEAR_LIMIT_THRESHOLD {
        return PathLengthClass::NearLimit;
    }

    PathLengthClass::Safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn short_path_is_safe() {
        assert_eq!(classify_path_length(&p(r"C:\foo.txt")), PathLengthClass::Safe);
        assert_eq!(
            classify_path_length(&p("/var/log/app.log")),
            PathLengthClass::Safe
        );
    }

    #[test]
    fn near_limit_is_classified() {
        // 250 byte path.
        let s = "a".repeat(250);
        assert_eq!(classify_path_length(&p(&s)), PathLengthClass::NearLimit);
    }

    #[test]
    fn over_limit_without_unc() {
        let s = "a".repeat(280);
        assert_eq!(
            classify_path_length(&p(&s)),
            PathLengthClass::OverLimitWithoutUnc
        );
    }

    #[test]
    fn over_limit_with_unc_prefix() {
        let mut s = String::from(r"\\?\C:\");
        // Toplam ≥ 260 olacak şekilde.
        s.push_str(&"a".repeat(280));
        assert_eq!(classify_path_length(&p(&s)), PathLengthClass::OverLimitWithUnc);
    }

    #[test]
    fn over_limit_with_unc_unc_share_prefix() {
        let mut s = String::from(r"\\?\UNC\server\share\");
        s.push_str(&"a".repeat(280));
        assert_eq!(classify_path_length(&p(&s)), PathLengthClass::OverLimitWithUnc);
    }

    #[test]
    fn pathological_length() {
        let s = "a".repeat(33_000);
        assert_eq!(
            classify_path_length(&p(&s)),
            PathLengthClass::Pathological
        );
    }

    /// Sınır değerleri: 239 = Safe, 240 = NearLimit, 259 = NearLimit, 260 = OverLimit.
    #[test]
    fn boundary_values() {
        assert_eq!(
            classify_path_length(&p(&"a".repeat(239))),
            PathLengthClass::Safe
        );
        assert_eq!(
            classify_path_length(&p(&"a".repeat(240))),
            PathLengthClass::NearLimit
        );
        assert_eq!(
            classify_path_length(&p(&"a".repeat(259))),
            PathLengthClass::NearLimit
        );
        assert_eq!(
            classify_path_length(&p(&"a".repeat(260))),
            PathLengthClass::OverLimitWithoutUnc
        );
    }
}
