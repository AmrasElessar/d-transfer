//! Application settings — disk-persistent kullanıcı tercihleri.
//!
//! ## Kapsam
//!
//! Theme + locale UI ihtiyacı nedeniyle `localStorage`'da (cold-start anlık);
//! geri kalan tercihler (default download dir, concurrency, chunk size,
//! bandwidth limit, fsync policy, vb.) bu modülde `app_data_dir/settings.json`
//! içinde saklanır.
//!
//! ## Yazma disiplini
//!
//! - **Atomic write**: `settings.json` doğrudan üzerine yazılmaz — `.tmp` + rename
//!   (Bölüm 14.2 atomic finalization). Power-cut'a karşı korunaklı.
//! - **Migration**: `schema_version` field'ı taşır; yeni versiyon eklendikçe
//!   `migrate()` zincirleme yükseltir.
//! - **Mutex**: tek `Mutex<AppSettings>` üzerinden serileştirilir; yüksek
//!   frekanslı update beklenmiyor.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const SETTINGS_FILE_NAME: &str = "settings.json";
const TMP_SUFFIX: &str = ".tmp";
const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("schema version {found} too new — current max supported {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChecksumAlgo {
    None,
    Sha256,
    XxHash3,
}

impl Default for ChecksumAlgo {
    fn default() -> Self {
        Self::Sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FsyncPolicy {
    /// Hiç fsync yok — power-cut'ta veri kaybı kabul (opt-in performance).
    None,
    /// `sync_data()` çağrılır + parent dir POSIX'te sync (Bölüm 14.6 default).
    DataOnly,
    /// `sync_all()` — data + metadata.
    Full,
}

impl Default for FsyncPolicy {
    fn default() -> Self {
        Self::DataOnly
    }
}

/// Diske persist edilen uygulama tercihleri (theme/locale hariç).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: u32,

    /// Download'lar için varsayılan hedef klasör. None = OS default (Downloads).
    pub default_download_dir: Option<PathBuf>,

    /// Eş zamanlı transfer üst sınırı (scheduler concurrency). Faz 3 hardcoded
    /// 1 — Faz 4'te bu setting devreye girer.
    pub max_concurrent_transfers: u32,

    /// Adapter chunk_size varsayılanı (MiB). TransferOptions::chunk_size mapping.
    pub default_chunk_size_mb: u32,

    /// SFTP/protokol bazlı `max_inflight_bytes` (MiB) — Bölüm 9.2.
    pub default_max_inflight_mb: u32,

    /// Global bandwidth limit (bytes/sec). None = sınırsız.
    pub bandwidth_limit_bps: Option<u64>,

    /// Varsayılan transfer checksum doğrulaması.
    pub verify_checksum: ChecksumAlgo,

    /// fsync politikası (Bölüm 14.6).
    pub fsync_policy: FsyncPolicy,

    /// Auto-update kontrolü (Bölüm 27). Production'da default true; dev'de
    /// kullanıcı manuel kapatabilir.
    pub auto_update: bool,

    /// Telemetri — Bölüm 31 default sıfır. Bu setting **opt-in flag**; ileride
    /// telemetry adapter eklense bile bu false ise hiçbir veri gönderilmez.
    pub telemetry: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            default_download_dir: None,
            max_concurrent_transfers: 1,
            default_chunk_size_mb: 8,
            default_max_inflight_mb: 64,
            bandwidth_limit_bps: None,
            verify_checksum: ChecksumAlgo::default(),
            fsync_policy: FsyncPolicy::default(),
            auto_update: true,
            telemetry: false,
        }
    }
}

/// Per-field partial update DTO — UI yalnızca değiştirmek istediği alanı yollar.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
    pub default_download_dir: Option<Option<PathBuf>>,
    pub max_concurrent_transfers: Option<u32>,
    pub default_chunk_size_mb: Option<u32>,
    pub default_max_inflight_mb: Option<u32>,
    pub bandwidth_limit_bps: Option<Option<u64>>,
    pub verify_checksum: Option<ChecksumAlgo>,
    pub fsync_policy: Option<FsyncPolicy>,
    pub auto_update: Option<bool>,
    pub telemetry: Option<bool>,
}

impl AppSettings {
    pub fn apply_patch(&mut self, patch: AppSettingsPatch) {
        if let Some(v) = patch.default_download_dir {
            self.default_download_dir = v;
        }
        if let Some(v) = patch.max_concurrent_transfers {
            self.max_concurrent_transfers = v.max(1);
        }
        if let Some(v) = patch.default_chunk_size_mb {
            self.default_chunk_size_mb = v.clamp(1, 1024);
        }
        if let Some(v) = patch.default_max_inflight_mb {
            self.default_max_inflight_mb = v.clamp(8, 4096);
        }
        if let Some(v) = patch.bandwidth_limit_bps {
            self.bandwidth_limit_bps = v;
        }
        if let Some(v) = patch.verify_checksum {
            self.verify_checksum = v;
        }
        if let Some(v) = patch.fsync_policy {
            self.fsync_policy = v;
        }
        if let Some(v) = patch.auto_update {
            self.auto_update = v;
        }
        if let Some(v) = patch.telemetry {
            self.telemetry = v;
        }
    }
}

pub struct SettingsStore {
    state: Mutex<AppSettings>,
    file_path: PathBuf,
}

impl SettingsStore {
    /// `app_data_dir`'i alır, içindeki `settings.json`'ı yükler ya da default
    /// ile oluşturur.
    pub fn load_or_init(app_data_dir: &Path) -> Result<Self, SettingsError> {
        std::fs::create_dir_all(app_data_dir)?;
        let file_path = app_data_dir.join(SETTINGS_FILE_NAME);

        let state = match std::fs::read_to_string(&file_path) {
            Ok(raw) => parse_and_migrate(&raw)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let defaults = AppSettings::default();
                write_atomic(&file_path, &defaults)?;
                defaults
            }
            Err(e) => return Err(e.into()),
        };

        Ok(Self {
            state: Mutex::new(state),
            file_path,
        })
    }

    pub fn snapshot(&self) -> AppSettings {
        self.state
            .lock()
            .expect("settings mutex poisoned")
            .clone()
    }

    /// Patch'i uygula, diske persist et, güncel snapshot döner.
    pub fn apply(&self, patch: AppSettingsPatch) -> Result<AppSettings, SettingsError> {
        let snapshot = {
            let mut guard = self.state.lock().expect("settings mutex poisoned");
            guard.apply_patch(patch);
            guard.clone()
        };
        write_atomic(&self.file_path, &snapshot)?;
        Ok(snapshot)
    }
}

fn parse_and_migrate(raw: &str) -> Result<AppSettings, SettingsError> {
    // Önce sadece schema_version'u oku — ileri sürümde abort.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct VersionPeek {
        #[serde(default)]
        schema_version: u32,
    }
    let peek: VersionPeek = serde_json::from_str(raw)?;
    if peek.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(SettingsError::UnsupportedSchemaVersion {
            found: peek.schema_version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    // v1 (current): doğrudan deserialize. Eski sürüm gelirse buraya migration
    // adımları eklenir (v0→v1, v1→v2, vb.).
    let settings: AppSettings = serde_json::from_str(raw)?;
    Ok(settings)
}

fn write_atomic(path: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    let json = serde_json::to_string_pretty(settings)?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(TMP_SUFFIX);
    let tmp_path = PathBuf::from(tmp);

    // Önceki orphan tmp varsa sil (best-effort).
    let _ = std::fs::remove_file(&tmp_path);

    std::fs::write(&tmp_path, json.as_bytes())?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_or_init_creates_defaults_when_missing() {
        let dir = tempdir().unwrap();
        let store = SettingsStore::load_or_init(dir.path()).unwrap();
        let snap = store.snapshot();
        assert_eq!(snap.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(snap.max_concurrent_transfers, 1);
        assert_eq!(snap.default_chunk_size_mb, 8);
        assert!(!snap.telemetry);
        assert!(dir.path().join("settings.json").exists());
    }

    #[test]
    fn apply_persists_to_disk_atomically() {
        let dir = tempdir().unwrap();
        let store = SettingsStore::load_or_init(dir.path()).unwrap();

        let patch = AppSettingsPatch {
            max_concurrent_transfers: Some(4),
            default_chunk_size_mb: Some(16),
            telemetry: Some(false),
            ..Default::default()
        };
        let updated = store.apply(patch).unwrap();
        assert_eq!(updated.max_concurrent_transfers, 4);
        assert_eq!(updated.default_chunk_size_mb, 16);

        // Reload yeni store ile — diske gerçekten yazılmış mı?
        drop(store);
        let reloaded = SettingsStore::load_or_init(dir.path()).unwrap();
        let snap = reloaded.snapshot();
        assert_eq!(snap.max_concurrent_transfers, 4);
        assert_eq!(snap.default_chunk_size_mb, 16);

        // Tmp dosyası kalmamış olmalı.
        let tmp = dir.path().join("settings.json.tmp");
        assert!(!tmp.exists(), "tmp file must be cleaned after rename");
    }

    #[test]
    fn patch_clamps_out_of_range_values() {
        let dir = tempdir().unwrap();
        let store = SettingsStore::load_or_init(dir.path()).unwrap();

        let absurd = AppSettingsPatch {
            max_concurrent_transfers: Some(0),
            default_chunk_size_mb: Some(99999),
            default_max_inflight_mb: Some(1),
            ..Default::default()
        };
        let s = store.apply(absurd).unwrap();
        // max_concurrent en az 1
        assert_eq!(s.max_concurrent_transfers, 1);
        // chunk_size_mb clamp [1, 1024]
        assert_eq!(s.default_chunk_size_mb, 1024);
        // max_inflight_mb clamp [8, 4096]
        assert_eq!(s.default_max_inflight_mb, 8);
    }

    #[test]
    fn future_schema_version_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"schemaVersion": 999, "maxConcurrentTransfers": 1, "defaultChunkSizeMb": 8,
                 "defaultMaxInflightMb": 64, "bandwidthLimitBps": null, "verifyChecksum": "sha256",
                 "fsyncPolicy": "dataOnly", "autoUpdate": true, "telemetry": false,
                 "defaultDownloadDir": null}"#,
        )
        .unwrap();
        match SettingsStore::load_or_init(dir.path()) {
            Err(SettingsError::UnsupportedSchemaVersion {
                found: 999,
                supported: 1,
            }) => {}
            Ok(_) => panic!("expected UnsupportedSchemaVersion, got Ok"),
            Err(other) => panic!("expected UnsupportedSchemaVersion, got {other}"),
        }
    }

    #[test]
    fn partial_patch_leaves_other_fields_intact() {
        let dir = tempdir().unwrap();
        let store = SettingsStore::load_or_init(dir.path()).unwrap();
        let before = store.snapshot();

        let only_telemetry = AppSettingsPatch {
            telemetry: Some(true),
            ..Default::default()
        };
        let after = store.apply(only_telemetry).unwrap();
        assert!(after.telemetry);
        assert_eq!(after.max_concurrent_transfers, before.max_concurrent_transfers);
        assert_eq!(after.default_chunk_size_mb, before.default_chunk_size_mb);
        assert_eq!(after.fsync_policy, before.fsync_policy);
    }

    #[test]
    fn unset_download_dir_via_explicit_none() {
        let dir = tempdir().unwrap();
        let store = SettingsStore::load_or_init(dir.path()).unwrap();

        // İlk set
        let set = AppSettingsPatch {
            default_download_dir: Some(Some(PathBuf::from("C:\\downloads"))),
            ..Default::default()
        };
        let s1 = store.apply(set).unwrap();
        assert_eq!(s1.default_download_dir, Some(PathBuf::from("C:\\downloads")));

        // Sonra None'a çek (explicit Some(None))
        let unset = AppSettingsPatch {
            default_download_dir: Some(None),
            ..Default::default()
        };
        let s2 = store.apply(unset).unwrap();
        assert_eq!(s2.default_download_dir, None);

        // `None` (yokmuş gibi) bırakırsa değiştirmez
        let no_op = AppSettingsPatch::default();
        let s3 = store.apply(no_op).unwrap();
        assert_eq!(s3.default_download_dir, None);
    }
}
