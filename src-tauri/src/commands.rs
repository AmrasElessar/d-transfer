//! Tauri IPC command layer. Faz 1'de yalnızca status uçları — asıl transfer
//! command'ları (start_transfer, pause_transfer, list_remote, ...) Faz 2'de.

use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;
use uuid::Uuid;

use crate::engine::TransferDirection;
use crate::errors::TransferError;
use crate::events::TransferState;
use crate::profiles::{ConnectionProfile, ProfileProtocol, KIND_PASSWORD};
use crate::protocols::{AdapterCapabilities, ListOpts, LocalAdapter, ProtocolAdapter, RemotePath};
use crate::queue::PersistedTransferTask;
use crate::settings::{AppSettings, AppSettingsPatch};
use crate::AppState;
use futures::StreamExt;
use tracing::warn;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub running: bool,
    pub cancelled: bool,
    pub event_subscribers: usize,
}

#[tauri::command]
pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> EngineStatus {
    EngineStatus {
        running: true,
        cancelled: state.root_cancel.is_cancelled(),
        event_subscribers: state.events.subscriber_count(),
    }
}

/// Local adapter'a verilen root path ile bağlanmayı dener ve capability
/// raporunu döner. UI'ın "Test Local Adapter" debug akışı tarafından çağrılır;
/// herhangi bir kalıcı state bırakmaz — adapter call sonunda drop edilir.
#[tauri::command]
pub async fn probe_local_adapter(root: String) -> Result<AdapterCapabilities, TransferError> {
    let mut adapter = LocalAdapter::new();
    adapter.connect(&json!({ "root": root })).await?;
    let caps = adapter.capabilities();
    let _ = adapter.disconnect().await;
    Ok(caps)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTransferRequest {
    pub root: String,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTransferReport {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub duration_ms: u64,
    pub avg_speed_bps: f64,
}

/// Local-to-local transfer'i kuyruğa ekler ve completion'ı bekler.
///
/// Faz 3 entegrasyonu: artık doğrudan `TransferEngine.submit()` çağırmıyor.
/// Pipeline şu:
///   1. `LocalAdapterFactory.register_local()` → geçici profile_id.
///   2. `PersistedTransferTask`'i `Queued` state'iyle queue.db'ye yaz.
///   3. `QueueScheduler.submit()` task'i sıraya alıp scheduler'ı uyandırır,
///      `oneshot::Receiver<TransferOutcome>` döner.
///   4. IPC outcome'u bekler — terminal state UI'a rapor edilir.
///   5. profile_id `unregister` edilir (in-memory cleanup).
///
/// Bu yolla artık transferler crash-resilient: `Active` durumdayken process
/// ölürse startup recovery `Active → Queued`'a çevirir, sıradaki çalıştırmada
/// scheduler yeniden dispatch eder.
#[tauri::command]
pub async fn start_local_transfer(
    request: LocalTransferRequest,
    state: State<'_, AppState>,
) -> Result<LocalTransferReport, TransferError> {
    let profile_id = state
        .factory
        .register_local(PathBuf::from(&request.root));

    let now = Utc::now();
    let task = PersistedTransferTask {
        id: Uuid::new_v4(),
        profile_id,
        direction: TransferDirection::Upload,
        state: TransferState::Queued,
        priority: 0,
        local_path: PathBuf::from(&request.source),
        remote_path: request.destination.clone(),
        bytes_total: 0,
        bytes_done: 0,
        chunk_size: 8 * 1024 * 1024,
        retry_count: 0,
        last_error: None,
        schema_version: 1,
        created_at: now,
        updated_at: now,
        started_at: None,
        completed_at: None,
    };
    let transfer_id = task.id;

    let outcome_rx = state.scheduler.submit(task).await?;

    let outcome = outcome_rx.await.map_err(|_| TransferError::Protocol {
        message: "scheduler closed before outcome".into(),
    })?;

    // Adapter profile'ını temizle — bir sonraki transfer yeni id alır.
    state.factory.unregister(profile_id);

    match outcome.final_state {
        TransferState::Completed => Ok(LocalTransferReport {
            transfer_id: transfer_id.to_string(),
            bytes_transferred: outcome.bytes_transferred,
            duration_ms: outcome.duration_ms,
            avg_speed_bps: outcome.avg_speed_bps,
        }),
        TransferState::Cancelled => Err(TransferError::Cancelled),
        _ => {
            let message = outcome
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "transfer failed without error detail".into());
            Err(TransferError::Protocol { message })
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.snapshot()
}

#[tauri::command]
pub fn update_settings(
    patch: AppSettingsPatch,
    state: State<'_, AppState>,
) -> Result<AppSettings, TransferError> {
    state.settings.apply(patch).map_err(|e| TransferError::Protocol {
        message: format!("settings persist failed: {e}"),
    })
}

// ============================================================================
// Local filesystem browser IPC (Bölüm 19 — DualPane local pane).
// ============================================================================
//
// `LocalAdapter::list_dir` profile-bound (root altında jail) iken UI browser
// kullanıcının her yere navigate etmesini ister — A:\ → C:\Users → projects,
// arbitrary drive root. Bu yüzden adapter'dan ayrı, traversal koruması
// içermeyen bir UI-only komut. Permission denied'da `Authorization` error
// dönmesi UI'ın hata banner'ı için yeterli.

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLocalDirRequest {
    pub path: String,
    pub include_hidden: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLocalDirResponse {
    /// Canonical absolute path of the listed directory.
    pub path: String,
    /// Parent directory absolute path, or `None` if at drive/filesystem root.
    pub parent: Option<String>,
    pub entries: Vec<LocalEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEntry {
    pub name: String,
    /// Absolute path; UI uses this for navigation and selection identity.
    pub path: String,
    /// One of `"file"`, `"directory"`, `"symlink"`, `"other"`.
    pub kind: String,
    /// File size in bytes. `None` for directories and unknowable entries.
    pub size: Option<u64>,
    /// Modification time as unix epoch milliseconds. Signed to keep parity
    /// with JS `Date` (negative pre-1970 timestamps theoretically possible
    /// on some filesystems).
    pub modified_unix_ms: Option<i64>,
    pub is_hidden: bool,
}

fn map_local_io_error(err: std::io::Error, path: &str) -> TransferError {
    use std::io::ErrorKind as K;
    match err.kind() {
        K::NotFound => TransferError::NotFound { path: path.into() },
        K::PermissionDenied => TransferError::Authorization { path: path.into() },
        _ => TransferError::Io(err),
    }
}

fn expand_user_path(input: &str) -> PathBuf {
    // `~` veya boş input → home dir. Tauri'nin built-in path plugin'ini
    // kullanmamak için kendi expand'imiz; UI ergonomisi için tek seviye.
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "~" {
        if let Some(home) = resolve_home_dir() {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = resolve_home_dir() {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(trimmed)
}

fn resolve_home_dir() -> Option<String> {
    // dirs crate'ini eklemeden std env üzerinden — Windows `USERPROFILE`,
    // POSIX `HOME`. Linux'ta HOME unset olabiliyor (systemd minimal env);
    // o durumda `None` dönmek UI'a default davranışı tetikletir.
    #[cfg(windows)]
    {
        if let Ok(v) = std::env::var("USERPROFILE") {
            if !v.is_empty() {
                return Some(v);
            }
        }
        // Bazı domain ortamlarında USERPROFILE eksik; HOMEDRIVE+HOMEPATH fallback.
        let drive = std::env::var("HOMEDRIVE").ok();
        let path = std::env::var("HOMEPATH").ok();
        if let (Some(d), Some(p)) = (drive, path) {
            if !d.is_empty() && !p.is_empty() {
                return Some(format!("{d}{p}"));
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().filter(|v| !v.is_empty())
    }
}

#[cfg(windows)]
fn is_hidden_meta(name: &str, meta: &std::fs::Metadata) -> bool {
    // Windows hidden semantiği POSIX'ten farklı: dosya adının `.` ile başlaması
    // hidden anlamına gelmez (Cygwin/WSL etkilemez). NTFS attribute bit
    // FILE_ATTRIBUTE_HIDDEN (0x2) primary truth.
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    if meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
        return true;
    }
    // Cross-platform consistency: dotfile gelenek olarak hidden gösterilsin
    // (developer workflow — `.git`, `.env` vb.).
    name.starts_with('.')
}

#[cfg(not(windows))]
fn is_hidden_meta(name: &str, _meta: &std::fs::Metadata) -> bool {
    // POSIX hidden = name starts with `.`. Filesystem attribute yok.
    name.starts_with('.')
}

fn system_time_to_unix_ms(time: std::time::SystemTime) -> Option<i64> {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_millis()).ok(),
        Err(e) => {
            // Pre-1970 timestamp — bazı eski filesystem entry'lerinde olur.
            let secs = e.duration().as_secs() as i64;
            let nanos = e.duration().subsec_millis() as i64;
            Some(-(secs * 1000 + nanos))
        }
    }
}

#[tauri::command]
pub async fn list_local_dir(
    request: ListLocalDirRequest,
) -> Result<ListLocalDirResponse, TransferError> {
    let target = expand_user_path(&request.path);
    // Canonicalize: symlink'leri çöz, normalize forward/backward slash; aynı
    // path için tutarlı string döner. Drive root (`C:\`) Windows'ta zaten
    // canonical.
    let canonical = tokio::fs::canonicalize(&target)
        .await
        .map_err(|e| map_local_io_error(e, &target.to_string_lossy()))?;

    let meta = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| map_local_io_error(e, &canonical.to_string_lossy()))?;
    if !meta.is_dir() {
        return Err(TransferError::Protocol {
            message: format!("not a directory: {}", canonical.to_string_lossy()),
        });
    }

    let mut read_dir = tokio::fs::read_dir(&canonical)
        .await
        .map_err(|e| map_local_io_error(e, &canonical.to_string_lossy()))?;

    let mut entries: Vec<LocalEntry> = Vec::new();
    loop {
        let item = read_dir
            .next_entry()
            .await
            .map_err(|e| map_local_io_error(e, &canonical.to_string_lossy()))?;
        let Some(entry) = item else { break };
        let name = entry.file_name().to_string_lossy().into_owned();
        // entry.metadata() symlink'i takip etmez; UI symlink "↪" göstermek için
        // bunu istiyoruz. is_file/is_dir resolved target'ı yansıtmaz — kind
        // dedicated check ile belirlenir.
        let entry_meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue, // best-effort: locked dosyalar listede görünmez
        };

        let kind = if entry_meta.is_dir() {
            "directory"
        } else if entry_meta.is_file() {
            "file"
        } else if entry_meta.file_type().is_symlink() {
            "symlink"
        } else {
            "other"
        };

        let is_hidden = is_hidden_meta(&name, &entry_meta);
        if !request.include_hidden && is_hidden {
            continue;
        }

        let abs_path = entry.path().to_string_lossy().into_owned();
        let size = if entry_meta.is_file() {
            Some(entry_meta.len())
        } else {
            None
        };
        let modified_unix_ms = entry_meta
            .modified()
            .ok()
            .and_then(system_time_to_unix_ms);

        entries.push(LocalEntry {
            name,
            path: abs_path,
            kind: kind.into(),
            size,
            modified_unix_ms,
            is_hidden,
        });
    }

    // Sort: directories first, then by name (case-insensitive). Locale-aware
    // collation overkill; ASCII lowercase compare yeterli — Türkçe i/ı edge
    // case'lerini UI'da ileride özelleştirebiliriz.
    entries.sort_by(|a, b| {
        let a_dir = a.kind == "directory";
        let b_dir = b.kind == "directory";
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    let parent = canonical
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        // canonicalize "\\?\C:" prefix verir bazen — parent same string olursa
        // root sayılır, None döneriz. Path::parent zaten `C:\` için None döner.
        .filter(|p| !p.is_empty());

    Ok(ListLocalDirResponse {
        path: canonical.to_string_lossy().into_owned(),
        parent,
        entries,
    })
}

#[tauri::command]
pub fn list_local_drives() -> Vec<String> {
    // Windows: A..Z drive letter probe. `tokio::fs::metadata` async runtime
    // gerektirir; std sync metadata yeterli çünkü local probe << 1ms / drive.
    #[cfg(windows)]
    {
        let mut out = Vec::new();
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            if std::fs::metadata(&root).is_ok() {
                out.push(root);
            }
        }
        out
    }
    #[cfg(not(windows))]
    {
        // POSIX: tek kök, mountpoint browsing UI'ı şimdilik desteklemiyor.
        vec!["/".into()]
    }
}

#[tauri::command]
pub fn home_dir() -> Option<String> {
    resolve_home_dir()
}

// ============================================================================
// Queue / Transfer IPC (Bölüm 15).
// ============================================================================
//
// UI'ın QueuePanel'da gösterdiği satırlar. `list_transfers` initial load
// içindir; sonradan engine event akışı (transferStateChanged/Progress/...) ile
// güncellenir.

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferDto {
    pub id: String,
    pub profile_id: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub priority: i32,
    pub local_path: String,
    pub remote_path: String,
    pub bytes_total: u64,
    pub bytes_done: u64,
    pub chunk_size: u64,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<PersistedTransferTask> for TransferDto {
    fn from(t: PersistedTransferTask) -> Self {
        Self {
            id: t.id.to_string(),
            profile_id: t.profile_id.to_string(),
            direction: t.direction,
            state: t.state,
            priority: t.priority,
            local_path: t.local_path.to_string_lossy().into_owned(),
            remote_path: t.remote_path,
            bytes_total: t.bytes_total,
            bytes_done: t.bytes_done,
            chunk_size: t.chunk_size as u64,
            retry_count: t.retry_count,
            last_error: t.last_error,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
            started_at: t.started_at.map(|d| d.to_rfc3339()),
            completed_at: t.completed_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// Queue panel initial load. Default limit 200; UI 200'ü aşarsa kaydırma.
#[tauri::command]
pub async fn list_transfers(
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<TransferDto>, TransferError> {
    let rows = state
        .queue
        .list_all(limit.unwrap_or(200))
        .await
        .map_err(|e| TransferError::Protocol {
            message: format!("list_transfers failed: {e}"),
        })?;
    Ok(rows.into_iter().map(TransferDto::from).collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueTransferRequest {
    pub profile_id: Uuid,
    pub local_path: String,
    pub remote_path: String,
    /// `bytes_total` UI'da biliniyorsa (upload local stat'tan, download remote
    /// list'ten) önceden bildirilir → progress bar hemen yüzde gösterir.
    /// Bilinmiyorsa 0; engine ilk progress tick'inde günceller.
    #[serde(default)]
    pub bytes_total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueTransferResponse {
    pub transfer_id: String,
}

/// Yardımcı: yeni `PersistedTransferTask` üret + scheduler'a submit et.
/// Submit'ten dönen oneshot waiter **kasıtlı olarak drop edilir** — UI fire
/// and forget kullanır, completion engine-event üzerinden gelir.
async fn enqueue_task(
    state: &AppState,
    profile_id: Uuid,
    direction: TransferDirection,
    local_path: String,
    remote_path: String,
    bytes_total: u64,
) -> Result<EnqueueTransferResponse, TransferError> {
    // Profile var mı? UnifiedAdapterFactory zaten dispatch sırasında build
    // edecek ama burada erken doğrulama → kullanıcıya hızlı hata feedback'i.
    let profile = state
        .queue
        .profile_get(profile_id)
        .await
        .map_err(|e| TransferError::Protocol {
            message: format!("profile lookup failed: {e}"),
        })?;
    if profile.is_none() && !state.factory.has(profile_id) {
        return Err(TransferError::NotFound {
            path: profile_id.to_string(),
        });
    }

    let chunk_size_bytes = (state.settings.snapshot().default_chunk_size_mb as usize)
        .saturating_mul(1024 * 1024);
    let now = Utc::now();
    let task = PersistedTransferTask {
        id: Uuid::new_v4(),
        profile_id,
        direction,
        state: TransferState::Queued,
        priority: 0,
        local_path: PathBuf::from(&local_path),
        remote_path: remote_path.clone(),
        bytes_total,
        bytes_done: 0,
        chunk_size: chunk_size_bytes,
        retry_count: 0,
        last_error: None,
        schema_version: 1,
        created_at: now,
        updated_at: now,
        started_at: None,
        completed_at: None,
    };
    let transfer_id = task.id;
    // Waiter'ı drop ediyoruz; UI engine event'leriyle tamamlanmayı takip eder.
    let _waiter = state.scheduler.submit(task).await?;

    Ok(EnqueueTransferResponse {
        transfer_id: transfer_id.to_string(),
    })
}

#[tauri::command]
pub async fn enqueue_upload(
    request: EnqueueTransferRequest,
    state: State<'_, AppState>,
) -> Result<EnqueueTransferResponse, TransferError> {
    enqueue_task(
        &state,
        request.profile_id,
        TransferDirection::Upload,
        request.local_path,
        request.remote_path,
        request.bytes_total,
    )
    .await
}

#[tauri::command]
pub async fn enqueue_download(
    request: EnqueueTransferRequest,
    state: State<'_, AppState>,
) -> Result<EnqueueTransferResponse, TransferError> {
    enqueue_task(
        &state,
        request.profile_id,
        TransferDirection::Download,
        request.local_path,
        request.remote_path,
        request.bytes_total,
    )
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTransferResponse {
    /// `true` ise scheduler aktif transferi buldu ve cancel sinyali yolladı;
    /// `false` zaten terminal state'te idi.
    pub cancelled: bool,
}

/// Aktif (Active/Verifying/Finalizing) bir transferi iptal et. Engine
/// cooperative cancellation kullanır — sinyal ulaşana kadar geçen kısa süre
/// içinde transfer biraz daha veri yazmış olabilir.
#[tauri::command]
pub async fn cancel_transfer(
    transfer_id: Uuid,
    state: State<'_, AppState>,
) -> Result<CancelTransferResponse, TransferError> {
    let cancelled = state.scheduler.cancel_transfer(transfer_id);
    Ok(CancelTransferResponse { cancelled })
}

// ============================================================================
// ConnectionProfile IPC (Bölüm 25).
// ============================================================================
//
// Profil meta'sı `queue.db.profiles` tablosunda; sırlar OS keystore'da. UI'dan
// gelen `secret` parametresi `Option<String>` semantiği:
//
//   None           → sır alanına dokunma (update'te eski değer korunur).
//   Some("")       → sırrı sil (auth metodu None'a dönüldüğünde tipik).
//   Some(value)    → keystore'a yaz/overwrite.
//
// `auth_method` payload'unun bir alanı olarak DB'ye yazılır — secret yokluğu
// auth metodunu otomatik değiştirmez (UI'ın açık kararı).

/// Yardımcı: secret değerinin keystore mutasyonunu sırasıyla uygula. Hata
/// olursa `TransferError::Authentication` ile wrap'le çünkü UI bunu
/// "kimlik bilgisi sorununu çöz" şeklinde sunabiliyor.
fn apply_secret_to_vault(
    state: &AppState,
    profile_id: Uuid,
    secret: Option<String>,
) -> Result<(), TransferError> {
    let Some(value) = secret else {
        return Ok(());
    };
    if value.is_empty() {
        state
            .credentials
            .delete(profile_id, KIND_PASSWORD)
            .map_err(|e| TransferError::Authentication {
                reason: format!("vault delete failed: {e}"),
            })?;
    } else {
        state
            .credentials
            .store(profile_id, KIND_PASSWORD, &value)
            .map_err(|e| TransferError::Authentication {
                reason: format!("vault store failed: {e}"),
            })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionProfile>, TransferError> {
    state.queue.profile_list().await.map_err(|e| {
        TransferError::Protocol {
            message: format!("profile list failed: {e}"),
        }
    })
}

#[tauri::command]
pub async fn create_profile(
    profile: ConnectionProfile,
    secret: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConnectionProfile, TransferError> {
    // Önce DB'ye yaz — başarısızsa keystore'a dokunmuyoruz (sızıntı yok).
    state
        .queue
        .profile_insert(profile.clone())
        .await
        .map_err(|e| TransferError::Protocol {
            message: format!("profile insert failed: {e}"),
        })?;

    if let Err(e) = apply_secret_to_vault(&state, profile.id, secret) {
        // Sır yazılamadıysa DB satırını da geri al — orphan profil bırakma.
        if let Err(rollback) = state.queue.profile_delete(profile.id).await {
            warn!(?rollback, "profile rollback after secret failure also errored");
        }
        return Err(e);
    }

    Ok(profile)
}

#[tauri::command]
pub async fn update_profile(
    profile: ConnectionProfile,
    secret: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConnectionProfile, TransferError> {
    state
        .queue
        .profile_update(profile.clone())
        .await
        .map_err(|e| match e {
            crate::queue::DbError::NotFound(_) => TransferError::NotFound {
                path: profile.id.to_string(),
            },
            other => TransferError::Protocol {
                message: format!("profile update failed: {other}"),
            },
        })?;

    apply_secret_to_vault(&state, profile.id, secret)?;
    Ok(profile)
}

#[tauri::command]
pub async fn delete_profile(
    id: Uuid,
    state: State<'_, AppState>,
) -> Result<(), TransferError> {
    state.queue.profile_delete(id).await.map_err(|e| match e {
        crate::queue::DbError::NotFound(_) => TransferError::NotFound {
            path: id.to_string(),
        },
        other => TransferError::Protocol {
            message: format!("profile delete failed: {other}"),
        },
    })?;

    // Best-effort: keystore'daki sırları temizle. Hatalar warn log; UI'ı
    // bloklamıyoruz çünkü DB tarafı zaten silindi.
    state.credentials.purge_all_known_kinds(id);
    Ok(())
}

/// Geçici adapter inşa eder, connect dener, capabilities döner, disconnect eder.
/// Hiçbir kalıcı state bırakmaz (profile UI henüz kayıt edilmemiş olabilir).
///
/// Faz 1: yalnızca `Local` protokolü destekleniyor; SFTP/S3/WebDAV adapter'ları
/// `sftp-stack` agent'ı ve Faz 2 dispatch genişlemesi gelince bağlanacak. O
/// zamana kadar bunlar `CapabilityNotSupported` döner — UI bunu "yapılandırıldı
/// ama henüz test edilemiyor" mesajıyla göstermeli.
#[tauri::command]
pub async fn test_connection(
    profile: ConnectionProfile,
    secret: Option<String>,
) -> Result<AdapterCapabilities, TransferError> {
    // Secret henüz adapter'a inmediği için suppress _; SFTP entegrasyonunda
    // password-auth path'ine bind edilecek.
    let _ = secret;
    match profile.protocol {
        ProfileProtocol::Local => {
            let root = profile
                .local_root
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| TransferError::Protocol {
                    message: "local profile missing local_root".into(),
                })?;
            let mut adapter = LocalAdapter::new();
            adapter.connect(&json!({ "root": root })).await?;
            let caps = adapter.capabilities();
            let _ = adapter.disconnect().await;
            Ok(caps)
        }
        ProfileProtocol::Sftp | ProfileProtocol::S3 | ProfileProtocol::Webdav => {
            Err(TransferError::CapabilityNotSupported {
                capability: format!(
                    "{} adapter not wired yet (Phase 2)",
                    profile.protocol.as_str()
                ),
            })
        }
    }
}

// ============================================================================
// Remote browser IPC (Bölüm 19 + Faz 4 ConnectionManager).
// ============================================================================
//
// `connect_profile` / `disconnect_profile` / `list_remote_dir` üçlüsü
// `ConnectionManager` üzerinden kalıcı bağlantı havuzunu yönetir. UI tarafı
// active profile değişince connect tetikler, listing'ler aynı Arc adapter
// üzerinden gider; pahalı SSH handshake yalnızca bir kez yapılır.
//
// Secret argümanı YOK — connect_profile profile_id'den yola çıkıp vault'tan
// password çekmeyi `ConnectionManager`'a delege eder. Bu sayede UI'da parola
// state'i tutmuyoruz (sızıntı yüzeyi azalır).

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRemoteDirRequest {
    pub profile_id: Uuid,
    /// Remote path; SFTP'de `/` veya absolute path, Local profil için remote
    /// jail'ine relative. Boş string → kök.
    pub path: String,
    pub include_hidden: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRemoteDirResponse {
    pub path: String,
    /// POSIX parent path; remote_root altında jail edilmez — UI breadcrumb için
    /// bilgilendirici. `/` veya boş kök için `None`.
    pub parent: Option<String>,
    pub entries: Vec<RemoteEntryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEntryDto {
    pub name: String,
    pub path: String,
    /// `"file"` | `"directory"` | `"symlink"` | `"other"`.
    pub kind: String,
    pub size: Option<u64>,
    pub modified_unix_ms: Option<i64>,
    pub is_hidden: bool,
}

async fn load_profile(state: &AppState, id: Uuid) -> Result<ConnectionProfile, TransferError> {
    state
        .queue
        .profile_get(id)
        .await
        .map_err(|e| TransferError::Protocol {
            message: format!("profile get failed: {e}"),
        })?
        .ok_or_else(|| TransferError::NotFound {
            path: id.to_string(),
        })
}

/// Active profile'a connect et + capability raporu döndür. Tekrar çağrılırsa
/// cached adapter'ı kullanır (no-op'a yakın).
#[tauri::command]
pub async fn connect_profile(
    profile_id: Uuid,
    state: State<'_, AppState>,
) -> Result<AdapterCapabilities, TransferError> {
    let profile = load_profile(&state, profile_id).await?;
    let adapter = state.connections.get_or_connect(&profile).await?;
    Ok(adapter.capabilities())
}

#[tauri::command]
pub async fn disconnect_profile(
    profile_id: Uuid,
    state: State<'_, AppState>,
) -> Result<(), TransferError> {
    state.connections.disconnect(profile_id).await
}

/// Remote dizini stream'le → topla, sırala, DTO'ya çevir.
///
/// Stream tüketimi sırasında entry başına hata gelirse ilk hatayı upstream'e
/// yansıtıyoruz (partial list'ten ziyade temiz fail). UI banner gösterir;
/// kullanıcı parent'a çıkıp tekrar deneyebilir.
#[tauri::command]
pub async fn list_remote_dir(
    request: ListRemoteDirRequest,
    state: State<'_, AppState>,
) -> Result<ListRemoteDirResponse, TransferError> {
    let profile = load_profile(&state, request.profile_id).await?;
    let adapter = state.connections.get_or_connect(&profile).await?;

    let opts = ListOpts {
        include_hidden: request.include_hidden,
        ..ListOpts::default()
    };
    let remote_path = RemotePath::new(request.path.clone());

    let mut stream = adapter.list_dir(&remote_path, opts);
    let mut entries: Vec<RemoteEntryDto> = Vec::new();
    while let Some(item) = stream.next().await {
        let entry = item?;
        let kind = match entry.kind {
            crate::protocols::types::RemoteEntryKind::File => "file",
            crate::protocols::types::RemoteEntryKind::Directory => "directory",
            crate::protocols::types::RemoteEntryKind::Symlink => "symlink",
            crate::protocols::types::RemoteEntryKind::Other => "other",
        };
        let modified_unix_ms = entry.modified.and_then(system_time_to_unix_ms);
        let is_hidden = entry.name.starts_with('.');
        if !request.include_hidden && is_hidden {
            // SFTP adapter zaten filtreliyor ama LocalAdapter `ListOpts` üzerinden
            // hidden'ı kabul ediyor; double-check zarar vermez, UI tutarlılığı
            // tek noktada toplanır.
            continue;
        }
        entries.push(RemoteEntryDto {
            name: entry.name,
            path: entry.path.0,
            kind: kind.into(),
            size: entry.size,
            modified_unix_ms,
            is_hidden,
        });
    }

    // Sort: directories first, then by name (case-insensitive). LocalPane ile
    // tutarlılık.
    entries.sort_by(|a, b| {
        let a_dir = a.kind == "directory";
        let b_dir = b.kind == "directory";
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    let parent = posix_parent(&request.path);

    Ok(ListRemoteDirResponse {
        path: request.path,
        parent,
        entries,
    })
}

/// POSIX parent path. `/foo/bar` → `Some("/foo")`, `/foo` → `Some("/")`,
/// `/` / `""` / `"."` → `None`. Windows-style backslash'ı **takmıyoruz** —
/// remote path'ler her zaman POSIX kabul edilir (SFTP standardı + S3/WebDAV).
fn posix_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        return None;
    }
    match trimmed.rfind('/') {
        Some(0) => Some("/".into()),
        Some(idx) => Some(trimmed[..idx].into()),
        None => None,
    }
}

#[cfg(test)]
mod remote_browser_tests {
    use super::*;

    #[test]
    fn posix_parent_root_yields_none() {
        assert_eq!(posix_parent("/"), None);
        assert_eq!(posix_parent(""), None);
        assert_eq!(posix_parent("."), None);
    }

    #[test]
    fn posix_parent_first_level_yields_root() {
        assert_eq!(posix_parent("/foo").as_deref(), Some("/"));
        assert_eq!(posix_parent("/foo/").as_deref(), Some("/"));
    }

    #[test]
    fn posix_parent_nested() {
        assert_eq!(posix_parent("/foo/bar").as_deref(), Some("/foo"));
        assert_eq!(posix_parent("/foo/bar/baz").as_deref(), Some("/foo/bar"));
    }

    #[test]
    fn posix_parent_relative() {
        assert_eq!(posix_parent("foo/bar").as_deref(), Some("foo"));
        // Tek bileşen relative path — parent yok.
        assert_eq!(posix_parent("foo"), None);
    }
}
