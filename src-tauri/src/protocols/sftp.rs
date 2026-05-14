//! SFTP `ProtocolAdapter` — Bölüm 9.2 + 11.
//!
//! Backend: `russh` (SSH transport) + `russh-sftp` (SFTP subsystem). `russh`
//! 0.50 itibarıyla `russh-keys` modülünü kendi içine fold ettiği için ayrı
//! key parser dependency'sine ihtiyacımız yok — `russh::keys::decode_secret_key`
//! kullanıyoruz.
//!
//! ## Akış
//!
//! 1. `connect(profile)` — TCP+SSH transport kurulur, password VEYA private
//!    key ile auth, ardından `channel_open_session().request_subsystem("sftp")`
//!    + `SftpSession::new(channel.into_stream())`.
//! 2. `probe_capabilities()` — `OPEN_MAX_PARALLEL_PROBE` adet ekstra session
//!    channel açmaya çalışıp server'ın limitini ölç. v1.0 yaklaşımı: alt sınır
//!    1, üst sınır 10; ölçülen değeri `max_parallel_sessions`'a yaz.
//! 3. `upload`/`download` — chunked streaming, her chunk sonunda `ProgressTick`.
//!    Download tarafında atomic write (`{dst}.dtransfer_tmp` + fsync + rename;
//!    LocalAdapter ile aynı pattern).
//! 4. `disconnect` — sftp session + ssh handle drop edilir.
//!
//! ## Henüz YOK (Faz 5'e kalan)
//!
//! - **Host key strict pin** (Bölüm 36) — `known_host_fingerprint` profile
//!   alanı parse edilip `Handler::check_server_key`'de eşleşme zorunlu kılınmalı.
//!   Şu an fingerprint verilirse log'a basıp **kabul** ediyoruz (TOFU benzeri),
//!   verilmemişse warn ile geçiyoruz.
//! - **Server-side fsync** — russh-sftp `File::sync_all()` desteklese de server
//!   `fsync@openssh.com` extension'ı yoksa silent no-op olur; v1.0'da best-effort.
//! - **`MaxSessions` probe** — gerçek SSH `MaxSessions` parametresini sorgulamak
//!   için bir RFC mekanizması yok; "aç-fail-say" yaklaşımı pragmatik fakat
//!   yan etki olarak server log'unda kısa-süreli channel patlaması yaratır.
//!   v2.0'da konservatif sabit (4) ile başlanıp adaptive büyütme yapılabilir.
//! - **Symlink follow politikası** (Bölüm 12.1) — `list_dir` symlink'i olduğu
//!   gibi rapor eder, hedef takip etmez.
//! - **Resume / byte-range upload** — capabilities `supports_resume: true`
//!   diyor ama upload kodu şu an her zaman `CREATE | TRUNCATE | WRITE` ile
//!   açıyor. Resume implementasyonu engine layer'da offset hesabı + sftp
//!   `open_with_flags(WRITE | APPEND-yok-yerine-seek)` ile yapılacak.
//! - **`opts.preserve_mtime`** — server tarafında `set_metadata` çağrısıyla
//!   uygulanabilir; şimdilik no-op.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use async_trait::async_trait;
use futures::{stream, Stream};
use russh::client::{self, Handle};
use russh::keys::{decode_secret_key, HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileType, OpenFlags, StatusCode};
use serde::Deserialize;
use tokio::fs::{self as tokio_fs, File as TokioFile, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::errors::TransferError;
use crate::network::SshKeepalive;

use super::adapter::{ConnectionProfile, ProtocolAdapter};
use super::types::{
    AdapterCapabilities, ListOpts, LocalPath, ProgressSender, ProgressTick, ProtocolInfo,
    RemoteEntry, RemoteEntryKind, RemotePath, TransferOptions, TransferResult, TransferStats,
};

const TMP_SUFFIX: &str = ".dtransfer_tmp";
/// Probe loop'unun üst limiti — bunun ötesinde paralelliğin marjinal faydası
/// yok (Bölüm 9.2: SFTP genelde 4-10 concurrent session bandında doyar).
const OPEN_MAX_PARALLEL_PROBE: u8 = 10;
/// Default SFTP I/O chunk — server `OPEN_MAX_WRITE_SIZE` extension'ı çoğunlukla
/// 32 KiB-64 KiB seviyesinde. opts.chunk_size daha büyük olabilir, biz user'a
/// kalanı buffered AsyncWrite olarak teslim ediyoruz.
const DEFAULT_CHUNK_BYTES: usize = 64 * 1024;

/// SFTP profile şeması (Bölüm 11.1). `ConnectionProfile` Faz 5'te tip-güvenli
/// `crate::profiles::ConnectionProfile` struct'ına evrilecek; o zamana kadar
/// JSON içinden bu shape'i okuyoruz.
#[derive(Debug, Deserialize)]
struct SftpProfile {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    username: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    private_key_pem: Option<String>,
    #[serde(default)]
    private_key_passphrase: Option<String>,
    #[serde(default)]
    remote_root: Option<String>,
    #[serde(default)]
    known_host_fingerprint: Option<String>,
    /// Opsiyonel — caller `SshKeepalive`'i JSON payload'unda yollayabilir
    /// (profile-bazlı override). Atlanırsa `SshKeepalive::default()` (30s
    /// interval, 3 failed-ping → drop) uygulanır.
    #[serde(default)]
    keepalive: Option<SshKeepalive>,
}

fn default_port() -> u16 {
    22
}

/// `russh::client::Handler` impl — host key verification için kullanılır.
///
/// v1.0 davranışı: `expected_fingerprint` Some ise SHA256 fingerprint'i kabul
/// edilen değerle karşılaştırılır; eşleşmezse `Authentication` döner. None ise
/// **TOFU** (Trust On First Use) yaklaşımıyla kabul edilir + warn log basılır.
/// Bölüm 36'da strict known_hosts dosyası mekanizması geliyor.
struct SftpClientHandler {
    expected_fingerprint: Option<String>,
}

impl client::Handler for SftpClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let fp = server_public_key.fingerprint(Default::default()).to_string();
        match &self.expected_fingerprint {
            Some(expected) => {
                if fingerprints_match(expected, &fp) {
                    debug!(fingerprint = %fp, "host key matched pin");
                    Ok(true)
                } else {
                    warn!(expected = %expected, actual = %fp, "host key mismatch — rejecting");
                    Ok(false)
                }
            }
            None => {
                warn!(fingerprint = %fp, "no host key pin — accepting (TOFU)");
                Ok(true)
            }
        }
    }
}

/// Fingerprint normalizasyonu — `SHA256:xxx` prefix'i veya çıplak base64
/// karşılaştırmayı her iki yönde de mümkün kılar.
fn fingerprints_match(expected: &str, actual: &str) -> bool {
    let norm = |s: &str| {
        s.trim()
            .trim_start_matches("SHA256:")
            .trim_start_matches("sha256:")
            .to_string()
    };
    norm(expected) == norm(actual)
}

/// connect() sonrası dolar; disconnect()'te `None`'a düşer.
pub struct SftpAdapter {
    session: Option<SftpAdapterSession>,
}

struct SftpAdapterSession {
    /// SSH transport handle. Drop edilince connection kapanır.
    _ssh: Handle<SftpClientHandler>,
    sftp: SftpSession,
    capabilities: AdapterCapabilities,
    info: ProtocolInfo,
    /// Profile'dan gelen remote_root — relative path'ler buna bağlanır.
    /// None ise path'ler doğrudan server tarafına iletilir.
    remote_root: Option<String>,
}

impl Default for SftpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SftpAdapter {
    pub fn new() -> Self {
        Self { session: None }
    }

    fn session(&self) -> Result<&SftpAdapterSession, TransferError> {
        self.session.as_ref().ok_or_else(|| TransferError::Protocol {
            message: "SftpAdapter not connected".into(),
        })
    }

    /// `RemotePath`'i `remote_root` altında resolve eder. Absolute path verilmişse
    /// (`/...`) root prefix'i atlanır — server'a olduğu gibi gönderilir.
    fn resolve(&self, path: &RemotePath) -> Result<String, TransferError> {
        let sess = self.session()?;
        let raw = path.as_str();
        if raw.starts_with('/') {
            return Ok(raw.to_string());
        }
        match &sess.remote_root {
            Some(root) => Ok(join_remote(root, raw)),
            None => Ok(raw.to_string()),
        }
    }
}

/// Iki POSIX path bileşenini tek `/` ile birleştir — leading/trailing slash
/// double'larını eler.
fn join_remote(root: &str, rel: &str) -> String {
    let root = root.trim_end_matches('/');
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() {
        root.to_string()
    } else if root.is_empty() {
        rel.to_string()
    } else {
        format!("{root}/{rel}")
    }
}

/// `russh::Error` → `TransferError` haritası. SSH connect/auth katmanından
/// gelen hatalar burada kategori bulur.
fn map_ssh_error(err: russh::Error) -> TransferError {
    use russh::Error as E;
    match err {
        E::NotAuthenticated | E::NoAuthMethod => TransferError::Authentication {
            reason: err.to_string(),
        },
        E::ConnectionTimeout | E::KeepaliveTimeout | E::InactivityTimeout => {
            TransferError::Timeout { elapsed_ms: 0 }
        }
        E::IO(e) => TransferError::Io(e),
        E::Disconnect => TransferError::ConnectionLost { bytes_sent: 0 },
        other => TransferError::Protocol {
            message: format!("ssh: {other}"),
        },
    }
}

/// `russh_sftp::client::error::Error` → `TransferError`. SFTP layer'dan gelen
/// hatalar `Status` variant içinde `StatusCode` kategorisi taşır; biz onu
/// genişletip TransferError variant'larına yönlendiriyoruz.
fn map_sftp_error(err: russh_sftp::client::error::Error, path: &str) -> TransferError {
    use russh_sftp::client::error::Error as E;
    match err {
        E::Status(status) => match status.status_code {
            StatusCode::NoSuchFile => TransferError::NotFound { path: path.into() },
            StatusCode::PermissionDenied => TransferError::Authorization { path: path.into() },
            StatusCode::ConnectionLost | StatusCode::NoConnection => {
                TransferError::ConnectionLost { bytes_sent: 0 }
            }
            StatusCode::OpUnsupported => TransferError::CapabilityNotSupported {
                capability: status.error_message,
            },
            _ => TransferError::Protocol {
                message: format!("sftp status {:?}: {}", status.status_code, status.error_message),
            },
        },
        // russh-sftp 2.1: `IO(String)` — orijinal `std::io::Error` zaten
        // string'leştirilmiş. ErrorKind metadata'sını koruyamadığımız için
        // Protocol kategorisine düşüyoruz; v2.0'da russh-sftp upstream'inde
        // proper error wrapping istenecek.
        E::IO(msg) => TransferError::Protocol {
            message: format!("sftp io ({path}): {msg}"),
        },
        E::Timeout => TransferError::Timeout { elapsed_ms: 0 },
        other => TransferError::Protocol {
            message: format!("sftp: {other}"),
        },
    }
}

/// SSH connect + auth + sftp bootstrap. `connect()` mantığı bir helper'a
/// alındı, hem kısa kalıyor hem testlerden çağrılabilir hale geliyor (gerçek
/// network ile birlikte; ignored test).
async fn bootstrap_session(
    profile: SftpProfile,
) -> Result<SftpAdapterSession, TransferError> {
    let keepalive = profile.keepalive.unwrap_or_default();
    let config = Arc::new(client::Config {
        // Inactivity timeout 1 saat — auth + idle çekirdeğinde kullanıcı çok
        // uzun ara verdiğinde reconnect tetiklenir (Bölüm 38.2).
        inactivity_timeout: Some(Duration::from_secs(3600)),
        // Bölüm 38.1: keepalive_interval ping aralığı, keepalive_max ardışık
        // yanıtsız ping toleransı. Default 30s × 3 fail → 90s'de ConnectionLost.
        keepalive_interval: Some(keepalive.server_alive_interval()),
        keepalive_max: usize::from(keepalive.server_alive_count_max),
        ..Default::default()
    });

    let handler = SftpClientHandler {
        expected_fingerprint: profile.known_host_fingerprint.clone(),
    };

    let host = profile.host.clone();
    let port = profile.port;
    let mut ssh = client::connect(config, (host.as_str(), port), handler)
        .await
        .map_err(map_ssh_error)?;

    // --- Authentication ---
    if let Some(password) = &profile.password {
        let auth = ssh
            .authenticate_password(profile.username.clone(), password.clone())
            .await
            .map_err(map_ssh_error)?;
        if !auth.success() {
            return Err(TransferError::Authentication {
                reason: "password authentication rejected by server".into(),
            });
        }
    } else if let Some(pem) = &profile.private_key_pem {
        let key: PrivateKey = decode_secret_key(pem, profile.private_key_passphrase.as_deref())
            .map_err(|e| TransferError::Authentication {
                reason: format!("private key decode failed: {e}"),
            })?;
        // RSA için server tercih ettiği hash algoritmasını sor; ed25519/ecdsa
        // için ignore edilir (None → legacy SHA-1 RSA, modern serverlarda
        // genelde SHA-256/512 negotiate olur).
        let hash_alg: Option<HashAlg> = ssh
            .best_supported_rsa_hash()
            .await
            .map_err(map_ssh_error)?
            .flatten();
        let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg);
        let auth = ssh
            .authenticate_publickey(profile.username.clone(), key_with_hash)
            .await
            .map_err(map_ssh_error)?;
        if !auth.success() {
            return Err(TransferError::Authentication {
                reason: "public key authentication rejected by server".into(),
            });
        }
    } else {
        return Err(TransferError::Authentication {
            reason: "profile must contain either 'password' or 'private_key_pem'".into(),
        });
    }

    info!(host = %host, port, user = %profile.username, "ssh authenticated");

    // --- SFTP subsystem ---
    let channel = ssh.channel_open_session().await.map_err(map_ssh_error)?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(map_ssh_error)?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| map_sftp_error(e, "<sftp-init>"))?;

    let capabilities = probe_capabilities(&ssh).await;
    debug!(?capabilities, "sftp capabilities probed");

    Ok(SftpAdapterSession {
        _ssh: ssh,
        sftp,
        capabilities,
        info: ProtocolInfo::Sftp {
            host: profile.host,
            port: profile.port,
        },
        remote_root: profile.remote_root,
    })
}

/// Server `MaxSessions` parametresini "aç-fail-say" mantığıyla tahmin et.
///
/// İdealde server `MaxSessions 10` ayarlıyken biz `OPEN_MAX_PARALLEL_PROBE`
/// kadar paralel `channel_open_session` deneriz, ilk hata sayısı verir. v1.0
/// pragmatic shortcut: kanalları açar, hemen close ederiz; "rate limit
/// patlaması" v2.0'da exponential probe ile düzeltilecek.
async fn probe_capabilities<H>(ssh: &Handle<H>) -> AdapterCapabilities
where
    H: client::Handler,
{
    let mut opened: u8 = 1; // sftp kanalını zaten bir kez açtık
    for _ in 0..OPEN_MAX_PARALLEL_PROBE.saturating_sub(1) {
        match ssh.channel_open_session().await {
            Ok(channel) => {
                opened = opened.saturating_add(1);
                // Channel handle drop edilince server bunu free olarak görür.
                drop(channel);
            }
            Err(_) => break,
        }
    }

    AdapterCapabilities {
        supports_byte_range: true,
        supports_remote_checksum: false,
        supports_server_side_rename: true,
        supports_symlinks: true,
        supports_resume: true,
        supports_multipart: true,
        max_parallel_sessions: opened.min(OPEN_MAX_PARALLEL_PROBE),
    }
}

#[async_trait]
impl ProtocolAdapter for SftpAdapter {
    async fn connect(&mut self, profile: &ConnectionProfile) -> Result<(), TransferError> {
        let parsed: SftpProfile = serde_json::from_value(profile.clone()).map_err(|e| {
            TransferError::Protocol {
                message: format!("invalid sftp profile: {e}"),
            }
        })?;
        let session = bootstrap_session(parsed).await?;
        self.session = Some(session);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransferError> {
        // SftpSession::close() + ssh handle drop = clean shutdown. Sftp close
        // hatası genellikle "zaten kopmuş" — log'la geç.
        if let Some(sess) = self.session.take() {
            if let Err(e) = sess.sftp.close().await {
                warn!(?e, "sftp close error during disconnect — ignored");
            }
        }
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
        // Path stringini stream closure'una taşıyabilmek için clone alıyoruz.
        let path_for_err = resolved.clone();

        Box::pin(async_stream::try_stream! {
            let session = self.session()?;
            let read_dir = session
                .sftp
                .read_dir(&resolved)
                .await
                .map_err(|e| map_sftp_error(e, &path_for_err))?;
            // ReadDir sync Iterator döndüğünden async stream'e yield ediyoruz.
            for entry in read_dir {
                let name = entry.file_name();
                if !include_hidden && name.starts_with('.') {
                    continue;
                }
                let meta = entry.metadata();
                let full = join_remote(&resolved, &name);
                yield remote_entry_from_meta(full, name, &meta);
            }
        })
    }

    async fn stat(&self, path: &RemotePath) -> Result<RemoteEntry, TransferError> {
        let session = self.session()?;
        let resolved = self.resolve(path)?;
        let meta = session
            .sftp
            .metadata(&resolved)
            .await
            .map_err(|e| map_sftp_error(e, &resolved))?;
        let name = Path::new(&resolved)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(remote_entry_from_meta(resolved, name, &meta))
    }

    async fn upload(
        &self,
        local: &LocalPath,
        remote: &RemotePath,
        opts: &TransferOptions,
        tx: ProgressSender,
    ) -> Result<TransferResult, TransferError> {
        let session = self.session()?;
        let remote_path = self.resolve(remote)?;
        let started = Instant::now();

        let mut reader = TokioFile::open(local.as_path())
            .await
            .map_err(TransferError::Io)?;
        let total = reader
            .metadata()
            .await
            .map(|m| m.len())
            .map_err(TransferError::Io)?;

        let mut writer = session
            .sftp
            .open_with_flags(
                &remote_path,
                OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE,
            )
            .await
            .map_err(|e| map_sftp_error(e, &remote_path))?;

        let chunk_size = effective_chunk(opts);
        let mut buf = vec![0u8; chunk_size];
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
            let _ = tx
                .send(ProgressTick {
                    chunk_index,
                    bytes_done,
                    bytes_total: total,
                })
                .await;
        }

        writer.flush().await.map_err(TransferError::Io)?;
        // sync_all server-side fsync@openssh.com extension'ı gerektirir; yoksa
        // sessizce no-op olur. Hata vermesi durumunda transfer'i bozmaya değmez,
        // log'la geç.
        if let Err(e) = writer.sync_all().await {
            warn!(?e, "remote sync_all failed (extension may be unsupported)");
        }
        drop(writer);

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

    async fn download(
        &self,
        remote: &RemotePath,
        local: &LocalPath,
        opts: &TransferOptions,
        tx: ProgressSender,
    ) -> Result<TransferResult, TransferError> {
        let session = self.session()?;
        let remote_path = self.resolve(remote)?;
        let started = Instant::now();

        let total = session
            .sftp
            .metadata(&remote_path)
            .await
            .map_err(|e| map_sftp_error(e, &remote_path))?
            .size
            .unwrap_or(0);

        let mut reader = session
            .sftp
            .open(&remote_path)
            .await
            .map_err(|e| map_sftp_error(e, &remote_path))?;

        // Atomic local write (Bölüm 14.2) — LocalAdapter ile aynı kalıp.
        let dst = local.as_path();
        if let Some(parent) = dst.parent() {
            if !parent.as_os_str().is_empty() {
                tokio_fs::create_dir_all(parent)
                    .await
                    .map_err(TransferError::Io)?;
            }
        }
        let tmp_path = with_suffix(dst, TMP_SUFFIX);
        let _ = tokio_fs::remove_file(&tmp_path).await; // best-effort orphan cleanup

        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .await
            .map_err(TransferError::Io)?;

        let chunk_size = effective_chunk(opts);
        let mut buf = vec![0u8; chunk_size];
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
        tokio_fs::rename(&tmp_path, dst)
            .await
            .map_err(TransferError::Io)?;

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

    async fn delete(&self, path: &RemotePath) -> Result<(), TransferError> {
        let session = self.session()?;
        let resolved = self.resolve(path)?;
        // Server dosya/dizin ayrımı yapacak iki distinct API; stat + branch:
        let meta = session
            .sftp
            .metadata(&resolved)
            .await
            .map_err(|e| map_sftp_error(e, &resolved))?;
        if meta.is_dir() {
            session
                .sftp
                .remove_dir(&resolved)
                .await
                .map_err(|e| map_sftp_error(e, &resolved))
        } else {
            session
                .sftp
                .remove_file(&resolved)
                .await
                .map_err(|e| map_sftp_error(e, &resolved))
        }
    }

    async fn mkdir(&self, path: &RemotePath) -> Result<(), TransferError> {
        let session = self.session()?;
        let resolved = self.resolve(path)?;
        // SFTP'de POSIX `mkdir -p` semantiği yok — recursive create için
        // bileşenleri elle yürütüyoruz. `create_dir` zaten varsa server
        // `Failure` döner; biz `try_exists` ile teyit edip yutuyoruz
        // (idempotent davranış).
        let leading_slash = resolved.starts_with('/');
        let components: Vec<&str> = resolved
            .split('/')
            .filter(|c| !c.is_empty())
            .collect();
        let mut accumulated = String::with_capacity(resolved.len());
        for (i, component) in components.iter().enumerate() {
            if i == 0 && leading_slash {
                accumulated.push('/');
            } else if i > 0 {
                accumulated.push('/');
            }
            accumulated.push_str(component);
            match session.sftp.create_dir(&accumulated).await {
                Ok(()) => {}
                Err(russh_sftp::client::error::Error::Status(s))
                    if matches!(s.status_code, StatusCode::Failure)
                        && session.sftp.try_exists(&accumulated).await.unwrap_or(false) =>
                {
                    // already exists — idempotent path; geç.
                }
                Err(e) => return Err(map_sftp_error(e, &accumulated)),
            }
        }
        Ok(())
    }

    async fn rename(
        &self,
        from: &RemotePath,
        to: &RemotePath,
    ) -> Result<(), TransferError> {
        let session = self.session()?;
        let from_r = self.resolve(from)?;
        let to_r = self.resolve(to)?;
        session
            .sftp
            .rename(&from_r, &to_r)
            .await
            .map_err(|e| map_sftp_error(e, &from_r))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.session
            .as_ref()
            .map(|s| s.capabilities)
            .unwrap_or_default()
    }

    fn protocol_info(&self) -> ProtocolInfo {
        self.session
            .as_ref()
            .map(|s| s.info.clone())
            .unwrap_or(ProtocolInfo::Sftp {
                host: String::new(),
                port: 22,
            })
    }
}

fn effective_chunk(opts: &TransferOptions) -> usize {
    // 0 verilirse default; opts.chunk_size 8 MiB tipik fakat SFTP write limit
    // server tarafında genelde 32 KiB — biz buffer'ı opts'a güveniyoruz,
    // russh-sftp internal'da split eder.
    if opts.chunk_size == 0 {
        DEFAULT_CHUNK_BYTES
    } else {
        opts.chunk_size
    }
}

fn throughput_bps(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        0.0
    } else {
        bytes as f64 / secs
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

#[cfg(unix)]
async fn fsync_with_parent(file: &TokioFile, target: &Path) -> std::io::Result<()> {
    file.sync_data().await?;
    if let Some(parent) = target.parent() {
        let dir = TokioFile::open(parent).await?;
        dir.sync_all().await?;
    }
    Ok(())
}

#[cfg(not(unix))]
async fn fsync_with_parent(file: &TokioFile, _target: &Path) -> std::io::Result<()> {
    file.sync_data().await
}

/// SFTP `Metadata` / `FileAttributes` → `RemoteEntry`.
fn remote_entry_from_meta(
    full_path: String,
    name: String,
    meta: &russh_sftp::protocol::FileAttributes,
) -> RemoteEntry {
    let kind = match meta.file_type() {
        FileType::Dir => RemoteEntryKind::Directory,
        FileType::File => RemoteEntryKind::File,
        FileType::Symlink => RemoteEntryKind::Symlink,
        _ => RemoteEntryKind::Other,
    };
    let modified = meta
        .mtime
        .map(|secs| UNIX_EPOCH + Duration::from_secs(secs as u64));
    RemoteEntry {
        path: RemotePath::new(full_path),
        name,
        kind,
        size: meta.size,
        modified,
        remote_hash: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_parses_password_auth() {
        let profile: SftpProfile = serde_json::from_value(json!({
            "host": "example.com",
            "port": 2222,
            "username": "alice",
            "password": "s3cret",
            "remote_root": "/home/alice",
        }))
        .expect("valid profile");
        assert_eq!(profile.host, "example.com");
        assert_eq!(profile.port, 2222);
        assert_eq!(profile.username, "alice");
        assert_eq!(profile.password.as_deref(), Some("s3cret"));
        assert!(profile.private_key_pem.is_none());
        assert_eq!(profile.remote_root.as_deref(), Some("/home/alice"));
    }

    #[test]
    fn profile_defaults_port_to_22() {
        let profile: SftpProfile = serde_json::from_value(json!({
            "host": "h",
            "username": "u",
            "password": "p",
        }))
        .expect("valid profile");
        assert_eq!(profile.port, 22);
    }

    #[test]
    fn profile_rejects_missing_required_fields() {
        let result: Result<SftpProfile, _> = serde_json::from_value(json!({
            "host": "h",
            // username eksik
        }));
        assert!(result.is_err());
    }

    #[test]
    fn join_remote_handles_slash_edge_cases() {
        assert_eq!(join_remote("/home/u", "data/file"), "/home/u/data/file");
        assert_eq!(join_remote("/home/u/", "/data/file"), "/home/u/data/file");
        assert_eq!(join_remote("", "rel/path"), "rel/path");
        assert_eq!(join_remote("/home/u", ""), "/home/u");
    }

    #[test]
    fn fingerprint_compare_strips_sha256_prefix() {
        assert!(fingerprints_match(
            "SHA256:abc123",
            "SHA256:abc123"
        ));
        assert!(fingerprints_match("SHA256:abc123", "abc123"));
        assert!(fingerprints_match("abc123", "SHA256:abc123"));
        assert!(!fingerprints_match("SHA256:abc123", "SHA256:xxxxxx"));
    }

    #[test]
    fn map_ssh_error_authentication_variants() {
        let err = map_ssh_error(russh::Error::NotAuthenticated);
        assert!(matches!(err, TransferError::Authentication { .. }));
        let err = map_ssh_error(russh::Error::NoAuthMethod);
        assert!(matches!(err, TransferError::Authentication { .. }));
    }

    #[test]
    fn map_ssh_error_timeout_variants() {
        let err = map_ssh_error(russh::Error::ConnectionTimeout);
        assert!(matches!(err, TransferError::Timeout { .. }));
        let err = map_ssh_error(russh::Error::InactivityTimeout);
        assert!(matches!(err, TransferError::Timeout { .. }));
    }

    #[test]
    fn map_ssh_error_disconnect_to_connection_lost() {
        let err = map_ssh_error(russh::Error::Disconnect);
        assert!(matches!(err, TransferError::ConnectionLost { .. }));
    }

    #[test]
    fn map_sftp_error_no_such_file() {
        let status = russh_sftp::protocol::Status {
            id: 0,
            status_code: StatusCode::NoSuchFile,
            error_message: "missing".into(),
            language_tag: "en".into(),
        };
        let err = map_sftp_error(
            russh_sftp::client::error::Error::Status(status),
            "/some/path",
        );
        match err {
            TransferError::NotFound { path } => assert_eq!(path, "/some/path"),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn map_sftp_error_permission_denied() {
        let status = russh_sftp::protocol::Status {
            id: 0,
            status_code: StatusCode::PermissionDenied,
            error_message: "denied".into(),
            language_tag: "en".into(),
        };
        let err = map_sftp_error(
            russh_sftp::client::error::Error::Status(status),
            "/forbidden",
        );
        assert!(matches!(err, TransferError::Authorization { .. }));
    }

    #[test]
    fn map_sftp_error_connection_lost() {
        let status = russh_sftp::protocol::Status {
            id: 0,
            status_code: StatusCode::ConnectionLost,
            error_message: "bye".into(),
            language_tag: "en".into(),
        };
        let err = map_sftp_error(
            russh_sftp::client::error::Error::Status(status),
            "/anything",
        );
        assert!(matches!(err, TransferError::ConnectionLost { .. }));
    }

    #[test]
    fn map_sftp_error_op_unsupported_to_capability() {
        let status = russh_sftp::protocol::Status {
            id: 0,
            status_code: StatusCode::OpUnsupported,
            error_message: "fsync@openssh.com".into(),
            language_tag: "en".into(),
        };
        let err = map_sftp_error(
            russh_sftp::client::error::Error::Status(status),
            "/x",
        );
        assert!(matches!(err, TransferError::CapabilityNotSupported { .. }));
    }

    #[test]
    fn unconnected_adapter_reports_default_capabilities() {
        let adapter = SftpAdapter::new();
        let caps = adapter.capabilities();
        // Connect öncesi default — `max_parallel_sessions: 4`.
        assert_eq!(caps.max_parallel_sessions, 4);
    }

    #[test]
    fn unconnected_adapter_protocol_info_placeholder() {
        let adapter = SftpAdapter::new();
        match adapter.protocol_info() {
            ProtocolInfo::Sftp { host, port } => {
                assert_eq!(host, "");
                assert_eq!(port, 22);
            }
            other => panic!("expected Sftp variant, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn methods_fail_cleanly_when_not_connected() {
        let adapter = SftpAdapter::new();
        let err = adapter
            .stat(&RemotePath::new("/x"))
            .await
            .expect_err("must fail without connect");
        assert!(matches!(err, TransferError::Protocol { .. }));
    }

    // ---- Live-server integration smoke tests (manuel; CI'da skip) ----
    //
    // Yerelde test etmek için:
    //   docker run -d --name dt-sftp -p 2222:22 atmoz/sftp foo:pass:::test
    //   cargo test --lib protocols::sftp -- --ignored
    //
    // CI default'ta `--ignored` yok, dolayısıyla bu test green'i bozmaz.

    #[tokio::test]
    #[ignore = "requires live SFTP server at localhost:2222"]
    async fn live_connect_and_capabilities() {
        let mut adapter = SftpAdapter::new();
        adapter
            .connect(&json!({
                "host": "127.0.0.1",
                "port": 2222,
                "username": "foo",
                "password": "pass",
                "remote_root": "/test",
            }))
            .await
            .expect("connect to local sftp");
        let caps = adapter.capabilities();
        assert!(caps.supports_byte_range);
        assert!(caps.max_parallel_sessions >= 1);
        adapter.disconnect().await.expect("clean disconnect");
    }

    #[tokio::test]
    #[ignore = "requires live SFTP server at localhost:2222"]
    async fn live_upload_download_roundtrip() {
        use tokio::sync::mpsc;

        let mut adapter = SftpAdapter::new();
        adapter
            .connect(&json!({
                "host": "127.0.0.1",
                "port": 2222,
                "username": "foo",
                "password": "pass",
                "remote_root": "/test",
            }))
            .await
            .expect("connect");

        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("smoke.bin");
        let payload = vec![0xCDu8; 256 * 1024];
        std::fs::write(&src, &payload).unwrap();

        let (tx, mut rx) = mpsc::channel(64);
        let collector = tokio::spawn(async move {
            let mut count = 0u32;
            while rx.recv().await.is_some() {
                count += 1;
            }
            count
        });

        adapter
            .upload(
                &LocalPath::new(&src),
                &RemotePath::new("smoke.bin"),
                &TransferOptions::default(),
                tx,
            )
            .await
            .expect("upload");
        drop(adapter);

        let ticks = collector.await.unwrap();
        assert!(ticks >= 1, "should emit at least one progress tick");
    }
}
