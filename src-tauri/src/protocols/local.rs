//! Local filesystem `ProtocolAdapter` — Bölüm 9.1 trait'inin ilk gerçek
//! implementasyonu. Bölüm 12 (Filesystem Edge-Case Matrisi) yardımcıları
//! şimdilik gömülü; Faz 2 ilerledikçe `fs_edge` modülüne taşınacak.
//!
//! ## Davranış
//!
//! - `connect` profile'dan `root` path'i okur, var olduğunu doğrular,
//!   relative remote yollarını root'a bağlar.
//! - `list_dir` `tokio_stream::wrappers::ReadDirStream` üzerinden Stream döner;
//!   `Vec` kullanılmaz (Bölüm 9.1 paginated streaming kuralı).
//! - `upload`/`download` local-to-local kopya. **Atomic finalization** (Bölüm
//!   14.2): hedef `{target}.dtransfer_tmp` olarak yazılır, fsync edilir, sonra
//!   atomic rename ile final isme alınır.
//! - `fsync` politikası: DataOnly default — written file fsync edilir; parent
//!   dir fsync POSIX'te yapılır (Bölüm 14.6), Windows'ta NTFS rename atomik
//!   olduğu için skip.
//! - **Path traversal koruması**: `..` ile root dışına çıkan istekler
//!   `Authorization` hatası döner.
//!
//! ## Henüz YOK (sonraki adımlar)
//!
//! - Symlink politikası (Bölüm 12.1) — şu an symlink olduğu gibi takip edilir
//! - Unicode normalization (Bölüm 12.2) — byte-exact compare
//! - Windows reserved names (Bölüm 12.4) — sanitize_for_target çağrılmaz
//! - AV lock micro-retry (Bölüm 14.2) — yalnızca tek rename denemesi
//! - Sparse file (Bölüm 14.5)
//! - Cancellation token threading — engine layer'da race edilecek

use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::Instant;
use tokio_stream::wrappers::ReadDirStream;

use crate::errors::TransferError;

use super::adapter::{ConnectionProfile, ProtocolAdapter};
use super::types::{
    AdapterCapabilities, ListOpts, LocalPath, ProgressSender, ProgressTick, ProtocolInfo,
    RemoteEntry, RemoteEntryKind, RemotePath, TransferOptions, TransferResult, TransferStats,
};

const TMP_SUFFIX: &str = ".dtransfer_tmp";
/// Streaming I/O buffer. Disk throughput'una göre 1 MiB optimal — daha büyüğü
/// progress granularity'sini düşürür, daha küçüğü syscall fırtınası yaratır.
const COPY_BUF_SIZE: usize = 1024 * 1024;

#[derive(Default)]
pub struct LocalAdapter {
    root: Option<PathBuf>,
}

impl LocalAdapter {
    pub fn new() -> Self {
        Self { root: None }
    }

    fn require_root(&self) -> Result<&Path, TransferError> {
        self.root
            .as_deref()
            .ok_or_else(|| TransferError::Protocol {
                message: "LocalAdapter not connected".into(),
            })
    }

    /// Remote yolunu root altına güvenli şekilde çöz. `..` ile dışarı çıkış
    /// reddedilir (Bölüm 12 path traversal koruması).
    fn resolve(&self, remote: &RemotePath) -> Result<PathBuf, TransferError> {
        let root = self.require_root()?;
        let rel = Path::new(remote.as_str());

        // Absolute path veya prefix ile root'u override eden isteklere izin verme.
        let stripped = rel
            .strip_prefix("/")
            .or_else(|_| rel.strip_prefix("\\"))
            .unwrap_or(rel);

        let mut resolved = root.to_path_buf();
        for component in stripped.components() {
            match component {
                Component::Normal(part) => resolved.push(part),
                Component::CurDir => continue,
                Component::ParentDir => {
                    return Err(TransferError::Authorization {
                        path: remote.as_str().into(),
                    });
                }
                Component::RootDir | Component::Prefix(_) => continue,
            }
        }

        if !resolved.starts_with(root) {
            return Err(TransferError::Authorization {
                path: remote.as_str().into(),
            });
        }
        Ok(resolved)
    }

    async fn entry_from_metadata(
        path: PathBuf,
        name: String,
        meta: std::fs::Metadata,
    ) -> RemoteEntry {
        let kind = if meta.is_dir() {
            RemoteEntryKind::Directory
        } else if meta.is_file() {
            RemoteEntryKind::File
        } else if meta.file_type().is_symlink() {
            RemoteEntryKind::Symlink
        } else {
            RemoteEntryKind::Other
        };

        RemoteEntry {
            path: RemotePath::new(path.to_string_lossy()),
            name,
            kind,
            size: meta.is_file().then(|| meta.len()),
            modified: meta.modified().ok(),
            remote_hash: None,
        }
    }
}

fn map_io_error(err: std::io::Error, path: &str) -> TransferError {
    use std::io::ErrorKind as K;
    match err.kind() {
        K::NotFound => TransferError::NotFound { path: path.into() },
        K::PermissionDenied => TransferError::Authorization { path: path.into() },
        _ => TransferError::Io(err),
    }
}

/// fsync hedef dosya + (POSIX'te) parent dir — Bölüm 14.6 DataOnly + parent dir.
#[cfg(unix)]
async fn fsync_with_parent(file: &File, target: &Path) -> std::io::Result<()> {
    file.sync_data().await?;
    if let Some(parent) = target.parent() {
        let dir = File::open(parent).await?;
        dir.sync_all().await?;
    }
    Ok(())
}

#[cfg(not(unix))]
async fn fsync_with_parent(file: &File, _target: &Path) -> std::io::Result<()> {
    // Windows: NTFS MoveFileEx atomik; parent dir fsync syscall'ı yok.
    file.sync_data().await
}

#[async_trait]
impl ProtocolAdapter for LocalAdapter {
    async fn connect(&mut self, profile: &ConnectionProfile) -> Result<(), TransferError> {
        let root = profile
            .get("root")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TransferError::Protocol {
                message: "LocalAdapter profile missing 'root' field".into(),
            })?;
        let path = PathBuf::from(root);
        let meta = fs::metadata(&path).await.map_err(|e| map_io_error(e, root))?;
        if !meta.is_dir() {
            return Err(TransferError::Protocol {
                message: format!("LocalAdapter 'root' is not a directory: {root}"),
            });
        }
        // Symlink-resolved canonical path — root traversal kontrolü için.
        let canonical = fs::canonicalize(&path)
            .await
            .map_err(|e| map_io_error(e, root))?;
        self.root = Some(canonical);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransferError> {
        self.root = None;
        Ok(())
    }

    fn list_dir(
        &self,
        path: &RemotePath,
        opts: ListOpts,
    ) -> Pin<Box<dyn Stream<Item = Result<RemoteEntry, TransferError>> + Send + '_>> {
        let resolved = match self.resolve(path) {
            Ok(p) => p,
            Err(e) => return Box::pin(stream::iter(std::iter::once(Err(e)))),
        };
        let include_hidden = opts.include_hidden;

        Box::pin(async_stream::try_stream! {
            let read_dir = fs::read_dir(&resolved)
                .await
                .map_err(|e| map_io_error(e, &resolved.to_string_lossy()))?;
            let mut entries = ReadDirStream::new(read_dir);
            while let Some(item) = entries.next().await {
                let dir_entry = item.map_err(TransferError::Io)?;
                let name = dir_entry.file_name().to_string_lossy().into_owned();
                if !include_hidden && name.starts_with('.') {
                    continue;
                }
                let entry_path = dir_entry.path();
                let meta = dir_entry.metadata().await.map_err(TransferError::Io)?;
                yield LocalAdapter::entry_from_metadata(entry_path, name, meta).await;
            }
        })
    }

    async fn stat(&self, path: &RemotePath) -> Result<RemoteEntry, TransferError> {
        let resolved = self.resolve(path)?;
        let meta = fs::metadata(&resolved)
            .await
            .map_err(|e| map_io_error(e, path.as_str()))?;
        let name = resolved
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self::entry_from_metadata(resolved, name, meta).await)
    }

    async fn upload(
        &self,
        local: &LocalPath,
        remote: &RemotePath,
        opts: &TransferOptions,
        tx: ProgressSender,
    ) -> Result<TransferResult, TransferError> {
        copy_to(local.as_path(), &self.resolve(remote)?, opts, tx).await
    }

    async fn download(
        &self,
        remote: &RemotePath,
        local: &LocalPath,
        opts: &TransferOptions,
        tx: ProgressSender,
    ) -> Result<TransferResult, TransferError> {
        copy_to(&self.resolve(remote)?, local.as_path(), opts, tx).await
    }

    async fn delete(&self, path: &RemotePath) -> Result<(), TransferError> {
        let resolved = self.resolve(path)?;
        let meta = fs::metadata(&resolved)
            .await
            .map_err(|e| map_io_error(e, path.as_str()))?;
        if meta.is_dir() {
            fs::remove_dir_all(&resolved)
                .await
                .map_err(|e| map_io_error(e, path.as_str()))
        } else {
            fs::remove_file(&resolved)
                .await
                .map_err(|e| map_io_error(e, path.as_str()))
        }
    }

    async fn mkdir(&self, path: &RemotePath) -> Result<(), TransferError> {
        let resolved = self.resolve(path)?;
        fs::create_dir_all(&resolved)
            .await
            .map_err(|e| map_io_error(e, path.as_str()))
    }

    async fn rename(
        &self,
        from: &RemotePath,
        to: &RemotePath,
    ) -> Result<(), TransferError> {
        let from = self.resolve(from)?;
        let to = self.resolve(to)?;
        fs::rename(&from, &to)
            .await
            .map_err(|e| map_io_error(e, &from.to_string_lossy()))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            supports_byte_range: true,
            supports_remote_checksum: true,
            supports_server_side_rename: true,
            supports_symlinks: cfg!(unix),
            supports_resume: true,
            supports_multipart: true,
            max_parallel_sessions: 16,
        }
    }

    fn protocol_info(&self) -> ProtocolInfo {
        ProtocolInfo::Local
    }
}

/// Atomic copy `src` → `dst` with progress emission.
///
/// 1. Hedef parent dir create_dir_all.
/// 2. `{dst}.dtransfer_tmp` aç.
/// 3. 1 MiB buffer ile chunk chunk kopyala, her chunk sonunda `ProgressTick`
///    emit et.
/// 4. fsync_with_parent (Bölüm 14.6 DataOnly + parent dir on POSIX).
/// 5. atomic rename → `dst`.
async fn copy_to(
    src: &Path,
    dst: &Path,
    _opts: &TransferOptions,
    tx: ProgressSender,
) -> Result<TransferResult, TransferError> {
    let started = Instant::now();
    let src_meta = fs::metadata(src)
        .await
        .map_err(|e| map_io_error(e, &src.to_string_lossy()))?;
    let total = src_meta.len();

    if let Some(parent) = dst.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| map_io_error(e, &parent.to_string_lossy()))?;
        }
    }

    let tmp_path = with_suffix(dst, TMP_SUFFIX);
    // Mevcut .dtransfer_tmp varsa (önceki crash) sil — Bölüm 28 orphan cleanup
    // resmî mekanizmasına evrilecek; şimdilik best-effort.
    let _ = fs::remove_file(&tmp_path).await;

    let mut reader = File::open(src)
        .await
        .map_err(|e| map_io_error(e, &src.to_string_lossy()))?;
    let mut writer = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)
        .await
        .map_err(|e| map_io_error(e, &tmp_path.to_string_lossy()))?;

    let mut buf = vec![0u8; COPY_BUF_SIZE];
    let mut bytes_done: u64 = 0;
    let mut chunk_index: u32 = 0;

    loop {
        let n = reader.read(&mut buf).await.map_err(TransferError::Io)?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&buf[..n])
            .await
            .map_err(TransferError::Io)?;
        bytes_done += n as u64;
        chunk_index = chunk_index.wrapping_add(1);
        // Send is non-blocking; receiver yoksa (Faz 1 yok), drop'a izin ver.
        let _ = tx
            .send(ProgressTick {
                chunk_index,
                bytes_done,
                bytes_total: total,
            })
            .await;
    }

    fsync_with_parent(&writer, &tmp_path)
        .await
        .map_err(TransferError::Io)?;
    drop(writer);

    fs::rename(&tmp_path, dst)
        .await
        .map_err(|e| map_io_error(e, &dst.to_string_lossy()))?;

    let duration = started.elapsed();
    let stats = TransferStats {
        bytes_transferred: bytes_done,
        duration_ms: duration.as_millis() as u64,
        avg_speed_bps: throughput_bps(bytes_done, duration),
        checksum: None,
        chunks_retried: 0,
    };

    Ok(TransferResult {
        stats,
        remote_metadata: None,
    })
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn throughput_bps(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return 0.0;
    }
    bytes as f64 / secs
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use serde_json::json;
    use std::fs::create_dir_all;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    async fn connected(root: &Path) -> LocalAdapter {
        let mut adapter = LocalAdapter::new();
        adapter
            .connect(&json!({ "root": root.to_str().unwrap() }))
            .await
            .expect("connect");
        adapter
    }

    fn progress_channel() -> (ProgressSender, mpsc::Receiver<ProgressTick>) {
        mpsc::channel(64)
    }

    #[tokio::test]
    async fn connect_rejects_missing_root() {
        let mut adapter = LocalAdapter::new();
        let err = adapter
            .connect(&json!({ "root": "C:\\definitely\\does\\not\\exist\\dtransfer-test" }))
            .await
            .expect_err("expected NotFound");
        assert!(matches!(err, TransferError::NotFound { .. }));
    }

    #[tokio::test]
    async fn connect_rejects_non_directory_root() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("notdir.txt");
        std::fs::write(&file, b"x").unwrap();
        let mut adapter = LocalAdapter::new();
        let err = adapter
            .connect(&json!({ "root": file.to_str().unwrap() }))
            .await
            .expect_err("expected non-dir error");
        assert!(matches!(err, TransferError::Protocol { .. }));
    }

    #[tokio::test]
    async fn list_dir_streams_entries() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"a").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"bb").unwrap();
        create_dir_all(dir.path().join("nested")).unwrap();

        let adapter = connected(dir.path()).await;
        let mut stream = adapter.list_dir(&RemotePath::new("/"), ListOpts::default());
        let mut names = Vec::new();
        while let Some(item) = stream.next().await {
            names.push(item.unwrap().name);
        }
        names.sort();
        assert_eq!(names, vec!["a.txt", "b.txt", "nested"]);
    }

    #[tokio::test]
    async fn list_dir_hides_dotfiles_by_default() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("visible"), b"v").unwrap();
        std::fs::write(dir.path().join(".hidden"), b"h").unwrap();

        let adapter = connected(dir.path()).await;
        let mut stream = adapter.list_dir(&RemotePath::new("/"), ListOpts::default());
        let mut names = Vec::new();
        while let Some(item) = stream.next().await {
            names.push(item.unwrap().name);
        }
        assert_eq!(names, vec!["visible"]);
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let dir = tempdir().unwrap();
        let adapter = connected(dir.path()).await;
        let err = adapter
            .stat(&RemotePath::new("../../../etc/passwd"))
            .await
            .expect_err("expected Authorization");
        assert!(matches!(err, TransferError::Authorization { .. }));
    }

    #[tokio::test]
    async fn upload_writes_atomically_with_progress() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let src = src_dir.path().join("source.bin");
        let payload = vec![0xABu8; 3 * COPY_BUF_SIZE + 12_345]; // 3+ chunks
        std::fs::write(&src, &payload).unwrap();

        let adapter = connected(dst_dir.path()).await;
        let (tx, mut rx) = progress_channel();

        let collector = tokio::spawn(async move {
            let mut ticks = Vec::new();
            while let Some(tick) = rx.recv().await {
                ticks.push(tick);
            }
            ticks
        });

        let result = adapter
            .upload(
                &LocalPath::new(&src),
                &RemotePath::new("out/dest.bin"),
                &TransferOptions::default(),
                tx,
            )
            .await
            .unwrap();
        drop(adapter); // tx çoğaltılmadı; üst drop progress collector'ı kapatır

        let ticks = collector.await.unwrap();
        assert!(ticks.len() >= 3, "at least one tick per buffer");
        let last = ticks.last().unwrap();
        assert_eq!(last.bytes_done, payload.len() as u64);
        assert_eq!(last.bytes_total, payload.len() as u64);

        let written = std::fs::read(dst_dir.path().join("out/dest.bin")).unwrap();
        assert_eq!(written.len(), payload.len());
        assert_eq!(&written[..16], &payload[..16]);

        assert_eq!(result.stats.bytes_transferred, payload.len() as u64);
        assert_eq!(result.stats.chunks_retried, 0);

        // .dtransfer_tmp temizlenmiş olmalı (rename sonrası)
        let tmp = dst_dir.path().join("out/dest.bin.dtransfer_tmp");
        assert!(!tmp.exists(), "tmp file must be renamed away");
    }

    #[tokio::test]
    async fn download_reverses_direction() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let remote_file = src_dir.path().join("remote.txt");
        std::fs::write(&remote_file, b"hello dtransfer").unwrap();

        let adapter = connected(src_dir.path()).await;
        let local_target = dst_dir.path().join("local-copy.txt");
        let (tx, _rx) = progress_channel();

        adapter
            .download(
                &RemotePath::new("remote.txt"),
                &LocalPath::new(&local_target),
                &TransferOptions::default(),
                tx,
            )
            .await
            .unwrap();

        let body = std::fs::read_to_string(&local_target).unwrap();
        assert_eq!(body, "hello dtransfer");
    }

    #[tokio::test]
    async fn mkdir_then_rename_then_delete() {
        let dir = tempdir().unwrap();
        let adapter = connected(dir.path()).await;

        adapter.mkdir(&RemotePath::new("a/b/c")).await.unwrap();
        assert!(dir.path().join("a/b/c").is_dir());

        adapter
            .rename(&RemotePath::new("a/b/c"), &RemotePath::new("a/b/d"))
            .await
            .unwrap();
        assert!(!dir.path().join("a/b/c").exists());
        assert!(dir.path().join("a/b/d").is_dir());

        adapter.delete(&RemotePath::new("a")).await.unwrap();
        assert!(!dir.path().join("a").exists());
    }

    #[tokio::test]
    async fn capabilities_match_local_fs_truth() {
        let adapter = LocalAdapter::new();
        let caps = adapter.capabilities();
        assert!(caps.supports_byte_range);
        assert!(caps.supports_resume);
        assert!(caps.supports_server_side_rename);
        assert_eq!(caps.supports_symlinks, cfg!(unix));
    }
}
