<div align="center">

# D-Transfer

**Deterministic, crash-resilient transfer infrastructure client**

*SFTP · S3 · WebDAV · Local — tek motor, kuyruk dayanıklılığı, doğrulanabilir transfer*
*SFTP · S3 · WebDAV · Local — single engine, queue durability, verifiable transfers*

🌐 **TR · EN** — Bu README iki dillidir / This README is bilingual (English collapsibles below each section)

</div>

<div align="center">

[![License: GPL v3+](https://img.shields.io/badge/License-GPL_v3+-blue.svg?style=flat-square)](LICENSE)
[![Status](https://img.shields.io/badge/status-pre--alpha-orange?style=flat-square)](#-yol-haritası--roadmap)
![Platform](https://img.shields.io/badge/platform-Windows_10%2F11_%C2%B7_Linux-blue?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&style=flat-square)
![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vuedotjs&style=flat-square)
![Rust](https://img.shields.io/badge/Rust-stable-CE412B?logo=rust&style=flat-square)
[![D Brand](https://img.shields.io/badge/D_Brand-AmrasElessar-00FF66?style=flat-square)](https://github.com/AmrasElessar)
[![Spec](https://img.shields.io/badge/spec-v2.1_🔒_frozen-success?style=flat-square)](./dtransfer-teknik-dokuman-v2_1.md)

</div>

---

## 📌 Kısaca

D-Transfer, **deterministik ve crash-resilient** bir masaüstü dosya transfer istemcisidir. SFTP, S3 (R2/B2 dahil), WebDAV ve yerel dosya sistemi adaptörlerini **tek bir Rust motorunda** birleştirmeyi hedefler — adapter sadece protokole özgü kısmı (auth, listing, presigned URL) yapar; retry, multipart, checksum ve encryption motor seviyesinde paylaşılır.

**Tauri v2** üzerine yazıldı — Rust core + WebView2 (Windows) / WebKitGTK (Linux). Binary boyutu ve RAM ayak izi tasarım kısıtlarıdır; kesin sayılar v1.0 release'de ölçülüp paylaşılacaktır.

**GPL-3.0-or-later** lisanslı bir D Brand projesidir. Windows 10/11 + Linux (Ubuntu 22.04+, Fedora 38+, Debian 12+) birinci sınıf hedef; macOS portu topluluk katkısına açıktır.

> ⚠️ **Pre-alpha:** Mimari spec v2.1'de donmuş, Faz 1-4 + sistem katmanları kod tarafında **mevcut**. Atomic finalization, crash recovery, queue persistence ve OS-native credential vault **çalışıyor**; resume, per-chunk hash, multipart, S3/WebDAV adapter ve encryption **planlı** (roadmap aşağıda). **Henüz release yok.**

<details>
<summary>🇬🇧 At a glance (English)</summary>

D-Transfer is a **deterministic and crash-resilient** desktop file-transfer client. It aims to share SFTP, S3 (incl. R2/B2), WebDAV and a local-filesystem adapter under **one Rust engine** — adapters only handle the protocol-specific parts (auth, listing, presigned URL), while retry, multipart, checksum, and encryption live in the engine.

Built on **Tauri v2** — Rust core + WebView2 (Windows) / WebKitGTK (Linux). Binary size and RAM footprint are design constraints; concrete numbers will be measured and published at the v1.0 release.

It is a **GPL-3.0-or-later** licensed D Brand project. Windows 10/11 + Linux (Ubuntu 22.04+, Fedora 38+, Debian 12+) are first-class targets; a macOS port is open to community contributions.

> ⚠️ **Pre-alpha:** Spec v2.1 is frozen; Phases 1-4 + system layers exist in code. Atomic finalization, crash recovery, queue persistence, and the OS-native credential vault **work today**; resume, per-chunk hash, multipart, S3/WebDAV adapters, and encryption are **planned** (roadmap below). **No releases yet.**

</details>

---

## 🆕 Şu ana kadar yapılanlar / What's done so far

> Mimari spec v2.1'de **donduruldu** (Bölüm 1-40, major bump ile değişir). Implementation Discovery Log (Bölüm 42) canlı. Aşağıdaki maddeler kod tarafında **mevcut**; release henüz çıkmadı.

- 🦀 **Rust core engine** — `TransferEngine` + `ProgressAggregator` (250 ms batched), `CancellationToken` hiyerarşisi (App / Profile / Transfer / Chunk), structured `EngineEvent` bus (broadcast UI + unbounded diagnostics)
- 📦 **Queue persistence** — SQLite WAL + DbActor pattern (tek-yazıcı serialization), state machine validatörü, crash recovery (`Active → Queued`, `Verifying/Finalizing → Failed`), `.dtransfer_tmp` orphan cleanup (24 h+)
- 🌐 **Protokol adapter trait + iki implementasyon** — `LocalAdapter` (atomic write, fsync policy, parent dir sync POSIX) + `SftpAdapter` (russh 0.54 + russh-sftp 2.1, host key fingerprint pin opsiyonel, capability probe)
- 🔐 **Credential vault** — Windows Credential Manager / macOS Keychain / Linux Secret Service üzerinden parola depolama; sırlar UI'a düşmez
- 🗄 **Bağlantı profilleri** — DB-destekli `ConnectionProfile` CRUD + adapter cache (`ConnectionManager`), UI'da TR/EN profil dialog'u (test connection + kapasiteleri göster)
- 📁 **Filesystem edge-case modülü** — `NormalizedPath` (NFC, Windows reserved name'leri), symlink policy, case-conflict detect, path length sınır kontrolü, sanitize_for_target
- ⚙️ **Settings** — `settings.json` atomic write + schema migration (`Option<Option<T>>` projeksiyonu ile semantik-doğru patch)
- 🔭 **Diagnostics & limits (Bölüm 30)** — `LimitProfile::detect()` (Linux `/proc/meminfo`), `RuntimeLimits` 5 preset (LowMemory/Desktop/Workstation/Server/Custom), `RuntimeMetrics` snapshot, log rotation policy, `CrashLoopDedup` (FNV-1a stack hash, count + zaman aralığı özet)
- 🚦 **Rate limit (Bölüm 16)** — `profile_id` keyed limiter (host değil — aynı servisin farklı hesapları birbirini etkilemez), Retry-After + X-RateLimit-* header parse, adaptive backoff + jitter
- 📑 **Audit trail (Bölüm 17)** — opt-in `audit.db` (queue.db'den ayrı), 500 ms tick + 64-batch flush, granüler `MaskingEngine` (IP / path / filename / username / presigned URL — sonuncu her zaman redact), `Redacted<T>` Debug wrapper PII koruması
- 🛰 **Network (Bölüm 37, 38.1)** — `SshKeepalive` (30 s × 3 → 90 s drop), `ConnectStrategy` staggered connect (250–500 ms jitter, Fail2Ban koruması), `ProxyConfig` + glob bypass matcher (`*.local`, `192.168.*`)
- 🎨 **Vue UI iskeleti** — TitleBar / DualPane / LocalPane / RemotePane / ProfileSidebar / QueuePanel / SettingsPanel, Tailwind v4, JetBrains Mono, light/dark/system tema, TR + EN i18n
- 🧩 **Widget kütüphanesi** — `FieldNumber` / `FieldToggle` / `FieldSegmented` (generic `<T extends string>`) — settings + profile dialog'larında tekrar eden form pattern'lerini tek bir kalıba indirgedi
- ✅ **188 unit test geçiyor** — engine, queue, settings, rate_limit, audit, network, fs_edge, diagnostics

<details>
<summary>🇬🇧 What's done so far (English)</summary>

> The architecture spec is **frozen** at v2.1 (Sections 1-40 — change only via major bump). The Implementation Discovery Log (Section 42) is live. The items below exist in code; no release yet.

- 🦀 **Rust core engine** — `TransferEngine` + `ProgressAggregator` (250 ms batched), `CancellationToken` hierarchy (App / Profile / Transfer / Chunk), structured `EngineEvent` bus (broadcast UI + unbounded diagnostics)
- 📦 **Queue persistence** — SQLite WAL + DbActor pattern (single-writer serialization), state-machine validator, crash recovery (`Active → Queued`, `Verifying/Finalizing → Failed`), `.dtransfer_tmp` orphan cleanup (24 h+)
- 🌐 **Protocol adapter trait + two implementations** — `LocalAdapter` (atomic write, fsync policy, POSIX parent-dir sync) + `SftpAdapter` (russh 0.54 + russh-sftp 2.1, optional host-key fingerprint pin, capability probe)
- 🔐 **Credential vault** — Windows Credential Manager / macOS Keychain / Linux Secret Service; secrets never reach the UI
- 🗄 **Connection profiles** — DB-backed `ConnectionProfile` CRUD + adapter cache (`ConnectionManager`), TR/EN profile dialog with test connection + capability report
- 📁 **Filesystem edge-case module** — `NormalizedPath` (NFC, Windows reserved names), symlink policy, case-conflict detection, path-length thresholds, `sanitize_for_target`
- ⚙️ **Settings** — `settings.json` atomic write + schema migration (semantically-correct `Option<Option<T>>` patch projection)
- 🔭 **Diagnostics & limits (Section 30)** — `LimitProfile::detect()` (Linux `/proc/meminfo`), `RuntimeLimits` 5 presets (LowMemory/Desktop/Workstation/Server/Custom), `RuntimeMetrics` snapshot, log rotation policy, `CrashLoopDedup` (FNV-1a stack hash, count + time-range summary)
- 🚦 **Rate limit (Section 16)** — `profile_id` keyed limiter (not host — different accounts on the same service stay independent), Retry-After + X-RateLimit-* header parsing, adaptive backoff + jitter
- 📑 **Audit trail (Section 17)** — opt-in `audit.db` (separate from queue.db), 500 ms tick + 64-batch flush, granular `MaskingEngine` (IP / path / filename / username / presigned URL — last one always redacted), `Redacted<T>` Debug wrapper for PII protection
- 🛰 **Network (Sections 37, 38.1)** — `SshKeepalive` (30 s × 3 → 90 s drop), `ConnectStrategy` staggered connect (250–500 ms jitter, Fail2Ban protection), `ProxyConfig` + glob bypass matcher (`*.local`, `192.168.*`)
- 🎨 **Vue UI shell** — TitleBar / DualPane / LocalPane / RemotePane / ProfileSidebar / QueuePanel / SettingsPanel, Tailwind v4, JetBrains Mono, light/dark/system theme, TR + EN i18n
- 🧩 **Widget library** — `FieldNumber` / `FieldToggle` / `FieldSegmented` (generic `<T extends string>`) collapses repeated form patterns in settings + profile dialogs into a single shape
- ✅ **188 unit tests passing** — engine, queue, settings, rate_limit, audit, network, fs_edge, diagnostics

</details>

---

## 🎯 Vizyon

D-Transfer, FileZilla'nın yerini almayı **hedeflemez**. Yer aldığı zemin daha dar: kullanıcının "bu dosya buraya gitsin ve tamamlandığında **kesin doğrulansın**" beklediği transfer iş yüklerinde tahmin edilebilir davranış. Bu, üç mühendislik kararına dayanır:

1. **Control Plane / Data Plane ayrımı** — Adapter (auth, listing, presigned URL) ve motor (retry, multipart, checksum, encryption) birbirinden bağımsızdır. Adapter sadece protokole özgü parçayı yapar; gerisi paylaşılır.
2. **Correctness before performance** — Optimizasyon doğruluğu bozarsa regression sayılır. Hız atomic finalization, fsync ve checksum'dan **sonra** gelir.
3. **Explicit non-goals** — FTP/FTPS bilinçli olarak yok (engineering maturity); plugin sistemi v2+; "geçici uzak dizin senkronu" (rsync-vari sürekli sync) yok. Bilinçli daraltma, garanti edilenleri net tutar.

Detaylı garantiler ve **garantilemediklerimiz** için: [Teknik döküman Bölüm 5](./dtransfer-teknik-dokuman-v2_1.md).

<details>
<summary>🇬🇧 Vision (English)</summary>

D-Transfer is **not** trying to replace FileZilla. Its lane is narrower: predictable behavior for transfer workloads where the user expects "this file goes there and is **conclusively verified** when finished". That rests on three engineering decisions:

1. **Control Plane / Data Plane separation** — adapters (auth, listing, presigned URL) and the engine (retry, multipart, checksum, encryption) are decoupled. Adapters only handle the protocol-specific part; the rest is shared.
2. **Correctness before performance** — if an optimization breaks correctness, it is a regression. Speed comes **after** atomic finalization, fsync, and checksum.
3. **Explicit non-goals** — FTP/FTPS is deliberately out (engineering maturity); the plugin system is v2+; continuous remote-directory sync (rsync-like) is out. Conscious narrowing keeps guarantees crisp.

For detailed guarantees and **non-guarantees**, see [Technical doc Section 5](./dtransfer-teknik-dokuman-v2_1.md).

</details>

---

## ✨ Öne Çıkan Özellikler / Key Features

### 🛡 Crash-Resilience

- **Queue persistence** — SQLite WAL + DbActor; restart sonrası `Active` task'lar deterministik `Queued`'a döner
- **Atomic finalization** — `{target}.dtransfer_tmp` → fsync → rename; power-cut sırasında yarım dosya kalmaz
- **Orphan cleanup** — startup'ta 24 h+ eski `.dtransfer_tmp` dosyaları taranıp silinir
- **Schema-versioned recovery** — DB ve `.dtresume` (planned) schema bump'ları idempotent migration
- **State machine validatörü** — geçersiz state geçişleri DB seviyesinde reddedilir, "Verifying'ten Queued'a manuel atlama" gibi yan etkiler imkânsız

<details>
<summary>🇬🇧 Crash-resilience (English)</summary>

- **Queue persistence** — SQLite WAL + DbActor; on restart, `Active` tasks deterministically revert to `Queued`
- **Atomic finalization** — `{target}.dtransfer_tmp` → fsync → rename; no half-written file survives a power cut
- **Orphan cleanup** — at startup, `.dtransfer_tmp` files older than 24 h are scanned and removed
- **Schema-versioned recovery** — idempotent migration for DB and `.dtresume` (planned) schema bumps
- **State-machine validator** — invalid state transitions are rejected at the DB layer; manually jumping from `Verifying` to `Queued` is impossible

</details>

### 🔌 Protokol Desteği / Protocol Support

- **SFTP** ✅ — russh 0.54 + russh-sftp 2.1, password + private-key auth, keepalive 30 s × 3, host-key fingerprint pin opsiyonel
- **Local FS** ✅ — yerel-yerele transfer, fsync policy (None / DataOnly / Full), POSIX parent-dir sync
- **S3 / R2 / B2** 🚧 — aws-sdk-s3 hedefli, multipart + presigned URL refresh planlandı
- **WebDAV** 🚧 — RFC 4918 PROPFIND/PUT/MKCOL + Basic/Digest auth planlandı
- **FTP / FTPS** ❌ — bilinçli olarak **dışarıda** (Bölüm 11): engineering maturity, modern cloud workflow'larda gerek yok

<details>
<summary>🇬🇧 Protocol support (English)</summary>

- **SFTP** ✅ — russh 0.54 + russh-sftp 2.1, password + private-key auth, keepalive 30 s × 3, optional host-key fingerprint pin
- **Local FS** ✅ — local-to-local transfer, fsync policy (None / DataOnly / Full), POSIX parent-dir sync
- **S3 / R2 / B2** 🚧 — aws-sdk-s3 targeted, multipart + presigned URL refresh planned
- **WebDAV** 🚧 — RFC 4918 PROPFIND/PUT/MKCOL + Basic/Digest auth planned
- **FTP / FTPS** ❌ — deliberately **out of scope** (Section 11): engineering maturity, no need in modern cloud workflows

</details>

### 🔐 Güvenlik / Security

- **OS-native credential vault** — Windows Credential Manager / macOS Keychain / Linux Secret Service; UI plaintext sır görmez
- **Host-key pinning** — SHA-256 fingerprint match (opsiyonel — TOFU varsayılan, strict pin profile başına override)
- **Client-side encryption** 🚧 — XChaCha20-Poly1305 + AES-256-GCM (FIPS-uyumlu ortamlar için), Argon2id KDF, key cache + zeroize-on-drop (Bölüm 13 spec, v1.0 hedefli)
- **Audit trail** — opt-in `audit.db`, granüler masking (IP / path / filename / username), presigned URL **her zaman** redact
- **PII koruması** — `Redacted<T>` wrapper + `tracing` filtresi (`credentials=off`); diagnostics bundle export edilirken regex post-pass ile defense-in-depth
- **TLS sertifika yönetimi** 🚧 — System / OS / Custom / Pin-Only trust modları (Bölüm 36 spec)

<details>
<summary>🇬🇧 Security (English)</summary>

- **OS-native credential vault** — Windows Credential Manager / macOS Keychain / Linux Secret Service; the UI never sees plaintext secrets
- **Host-key pinning** — SHA-256 fingerprint match (optional — TOFU default, strict pin overridable per profile)
- **Client-side encryption** 🚧 — XChaCha20-Poly1305 + AES-256-GCM (for FIPS environments), Argon2id KDF, key cache + zeroize-on-drop (Section 13 spec, v1.0 target)
- **Audit trail** — opt-in `audit.db`, granular masking (IP / path / filename / username), presigned URLs are **always** redacted
- **PII protection** — `Redacted<T>` wrapper + `tracing` filter (`credentials=off`); regex post-pass on diagnostics export for defense-in-depth
- **TLS certificate management** 🚧 — System / OS / Custom / Pin-Only trust modes (Section 36 spec)

</details>

### 📊 Kuyruk ve Scheduler / Queue & Scheduler

- **DbActor pattern** — DB yazımı tek thread'de serileştirilir; SQLite `database is locked` riski yapısal olarak kaldırılır
- **Schema migration v1 → vN** — idempotent ALTER, eski sürüm geri-okuma için kolon drop edilmez
- **WAL checkpoint policy** 🚧 — PASSIVE / TRUNCATE fallback (Bölüm 15.7)
- **Priority + FIFO** — DB index'inde `priority DESC, created_at ASC`; eşitlikte ekleniş sırası garanti
- **Per-profile concurrency** 🚧 — semaphore + `max_connections_per_host` (Bölüm 9, 30.2)

<details>
<summary>🇬🇧 Queue & scheduler (English)</summary>

- **DbActor pattern** — DB writes are serialized in a single thread; `database is locked` is structurally eliminated
- **Schema migration v1 → vN** — idempotent ALTER, old columns are not dropped so older binaries can still read
- **WAL checkpoint policy** 🚧 — PASSIVE / TRUNCATE fallback (Section 15.7)
- **Priority + FIFO** — DB index on `(priority DESC, created_at ASC)`; ties broken by insert order
- **Per-profile concurrency** 🚧 — semaphore + `max_connections_per_host` (Sections 9, 30.2)

</details>

### 🛰 Network Resilience

- **SSH keepalive** — 30 s ping × 3 fail → 90 s'de `ConnectionLost`; russh `keepalive_interval` + `keepalive_max` ile entegre
- **Staggered connect** — `ConnectStrategy` 250–500 ms jitter; aynı host'a 8 paralel handshake yerine doğal client pattern'i (Fail2Ban / corporate firewall koruması)
- **Rate limit awareness** — provider Retry-After + X-RateLimit-* header'ları parse edilir; key = `profile_id` (host değil), aynı servisin farklı token'ları birbirini etkilemez
- **Proxy** — HTTP / HTTPS / SOCKS5 type-level tanımlı; glob bypass matcher (`*.local`, `192.168.*`); apply-to-transport entegrasyonu 🚧
- **Network change monitor** 🚧 — sleep/wake + WiFi roaming + IP change agresif reconnect (Bölüm 38.5)
- **Application-level write timeout** 🚧 — Liar NAT koruması (Bölüm 38.6)

<details>
<summary>🇬🇧 Network resilience (English)</summary>

- **SSH keepalive** — 30 s ping × 3 fail → `ConnectionLost` at 90 s; wired into russh `keepalive_interval` + `keepalive_max`
- **Staggered connect** — `ConnectStrategy` 250–500 ms jitter; instead of 8 parallel handshakes against the same host, a natural client pattern (protection against Fail2Ban / corporate firewalls)
- **Rate limit awareness** — provider Retry-After + X-RateLimit-* headers are parsed; key = `profile_id` (not host), so different tokens on the same service stay independent
- **Proxy** — HTTP / HTTPS / SOCKS5 defined at the type level; glob bypass matcher (`*.local`, `192.168.*`); transport apply integration 🚧
- **Network change monitor** 🚧 — aggressive reconnect on sleep/wake + WiFi roaming + IP change (Section 38.5)
- **Application-level write timeout** 🚧 — Liar NAT protection (Section 38.6)

</details>

### 🎨 UI / UX

- **Çift pane** — yerel ↔ uzak browse, virtual scroller hazırlığı (büyük dizinler için)
- **Pinia stores** — settings / theme / locale / queue / liveProgress / profiles / connection / debug; engine event'leri tek noktadan akar
- **Tema sistemi** — light / dark / system, token tabanlı CSS disiplini (Bölüm 19); ANSI escape veya inline renk yok
- **i18n** — TR + EN (vue-i18n), native-quality lokalizasyon hedefi; "sadece Türk uygulaması" izlenimi vermez
- **Klavye-first** 🚧 — kapsayıcı kısayol editörü + komut paleti (Bölüm 21)
- **Drag & Drop** 🚧 — yerel pane'den uzak pane'e (v1.0)
- **Conflict resolution UX** 🚧 — `ConflictPolicy` enum mevcut (Bölüm 23), modal henüz yok

<details>
<summary>🇬🇧 UI/UX (English)</summary>

- **Dual pane** — local ↔ remote browse, virtual scroller scaffolding (for huge directories)
- **Pinia stores** — settings / theme / locale / queue / liveProgress / profiles / connection / debug; engine events flow through a single point
- **Theme system** — light / dark / system, token-based CSS discipline (Section 19); no ANSI escape or inline colors
- **i18n** — TR + EN (vue-i18n), targeting native-quality localization without giving "Turkish-only" impression
- **Keyboard-first** 🚧 — comprehensive shortcut editor + command palette (Section 21)
- **Drag & Drop** 🚧 — from local pane to remote pane (v1.0)
- **Conflict resolution UX** 🚧 — `ConflictPolicy` enum present (Section 23), modal not yet implemented

</details>

---

## 🛠️ Teknoloji / Tech Stack

| | |
|---|---|
| **Tauri v2** | Rust core + WebView2 / WebKitGTK |
| **Vue 3** | TypeScript + Vite + Pinia |
| **Tailwind v4** | Token-based styling, JetBrains Mono |
| **rusqlite** | SQLite WAL mode (bundled, sistem libsqlite bağımlılığı yok) |
| **russh** | 0.54 — SSH transport (ring backend, NASM gerekmez) |
| **russh-sftp** | 2.1 — SFTP subsystem |
| **keyring** | Cross-platform OS credential storage |
| **tokio** | Async runtime + spawn_blocking pool (profile-aware sizing) |
| **tracing** | Structured logs + diagnostics buffer, PII filter direktifleri |

### 📐 Mimari Belge / Architecture Document

Tüm mimari kararlar, garanti edilenler, garantilemediklerimiz ve Implementation Discovery Log için **tek dosya referans**: [`dtransfer-teknik-dokuman-v2_1.md`](./dtransfer-teknik-dokuman-v2_1.md) (40 bölüm spec + Bölüm 42 Discovery Log).

---

## 🗺️ Yol Haritası / Roadmap

| Faz / Phase | Durum / Status | İçerik / Content |
|---|---|---|
| **Faz 1 — İskelet** | ✅ done | ProtocolAdapter trait, error taxonomy, EngineEvent bus, CancellationToken, fs_edge, tracing |
| **Faz 2 — Queue & Engine** | ✅ done | TransferEngine, DbActor, scheduler (FIFO, max_concurrent=1), Local + SFTP adapter |
| **Faz 3 — Profiles** | ✅ done | ConnectionProfile DB, CredentialVault, ConnectionManager (adapter cache) |
| **Faz 4 — UI Shell** | ✅ done | Vue + Pinia + Tailwind iskeleti, i18n, tema, profile + settings dialog'ları |
| **Sistem katmanları** | ✅ done | Diagnostics (LimitProfile + RuntimeLimits), Rate Limit, Audit Trail, Network (Keepalive + Proxy types), orphan tmp cleanup |
| **Faz 5 — Transfer feature complete** | 🚧 in progress | Multipart + resume, `.dtresume` per-chunk hash, retry policy, drag-drop UI, conflict modal, queue ↔ UI binding, settings'in motora bağlanması |
| **Faz 6 — Cloud protokoller** | 📅 next | S3 (aws-sdk-s3, multipart, presigned URL refresh), WebDAV (RFC 4918), TLS pin (Bölüm 36) |
| **Faz 7 — Encryption** | 📅 planned | XChaCha20-Poly1305 + AES-256-GCM (FIPS), Argon2id, master key UI, crypto agility contract (Bölüm 13) |
| **Faz 8 — Resilience++** | 📅 planned | Network change monitor (sleep/wake + WiFi roaming), application-level write timeout, view_cache.db paginated remote listing |
| **v1.0** | 📅 — | Auto-update transaction (Ed25519 + anti-rollback, Bölüm 27), code signing, MSI/AppImage/deb/rpm release |

> 🇹🇷 Spec v2.1 **donmuş** — Bölüm 1-40 değişiklikleri ancak major bump ile mümkün. Implementation Discovery Log (Bölüm 42) canlı; kod yazarken çıkan keşifler oraya işlenir.
> 🇬🇧 Spec v2.1 is **frozen** — Sections 1-40 change only on a major bump. The Implementation Discovery Log (Section 42) is live; discoveries made while writing code are recorded there.

---

## 📥 Kurulum / Installation

> ⚠️ Henüz binary release yok. Pre-alpha — kaynaktan derleme tek seçenek.
> ⚠️ No binary releases yet. Pre-alpha — building from source is the only option.

### Kaynaktan derleme / Build from source

**Gereksinimler / Requirements:**

- [Rust](https://rustup.rs) stable toolchain (≥ 1.78)
- [Node.js](https://nodejs.org) ≥ 20 + [pnpm](https://pnpm.io) ≥ 10
- Platform tooling:
  - **Windows:** MSVC build tools + WebView2 (Win11'de yerleşik / built-in on Win11)
  - **Linux:** `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`

```bash
git clone https://github.com/AmrasElessar/d-transfer.git
cd d-transfer
pnpm install
pnpm tauri:dev   # dev mode (HMR + Rust hot recompile)
# veya / or:
pnpm tauri:build # release MSI / AppImage / deb
```

Çıktı / Output: `src-tauri/target/release/bundle/` altında.

---

## 🤝 Katkı / Contributing

D-Transfer **kişisel bir D Brand projesidir**; çekirdek mimari ve özellik geliştirme tek elden ilerliyor. Spec v2.1 donduğu için "yeni feature PR"ı zemini kapalıdır — ama topluluğun değer katabileceği üç şerit açık.

| ✅ Kabul edilen / Accepted | ❌ Şu an kabul edilmeyen / Not currently accepted |
|---|---|
| 🐛 Bug raporu (issue) / Bug reports | 🏗️ Mimari refactor PR'ı / PRs |
| 💡 Discovery Log notu (gerçek kod yazarken karşılaşılan edge-case) / DL notes | ✨ Spec-dışı feature PR'ı / Off-spec feature PRs |
| 🌍 Dil paketi / Language packs (`src/i18n/locales/<kod \| code>.json`) | 🤖 Adapter eklemek için PR (FTP gibi spec'ten dışlanmışlar) / Adapter additions for spec-excluded protocols |
| 🎨 Tema token tanımı / Theme tokens (Bölüm 19 disipline tabi / per Section 19 discipline) | |

> Tüm mimari kararlar [`dtransfer-teknik-dokuman-v2_1.md`](./dtransfer-teknik-dokuman-v2_1.md) içindedir. Spec'e aykırı PR'lar kapatılır; spec'in netleştirilmesi gereken bir nokta varsa **issue açın** — Discovery Log'a girer veya minor bump tetikler.

<details>
<summary>🇬🇧 Contributing (English)</summary>

D-Transfer is a **personal D Brand project**; core architecture and feature work are owned by the maintainer. Because spec v2.1 is frozen, "new feature PR" surface is closed — but three lanes are open where the community can add real value: bug reports, Discovery Log notes from real coding sessions, and language packs / theme tokens. All architectural decisions live in [`dtransfer-teknik-dokuman-v2_1.md`](./dtransfer-teknik-dokuman-v2_1.md). Spec-divergent PRs will be closed; if the spec needs clarification on a point, **open an issue** — it lands in the Discovery Log or triggers a minor bump.

</details>

---

## 🎨 D Brand Ailesi / D Brand Family

D-Transfer, D Brand ailesinin masaüstü dosya transfer kanadıdır.

| Ürün / Product | Platform | Açıklama / Description |
|---|---|---|
| **[D-Terminal](https://github.com/AmrasElessar/d-terminal)** | Windows | Agent-aware terminal *(pre-alpha, aktif geliştirme / active dev)* |
| **D-Transfer** | Windows + Linux | Deterministic, crash-resilient transfer client *(this project, pre-alpha)* |
| **D-Player** | Android | Kişisel müzik çalar, DSP motoru / personal music player *(in development)* |
| **DCar Launcher** | Android (Auto) | Head Unit araç içi OS katmanı / Head Unit in-car OS layer *(in development)* |
| **D-Watchtower** | — | Gözetim ve izleme platformu / surveillance & monitoring platform *(in development)* |

---

## 💖 Sponsorlar / Sponsors

D-Transfer açık kaynak (GPL-3.0+) ve sürekli geliştiriliyor. Sponsorluk doğrudan **D Brand uygulama portföyüne** dönüşür — yapılacaklar listesinde başka fikirler de var.

[![Sponsor on GitHub](https://img.shields.io/badge/Sponsor-AmrasElessar-db61a2?logo=githubsponsors)](https://github.com/sponsors/AmrasElessar)

<details>
<summary>🇬🇧 Sponsors (English)</summary>

D-Transfer is open source (GPL-3.0+) and under active development. Sponsorships translate directly into **the broader D Brand app portfolio** — more ideas are in the queue.

</details>

<!-- SPONSORS:LIST -->
<sub>Henüz sponsor yok / No sponsors yet. **İlk sponsor sen ol / Be the first →** [github.com/sponsors/AmrasElessar](https://github.com/sponsors/AmrasElessar)</sub>
<!-- /SPONSORS:LIST -->

---

## ❤️ D-Transfer'i destekle / Support D-Transfer

<table>
<tr>
<td align="center" width="33%">

### ⭐ Star at / Star it

GitHub'da **Star** projeyi başkalarına da görünür kılar.
Make the project visible to others.

[⭐ github.com/AmrasElessar/d-transfer](https://github.com/AmrasElessar/d-transfer)

</td>
<td align="center" width="33%">

### 💖 Sponsor ol / Sponsor

Geliştirme aktif, D Brand portföyünde başka fikirler de var.
Active development, more D Brand ideas in the queue.

[💖 github.com/sponsors/AmrasElessar](https://github.com/sponsors/AmrasElessar)

</td>
<td align="center" width="33%">

### 👀 Watch & Follow

Pre-alpha — release çıktığında haberdar olmak için.
Pre-alpha — get notified when the first release ships.

[👀 Watch repo](https://github.com/AmrasElessar/d-transfer/subscription)

</td>
</tr>
</table>

---

## 📜 Lisans / License

**GPL-3.0-or-later** © Orhan Engin OKAY — bkz / see [LICENSE](./LICENSE)

> 🇹🇷 GPL-3.0-or-later seçimi (Bölüm 40): **copyleft koruma** — türev çalışmalar aynı şartlarda açık kalır; D Brand felsefesi (Privacy First, Local by Default, Open Source) ile uyumludur; FileZilla'nın GPLv2 mirasıyla aynı ailededir, GPLv3'ün **patent grant** ve **TiVo-clause** hükümleriyle modernleştirilmiştir.
> 🇬🇧 Why GPL-3.0-or-later (Section 40): **copyleft protection** — derivative works stay open under the same terms; aligned with D Brand philosophy (Privacy First, Local by Default, Open Source); same family as FileZilla's GPLv2 lineage, modernized with GPLv3's **patent grant** and **TiVo-clause**.

---

<div align="center">

**Part of [D Brand](https://github.com/AmrasElessar)** · Built by [Orhan Engin OKAY](https://github.com/AmrasElessar)

*Made with ❤️ in Türkiye 🇹🇷*

</div>
