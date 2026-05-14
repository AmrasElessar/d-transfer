# DTransfer — Teknik Mimari Dökümanı
**Sürüm:** v2.1 · **Tarih:** Mayıs 2026 · **Durum:** 🔒 Semantic Contract Frozen
**Hedef Platform:** Windows 10/11 + Linux (Ubuntu 22.04+ / Fedora 38+ / Debian 12+) · x86_64 · **Lisans:** GPL-3.0-or-later

> **v2.1 Semantic Contract Pass:** v2.0 pivot'un üstüne ChatGPT review'unun 15 önerisi — semantic guarantees, behavior contract, trust boundaries — açıkça yazılı. *Spec artık 'feature inventory' değil 'davranış sözleşmesi'.* v2.0'ın baseline'ı: FTP/FTPS desteği bilinçli olarak çıkarıldı (engineering maturity); subprocess adapter runtime + adapter extension (v2+) v2+ ürünlerine taşındı; protocol adapter'lar compile-time bağlı tek monolitik binary'de yaşar. Spec kimliği netleşti: *deterministic, crash-resilient transfer infrastructure client* — FileZilla rewrite değil.
>
> Spec'in iki bölümü vardır:
> 1. **Mimari spec (Bölüm 1-39)** — donmuş, major bump ile değişir
> 2. **Implementation Discovery Log (Bölüm 41)** — append-only, tarihli, versiyon bump yok

---

## İçindekiler

1. [Proje Vizyonu](#1-proje-vizyonu)
2. [Design Philosophy](#2-design-philosophy)
3. [Compatibility Philosophy](#3-compatibility-philosophy)
4. [Non-Goals (Bilinçli Kararlar)](#4-non-goals-bilinçli-kararlar)
5. [Garantiler ve Garantilemediklerimiz](#5-garantiler-ve-garantilemediklerimiz)
6. [Release Stratejisi](#6-release-stratejisi)
7. [Teknoloji Yığını](#7-teknoloji-yığını)
8. [Mimari Genel Bakış](#8-mimari-genel-bakış)
9. [Rust Core Engine](#9-rust-core-engine)
10. [Structured Error Taxonomy](#10-structured-error-taxonomy)
11. [Protokol Desteği](#11-protokol-desteği)
12. [Filesystem Edge-Case Matrisi](#12-filesystem-edge-case-matrisi)
13. [Şifreleme Katmanı](#13-şifreleme-katmanı)
14. [Çok Parçalı Transfer (Multipart)](#14-çok-parçalı-transfer-multipart)
15. [Transfer Queue ve Scheduler](#15-transfer-queue-ve-scheduler)
16. [Rate Limiting ve API Quota](#16-rate-limiting-ve-api-quota)
17. [Audit Trail ve KVKK/GDPR](#17-audit-trail-ve-kvkkgdpr)
18. [UI Mimarisi](#18-ui-mimarisi)
19. [Tema Sistemi ve CSS Disiplini](#19-tema-sistemi-ve-css-disiplini)
20. [i18n — Çift Anadil Sistemi](#20-i18n-çift-anadil-sistemi)
21. [Erişilebilirlik ve Klavye Kısayolları](#21-erişilebilirlik-ve-klavye-kısayolları)
22. [Veri Görüntüleme Katmanı](#22-veri-görüntüleme-katmanı)
23. [Conflict Resolution UX](#23-conflict-resolution-ux)
24. [API Entegrasyon Stratejisi](#24-api-entegrasyon-stratejisi)
25. [Bağlantı Profil Yönetimi](#25-bağlantı-profil-yönetimi)
26. [Network Optimization Wizard](#26-network-optimization-wizard)
27. [Modülerlik ve Test Mimarisi](#27-modülerlik-ve-test-mimarisi)
28. [Crash Recovery ve Fault Tolerance](#28-crash-recovery-ve-fault-tolerance)
29. [Failure Semantics](#29-failure-semantics)
30. [Diagnostics Bundle](#30-diagnostics-bundle)
31. [Telemetry Policy](#31-telemetry-policy)
32. [CancellationToken Standardizasyonu](#32-cancellationtoken-standardizasyonu)
33. [Unified EngineEvent Bus](#33-unified-engineevent-bus)
34. [Config Migration Sistemi](#34-config-migration-sistemi)
35. [Internal Tracing Spans](#35-internal-tracing-spans)
36. [TLS Sertifika Yönetimi](#36-tls-sertifika-yönetimi)
37. [Proxy Desteği (HTTP / SOCKS5)](#37-proxy-desteği-http-socks5)
38. [SSH / SFTP Keepalive ve Network Resilience](#38-ssh-sftp-keepalive-ve-network-resilience)
39. [Geliştirme Yol Haritası](#39-geliştirme-yol-haritası)
40. [Teknik Kararlar ve Gerekçeler](#40-teknik-kararlar-ve-gerekçeler)
41. [Threat Model & Trust Boundaries](#41-threat-model-trust-boundaries)
42. [Implementation Discovery Log](#42-implementation-discovery-log)

---

## Versiyon Geçmişi

| Sürüm | Değişiklik |
|---|---|
| v1.0-v1.6 | İlk taslak, mimari iskelet (arşiv) |
| v1.7-v1.12 | Multi-pass AI review iterations (Gemini ×3, ChatGPT ×2): protokol matrisi, error taxonomy, adapter sandbox, conflict UX, threat model temelleri, KVKK audit, recovery playbook, ImpersonateConnect, sync stub v2, paginated listing, AV lock retry, symlink sanitization (arşiv) |
| v1.13-v1.14 | Errata + monolitik integration pass: SFTP probe channel-level, EventBus diagnostics ayrımı, .dtresume chunk_size immutable, WAL checkpoint policy, Linux credential headless fallback, WebDAV auth schemes, AV lock UX sub-state, fuzz testing, Tracing PII redaction, HTTP/2 pool tuning + Discovery Log workflow (arşiv) |
| v1.15 | Platform/operational completeness pass: headless master key injection, single instance lock, Wayland D&D fallback, view_cache.db startup orphan cleanup, per-user install path, adapter stdout disiplini, AV retry file-size scaled, schema BLOB paths, updater clock drift, salt storage clarification, WAL TRUNCATE→PASSIVE fallback (arşiv) |
| v1.16 | Second-order completeness pass: resume schema versioning, unified backpressure model, cancellation guarantee table, S3 ETag policy, TLS trust lifecycle, log retention, crypto agility, update transaction, Threat Model bölümü formalize (arşiv) |
| **v2.0** | **Post-pivot redesign**: kimlik netleşmesi — "deterministic, crash-resilient transfer infrastructure client", FileZilla rewrite değil. FTP/FTPS explicit unsupported (engineering maturity). Subprocess adapter runtime + adapter extension (v2+) v2+ scope'a. ProtocolAdapter compile-time bağlı, tek binary. Yeni 3 bölüm: **Design Philosophy** (6 ilke), **Compatibility Philosophy** (quirk karar matrisi, vendor-specific tuzaklar koruması), **Non-Goals** (bilinçli kararlar formalize). KVKK wording "uyumlu" → "conscious tasarım" (defensible). 6 bölüm tamamen silindi (Plugin Mimarisi, Plugin Supply Chain, Plugin Permissions v2, TR-glossary, License-section, Dizin Yapısı). v1.16 → v2.0: 44 bölüm → 40 bölüm. Tüm önceki review pass'lerinin değerli mimari kararları korundu (Threat Model, Crypto Agility, Cancellation Semantics, Backpressure Model, Resume Schema, Update Transaction, Headless Mode, Single Instance, Diagnostics Retention, TLS Trust Lifecycle, KVKK Audit). |
| **v2.1** | **Semantic contract pass (ChatGPT review, 15 öneri)**: Spec'in karakteri değişti — feature inventory'den **davranış sözleşmesi**ne. **Yeni Bölüm 5 — Garantiler ve Garantilemediklerimiz**: explicit behavior guarantees (crash/recovery, network, security), explicit non-guarantees (remote atomicity yok, exactly-once yok, non-compliant server interop yok, sürekli sync yok, remote timestamp identity değil), backend capability matrix (SFTP/S3/WebDAV/Local FS feature-by-feature), best-effort disiplini, correctness-before-performance operasyonel sözleşme, test kapsam zorunluluğu. **Design Philosophy 7. ilke eklendi**: "Correctness before performance" — optimization correctness'i bozarsa regression. **Bölüm 41.6 yeni**: Per-Component Trust Levels (explicit tablo) — local memory trusted, remote metadata untrusted, S3 ETag NOT integrity proof, vb. Wording iyileştirmeleri: "modern" abuse azaldı → "deterministic/crash-resilient/protocol-aware"; KVKK identity'den supporting pillar'a düşürüldü; "delta transfer" → "akıllı kısmi transfer optimizasyonu" (rsync expectation trap'i önle); "encrypted at rest" → "optional encrypted local state" (scope honest); "10 milyon dosya" gibi büyük sayılar → "büyük ölçekli" (benchmark sorusu önle); "atomic" → "local atomic finalization" (remote scope ayrı); Local FS pozisyonu güçlendirildi (yerel↔uzak birleşik akış); Turkish-first → "native-quality localization, globally usable" (sadece TR algısı önle). "Best-effort" wording disiplini throughout. ChatGPT'nin tavsiyesi: *"feature reduction değil, architectural sharpening"* — bu pass tam olarak bu. |

---

## 1. Proje Vizyonu

DTransfer, FileZilla'nın yerini hedefleyen modern bir Windows ve Linux masaüstü dosya transfer istemcisidir.

**Temel felsefe:** Önce taş gibi çalışan bir transfer motoru, sonra katman katman büyüme.

**Mimari felsefe:** Control Plane / Data Plane ayrımı. Protocol adapter'ları (compile-time bağlı) auth, listing ve presigned URL sağlar. Asıl veri akışı, retry, multipart, checksum, encryption her zaman ana Rust motorunda — adapter sadece protokol-spesifik kısımları yapar.

**Platform felsefesi:** v1.0–v1.2 Windows + Linux birinci sınıf. macOS portu GPL-3.0-or-later lisansı altında topluluk katkısına açıktır. Windows + Linux ikilisi farklı WebView engine'ler (Blink + WebKit) üzerinde test edilir — bu çapraz uyumluluk macOS WKWebView portunu da otomatik kolaylaştırır.

### Temel Fark Noktaları

| Kriter | FileZilla | DTransfer |
|---|---|---|
| Modern cloud protokolleri (S3/WebDAV native) | ✗ | ✓ |
| Client-side şifreleme | ✗ | ✓ (XChaCha20 / AES-256-GCM) |
| Audit trail (KVKK-conscious) | ✗ | ✓ (opt-in) |
| Rust çekirdek motor | ✗ | ✓ (tokio async) |
| Çift anadil TR/EN | ✗ | ✓ |
| Multipart + resume | ✗ | ✓ (atomic write) |
| Queue persistence | ✗ | ✓ (restart/crash sonrası devam) |
| Structured error taxonomy | ✗ | ✓ (UI retry/refresh kararı) |
| Rate limiting / API quota | ✗ | ✓ (adaptive backoff) |
| Adapter ekosistemi (v2+) | ✗ | ⏳ planlı (sandbox + capability v2) |
| Network Wizard | ✗ | ✓ (opt-in) |
| Drag & Drop | ✗ | ✓ (v1.0) |
| Telemetry | Var | Sıfır |
| HTTP / SOCKS5 proxy | Kısmi | ✓ (auth dahil, profile bazlı) |
| TLS sertifika yönetimi | Temel | ✓ (granüler trust store, pin) |
| Lisans | GPLv2 | GPL-3.0-or-later |

---

## 2. Design Philosophy

DTransfer'ın tüm spec kararlarının arkasında 6 değer ilkesi vardır. Bu ilkeler issue tartışmalarında, PR review'larında, roadmap kararlarında ve contributor onboarding'inde referans noktasıdır.

### 2.1 Altı İlke

1. **Reliability over feature count.** Az ama güvenilir özellik; çok ama kırılgan özellik değil. Kullanıcı *"bu işe yaramayabilir"* hissi taşımamalı.

2. **Explicit semantics over heuristic magic.** "Akıllı tahmin" yerine "açık kontrat". Sistem ne yapacağını söylemeli, varsayım üzerinden çalışmamalı. *Magic-comportment kullanıcı güvenini öldürür.*

3. **Modern protocols over legacy interoperability.** Standart-uyumlu modern protokol implementasyonu hedef; her server'ın quirk'üne workaround yazmak değil. Legacy uyumluluk bataklığına çekilmemek bilinçli karar.

4. **Crash consistency over maximum throughput.** %5 hız kaybı pahasına garanti edilmiş durum (resume çalışır, queue kaybolmaz, .dtresume tutarlıdır) tercih edilir. Performans optimization sonradan; correctness ilk gün.

5. **User-visible correctness over hidden automation.** Kullanıcının göremediği "yardım" yerine kullanıcının görüp anlayabileceği davranış. Conflict resolution explicit prompt; "akıllı" silent merge değil.

6. **Deterministic behavior over implicit background sync.** Kullanıcı tetiklediği şey çalışır, ekstrası çalışmaz. Sürekli daemon, otomatik watcher, "ne olur ne olmaz" arka plan aktivitesi yok.

7. **Correctness before performance.** Optimization correctness'i bozarsa optimization değil regression'dır. %5 hız kaybı pahasına garanti edilmiş davranış her zaman tercih edilir. Performance optimizasyonları **correctness budget**'inden harcamayan miktarda yapılır; hızı bozan kestirme yol yok. Bu ilke ileride benchmark/profil baskısı geldiğinde dengeleyicidir.

### 2.2 Non-Goals Olarak Mimari Karar

Non-goal'lar **birinci sınıf mimari karar**dır (Bölüm 7). "Yapmıyoruz" satırı "henüz yapmadık" değil; "yapmamaya karar verdik" demek. Bu fark roadmap discipline'inin omurgasıdır — feature creep'e karşı net savunma.

### 2.3 Karar Felsefesi Pratikte

Yeni bir özellik veya değişiklik teklif edildiğinde sorulacak sorular:

- **Reliability'i artırır mı, azaltır mı?** (Artırır → değerlendir; azaltır → reddet)
- **Semantics'i daha explicit mi yapar, daha implicit mi?** (Explicit → değerlendir)
- **Bir legacy uyumluluğu için mi geliyor?** (Evet → çok güçlü gerekçe ister)
- **Crash sırasındaki davranış kötüleşir mi?** (Kötüleşir → reddet)
- **Kullanıcının görmediği bir background davranış mı?** (Evet → ya görünür yap ya reddet)
- **Daemon/watcher gerektirir mi?** (Evet → v2 scope)

---

## 3. Compatibility Philosophy

DTransfer **standards-compliant implementasyonları** birinci sınıf hedef alır. Bu satır kelime oyunu değil — günlük geliştirme kararlarını şekillendiren bir savunma hattıdır.

### 3.1 Temel Tavır

> *DTransfer targets standards-compliant implementations first. Server-specific compatibility quirks may be implemented selectively when they do not compromise reliability, security, or maintainability.*

Türkçesi: "Standart uyumlu uygulamaları öncelikli hedefleriz. Sunucu-spesifik uyumluluk tuhaflıkları, **güvenilirlik, güvenlik veya bakım yapılabilirliği** zayıflatmadığı sürece seçici olarak implement edilebilir."

### 3.2 Quirk Karar Matrisi

Bir server-specific davranış raporlandığında karar tablosu:

| Quirk Tipi | Davranış |
|---|---|
| Standart belirsizliği nedeniyle yaygın sapma (5+ farklı server'da gözlenen) | Adapter'a workaround eklenir |
| Tek bir vendor'a özgü davranış, standart ihlali, yaygın değil | **Eklenmez** — bug raporu vendor'a yönlendirilir |
| Workaround diğer server'larda yan etki yaratır mı? Hayır | Eklenebilir |
| Workaround diğer server'larda yan etki yaratır mı? Evet | **Eklenmez** — correctness > compatibility |
| Quirk security implication yaratır mı? | Otomatik **reddedilir** |
| Quirk reliability garantilerini kırar mı (resume, atomic ops)? | Otomatik **reddedilir** |

### 3.3 "Bilinçli Selectiveness" Disiplini

FileZilla'nın 25 yıllık quirk database'i bir uyarı hikayesidir: her quirk codebase'in başka bir yerine yan etki yapar, regression test surface'i exponential büyür, hiçbir refactor güvenle yapılamaz.

DTransfer adapter'ları her quirk'ü **belgelenmiş** ve **gerekçesi yazılı** olarak kabul eder. Discovery Log'a (Bölüm 42) eklenir; sebep, kapsam, alternatif olarak değerlendirilen yaklaşımlar belirtilir. Gizli "if server == 'X' then do Y" kodu yazılmaz.

### 3.4 Standart-Uyumlu Sunucu Spektrumu

v1.0 hedef sunucu spektrumu:

| Protokol | Birinci sınıf hedef | Selective workaround |
|---|---|---|
| SFTP | OpenSSH 7.0+, Dropbear modern | russh tarafından sağlanan compat layer |
| S3 | AWS S3, MinIO, Cloudflare R2 | aws-sdk-s3 endpoint feature flag'leri |
| WebDAV | RFC 4918 uyumlu (Nextcloud, ownCloud, IIS modern) | Microsoft Office WebDAV extensions seçici |
| Local FS | POSIX (Linux), NTFS (Windows) | OS API farkları için abstraction katmanı |

Bu spektrum dışındaki sistemler "best effort" — çalışabilir ama garanti edilmez, bug raporu kabul edilir ancak öncelik düşer.

### 3.5 Long-Term Maintenance Filosofisi

Her workaround **future maintenance cost**'tur. Eklerken kendine sor:

- 3 yıl sonra bu workaround hâlâ gerekli olacak mı?
- Vendor bug'ı fix edilirse workaround zarar verir mi?
- Yeni geliştirici bu kodu okuyunca "neden böyle?" diye anlayabilecek mi?

Cevaplar net değilse → eklenmez. Belirsiz kalan workaround'lar codebase'in en zehirli parçalarıdır.

---

## 4. Non-Goals (Bilinçli Kararlar)

DTransfer'ın **yapmadığı şeyler** — eksik özellik değil, bilinçli mimari karar. Her satırın gerekçesi var.

### 4.1 Açıkça Desteklenmeyen Protokoller

**FTP / FTPS** — DTransfer v1.0 intentionally excludes FTP/FTPS.

> *The FTP ecosystem contains decades of server-specific interoperability behavior and undefined edge cases accumulated across thousands of deployments. Rather than implementing a partial or unreliable FTP stack, DTransfer focuses on modern protocols with stronger semantics, security, and testability.*

Türkçesi: FTP ekosistemi binlerce deployment üzerinden yıllarca birikmiş server-specific uyumluluk davranışı ve tanımsız edge-case içerir. Kısmi veya güvenilmez bir FTP stack'i implement etmek yerine, DTransfer daha güçlü semantik, güvenlik ve test edilebilirliğe sahip modern protokollere odaklanır.

Bu **engineering maturity kararı**dır — korkaklık değil. FileZilla 25 yılda biriktirdiği FTP quirk database'ini sıfırdan üretmek tek geliştirici için imkansız iş; "yarım yapıyorum" alternatifi de kullanıcıyı bezdirir.

**SCP, Azure Blob, GCS, Dropbox, Google Drive, OneDrive** — v1.1+ değerlendirme. v1.0'da gereksiz yüzey alanı.

### 4.2 Yapısal Non-Goals

| Non-Goal | Sebep |
|---|---|
| **Bidirectional sync engine** | Sync engine = transfer'in 5 katı karmaşıklık; conflict graphs, tombstones, snapshot semantics, rename detection, causal ordering. v2+ ürünü; v1.0 mimarisini hostage etmez |
| **Third-party adapter extension (v2+)** | Untrusted adapter runtime + sandboxing + ABI stability + crash isolation = ayrı bir ürün scope; D Brand v1.0'da sadece kendi adapter'larını barındırır (compile-time bağlı) |
| **Subprocess adapter runtime** | Adapter marketplace altyapısı; D Brand closed-core modelde gereksiz, monolitik binary daha basit + güvenilir + hızlı |
| **Collaborative features** | Multi-user, paylaşım, yorum — Dropbox/Drive'ın domain'i; DTransfer single-user transfer client'ı |
| **Remote editing** | "Aç, edit et, kaydet, geri yükle" — kullanıcının editor'ünün (Vim/VSCode) işi; transfer client'ın değil |
| **Virtual filesystem mounting (FUSE / WinFsp)** | rclone'un domain'i; tamamen farklı problem space, OS-level integration karmaşıklığı |
| **Background daemon / continuous sync** | "Kullanıcı tetikler, sistem çalıştırır" felsefesi (Design Philosophy madde 6); arka plan watcher v2+ |
| **Mobile app (iOS / Android)** | Desktop-first ürün; mobile transfer farklı UX modeli, ayrı proje |
| **Web tarayıcı adapter'ı** | Native desktop deneyimi öncelik; tarayıcı extension'ı feature parity sağlayamaz |

### 4.3 Non-Goal'lar Neden Yazılı?

- **Scope discipline:** Yeni feature teklifinde ilk durak burası. Listede ise → reddedildi, gerekçesi yazılı.
- **Contributor onboarding:** "Bunu yapsak nasıl olur?" sorusuna hızlı cevap.
- **Roadmap protection:** v2 planlaması yapılırken hangi şeylerin ayrı ürün olduğu net.
- **Marketing honesty:** Kullanıcı baştan ne aldığını / almadığını bilir.

### 4.4 Non-Goal Yeniden Değerlendirme

Non-goal'lar **donmuş kararlar** değil, **mevcut kararlar**dır. v1.x süresince yeniden değerlendirilebilir, ama yeniden değerlendirme için:

1. Yeni gerekçe gerekli (eski sebep neden geçerli değil artık?)
2. Design Philosophy ile uyum kontrolü
3. Scope etkisi analizi (taşıdığı zincirleme karmaşıklık)
4. Decision log'a giriş (Bölüm 40)

Bu disiplin olmadan non-goal listesi anlamını kaybeder.

---

## 5. Garantiler ve Garantilemediklerimiz

DTransfer'ın **davranış sözleşmesi** (behavior contract). Spec'in geri kalanı "nasıl yaparız"; bu bölüm "ne garanti ederiz, ne etmeyiz" — daha sonra ortaya çıkacak her belirsizliğin referans noktası.

### 5.1 Ne Garanti Ederiz (Behavior Guarantees)

**Crash & Recovery:**

| Garanti | Kapsam |
|---|---|
| **Queue state survival** | Beklenmedik process termination, kernel panic, elektrik kesintisi sonrası — `queue.db` WAL fsync sınırı içinde durum korunur. Sınır: son `bytes_done` checkpoint'inden (5 saniye batch) sonraki tamamlanmış chunk'lar kaybedilebilir, ama task durumu ve resume bilgisi korunur |
| **Atomic local finalization** | İndirilen dosya başlangıçta `.dtransfer_tmp` olarak yazılır, fsync + rename ile final isme alınır. Yarım indirilmiş dosya **asla** final isimde görünmez |
| **Resume after restart** | Pause edilen veya crash sırasında yarım kalan transferler yeniden başlatıldığında kaldıkları yerden devam eder (backend chunk-resume desteği şartıyla — bkz. §5.2) |
| **`.dtresume` schema compatibility** | `ResumeHeader.schema_version` ile backward-compatible okuma, forward-incompatible reject (Bölüm 13) |
| **Crash-loop deduplication** | Aynı stack trace 100 kez tekrarlanırsa log'da tek özet satır olarak yazılır; disk şişmesi engellenir (Bölüm 30) |

**Network & Transfer:**

| Garanti | Kapsam |
|---|---|
| **TLS 1.2+ minimum** | Daha eski TLS sürümleriyle handshake **reddedilir** |
| **Cert pin enforcement** | Pin'lenmiş profile bağlanırken cert/SPKI değişimi → bağlantı reddi + kullanıcı modal (Bölüm 35) |
| **Bandwidth limit honor** | Settings'te tanımlı kural aktifse, transfer hızı o değeri **aşmaz** (ölçüm penceresi 1 saniye) |
| **Single instance** | İkinci DTransfer instance'ı açılmaya çalışılırsa mevcut pencere öne alınır, ikinci process başlatılmaz (Bölüm 6) |

**Security:**

| Garanti | Kapsam |
|---|---|
| **Credential encryption at rest** | OS keychain veya Argon2id-encrypted file ile şifrelenmiş saklama (Bölüm 24) |
| **Secret zeroization on drop** | Master key, OAuth token gibi secret'lar `Drop` impl ile heap'te sıfırlanır (Bölüm 12.6) |
| **Updater signed manifest** | Anti-rollback + Ed25519 signature + anchor pubkey + clock drift check (Bölüm 27) |
| **No telemetry** | Hiçbir kullanım verisi, crash raporu, IP, metric uzak sunucuya gönderilmez (Bölüm 30) |

### 5.2 Ne Garanti Etmeyiz (Explicit Non-Guarantees)

ChatGPT review'unun en kritik tavsiyesi: ileride bizi yanlış anlaşılmalardan koruyacak satırlar.

> **DTransfer does not guarantee...** (bu liste itiraflar değil, mühendislik dürüstlüğüdür)

| Garanti **YOK** | Sebep |
|---|---|
| **Remote atomicity across all protocols** | S3 multipart complete "atomic-ish", FTP server'a bağlı, WebDAV `MOVE` implementation-specific. Backend capability matrix'i (§5.3) durumu açık eder |
| **Exactly-once delivery semantics** | Network retry + timeout ambiguity + ACK loss durumlarında çoğu transfer sistemi **at-least-once** veya **maybe-once** semantics verir. DTransfer **best-effort exactly-once** hedefler ama formal garanti vermez |
| **Interoperability with non-compliant servers** | Vendor-specific davranışa workaround eklenmesi seçici (Bölüm 3 — Compatibility Philosophy). Sunucu RFC ihlali yapıyorsa, fail loud — silent compat hack yok |
| **Continuous synchronization daemon behavior** | DTransfer arka plan watcher değil; kullanıcı tetikler, sistem çalıştırır. "Yokken kendi başına sync" yapmaz (Design Philosophy madde 6) |
| **Remote timestamps as authoritative identity proofs** | `mtime` heuristic'tir, tek başına identity değildir. FAT timestamp 2sn granular, timezone drift, server clock skew, DST transition — hepsi false positive üretir. Identity için size+mtime+optional hash kombine kullanılır |
| **Remote object stability during transfer** | Yükleme sırasında remote'da paralel değişiklik (başka client overwrite eder vs.) → DTransfer detect edemez, son ACK alınan state final kabul edilir. Sync engine'in işi, transfer client'ın değil |
| **Cross-FS Unicode normalization invariance** | macOS NFD ↔ Linux raw bytes ↔ Windows NFC karşılaşmasında automatic merge yapılmaz — Conflict Resolution UX devreye girer (Bölüm 23) |
| **Bandwidth across all network conditions** | TCP throughput network'e bağlı; DTransfer üst sınır koyar (rate limit) ama minimum hız garantisi vermez |
| **Storage durability beyond OS fsync** | Disk firmware'inin fsync semantiğine güvenilir (POSIX). Yanlış implement eden disklerde durability OS sorumluluğu, DTransfer'ın değil |
| **Plugin/extension ecosystem** | v1.0'da üçüncü taraf adapter ekosistemi yok (Non-Goals, Bölüm 4). Bu özellikle **şu anki garanti yokluğu**, gelecek vaadi değil |

### 5.3 Backend Capability Matrix

Protokol özelinde **ne garanti edilir, ne capability-dependent'tir**:

| Capability | SFTP | S3 | WebDAV | Local FS |
|---|---|---|---|---|
| Atomic upload (single object) | ✓ (tmp + rename) | ✓ (multipart complete atomic) | ⚠️ server-bağımlı (`Overwrite: F`) | ✓ (tmp + rename) |
| Atomic rename | ✓ POSIX rename | ✗ (object store, copy+delete) | ⚠️ `MOVE` (implementation-specific) | ✓ |
| Durable commit ACK | ✓ fsync (sunucu config'ine bağlı) | ✓ HTTP 200/201 + multipart complete | ⚠️ HTTP 200/204 (server fsync garanti yok) | ✓ fsync |
| Partial chunk resume | ✓ (REGREQ offset write) | ✓ (multipart part PUT, ETag tracking) | ⚠️ `Range` write (server-bağımlı) | ✓ (seek + write) |
| Remote checksum (content hash) | ✗ (genellikle yok, hash hesaplamak için file read) | ✓ AWS Additional Checksums API | ✗ (`If-Match` ETag, content hash değil) | ✓ (local re-read) |
| Idempotent retry safe | ✓ (offset write) | ✓ (multipart part PUT idempotent) | ⚠️ HTTP method'a göre (PUT idempotent, POST değil) | ✓ |
| Server-side rename (move) | ✓ `rename` SFTP op | ✗ (copy + delete, not atomic) | ⚠️ `MOVE` | ✓ |
| Symlink preserve | ⚠️ server-bağımlı | ✗ (yok) | ✗ (yok) | ✓ (POSIX) / ⚠️ (Windows) |

**Capability negotiation runtime:** `ProtocolAdapter::capabilities()` startup'ta probe sonucu döner; UI bu capability'leri sorgular ve desteklenmeyen özellikleri (delta, server-side rename, vb.) **gizler** — kullanıcıya "bu protocol bu işlemi desteklemez" hatası göstermez, hiç sunmaz.

### 5.4 "Best Effort" Disiplini

Bazı davranışlar deterministic olamaz çünkü altyapı izin vermez. Bu durumda spec'te **"best effort"** dilini kullanırız:

- *Best-effort verification* — hash check yapılabilirse yapılır, yapılamıyorsa kullanıcı bilgilendirilir
- *Capability-dependent behavior* — backend protokole göre değişen davranış, runtime'da probe edilir
- *Backend-specific semantics* — sunucu davranışına bağlı sonuç, garanti edilmez

Bu wording mühendislik dürüstlüğüdür: gerçeklik altyapıdadır, biz onun üstüne yalan söylemiyoruz.

### 5.5 Correctness Before Performance — Operasyonel Sözleşme

Bölüm 2.1'deki 7. ilkenin pratik karşılığı: ileride performance regression incelendiğinde:

1. Optimization correctness'i bozarsa → reddedilir, alternatif aranır
2. Optimization correctness'i değiştirmeden hız kazandırırsa → benchmark + integrate
3. Optimization "biraz" correctness'i azaltırsa (örn. fsync skip) → opt-in flag arkasına, default kapalı

Bu disiplin "performance vs correctness" tartışmasını her seferinde yeniden açmaktan korur.

### 5.6 Garantiler İçin Test Kapsamı

Bölüm 5.1'deki her satır için **integration test** yazılır. Garanti edilen davranış test edilmemişse garanti değil, niyettir. Faz 1 implementation'ında bu test suite öncelikli teslim edilir.

---

## 6. Release Stratejisi

### v1.0 — Taş Motor

**Hedef:** Piyasanın en güvenilir modern SFTP istemcisi.

- S3 + MinIO + R2 + B2
- WebDAV
- Dual panel + Drag & Drop
- Multipart download/upload + Resume (atomic write)
- XChaCha20-Poly1305 client-side şifreleme
- Transfer Queue (kalıcı, `queue.db`)
- Structured error taxonomy
- Rate limiting / adaptive backoff
- Profil yöneticisi + Windows Credential Manager / Linux Secret Service (libsecret)
- TLS sertifika yönetimi (system roots / pin / insecure ack)
- HTTP / SOCKS5 proxy desteği (auth + bypass)
- SSH/SFTP keepalive (idle drop koruması)
- Crash recovery + Diagnostics Bundle
- TR/EN + Açık/Koyu tema + Tam klavye erişilebilirliği

Üçüncü taraf adapter yok · Wizard yok · Audit yok · SCP yok · Delta yok

**Install Path Policy (v1.15):** v1.0+ varsayılan kurulum **per-user**: Windows `%LOCALAPPDATA%\Programs\DTransfer\` (yani `AppData\Local\Programs\`), Linux `~/.local/share/dtransfer/` (AppImage durumunda `~/Applications/`). Sebep: **Auto-update UAC/sudo gerektirmez.** Eğer kullanıcı bilinçli olarak `C:\Program Files\` veya `/opt/` altına system-wide install ederse, Updater bunu algılar (`AppData\Local` dışı path) ve **manual update mode**'a geçer — kullanıcıya "Yönetici izinli yeni sürüm hazır, indirip kurun" toast'u, sessiz auto-install yapmaz. Bu davranış MSI installer'da seçenek olarak sunulur ("Bu kullanıcı için" varsayılan / "Tüm kullanıcılar için" alternatif).

### v1.1 — Genişletilmiş Motor

- SCP
- Akıllı kısmi transfer optimizasyonu (S3 / WebDAV / SFTP)
- Ek cloud adapter'lar (Azure Blob, GCS) — değerlendirme aşamasında
- Adapter'lar compile-time bağlı (ProtocolAdapter trait)

### v1.2 — Premium Feature Katmanı (Opt-In)

- Network Optimization Wizard (AV uyarılı, opt-in)
- Audit Trail (KVKK/GDPR rıza metniyle)

### v2.0 — Platform Katmanı *(altyapı v1'de rezerve)*

- Sync Engine (watch mode, bidirectional, conflict resolution)
- Scheduler fairness (WeightedFair, HostAware)

- Memory-mapped large file (16GB+)
- Third-party adapter extension'ı (v2+, sandbox + capability permissions ile)

---

## 7. Teknoloji Yığını

### Frontend
- Vue 3 (Composition API) · Pinia · Tailwind CSS v4 · Vite 6
- JetBrains Mono · Vitest + Playwright
- `vue-virtual-scroller` — 10k+ dosya listesi için zorunlu

### Desktop Runtime — Tauri 2
- `tauri::command` IPC · Native menü/dialog · Updater
- Credential store: `keyring` crate (Windows Credential Manager + Linux Secret Service via D-Bus)
- `tauri-plugin-drag-drop` (v1.0)
- **`tauri-plugin-single-instance` (v1.0 — zorunlu, v1.15)** — DbActor ve RateLimiter in-memory state varsayar; ikinci instance açılırsa aynı `queue.db`'ye paralel yazma → SQLITE_BUSY, aynı API quota'sını habersiz tüketme → 429 ban. Single instance lock ile ikinci başlatma denemesi mevcut pencereyi öne getirir + komut satırı argümanlarını forward eder
- Hedef: Windows 10 1809+, Windows 11, Ubuntu 22.04+, Fedora 38+, Debian 12+ (x86_64)
- Paketleme: MSI + portable ZIP (Win) · AppImage + .deb (Linux) · macOS topluluk portu

### Core Engine (Rust)

| İş | Crate |
|---|---|
| Async runtime | tokio |
| HTTP | reqwest |
| SSH/SFTP | russh + russh-sftp |
| S3 | aws-sdk-s3 |
| Şifreleme (varsayılan) | chacha20poly1305 |
| Şifreleme (alternatif) | aes-gcm |
| KDF | argon2 |
| Secret RAM koruması | zeroize + secrecy |
| Checksum | sha2 + xxhash-rust |
| Seri/Deseri | serde + serde_json |
| Hata | thiserror + anyhow |
| Loglama | tracing + tracing-subscriber |
| Queue DB | rusqlite (WAL mode) + tokio-rusqlite wrapper |
| Filesystem semantik | unicode-normalization |
| TLS | rustls + rustls-platform-verifier |
| Proxy | tokio-socks (SOCKS5) |
| Windows native | windows-rs |
| Stall detector | tokio-metrics |
| Network change | if-watch |
| Path filtering | ignore (gitignore-style globs) |

> **Not:** `rusqlite` tokio ile blocking'dir. Queue progress update yüksek frekanslı olduğu için DbActor pattern (Bölüm 15.4) ile mpsc serileştirme zorunludur.

> **SQLite versiyon determinism (v1.14):** `rusqlite` default olarak sistem SQLite'ı linkler. Ubuntu 22.04 → 3.37, Debian 12 → 3.40, Alpine 3.18 → 3.41. Versiyon farkı WAL davranışı, JSON1 imzaları, `journal_size_limit` semantiği, FTS5 syntax'i konularında "user'da çalışmıyor, bende çalışıyor" bug yatağı. Çözüm: `rusqlite = { version = "0.31", features = ["bundled", "blob", "backup"] }`. Bundled feature SQLite source'u Cargo build'inde derler, tüm platformlarda aynı versiyon (rusqlite 0.31 ↔ SQLite 3.45.x). Trade-off: +30sn build süresi, +700KB binary. Kabul edilebilir.

> **CI matrix:** Default `sqlite-bundled` feature; opsiyonel `sqlite-system` feature Linux distro maintainer'lar için. İki ayrı job ile her ikisi de test edilir.

### Cargo Feature Flags

```toml
[features]
default  = ["ftp", "sftp", "s3", "webdav", "sqlite-bundled"]
ftp      = ["dep:suppaftp"]
sftp     = ["dep:russh", "dep:russh-sftp"]
s3       = ["dep:aws-sdk-s3", "dep:aws-config"]
webdav   = ["dep:diqwest", "dep:reqwest-ntlm"]   # v1.14 — auth scheme crate'leri
sqlite-bundled = ["rusqlite/bundled"]             # v1.14 — versiyon determinism
sqlite-system  = []                               # Linux distro maintainer'lar için
scp      = ["dep:russh"]                    # v1.1
delta    = []                               # v1.1
wizard   = ["dep:command-group"]            # v1.2 (network wizard helper)
sync     = []                               # v2 stub — henüz impl yok
all      = ["sftp","s3","webdav","wizard","sqlite-bundled"]
```

---

## 8. Mimari Genel Bakış

```
┌────────────────────────────────────────────────────────────┐
│                    Vue 3 UI Layer                         │
│  DualPane │ Queue │ Profiles │ Settings │ Diagnostics │
└──────────────────────┬─────────────────────────────────────┘
                       │ tauri::invoke
┌──────────────────────▼─────────────────────────────────────┐
│                 Tauri Command Layer                        │
└──────────────────────┬─────────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────────┐
│                  Rust Core Engine                         │
│                                                           │
│  ┌──────────────────────────────────┐                    │
│  │         DATA PLANE               │                    │
│  │  TransferEngine                  │                    │
│  │  ├── MultipartManager            │                    │
│  │  ├── ResumeEngine (atomic)       │                    │
│  │  ├── RetryManager                │                    │
│  │  ├── RateLimiter (per-profile)   │  ← v1.5           │
│  │  ├── ProgressAggregator (250ms)  │  ← v1.5           │
│  │  ├── CryptoEngine (XChaCha20)    │                    │
│  │  ├── ChecksumVerifier            │                    │
│  │  └── ConnectionPool (semaphore)  │                    │
│  └──────────────────────────────────┘                    │
│                                                           │
│  ┌──────────────────────────────────┐                    │
│  │         QUEUE LAYER              │  ← v1.4           │
│  │  PersistentQueue (queue.db WAL)  │                    │
│  │  ├── PersistedTransferTask       │                    │
│  │  ├── QueueScheduler (FIFO v1)    │                    │
│  │  └── [WeightedFair stub — v2]    │                    │
│  └──────────────────────────────────┘                    │
│                                                           │
│  ┌──────────────────────────────────┐                    │
│  │         CONTROL PLANE            │                    │
│  │  ProtocolAdapter (SFTP/S3)   │                    │
│  │  RemoteProvider (IPC proxy)      │                    │
│  │  ├── JSON-RPC 2.0 dispatcher     │                    │
│  │  ├── PresignedRequest + expiry   │                    │
│  │  └── CapabilityNegotiator        │                    │
│  └──────────────────────────────────┘                    │
│                                                           │
│  ┌──────────────────────────────────┐                    │
│  │  [SYNC ENGINE STUB — v2]         │                    │
│  │  SyncEngine (placeholder)        │                    │
│  └──────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────┘
         │                    │

   JSON-RPC 2.0           JSON-RPC 2.0
```

---

## 9. Rust Core Engine

### 9.1 ProtocolAdapter Trait

```rust
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    async fn connect(&mut self, profile: &ConnectionProfile)  -> Result<(), TransferError>;
    async fn disconnect(&mut self)                             -> Result<(), TransferError>;
    /// Stream API — Vec<RemoteEntry> RAM'i şişirir (2M dosya senaryosu OOM)
    /// Adapter sayfa sayfa fetch eder, scheduler view_cache.db'ye yazar
    fn list_dir(&self, path: &RemotePath, opts: ListOpts)
        -> Pin<Box<dyn Stream<Item = Result<RemoteEntry, TransferError>> + Send + '_>>;
    async fn stat(&self, path: &RemotePath)                   -> Result<RemoteEntry, TransferError>;
    async fn upload(&self, local: &LocalPath, remote: &RemotePath,
                    opts: &TransferOptions, tx: ProgressSender) -> Result<TransferResult, TransferError>;
    async fn download(&self, remote: &RemotePath, local: &LocalPath,
                      opts: &TransferOptions, tx: ProgressSender) -> Result<TransferResult, TransferError>;
    async fn delete(&self, path: &RemotePath)                 -> Result<(), TransferError>;
    async fn mkdir(&self, path: &RemotePath)                  -> Result<(), TransferError>;
    async fn rename(&self, from: &RemotePath, to: &RemotePath)-> Result<(), TransferError>;
    fn supports_byte_range(&self)      -> bool;
    fn supports_remote_checksum(&self) -> bool;
    fn capabilities(&self)             -> AdapterCapabilities;
    fn protocol_info(&self)            -> ProtocolInfo;
}

pub struct ListOpts {
    pub page_size:    u32,         // SFTP default 1024, S3 default 1000
    pub recursive:    bool,         // false = sadece bu dizin
    pub include_hidden: bool,
}
```

**Paginated streaming kuralı (kritik mimari karar):**

`list_dir` **asla** `Vec<RemoteEntry>` döndürmez. 2 yüksek sayılı bir S3 bucket veya `/var/log/` Linux dizini, single-allocation ile RAM'de tutmaya çalışılırsa süreç OOM Killer tarafından öldürülür. Çözüm:

1. Adapter **sayfa sayfa** fetch eder (SFTP `SSH_FXP_READDIR` paketleri, S3 `ContinuationToken`, FTP `MLSD` blokları)
2. Her sayfa Stream üzerinden tüketiciye akar
3. Tüketici (DirectoryListingService) gelen entry'leri `view_cache.db` SQLite tablosuna yazar (Bölüm 22.X)
4. UI (VirtualScroller) RAM yerine view_cache.db'den `LIMIT/OFFSET` ile okur
5. 10 milyon dosya = ~30-50MB RAM (sadece UI viewport + cache index)

**Edge case:** S3 listing 1000 entry/page'in altına düşmez. SFTP server-dependent (32-1024 arası). Adapter `ListOpts.page_size` ile override edebilir, ama default'lar her protokol için sane.

### 9.2 SFTP CapabilityProfile

OpenSSH default `MaxSessions = 10` (per network connection, multiplexed channels). Ancak yıllar içinde bazı sshd türevleri ve gömülü cihazlar bu sınırı düşürüyor — bilinmeyen banner'a karşı **hardcoded liste yerine probe** kullanılır.

```rust
pub struct CapabilityProfile {
    pub max_parallel_sessions: u8,     // probe sonucu (SSH session channels); bilinmiyorsa default: 4
    pub supports_resume:       bool,
    pub supports_fsync:        bool,
    pub server_banner:         String,
    pub max_packet_size:       u32,
}
```

**Probe akışı:** SFTP server kapasitesi iki ayrı sınırla sınırlı:
1. **SSH MaxSessions** — aynı TCP bağlantısı üzerinde paralel açılabilecek SSH **session channel** sayısı. OpenSSH default 10, gömülü cihazlar 2–4'e düşürür.
2. **Per-session inflight file handles** — açık tek bir SFTP subsystem channel'ı içinde simultane açılabilecek dosya handle sayısı.

DTransfer **SSH session channel sınırını** probe eder, dosya handle'ını değil:

```rust
// Aynı SSH transport üzerinde ardışık `subsystem sftp` channel açma denemesi
let mut sessions = Vec::new();
for _ in 0..MAX_PROBE {                       // MAX_PROBE = 10 (üst sınır)
    match transport.open_session_with_subsystem("sftp").await {
        Ok(ch) => sessions.push(ch),          // başarılı, devam et
        Err(_) => break,                       // ilk fail = limit bulundu
    }
}
let max_parallel_sessions = sessions.len().max(1);
// sessions düşürülürse channel'lar otomatik kapanır (RAII)
```

İlk fail olduğunda son başarılı sayı `max_parallel_sessions` olarak kaydedilir (üst sınır 10). Sonuç profile cache'lenir; aynı host'a yeniden bağlanınca tekrar probe yapılmaz.

> **Önemli ayrım:** `SSH_FXP_OPEN` SFTP içinde **dosya** açan request'tir, channel değil. Channel probe için yukarıda gösterilen şekilde `ssh-userauth → ssh-connection → session channel + subsystem` zinciri kullanılır. Bu nüans çok geliştiriciyi yanıltır — değişken adı `max_parallel_channels` veya `max_parallel_sessions` SSH-level kavramı, **dosya inflight limiti farklı** (`SftpTransferLimits.max_inflight_bytes` ile yönetilir).

**Probe başarısız olursa default 4** — modern serverlar 8–10 destekliyor, gömülü cihazlar genellikle 2–4. 4 her iki yönde dengeli.

**SFTP RAM Koruması:** `max_buffered_chunks` tek başına yetmez. Yüksek latency + büyük chunk + yüksek parallelism kombinasyonunda RAM patlaması olur. `max_inflight_bytes` ile byte bazlı üst sınır eklenir:

```rust
pub struct SftpTransferLimits {
    pub max_parallel_sessions: u8,     // CapabilityProfile probe sonucu
    pub max_inflight_bytes:    usize,  // default: 64MB
    // Örnek: 8 chunk × 8MB = 64MB inflight limit
    // Bu sınıra ulaşınca yeni chunk başlatılmaz (backpressure)
}
```

### 9.3 TransferOptions

```rust
pub struct TransferOptions {
    pub chunk_size:          usize,
    pub parallel_streams:    u8,          // CapabilityProfile ile üst sınır
    pub max_inflight_bytes:  usize,       // SFTP RAM koruması: default 64MB
    pub retry_max:           u8,
    pub retry_backoff_ms:    u64,
    pub speed_limit_bps:     Option<u64>,
    pub delta_enabled:       bool,        // v1.1+
    pub verify_checksum:     ChecksumAlgo,
    pub encrypt_at_rest:     bool,
    pub overwrite_policy:    OverwritePolicy,
    pub preserve_mtime:      bool,
    pub max_buffered_chunks: usize,       // backpressure: parallel × 2
}
```

### 9.4 ProgressAggregator

100 transfer × 8 chunk × sürekli emit = Vue render baskısı. `250ms` aggregation window ile throttle edilir:

```rust
pub struct ProgressAggregator {
    // Her transfer_id için son state saklanır
    states:   HashMap<Uuid, ProgressPayload>,
    interval: tokio::time::Interval,  // 250ms
}

impl ProgressAggregator {
    pub fn update(&mut self, payload: ProgressPayload) {
        // Sadece state güncellenir, emit yapılmaz
        self.states.insert(payload.transfer_id, payload);
    }

    pub async fn flush(&mut self, app: &AppHandle) {
        // 250ms'de bir tüm biriken state'leri tek seferde emit et
        for (_, payload) in self.states.drain() {
            app.emit("transfer_progress", &payload).ok();
        }
    }
}

// tokio::spawn ile 250ms interval döngüsü:
// loop { interval.tick().await; aggregator.flush(&app).await; }
```

Aynı prensip hız grafiği için de geçerli — `speed_sample` eventleri de aggregator'dan geçer.

### 9.5 Async / Blocking Sınırları

Tokio runtime'ında **iki ayrı thread pool** vardır:
- **Async worker pool** (default: CPU sayısı kadar) — `async fn` ve `.await` bunlar üzerinde çalışır. Bir worker'ı sync iş ile bloklamak diğer transfer'leri durdurur.
- **Blocking pool** (default: 512 thread, lazy spawn) — `tokio::task::spawn_blocking` ve `tokio::fs` bu havuzu kullanır.

Yanlış anlaşılan nokta: `tokio::fs` zaten internally `spawn_blocking` çağırır. `tokio::fs::rename()` async worker'ı bloklamaz — file I/O sorun değil. **Asıl risk: CPU-bound iş sync olarak `async fn` içinde çalıştırıldığında.**

```rust
// ❌ YANLIŞ — async worker'ı 30ms boyunca tutar (80MB/s SHA-256)
async fn checksum_chunk(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

// ✓ DOĞRU — blocking pool'a yollanır, async worker özgür kalır
async fn checksum_chunk(data: Vec<u8>) -> Result<[u8; 32]> {
    tokio::task::spawn_blocking(move || {
        use sha2::{Sha256, Digest};
        let mut h = Sha256::new();
        h.update(&data);
        Ok::<_, std::io::Error>(h.finalize().into())
    }).await?
}
```

**İş sınıflandırma kuralı:**

| İş tipi | Örnek | Strateji |
|---|---|---|
| Async I/O | TCP socket, TLS handshake, HTTP request | Doğrudan `.await` (zaten async) |
| Async file I/O | `tokio::fs::read/write/rename/remove` | Doğrudan `.await` (tokio internal blocking pool) |
| CPU-bound (kısa, <1ms) | Tek paket parse, JSON deserialize | Async fn içinde sync — sorun değil |
| CPU-bound (orta, 1–10ms) | Küçük chunk hash, JWT verify | `spawn_blocking` önerilir |
| CPU-bound (uzun, >10ms) | Büyük dosya hash, encrypt/decrypt | `spawn_blocking` **zorunlu** |
| Heavy disk + seek | 10GB chunk assemble | `spawn_blocking` + chunked progress callback |
| Sync external lib | `sqlite::busy_handler`, `libssh2` legacy | `spawn_blocking` zorunlu |

**Performans hedefleri açısından kritik yer:**

- **80 MB/s SHA-256** = ~30ms tek thread CPU. Async worker'da çalıştırılırsa o thread 30ms boyunca network I/O okuyamaz → 8 paralel SFTP transferinde 240ms birikim → throughput çöker.
- **XChaCha20 1.5 GB/s** = chunk başına ~5ms. Sınırda ama `spawn_blocking` ile riski sıfırla.
- **10 GB dosya assemble** (rename + seek + truncate sequence) — `spawn_blocking` içinde, her 100MB'da progress callback ile UI'a haber.

**Blocking pool boyutu:** 512 thread default normalde fazla; 16 paralel transfer × 4 chunk concurrent = 64 spawn_blocking peak. Blocking pool tükenmesi olası değil ama `tokio::runtime::Builder::max_blocking_threads(256)` ile makul tutulur (her thread ~512KB stack = 128MB potansiyel).

```rust
// main.rs
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(num_cpus::get())          // CPU bound async worker
    .max_blocking_threads(256)                // CPU/IO bound blocking
    .thread_name("dtransfer-worker")
    .enable_all()
    .build()?;
```

**Test stratejisi:** `tokio::time::Instant::now()` ile her async fn'in execution time histogram'ını tracing span'e yaz. Diagnostics bundle'da "fn xyz median 8ms / p99 45ms" gibi metrikler görünür — 10ms üzeri async worker'da çalışan fn varsa flag'lenir.

**Stall Detector (Watchdog):** `tokio-metrics` ile runtime metrics, ek olarak özel watchdog task:

```rust
pub struct StallWatchdog {
    last_tick: Arc<AtomicU64>,        // monotonic ms
    threshold_ms: u64,                 // default: 2000
}

// Heartbeat task — async worker'da koşar, her 100ms timestamp günceller
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    loop {
        interval.tick().await;
        last_tick.store(now_ms(), Ordering::Release);
    }
});

// Watchdog task — DEDICATED OS THREAD, async runtime DIŞINDA
std::thread::spawn(move || {
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let last = last_tick.load(Ordering::Acquire);
        let lag = now_ms().saturating_sub(last);
        if lag > threshold_ms {
            // Async worker 2sn+ bloklandı — diagnostics'e dump
            tracing::error!(lag_ms = lag, "tokio runtime stall detected");
            // Snapshot: tüm task backtrace, mevcut span'lar
            dump_runtime_state();
        }
    }
});
```

**Kritik:** Watchdog **async runtime'da değil**, normal OS thread'inde koşar. Çünkü stall olduysa async runtime'daki herhangi bir watchdog da donar. Bu detay genelde gözden kaçar.

Diagnostics bundle'da stall event'leri ayrı log dosyası (`stalls.ndjson`) olarak tutulur — debugging'de "ne zaman blok oldu" sorusunu cevaplar.

**Single-Worker Stall Sınırlaması (v1.13 errata):**

Yukarıdaki watchdog **runtime-wide** stall'ı yakalar — yani **tüm async worker'lar** birden bloklandığında. Ama tipik üretim senaryosu farklı: 8 worker'dan **sadece 1 tanesi** CPU-bound bir `async fn` ile 5 saniye bloklanırsa, heartbeat task başka worker'da koşmaya devam eder, `last_tick` güncel kalır, watchdog tetiklenmez. Yine de o tek bloklanan worker'a düşmüş 2 transfer pratikte donar.

Çözüm: `tokio_metrics::TaskMonitor` ile **per-task poll duration** izle:

```rust
use tokio_metrics::TaskMonitor;

pub struct PerTaskStallDetector {
    monitors: HashMap<&'static str, TaskMonitor>,  // "chunk_upload" / "presign_refresh" / ...
}

impl PerTaskStallDetector {
    pub fn instrument<F: Future>(&self, name: &'static str, fut: F) -> impl Future<Output = F::Output> {
        let monitor = self.monitors.get(name).expect("registered task");
        monitor.instrument(fut)
    }

    /// 10sn'de bir tüm task tiplerinin p99 poll süresini kontrol et
    pub async fn audit_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            for (name, monitor) in &self.monitors {
                let metrics = monitor.cumulative();
                let mean_poll_us = metrics.mean_poll_duration().as_micros();
                let slow_poll_count = metrics.slow_poll_count;  // > 50ms (default)
                if slow_poll_count > 0 {
                    tracing::warn!(task = name, slow_polls = slow_poll_count, mean_us = mean_poll_us,
                                   "task showing CPU-bound poll pattern; review for spawn_blocking");
                }
            }
        }
    }
}

// Kullanım:
let task_fut = upload_chunk(chunk_data);
let instrumented = stall_detector.instrument("chunk_upload", task_fut);
let result = instrumented.await;
```

**Slow poll eşiği:** `TaskMonitor` default 50ms. 80MB/s SHA-256 chunk hash'i async worker'da koşturulursa 30ms — eşiğin altında ama yine de sub-optimal. Production öncesi kritik async fn'ler için custom `slow_poll_threshold(Duration::from_millis(10))` set edilir.

**Hangi task'lar instrument edilir:**
- `chunk_upload` / `chunk_download` (yüksek frekans, kritik path)
- `presign_refresh` (network + crypto karışık)
- `db_actor_command` (I/O latency göstergesi)
- `tls_handshake` (CPU-bound RSA verify yer yer 50ms+)

`audit_loop` 10sn aralıklı çalışır; slow poll tespit edilen task tipi diagnostics bundle'da `per_task_stalls.ndjson` olarak ayrı dosyaya yazılır. Runtime-wide stall watchdog ile **birlikte** kullanılır — biri toplu donmayı, diğeri tekil yavaşlamayı yakalar.

### 9.6 Unified Backpressure Model (v1.16)

DTransfer'da parçalı backpressure mekanizmaları zaten var (inflight bytes, ProgressAggregator throttle, EngineEventAggregator batch, DbActor mpsc bounded channel). Ama uçtan uca **flow control graph** açıkça yazılmamıştı — bu da memory ballooning, event storm, lagging UI riskine açık bırakıyordu.

**Tam veri yolu (download örneği):**

```
SFTP stream (network read)
    │  [backpressure: max_inflight_bytes — Bölüm 9.2]
    ▼
Decrypt worker (spawn_blocking, Bölüm 9.5)
    │  [backpressure: blocking pool size, profile-aware — Bölüm 30.4]
    ▼
Per-chunk hash (spawn_blocking)
    │  [backpressure: yine blocking pool]
    ▼
Disk write (tokio::fs, atomic temp file)
    │  [backpressure: disk queue depth, OS-level]
    ▼
ProgressUpdate event emit
    │  [backpressure: ProgressAggregator 250ms throttle — Bölüm 9.4]
    ▼
EngineEvent bus (Bölüm 33)
    ├── UI: broadcast (lossy backpressure tolere) → Aggregator batch 250ms
    └── Diagnostics: mpsc unbounded → DiagnosticsBuffer disk writer (slow olabilir)
    │  [backpressure: yok diagnostics'te by design; disk full → recovery playbook]
    ▼
DbActor command (bytes_done 5s batch)
    │  [backpressure: mpsc channel buffer 1024 — Bölüm 15.4]
    ▼
queue.db write (WAL)
```

**Backpressure choke points (yavaşlama noktaları):**

| Katman | Mekanizma | Limit kaynağı | Yetersizlikte ne olur |
|---|---|---|---|
| Network → Decrypt | `max_inflight_bytes` semaphore | TransferOptions (default 64MB) | Network read durur, peer ACK gönderemez, TCP window kapanır → doğal flow control |
| Decrypt → Hash | spawn_blocking pool semaphore | LimitProfile (LowMemory 64, Server 1024) | Yeni hash job queue'da bekler, decrypt continuation blok eder → network read'e geri yansır |
| Hash → Disk | OS-level (tokio::fs blocking pool) | OS I/O scheduler | Disk yavaşsa write call duration artar, async runtime başka task'lara geçer |
| Disk → ProgressAggregator | Throttle window | 250ms aggregator interval | Update'ler batch'lenir, UI doğal yavaşlar |
| Aggregator → EventBus | Arc<EngineEvent> clone (refcount, cheap) | Yok (refcount unbounded) | Broadcast subscriber yavaşsa eski event'ler düşer (acceptable) |
| EventBus → Diagnostics | mpsc unbounded → disk writer | Disk hızı | Yavaş diskte buffer büyür, RuntimeLimits hard cap'i aşılırsa diagnostics drop + banner |
| EventBus → DbActor | mpsc channel(1024) | Sabit | Channel dolarsa producer'lar `await`'te bekler → tüm zincirde upstream throttling |

**Global pressure detection:**

Backpressure tek başına yetmez — sistem-wide darboğaz nokta tespiti için:

```rust
pub struct BackpressureMetrics {
    /// Her stage için "queue depth": kaç item bekliyor
    pub inflight_bytes:                u64,      // network → decrypt
    pub blocking_pool_active:          u32,      // decrypt + hash threads aktif
    pub progress_aggregator_pending:   u32,      // bekleyen update sayısı
    pub event_bus_lag:                 u64,      // broadcast subscriber'da kaç eski event
    pub db_actor_queue_depth:          u32,      // mpsc(1024) içinde kaç komut
    pub diagnostics_buffer_bytes:      u64,      // mpsc unbounded buffer şişmesi
}

impl BackpressureMetrics {
    /// Hangi stage tıkanmış?
    pub fn bottleneck(&self) -> Option<Bottleneck> {
        if self.db_actor_queue_depth > 800 {
            return Some(Bottleneck::DbWriter);          // disk yavaş
        }
        if self.diagnostics_buffer_bytes > 100 * 1024 * 1024 {
            return Some(Bottleneck::DiagnosticsDisk);   // log diski yavaş
        }
        if self.blocking_pool_active >= self.blocking_pool_capacity() * 9 / 10 {
            return Some(Bottleneck::Crypto);            // CPU bound
        }
        if self.inflight_bytes >= self.max_inflight_bytes() * 9 / 10 {
            return Some(Bottleneck::Network);           // network bound
        }
        None
    }
}
```

**Diagnostics gösterimi:** Diagnostics panel'inde "Bottleneck: Network" / "Bottleneck: Disk Write" / "No bottleneck" badge. Kullanıcı hangi katmanın limit olduğunu görür — performance tuning kararları için (chunk_size, parallel_streams) kanıt sağlar.

**Kapanmamış senaryolar (kabul edilen):**

- **Network → Memory loop:** Eğer network çok hızlı + decrypt yavaş + downstream tıkalı → max_inflight_bytes semaphore reader-side'ı bloklayana kadar geçici memory büyümesi olur. Bu max_inflight_bytes'la (default 64MB) cap'lenir — kabul edilen worst case.
- **EventBus broadcast lag:** Slow UI subscriber broadcast channel'da geride kalırsa eski event'ler düşer. UI gerçekten ihtiyaç duyarsa snapshot çağrısı yapar — acceptable lossy.

Bu unified model **var olan parçaları** birbirine bağlar; yeni mekanizma eklemez. Implementasyon sırasında her stage'in metric'i `BackpressureMetrics` struct'ı üzerinden expose edilir, RuntimeMetrics (Bölüm 30.2) ile entegre olur.

---

## 10. Structured Error Taxonomy

`anyhow` transport layer'da kalır; UI kararları için domain error modeli şart.

```rust
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    // Kimlik doğrulama
    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    #[error("Authorization denied: {path}")]
    Authorization { path: String },

    // Ağ
    #[error("Connection lost after {bytes_sent} bytes")]
    ConnectionLost { bytes_sent: u64 },

    #[error("Connection timeout after {elapsed_ms}ms")]
    Timeout { elapsed_ms: u64 },

    // Dosya sistemi
    #[error("Disk full: {available_bytes} bytes available")]
    DiskFull { available_bytes: u64 },

    #[error("Remote file locked: {path}")]
    RemoteLocked { path: String },

    // Transfer bütünlüğü
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Encryption failure: {reason}")]
    EncryptionFailure { reason: String },

    // API / Rate limit
    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("API quota exceeded")]
    QuotaExceeded,

    #[error("Presigned URL expired")]
    UrlExpired,

    // Adapter

    #[error("Adapter capability not supported: {capability}")]
    CapabilityNotSupported { capability: String },

    // Sistem
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

UI bu enum'a göre karar verir:

| Hata | UI Aksiyonu |
|---|---|
| `Authentication` | Kimlik bilgilerini güncelle modal |
| `ConnectionLost` | Otomatik reconnect + retry |
| `Timeout` | Retry with backoff |
| `DiskFull` | Kullanıcıya uyar, pause |
| `ChecksumMismatch` | Yeniden indir, hata logla |
| `RateLimited` | `retry_after_secs` bekle, otomatik devam |
| `UrlExpired` | `refresh_presigned` IPC çağrısı |


---

## 11. Protokol Desteği

### 11.1 v1.0 Ana Paket

| Protokol | Kütüphane | Multipart | Delta |
|---|---|---|---|
| SFTP (SSH-2) + CapabilityProfile | russh-sftp | ✓ | ✗ (v1.1) |
| WebDAV | reqwest custom + auth scheme'ler | ⚠ sunucu | ✗ (v1.1) |
| AWS S3 / MinIO / R2 | aws-sdk-s3 | ✓ Native | ✗ (v1.1) |
| Backblaze B2 | REST | ✓ | ✗ |

### 11.1.1 WebDAV Auth Schemes (v1.14)

Gerçek dünya WebDAV server'ları farklı auth scheme'ler talep eder. Bu desteklenmeden WebDAV "implement edildi" denemez — özellikle kurumsal SharePoint pazarı NTLM/Negotiate olmadan erişilemez.

| Server | Tipik Auth Scheme |
|---|---|
| Nextcloud | Basic over HTTPS + opsiyonel app password |
| ownCloud | Basic + Bearer (OAuth2) |
| SharePoint (on-prem) | NTLM (kurumsal AD entegre) |
| SharePoint Online | OAuth2 Bearer (Azure AD) |
| Synology WebDAV | Basic veya Digest |
| Apache mod_dav | Basic, Digest |
| IIS WebDAV | NTLM, Negotiate (Kerberos), Basic |

```rust
pub enum WebDavAuth {
    None,                                    // public read-only
    Basic    { username: SecretString, password: SecretString },
    Digest   { username: SecretString, password: SecretString },
    Bearer   { token: SecretString },        // OAuth2 access token
    Ntlm     { username: SecretString, password: SecretString, domain: Option<String> },
    Negotiate,                               // v1.1 — Kerberos, sistem keytab gerekli
}

pub struct WebDavClient {
    base_url:  Url,
    auth:      WebDavAuth,
    http:      reqwest::Client,
}
```

**Auth scheme detection (probe):**

İlk bağlantıda OPTIONS isteği at, `WWW-Authenticate` header'larından server'ın desteklediği scheme'leri çıkar:

```rust
impl WebDavClient {
    pub async fn probe_auth_schemes(&self) -> Result<Vec<&'static str>> {
        let resp = self.http.request(Method::OPTIONS, self.base_url.clone()).send().await?;
        if resp.status() != StatusCode::UNAUTHORIZED {
            return Ok(vec![]);
        }
        // WWW-Authenticate header'larını parse et: "Basic realm=...", "Digest realm=...", "NTLM", "Negotiate"
        let schemes: Vec<&'static str> = resp
            .headers()
            .get_all("WWW-Authenticate")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|s| s.split(','))
            .filter_map(|s| {
                let token = s.trim().split_whitespace().next()?;
                Some(match token.to_lowercase().as_str() {
                    "basic"     => "Basic",
                    "digest"    => "Digest",
                    "bearer"    => "Bearer",
                    "ntlm"      => "NTLM",
                    "negotiate" => "Negotiate",
                    _ => return None,
                })
            })
            .collect();
        Ok(schemes)
    }
}
```

**Crate seçimi:**

| Auth | Crate | Notlar |
|---|---|---|
| Basic | reqwest built-in | `basic_auth()` |
| Bearer | reqwest built-in | `bearer_auth()` |
| Digest | `diqwest` veya `reqwest-middleware` + custom | RFC 7616 |
| NTLM | `ntlmclient` veya `reqwest-ntlm` | NTLM SSP gerektirir |
| Negotiate (Kerberos) | `cross-krb5` | v1.1+, sistem keytab gerekli |

**PROPFIND XML lenience:**

WebDAV server'lar PROPFIND XML formatında farklılaşır:
- Nextcloud: namespaced (`d:`, `oc:`, `nc:`) properties
- SharePoint: kendine has property'ler, bazıları RFC dışı
- Apache: standart RFC 4918

XML parser **lenient** olmalı — tanımadığı property'leri ignore etmeli. `quick-xml` ile manuel parse veya `serde-xml-rs` (daha yavaş ama strict). `quick-xml` önerilir.

### 11.2 v1.1 Eklentiler

- SCP (russh)
- Akıllı kısmi transfer optimizasyonu — S3 (ETag), WebDAV (ETag), SFTP (checksum map)
- Ek bulut adapter'ları (Azure Blob, GCS) — değerlendirme aşamasında

### 11.3 Delta Transfer Kısıtı

Uzak checksum'ı bulunmayan protokollerde (örn. local FS over slow networks) tüm dosya okunur — kazanç negatif olur. `ProtocolAdapter::supports_remote_checksum()` → `false` dönerse UI delta seçeneğini gizler.

### 11.4 Protocol Capability Tier Matrix

Hangi protokol hangi yeteneği gerçekten destekliyor? Bu tablo:
- Implementation scope'unu korur
- UX beklentisini netleştirir ("neden burada rename atomic değil?" sorusunu sıfırlar)
- Bug rapor sayısını azaltır
- Test matrisinin temelini oluşturur

| Capability | Local FS | SFTP | FTP/FTPS | S3 / Object Store | WebDAV |
|---|---|---|---|---|---|
| Atomic rename | ✅ | ⚠️ server'a bağlı | ❌ (RNFR/RNTO non-atomic) | pseudo (CopyObject + Delete) | ⚠️ MOVE method |
| Resume reliable | ✅ | ✅ | ⚠️ REST komut server-dep | ✅ multipart resume | ⚠️ Range header server-dep |
| Sparse preserve | ✅ | ⚠️ extension @openssh | ❌ | ❌ (object store) | ❌ |
| fsync remote | n/a | extension `fsync@openssh.com` | ❌ | object-store eventual consistency | ❌ |
| Checksum verify | sha256 (lokal) | extension `check-file` | ❌ | ETag (MD5 single-part, custom multi) | optional `getetag` |
| Symlink preserve | ✅ | ✅ (SSH_FXP_SYMLINK) | ⚠️ `SITE SYMLINK` non-standard | ❌ | ❌ |
| Permissions | ✅ (chmod) | ✅ | ✅ (`SITE CHMOD`) | ❌ (ACL ayrı) | ❌ |
| Mtime preserve | ✅ | ✅ (SETSTAT) | ⚠️ MFMT extension | ⚠️ metadata header | ✅ |
| Partial write recovery | ✅ | ✅ | ❌ | ✅ (multipart abort+retry) | ⚠️ |
| Long path > 260 char | ✅ (`\\?\` Win) | ✅ | server-dep | ✅ (1024 byte cap) | server-dep |
| Unicode NFC/NFD | OS-dep | server-dep | server-dep | NFC tercih | server-dep |
| Resumable multipart | n/a | chunk paralel | ❌ | ✅ S3 native | ❌ |

**Tier sınıfları:**
- **Tier 1** (Local FS, SFTP modern, S3): Tam capability set, default UX akışları çalışır
- **Tier 2** (FTP/FTPS, WebDAV, SFTP eski): Bazı özellikler degraded, UI banner ile bilgilendir
- **Tier 3** (Edge cases): Symlink/sparse/fsync gibi nadir özellikler — kullanıcıya açıkça "bu sunucu desteklemiyor" denir

**Implementation kuralı:** ProtocolAdapter trait her capability için `supports_*()` metodu döndürür. UI bu metoda göre seçenekleri görsel olarak disable eder veya gizler — kullanıcı tıklayıp hata alma deneyimi yaşamaz.

```rust
pub trait ProtocolAdapter {
    fn supports_atomic_rename(&self) -> bool;
    fn supports_resume(&self) -> bool;
    fn supports_sparse(&self) -> bool;
    fn supports_remote_checksum(&self) -> ChecksumKind;
    fn supports_symlinks(&self) -> SymlinkSupport;
    fn supports_fsync(&self) -> bool;
    fn supports_permissions(&self) -> bool;
    fn supports_long_paths(&self) -> Option<usize>;  // None = unlimited, Some(N) = byte cap
    fn supports_multipart_resume(&self) -> bool;
}
```

UI tarafında composable:

```typescript
const caps = useProtocolCapabilities(profile.protocol);
// caps.atomicRename: ComputedRef<boolean>
// Modal'da "atomic rename yok, sunucu destek vermiyor" banner'ı
```

### 11.5 HTTP/2 Connection Pool Tuning (S3 Fan-out, v1.14)

aws-sdk-s3 hyper kullanır, hyper kendi connection pool'unu yönetir. Default `pool_max_idle_per_host = 32`. 1000+ küçük dosya S3'e upload edilirken bu yetersiz kalır — TLS handshake overhead (~50ms per yeni connection) throughput'u öldürür.

```rust
use aws_sdk_s3::config::HttpClient;
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;

pub fn build_s3_client(config: &S3Config) -> aws_sdk_s3::Client {
    let hyper_client = hyper::Client::builder()
        .pool_max_idle_per_host(64)                       // default 32 → 64
        .pool_idle_timeout(Duration::from_secs(90))
        .http2_only(false)                                // S3 HTTP/1.1 + multiplexing
        .http2_keep_alive_interval(Duration::from_secs(30))
        .build::<_, hyper::Body>(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .enable_http2()
                .build()
        );

    let sdk_config = aws_config::from_env()
        .http_client(HyperClientBuilder::new().build(hyper_client))
        .load()
        .await;

    aws_sdk_s3::Client::new(&sdk_config)
}
```

**Benchmark hedefi:** 10,000 küçük dosya (1KB each) S3 upload. Default pool: ~5dk; tuned (64 idle): ~2dk. Beklenen kazanç %50-70.

**Profile-aware tuning:** `LimitProfile::Server` için `pool_max_idle_per_host = 128`; `LowMemory` için 16. RuntimeLimits içine yeni field eklenir.

### 11.6 S3 ETag ≠ MD5 — Checksum Warning Policy (v1.16)

S3 protokolünde `ETag` header'ı yaygın olarak content checksum gibi kullanılır — ama bu **çoğu zaman yanlış**. Naïve "ETag = MD5" varsayımı sessiz data corruption confidence yaratır.

**ETag ne zaman MD5'tir, ne zaman değildir:**

| Senaryo | ETag içeriği |
|---|---|
| Single-part PUT (<5MB), no encryption | `MD5(content)` hex string ✓ |
| Single-part PUT, SSE-S3 (`AES256`) | MD5'e benzer hex ama **encrypted blob'un** MD5'i, plaintext değil — verify için kullanılamaz |
| Single-part PUT, SSE-KMS | Tamamen opak — KMS-managed encryption, MD5 değil |
| Single-part PUT, SSE-C (customer-provided key) | Plaintext MD5'i; ama her zaman doğrulamak zor |
| **Multipart upload** (>5MB veya parça boyutu) | `MD5(MD5(part1) ‖ MD5(part2) ‖ ...)-<part_count>` — **hierarchical hash**, content MD5 değil |
| MinIO compatibility layer | S3 davranışını taklit eder ama bazı edge case'lerde sapma |
| Cloudflare R2 / Backblaze B2 / Wasabi | Genellikle uyumlu ama her birinin küçük farkları var |
| AWS gateway / VPC endpoint | ETag header proxy'de modify edilebilir (nadiren) |

**Akıllı kısmi transfer optimizasyonu (Bölüm 11.2/8.3) implikasyonu:**

Akıllı kısmi transfer optimizasyonu "remote ETag = local checksum mı?" sorusuna dayanır. Eğer ETag MD5 değilse delta yanılır → **silent data corruption** veya **false positive re-download**.

**Çözüm: katmanlı checksum policy.**

```rust
pub enum S3ChecksumStrategy {
    /// ETag'i checksum olarak güven (varsayılan değil)
    /// Sadece "biliyorum ki bu bucket multipart yok + SSE yok"
    TrustEtag,

    /// AWS Additional Checksums (CRC32/CRC32C/SHA1/SHA256) header'larını kullan
    /// PutObject'te ChecksumAlgorithm=SHA256 ile yükle, GetObject'te response header'da
    /// gerçek content hash dön. Bu modern S3 API (2022+) ve MinIO'da destekli.
    UseAwsChecksumApi { algorithm: S3ChecksumAlg },

    /// İndirme sonrası local'de re-hash et, ayrı verify
    /// Multipart upload yapılmış object'lerde tek güvenli yol
    LocalRehashVerify,

    /// Hiç verify yapma (kullanıcı manuel onay; UI'da uyarı)
    NoVerify,
}

pub enum S3ChecksumAlg { Crc32, Crc32c, Sha1, Sha256 }
```

**Default davranış (v1.0):** `UseAwsChecksumApi { algorithm: Sha256 }`. AWS 2022'de eklenen Additional Checksums API — `x-amz-checksum-sha256` header ile gerçek content hash. S3, MinIO 2023+, R2 destekli. Default'u SHA256, fallback CRC32C (daha hızlı, S3 native support).

**Fallback chain (server desteklemiyorsa):**

```rust
async fn verify_s3_object(client: &S3Client, key: &str, expected: &Sha256Hash) -> Result<bool> {
    // 1. AWS Additional Checksums (en güvenilir)
    if let Some(server_sha256) = client.head_object(key).await?.checksum_sha256() {
        return Ok(server_sha256 == expected.base64());
    }

    // 2. Single-part + no SSE → ETag MD5'tir, MD5 hesaplayıp karşılaştır
    let meta = client.head_object(key).await?;
    let is_multipart = meta.etag().contains('-');           // "abc123-5" formatı
    let has_sse = meta.server_side_encryption().is_some();

    if !is_multipart && !has_sse {
        let local_md5 = md5(local_content);
        let server_md5 = meta.etag().trim_matches('"');
        return Ok(local_md5.hex() == server_md5);
    }

    // 3. Fallback: local re-hash verify
    tracing::warn!(key, "ETag unreliable (multipart={}, sse={}); falling back to local re-hash verify",
                   is_multipart, has_sse);
    Ok(LocalRehashVerify::verify(local_path, expected).await?)
}
```

**Kullanıcı uyarısı (Settings → S3 → Verification):**

```
☐ ETag'i checksum olarak kullan (HIZLI, ama multipart/SSE'de yanlış olabilir — TEHLİKELİ)
☑ AWS Additional Checksums API (GÜVENLİ, default; SHA-256/CRC32C)
☐ Local re-hash verify (HER ZAMAN GÜVENİLİR ama yavaş — büyük dosyalarda 2x I/O)
```

UI'da bu seçenek **per-profile** override edilebilir — bazı bucket'lar (örn. legacy gateway) Additional Checksums API destekleemiyor, kullanıcı bilinçli olarak `LocalRehashVerify`'a geçer.

**Loud warning policy:**

İlk kez bir S3 bucket'a bağlanıldığında, DTransfer probe eder:
1. Test object yükle (1KB, multipart=hayır, SSE=hayır)
2. `x-amz-checksum-sha256` header döndü mü? → Additional Checksums destekli
3. Destek yoksa kullanıcıya **modal uyarı**: *"Bu S3 endpoint'i modern checksum API'sini desteklemiyor. ETag MD5 değil — multipart yüklemelerde silent corruption riski var. Local re-hash verify mode'a geçilsin mi?"*

**Bölüm 11.3 Delta Transfer ile cross-reference:** Akıllı kısmi transfer optimizasyonu artık `S3ChecksumStrategy::UseAwsChecksumApi` veya `LocalRehashVerify` zorunlu — `TrustEtag` mode'da delta disabled. Adapter `supports_remote_checksum()` ETag güvenilir değilse `false` döner, UI delta seçeneğini gizler.

---

## 12. Filesystem Edge-Case Matrisi

Production'da en sık bug üretilen yer. Network/protocol katmanı sağlam olsa bile filesystem semantik farkları (Win vs Linux vs S3) sessizce data corruption üretir. Symlink, Unicode, case sensitivity, reserved name, path length — beşi de v1.0'da çözülmek zorunda.

### 12.1 Symlink Politikası

```rust
pub enum SymlinkPolicy {
    /// Hedef dosyayı transfer et (symlink'i takip et)
    Follow,
    /// Symlink'i symlink olarak transfer et (hedefi değil)
    Preserve,
    /// Symlink'leri atla, log'a yaz
    Skip,
}

pub struct SymlinkOptions {
    pub policy:           SymlinkPolicy,
    pub max_depth:        u8,           // recursive symlink loop koruması, default 8
    pub allow_dangling:   bool,         // hedef olmayan symlink, default false
}
```

**Davranış matrisi:**

| Durum | Follow | Preserve | Skip |
|---|---|---|---|
| `link → file.txt` | file.txt içeriği | symlink (ProtocolAdapter destekliyorsa) | atla |
| `link → /nonexistent` | hata: dangling | preserve (allow_dangling) veya hata | atla |
| `a → b → c → a` (loop) | hata: max_depth aşıldı | preserve (link olarak) | atla |
| Windows junction | yumuşak symlink gibi davran | (Win-specific) preserve junction | atla |
| Windows reparse point | follow (system'in çözdüğü yere) | preserve mümkünse | atla |

**Protokol bazlı destek:**

- **Local FS:** Tümü destekli (`std::os::unix::fs::symlink`, `std::os::windows::fs::symlink_*`)
- **SFTP:** `Preserve` mümkün (`SSH_FXP_SYMLINK`), `Follow` her zaman çalışır
- **S3 / WebDAV / Cloud:** Symlink kavramı yok → `Follow` zorunlu, log'da uyarı

**Default:** `Follow` + `max_depth=8` + `allow_dangling=false`. Power user `Preserve`'a geçer (rsync benzeri full backup için).

**Absolute Symlink Sanitization (güvenlik):**

`Preserve` modunda kötü niyetli sunucu istemciyi şu saldırıya açabilir:

```
remote: /var/www/data/passwd_link → /etc/passwd
local indirildi: ~/Downloads/data/passwd_link → /etc/passwd
```

Kullanıcı `passwd_link`'e tıklarsa kendi sisteminin `/etc/passwd`'ına yönlenir. Windows'ta benzeri: `→ C:\Windows\System32\config\SAM`. Bu **CVE-class güvenlik açığıdır** — bilinçli olmasa bile passive zaafiyet.

```rust
pub enum SymlinkTargetPolicy {
    /// Mutlak yol → göreli yol çevrimi (transfer kök dizinine göre)
    /// /etc/passwd → ../../etc/passwd (eğer transfer kökü içindeyse)
    /// Aksi halde Skipped + log uyarı
    SanitizeOrSkip,
    /// Mutlak yol → her zaman Skipped, asla preserve etme
    AlwaysSkipAbsolute,
    /// Olduğu gibi koru — UYARI: güvenlik riski, sadece güvenli kaynak
    PreserveAsIs,
}

pub fn sanitize_symlink_target(
    target: &Path,
    transfer_root: &Path,
    policy: SymlinkTargetPolicy,
) -> Option<PathBuf> {
    if !target.is_absolute() { return Some(target.to_path_buf()); }
    match policy {
        SymlinkTargetPolicy::SanitizeOrSkip => {
            // Hedef transfer kökü içindeyse → relative
            target.strip_prefix(transfer_root).ok().map(|p| p.to_path_buf())
        }
        SymlinkTargetPolicy::AlwaysSkipAbsolute => None,
        SymlinkTargetPolicy::PreserveAsIs => Some(target.to_path_buf()),
    }
}
```

**Default:** `SanitizeOrSkip`. Kullanıcı bilinçli olarak `PreserveAsIs`'e geçmedikçe absolute target'lar Skipped olarak `~/.dtransfer/skipped.log`'a yazılır + UI'da yellow warning badge.

**Windows-specific:** `\\server\share\` UNC path symlink'leri — network share'e yönlendiren symlink local indirildiğinde aynı şekilde dış kaynak hijack riski. Aynı politika uygulanır.

### 12.2 Unicode Filename Normalization

`unicode-normalization` crate. macOS NFD, Windows/Linux genelde NFC. Türkçe `ş`, `ğ`, `ı` karakterleri:
- NFC: `U+015F` tek codepoint
- NFD: `U+0073 U+0327` (s + combining cedilla)

Aynı görsel dosya, farklı binary. Naive byte compare = duplicate detection bozulur, sync engine v2'de patlar, conflict resolution yanlış sonuç verir.

```rust
pub enum UnicodeNormalization {
    /// Normalizasyon yok, byte-exact karşılaştır (SFTP-default)
    None,
    /// Internal compare için NFC normalize et (DTransfer default)
    Nfc,
    /// Filesystem'in tercihi (macOS=NFD, diğerleri=NFC) — port için
    Native,
}

pub fn normalize_for_compare(path: &str) -> Cow<str> {
    use unicode_normalization::UnicodeNormalization as UN;
    Cow::Owned(path.nfc().collect::<String>())
}
```

**Kural:**
- **Wire-format:** Server'ın verdiği byte'larla yazılır (NFC'ye çevrilmez — server kafası karışır)
- **Internal compare:** Daima NFC'ye normalize edilmiş halde karşılaştırılır (`HashMap<NormalizedPath, ...>`)
- **Display:** UTF-8 NFC olarak gösterilir, copy-paste'ten gelen NFD input NFC'ye düzeltilir

`NormalizedPath` newtype:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedPath(String);  // her zaman NFC

impl NormalizedPath {
    pub fn from_raw(s: &str) -> Self { Self(s.nfc().collect()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

**NFC/NFD Cross-FS Collision (v1.16 clarification):**

Senaryo: macOS APFS'te oluşturulan `é` (NFD: `e` + `\u{0301}`) dosyası Linux ext4'e kopyalanır → ayrı inode olarak `é` (NFC, U+00E9) ile yan yana durur. Linux iki ayrı dosya olarak görür. Sonra bu Linux dizini Windows NTFS'e kopyalanır → **collision** (Windows case-insensitive bonus, NFC vs NFD aynı görünür).

DTransfer policy: NormalizedPath (NFC) internal compare zaten **collision'ı yakalar**. ConflictPolicy enum (Bölüm 23.1) ile UX:

- `Ask` (default): Modal "İki dosya aynı isimde gözüküyor ama farklı Unicode normalization'a sahip (NFC vs NFD). Hangisini tutmalı?"
- `KeepNewer`: Mtime kazanır, diğeri sessiz skip
- `Rename`: `é (NFD).txt` ve `é (NFC).txt` olarak ayrı isimde tut

Bölüm 23'deki Conflict UX ile birleşik — yeni bölüm açmaya gerek yok, sadece kullanıcı net şekilde "neden iki aynı isimli dosya?" sorusuna cevap alır.

### 12.3 Case Sensitivity Matrisi

| FS | Case |
|---|---|
| Windows NTFS | Insensitive (default) |
| Windows NTFS (Win 10+ flag'li dizin) | Sensitive (per-directory, nadir) |
| Linux ext4/btrfs/xfs | Sensitive |
| macOS APFS (default) | Insensitive |
| macOS APFS (case-sensitive volume) | Sensitive |
| S3 / GCS / Azure Blob | Sensitive |
| SFTP server | Server'ın FS'ine bağlı |

**Conflict tipleri:**

```rust
pub enum PathConflict {
    /// Aynı path, byte-exact (gerçek conflict)
    ExactMatch,
    /// Sadece case farkı (Windows'ta aynı dosya, Linux'ta farklı)
    SamePathDifferentCase {
        existing: String,
        incoming: String,
    },
    /// Unicode equivalent (NFC vs NFD)
    UnicodeEquivalent,
    /// Reserved name conflict (CON.txt vs con.txt Windows'ta aynı)
    ReservedNameMatch,
}

pub fn detect_conflict(
    existing: &NormalizedPath,
    incoming: &NormalizedPath,
    target_fs: FilesystemKind,
) -> Option<PathConflict> {
    if existing == incoming { return Some(PathConflict::ExactMatch); }
    if target_fs.is_case_insensitive()
        && existing.as_str().eq_ignore_ascii_case(incoming.as_str())
    {
        return Some(PathConflict::SamePathDifferentCase {
            existing: existing.as_str().into(),
            incoming: incoming.as_str().into(),
        });
    }
    None
}
```

**Senaryo (production bug):** Linux SFTP server'ında `Readme.txt` ve `README.txt` ayrı dosya. Windows'a indirilirken ikinci dosya birinciyi ezer. Bu sessizce olur — kullanıcı veri kaybeder. Conflict Resolution (Bölüm 23) bu durumu yakalar, kullanıcıya sorar.

### 12.4 Reserved Filename ve Invalid Path

**Windows'ta yasak isimler** (case-insensitive, uzantıyla bile):

```
CON, PRN, AUX, NUL,
COM1, COM2, ..., COM9, COM¹, COM², COM³,
LPT1, LPT2, ..., LPT9, LPT¹, LPT², LPT³
```

`CON.txt`, `con.json`, `aux.log` — hepsi yasak. Linux'ta valid, S3'te valid, SFTP server (Linux) valid → Windows'a indirirken sorun.

**Trailing dot/space:** `file.txt.` veya `file.txt ` Windows'ta oluşturulamaz (Win API otomatik kırpar, sonuç `file.txt`).

**Yasak karakterler (Windows):** `< > : " / \ | ? *` ve kontrol karakterleri (0x00-0x1F).

**Linux yasak:** sadece `/` ve `\0`.

**S3 valid ama problemli:** Anchor character'lar (`?`, `#`, `&`) URL encoding gerektirir.

```rust
pub enum InvalidPathStrategy {
    /// Hata, transfer'i durdur
    Fail,
    /// Otomatik isim değiştir (CON.txt → CON_.txt, file. → file_.)
    Rename { suffix: String },     // default: "_"
    /// URL-encode tarzı escape (CON.txt → %43%4F%4E.txt) — geri dönüştürülemez
    Escape,
}

pub fn sanitize_for_target(path: &str, target_fs: FilesystemKind) -> Result<String, InvalidPath> {
    match target_fs {
        FilesystemKind::Windows => {
            // 1. Yasak karakter kontrol
            // 2. Reserved name kontrol (filename + extension stripping ile)
            // 3. Trailing dot/space kontrol
            // ...
        }
        FilesystemKind::Linux | FilesystemKind::Posix => {
            // Sadece / ve \0
        }
        FilesystemKind::S3 | FilesystemKind::ObjectStore => {
            // URL encoding for ?, #, &, %
        }
    }
}
```

**Default:** `Rename { suffix: "_" }`. Power user `Fail`'e geçer (data integrity uber alles).

### 12.5 Path Length Limits

| Sistem | Sınır |
|---|---|
| Windows MAX_PATH (legacy API) | 260 karakter |
| Windows long path (`\\?\` prefix) | 32,767 karakter |
| Windows UNC path (`\\server\share\...`) | 260 ya da `\\?\UNC\server\share\...` ile 32,767 |
| Linux PATH_MAX | 4096 byte |
| Linux NAME_MAX | 255 byte (tek bileşen) |
| S3 object key | 1024 byte |

**Strateji:** DTransfer Win 10+ için `\\?\` prefix kullanır (long path opt-in). Manifest'te `enable_long_paths: true`. Linux'ta `PATH_MAX 4096` yeterli, S3'te 1024 cap kontrolü.

```rust
pub const WINDOWS_LONG_PATH_PREFIX: &str = r"\\?\";
pub const WINDOWS_UNC_LONG_PREFIX:  &str = r"\\?\UNC\";

pub fn windows_canonicalize(path: &Path) -> PathBuf {
    let s = path.display().to_string();
    if s.len() > 240 && !s.starts_with(WINDOWS_LONG_PATH_PREFIX) {
        if let Some(rest) = s.strip_prefix(r"\\") {
            // UNC path → \\?\UNC\server\share
            PathBuf::from(format!("{}{}", WINDOWS_UNC_LONG_PREFIX, rest))
        } else {
            PathBuf::from(format!("{}{}", WINDOWS_LONG_PATH_PREFIX, s))
        }
    } else {
        path.to_path_buf()
    }
}
```

**Long path uyumsuzluk uyarıları:**
- **Windows Explorer < Win 10 1607:** `\\?\` prefixli path'leri açamaz. Kullanıcıya local kopyalanan dosya 280 karakterse "Explorer'dan açılamayabilir, kısaltmayı düşünün" uyarısı.
- **robocopy / xcopy:** Long path desteği var ama `/256` flag gerekli. DTransfer manifest'te dosyayı oluştururken normal path kalır, kullanıcı third-party tool'a yazarsa bu sorun.
- **Antivirus tarayıcılar:** Bazı eski AV motorları `\\?\` path'leri tarayamaz, transfer sonrası AV scan window'unda bypass olabilir. Bilinçli karar — performance vs scan coverage.

UI tarafında local path > 240 karakter = uyarı badge "Long path — Explorer compat. limited". Conflict Resolution modal'ında otomatik gösterim.

### 12.6 Edge-Case Test Matrisi

```rust
#[test] fn ntfs_case_insensitive_collision_detected()    { ... }
#[test] fn nfc_nfd_filename_treated_as_duplicate()       { ... }
#[test] fn windows_reserved_name_renamed_with_suffix()   { ... }
#[test] fn dangling_symlink_skipped_when_disallowed()    { ... }
#[test] fn symlink_loop_max_depth_8_terminates()         { ... }
#[test] fn long_path_260_plus_uses_long_prefix()         { ... }
#[test] fn unc_long_path_uses_unc_prefix()               { ... }
#[test] fn windows_junction_treated_as_symlink()         { ... }
#[test] fn s3_url_encoded_for_special_chars()            { ... }
#[test] fn trailing_dot_filename_rejected_on_windows()   { ... }
#[test] fn invalid_utf8_path_preserved_as_bytes()        { ... }
#[test] fn invalid_utf8_serialized_as_base64_in_ipc()    { ... }
```

### 12.7 Invalid UTF-8 Path Politikası

Linux'ta filename byte sequence olabilir, UTF-8 olmak zorunda değil. Bir kullanıcı `\xff\xfe.txt` adında dosya oluşturabilir (bilinçli ya da bozuk encoding ile). Rust'ın `PathBuf` (`Path`) bunu **OsString** olarak taşır — UTF-8 garantisi yok. Ama:
- JSON serialize: `serde_json` UTF-8 zorunlu, panic eder
- UI render: HTML/JS UTF-8, "?" placeholder göstermek kullanıcı için anlamsız

```rust
pub enum PathTransport {
    /// UTF-8 path, doğrudan string
    Utf8(String),
    /// Non-UTF-8 path, base64-encoded raw bytes
    RawBytes { base64: String, hint: String },  // hint: lossy display
}

impl PathTransport {
    pub fn from_path(p: &Path) -> Self {
        if let Some(s) = p.to_str() {
            Self::Utf8(s.to_string())
        } else {
            // Non-UTF-8 (Linux only) — base64 transport, lossy hint
            #[cfg(unix)]
            let bytes = p.as_os_str().as_bytes();
            #[cfg(windows)]
            let bytes = p.as_os_str().to_string_lossy().as_bytes().to_vec();
            Self::RawBytes {
                base64: base64::encode(bytes),
                hint: p.to_string_lossy().into_owned(),  // ? karakterleri ile
            }
        }
    }
}
```

**Davranış matrisi:**

| Durum | Politika |
|---|---|
| Local Linux invalid UTF-8 | Hex-escape display (`\xff\xfe.txt`) UI'da, raw bytes preserve |
| Remote (SFTP) invalid UTF-8 filename | Lossy display (`?` ile) + raw bytes preserve, transfer'de byte-exact write |

| Sync engine v2 compare | NFC normalize sonrası bytes'a fallback (UTF-8 çevrim olmazsa) |
| Conflict modal display | Tooltip'te hex dump, ana satırda lossy preview |

**UI rule:** Invalid UTF-8 path mutlaka **kırmızı badge ile** işaretlenir: "Non-standard filename". Kullanıcı bilinçli olarak fark eder, başka kullanıcılarla paylaşırken sorun çıkarabileceğini bilir.

**Edge case:** Windows tarafına invalid UTF-8 dosya transfer edilirse → NTFS UTF-16 zorunlu, bytes UTF-8 → UTF-16 dönüşümünde fail eder. Bu durumda `InvalidPathStrategy::Rename` devreye girer (Bölüm 12.4), filename `_invalid_utf8_<hash>.bin` formatında yeniden adlandırılır + audit log'a orijinal raw bytes yazılır.

### 12.8 Windows Shared Read (Kilitli Dosya Yedekleme)

Kullanıcı yedekleme yapıyor: aktif `server.log`, açık Outlook `.pst`, çalışan SQLite `.db`. Standart Rust `File::open` (CreateFile default flag'leri) bu dosyalar başka process tarafından **exclusive** açılmışsa `ERROR_SHARING_VIOLATION` döner. Klasik istemci dosyayı `Failed` olarak işaretler — kullanıcının en kritik yedekleme senaryosu çöker.

```rust
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_DELETE};

pub fn open_for_backup_read(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(windows)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(path)
    }
    #[cfg(unix)]
    {
        // POSIX'te shared read default — özel flag gerekmez
        std::fs::File::open(path)
    }
}
```

**Davranış:**
- `FILE_SHARE_READ | FILE_SHARE_WRITE`: dosya başka process tarafından yazılırken bile DTransfer **o anki snapshot'ı** okuyabilir
- `FILE_SHARE_DELETE`: yazan process dosyayı silmeye çalışsa bile DTransfer'ın handle'ı son okumayı tamamlar
- Snapshot consistency garantisi YOK (dosya yazılırken tutarsız byte'lar görülebilir) — bu **bilinçli trade-off**: Outlook .pst yedekleme gibi senaryolarda "tutarsız ama kopya alınmış" > "hiç kopya yok"

**Sınırlamalar (kullanıcıyı bilgilendir):**
- **Word/Excel açık dosya:** Office FILE_SHARE_READ vermiyor — bu durumda `ERROR_SHARING_VIOLATION` kalıcı, çözüm: VSS (Volume Shadow Copy Service) gerek → v1.1+ planlı
- **Antivirüs scan locking:** Bölüm 14.2 AV Lock micro-retry ile ayrı ele alınır
- **Tutarlılık uyarısı:** UI'da "Bu dosya kullanımda — tutarsız snapshot olabilir" warning badge gösterilir (örn. `.pst`, `.log`, `.sqlite` extension'larında)

**Linux/POSIX:** Shared read default davranıştır, özel flag gerekmez. Linux'ta gerçek sorun **dosya yazılırken inode value değişimi** (vim swap pattern) — bu durumda transfer'in başlangıçta açtığı handle stale'leşir. Mitigation: `stat()` + `inode_id` snapshot, transfer sonrası inode değiştiyse warning + retry.

---

## 13. Şifreleme Katmanı

### 13.1 XChaCha20-Poly1305 Varsayılan

| Kriter | XChaCha20-Poly1305 | AES-256-GCM |
|---|---|---|
| Nonce misuse toleransı | Yüksek (192-bit) | Düşük (96-bit reuse = felaket) |
| Paralel chunk şifreleme | Güvenli | Dikkatli nonce yönetimi şart |
| AES-NI olmayan cihaz | Hızlı | Yavaş |
| Desktop async multipart | İdeal | Riskli |

### 13.2 Nonce Yönetimi

```rust
fn generate_nonce(chunk_index: u64) -> XNonce {
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce[..16]);                // 16 byte random prefix (per-file)
    nonce[16..24].copy_from_slice(&chunk_index.to_le_bytes());  // 8 byte counter (per-chunk)
    XNonce::from(nonce)
}
// Garanti: aynı (file_random, chunk_index) çifti tekrar üretilmez
// 16 byte random → 2^128 file-level uniqueness, çakışma probabilistic olarak imkansız
// Counter suffix → aynı dosyanın chunk'ları arasında 100% deterministik unique
```

> **Not (v1.13 errata):** v1.12'de bu blok'ta `nonce[16+i] ^= counter[i]` XOR vardı. Üst 8 byte `[0u8; 24]` ile zaten sıfırlanmış olduğundan `0 XOR x == x` — yani XOR dead code'du. Davranış aynı, ama "neden XOR?" sorusu code review'da kafa karıştırıyor; doğrudan `copy_from_slice` ile niyet açık.

### 13.3 Algoritma Tablosu

| Kullanım | Algoritma | Crate |
|---|---|---|
| Dosya (varsayılan) | XChaCha20-Poly1305 | chacha20poly1305 |
| Dosya (alternatif) | AES-256-GCM | aes-gcm |
| KDF | Argon2id | argon2 |
| Checksum (güvenlik) | SHA-256 | sha2 |
| Checksum (hız) | XXH3-128 | xxhash-rust |
| HMAC | HMAC-SHA256 | ring |
| Token saklama | Windows Credential Manager | keyring |
| Updater imza | Ed25519 | ed25519-dalek |
| Audit DB | AES-256-GCM | aes-gcm |

### 13.4 Argon2id Parametreleri

OWASP 2024 önerilerine göre masaüstü hedefli, RFC 9106 second-preferred profili:

```rust
use argon2::{Argon2, Algorithm, Version, Params};

pub fn kdf_default() -> Argon2<'static> {
    let params = Params::new(
        19_456,   // memory_cost: 19 MiB (RFC 9106 m=19456 KiB)
        2,        // time_cost:   2 iteration
        1,        // parallelism: 1 lane (desktop, single-user)
        Some(32), // output_len:  32 byte (XChaCha20 / AES-256 key)
    ).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}
```

**Hedef performans:** Modern CPU'da ~250–500ms tek password derivation. Kullanıcı şifresi ile profil dışa aktarımı (`.dtransfer` dosyası) açılırken hissedilir ama cracking maliyetini ciddi yükseltir. Bu parametreler `crypto_config.json`'da versiyonlanır — gelecekte güçlendirilirse Config Migration ile mevcut profiller etkilenmeden kalır.

> **Not:** Argon2id parametreleri profil DB'sine yazılır. Profil bir cihazdan diğerine taşınınca aynı parametreler kullanılır — bu yüzden parametre sürümü `crypto_config.params_version` alanında saklanır.

**Salt Storage — Yapı Bütünlüğü (v1.15):**

Argon2id 16-byte salt **her şifrelenmiş artifact için ayrı** üretilir ve **artifact'in yanında saklanır** (Linux Secret Service / Windows DPAPI / macOS Keychain durumunda Argon2 kullanılmaz; OS bunu zaten kendi crypto'su ile hallediyor — Bölüm 25.1 backend matrix). Argon2 kullanılan üç senaryo:

| Senaryo | Salt nerede saklanır |
|---|---|
| **Encrypted file backend** (`~/.dtransfer/credentials.enc`, headless/Alpine/WSL2) | Dosya header'ında (Bölüm 25.1, 16 byte salt field) |
| **Profil export** (`.dtransfer` paylaşılabilir dosya, master password ile koruma) | Dosya header'ında, aynı format |
| **CryptoEngine encrypt_local_state = true** (client-side file encryption) | Encrypted chunk file'ın metadata block'unda, file_id ile birlikte |

**Salt format kuralları:**
- Her artifact için **bağımsız rastgele** salt (`OsRng.fill_bytes`)
- Asla hardcode değil, asla deterministik değil (rainbow table immunity)
- 16 byte (128-bit) — Argon2 spec minimum, OWASP 2024 önerisi
- Plaintext saklanır (salt gizli değil, sadece unique olmalı)

**Profil export (`.dtransfer`) format:**

Bölüm 25'de "AES-256-GCM şifreli `.dtransfer` dosyası" deniyordu. v1.15'te bu format açıkça spec'lenir — encrypted file backend ile aynı yapıyı kullanır:

```
+------------------+
| Magic: "DTEXP"   | 5 bytes
+------------------+
| Version: 1       | u8
+------------------+
| Argon2 salt      | 16 bytes (bu .dtransfer dosyasına özel, rastgele)
+------------------+
| Argon2 params    | m=46MB t=2 p=1 (export sıkı param, encrypted file ile aynı)
+------------------+
| AES-256-GCM nonce | 12 bytes
+------------------+
| Ciphertext       | JSON: {profile: ConnectionProfile, credentials: ...}
+------------------+
| GCM auth tag     | 16 bytes
+------------------+
```

İlk kez export edilen profile için kullanıcı master password sorulur. Aynı profile tekrar export edilirse yine sorulur — salt her seferinde yeni (rastgele). Aynı password + farklı salt = farklı encryption key, replay attack koruması.

**OS keychain backend'inde Argon2 yok:**

Windows DPAPI: işletim sistemi kullanıcı login credential'larından **kendi master key**'ini türetir; uygulama bir secret submit eder, DPAPI encrypted blob döner. Argon2 zincirinde değil. Aynı şey Linux Secret Service ve macOS Keychain için: OS kendi crypto layer'ını yönetir.

Yani salt yönetimi sadece **Argon2 kullanılan 3 senaryo** için relevant. Bunların hepsinin format'ında salt yer alıyor — spec eksiği değil ama v1.14'te ayrı bölümlerde dağınıktı, v1.15 ile bir yerde toplu.

### 13.5 Chunk + Auth Tag Boyut Yönetimi

XChaCha20-Poly1305 her chunk'a 16-byte authentication tag ekler. Multipart resume açısından netlik:

```
[Plaintext chunk]            → boyut: chunk_size (örn. 8 MiB)
[Encrypted chunk + tag]      → boyut: chunk_size + 16 byte
[Network frame]              → wire format: [nonce 24B][ciphertext + tag]
```

**Resume kuralı:** `.dtresume` dosyası **plaintext offset** tutar — encrypted offset değil. Yeniden başladığında engine plaintext offset → encrypted byte aralığını chunk_size + 16 üzerinden hesaplar. Aksi halde chunk_size değiştirildiğinde resume bozulur.

```rust
pub struct ResumeChunk {
    pub index:           u32,
    pub plaintext_start: u64,        // dosyada plaintext offset
    pub plaintext_size:  u32,        // genellikle chunk_size
    pub nonce:           [u8; 24],
    pub state:           ChunkState, // Pending | InFlight | Completed
}
```

Şifrelenmemiş transfer'da `plaintext_*` alanları doğrudan wire byte aralığına eşittir.

**Chunk Size Immutability (v1.13 errata):**

Kullanıcı **transfer ortasında** Settings'ten `chunk_size` değiştirirse şu üç sorun oluşur:

1. **Local resume bozulur** — `.dtresume` plaintext offset'i yeni chunk_size'la beklenen offset'e denk gelmez, ortalardaki bir chunk yeniden hesaplanır.
2. **S3 multipart upload geçersiz olur** — S3 her `UploadPart` ETag'ini chunk içeriği üzerinden hesaplar. Chunk boyutu değişirse upload finalize'da `CompleteMultipartUpload` ETag mismatch'le fail eder, tüm part'lar boşa gider.
3. **Per-chunk hash table tutarsız hale gelir** — `{file_id}.chunkmap` blob'unda chunk hash'leri eski boyuta göre hesaplanmış; doğrulama yanlış chunk üzerinde yapılır.

**Çözüm: chunk_size `.dtresume`'da kilitli immutable field.**

```rust
pub struct ResumeState {
    pub file_id:           Uuid,
    pub algorithm:         CryptoAlgorithm,    // immutable mid-transfer
    pub chunk_size:        u32,                // 🔒 immutable mid-transfer
    pub params_version:    u16,                // 🔒 immutable mid-transfer
    pub total_chunks:      u32,
    pub chunks:            Vec<ResumeChunk>,
    pub created_at:        DateTime<Utc>,
    pub last_updated_at:   DateTime<Utc>,
}

impl ResumeState {
    /// Settings'teki chunk_size mevcut `.dtresume` ile uyuşmuyorsa hata
    pub fn validate_against_settings(&self, settings: &TransferOptions) -> Result<(), TransferError> {
        if settings.chunk_size != self.chunk_size {
            return Err(TransferError::ResumeMismatch {
                field: "chunk_size",
                resume_value: self.chunk_size as u64,
                settings_value: settings.chunk_size as u64,
                hint: "Mid-transfer chunk_size değişimi desteklenmez. Yeni chunk_size için transfer'i Cancel edip yeniden başlatın.",
            });
        }
        if settings.crypto_algorithm != self.algorithm {
            return Err(TransferError::ResumeMismatch { /* ... */ });
        }
        Ok(())
    }
}
```

**UI davranışı:** Kullanıcı in-progress bir transfer varken `chunk_size` değiştirirse, Settings'te warning gösterilir: *"Yeni değer sadece yeni transfer'lere uygulanır. Devam eden 3 transfer eski değerle bitirilecek."*

**Yeni transfer ekleme:** Yeni queue task `chunk_size`'ı **task creation anında** snapshot'lar; daha sonra Settings değişse bile o task eski değerle bitirir. Bu queue.db'ye `chunk_size INTEGER NOT NULL` kolonu olarak yazılır.

**Resume Schema Versioning (v1.16):**

`.dtresume` dosyaları kalıcı artifact'lar — kullanıcı transfer'i pause edip haftalar sonra resume edebilir. Bu süre içinde DTransfer upgrade edilirse (yeni chunk algorithm, yeni AEAD, yeni hash function), eski resume dosyaları **deserialize edilemeyen legacy blob**'a dönüşebilir.

**Çözüm: explicit schema version header.**

```rust
/// .dtresume dosyasının ilk alanı — backward/forward compatibility kontrol
pub struct ResumeHeader {
    pub schema_version:    u16,                  // current: 1; her breaking change'de artar
    pub min_reader_version: u16,                 // bu dosyayı okumak için min schema_version (cap)
    pub chunking_strategy: ChunkingStrategy,     // FixedSize | ContentDefined (v2)
    pub crypto_suite:      CryptoSuite,          // None | XChaCha20Poly1305 | Aes256Gcm
    pub hash_algorithm:    HashAlgorithm,        // Sha256 | Xxh3_128 | Blake3 (v1.1+)
    pub kdf:               KdfAlgorithm,         // Argon2id (v1.0); future: Scrypt, Balloon
    pub created_at:        DateTime<Utc>,
}

pub struct ResumeFileV1 {
    pub header:  ResumeHeader,
    pub state:   ResumeState,                    // immutable fields: chunk_size, algorithm, params_version
}
```

**Compatibility kuralları:**

| Senaryo | Davranış |
|---|---|
| Reader v1, file schema_version=1 | ✅ Read & resume |
| Reader v2 (yeni), file schema_version=1 | ✅ Backward read (yeni reader eski formatı bilmek zorunda) |
| Reader v1 (eski), file schema_version=2 | ❌ Forward reject: "Bu transfer daha yeni bir DTransfer sürümüyle başlatıldı, app'i güncelleyin" |
| Reader v2, file `min_reader_version=2`, reader v1 | ❌ "Bu resume dosyası v2 reader gerektiriyor" |

**Migration politikası:**

- **Minor schema bump** (additive: yeni opsiyonel field): `schema_version` artmaz, `serde` default'larla absorbe edilir
- **Breaking schema bump** (field rename, enum variant remove, AEAD swap): `schema_version` +1, eski file'lar için **convert wizard** sunulur ya da transfer abort + warn
- **Crypto algorithm değişimi**: eski crypto suite'in destek penceresi (deprecation): 2 major release boyunca okunabilir, 3. release'te abort

**Konkre örnekler:**

```rust
// XChaCha20Poly1305 → Aes256Gcm migration (hypothetical v2.0)
match resume_header.crypto_suite {
    CryptoSuite::XChaCha20Poly1305 => {
        // v2.0'da hâlâ destekli (2 release deprecation window)
        // UI'da subtle "Bu transfer eski crypto suite'i kullanıyor" badge
    }
    CryptoSuite::Aes256Gcm => { /* current default */ }
    CryptoSuite::Future(name) => {
        return Err(TransferError::ResumeForwardIncompatible {
            crypto_suite: name,
            this_version: env!("CARGO_PKG_VERSION"),
        });
    }
}
```

**Partial compatibility:** Bazı durumlarda resume mümkün olmayabilir ama dosya o ana kadar inenler local'de — kullanıcıya iki seçenek:
1. *"Kaldığı yerden devam et (deneysel — yeni format'a migrate edilecek)"*
2. *"Sıfırdan başla (mevcut .partial dosyayı sil)"*

Crypto suite değişimi durumunda **#2 default**, sessiz migration tehlikeli (auth tag invalidate olur).

### 13.6 Secret Memory Zeroization

Argon2 ile derive edilen master key, plaintext password, OAuth token, presigned signature header — Rust `Drop` impl ile heap'te sıfırlanmazsa core dump'ta veya swap dosyasında plaintext kalır. Production-grade transfer client'ı için bu sızıntı senaryoları kabul edilemez.

**Crate'ler:** `zeroize` (memory clear), `secrecy` (compile-time `Debug`/`Display` engelleyici wrapper).

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};
use secrecy::{Secret, ExposeSecret, SecretString};

#[derive(ZeroizeOnDrop)]
pub struct MasterKey([u8; 32]);

impl MasterKey {
    pub fn derive(password: &SecretString, salt: &[u8]) -> Result<Self> {
        let mut key = [0u8; 32];
        Argon2::new(/* params */).hash_password_into(
            password.expose_secret().as_bytes(),
            salt,
            &mut key,
        )?;
        Ok(Self(key))
    }
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

// Drop otomatik: zeroize_on_drop trait
// Compiler garanti veriyor, manuel Drop impl yok
```

**Korunan alanlar:**

| Alan | Tipi | Tutulduğu yer |
|---|---|---|
| Plaintext password | `SecretString` | Profile editör modal (RAM'de yalnızca submit anına kadar) |
| Argon2 derived key | `Secret<[u8; 32]>` (ZeroizeOnDrop) | Master Key Cache (Bölüm 13.7), 30dk auto-lock |
| OAuth access token | `SecretString` | Profile credential store'da Secret wrapper içinde |
| OAuth refresh token | `SecretString` | Windows Credential Manager / Secret Service |
| Presigned URL (signature kısmı) | `SecretString` | Log masking (Bölüm 17.3) |
| TLS client private key | `Secret<Vec<u8>>` | mTLS profile'lerde |


**Yan kanal disiplini:**

```rust
// ❌ YANLIŞ — Debug/Display ile log'a sızar
let pwd = "secret123".to_string();
tracing::info!(password = ?pwd, "auth attempt");

// ✓ DOĞRU — secrecy compile error verir
let pwd: SecretString = "secret123".to_string().into();
tracing::info!(password = ?pwd, "auth attempt");
// → error: SecretString does not implement Debug
```

`secrecy::Secret<T>` bilinçli olarak `Debug`/`Display` implement etmez — yanlışlıkla `{:?}` ile log'a basmak compile error olur. Tracing macros tip kontrolünden geçer, runtime sızıntı yok.

**Page lock (opsiyonel, v1.1):** Master key'i swap'a yazılmaktan korumak için Windows `VirtualLock` / Linux `mlock`. v1.0'da gerekmez (sıradan kullanıcı düşman threat model'e değil), v1.1 enterprise mod için planlı.

### 13.7 Master Key Cache

```rust
pub struct MasterKeyCache {
    inner: Arc<RwLock<Option<CachedKey>>>,
    auto_lock_after: Duration,  // default: 30 dk
}

#[derive(ZeroizeOnDrop)]
struct CachedKey {
    key:        [u8; 32],
    derived_at: Instant,
}
```

30 dakika idle sonrası `inner` `None`'a set edilir, `CachedKey` drop ile zeroize. Kullanıcı password'ü tekrar girer.

### 13.8 Crypto Agility Contract (v1.16)

Şifreleme primitives (AEAD, KDF, hash) zaman içinde değişir — yeni saldırılar, NIST guidelines, post-quantum geçişler. v1.0 XChaCha20-Poly1305 + Argon2id + SHA-256 sabit set. Ama v1.5'te Argon2 parametreleri sıkılaşırsa? v2'de post-quantum AEAD geçişi gerekirse? Mevcut encrypted artifact'lar ne olur?

**Sözleşme:**

```rust
/// Crypto suite versiyonu — ResumeHeader, encrypted file header, audit DB içinde saklanır
pub struct CryptoSuiteVersion {
    pub suite_id:        CryptoSuiteId,    // enum, hardcoded variant list
    pub params_version:  u16,               // suite içinde parametre versiyonu
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptoSuiteId {
    /// v1.0 default
    XChaCha20Poly1305_Argon2id_SHA256 = 1,
    /// v1.0 alternative
    Aes256Gcm_Argon2id_SHA256        = 2,
    /// Future variants
    Future(u16),
}

#[derive(Debug, Clone)]
pub struct Argon2Params {
    pub memory_kib:  u32,    // params_version 1: 19456 (19MB); v2 hypothetical: 47104 (46MB)
    pub iterations:  u32,
    pub parallelism: u32,
}

pub const ARGON2_PARAMS_BY_VERSION: &[(u16, Argon2Params)] = &[
    (1, Argon2Params { memory_kib: 19456, iterations: 2, parallelism: 1 }),  // v1.0 OWASP 2024 second-preferred
    // (2, Argon2Params { memory_kib: 47104, iterations: 1, parallelism: 1 }),  // v1.5 hypothetical
];
```

**Migration matrix:**

| Senaryo | Davranış |
|---|---|
| Eski artifact, suite_id known, params_version known, decrypt başarılı | ✅ Decrypt with stored params, kullanıcıya silent migrate önerisi (yeni encrypt'te güncel params) |
| Eski artifact, suite_id known, params_version unknown | ⚠️ Warn + decrypt deneme (forward-compatible params struct) |
| Eski artifact, suite_id deprecated (örn. v2.0'da CryptoSuiteId::Aes256Gcm_... legacy) | ⚠️ Decrypt OK + UI badge "Bu artifact eski crypto suite kullanıyor, yeniden şifrelemek için Settings → Re-encrypt" |
| Eski artifact, suite_id removed (örn. v3.0'da kaldırılmış) | ❌ Error: "Bu artifact için DTransfer v2.x'i kullanın ve manuel re-encrypt yapın" |
| Yeni artifact, suite_id unknown (DTransfer eski, artifact yeni) | ❌ Forward-incompatible error: "Bu artifact DTransfer v{X}+ gerektiriyor" |

**Deprecation timeline (kontrat):**

Bir crypto suite resmi olarak deprecate olduğu release'ten itibaren:
- **2 major release** boyunca: decrypt + re-encrypt destekli, UI'da subtle badge
- **3. major release'te**: decrypt-only mode (re-encrypt artık yapılamaz, sadece okuma için)
- **4. major release'te**: tamamen kaldırılır, kullanıcıya eski sürüm referansı

Bu pattern user data lock-in'i engeller — kullanıcı her zaman 2+ release süreci boyunca eski crypto'ya erişebilir, migrate edebilir.

**Re-encrypt workflow:**

```rust
// Background task (idle CPU'da çalışır, transfer'leri bloklamaz)
pub async fn migrate_legacy_crypto(
    db: &Db,
    target_suite: CryptoSuiteVersion,
    cancel: CancellationToken,
) -> Result<MigrationReport> {
    let legacy_artifacts = db.list_artifacts_with_old_suite(target_suite.suite_id).await?;

    let mut report = MigrationReport::default();
    for artifact in legacy_artifacts {
        if cancel.is_cancelled() { return Err(MigrationError::Cancelled); }

        // 1. Decrypt with old suite
        let plaintext = decrypt_with_legacy(&artifact, &master_key).await?;
        // 2. Encrypt with new suite
        let new_artifact = encrypt_with_target(&plaintext, &master_key, target_suite).await?;
        // 3. Atomic replace (tmp → rename)
        atomic_replace(&artifact.path, &new_artifact.path).await?;
        // 4. Update DB metadata
        db.update_artifact_suite(artifact.id, target_suite).await?;

        report.migrated += 1;
    }
    Ok(report)
}
```

**KDF parameter migration:**

Argon2 parametreleri sıkılaşırsa (v1.5'te memory_kib 19456 → 47104), eski şifrelenmiş artifact'lar **mevcut params ile decrypt edilir** (params artifact içinde saklı). Yeniden encrypt'te yeni params kullanılır.

Bu sayede:
- Eski artifact'lar **bozulmaz** — her zaman decrypt edilebilir
- Yeni artifact'lar **güçlü params** alır
- Migration **lazy** — kullanıcı dosyaya dokunduğunda re-encrypt, bekleyenler beklemeye devam eder

**Implementation contract:**

Her encrypt operation `CryptoSuiteVersion` field'ını artifact metadata'sına yazar (zorunlu). Decrypt operation **önce metadata'yı okur**, sonra uygun suite ile decrypt yapar. Asla "current default ile decrypt et" assumption'ı yapılmaz — bu lock-in yaratır.

---

## 14. Çok Parçalı Transfer (Multipart)

> **Resume sözleşmesi (Bölüm 5.2):** Resume support **backend capability**'sine bağlıdır (Bölüm 5.3 matrix). Sunucu Range write veya multipart part PUT desteklemiyorsa, resume **mümkün değildir** ve transfer baştan başlatılır. Ayrıca remote object'in transfer süresince **immutable** olduğu varsayılır; remote tarafında üçüncü taraf değişiklik DTransfer tarafından detect edilemez.

### 14.1 Protokol Matrisi

| Protokol | İndirme | Yükleme | Mekanizma |
|---|---|---|---|
| HTTP / WebDAV | ✓ | ⚠ | `Range: bytes=N-M` |

| SFTP | ✓ | ✓ | Paralel SSH channel (CapabilityProfile sınırlı) |
| S3 / R2 / B2 | ✓ | ✓ Native | `GetObject` range / `CreateMultipartUpload` |

### 14.2 Atomic Write

```rust
// Chunk birleştirme
let tmp_out = local.with_extension("dtransfer_tmp");
assemble_chunks(&tmp_dir, &tmp_out, &chunks).await?;
atomic_rename_with_av_retry(&tmp_out, local).await?;  // AV-aware

// Resume durum dosyası
async fn save_resume_state(state: &ResumeState, path: &Path) -> Result<()> {
    let tmp = path.with_extension("dtresume_tmp");
    tokio::fs::write(&tmp, serde_json::to_vec(state)?).await?;
    atomic_rename_with_av_retry(&tmp, path).await?;
    Ok(())
}
```

**AV Lock Micro-Retry (Windows-specific):**

Atomic rename teorik olarak tek atomic syscall'dur. Ama Windows'ta tam o milisaniyede:
1. DTransfer dosyayı `_tmp` olarak yazıp kapatır
2. Windows Defender (veya 3rd party AV) "yeni dosya yazıldı" event'ini yakalar
3. AV dosyaya **exclusive read lock** koyar, taramaya başlar (~200-500ms)
4. DTransfer `MoveFileEx` çağırır → `ERROR_ACCESS_DENIED` (5) veya `ERROR_SHARING_VIOLATION` (32)
5. Transfer "Failed" durumuna düşer — halbuki dosya %100 inmiş

Çözüm: rename özelinde, **transfer'in genel retry sayacından bağımsız** çok kısa aralıklı micro-retry. AV taraması genelde 1 saniyenin altında biter.

```rust
async fn atomic_rename_with_av_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    const RETRY_DELAYS_MS: &[u64] = &[50, 150, 300, 600, 1000];

    for (attempt, delay_ms) in std::iter::once(&0).chain(RETRY_DELAYS_MS).enumerate() {
        if *delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(*delay_ms)).await;
        }
        match tokio::fs::rename(from, to).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                #[cfg(windows)]
                {
                    let raw = e.raw_os_error().unwrap_or(0);
                    // 5 = ACCESS_DENIED, 32 = SHARING_VIOLATION
                    if raw == 5 || raw == 32 {
                        tracing::debug!(attempt, delay_ms, "rename blocked, micro-retry");
                        continue;
                    }
                }
                return Err(e);  // Diğer hatalar genel retry'a bırak
            }
        }
    }
    // Toplam ~2.1sn sonra hâlâ kilitli — gerçek sorun
    Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "atomic rename: file locked by external process for >2s (AV scan stuck?)",
    ))
}
```

**Toplam bekleme:** 50+150+300+600+1000 = 2.1sn worst case. Kullanıcı 2sn'lik takılma fark eder ama "Failed" görmesi yerine başarı görür. AV tarama 2sn'den uzun sürerse zaten gerçek bir sorun var (büyük dosyada deep scan), kullanıcıya bildir.

**UX Sub-state Indicator (v1.14):**

Kullanıcı progress %100 görür, sonra 2sn nothing, sonra "Completed". Bu sırada "donmuş mu?" sorusu sorulur. Çözüm: `TransferState::Active` alt state'ine granularity:

```rust
pub enum TransferActiveSubState {
    Connecting,
    Transferring { speed_bps: u64 },
    Verifying,                                  // checksum hesaplanıyor
    Finalizing,                                 // AV scan / atomic rename bekleniyor (<10sn)
    WaitingForAntivirus { elapsed_secs: u64 },  // v1.15: uzun AV taraması (>10sn)
    CleanupPending,                             // tmp dosyalar temizleniyor
}

pub enum EngineEvent {
    // ... existing variants ...
    TransferSubStateChanged { transfer_id: Uuid, sub_state: TransferActiveSubState },
}
```

UI'da gösterim:
- "Aktarılıyor (45.2 MB/s)" → durum çubuğunda hız
- "Doğrulanıyor..." → checksum sırasında, spinner
- "Antivirüs taraması bekleniyor..." → AV retry sırasında, info ikon + tooltip *"Windows Defender dosyayı tarıyor — birkaç saniye sürebilir"*
- "Antivirüs hâlâ tarıyor (45sn)..." → uzun tarama, "Manuel İptal" butonu yanında

**AV Retry File-Size Scaling (v1.15):**

v1.14'teki sabit `[50, 150, 300, 600, 1000]ms` toplam ~2.1sn timeout, 50GB video/database backup gibi büyük dosyalarda yetersiz. Windows Defender büyük dosyayı tararken I/O hızına bağlı dakikalar harcayabilir, ama 2.1sn sonra `Failed` durumu = yanlış hata raporu.

**Çözüm: iki katmanlı retry chain.**

```rust
async fn atomic_rename_with_av_retry(
    from: &Path,
    to: &Path,
    file_size_bytes: u64,
    event_bus: &EventBus,
    transfer_id: Uuid,
    cancel: CancellationToken,
) -> std::io::Result<()> {
    // Katman 1: micro-retry (kısa AV scan)
    const MICRO_RETRY_MS: &[u64] = &[50, 150, 300, 600, 1000];

    for (attempt, delay_ms) in std::iter::once(&0).chain(MICRO_RETRY_MS).enumerate() {
        if *delay_ms > 0 {
            if attempt == 2 {
                event_bus.emit(EngineEvent::TransferSubStateChanged {
                    transfer_id,
                    sub_state: TransferActiveSubState::Finalizing,
                });
            }
            tokio::time::sleep(Duration::from_millis(*delay_ms)).await;
        }
        match try_rename_once(from, to).await {
            RenameResult::Ok => return Ok(()),
            RenameResult::AvLocked => continue,
            RenameResult::Other(e) => return Err(e),
        }
    }

    // Katman 2: macro-retry (uzun AV scan, file-size scaled)
    // Heuristic: dosya boyutu başına ~10sn ek tolerans (10GB = +100sn)
    let max_wait_secs = 30 + (file_size_bytes / (1024 * 1024 * 1024)) * 10;
    let max_wait_secs = max_wait_secs.min(600);  // hard cap: 10dk

    event_bus.emit(EngineEvent::TransferSubStateChanged {
        transfer_id,
        sub_state: TransferActiveSubState::WaitingForAntivirus { elapsed_secs: 0 },
    });

    let start = Instant::now();
    let mut backoff_ms = 1000;

    loop {
        let elapsed = start.elapsed().as_secs();
        if elapsed > max_wait_secs {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("atomic rename: AV lock persisted for {}s (file: {} bytes); manual retry required",
                        elapsed, file_size_bytes),
            ));
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
            _ = cancel.cancelled() => return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted, "cancelled while waiting for AV scan"
            )),
        }

        match try_rename_once(from, to).await {
            RenameResult::Ok => return Ok(()),
            RenameResult::AvLocked => {
                // UI'a elapsed güncellemesi (her 5sn'de bir)
                if elapsed % 5 == 0 {
                    event_bus.emit(EngineEvent::TransferSubStateChanged {
                        transfer_id,
                        sub_state: TransferActiveSubState::WaitingForAntivirus { elapsed_secs: elapsed },
                    });
                }
                backoff_ms = (backoff_ms * 3 / 2).min(10_000);  // exponential, cap 10sn
            }
            RenameResult::Other(e) => return Err(e),
        }
    }
}

enum RenameResult {
    Ok,
    AvLocked,                  // ERROR_ACCESS_DENIED (5) / ERROR_SHARING_VIOLATION (32)
    Other(std::io::Error),
}

async fn try_rename_once(from: &Path, to: &Path) -> RenameResult {
    match tokio::fs::rename(from, to).await {
        Ok(()) => RenameResult::Ok,
        Err(e) => {
            #[cfg(windows)]
            {
                let raw = e.raw_os_error().unwrap_or(0);
                if raw == 5 || raw == 32 {
                    return RenameResult::AvLocked;
                }
            }
            RenameResult::Other(e)
        }
    }
}
```

**File-size hesaplama:**

| Dosya boyutu | Max bekleme |
|---|---|
| <1 GB | 30sn |
| 5 GB | 80sn |
| 10 GB | 130sn |
| 50 GB | 10dk (hard cap) |
| 100 GB+ | 10dk (hard cap) |

**Hard cap 10dk** — bunun ötesi gerçek bug (Defender stuck, exclusion gerekli, custom AV imkansız davranış). Kullanıcıya ayrıca "Bu dosya antivirüsünüzde exclusion gerektiriyor olabilir" hint gösterilir.

**Cancel desteği:** Uzun bekleme sırasında kullanıcı transfer'i iptal edebilir; `CancellationToken` ile `tokio::select!` sayesinde sleep anında abort eder.

**Future: ReadDirectoryChangesW polling alternatifi (v1.1).** Windows API ile dizin değişimi dinleme, polling yerine event-driven retry — ama platform-specific, v1.0 scope dışı.

**Linux/POSIX:** Bu sorun yok — `rename()` syscall AV scanning interpose etmiyor (Linux'ta AV genelde kernel module + farklı timing model).

**Test:** Windows Defender'ı test ortamında aktif tut, 100MB+ dosya transfer ettir, attempt count log'la. CI'da Windows runner'ı bu davranışı doğrular.

### 14.3 Memory Backpressure

```rust
pub struct MultipartConfig {
    pub max_buffered_chunks: usize,  // default: parallel_streams × 2
    // Bu sayıya ulaşınca yeni chunk başlatılmaz
}
```

### 14.4 Per-Chunk Hash (Partial Corruption Recovery)

10 GB dosyada tek bit corruption tüm dosyayı yeniden indirtmek 2026 standartlarında kabul edilemez. Her chunk için ayrı hash + dosya seviyesi hash birlikte tutulur. Corruption tespit edildiğinde yalnızca etkilenen chunk(lar) yeniden indirilir.

```rust
pub struct ChunkManifest {
    pub file_id:        Uuid,
    pub chunk_size:     u32,
    pub total_chunks:   u32,
    pub algorithm:      ChecksumAlg,   // Sha256 (güvenlik) | Xxh3_128 (hız)
    pub file_hash:      [u8; 32],      // tüm dosya
    // chunk hash'leri ayrı blob table'da, lazy-load
}

pub enum ChecksumAlg { Sha256, Xxh3_128 }
```

**Storage modeli (binary blob file, SQLite ayrımı):**

ChatGPT'nin son turunda işaret ettiği nüans: chunk hash'leri SQLite'ta tutarsak — küçük table bile olsa — WAL şişer, VACUUM ağırlaşır, checkpoint latency oluşur. 500k transfer × 2560 chunk = milyar mertebesi row. Çözüm: **SQLite sadece manifest var/yok bayrağını tutar**, hash'lerin kendisi binary blob dosyasına yazılır.

```
~/.dtransfer/queue/
├── queue.db                    # SQLite (transfer state, queue meta)
└── chunkmaps/
    ├── {file_id_1}.chunkmap    # binary blob: header + hash array
    ├── {file_id_2}.chunkmap
    └── ...
```

```sql
-- queue.db
CREATE TABLE chunk_manifests (
    file_id        TEXT PRIMARY KEY,
    chunk_size     INTEGER NOT NULL,
    total          INTEGER NOT NULL,
    algorithm      TEXT NOT NULL,
    file_hash      BLOB NOT NULL,
    chunkmap_path  TEXT NOT NULL,    -- relative path to .chunkmap blob
    created_at     INTEGER NOT NULL
);
-- Hash'lerin kendisi SQLite'ta YOK — sadece blob file path
```

**Binary blob format (`{file_id}.chunkmap`):**

```
+------------------+
| Magic (4 bytes)  |  "DTCM" (DTransfer ChunkMap)
+------------------+
| Version (u8)     |  1
+------------------+
| Algorithm (u8)   |  0=Sha256, 1=Xxh3_128
+------------------+
| Hash size (u8)   |  32 (SHA-256) or 16 (XXH3-128)
+------------------+
| Chunk count (u32)|  little-endian
+------------------+
| Chunk hashes     |  N × hash_size bytes, contiguous
+------------------+
| CRC32 (u32)      |  blob integrity
+------------------+
```

**Avantajlar:**
- SQLite'a hiç dokunmaz — UI listesi sorgusu blob'a değmez
- mmap ile zero-copy okuma mümkün (`memmap2` crate)
- Lazy load: sadece corruption recovery anında okunur
- VACUUM SQLite üzerinde rahat, blob dosyaları transfer tamamlanınca silinir
- Cross-platform sorunsuz (endian-aware format)

```rust
pub struct ChunkMapBlob {
    mmap: Mmap,           // zero-copy access
    header: ChunkMapHeader,
}

impl ChunkMapBlob {
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let header = ChunkMapHeader::parse(&mmap[..16])?;
        // CRC verify lazy — sadece corruption şüphesinde
        Ok(Self { mmap, header })
    }

    pub fn hash_at(&self, idx: u32) -> Option<&[u8]> {
        let offset = 16 + (idx as usize) * (self.header.hash_size as usize);
        let end = offset + (self.header.hash_size as usize);
        if end > self.mmap.len() - 4 { return None; }  // -4 for CRC tail
        Some(&self.mmap[offset..end])
    }
}
```

**Atomic write:** Yeni `.chunkmap` blob `_temp` suffix ile yazılır, fsync, rename — partial write krizi olmaz.

**Corruption recovery akışı:**

```rust
pub async fn verify_and_recover(
    file: &Path,
    manifest: &ChunkManifest,
    db: &DbHandle,
) -> Result<Vec<u32>> {  // dönen: yeniden indirilecek chunk index'leri
    // 1. Hızlı yol: tüm dosya hash (XXH3 ile)
    let file_hash = hash_file_xxh3(file).await?;
    if file_hash == manifest.file_hash { return Ok(vec![]); }  // sağlam

    // 2. Per-chunk verify (lazy load, sadece corruption durumunda)
    let chunks = db.load_chunk_hashes(manifest.file_id).await?;
    let mut bad = Vec::new();
    for (idx, expected) in chunks.iter().enumerate() {
        let actual = hash_chunk_at(file, idx as u32, manifest.chunk_size).await?;
        if &actual != expected { bad.push(idx as u32); }
    }
    Ok(bad)
}
```

**Hash hesaplama maliyeti:** SHA-256 ~80MB/s tek thread. 10GB dosya = 125 saniye. CPU-bound, `spawn_blocking` zorunlu (Bölüm 9.5). Background verify olarak transfer sonrası 2 saat içinde scheduled — kullanıcıyı bloklamaz.

### 14.5 Sparse File Desteği

VM image (qcow2/vmdk), database snapshot, Docker layer — sparse dosyalar yaygın. Naive byte-by-byte transfer 100GB sparse dosyayı 100GB net wire byte gönderir, halbuki gerçek veri 10GB olabilir. **Sadece Local FS ve SFTP** sparse'i destekler.

```rust
pub struct SparseDetection {
    /// Sparse blok eşik: bu kadar byte sıfır → sparse hole olarak transfer
    pub zero_run_threshold: usize,   // default: 64 KiB (FS block size)
    pub preserve_holes:     bool,    // default: true (Local + SFTP)
}

pub enum SparseSupport {
    /// Tam destek: SFTP @openssh sparse extension veya Local seek
    Native,
    /// Yok: object store, WebDAV — sparse dosya dense olarak kopyalanır
    None,
}
```

**SFTP openssh extension probe:** İlk bağlantıda `posix-rename@openssh.com` ve sparse extension probe edilir, `CapabilityProfile.supports_sparse` set edilir.

**Linux'ta detection:** `SEEK_HOLE` / `SEEK_DATA` fcntl. Windows'ta `FSCTL_QUERY_ALLOCATED_RANGES`. Hole detection sırasında `tokio::fs` kullanılır (zaten blocking pool).

**S3 / object store:** Sparse desteği yok. Wire byte = file size (hole'ları sıfır olarak gönderir). Kullanıcıya "S3 sparse desteklemiyor, 100GB veri gönderilecek" uyarısı modal'da gösterilir.

### 14.6 fsync Politikası

Atomic rename yetmez. Linux ext4'te delayed allocation, NTFS'te write cache — rename sonrası power loss'ta dosya boş çıkabilir. POSIX'te tam durability için **file fsync + parent directory fsync** ikilisi şart.

```rust
pub enum SyncPolicy {
    /// fsync yapma — hızlı ama power loss'ta kayıp olabilir
    None,
    /// File fsync (default), parent dir fsync yok — pratik denge
    DataOnly,
    /// File fsync + parent directory fsync (POSIX full durability)
    Full,
}
```

**Default: `DataOnly`** — kullanıcıların %95'i throughput ister, full fsync 80 MB/s'i 30 MB/s'e düşürebilir (NVMe + Windows'ta sync cost).

**Full mode davranışı (önemli detay):**

```rust
async fn write_with_full_sync(path: &Path, data: &[u8]) -> Result<()> {
    let temp = path.with_extension("tmp");
    let mut f = tokio::fs::File::create(&temp).await?;
    f.write_all(data).await?;
    f.sync_all().await?;            // 1) file content + metadata durable
    drop(f);
    tokio::fs::rename(&temp, path).await?;
    // 2) Parent directory fsync — rename'in durable olması için ŞART
    if let Some(parent) = path.parent() {
        let dir = tokio::fs::File::open(parent).await?;
        dir.sync_all().await?;
        // Linux: ext4 delayed allocation directory entry'si flush edilir
        // Windows: NTFS rename zaten durable (NTFS journal), bu çağrı no-op gibi
    }
    Ok(())
}
```

**Parent dir fsync subtle kuralı:** Rename atomic ama directory entry yazımı async olabilir. Power loss anında dosya içeriği var, ama parent dizin "yeni isim eski isim olarak göstermeye devam edebilir" — yani rename geri alınmış gibi görünür. Çoğu dev bunu bilmez. POSIX standardı için zorunlu.

**Konfigürasyon:** Profile bazında `sync_policy` field'ı. UI'da advanced settings altında, default DataOnly. Power user/enterprise compliance için Full mode opt-in.

---

## 15. Transfer Queue ve Scheduler

> **Garanti sözleşmesi (Bölüm 5.1):** Queue state, beklenmedik process termination veya power loss durumlarında **defined persistence boundaries** içinde korunur. Sınır: son 5 saniyelik batch checkpoint'ten sonraki `bytes_done` artışları kaybedilebilir, ama task identity, state ve resume metadata her zaman korunur.

### 15.1 Queue Persistence

Transfer kuyruğu runtime'da değil, `queue.db` SQLite WAL'da yaşar. Uygulama restart, OS reboot, crash sonrası kuyruk devam eder.

```rust
pub struct PersistedTransferTask {
    pub id:                Uuid,
    pub state:             TransferState,
    pub priority:          u8,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
    pub retries_done:      u8,
    pub source:            TransferEndpoint,
    pub destination:       TransferEndpoint,
    pub resume_state_path: Option<PathBuf>,
    pub error_last:        Option<String>,
}

pub enum TransferState {
    Queued, Active, Paused, Completed, Failed, Cancelled,
}

impl TransferState {
    /// Geçersiz transition'ları önler — race condition ve double retry koruması
    pub fn can_transition_to(&self, next: &TransferState) -> bool {
        match (self, next) {
            (Self::Queued,    Self::Active)    => true,
            (Self::Queued,    Self::Cancelled) => true,
            (Self::Active,    Self::Paused)    => true,
            (Self::Active,    Self::Completed) => true,
            (Self::Active,    Self::Failed)    => true,
            (Self::Active,    Self::Cancelled) => true,
            (Self::Paused,    Self::Active)    => true,
            (Self::Paused,    Self::Cancelled) => true,
            (Self::Failed,    Self::Queued)    => true,  // retry
            // Bunlar yasak:
            // Completed → Active, Failed → Active, Cancelled → Active
            _ => false,
        }
    }
}

pub struct TransferEndpoint {
    pub profile_id: Uuid,
    pub path:       String,
}
```

### 15.2 Queue DB Şeması

```sql
-- queue.db (WAL mode)
CREATE TABLE transfer_tasks (
    id                TEXT PRIMARY KEY,
    state             TEXT NOT NULL,
    priority          INTEGER DEFAULT 0,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    retries_done      INTEGER DEFAULT 0,
    source_profile_id TEXT NOT NULL,
    source_path       BLOB NOT NULL,         -- raw bytes; Invalid UTF-8 safe (Bölüm 12.7)
    source_path_kind  TEXT NOT NULL,         -- 'utf8' | 'raw_bytes'
    dest_profile_id   TEXT NOT NULL,
    dest_path         BLOB NOT NULL,         -- raw bytes
    dest_path_kind    TEXT NOT NULL,
    resume_state_path BLOB,                  -- local FS path; raw bytes
    error_last        TEXT,
    bytes_total       INTEGER,
    bytes_done        INTEGER DEFAULT 0,
    chunk_size        INTEGER NOT NULL       -- v1.13 errata: snapshot at task creation
);

CREATE INDEX idx_state ON transfer_tasks(state);
CREATE INDEX idx_priority ON transfer_tasks(priority DESC, created_at ASC);
```

**Path Sütunları BLOB Olarak (v1.15):**

v1.14'e kadar şema'da path sütunları `TEXT NOT NULL` idi. Bu Bölüm 12.7 ile **schema inconsistency** yaratıyordu: Linux'ta geçersiz UTF-8 dosya adları için `PathTransport::RawBytes` enum desteklendiği halde, veritabanı kolonu UTF-8 zorlardı (rusqlite `String` ↔ TEXT conversion noktasında validation hatası fırlatır, panic potansiyeli).

**Düzeltme:**
- Tüm path sütunları **`BLOB`** olarak tanımlanır (raw bytes preserved)
- Yan kolon `*_path_kind TEXT NOT NULL` → `'utf8'` veya `'raw_bytes'` (parser hint)
- UI gösterimi: `utf8` ise direkt render, `raw_bytes` ise hex notation (`\xc3\xa9...`) + "Geçersiz UTF-8 dosya adı" badge

```rust
pub fn store_path_to_db(path: &PathTransport, kind_col: &str, blob_col: &str) -> SqlBind {
    match path {
        PathTransport::Utf8(s) => {
            ("utf8".to_string(), s.as_bytes().to_vec())
        }
        PathTransport::RawBytes(bytes) => {
            ("raw_bytes".to_string(), bytes.clone())
        }
    }
}

pub fn load_path_from_db(kind: &str, blob: Vec<u8>) -> PathTransport {
    match kind {
        "utf8" => PathTransport::Utf8(String::from_utf8(blob).expect("schema invariant")),
        "raw_bytes" => PathTransport::RawBytes(blob),
        _ => unreachable!("unknown path_kind in db"),
    }
}
```

Aynı düzeltme `view_cache.db` (Bölüm 22.8) ve `audit.db` (Bölüm 17) için de uygulanır — herhangi bir filename/path saklayan kolon BLOB + kind ayrımı kullanır.

### 15.3 QueueScheduler

v1.0: FIFO + priority. v2 altyapısı şimdiden rezerve:

```rust
pub enum SchedulingPolicy {
    Fifo,                    // v1.0 — varsayılan
    PriorityFifo,            // v1.0 — priority alanı ile
    WeightedFair,            // v2 stub
    HostAware,               // v2 stub — aynı host'a concurrency limiti
    SmallFilesFirst,         // v2 stub
}

pub struct QueueScheduler {
    policy:           SchedulingPolicy,
    host_limiter:     HashMap<String, u8>,  // host → max concurrent (v2)
    max_concurrent:   u8,                   // global limit
}
```

**`bytes_done` Yazma Sıklığı:** 16 paralel transferin her 500ms'de `bytes_done` yazması disk I/O'sunu felç eder. Tıpkı Audit batch write gibi:

- **UI progress** → Pinia store (anlık, ProgressAggregator 250ms)
- **queue.db `bytes_done`** → 5 saniyede bir veya chunk tamamlandığında batch update

**Trade-off (kabul edilen davranış):** Crash anında en fazla 5 saniyelik `bytes_done` ilerlemesi DB'ye yazılmamış olabilir. Bu durumda restart sonrası transfer son DB değerinden devam eder — bu da en kötü durumda son 5 saniyelik veriyi yeniden gönderir. Multipart resume kayıt birimi *chunk* olduğundan (`.dtresume` her chunk tamamlanınca atomic flush edilir), gerçek kayıp daima son tamamlanmamış chunk seviyesinde kalır. **DB byte sayacı sadece UI/queue restore içindir, transfer doğruluğunu chunk tablosu garanti eder.** Trade-off bilinçli: per-chunk DB write 16+ paralel transferde WAL fsync amplification'a yol açar.

```rust
// QueueProgressWriter: bytes_done'ı direkt DB'ye yazmaz
pub struct QueueProgressWriter {
    pending:  HashMap<Uuid, u64>,    // transfer_id → bytes_done
    interval: tokio::time::Interval, // 5s
}

impl QueueProgressWriter {
    pub fn update(&mut self, id: Uuid, bytes: u64) {
        self.pending.insert(id, bytes);
    }
    pub async fn flush(&mut self, db: &QueueDb) {
        // Tek transaction ile hepsini yaz
        db.batch_update_bytes_done(&self.pending).await.ok();
        self.pending.clear();
    }
    /// Chunk tamamlanınca tetiklenir — son chunk'ın doğru sayılması için
    /// flush'i beklemeden ilgili transfer'i hemen yaz.
    pub async fn flush_now(&mut self, id: Uuid, db: &QueueDb) {
        if let Some(bytes) = self.pending.remove(&id) {
            db.update_bytes_done(id, bytes).await.ok();
        }
    }
}
```

### 15.4 DbActor Pattern — Yazma Serileştirmesi

Yukarıdaki `QueueProgressWriter::flush_now` 16 paralel transfer aynı anda chunk tamamlandığında 16 concurrent DB write tetikler. tokio-rusqlite kendi internal queue'sundan geçirir ama uygulama seviyesinde yine race oluşur — özellikle WAL checkpoint anlarında `SQLITE_BUSY` riski. Buna ek olarak transfer state transition'ları (`Running → Completed`), task insert/delete, audit log batch'i de aynı DB'ye yazıyor. Concurrency = belirsiz davranış.

**Çözüm:** Tüm yazma operasyonlarını tek bir owned DbActor üzerinden serileştir. Producer'lar mpsc kanalına command yollar, actor sırayla işler.

```rust
pub enum DbCommand {
    BatchProgress { entries: HashMap<Uuid, u64> },
    StateTransition { id: Uuid, from: TransferState, to: TransferState, ack: oneshot::Sender<Result<(), DbError>> },
    InsertTask { task: Box<PersistedTransferTask>, ack: oneshot::Sender<Result<(), DbError>> },
    DeleteTask { id: Uuid, ack: oneshot::Sender<Result<(), DbError>> },
    AuditBatch { entries: Vec<AuditEntry> },
    Checkpoint { ack: oneshot::Sender<Result<(), DbError>> },
    Shutdown { ack: oneshot::Sender<()> },
}

pub struct DbActor {
    rx: mpsc::Receiver<DbCommand>,
    db: QueueDb,
}

impl DbActor {
    pub fn spawn(db: QueueDb) -> mpsc::Sender<DbCommand> {
        let (tx, rx) = mpsc::channel(1024);  // 16 transfer × 64 chunk burst yeter
        tokio::spawn(async move {
            let mut actor = DbActor { rx, db };
            actor.run().await;
        });
        tx
    }

    async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            // Kritik: sırayla işlem, asla concurrent
            match cmd {
                DbCommand::BatchProgress { entries } => {
                    let _ = self.db.batch_update_bytes_done(&entries).await;
                }
                DbCommand::StateTransition { id, from, to, ack } => {
                    let result = self.db.transition_state(id, from, to).await;
                    let _ = ack.send(result);  // caller bekliyorsa ack
                }
                DbCommand::InsertTask { task, ack } => {
                    let result = self.db.insert(*task).await;
                    let _ = ack.send(result);
                }
                DbCommand::Shutdown { ack } => {
                    self.db.checkpoint().await.ok();  // graceful close
                    let _ = ack.send(());
                    return;
                }
                // ... diğer komutlar
            }
        }
    }
}
```

**Producer tarafı (caller):**

```rust
// Fire-and-forget (progress)
db_tx.send(DbCommand::BatchProgress { entries }).await.ok();

// Confirmed (state transition — caller başarıyı bilmeli)
let (ack_tx, ack_rx) = oneshot::channel();
db_tx.send(DbCommand::StateTransition {
    id: transfer_id,
    from: TransferState::Running,
    to: TransferState::Completed,
    ack: ack_tx,
}).await?;
let result = ack_rx.await?;  // Result<(), DbError>
```

**Avantajlar:**
- **Sıfır lock contention** — actor tek tüketici, mutex/RwLock yok
- **SQLITE_BUSY riski sıfır** — yazmalar serial
- **Backpressure built-in** — kanal dolarsa producer'lar `await`'te bekler, runaway yok
- **Test edilebilir** — actor'ı mock'la, command sequence'i assertion'la
- **Graceful shutdown** — `Shutdown` komutu ile checkpoint + close
- **Panic isolation** — actor task panic ederse Tokio'nun JoinHandle'ı yakalar, supervisor restart eder (queue.db açık, WAL durumda kalır)

**Trade-off:** Read-only sorgular (kuyruk listele, geçmiş audit görüntüle) actor'dan geçmez — `QueueDb::list()` direkt SQLite read connection açar (WAL mode reader'ı writer'ı bloklamaz). Sadece **yazma** serileştirilir.

**Kritik:** `db_tx` bir `Arc<...>` değil `Sender<DbCommand>` — clone ucuz (Arc içeride), `Send + Sync`, 16 transfer worker'ına dağıtılır.

```rust
// Engine init
let db = QueueDb::open(&path).await?;
let db_tx = DbActor::spawn(db);  // Sender<DbCommand>

// Her transfer worker bu sender'ın klonunu alır
for task in initial_tasks {
    let tx = db_tx.clone();
    tokio::spawn(async move { run_transfer(task, tx).await });
}
```

### 15.5 Directory Traversal Streaming ("node_modules" Problemi)

100,000 küçük dosyalı klasör (`.git`, `node_modules`, fotoğraf koleksiyonu) FileZilla'yı dakikalarca dondurur — önce tüm ağacı tarar, sonra tek tek STOR komutu yollar. DTransfer bu pattern'i kökten reddeder.

**İlke:** Klasör tarama asla bloklayıcı olmamalı. İlk dosya bulunur bulunmaz scheduler beslenmeye başlar.

```rust
pub struct DirectoryStream {
    /// Bulunan dosyaları stream olarak yayınlar
    tx: mpsc::Sender<DiscoveredFile>,
    /// Worker thread tarama yapar (CPU-bound: stat() syscall'ları)
    worker: tokio::task::JoinHandle<TraversalResult>,
}

pub struct DiscoveredFile {
    pub path:        NormalizedPath,
    pub size:        u64,
    pub kind:        FileKind,        // Regular | Symlink | Dir | Special
    pub permissions: bool,             // okunabilir mi?
}

pub fn start_traversal(root: &Path, opts: TraversalOpts) -> DirectoryStream {
    let (tx, rx) = mpsc::channel(256);   // 256 dosya buffer
    let worker = tokio::task::spawn_blocking(move || {
        // walkdir crate, BFS sırası (kullanıcı root'taki dosyaları daha hızlı görür)
        for entry in WalkDir::new(&root).follow_links(opts.follow_symlinks) {
            let entry = match entry { Ok(e) => e, Err(_) => continue };
            let discovered = DiscoveredFile { /* ... */ };
            // Sender bloke olursa scheduler tüketmedi → tarama da yavaşlar (backpressure)
            if tx.blocking_send(discovered).is_err() { break; }
        }
    });
    DirectoryStream { tx, worker }
}
```

**Scheduler beslemesi (akış):**

```rust
// scheduler.rs
pub async fn ingest_directory_stream(stream: DirectoryStream, queue: &QueueWriter) {
    let mut count = 0;
    while let Some(file) = stream.rx.recv().await {
        // İlk N dosyada hemen başla, geri kalanlar background queue
        queue.enqueue(file.into_task()).await;
        count += 1;
        // İlk 100 dosyadan sonra UI throttle (her 1000'de bir progress event)
        if count > 100 && count % 1000 == 0 {
            queue.emit_traversal_progress(count);
        }
    }
}
```

**Davranış:**
- Tarama 0.5 saniyede 200 dosya bulur → ilk 200 hemen kuyruğa
- Tarama devam ederken transfer'ler paralel başlar (ilk 100 → 8 connection × ilk 100/8 = 12 dosya per worker)
- UI'da "Taranıyor… 12,847 dosya" canlı sayaç (1Hz update)
- Tarama biterken transfer ortasında olabilir — sorun yok, kuyruk akıyor

**Auto-Tar (v2 stub, sandbox spec'i ile birlikte):** Hedef SFTP ise + shell access var + tar binary varsa, çok küçük dosyalar (örn. 10K+ adet 4KB altı) on-the-fly tar stream'ine çevrilir, karşıda untar. Bu v1.0'da değil — v2'de Auto-Archive Pipeline olarak rezerve.

### 15.6 Transfer Filter ve Ignore Rules (.transferignore)

Kullanıcı "Belgelerim" klasörünü atar — içinde `.DS_Store`, `Thumbs.db`, `desktop.ini`, izin yok dosyalar var. Klasik istemci her birine modal açar, kullanıcı 50 popup'la boğuşur. DTransfer Git mantığını kullanır.

```rust
pub struct TransferFilter {
    /// Default ignore patterns (OS metadata)
    default_ignores: GlobSet,
    /// User-defined .transferignore (gitignore syntax)
    user_ignores:    Option<Gitignore>,
    /// Permission denied stratejisi
    on_permission_denied: PermissionStrategy,
}

pub enum PermissionStrategy {
    /// Default — kuyruğa "Skipped (Permission Denied)" olarak ekle, transfer'i durdurma
    SkipSilently,
    /// Hata olarak işaretle, kullanıcıya rapor (bilinçli backup senaryosu)
    FailLoudly,
    /// Modal aç, kullanıcı karar versin (her dosya için)
    Ask,
}

const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    // macOS
    ".DS_Store",
    ".AppleDouble",
    ".LSOverride",
    "._*",                  // resource fork
    ".Spotlight-V100",
    ".Trashes",
    // Windows
    "Thumbs.db",
    "ehthumbs.db",
    "Desktop.ini",
    "$RECYCLE.BIN/",
    // Linux
    ".directory",
    ".Trash-*",
    // Editor
    "*.swp",
    "*.swo",
    "*~",
    ".vscode/.session",
    // VCS (opt-in, kullanıcı .git transferi isteyebilir)
    // .git/ default'ta DAHIL — kullanıcı .transferignore'a ekleyerek çıkarır
];
```

**Skipped vs Failed ayrımı (kritik UX):**

| Durum | Status | UI Görünüm |
|---|---|---|
| `.DS_Store` (default ignore) | `Skipped(OsMetadata)` | Listeden gizli, sayaçta "12 atlandı" |
| `.transferignore` match | `Skipped(UserIgnore)` | Liste filtresinde "Skipped" tab'ı |
| Permission denied (read) | `Skipped(PermissionDenied)` | Sarı warning ikon, tıklanınca detay |
| Network failure | `Failed(NetworkError)` | Kırmızı error ikon, retry button |
| Disk full | `Failed(DiskFull)` | Engine paused — kullanıcıya banner |

**Kritik fark:** `Skipped` ana transfer akışını **durdurmaz**. 100,000 dosyalık transfer'de 50 izinli olmayan dosya = 50 skip + 99,950 başarılı. FileZilla'da bu senaryoda kullanıcı 50 modal'ı kapatmak zorunda kalır ya da "abort all" basıp baştan başlar.

**.transferignore syntax (gitignore-uyumlu):**

```
# Build artifacts
node_modules/
target/
dist/

# Logs (büyük olabilir, manuel arşiv için)
*.log
logs/

# Geçici
*.tmp
.cache/

# Negate (override default)
!Thumbs.db    # Bu klasörde Thumbs.db'yi DAHIL ET (tersine çevir)
```

**Profile bazında konfigürasyon:** Her bağlantı profilinin kendi default `.transferignore` set'i olur. Kullanıcı UI'dan template seçer: "Web Development" (`node_modules`, `dist`, `.next`), "Photography" (`*.tmp`, `Thumbs.db` ama RAW dosyaları DAHIL), "Custom" (manuel düzenler).

**Engine entegrasyonu:** `DirectoryStream` worker, her bulunan dosya için `TransferFilter::should_include()` çağırır. Filter pozitifse `mpsc::Sender::send`, değilse skip count'a ekler. Filter işlemi CPU-light (regex match) — async runtime'da pahalılık yok.

### 15.7 SQLite WAL Checkpoint Policy (v1.14)

DbActor pattern serileştirilmiş yazma sağlar, ama checkpoint stratejisi belirsiz bırakılırsa WAL şişmesi production'da disk doldurur:

- 16 paralel transfer × 5sn batch progress write → WAL hızla büyür
- Long-running read connection (audit panel açık) checkpoint'i bloklar
- Default `wal_autocheckpoint=1000` page = 4MB; yetersiz olabilir
- WAL dosyası GB'a çıkarsa cold-start recovery ağırlaşır

**Startup PRAGMA seti:**

```rust
pub fn configure_queue_db(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;             -- FULL=güvenli ama yavaş, NORMAL WAL'da yeterli
        PRAGMA journal_size_limit = 67108864;    -- 64MB hard cap
        PRAGMA wal_autocheckpoint = 1000;        -- her 1000 page (4MB) auto checkpoint
        PRAGMA busy_timeout = 5000;              -- 5sn wait on lock
        PRAGMA cache_size = -32000;              -- 32MB page cache
        PRAGMA temp_store = MEMORY;
        PRAGMA mmap_size = 268435456;            -- 256MB mmap (büyük read'ler için)
    ")?;
    Ok(())
}
```

**Periodic forced checkpoint:**

DbActor 5 dakikada bir explicit `wal_checkpoint(TRUNCATE)` çağırır — long-running reader olsa bile WAL'i temizler:

```rust
pub enum DbCommand {
    // ... existing variants ...
    Checkpoint { mode: CheckpointMode, ack: oneshot::Sender<Result<(), DbError>> },
}

pub enum CheckpointMode {
    Passive,      // mevcut transaction'lar bitince
    Restart,      // yeni transaction'ları bekletir, mevcut bitince temizler
    Truncate,     // WAL'i sıfır boyuta truncate et — en agresif
}

// DbActor::run içinde periodic timer
let mut checkpoint_interval = tokio::time::interval(Duration::from_secs(300));  // 5dk
loop {
    tokio::select! {
        Some(cmd) = self.rx.recv() => { /* handle cmd */ }
        _ = checkpoint_interval.tick() => {
            // v1.15: TRUNCATE → PASSIVE fallback chain
            // Uzun süreli reader (Audit panel 1M kayıt) varsa TRUNCATE → SQLITE_BUSY
            // PASSIVE BUSY dönmez ama daha az agresif (sadece readable WAL'i temizler)
            match self.db.checkpoint(CheckpointMode::Truncate).await {
                Ok(_) => { /* WAL temizlendi */ }
                Err(DbError::Busy) => {
                    tracing::warn!("WAL checkpoint TRUNCATE busy (long reader); falling back to PASSIVE");
                    let _ = self.db.checkpoint(CheckpointMode::Passive).await;
                    // 1 saatte 3+ kez peş peşe BUSY → diagnostics warning
                    self.checkpoint_busy_streak += 1;
                    if self.checkpoint_busy_streak >= 3 {
                        event_bus.emit(EngineEvent::PersistentBusyCheckpoint);
                    }
                }
                Err(e) => tracing::error!(?e, "WAL checkpoint failed"),
            }
        }
    }
}
```

**TRUNCATE vs PASSIVE davranışı (v1.15):**

SQLite kurallarına göre `wal_checkpoint(TRUNCATE)` aktif reader varsa SQLITE_BUSY döner ve hiçbir şey yapmaz. Audit panel'de kullanıcı 1M kayıtlı log tablosunu filtreliyorsa uzun-running reader → 5 dakikalık TRUNCATE'lar peş peşe fail eder, WAL gigabaytlara şişer.

| Mod | Davranış | Reader varken |
|---|---|---|
| `TRUNCATE` | WAL'i sıfır boyuta çeker | ❌ SQLITE_BUSY, no-op |
| `RESTART` | Reader bitince WAL'i temizler, yeni yazmalar yeni segmente | ❌ SQLITE_BUSY |
| `PASSIVE` | Reader-safe; sadece okunmayan WAL frame'leri temizler | ✅ Çalışır, WAL kısmen temizlenir |

**UI tarafı disiplini (Bölüm 22.8 + 14 ile uyumlu):** Audit/View Cache gibi büyük listeleri UI'a yüklerken `LIMIT 1000` chunk'larla oku, her chunk arasında SQLite connection drop et. SQLite uzun reader olmasın diye:

```rust
// ❌ YANLIŞ — connection 30 saniye boyunca read open
let conn = db.pool.get().await?;
let stmt = conn.prepare("SELECT * FROM audit ORDER BY ts")?;
let rows = stmt.query_map([], row_to_record)?;
for row in rows { /* UI'a stream */ }

// ✓ DOĞRU — her chunk için kısa connection
let mut last_id = 0;
loop {
    let conn = db.pool.get().await?;
    let chunk: Vec<Record> = conn.prepare("SELECT * FROM audit WHERE id > ? ORDER BY id LIMIT 1000")?
        .query_map([last_id], row_to_record)?.collect::<Result<_, _>>()?;
    drop(conn);                          // connection serbest, checkpoint çalışabilir
    if chunk.is_empty() { break; }
    last_id = chunk.last().unwrap().id;
    ui_send_chunk(chunk).await;
}
```

**Test edilmesi gereken:**
- 1 saatlik soak test: 16 paralel transfer, periodic checkpoint, WAL boyut histogram'ı
- Audit panel açıkken `PRAGMA wal_checkpoint(TRUNCATE)` davranışı (SQLITE_BUSY dönüyor → PASSIVE fallback'e geçtiğini verify et)
- 64MB hard cap'e ulaşılırsa SQLite davranışı (yazma fail eder mi, blok mu?)

---

## 16. Rate Limiting ve API Quota

Cloud provider'lar (Dropbox, OneDrive, Graph API, GCS, Azure) agresif rate limit uygular. Bu katman olmadan adapter'lar production'da throttle yer.

### 16.1 RateLimitState

**Önemli:** Rate limit key'i `host` değil `profile_id` olmalı. Dropbox/OneDrive gibi servisler host bazlı değil, token/user bazlı limit uygular. İki farklı Dropbox hesabı aynı host'u paylaşır — host key olursa biri throttle yediğinde diğeri de yavaşlar.

```rust
pub struct RateLimitState {
    pub remaining:   u32,
    pub limit:       u32,
    pub reset_at:    DateTime<Utc>,
    pub retry_after: Option<Duration>,
}

pub struct RateLimiter {
    // KEY: profile_id — host değil
    // Aynı servisin farklı hesapları birbirini etkilemez
    states: HashMap<Uuid, RateLimitState>,  // profile_id → state
}

impl RateLimiter {
    pub fn update_from_response(&mut self, profile_id: Uuid, headers: &HeaderMap);
    pub async fn wait_if_limited(&self, profile_id: Uuid);
    pub fn should_throttle(&self, profile_id: Uuid) -> bool;
}
```

### 16.2 Adaptive Backoff Stratejisi

```rust
pub struct BackoffConfig {
    pub initial_ms:    u64,    // 1000
    pub multiplier:    f64,    // 2.0
    pub max_ms:        u64,    // 60_000
    pub jitter:        bool,   // true — thundering herd önleme
}

// TransferError::RateLimited { retry_after_secs } → doğrudan Retry-After kullan
// TransferError::ConnectionLost               → exponential backoff
// TransferError::Timeout                      → exponential backoff
// HTTP 503 / 429 without Retry-After          → backoff config ile
```

### 16.3 Provider Rate Limit Referansı

| Provider | Limit | Yanıt Header |
|---|---|---|
| Dropbox | 300 req/dakika | `X-RateLimit-*` |
| Microsoft Graph | 10k req/10dk | `Retry-After` |
| GCS | 1000 req/sn (ops) | `Retry-After` |
| Azure Blob | İstek boyutuna göre | `x-ms-error-code` |

---

## 17. Audit Trail ve KVKK/GDPR

> **Opt-in feature.** Ana pakette bulunur ama varsayılan kapalı. Etkinleştirmede KVKK/GDPR rıza metni zorunlu.

### 17.1 Hukuki Not

Audit özelliğini etkinleştiren kullanıcı oluşan kayıtların sorumluluğunu üstlenir. Yazılım geliştiricisi bu verilerden sorumlu değildir.

### 17.2 SQLite Batch Write

```rust
pub struct AuditEngine { tx: mpsc::Sender<AuditEvent> }

impl AuditEngine {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<AuditEvent>(1024);
        tokio::spawn(async move {
            let mut buffer: Vec<AuditEvent> = Vec::with_capacity(64);
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                tokio::select! {
                    Some(e) = rx.recv() => {
                        buffer.push(e);
                        if buffer.len() >= 64 { flush_to_db(&db, &mut buffer).await; }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() { flush_to_db(&db, &mut buffer).await; }
                    }
                }
            }
        });
        Self { tx }
    }

    pub async fn emit(&self, event: AuditEvent) {
        let _ = self.tx.send(event).await;  // fire-and-forget
    }
}
```

### 17.3 Maskeleme + Log Kuralları

```rust
pub struct MaskingEngine {
    pub mask_ip:              bool,
    pub mask_path:            bool,
    pub mask_filename:        bool,
    pub mask_username:        bool,
    pub redact_presigned_url: bool,  // her zaman true — credential içerir
}
```

**Kural:** Presigned URL log'a asla plaintext yazılmaz. `[PRESIGNED_URL_REDACTED]` ile değiştirilir.

### 17.3.1 Tracing PII Leak Koruması (v1.14)

`tracing::debug!(event = ?event, ...)` her event'in `Debug` impl'ini stringify eder. `TransferFailed { error: Authentication { reason: "Invalid password for user@example.com" } }` → tüm string log'a girer. `RemoteLocked { path: "/home/john_doe/secret_project/" }` → kullanıcı path'i sızar.

Default `RUST_LOG=info` filtreler bunu gizler ama diagnostics bundle'da `RUST_LOG=debug` ile çalışılınca dosyaya yazılır ve support'a gönderilir. KVKK / GDPR perspektifinden kabul edilemez.

**Çözüm 1 — Sensitive field wrapper:**

```rust
#[derive(Clone)]
pub struct Redacted<T>(pub T);

impl<T: std::fmt::Debug> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<redacted>")
    }
}

pub enum EngineEvent {
    TransferFailed {
        transfer_id: Uuid,
        error:       TransferError,
        remote_path: Redacted<String>,        // path açıkça redact edilir
    },
}
```

**Çözüm 2 — tracing-subscriber explicit off:**

```rust
use tracing_subscriber::EnvFilter;

pub fn init_tracing(level: tracing::Level) {
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy()
        // Hassas modülleri her durumda kapat
        .add_directive("dtransfer::credentials=off".parse().unwrap())
        
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()                       // structured logs, post-pass filtering kolay
        .with_writer(diagnostics_writer())
        .init();
}
```

**Çözüm 3 — Diagnostics bundle post-pass redaction:**

Bundle oluşturma anında log dosyaları üzerinden regex pass:

```rust
const PII_PATTERNS: &[(&str, &str)] = &[
    (r"/home/[^/\s]+",                 "/home/<user>"),     // Linux home dirs
    (r"C:\\Users\\[^\\\s]+",           r"C:\Users\<user>"), // Windows user dirs
    (r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+", "<email>"),       // Email addresses
    // Presigned URL signature kısmı zaten 14.3'te maskeli
];

pub fn redact_diagnostics_file(path: &Path) -> Result<()> {
    // Her line için regex replace
    // Bundle export öncesi son adım
}
```

**Üç katmanın birlikte sağladığı garanti:** tracing zamanında structural protection (wrapper + filter), bundle export zamanında defense-in-depth regex pass. Birinin atlandığı durumda diğeri yakalar.

### 17.4 KVKK / GDPR Kontrol Listesi

| Madde | Karşılık |
|---|---|
| Açık rıza | Audit etkinleştirmede onay kaydı (consent log) |
| Erişim hakkı | JSON dışa aktarım |
| Silme hakkı | Tarih aralığı veya tümünü sil |
| Veri taşınabilirliği | ISO 8601 JSON |
| Maskeleme | MaskingEngine, granüler kontrol |
| Saklama süresi | Otomatik temizleme (default: 90 gün) |

---

## 18. UI Mimarisi

### 18.1 Ekran Düzeni

```
┌──────────────────────────────────────────────────────────┐
│ [☰] DTransfer                [🌙/☀] [🔍] [⚙] [−][□][×] │
├─────────┬───────────────────────┬────────────────────────┤
│ Profil  │ ← Lokal ──────────── │ ─────── Uzak →        │
│         │ /home/user/           │ /var/www/              │
│ ★ S3    ├───────────────────────┼────────────────────────┤
│ ★ SFTP  │ [Virtual Scroller]    │ [Virtual Scroller]     │
│ ★ WebDAV│ Ad    Boyut  Tarih   │ Ad    Boyut  Tarih  Perm│
│         │ 📁 docs  –  ...     │ 📁 img  –   ...  drwxr │
│ Recent  │ 📄 f.txt 2KB ...    │ 📄 a.php 4KB ... -rw-r │
│ + Drop  │    ← Drag & Drop →   │                        │
│ + Wizard│                                               │
│ + Audit │                                               │
├─────────┴───────────────────────┴────────────────────────┤
│ Transfer Kuyruğu                     [Duraklat][Temizle] │
│ ▓▓▓▓░░  f.txt  P1████ P2████ P3██░  65%  4.2MB/s  ETA  │
│ ░░░░░░  g.zip  Kuyrukta (restart'tan kaldı)             │
├──────────────────────────────────────────────────────────┤
│ SFTP v2 · 14ms · TLS 1.3 · ↑6.2MB/s · 00:04:22        │
├──────────────────────────────────────────────────────────┤
│ [Log] [Grafik] [Konsol] [Audit]                       │
└──────────────────────────────────────────────────────────┘
```

### 18.2 Vue Virtual Scroller

10k+ dosyalı dizinlerde DOM render performansı için zorunlu:

```vue
<!-- FileList.vue -->
<RecycleScroller
  :items="files"
  :item-size="28"
  key-field="id"
  v-slot="{ item }"
>
  <FileListItem :file="item" />
</RecycleScroller>
```

`vue-virtual-scroller` (veya `@tanstack/vue-virtual`) kullanılır. Sıradan `v-for` 10k+ satırda UI'ı dondurur.

**Virtual Scroller + Drag & Drop Çatışması:** Virtual list DOM'da yalnızca görünür satırları tutar. Kullanıcı ekranda görünmeyen bir klasöre dosya sürüklemeye çalışırsa DOM hedefi yoktur — drop işlemi kaybolur.

Çözüm: Drop hedefini DOM'a değil, koordinat/index hesabına bağla:

```typescript
// FileList.vue — drag over handler
function onDragOver(event: DragEvent) {
  event.preventDefault()
  // Mouse Y koordinatından hedef index hesapla
  const listEl   = listRef.value.$el
  const rect     = listEl.getBoundingClientRect()
  const relY     = event.clientY - rect.top + listEl.scrollTop
  const index    = Math.floor(relY / ITEM_HEIGHT)
  const entry    = files.value[index]
  // DOM'a ihtiyaç yok — path direkt hesaplandı
  dropTargetPath.value = entry?.type === 'dir' ? entry.path : currentPath.value
}

function onDrop(event: DragEvent) {
  // dropTargetPath.value artık güvenilir — DOM bağımsız
  emit('drop', { files: getDragFiles(event), target: dropTargetPath.value })
}
```

Drag başladığında `dropTargetPath` reaktif olarak güncellenir. Scroll sırasında highlight efekti de index bazlı uygulanır — DOM yokluğu sorun olmaz.

**Linux Wayland D&D Fallback (v1.15):**

Ubuntu 22.04+, Fedora 38+, Debian 12+ varsayılan **Wayland** session kullanır. WebKitGTK'nın Wayland üzerindeki Drag & Drop API'si **yapısal olarak sorunlu**:
- Bazı event'ler hiç tetiklenmez (`dragenter` gelir, `drop` gelmez)
- Drag payload kaybolur (kullanıcı drop eder, target boş gelir)
- Cross-window D&D (dosya manager'dan DTransfer'a) %30 başarı oranı

X11 session'da bu sorunlar yok. Ama kullanıcı session türünü genelde bilmez ve bug'ı "DTransfer çalışmıyor" olarak yaşar.

**Çözüm — File Picker prominence:**

Linux'ta D&D area'nın yanında **eşit ağırlıkta** "Dosya/Klasör Seç" butonu. Sadece "Drag files here..." placeholder'ı yetmez — buton da olmalı, D&D fail durumunda kullanıcı net alternatife sahip.

```vue
<!-- FilePanel.vue -->
<template>
  <div class="upload-zone">
    <div v-if="dragOver" class="dropzone-active">
      Bırakmak için serbest bırakın
    </div>
    <div v-else class="dropzone-idle">
      <icon name="upload" />
      <p>Dosyaları buraya sürükleyin</p>
      <!-- Linux Wayland'de zorunlu, diğer platformlarda da yardımcı -->
      <p class="muted">veya</p>
      <button @click="openFilePicker" class="btn-primary">
        Dosya / Klasör Seç...
      </button>
    </div>
  </div>
</template>

<script setup>
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/adapter-dialog';

const isWayland = computed(() => {
  // Backend'den session type bilgisi (Linux'ta env: XDG_SESSION_TYPE)
  return platformInfo.value?.session === 'wayland';
});

async function openFilePicker() {
  const selected = await open({
    multiple: true,
    directory: false,    // veya true (folder mode)
    title: 'Yükleme için dosya seçin',
  });
  if (selected) {
    await queueUpload(Array.isArray(selected) ? selected : [selected]);
  }
}
</script>
```

**Backend Wayland detection:**

```rust
#[cfg(target_os = "linux")]
pub fn detect_session_type() -> &'static str {
    std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase().as_str() {
        "wayland" => "wayland",
        "x11"     => "x11",
        _         => "unknown",
    }
}
```

**Diagnostics bundle entry:** Bundle'da `system_info.json` içinde `session_type: "wayland"` field var. Kullanıcı D&D bug raporu açtığında destek "Wayland mı?" sorusunu sormaz, doğrudan görür.

**Settings → Linux → Drag & Drop:** Toggle "Force File Picker mode (D&D devre dışı)" — Wayland'de D&D ile defalarca sorun yaşayan kullanıcı bu mode'a geçebilir. UI tamamen file picker tabanlı çalışır.

**Tracking & telemetry yok:** D&D başarı oranı ölçülmez, kullanıcı opt-in olmadan veri toplanmaz (Bölüm 31 Telemetry Policy ile uyumlu).

### 18.3 Vue Bileşen Ağacı

```
App.vue
├── TitleBar.vue
├── Sidebar.vue
│   ├── ProfileList.vue
│   ├── QuickConnect.vue
├── MainContent.vue
│   ├── Toolbar.vue
│   └── DualPane.vue
│       └── FilePanel.vue × 2
│           ├── FileList.vue       ← Virtual Scroller
│           └── FileListItem.vue
├── BottomSection.vue
│   ├── TransferQueue.vue
│   │   └── TransferItem.vue      (chunk bar'ları + persist durumu)
│   ├── InfoPanel.vue
│   └── LogPanel.vue
│       ├── ProtocolConsole.vue
│       └── SpeedGraph.vue
└── Modals/
    ├── SettingsModal.vue
    ├── ProfileEditor.vue
    ├── CryptoConfig.vue
    └── ConsentDialog.vue
```

---

## 19. Tema Sistemi ve CSS Disiplini

### 19.1 Renk Token'ları

```css
[data-theme="dark"] {
  --bg-base:#0d1117; --bg-surface:#161b22; --bg-elevated:#21262d;
  --text-primary:#e6edf3; --text-secondary:#8b949e;
  --border-default:#30363d; --border-focus:#388bfd;
  --accent-primary:#388bfd; --accent-success:#3fb950;
  --accent-warning:#d29922; --accent-danger:#f85149;
}
[data-theme="light"] {
  --bg-base:#ffffff; --bg-surface:#f6f8fa; --bg-elevated:#ffffff;
  --text-primary:#1f2328; --text-secondary:#656d76;
  --border-default:#d1d9e0; --border-focus:#0969da;
  --accent-primary:#0969da; --accent-success:#1a7f37;
  --accent-warning:#9a6700; --accent-danger:#d1242f;
}
:root {
  --font-mono:'JetBrains Mono',monospace;
  --font-size-sm:12px; --font-size-md:13px;
  --radius-sm:4px; --radius-md:6px; --transition:150ms ease;
}
```

### 19.2 WebView Engine Matrisi

DTransfer iki farklı WebView engine üzerinde koşar:

| Platform | WebView | Engine |
|---|---|---|
| Windows 10/11 | WebView2 | Blink (Chromium 120+) |
| Linux | WebKitGTK 6.0+ | WebKit |
| macOS (topluluk portu) | WKWebView | WebKit (Linux ile aynı engine) |

Asıl test gerilimi **Blink ↔ WebKit** ekseninde. macOS portu açıldığında ekstra engine gelmiyor — Linux'taki WebKitGTK ile macOS'taki WKWebView aynı render motoruna sahip. Yani Win + Linux ikilisini doğru test etmek üç platformu kapsar.

### 19.3 CSS Disiplini Kuralları

WebView farklarını yönetmek için altı kuralı sıkı uygulamak gerekir. Her kural `frontend/RULES.md` içinde lint kuralı olarak kayıtlı; PR review checklist'inde:

**Kural 1 — Browser Default'a Asla Güvenme**

```css
/* frontend/src/styles/reset.css — TÜM sayfanın ilk import'u */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
button, input, select, textarea {
  font: inherit; color: inherit; background: none; border: none;
  appearance: none; -webkit-appearance: none; -moz-appearance: none;
}
img, svg, video, canvas { display: block; max-width: 100%; }
```

Tailwind preflight + bu reset birlikte çalışır.

**Kural 2 — Native HTML Widget'ları Yasak**

`<input type="date">`, `<input type="time">`, `<input type="color">`, `<input type="file">`, `<select>` Linux WebKitGTK ile Windows WebView2 arasında **tamamen farklı render** ediyor.

| Widget | DTransfer çözümü |
|---|---|
| Tarih seçici | Reka UI `Calendar` + custom popover |
| Saat | Custom input (`HH:MM` format) |
| Select / Combobox | Reka UI `Select` |
| File seçici | Tauri `dialog.open()` API (native, OS'a uygun) |
| Color picker | (kullanılmıyor) |
| Tooltip | Reka UI `Tooltip` (browser title attribute YOK) |

**Reka UI** (eski adıyla Radix Vue) Vue 3 native, headless component lib. Stil yok, sadece davranış. Tema token'ları kendi CSS'imiz.

**Kural 3 — Font Paketleme Zorunlu**

```
src/assets/fonts/JetBrainsMono/
├── JetBrainsMono-Regular.woff2
├── JetBrainsMono-Medium.woff2
├── JetBrainsMono-Bold.woff2
└── JetBrainsMono-Italic.woff2
```

```css
@font-face {
  font-family: 'JetBrains Mono';
  font-display: block;  /* swap değil, block — FOIT yerine ilk render geç ama tutarlı */
  src: url('@/assets/fonts/JetBrainsMono/JetBrainsMono-Regular.woff2') format('woff2');
  font-weight: 400;
}
/* Diğer ağırlıklar için tekrar */

* {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: geometricPrecision;
}
```

Sistem font'una asla düşme. Linux'ta JetBrains Mono yüklü değilse ve fallback'e düşersek (`monospace`) WebKitGTK seçtiği font ile WebView2'nin seçtiği font farklı boyutta olur — layout kayar.

**Kural 4 — Focus Ring Manuel**

`:focus` browser default'u Linux + Win arasında farklı (WebKit double-ring, Blink single ring). `outline:none` + manuel:

```css
*:focus { outline: none; }
.focus-ring:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}
```

Klavye erişilebilirliği için `:focus-visible` zorunlu — fare tıklamasında ring görünmez, klavye Tab'ında görünür.

**Kural 5 — Scrollbar Custom**

```css
/* WebKit + Chromium ortak */
*::-webkit-scrollbar { width: 12px; height: 12px; }
*::-webkit-scrollbar-track { background: var(--bg-surface); }
*::-webkit-scrollbar-thumb {
  background: var(--border-default);
  border-radius: 6px;
  border: 3px solid var(--bg-surface);
}
*::-webkit-scrollbar-thumb:hover { background: var(--text-secondary); }

/* Firefox fallback (gelecekte Servo/Verso geçişine karşı) */
* { scrollbar-width: thin; scrollbar-color: var(--border-default) var(--bg-surface); }
```

**Kural 6 — Backdrop Filter ve Modern CSS Fallback**

`backdrop-filter: blur()` Linux WebKitGTK 6.0+ destekliyor ama eski sürümlerde yok. macOS WKWebView vibrancy ile farklı. Fallback solid renk:

```css
.modal-backdrop {
  background: rgba(0, 0, 0, 0.6);  /* fallback */
}
@supports (backdrop-filter: blur(8px)) {
  .modal-backdrop {
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(8px);
  }
}
```

Aynı disiplin: `:has()`, `container queries`, `@scope`, `view transitions` — her biri `@supports` ile guard'lı kullanılır veya kullanılmaz.

### 19.4 Görsel Regression Test

CI'da Playwright screenshot diff. Win + Linux runner'da aynı sayfaları çek, baseline'dan %2'den fazla pixel farkı varsa PR bloklanır:

```typescript
// tests/visual/main-window.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Main window visual', () => {
  test('empty state renders consistently', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveScreenshot('empty-state.png', {
      maxDiffPixelRatio: 0.02,  // %2 eşik
    });
  });

  test('transfer queue 100 items', async ({ page }) => {
    await page.goto('/?seed=queue-100');
    await expect(page.locator('.transfer-queue')).toHaveScreenshot('queue-100.png', {
      maxDiffPixelRatio: 0.02,
    });
  });
});
```

```yaml
# .github/workflows/visual.yml
matrix:
  os: [windows-latest, ubuntu-latest]
steps:
  - run: pnpm playwright test --project=visual
  - if: failure()
    uses: actions/upload-artifact@v4
    with:
      path: test-results/  # diff görselleri PR'a iliştirilir
```

Baseline'lar `tests/visual/__screenshots__/{platform}/` altında ayrı tutulur — Win baseline'ı ve Linux baseline'ı doğal olarak farklı olabilir, her platform kendi referansını korur.

### 19.5 Ne Yapılmaz Listesi

Bunların hiçbiri kullanılmaz, PR'larda reddedilir:

- ❌ `<input type="date|time|file|color|month|week">`
- ❌ `<select>` (Reka UI Select kullan)
- ❌ `title="..."` attribute (Reka UI Tooltip kullan)
- ❌ `alert() / confirm() / prompt()` (Tauri dialog API kullan)
- ❌ Sistem font fallback (`monospace`, `sans-serif` solo)
- ❌ `outline: none` ardından `:focus-visible` koymadan bırakmak
- ❌ `backdrop-filter` `@supports` guard'ı olmadan
- ❌ Browser autofill renkleri (`-webkit-autofill` override zorunlu)

---

## 20. i18n — Çift Anadil Sistemi

- Vue I18n v9 · `tr.json` + `en.json` eş zamanlı güncellenir
- Fallback: EN → TR
- `Intl.DateTimeFormat` + `Intl.NumberFormat`

---

## 21. Erişilebilirlik ve Klavye Kısayolları

**Standartlar:** WCAG 2.2 AA · WAI-ARIA 1.2 · ISO 9241-171

### Global
| Kısayol | İşlev |
|---|---|
| `F1` | Yardım |
| `F2` | Yeniden Adlandır |
| `F3` | Ara |
| `F5` | Yükle |
| `F7` | Klasör Oluştur |
| `F8` | Sil |
| `F10` | Bağlantıyı Kes |
| `Ctrl+N` | Yeni Bağlantı |
| `Ctrl+L` | Adres Çubuğu |
| `Ctrl+T` | Tema Toggle |
| `Ctrl+D` | Diagnostics Bundle |
| `Ctrl+,` | Ayarlar |
| `Ctrl+Q` | Çıkış |

### Dosya Listesi
| Kısayol | İşlev |
|---|---|
| `↑↓` | Seçim |
| `Space` | Seç/Kaldır |
| `Ctrl+A` | Tümünü Seç |
| `Enter` | Klasörü Aç |
| `Backspace` | Üst Dizin |
| `Tab` | Lokal ↔ Uzak |
| `Ctrl+R` | Yenile |
| `Delete` | Sil |
| `Ctrl+H` | Gizli Dosyalar |

### Transfer / Panel
| Kısayol | İşlev |
|---|---|
| `P` | Duraklat / Devam |
| `C` | İptal |
| `R` | Tekrar Dene |
| `Ctrl+1–4` | Log / Grafik / Konsol / Audit |

---

## 22. Veri Görüntüleme Katmanı

Yüksek frekansta güncellenen alanlarda (transfer kuyruğu, hız grafiği) metin minimumda tutulur. **Disiplin: ikon + sayı + a11y metni + hover tooltip.** Salt ikon erişilebilirliği bozar, salt metin görsel olarak yorucudur — dördü birden zorunlu.

### 22.1 Görsel Dil Kuralı

```
[Görsel katman]      [A11y katman]              [Hover]
─────────────────    ────────────────────────   ──────────────
🕓 1.2 GB · 6.2 MB/s aria-label="Yükleniyor,    Tooltip: tam
                     1.2 gigabayt aktarıldı,    "Yükleniyor —
                     saniyede 6.2 megabayt"     1.2 / 4.8 GB
                                                6.2 MB/s
                                                ETA 9 dk 22 sn"
```

**Kural:**
- Görsel katmanda **kelime kullanma**: rakam, birim kısaltması, ikon yeter
- `aria-label` her transfer satırına zorunlu — screen reader durumu okur
- `title` attribute YASAK (Bölüm 18.5 yasak listesinde) — Reka UI Tooltip ile hover yap
- ETA "9d 22s" şeklinde kısalt: dakika `d`, saat `s`, gün `g`

### 22.2 Durum İkonları (Renk + Şekil)

| Durum | İkon | Renk Token'ı | A11y |
|---|---|---|---|
| Bekliyor | 🕓 (saat) | `--text-secondary` | "Kuyrukta bekliyor" |
| Bağlanıyor | ⟳ (dönen) | `--accent-primary` | "Bağlanıyor" |
| Aktif | ▶ (üçgen) | `--accent-primary` | "Aktarılıyor" |
| Duraklatıldı | ⏸ | `--accent-warning` | "Duraklatıldı" |
| Tamamlandı | ✓ | `--accent-success` | "Tamamlandı" |
| Hata (retry) | ⟳ | `--accent-warning` | "Yeniden deneniyor, N. deneme" |
| Hata (kalıcı) | ✕ | `--accent-danger` | "Başarısız oldu" |
| İptal | ⊘ | `--text-secondary` | "İptal edildi" |

İkonlar **lucide-vue-next** veya **@iconify-icons/tabler** — ikisi de SVG, font dependency yok, renk CSS `currentColor` ile tema'dan gelir.

**Renk körlüğü:** Yeşil/kırmızı tek başına yeterli değil — şekil farkı ekstra ayrım sağlar (✓ vs ✕). WCAG 2.2 başarım kriteri 1.4.1 karşılanır.

### 22.3 Bağlantı Detayları

```
SFTP · OpenSSH 9.2 · ⟷ 8 ch · ↻ 14ms · 🔒 TLS 1.3
↑ 6.2 MB/s   ↓ 0.8 MB/s   ⏱ 04:22
```

İkon çubuğu üst, sayısal panel alt. Kelime yok.

### 22.4 Transfer Kuyruğu Satır Şeması

```
[ikon] [ad]                        [hız]      [ETA]    [chunk]    [retry]
✓     report-2026-q1.pdf           —          —        12/12      —
▶     backup-archive.tar.gz        6.2 MB/s   9d22s    8/16       —
⟳     dataset.csv                  ↻ 30s      —        3/8        2.
🕓    photos.zip                   —          —        0/24       —
✕     server-config.yml            —          —        0/2        ∞
```

**Sütun genişlikleri:** İkon 24px sabit, ad flex (min 200px, ellipsis), hız 80px right-align, ETA 60px, chunk 60px, retry 32px. Sayılar **tabular-nums** font feature ile (JetBrains Mono zaten destekliyor) — 6.2 ve 12.4 aynı genişlikte hizalanır.

```css
.transfer-numeric {
  font-feature-settings: 'tnum' 1;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
```

### 22.5 Hız Grafiği

```
↑ 6.2 MB/s       [▁▂▃▅▆▇█▇▆▅▃▂▁ canvas, 120 örnek/30sn]
↓ 0.8 MB/s       [▁▁▁▂▂▁▁▂▃▂▁▁▁ canvas, 120 örnek/30sn]
```

Eksen etiketleri yok — pik + son değer yeterli. Hover'da Reka UI Tooltip ile "12.4 MB/s @ 14:23:18" gösterilir. Canvas üzerinde `mousemove` listener tek noktanın değerini çıkarır.

**Veri akışı:** tokio örnekleme → EngineEventAggregator (batch) → Vue `ringBuffer(120)` → Canvas 2D. Aggregator'dan geçtiği için per-sample IPC yok.

### 22.6 Boş Durum (Empty State)

Kuyruk boşken büyük metin değil, ikon + tek satır:

```
        📥
   Aktarım yok
[Sürükle bırak veya Ctrl+O]
```

Satır altı klavye kısayolu hint — kullanıcıya yön gösterir, tutorial gerekmez.

### 22.7 Erişilebilirlik Kontrol Listesi

Görsel sadeleştirme a11y ile takas edilmez. Her transfer satırı:

- [x] `role="row"` ve sütunlar `role="cell"`
- [x] `aria-label` durumu açık metinle söyler
- [x] `aria-live="polite"` ile screen reader durum değişimlerini duyurur
- [x] Klavye fokusu net `:focus-visible` ring'i (Bölüm 18.3 Kural 4)
- [x] Renk körlüğü için ikon + renk birlikte (1.4.1)
- [x] Kontrast oranı ≥ 4.5:1 (1.4.3) — token'lar tema sistemine göre garantili

### 22.8 view_cache.db (Paginated Remote Listing)

yüksek dosya sayılı S3 bucket veya `/var/log/` üzerinde naïve `Vec<RemoteEntry>` yaklaşımı OOM Killer'a davetiye. Bölüm 9.1'deki `list_dir` Stream API'nin UI tarafı:

```sql
-- ~/.dtransfer/view_cache.db (SQLite WAL, queue.db'den AYRI)
CREATE TABLE listing_session (
    session_id   TEXT PRIMARY KEY,    -- UUID, browser tab benzeri
    profile_id   TEXT NOT NULL,
    remote_path  BLOB NOT NULL,        -- v1.15: raw bytes (Invalid UTF-8 safe)
    path_kind    TEXT NOT NULL,        -- 'utf8' | 'raw_bytes'
    started_at   INTEGER NOT NULL,
    page_count   INTEGER DEFAULT 0,
    is_complete  INTEGER DEFAULT 0,    -- bool: tüm sayfalar geldi mi?
    sort_key     TEXT NOT NULL          -- 'name', 'size', 'mtime'
);

CREATE TABLE listing_entry (
    session_id   TEXT NOT NULL,
    entry_idx    INTEGER NOT NULL,     -- listing içindeki sıra
    name         BLOB NOT NULL,         -- v1.15: raw bytes (NormalizedPath kodu UTF-8 ise; Linux'ta raw)
    name_kind    TEXT NOT NULL,         -- 'utf8' | 'raw_bytes'
    size         INTEGER,
    mtime        INTEGER,
    kind         TEXT NOT NULL,         -- 'file', 'dir', 'symlink', 'special'
    permissions  INTEGER,
    raw_metadata BLOB,                  -- JSON: protocol-specific extras
    PRIMARY KEY (session_id, entry_idx)
);

CREATE INDEX idx_session_name ON listing_entry(session_id, name);
CREATE INDEX idx_session_size ON listing_entry(session_id, size);
CREATE INDEX idx_session_mtime ON listing_entry(session_id, mtime);
```

**UI akışı:**

```typescript
// Vue tarafı — VirtualScroller view_cache'den okur, RAM yerine
async function loadPage(sessionId: string, offset: number, limit: number) {
  const rows = await invoke('view_cache_query', {
    sessionId, offset, limit, sortBy: currentSort
  });
  return rows;  // ~30-50 entry, RAM'de geçici
}

// Scroll event → yeni offset → yeni query
// büyük ölçekli listelemede dizinde RAM kullanımı ~30MB sabit
```

**Lifecycle:**
- Session başladığında `listing_session` row create
- Adapter Stream'i her 1000 entry'de batch INSERT (DbActor ile serialize)
- UI listing tamamlanmadan da progressive görür (`is_complete=0` flag)
- Session kapandığında 1 saat sonra cleanup task siler (background scheduler)
- Crash sonrası restart'ta tüm `is_complete=0` session'lar invalidate

**Startup Orphan Cleanup (v1.15):**

1 saatlik in-process cleanup task'i crash anında çalışmaz. OS reboot, Task Manager kill, segfault sonrası — `view_cache.db` GB'larca veri ile diskte kalıcı orphan haline gelir. Aylar içinde sessizce kullanıcının diskini doldurabilir.

**Çözüm: startup'ta zorunlu sweep.**

```rust
pub async fn cleanup_view_cache_on_startup(db: &ViewCacheDb) -> Result<CleanupReport> {
    let now = Utc::now().timestamp();

    // 1. Tüm `is_complete=0` session'lar invalidate (crash anında yarım kalmış)
    let incomplete_dropped = db.execute("DELETE FROM listing_entry WHERE session_id IN
        (SELECT session_id FROM listing_session WHERE is_complete = 0)").await?;
    let sessions_dropped = db.execute("DELETE FROM listing_session WHERE is_complete = 0").await?;

    // 2. 24+ saat eski `is_complete=1` session'lar sil (cleanup task'in olamadığı durumlar)
    let cutoff = now - (24 * 3600);
    let old_entries = db.execute_with_params(
        "DELETE FROM listing_entry WHERE session_id IN
         (SELECT session_id FROM listing_session WHERE started_at < ?)", [cutoff]).await?;
    let old_sessions = db.execute_with_params(
        "DELETE FROM listing_session WHERE started_at < ?", [cutoff]).await?;

    // 3. Reclaim space (VACUUM async, blocking olmasın)
    if (sessions_dropped + old_sessions) > 10 {
        db.execute("PRAGMA incremental_vacuum(1000)").await?;  // 1000 page chunk
    }

    Ok(CleanupReport {
        incomplete_sessions: sessions_dropped,
        incomplete_entries: incomplete_dropped,
        expired_sessions: old_sessions,
        expired_entries: old_entries,
    })
}
```

UI'da gösterim: startup splash'inde "View cache cleanup: 3 incomplete sessions, 124K entries reclaimed (450MB)" toast (sadece >100MB reclaim varsa). Bölüm 28 (Operational Recovery Playbook) ile cross-reference: bu da bir recovery event.

**Sort Lockout While Loading (v1.15):**

geniş listeleme'lik listeleme yarım geldiğinde (örneğin 1.5M satır geldi, 8.5M daha yolda) kullanıcı UI'da "Boyuta Göre Sırala" butonuna basarsa `ORDER BY size DESC LIMIT 50` mevcut 1.5M üzerinden çalışır. 5sn sonra başka 10K satır gelir → liste sürekli **kayar/zıplar**. Kabul edilemez UX.

**Çözüm:** `is_complete = 0` iken server-side sort **lockout**. Sadece "Default (incoming order)" aktif. Sort dropdown disable + tooltip *"Listeleme tamamlandığında diğer sıralamalar aktif olur (%X yüklendi)"*.

```typescript
// Vue store
const sortLockout = computed(() => !currentSession.value?.is_complete);
const availableSorts = computed(() => {
  if (sortLockout.value) return ['default'];          // sadece default
  return ['default', 'name', 'size', 'mtime', 'kind']; // is_complete=1 → tüm sortlar
});

// Watch: is_complete 1'e döndüğünde kullanıcıya bildir
watch(() => currentSession.value?.is_complete, (newVal) => {
  if (newVal === 1) {
    toast('Listeleme tamamlandı. Diğer sıralamalar artık kullanılabilir.');
  }
});
```

**Search/filter ise loading sırasında çalışır** — kullanıcı dosya adıyla arama yapabilir, sonuçlar yeni entry'ler geldikçe progressively büyür. Sort'la fark: search "büyüyen sonuç seti" doğal hisse alır, sort "kayan liste" değil.

**Performans:**
- Insert batch 1000 entry → SQLite WAL'e ~50ms write
- LIMIT/OFFSET paging → ~5ms response (sorted index üzerinden)
- VACUUM scheduled (ayda bir, kullanıcı uygulamayı kapattığında)
- Worst case: geniş listeleme session, ~500MB SQLite — kullanıcı uyarılır, "Listeleme çok büyük, sıkıştırılıyor…" toast

**Sort behavior:** Veri SQLite içinde sıralı tutulduğu için sort değişiminde re-fetch gerekmez, farklı index üzerinden query. Search/filter de aynı pattern (`WHERE name LIKE ?`). **Ama** `is_complete=0` iken sort UI'da disable (v1.15).

**Memory garantisi:** UI'nin RAM ayak izi listing boyutundan **bağımsız**. 100 entry de geniş listeleme de aynı ~30-50MB UI memory. Bu Bölüm 30.2 RuntimeLimits ile uyumlu — `soft_memory_cap_mb` ihlal edilmez.

---

## 23. Conflict Resolution UX

FileZilla'nın **en sevilmeyen** yanı bu. Aynı isimde dosya bulunduğunda her seferinde modal açar, "apply to all" zayıf, checksum compare yok. DTransfer'ın gerçek diferansiyatörü olabilecek alanlardan biri.

### 23.1 ConflictPolicy Enum

```rust
pub enum ConflictPolicy {
    /// Modal aç, kullanıcıya sor (default)
    Ask,
    /// Sessizce üzerine yaz
    Overwrite,
    /// Sessizce atla, log'a yaz
    Skip,
    /// Yeni isimle indir (file (1).txt, file (2).txt)
    Rename,
    /// Boyut farklıysa resume mümkünse devam, değilse Ask
    ResumeIfPossible,
    /// Boyut + tarih karşılaştır, yenisi yoksa skip
    CompareSizeAndDate,
    /// Yenisi (newer mtime) kazanır
    KeepNewer,
    /// Checksum karşılaştır, farklıysa Ask (yavaş ama kesin)
    CompareChecksum,
}

pub struct ConflictDecision {
    pub policy:        ConflictPolicy,
    pub apply_to_all:  bool,
    /// Apply to all kapsamı: tüm queue / sadece bu klasör / sadece bu protokol / bu profile
    pub scope:         ApplyScope,
    pub remember:      bool,  // profile preference olarak kaydet
}

pub enum ApplyScope {
    OnlyThisFile,       // tek seferlik
    EntireQueue,        // tüm aktif queue
    CurrentDirectory,   // aynı parent path'teki dosyalar
    SameProfile,        // aynı bağlantı profilinin tüm transferleri
    Global,             // uygulama default'u
}
```

### 23.2 Conflict Detection

```rust
pub struct ConflictInfo {
    pub local:           FileMetadata,    // boyut, mtime, ctime, mode
    pub remote:          FileMetadata,
    pub conflict_kinds:  Vec<PathConflict>,  // Bölüm 12.3'ten
    pub resume_possible: bool,           // .dtresume var ve uyumlu mu
    pub size_match:      bool,
    pub mtime_diff_secs: i64,            // remote - local
    pub checksum_match:  Option<bool>,   // None = hesaplanmamış
}

impl ConflictInfo {
    /// FAT32 / exFAT (USB bellekler) mtime'ı 2sn granularity tutar.
    /// Aynı dosya farklı FS'lerde 0-2sn fark gösterebilir.
    const MTIME_GRANULARITY_TOLERANCE_SECS: i64 = 2;

    /// DST geçişleri ve bozuk timezone konfigürasyonları tipik 1-2 saat
    /// fark üretir. Eğer fark TAM saat katı VE boyutlar aynıysa, yüksek
    /// olasılıkla aynı dosya — gereksiz conflict modal'ı açma.
    fn is_likely_timezone_artifact(&self) -> bool {
        if !self.size_match { return false; }
        let abs = self.mtime_diff_secs.abs();
        // 1, 2, 3 ya da 4 saat (DST + offset bozukluğu kombinasyonu)
        // Tolerance: tam saatten ±5sn sapma kabul (NTP drift)
        for hours in 1..=4 {
            let expected = (hours as i64) * 3600;
            if (abs - expected).abs() <= 5 { return true; }
        }
        false
    }

    /// Hızlı kararlar için kullanılır
    pub fn quick_verdict(&self, policy: &ConflictPolicy) -> Option<ConflictAction> {
        // Önce FS granularity ve timezone artifact kontrolü
        // Bu kontroller TÜM policy'lerde uygulanır (ConflictPolicy::Ask hariç —
        // kullanıcı bilinçli olarak görmek istiyorsa modal açılır)
        if !matches!(policy, ConflictPolicy::Ask) && self.size_match {
            let abs = self.mtime_diff_secs.abs();
            if abs <= Self::MTIME_GRANULARITY_TOLERANCE_SECS
                || self.is_likely_timezone_artifact()
            {
                tracing::debug!(
                    diff = self.mtime_diff_secs,
                    "treated as same file (granularity/timezone heuristic)"
                );
                return Some(ConflictAction::Skip);
            }
        }
        // Klasik policy kararları
        match policy {
            ConflictPolicy::CompareSizeAndDate
                if self.size_match && self.mtime_diff_secs.abs() < 2 =>
                Some(ConflictAction::Skip),
            ConflictPolicy::KeepNewer if self.mtime_diff_secs > 0 =>
                Some(ConflictAction::Overwrite),
            ConflictPolicy::KeepNewer if self.mtime_diff_secs <= 0 =>
                Some(ConflictAction::Skip),
            ConflictPolicy::ResumeIfPossible if self.resume_possible =>
                Some(ConflictAction::Resume),
            _ => None,  // Ask gerekli
        }
    }
}

pub enum ConflictAction {
    Skip,
    Overwrite,
    RenameNew,
    RenameExisting,
    Resume,        // .dtresume'dan devam et
    Cancel,        // tüm queue iptal
}
```

**Pratik etki:**
- Yaz/kış saati geçişi sonrası ilk sync → 3600sn fark → skip
- USB bellekteki FAT32 dosya ile NTFS'teki dosya 1.7sn fark → skip
- **Gerçek edit:** 4500sn fark → herhangi bir tam saat katına uymuyor → conflict modal açılır (gerçek değişiklik)
- **Sahte conflict önleme:** Kullanıcı 100 dosyalık sync'te 100 modal görmek yerine sadece gerçekten farklı olanları görür

### 23.3 UI Modal Tasarımı

```
┌─ Çakışma — backup-2026.tar.gz ─────────────────────────┐
│                                                         │
│              [Yerel]              [Uzak]                │
│  Boyut:    1.24 GB             1.31 GB        ↑ +56MB  │
│  Tarih:    14 Şub 09:22        15 Şub 14:08   ↑ 1 gün  │
│  SHA-256:  ⏵ Hesapla            ⏵ Hesapla              │
│                                                         │
│  [Resume mümkün — 1.18 GB ortak başlangıç]              │
│                                                         │
│  ○ Resume (1.18 GB → 1.31 GB)                           │
│  ○ Üzerine yaz                                          │
│  ○ Yenisini "backup-2026 (1).tar.gz" olarak kaydet      │
│  ○ Atla                                                 │
│                                                         │
│  ☑ Bu klasördeki tüm çakışmalara uygula  [Klasör ▼]    │
│  ☐ Bu profil için varsayılan yap                        │
│                                                         │
│                            [İptal]  [Uygula]           │
└─────────────────────────────────────────────────────────┘
```

**Disiplin (Bölüm 22 ile uyumlu):** Sayılar tabular-nums, oklar (↑ ↓) ve renk farkları (yeşil/kırmızı) görsel dil için. Kelime yerine sembol. Hover'da Tooltip ile tam metin.

**Checksum compare lazy:** Default'ta hesaplanmaz (10GB dosya için 30sn). Kullanıcı "⏵ Hesapla" tıklayınca background'da `spawn_blocking` ile hash, sonuç modalda renklenir (eşleşirse ✓ yeşil, farklıysa ✕ kırmızı).

### 23.4 Bulk Conflict Handling

1000 dosyalık transfer'de her birine modal açmak işkence. Pattern:

```rust
pub struct BulkConflictResolver {
    /// Aktif çakışma kuyruğu
    pending: Vec<ConflictInfo>,
    /// Kullanıcı kararı bekleyen modal
    active_modal: Option<ConflictInfo>,
    /// Apply-to-all aktifse uygulanan policy
    sticky_decision: Option<(ConflictPolicy, ApplyScope)>,
}

impl BulkConflictResolver {
    pub fn resolve(&mut self, info: ConflictInfo) -> ConflictAction {
        // 1. Quick verdict (size+date, keep_newer, vs.)
        if let Some(action) = info.quick_verdict(&self.default_policy) {
            return action;
        }
        // 2. Sticky decision (apply-to-all aktif)
        if let Some((policy, scope)) = &self.sticky_decision {
            if scope.matches(&info) {
                return policy.to_action(&info);
            }
        }
        // 3. Modal aç, kullanıcı kararını bekle
        self.show_modal(info)
    }
}
```

**UX optimizasyonu:** Modal açıldığında pending count gösterilir: "Çakışma 3/47 — devam et veya tümüne uygula". Apply-to-all ile geri kalan 44 çakışma tek tıkla halledilir.

### 23.5 Resume Preference

`.dtresume` dosyası varsa ve içerikteki başlangıç byte'ları local dosya ile match ediyorsa **Resume** opsiyonu modal'da yeşil ile vurgulanır. Bölüm 14.5 ResumeChunk şemasıyla bütünleşir.

```rust
pub fn resume_compatibility(local: &Path, dtresume: &Path) -> ResumeStatus {
    let resume = read_dtresume(dtresume).ok()?;
    if !local.exists() { return ResumeStatus::None; }
    let local_size = local.metadata().ok()?.len();
    let last_complete_offset = resume.last_completed_chunk_end();
    // Hash karşılaştır: ilk N MB local ve resume manifest'te aynı mı?
    if local_size >= last_complete_offset
        && verify_partial_hash(local, &resume).await.unwrap_or(false)
    {
        ResumeStatus::Possible(last_complete_offset)
    } else {
        ResumeStatus::Incompatible
    }
}
```

### 23.6 Conflict Test Senaryoları

```rust
#[test] fn size_date_match_skips_silently()             { ... }
#[test] fn keep_newer_prefers_remote_when_remote_newer() { ... }
#[test] fn rename_creates_unique_suffix()               { ... }
#[test] fn apply_to_all_scope_directory_works()         { ... }
#[test] fn checksum_compare_blocks_until_complete()     { ... }
#[test] fn resume_offered_when_dtresume_compatible()    { ... }
#[test] fn case_insensitive_collision_triggers_modal()  { ... }
```

---

## 24. API Entegrasyon Stratejisi

### Presigned URL + Güvenlik

Tüm presigned URL'ler log masking ile korunur:

```rust
fn sanitize_for_log(url: &str) -> &str {
    if url.contains("X-Amz-Signature") || url.contains("sig=") || url.contains("sv=") {
        "[PRESIGNED_URL_REDACTED]"
    } else { url }
}
```

### Custom REST API

```rust
pub struct CustomRestProfile {
    pub base_url:         String,
    pub auth_type:        CustomAuthType,
    pub auth_credentials: EncryptedCredential,
    pub endpoints:        CustomEndpoints,
    pub custom_headers:   HashMap<String, String>,
    pub response_mapping: ResponseMapping,
    pub tls_verify:       bool,
    pub timeout_secs:     u64,
}
```

---

## 25. Bağlantı Profil Yönetimi

```rust
pub struct ConnectionProfile {
    pub id:             Uuid,
    pub name:           String,
    pub protocol:       ProtocolKind,
    pub host:           String,
    pub port:           u16,
    pub credential_ref: Option<KeychainRef>,  // parola JSON'da olmaz
    pub remote_root:    String,
    pub transfer_opts:  TransferOptions,
    pub crypto_config:  Option<CryptoConfig>,
    pub color:          Option<HexColor>,
    pub notes:          Option<String>,
    pub created_at:     DateTime<Utc>,
    pub last_used_at:   Option<DateTime<Utc>>,
}
```

Dışa aktarım: AES-256-GCM şifreli `.dtransfer` dosyası.

### 25.1 Credential Storage Backend (v1.14)

`keyring` crate platform-specific store kullanır ama bazı ortamlarda yok:

| Platform | Backend | Mevcudiyet |
|---|---|---|
| Windows | Credential Manager (DPAPI) | Tüm Windows sürümlerinde var |
| macOS | Keychain | Tüm macOS sürümlerinde var |
| Linux Desktop | Secret Service (gnome-keyring/kwallet via D-Bus) | GNOME/KDE'de var |
| Linux Server | — | **Yok** (D-Bus + keyring daemon olmadığı için) |
| Alpine, distroless | — | **Yok** |
| WSL2 (default) | — | **Yok** |
| Docker | — | **Yok** |

Ana doc "Linux birinci sınıf platform" iddiasıyla doğrudan çelişen durum. SFTP server-to-server transfer tipik kullanım, ama headless kullanıcı credential saklayamayacak.

**Çözüm: File-based encrypted credential store fallback.**

```rust
pub enum CredentialBackend {
    WindowsDpapi,
    LinuxSecretService,
    MacosKeychain,
    /// File-based fallback: ~/.dtransfer/credentials.enc
    /// Master password ile Argon2id + XChaCha20 ile encrypt
    /// Headless server, Alpine, WSL2 default için
    EncryptedFile { master_key: Arc<MasterKeyCache> },
}

impl CredentialBackend {
    /// Startup'ta otomatik backend seç
    pub async fn detect() -> Result<Self, CredentialError> {
        #[cfg(windows)]
        return Ok(Self::WindowsDpapi);

        #[cfg(target_os = "macos")]
        return Ok(Self::MacosKeychain);

        #[cfg(target_os = "linux")]
        {
            // Önce Secret Service dene
            if dbus_secret_service_available().await {
                return Ok(Self::LinuxSecretService);
            }
            // Fallback: encrypted file (Alpine, WSL2, headless server)
            let key = MasterKeyCache::prompt_or_load()?;
            return Ok(Self::EncryptedFile { master_key: Arc::new(key) });
        }
    }
}
```

**Encrypted file format (`~/.dtransfer/credentials.enc`):**

```
+------------------+
| Magic: "DTCRED"  | 6 bytes
+------------------+
| Version: 1       | u8
+------------------+
| Argon2 salt      | 16 bytes
+------------------+
| Argon2 params    | m=46MB t=2 p=1 (file = sıkı param, RAM cache farklı)
+------------------+
| XChaCha20 nonce  | 24 bytes
+------------------+
| Ciphertext       | JSON: {profile_id: credential, ...}
+------------------+
| Poly1305 tag     | 16 bytes
+------------------+
```

**UX akışı:**

- **İlk çalıştırma (Linux headless):** *"Linux Secret Service bulunamadı. Credential'larınızı korumak için bir master password belirleyin."* → password prompt + confirm + Argon2 derivation (~250ms beklenir, deliberate)
- **Sonraki çalıştırma:** *"DTransfer credential'ları için master password:"* → unlock
- **Master Key Cache** (Bölüm 13.7): 30dk auto-lock zaten var, aynı sistem kullanılır

**CLI flag:** `--no-keyring` → zorla encrypted file mode (Linux desktop'ta Secret Service var ama kullanılmasını istemeyen privacy-paranoid kullanıcılar için).

**SSH key file zaten dışarıda:** SSH private key dosyaları (`~/.ssh/id_*`) credential store'da değil, kullanıcının kendi dosya sisteminde. DTransfer sadece "bu profile için bu key dosyasını kullan" pointer'ı saklar — file path keychain'e değil, profile JSON'una yazılır.

### 25.2 Headless / Automation Master Key Injection (v1.15)

`EncryptedFile` backend interactive password prompt'a ihtiyaç duyar (Argon2id derivation için master password). `MasterKeyCache` 30dk idle sonrası auto-lock (Bölüm 13.7). Bu pattern **headless senaryolarda kırılır:**

- Linux server'da cron job ile gece 03:00 yedekleme → prompt yok, transfer fail
- CI/CD pipeline'da otomatik sync → terminal interactive değil
- Docker container'da background job → stdin TTY yok
- WSL2 script'i Windows Task Scheduler'dan → headless

**Çözüm: Üç-yollu master key resolution.**

```rust
pub enum MasterKeySource {
    /// Interactive prompt (default GUI mode)
    Prompt,
    /// Environment variable: DTRANSFER_MASTER_KEY
    /// Master password plaintext olarak env'de — convenience, low security
    EnvVar,
    /// Master password keyfile path: DTRANSFER_MASTER_KEY_FILE
    /// Dosyanın izinleri 0600 (umask check zorunlu)
    /// Dosya içeriği master password (single line)
    KeyFile,
}

impl MasterKeyCache {
    pub async fn unlock_for_session() -> Result<Self> {
        // Resolution order: env var → keyfile → interactive prompt
        if let Ok(pwd) = std::env::var("DTRANSFER_MASTER_KEY") {
            return Self::derive_from_password(&pwd, MasterKeySource::EnvVar);
        }

        if let Ok(path) = std::env::var("DTRANSFER_MASTER_KEY_FILE") {
            let path = PathBuf::from(path);
            // Permission check — Unix: 0600 zorunlu
            #[cfg(unix)]
            {
                let meta = tokio::fs::metadata(&path).await?;
                let mode = meta.permissions().mode() & 0o777;
                if mode != 0o600 {
                    return Err(MasterKeyError::InsecureKeyfilePermissions {
                        path: path.clone(),
                        actual: mode,
                        expected: 0o600,
                    });
                }
            }
            let pwd = tokio::fs::read_to_string(&path).await?;
            let pwd = pwd.trim();           // newline kaldır
            return Self::derive_from_password(pwd, MasterKeySource::KeyFile);
        }

        // Interactive prompt — only if attached to TTY
        if !atty::is(atty::Stream::Stdin) {
            return Err(MasterKeyError::NoTtyAndNoInjection {
                hint: "Headless mode requires DTRANSFER_MASTER_KEY or DTRANSFER_MASTER_KEY_FILE env var",
            });
        }
        let pwd = rpassword::read_password_with_prompt("Master password: ")?;
        Self::derive_from_password(&pwd, MasterKeySource::Prompt)
    }
}
```

**Auto-lock policy headless mode'da farklı:**

```rust
impl MasterKeyCache {
    pub fn auto_lock_duration(&self) -> Option<Duration> {
        match self.source {
            MasterKeySource::Prompt   => Some(Duration::from_secs(30 * 60)),  // 30dk (default)
            MasterKeySource::EnvVar   => None,                                 // process boyunca açık
            MasterKeySource::KeyFile  => None,                                 // process boyunca açık
        }
    }
}
```

Headless'ta auto-lock yok — long-running cron/daemon yedekleme job'u tamamlanana kadar key cache'te kalır. Process exit'inde `ZeroizeOnDrop` ile bellekten temizlenir.

**CLI helpers:**

```bash
# Bir kerelik headless transfer (env var ile)
DTRANSFER_MASTER_KEY="secret123" dtransfer-cli sync ./local s3://bucket/

# Keyfile ile (daha güvenli, env var leak korkusuz)
chmod 600 ~/.dtransfer/master.key
echo "secret123" > ~/.dtransfer/master.key
chmod 600 ~/.dtransfer/master.key  # tekrar — set'ten sonra mod
DTRANSFER_MASTER_KEY_FILE=~/.dtransfer/master.key dtransfer-cli sync ./local s3://bucket/

# Systemd service için
# /etc/systemd/system/dtransfer-backup.service
# Environment="DTRANSFER_MASTER_KEY_FILE=/etc/dtransfer/master.key"
# (dosya root:root 0600, sadece root erişebilir)
```

**Güvenlik trade-off'ları:**

| Yöntem | Güvenlik | UX |
|---|---|---|
| Interactive prompt | En yüksek (RAM-only, 30dk auto-lock) | İnsan müdahalesi gerekli |
| Keyfile (0600) | Yüksek (FS permissions koruması) | Setup gerekli, dosya yönetimi |
| Env var | Orta (`ps`/`/proc/*/environ` leak riski) | En kolay, drop-in |

UI'da Settings → "Headless Mode" toggle: env/keyfile destek olduğunu açıklayan doc link + örnek `systemd` service file. Production'da keyfile önerilir, env var ad-hoc CLI use için.

**Tehlike notu:** `DTRANSFER_MASTER_KEY` env var'ı `ps eww` veya `/proc/<pid>/environ` üzerinden başka kullanıcılar tarafından okunabilir (multi-tenant Linux server). Production headless deployment'ta keyfile + restricted permissions zorunlu.

---

## 26. Network Optimization Wizard

> **Opt-in feature.** Varsayılan kapalı, AV uyarılı rıza metniyle.

```
① Gecikme  ② Bant Genişliği  ③ BDP  ④ Paralel
⑤ Cipher   ⑥ Port/Firewall   ⑦ MTU  ⑧ DNS
```

BDP formülü: `bandwidth_bps × RTT_s = optimal_chunk_bytes`

---

## 27. Modülerlik ve Test Mimarisi

### Prensipler
- Rust dosya limiti: ~250 satır
- Vue bileşen limiti: ~150 şablon + ~100 script
- Feature flags: istenmeyen özellik binary'e girmez

### Test Kapsamı

```rust
#[cfg(test)]
mod tests {
    // Crypto
    #[test] fn xchacha20_roundtrip()                 { ... }
    #[test] fn nonce_uniqueness_across_chunks()       { ... }
    #[test] fn wrong_key_fails()                     { ... }

    // Transfer
    #[test] fn multipart_reassembles_correctly()     { ... }
    #[test] fn atomic_write_survives_interruption()  { ... }
    #[test] fn resume_skips_completed_chunks()       { ... }

    // SFTP
    #[test] fn capability_probe_limits_parallelism() { ... }
    #[test] fn nas_banner_sets_conservative_limit()  { ... }

    // Rate limit
    #[test] fn rate_limiter_respects_retry_after()   { ... }
    #[test] fn adaptive_backoff_with_jitter()        { ... }

    // Queue
    #[test] fn queue_persists_across_restart()           { ... }
    #[test] fn failed_task_retried_on_startup()          { ... }
    #[test] fn invalid_state_transition_rejected()       { ... }  // can_transition_to
    #[test] fn bytes_done_batch_write_not_per_chunk()    { ... }
    #[test] fn rate_limiter_isolates_per_profile()       { ... }  // profile_id key

    #[test] fn zombie_killed_on_parent_exit()             { ... }
    #[test] fn presigned_url_refreshed_before_expiry()   { ... }
    #[test] fn presigned_url_redacted_in_logs()          { ... }
    #[test] fn presigned_generation_stale_ignored()      { ... }  // race koruması
    #[test] fn jsonrpc_error_maps_to_transfer_error()    { ... }
    // Progress
    #[test] fn progress_aggregator_throttles_to_250ms()  { ... }
    #[test] fn progress_not_emitted_per_chunk()          { ... }

    // SFTP RAM
    #[test] fn max_inflight_bytes_limits_concurrent_chunks() { ... }

    // Error taxonomy
    #[test] fn rate_limited_error_extracts_retry_after() { ... }
    #[test] fn url_expired_triggers_refresh()            { ... }
}
```

Mock sunucular:

| Protokol | Mock |
|---|---|
| SFTP | russh test server |
| S3 | localstack (Docker) |
| HTTP/WebDAV | wiremock (Rust) |
| Rate limiting | wiremock → 429 + Retry-After |

### CI

```yaml
jobs:
  rust:
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - run: cargo test --all-features
      - run: cargo test --no-default-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
  vue:
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - run: pnpm vitest run
      - run: pnpm playwright test                # functional + visual
  build:
    strategy:
      matrix:
        include:
          - os: windows-latest
            target: msi-portable
          - os: ubuntu-latest
            target: appimage-deb
    runs-on: ${{ matrix.os }}
    steps:
      - run: pnpm tauri build
      # Win: Authenticode imzalama (signtool)
      # Linux: AppImage build + .deb pack (cargo-deb / dpkg-deb)
```

**macOS portu:** Topluluk PR açtığında `macos-latest` matrix'e eklenir. v1.0–v1.2'de koşmaz, MR runtime'ı kısa.

### Fuzz Testing — Protocol Parsers (v1.14)

Protocol parser'lar (FTP yanıt, SFTP packet, WebDAV PROPFIND XML, S3 XML error) ağdan gelen güvenilmez input parse eder. DoS, memory exhaustion, panic-based DoS, parse confusion bug'larının doğal yatağı. Unit + integration test bunları yakalamaz — fuzz şart.

**cargo-fuzz hedefleri:**

```rust
// fuzz/fuzz_targets/ftp_response.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = dtransfer::adapters::ftp::FtpResponse::parse(data);
});

// fuzz/fuzz_targets/sftp_packet.rs
fuzz_target!(|data: &[u8]| {
    let _ = dtransfer::adapters::sftp::Packet::parse(data);
});

// fuzz/fuzz_targets/webdav_propfind.rs
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = dtransfer::adapters::webdav::parse_propfind(s);
    }
});

// fuzz/fuzz_targets/dtresume_parse.rs
fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<dtransfer::queue::ResumeState>(data);
});

// fuzz/fuzz_targets/chunkmap_blob.rs
fuzz_target!(|data: &[u8]| {
    let _ = dtransfer::queue::ChunkMapBlob::parse(data);
});
```

**Coverage hedefleri:**

| Hedef | Min Coverage | Min CI süresi |
|---|---|---|
| SFTP packet parser | 95% | 1 saat |
| WebDAV XML parser | 90% | 1 saat |
| S3 XML error parser | 90% | 30 dk |
| .dtresume deserialize | 95% | 30 dk |
| Chunkmap blob parser | 95% | 30 dk |

**CI integration — haftalık scheduled job:**

```yaml
fuzz:
  schedule: cron: '0 3 * * 0'  # Pazar 03:00 UTC
  runs-on: ubuntu-latest
  timeout-minutes: 90
  steps:
    - uses: actions/checkout@v4
    - run: cargo install cargo-fuzz
    - run: cargo fuzz run ftp_response -- -max_total_time=600
    - run: cargo fuzz run sftp_packet -- -max_total_time=600
    - run: cargo fuzz run webdav_propfind -- -max_total_time=600
    - run: cargo fuzz run dtresume_parse -- -max_total_time=600
    - run: cargo fuzz run chunkmap_blob -- -max_total_time=600
    - name: Upload crashes
      if: failure()
      uses: actions/upload-artifact@v4
      with:
        name: fuzz-crashes
        path: fuzz/artifacts/
```

**Crash bulunduğunda:** GitHub issue otomatik açılır (`bug-fuzz-found` label), repro input artifact olarak yüklenir. PR merge'leri fuzz job'unun yeşil olmasına bağlı değil (zaman alır), ama crash bulunursa hotfix sprint'i tetiklenir.

---

## 28. Crash Recovery ve Fault Tolerance

### Transfer Kesintisi

| Senaryo | Çözüm |
|---|---|
| Güç kesintisi (indirme) | .dtresume atomic → kaldığı yerden |
| Güç kesintisi (yükleme) | S3 multipart abort + cleanup |
| AV lock | Retry + backoff; UI uyarı |
| Yarım chunk | tmp → rename atomic; orphan tmp silinir |

### Veritabanı

| Senaryo | Çözüm |
|---|---|
| SQLite WAL bozulması | `PRAGMA integrity_check` başlangıçta |
| Profil DB bozulması | Her yazma sonrası `.bak` kopyası |
| Queue DB bozulması | WAL recovery; başarısızsa sıfırla |

### Güncelleme Rollback (v1.16: atomik binary + config + DB schema rollback)

Eski (v1.15) policy şuydu:
```
1. dtransfer-v{eski}.backup oluştur
2. Yeni binary yerleştir
3. 5sn içinde IPC yanıtı alınamazsa otomatik rollback
```

Bu yetersiz — gerçek update sırasında **3 şey değişir** ve hepsi birlikte rollback edilmeli:

1. **Binary** — `dtransfer.exe` / `dtransfer` executable
2. **Config schema** — `config.json` v1 → v2 migration olabilir (Bölüm 34)
3. **DB schema** — `queue.db` migration (yeni kolonlar, index'ler, eski kolon drop)

Eğer sadece binary rollback yapılırsa: eski binary yeni config/DB schema'yı okuyamayabilir → app yine açılmaz, dead lock.

**v1.16 atomic rollback policy:**

```rust
pub struct UpdateTransaction {
    pub backup_binary:       PathBuf,           // dtransfer-v{old}.backup
    pub backup_config:       PathBuf,           // config-v{old_schema}.json.backup
    pub backup_db_snapshot:  PathBuf,           // queue-v{old_schema}.db.backup (snapshot, not WAL copy)
    pub new_binary_path:     PathBuf,
    pub config_migration:    Option<ConfigMigration>,  // None = no schema change
    pub db_migration:        Option<DbMigration>,
    pub started_at:          DateTime<Utc>,
}

impl UpdateTransaction {
    /// Tüm transaction'ı geri al — failure recovery için
    pub async fn rollback(&self) -> Result<()> {
        // 1. Binary'yi eskiye al (atomic rename)
        atomic_replace(&self.new_binary_path, &self.backup_binary).await?;

        // 2. Config'i eskiye al
        if let Some(mig) = &self.config_migration {
            mig.rollback().await?;                    // schema v2 → v1
            atomic_replace(&config_path(), &self.backup_config).await?;
        }

        // 3. DB'yi eskiye al
        if let Some(mig) = &self.db_migration {
            // DB rollback complex: schema değişikliğinin reverse'i
            // En güvenli: snapshot'tan restore (kullanıcı veri kaybı kabul edilebilir update öncesi state)
            mig.rollback_via_snapshot(&self.backup_db_snapshot).await?;
        }

        Ok(())
    }
}
```

**Update workflow (transaction pattern):**

```
1. UpdateTransaction başlat
   ├── 1.1 Eski binary backup
   ├── 1.2 Config snapshot (v1 schema'da)
   └── 1.3 queue.db online backup (PRAGMA backup INTO ...)

2. Yeni binary yerleştir
   └── (atomic, eski binary'yi silmez; backup duruyor)

3. Yeni binary başlat
   ├── 3.1 Config schema check + auto-migrate (gerekirse)
   ├── 3.2 DB schema check + auto-migrate (gerekirse)
   └── 3.3 IPC handshake "Started OK"

4. 30 saniye (eski 5sn yetersiz — büyük DB migration zaman alabilir) içinde handshake yok
   └── 4.1 UpdateTransaction.rollback() çağır
       ├── Binary geri al
       ├── Config geri al
       └── DB geri al

5. Başarılı handshake
   ├── 5.1 UpdateTransaction commit (backup'lar 7 gün sonra silinir)
   └── 5.2 last_seen_version, last_seen_released_at update (anti-rollback için)
```

**DB migration için snapshot stratejisi:**

`queue.db` migration'ları (yeni kolon, yeni index) idempotent ALTER statement'ları ile yapılır:

```sql
-- Migration: v15_to_v16 (BLOB path kolonları)
ALTER TABLE transfer_tasks ADD COLUMN source_path_new BLOB;
ALTER TABLE transfer_tasks ADD COLUMN source_path_kind TEXT;
UPDATE transfer_tasks SET source_path_new = CAST(source_path AS BLOB),
                          source_path_kind = 'utf8'
                          WHERE source_path IS NOT NULL;
-- ...
-- (Eski TEXT kolonu drop edilmez; rollback için tutulur)
-- DROP COLUMN source_path; -- BU YOK! Eski kolon kalır, sadece kullanılmaz.
```

Rollback'te yeni kolonlar silinir, eski kolonlar zaten yerinde. Veri kaybı yok.

**Hangi durumda rollback?**

| Durum | Davranış |
|---|---|
| 30sn içinde "Started OK" IPC handshake yok | Otomatik rollback |
| Yeni binary panic ile çıktı (exit code != 0) | Otomatik rollback |
| Config migration fail oldu | Otomatik rollback |
| DB migration fail oldu (corrupt, BUSY timeout, schema mismatch) | Otomatik rollback |
| Kullanıcı manuel "Rollback to previous version" tıkladı | Manuel rollback (UI Settings → Updates) |

**Backup retention:**

- Last 3 versiyon backup'ı tutulur (~150MB disk)
- 30+ gün eski backup'lar otomatik silinir
- Manual cleanup: `dtransfer-cli cleanup --backups`

---

### Updater Anti-Rollback (TUF Benzeri)

Sadece "Ed25519 imza" yetmez. **İmzalı eski (vulnerable) binary'yi kullanıcıya zorla yükletme** = downgrade attack. v1.0.5'te kritik bir güvenlik açığı kapandıysa, saldırganın v1.0.4 imzalı binary'yi auto-update server'ına yerleştirmesi her şeyi geri açar.

**Update manifest formatı:**

```json
{
  "schema": "2",
  "version":              "1.2.3",
  "min_supported_version": "1.1.0",
  "released_at":          "2026-08-15T10:00:00Z",
  "expires_at":           "2026-09-15T10:00:00Z",
  "binary_url":           "https://updates.dtransfer.app/v1.2.3/dtransfer-x86_64.zip",
  "binary_sha256":        "...",
  "ed25519_signature":    "...",
  "manifest_signature":   "..."
}
```

**Anti-rollback kuralları:**

```rust
pub struct UpdateValidator {
    current_version:        Version,
    last_seen_version:      Version,    // QueueDb'den persist
    last_seen_released_at:  DateTime<Utc>,
    anchor_pubkey:          Ed25519PublicKey,
}

impl UpdateValidator {
    pub fn validate(
        &self,
        manifest: &UpdateManifest,
        server_date_header: Option<DateTime<Utc>>,   // v1.15: HTTP Date header
    ) -> Result<(), UpdateError> {
        // 0. Clock drift check (v1.15) — yerel saat referansı ALMADAN ÖNCE
        let now = self.reference_time(server_date_header)?;

        // 1. Manifest imza kontrolü (anchor ile)
        verify_ed25519(&manifest.manifest_signature, &manifest.bytes(), &self.anchor_pubkey)?;

        // 2. Manifest expiry — replay attack koruması (server time veya safe local time)
        if now > manifest.expires_at {
            return Err(UpdateError::ManifestExpired);
        }

        // 3. Downgrade attack — versiyon mevcut'tan veya en son görülen'den eski olamaz
        if manifest.version < self.current_version {
            return Err(UpdateError::Downgrade);
        }
        if manifest.version < self.last_seen_version {
            return Err(UpdateError::DowngradeFromSeen);
        }

        // 4. Min supported version — eski binary'leri kalıcı olarak invalidate et
        if self.current_version < manifest.min_supported_version {
            return Err(UpdateError::ForcedUpgrade);
            // Kullanıcı manuel update yapmak zorunda — eski sürüm artık çalışmaz
        }

        // 5. Released_at monotonic — saldırgan eski tarihli imzalı manifest sokamaz
        if manifest.released_at < self.last_seen_released_at {
            return Err(UpdateError::TimestampRollback);
        }

        // 6. Binary SHA-256 + Ed25519 signature kontrolü (download sonrası)
        Ok(())
    }

    /// v1.15: Reference time = server date header (öncelikli) veya local time (fallback).
    /// Local clock drift >24 saat ise update reddedilmez — kullanıcıya saat uyarısı verilir.
    fn reference_time(&self, server_date: Option<DateTime<Utc>>) -> Result<DateTime<Utc>, UpdateError> {
        let local_now = Utc::now();

        match server_date {
            Some(server_now) => {
                let drift = (server_now - local_now).num_seconds().abs();
                if drift > 24 * 3600 {                            // 24 saatten fazla drift
                    return Err(UpdateError::ClockDriftWarning {
                        local: local_now,
                        server: server_now,
                        drift_secs: drift,
                        hint: "Sistem saatiniz yanlış. Lütfen saat ayarlarını düzeltip tekrar deneyin. \
                               Güvenlik için saate güvenmeden güncelleme yapılmaz.",
                    });
                }
                // Drift kabul edilebilir — server time'ı reference olarak kullan
                Ok(server_now)
            }
            None => {
                // Server Date header yok (proxy attığı vs.) — local time'a düşmek zorundayız
                // Ama bu durumu diagnostics'e işle
                tracing::warn!("Update server did not return Date header; falling back to local time");
                Ok(local_now)
            }
        }
    }
}
```

**Clock Drift Handling (v1.15):**

CMOS bataryası ölen veya sistem saati kasten yanlış set edilen kullanıcı, v1.14'te updater'ı **kalıcı brick** edebiliyordu:
- Sistem saati 2019 → tüm `expires_at` geçmiş, hiçbir update validate olmaz
- Sistem saati 2035 → tüm `released_at < last_seen_released_at` olur (last_seen 2026), TimestampRollback hatası

**Çözüm: HTTP `Date` header referansı.**

Updater her zaman update server'dan HTTP response'un `Date` header'ını alır. Bu standart HTTP/1.1 header'ı, server'ın UTC zamanını döndürür. Eğer yerel saat ile server saati arasında **24 saatten fazla** sapma varsa:
1. Update reddedilir (güvenlik aleyhine değil — saat doğrulanamadıysa güvenlik kararı vermiyoruz)
2. Kullanıcıya **brick olmayan modal** çıkar: *"Sistem saatiniz yanlış (Yerel: 2019-03-15, Sunucu: 2026-05-11). Güvenlik kontrolleri için saatinizi düzeltin ve tekrar deneyin."*
3. Settings → "System time" panel link verilir (platform-specific: `timedatectl` Linux, "Date & Time" Windows)
4. Kullanıcı saati düzeltince auto-retry — brick yok

**Niye 24 saat cap:** Daylight saving (1 saat), timezone misconfiguration (birkaç saat), genuine NTP drift (saniyeler-dakikalar) tolerans içinde. 24 saat üstü = açık CMOS dead veya manipülasyon.

**Server Date header yok ise:** Kurumsal proxy bazen header strip eder. Bu durumda local time'a düşülür ama diagnostics'e warn yazılır. Replay attack riski hafif artar ama brick'ten iyidir.

**Anchor pubkey rotation:** Anchor key uygulamanın binary'sine compile-time gömülü. Anchor değişimi = forced major release (v1.x → v2.0). Bu sırada:
1. Eski binary "yeni anchor key" göremez
2. Yeni anchor key compile-time gömülü v2.0 binary'sini kullanıcı **out-of-band kanaldan** (download.dtransfer.app) yeniden indirmek zorunda
3. Auto-update v1.x → v2.0 mümkün değil — bilinçli güvenlik tasarımı

**TUF (The Update Framework) benzerlik:** Tam TUF değil ama temel kavramlar var: signed manifest, expiry, monotonic version, root-of-trust anchor. Tam TUF (role separation, snapshot/timestamp metadata) v1.1+'da adapter supply chain ile birleşik (v2+ scope).

### Orphan Chunk Temizliği

Başlangıçta `~/.dtransfer/tmp/`:
- 24 saatten eski geçici dosyalar silinir
- .dtresume olmayan .dtransfer_tmp silinir
- Tamamlanmış transfer klasörleri silinir

### Operational Recovery Playbooks

Engine "ne garanti veriyoruz" tarafı güçlü (Bölüm 29 Failure Semantics). Ama **kullanıcı ne görür, support ne yapar** tarafı production maturity göstergesi. Recovery anında kullanıcıya net mesaj verilmezse "uygulama bozuldu" hissi oluşur — teknik olarak doğru çalışsa bile.

**Recovery event → kullanıcı görünümü tablosu:**

| Engine Event | Kullanıcıya Gösterilen | UI Element |
|---|---|---|
| Queue DB corruption (integrity_check fail) | "Kuyruk veritabanı tutarsız bulundu — sıfırlandı, X yarım transfer kurtarıldı, Y orphan chunk temizlendi" | Top banner (info, dismiss button) |
| Orphan cleanup (startup) | "3 yarım upload temizlendi (2.4 GB disk alanı geri kazanıldı)" | Notification center badge |
| S3 multipart abort (24+ hours stale) | "S3 sunucusunda 5 yarım yükleme timeout — temizleniyor…" | Status bar progress |
| TLS pin mismatch | "Sunucu sertifikası değişti — devam etmeden önce kontrol edin" + fingerprint compare | Modal (kritik, dismiss yok) |
| TLS cert expired | "Sertifika süresi dolmuş ([tarih]) — bağlantı reddedildi" | Modal + profile edit shortcut |
| Soft memory cap hit | "RAM dolu, yeni transferler beklemeye alındı" | Top banner (warning) |
| Hard memory cap hit | "Kritik bellek seviyesi — tüm transferler duraklatıldı" + "Yeniden başlat" / "Limitleri ayarla" | Modal (action gerekli) |
| Stall detected (2sn+) | "Sistem yanıt vermiyor — diagnostics yazılıyor" | Status bar warning + auto-recover after timeout |
| Updater rollback | "Yeni sürüm başarısız oldu, eski sürüm geri yüklendi" | Notification + restart prompt |
| Disk full (ENOSPC) | "Yer kalmadı — transfer beklemeye alındı, X GB ihtiyacı var" | Top banner + retry on space available |
| Network roaming (WiFi changed) | "Ağ değişti — yeniden bağlanılıyor…" | Status bar transient |
| Permission denied bulk (>10 files) | "12 dosya okuma izni nedeniyle atlandı — listeyi göster" | Notification + skipped files panel |

**Recovery UI bileşenleri:**

```typescript
interface RecoveryBanner {
  severity: 'info' | 'warning' | 'critical';
  message: string;            // i18n key
  details?: string;            // expandable
  actions: RecoveryAction[];   // kullanıcı butonu
  dismissible: boolean;
  auto_dismiss_ms?: number;    // info için 5sn
}

interface RecoveryAction {
  label: string;               // i18n
  action: 'retry' | 'open_settings' | 'show_diagnostics' | 'dismiss' | 'custom';
  destination?: string;        // route veya URL
}
```

**Support workflow:** Diagnostics bundle (Bölüm 30) içinde `recovery_events.ndjson` dosyası — son 30 günlük tüm recovery event'leri timestamp + tipiyle. Support ekibi bu dosyaya bakarak "kullanıcı 12 Mart'ta TLS pin sorunu yaşadı, 13'ünde memory cap'e takıldı" gibi pattern'leri görür, kullanıcı anlatmak zorunda kalmaz.

**Toast/Banner/Modal hiyerarşisi:**

- **Toast (3-5sn):** Geçici durumlar, action gerektirmez (ağ değişti yeniden bağlanılıyor)
- **Banner (top, dismissible):** Mevcut durum bildirimi, action opsiyonel (memory cap, orphan cleanup raporu)
- **Modal (kritik, dismiss kontrollü):** User action gerekli (TLS pin, hard memory cap, updater rollback restart)
- **Notification center:** Geçmiş event'ler, scrollable, audit niyetli

**i18n disiplini:** Tüm recovery mesajları `recovery/*.json` namespace'inde. Türkçe ve İngilizce paralel — örnek:

```json
{
  "queue_db_corruption": {
    "tr": "Kuyruk veritabanı tutarsız bulundu — sıfırlandı, {recovered} yarım transfer kurtarıldı",
    "en": "Queue database integrity check failed — reset, {recovered} pending transfers recovered"
  }
}
```

Plurals + interpolation (`{recovered}`) dahil. Hardcoded string yasak — Bölüm 20 i18n disiplini ile bağlantılı.

---

## 29. Failure Semantics

"Tam olarak ne garanti ediyoruz?" — production-grade yazılımın en kritik sorusu. Architecture iyi olsa bile garanti açık değilse regression çıkar, yeni katkıcı yanlış davranış implement eder, kullanıcıya verilen söz tutulamaz.

### 29.1 Garanti Tablosu

> **Bu tablo Failure Semantics'in canonical reference'ıdır.** Diğer bölümlerden (Crash Recovery 25, Operational Recovery Playbooks 25.X, Multipart 11.4 corruption recovery, Queue 12.1 idempotency) buraya link verilir. Tabloya yeni satır eklemek = yeni chaos test eklemek (Bölüm 29.4) demektir.

Her event'in sonucunda **uygulamanın kullanıcıya verdiği söz**. Bu tablo regression test gate'idir — ihlal edilen satır PR bloklar.

| Event | Garanti |
|---|---|
| Crash during download | Temp dosya korunur (`.dtresume` + chunk'lar diskte). Yeniden başlatınca kalan yerden devam |
| Crash during chunk write | O chunk yarım yazılmış, atomic temp dosyası `_temp_<chunk_id>` olarak kalır. Engine başlangıçta orphan tarama → temp sil + ResumeChunk state=Pending |
| Crash during atomic rename | Eski dosya sağlam (rename atomic). Yeni dosya temp adıyla kalır, orphan cleaner siler |
| Crash during DB write | DbActor mpsc kanal'ı kuyruğunda backlog kayıp. WAL mode + checkpoint sayesinde önceki commit'ler durable. Son 5sn'lik bytes_done kaybı kabul (Bölüm 15.3) |
| Chunk corruption (hash mismatch) | Sadece o chunk yeniden indirilir (Bölüm 14.5 per-chunk hash). 10GB dosya için tek chunk = 8MB tekrar |
| Queue replay (restart) | Tüm `Pending` ve `Running` task'lar kaldığı yerden devam. Idempotent: aynı task iki kere çalışmaz (queue.db `unique_idx_transfer`) |
| Partial S3 multipart failure | UploadId saklanır, abort edilmez. 24 saat içinde resume mümkün. 7 gün sonra otomatik abort scheduler |
| S3 abort failed (network) | UploadId queue.db `orphan_uploads` tablosunda persist. Sonraki başlatmada retry abort |
| Remote delete fail | Local consistency korunur — lokal dosya silinmez, transfer state `Failed` ile işaretlenir, retry policy uygulanır |
| Updater rollback during launch | Eski binary korunur, signature verify edilir. Rollback başarısızsa minimum stable build (factory) yüklenir |
| Power loss during fsync | DataOnly mode: dosya içeriği durable, metadata kaybolabilir (rename round-trip yapılır restart'ta). Full mode: hem içerik hem metadata durable (Bölüm 14.6) |
| Network partition mid-upload | TCP timeout → `ConnectionLost` event → exponential backoff retry (Bölüm 16). Resume mümkünse devam |
| Disk full during write | `ENOSPC` → transfer pause (`Stalled`). Kullanıcı yer açınca devam, partial dosya korunur |
| Cert validation fail | Bağlantı düşürülür, `TlsPinMismatch` veya `CertExpired` urgent event (Bölüm 33.1) — sessizce devam ETMEZ |
| Rate limit hit | Adaptive backoff (Bölüm 16.2), transfer `Backoff` durumuna geçer, retry_in_ms event ile kullanıcıya bildirilir |

### 29.2 Idempotency Kuralları

**Aynı task iki kere çalıştırılmamalı.** Queue replay sonrası bu garantili olmalı:

```rust
// queue.db
CREATE UNIQUE INDEX unique_idx_transfer
ON transfers(profile_id, source_path, dest_path, transfer_kind);
```

**Replay senaryosu:**
1. Restart sonrası `recover_pending_tasks()` queue.db'den okur
2. State machine `Running → Pending` rollback (orphan chunks tarama da yapılır)
3. Scheduler picker `Pending` task'ı al → yeni Running marker + actor command
4. Eğer aynı task daha önce kısmen çalıştıysa (`bytes_done > 0` ve chunk klasörü dolu): resume akışına gir, sıfırdan başlama

### 29.3 Orphan Cleanup Stratejisi

Restart sonrası `recover_orphans()` task'ı çalışır:

```rust
pub struct OrphanCleaner {
    queue_dir: PathBuf,    // chunk dosyalarının olduğu yer
    db: DbHandle,
}

impl OrphanCleaner {
    pub async fn run(&self) -> Result<CleanupReport> {
        let active_chunks = self.db.list_active_chunks().await?;
        let on_disk = list_chunks_on_disk(&self.queue_dir).await?;
        // Diskte var ama DB'de yok → orphan
        let orphans: Vec<_> = on_disk.iter()
            .filter(|f| !active_chunks.contains(&f.chunk_id))
            .collect();
        for orphan in &orphans {
            tracing::info!(chunk = ?orphan, "removing orphan chunk");
            tokio::fs::remove_file(&orphan.path).await.ok();
        }
        // S3 orphan multipart abort
        self.abort_stale_multiparts().await?;
        Ok(CleanupReport { removed: orphans.len() })
    }
}
```

**Çalışma zamanı:** App startup'ta engine başlamadan önce. Diagnostics bundle'a sayım yazılır.

### 29.4 Test Edilebilirlik — Chaos Testing

Failure semantics garantilerini doğrulamak için test framework:

```rust
// tests/chaos/crash_during_rename.rs
#[tokio::test]
async fn crash_during_rename_preserves_old_file() {
    let env = TestEnv::new();
    env.write_local("data.bin", b"OLD CONTENT").await;
    env.queue_download("data.bin");
    env.advance_to(TransferStage::AtomicRenamePending).await;

    // Inject crash: process abort sırasında rename
    env.crash_at(CrashPoint::JustBeforeRename).await;

    // Restart
    let app = env.restart().await;
    app.recover_orphans().await;

    // Garanti: eski dosya hâlâ orada
    assert_eq!(env.read_local("data.bin").await, b"OLD CONTENT");
    // Garanti: yeni dosya temp adıyla mevcut, orphan cleaner sildi
    assert!(!env.exists_local("data.bin._tmp_*").await);
}
```

**CrashPoint enum:** `JustBeforeRename`, `MidChunkWrite`, `AfterDbCommitBeforeFlush`, `DuringS3Multipart` — her biri spesifik failure mode'unu test eder.

`feature = "chaos-testing"` flag'i altında, sadece `cargo test` ile aktif. Production binary'de yok.

### 29.5 Garanti İhlali = PR Block

CI'da `tests/chaos/` klasörü her PR'da koşar. Failure semantics tablosunda yazılı her satır için en az bir chaos test'i olmalı. Test eksikse PR review'da `chaos-coverage-missing` etiketi ile bloklanır.

---

## 30. Diagnostics Bundle

Telemetry göndermez — ama yerel diagnostics paketi destek süreçlerini kolaylaştırır.

```
Help → Export Diagnostics Bundle → dtransfer-diag-{timestamp}.zip
```

Paket içeriği (tümü yerel, hiçbiri otomatik gönderilmez):

```
dtransfer-diag-20260509T1432/
├── app_info.json           { version, os, arch, tauri_version }
├── config_snapshot.json    { profil isimleri, protokol türleri — credential YOK }
├── queue_state.json        { aktif/bekleyen transfer sayısı, hata listesi }
├── capability_probes.json  { SFTP sunucu sonuçları }
├── adapter_status.json      { aktif adapter'lar (sftp/s3/webdav/local), son hata }
├── rate_limit_state.json   { host başına mevcut limit durumu }
├── runtime_metrics.json    { RuntimeMetrics snapshot (aşağıda) }
├── runtime_limits.json     { configured RuntimeLimits + actual usage }
├── stalls.ndjson           { stall watchdog event'leri (varsa) }
├── logs/
│   ├── app.log             { son 1000 satır, presigned URL redacted }
│   └── crash.log           { son çökme izi — varsa }
└── README.txt              { bu dosyanın içeriği ve gizlilik notu }
```

**Gizlilik:** Dosya adları, IP adresleri, kimlik bilgileri, presigned URL'ler pakete dahil edilmez.

### 30.1 RuntimeMetrics

"Uygulama yavaş" demekle debug edilemez. Structured metrics layer ile cevaplanabilir sorular:

```rust
pub struct RuntimeMetrics {
    // Transfer
    pub active_transfers:           u32,
    pub queued_transfers:           u32,
    pub completed_last_hour:        u32,
    pub failed_last_hour:           u32,
    pub inflight_bytes:             u64,
    pub bytes_transferred_total:    u64,

    // Per-connection
    pub avg_chunk_latency_ms:       u64,
    pub p50_chunk_latency_ms:       u64,
    pub p99_chunk_latency_ms:       u64,
    pub active_connections:         u32,
    pub connections_in_pool:        u32,

    // DB
    pub db_queue_depth:             usize,    // DbActor mpsc backlog
    pub db_avg_write_latency_ms:    u64,
    pub db_size_mb:                 u64,
    pub db_wal_size_mb:             u64,

    // Tokio runtime
    pub tokio_workers_busy:         u32,
    pub tokio_blocking_threads:     u32,
    pub tokio_tasks_alive:          u32,
    pub stall_events_24h:           u32,

    // System
    pub memory_rss_mb:              u64,
    pub open_file_descriptors:      u32,
    pub thread_count:               u32,
}
```

Bu struct her 5 saniyede bir güncellenir, ring buffer'da son 1 saatlik veri tutulur, diagnostics export'unda son snapshot + 1 saatlik trend dahil edilir.

UI tarafında power user için "About → Runtime Metrics" panel ile gösterilebilir (v1.0'da gizli, advanced toggle ile açılır).

### 30.2 RuntimeLimits

Production-grade robustness için kaynak sınırları açıkça tanımlı olmalı. 500k file transfer / millions of tiny files / low-RAM VPS senaryolarında sistem ne yaparsa yapsın çökmemeli — graceful degrade etmeli.

```rust
pub struct RuntimeLimits {
    /// Toplam aynı anda açık dosya descriptor sınırı
    pub max_open_files:        usize,    // default: 4096 (OS limit'in altında)

    /// Aynı anda inflight chunk sayısı (tüm transferler toplam)
    pub max_inflight_chunks:   usize,    // default: 256

    /// Toplam inflight byte (memory pressure)
    pub max_inflight_bytes:    u64,      // default: 512 MB

    /// Aynı anda aktif transfer sayısı
    pub max_concurrent_transfers: u32,   // default: 16

    /// Aynı host'a açık eş zamanlı bağlantı
    pub max_connections_per_host: u32,   // default: 8

    /// DbActor mpsc backlog (dolulukta producer bekler)
    pub max_db_command_backlog: usize,   // default: 1024

    /// Soft memory cap (bu değere ulaşınca yeni transfer pause)
    pub soft_memory_cap_mb:    u64,      // default: 600 MB

    /// Hard memory cap (kritik — graceful shutdown trigger)
    pub hard_memory_cap_mb:    u64,      // default: 1500 MB
}
```

**Davranış:** Limit ihlali = transfer pause, error değil. Kullanıcı "RAM dolu, sıraya alındı" görür. Limit aşılan ana göre:

| Limit | Aşıldığında |
|---|---|
| `max_open_files` | Yeni connection açma, mevcutleri reuse et |
| `max_inflight_chunks` | Yeni chunk başlatma, biten chunk'ı bekle |
| `max_inflight_bytes` | Aynı (byte-bazlı backpressure) |
| `max_concurrent_transfers` | Yeni transfer pending'de kal, queue'da bekle |
| `max_connections_per_host` | Pool wait — semaphore (Bölüm 9'da) |
| `max_db_command_backlog` | Producer await'te bekler (mpsc kanalı dolu) |
| `soft_memory_cap_mb` | Yeni transfer kabul etme, mevcutleri tamamla |
| `hard_memory_cap_mb` | Tüm transfer'leri pause et, kullanıcı uyar — son çare |

**Test edilebilirlik:** `RuntimeLimits` profile bazlı override edilebilir — düşük RAM VPS profili `soft_memory_cap_mb: 200` ile başlar. Chaos test'lerde küçük limit'ler ile graceful degrade davranışı verify edilir.

```rust
// Memory cap monitor — her 5sn'de RSS kontrol
async fn memory_monitor(limits: RuntimeLimits, app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let rss_mb = current_rss_mb();
        if rss_mb >= limits.hard_memory_cap_mb {
            tracing::error!(rss_mb, "hard memory cap reached, pausing all");
            app.emit("engine:urgent", &EngineEvent::MemoryCapHit { rss_mb }).ok();
            pause_all_transfers().await;
        } else if rss_mb >= limits.soft_memory_cap_mb {
            stop_accepting_new_transfers();
        }
    }
}
```

### 30.3 LimitProfile (Adaptive Limits)

8GB RAM laptop ile 128GB workstation aynı limitlerle çalışmamalı. Static `RuntimeLimits` ChatGPT'nin haklı eleştirisi — system probe ile adaptive olmalı.

```rust
pub enum LimitProfile {
    /// 4GB RAM altı VPS / eski laptop / Raspberry Pi
    LowMemory,
    /// 8-16GB RAM, tipik laptop / desktop (default)
    Desktop,
    /// 32GB+ RAM, developer/enterprise workstation
    Workstation,
    /// 64GB+ RAM, headless / batch transfer server
    Server,
    /// Kullanıcı manuel ayarladı
    Custom(RuntimeLimits),
}

impl LimitProfile {
    pub fn detect() -> Self {
        let total_ram_gb = system_total_ram_mb() / 1024;
        let fd_ulimit = current_fd_limit();
        let cpu_cores = num_cpus::get();
        let is_headless = std::env::var("DISPLAY").is_err()
            && std::env::var("WAYLAND_DISPLAY").is_err();

        match (total_ram_gb, is_headless, cpu_cores) {
            (r, _, _) if r < 4 => Self::LowMemory,
            (r, true, _) if r >= 32 => Self::Server,
            (r, _, _) if r >= 32 => Self::Workstation,
            _ => Self::Desktop,
        }
    }

    pub fn to_limits(&self) -> RuntimeLimits {
        match self {
            Self::LowMemory => RuntimeLimits {
                max_open_files:         512,
                max_inflight_chunks:    32,
                max_inflight_bytes:     64 * 1024 * 1024,    // 64 MB
                max_concurrent_transfers: 4,
                max_connections_per_host: 2,
                max_db_command_backlog: 128,
                soft_memory_cap_mb:     150,
                hard_memory_cap_mb:     400,
            },
            Self::Desktop => RuntimeLimits {
                max_open_files:         4096,
                max_inflight_chunks:    256,
                max_inflight_bytes:     512 * 1024 * 1024,   // 512 MB
                max_concurrent_transfers: 16,
                max_connections_per_host: 8,
                max_db_command_backlog: 1024,
                soft_memory_cap_mb:     600,
                hard_memory_cap_mb:     1500,
            },
            Self::Workstation => RuntimeLimits {
                max_open_files:         8192,
                max_inflight_chunks:    1024,
                max_inflight_bytes:     2 * 1024 * 1024 * 1024,  // 2 GB
                max_concurrent_transfers: 64,
                max_connections_per_host: 16,
                max_db_command_backlog: 4096,
                soft_memory_cap_mb:     2400,
                hard_memory_cap_mb:     6000,
            },
            Self::Server => RuntimeLimits {
                max_open_files:         16384,
                max_inflight_chunks:    4096,
                max_inflight_bytes:     8 * 1024 * 1024 * 1024,  // 8 GB
                max_concurrent_transfers: 256,
                max_connections_per_host: 32,
                max_db_command_backlog: 16384,
                soft_memory_cap_mb:     8000,
                hard_memory_cap_mb:     20000,
            },
            Self::Custom(limits) => limits.clone(),
        }
    }
}
```

**Override hierarchy:** App default = `LimitProfile::detect()` → user "Settings → Performance" override → per-profile override (özellikle headless server'da farklı transfer'ler farklı limitler ister) → Custom fine-grained.

**FD ulimit handling:** Linux'ta `ulimit -n` 1024 default, DTransfer startup'ta `setrlimit(RLIMIT_NOFILE)` ile artırmaya çalışır. Başarısızsa `max_open_files` ulimit'in %75'ine cap'lenir + log warn.

**Settings UI (Power user):** `LimitProfile` radio button + "Auto-detect" varsayılan. Custom seçilince tüm field'lar editable, "Reset to detected" butonu.

### 30.4 spawn_blocking Pool — Profile-Aware Sizing (v1.14)

Ana motorda `max_blocking_threads(256)` koymak Desktop profile'de yeterli ama Workstation'da yetersizdir. Workstation profile'inde `max_concurrent_transfers: 64`, her transfer paralel 4 chunk = 256 spawn_blocking peak. Pool tam dolu, yeni hash queue'da bekler.

Çözüm: pool boyutu da `LimitProfile`'a göre build edilir:

```rust
impl LimitProfile {
    pub fn build_tokio_runtime(&self) -> tokio::runtime::Runtime {
        let blocking_threads = match self {
            Self::LowMemory   => 64,
            Self::Desktop     => 256,
            Self::Workstation => 512,
            Self::Server      => 1024,
            Self::Custom(l)   => (l.max_concurrent_transfers as usize * 8).max(64),
        };
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_cpus::get())
            .max_blocking_threads(blocking_threads)
            .thread_name("dtransfer-worker")
            .enable_all()
            .build()
            .expect("runtime build")
    }
}
```

**Memory hesabı:** Her blocking thread ~512KB stack. 1024 thread = ~512MB peak. Server profile bunu absorbe edebilir (32GB+ RAM tipik), LowMemory edemez (4-8GB) — bu yüzden profile-aware şart.

**Stack size düşürme (deneysel, v1.1):** Hash hesaplama gibi stack-light işler için custom thread stack size `.thread_stack_size(256 * 1024)`. ⚠️ async-trait method'ları derin call-stack üretebilir, panic-with-stack-overflow riski; test edilmeden v1.0'da uygulanmaz.

**RuntimeLimits ek field:**

```rust
pub struct RuntimeLimits {
    // ... existing fields ...
    pub max_blocking_threads: usize,        // v1.14
    pub s3_pool_max_idle_per_host: u32,     // v1.14 (Bölüm 11.5)
}
```

### 30.5 Log Retention & Crash-Loop Spam Koruması (v1.16)

Diagnostics bundle çok detaylı log tutar — Bölüm 33 EngineEvent her şeyi bus'a yazar, tracing span'leri 32. bölümdeki internal tracing'i besler. Retention policy olmazsa `AppData` içinde **20GB+** log birikir.

**Aktif dosya rotation (Bölüm 33 DiagnosticsBuffer'da spec'lendi):**

- `max_file_size_bytes`: 50MB (rotation trigger)
- `max_file_count`: 10 (toplam ~500MB)
- `max_file_age_days`: 30
- `total_cap_bytes`: 1GB hard cap

**Cap aşıldığında strateji:**

```rust
impl DiagnosticsBuffer {
    async fn enforce_total_cap(&mut self) {
        let total = self.list_files().await.iter().map(|f| f.size).sum();
        if total < self.rotation_policy.total_cap_bytes { return; }

        // En eski dosyaları sil, total cap altına in
        let mut files = self.list_files().await;
        files.sort_by_key(|f| f.modified_at);
        let mut current = total;
        for file in files {
            if current < self.rotation_policy.total_cap_bytes * 9 / 10 { break; }
            tokio::fs::remove_file(&file.path).await.ok();
            current -= file.size;
            tracing::warn!(path = %file.path.display(), "rotated out due to total cap");
        }
    }
}
```

**Crash-loop spam koruması:**

Sıkça karşılaşılan senaryo: bir bug yüzünden uygulama her başlangıçta crash → kullanıcı tekrar açar → crash → tekrar... 100 crash = 100 stack trace + 100 startup log. Aynı içerik tekrarlanır, log dosyası şişer ama bilgi artmaz.

**Çözüm: deduplication on stable hash.**

```rust
pub struct CrashLoopDedup {
    seen: HashMap<u64, CrashInstance>,           // stack_trace_hash → instance count
    last_flush: Instant,
}

pub struct CrashInstance {
    pub first_seen:  DateTime<Utc>,
    pub last_seen:   DateTime<Utc>,
    pub count:       u32,
    pub stack_trace: String,                      // sadece bir kez saklanır
    pub context:     String,                      // değişebilir, son seen'in context'i
}

impl CrashLoopDedup {
    pub fn record_crash(&mut self, stack_trace: &str, context: &str) {
        let hash = xxh3_64(stack_trace.as_bytes());
        self.seen.entry(hash)
            .and_modify(|inst| {
                inst.count += 1;
                inst.last_seen = Utc::now();
                inst.context = context.to_string();
            })
            .or_insert(CrashInstance {
                first_seen:  Utc::now(),
                last_seen:   Utc::now(),
                count:       1,
                stack_trace: stack_trace.to_string(),
                context:     context.to_string(),
            });
    }

    /// Log'a yazarken: tek tek event yerine özet
    pub fn flush_to_log(&mut self) -> Vec<CrashSummaryEvent> {
        self.seen.drain().map(|(_, inst)| CrashSummaryEvent {
            stack_trace: inst.stack_trace,
            count: inst.count,
            first_seen: inst.first_seen,
            last_seen: inst.last_seen,
            last_context: inst.context,
        }).collect()
    }
}
```

**Log entry formatı (crash-loop):**

```ndjson
{"event":"CrashSummary","stack_trace":"...","count":47,"first_seen":"...","last_seen":"...","note":"47× repeated within 12 minutes; logged as single summary entry"}
```

100 ayrı crash event yerine 1 özet entry. Bilgi korunur (count + zaman aralığı), disk kullanımı 100x azalır.

**Test/development override:** `DTRANSFER_DEDUP_DISABLE=1` env var ile dedup kapatılır — her crash event raw yazılır, debugging için.

---

## 31. Telemetry Policy

**DTransfer hiçbir telemetry göndermez.**

- Kullanım istatistiği yok
- Crash raporu yok (opt-in bile değil)
- Transfer metadata uzak sunucuya gönderilmez
- Güncelleme kontrolü: yalnızca `registry.json` fetch (tek yönlü)

---

## 32. CancellationToken Standardizasyonu

Tüm iptal mekanizmaları tek `CancellationToken` üzerinden yönetilir. Queue cancel, app shutdown, retry abort, adapter disconnect, profile disconnect — hepsi aynı sisteme bağlanır.

```rust
use tokio_util::sync::CancellationToken;

pub struct TransferScope {
    pub transfer_id: Uuid,
    pub token:       CancellationToken,  // bu transfer'a özel
}

pub struct AppScope {
    pub token: CancellationToken,  // uygulama geneli shutdown
}

// Hiyerarşi:
// AppScope.token (root)
//   └── ProfileScope.token (bağlantı kopar → tüm transferler iptal)
//         └── TransferScope.token (tek transfer iptal)
//               └── ChunkScope.token (tek chunk iptal)

impl TransferEngine {
    pub async fn upload(
        &self,
        scope: &TransferScope,
        // ...
    ) -> Result<TransferResult, TransferError> {
        tokio::select! {
            result = self.do_upload() => result,
            _ = scope.token.cancelled() => {
                Err(TransferError::Cancelled)
            }
        }
    }
}
```

**Kullanım noktaları:**

| Olay | Token seviyesi |
|---|---|
| Kullanıcı "İptal" tıklar | `TransferScope.token.cancel()` |
| Bağlantı kopar / profile silinir | `ProfileScope.token.cancel()` |
| Uygulama kapanır | `AppScope.token.cancel()` |
| Retry abort (max retry aşıldı) | `TransferScope.token.cancel()` |

Token child → parent zinciri: üst token iptal olunca tüm alt token'lar otomatik iptal olur. `CancellationToken::child_token()` ile oluşturulur.

### 32.1 Cancellation Guarantee Boundary (v1.16)

`CancellationToken.cancel()` çağrısı yapıldığında transfer her aşamada hemen durur mu? Hayır — bazı işlemler atomik olarak tamamlanmalıdır. Kullanıcı "iptal ettim, neden hâlâ çalışıyor?" sormaması için **cancel davranışı her stage için açıkça yazılı** olmalı.

**Per-stage cancel davranış sözleşmesi:**

| Aşama | Cancel davranışı | Worst-case latency | Sebep |
|---|---|---|---|
| TCP socket read/write | **Immediate** | <100ms | `tokio::select!` ile race, socket drop |
| TLS handshake | **Immediate** | <500ms | Handshake task'i drop edilir, partial state cleanup |
| HTTP request (idle) | **Immediate** | <100ms | reqwest cancel-safe drop |
| HTTP request (body streaming) | **Immediate** | <chunk_size / network_speed | Connection drop, next chunk download iptal |
| SFTP channel operation | **Immediate** | <100ms | russh channel close |
| Decrypt single chunk (in `spawn_blocking`) | **Cooperative** | <30ms (8MB @ 1.5GB/s XChaCha20) | spawn_blocking görev tamamlanır, yeni chunk başlamaz |
| Per-chunk hash (in `spawn_blocking`) | **Cooperative** | <100ms (8MB @ 80MB/s SHA-256) | Aynı pattern, hash görev tamamlanır |
| Disk write (tokio::fs) | **Cooperative** | OS-dependent (~50ms typical, GB-class SSD) | tokio::fs blocking pool'da, started write tamamlanır |
| Atomic rename (`tokio::fs::rename`) | **Deferred** | <100ms; AV lock varsa Bölüm 14.2 micro-retry içinde de cancellable | Syscall atomik, mid-syscall iptal yok |
| File fsync (data) | **Deferred** | OS-dependent (1-1000ms) | Disk durability garantisi, başlamış fsync tamamlanmalı |
| File fsync (parent dir) | **Deferred** | <100ms | POSIX rename durability için şart |
| `.dtresume` write (atomic write_to_temp + rename) | **Deferred (non-cancellable critical section)** | <100ms | Cancel mid-write → resume state corrupt, asla cancel etme |
| DbActor `Checkpoint` komutu | **Non-cancellable** | <5sn (WAL truncate) | Race condition'ı engellemek için commit atomik |
| DbActor `StateTransition` (Active→Completed) | **Non-cancellable** | <100ms | Queue persistence consistency için |
| Presigned URL refresh request | **Cooperative** | <2sn (HTTP roundtrip) | Refresh task cancel olur ama in-flight HTTP tamamlanır |
| Chunk hash blob (chunkmap.blob) flush | **Deferred** | <50ms | Atomic write_to_temp + rename, mid-write iptal data corrupt |

**Davranış kategorileri:**

- **Immediate (<500ms):** Network I/O, idle TLS, socket ops. `tokio::select!` ile race, ek state cleanup yok.
- **Cooperative (<1sn):** spawn_blocking görev tamamlanır, yeni iş başlamaz. Tipik chunk-level granülerite.
- **Deferred (<5sn):** Atomik syscall'lar (fsync, rename, blob write). Mid-syscall cancel yok; ama syscall sonrası kontrol noktasında abort.
- **Non-cancellable (kritik):** Transaction commit, state transition, resume write. Cancel ignore edilir, işlem tamamlanır, sonra abort.

**Toplam cancel latency garanti:**

Tek dosya transfer'inde, kullanıcı "İptal" tıkladığından "Transfer Cancelled" görene kadar **<10 saniye** (worst case). Tipik: <2sn. Şu durumda 10sn'i aşabilir:
- Çok büyük chunk + yavaş disk + fsync zamanı (sparse 1GB chunk fsync 8sn alabilir SATA HDD'de)
- Çok yavaş adapter RPC (timeout 30sn'ye set'li → cancel'ı respect etmesi 30sn alır)

Bu durumlarda UI'da *"İptal işleniyor (8sn)..."* sub-status gösterilir (TransferActiveSubState extension).

**Implementation pattern:**

```rust
pub async fn transfer_chunk(
    chunk: ChunkInfo,
    scope: TransferScope,
) -> Result<(), TransferError> {
    // Immediate-cancellable: network read
    let encrypted = tokio::select! {
        result = network_read_chunk(&chunk) => result?,
        _ = scope.token.cancelled() => return Err(TransferError::Cancelled),
    };

    // Cooperative-cancellable: decrypt (check before/after, not during)
    if scope.token.is_cancelled() { return Err(TransferError::Cancelled); }
    let plaintext = tokio::task::spawn_blocking(move || decrypt(encrypted)).await??;
    if scope.token.is_cancelled() { return Err(TransferError::Cancelled); }

    // Cooperative: hash
    let hash = tokio::task::spawn_blocking(move || sha256(&plaintext)).await?;

    // Deferred: disk write (started write completes)
    write_chunk_to_disk(&plaintext, &chunk).await?;

    // Non-cancellable critical section: resume state update
    // Even if cancel arrives, this must complete to avoid corrupt .dtresume
    let _ = scope.token.cancelled();  // ignore cancel during this block
    update_resume_state(chunk.index, ChunkState::Completed).await?;
    Ok(())
}
```

Diagnostics: `cancel_latency_histogram` metric — kullanıcı cancel'ından gerçek abort'a kadar geçen süreyi ölç, p99 SLA'sı 10sn olmalı. Aşıyorsa bug, investigation gerekli.

---

## 33. Unified EngineEvent Bus

Progress, log, queue state, adapter state şu an farklı kanallardan akıyor. Tek `EngineEvent` enum tüm katmanları sadeleştirir. Diagnostics bundle, UI, internal tracing hepsi bu bus'tan beslenir.

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub enum EngineEvent {
    // Transfer
    TransferProgress {
        transfer_id: Uuid,
        bytes_done:  u64,
        bytes_total: u64,
        speed_bps:   f64,
        eta_secs:    Option<u64>,
    },
    TransferStateChanged {
        transfer_id: Uuid,
        old_state:   TransferState,
        new_state:   TransferState,
    },
    TransferCompleted {
        transfer_id: Uuid,
        checksum:    String,
        duration_ms: u64,
    },
    TransferFailed {
        transfer_id: Uuid,
        error:       TransferError,
        retry_in_ms: Option<u64>,
    },

    // Ağ / API
    RateLimited {
        profile_id:      Uuid,
        retry_after_secs: u64,
    },
    ConnectionLost { profile_id: Uuid },
    ConnectionRestored { profile_id: Uuid },

    // Adapter
    // Queue
    QueueRecovered { restored_count: usize },
    QueueDrained,

    // Sistem
    AppShutdownInitiated,
    DiagnosticsFlushed { path: PathBuf },
}

pub struct EventBus {
    /// UI ve external subscriber'lar için (lossy backpressure tolere edilebilir)
    ui_tx:          broadcast::Sender<Arc<EngineEvent>>,
    /// Diagnostics buffer için (event loss kabul edilemez)
    diagnostics_tx: mpsc::UnboundedSender<Arc<EngineEvent>>,
}

impl EventBus {
    pub fn emit(&self, event: EngineEvent) {
        let event = Arc::new(event);                              // tek allocation
        // Diagnostics — refcount++, deep clone yok
        let _ = self.diagnostics_tx.send(Arc::clone(&event));
        // Tracing span
        tracing::debug!(event = ?event, "engine_event");
        // UI broadcast — refcount++, deep clone yok
        let _ = self.ui_tx.send(event);
    }

    pub fn subscribe_ui(&self) -> broadcast::Receiver<Arc<EngineEvent>> {
        self.ui_tx.subscribe()
    }
}
```

> **v1.14 allocation reduction:** v1.13'te `broadcast::send` ve `mpsc::send` her seferinde `EngineEvent` clone yapıyordu — `String` field'lar (`reason`, `plugin_id`, `error_last`) heap allocation üretir. 6 subscriber × 1000 event/sec × String clone = sürekli allocator pressure. Yüksek transfer hızında glibc malloc contention oluşur. Çözüm: `Arc<EngineEvent>` ile sender'larda refcount++ (cheap), subscriber'lar `Arc<EngineEvent>` alır. Trade-off: subscriber'lar `&EngineEvent` yerine `Arc<EngineEvent>` ile çalışır, ufak ergonomi kaybı. Allocation profili çok daha temiz.

> **Static error reasons:** `TransferError::Authentication { reason: String }` yerine sık görülen reason'lar için enum variant. `Other(String)` allocation yapar ama %95 durumda allocation'sız variant kullanılır:
> ```rust
> pub enum AuthFailReason {
>     InvalidCredentials,
>     TokenExpired,
>     MfaRequired,
>     AccountLocked,
>     Other(String),                    // sadece beklenmedik durumlar
> }
> ```

> **v1.13 errata (broadcast → mpsc split):** v1.12'de tek `broadcast::Sender<EngineEvent>` kullanılıyordu. `tokio::sync::broadcast` slow subscriber'da **en eski event'i overwrite eder**. Eğer DiagnosticsBuffer disk I/O'su yavaşlarsa (büyük log rotation sırasında, disk full öncesi, virüs taraması sırasında) sessizce event kaybedilir. Sonradan diagnostics bundle açıldığında "bu olay neden yok?" sorusu cevapsız kalır — gerçek bug'lar görünmez. Çözüm: **diagnostics için ayrı `mpsc::unbounded_channel`**, broadcast sadece UI/external (lossy tolere edilebilir) subscriber'lar için.
>
> Unbounded mpsc memory unbounded olabilir teorik olarak; pratikte tek tüketici (DiagnosticsBuffer) sürekli draining yapar. Patolojik durumda (disk gerçekten dolu) Operational Recovery Playbook devreye girer (Bölüm 28): RuntimeLimits hard memory cap aşılırsa diagnostics drop edilir + kullanıcıya banner gösterilir.

**Diagnostics tüketici tarafı:**

```rust
pub struct DiagnosticsBuffer {
    rx:              mpsc::UnboundedReceiver<Arc<EngineEvent>>,
    writer:          BufWriter<File>,           // diagnostics/events.ndjson
    rotation_policy: RotationPolicy,
}

pub struct RotationPolicy {
    pub max_file_size_bytes: u64,        // default 50MB — rotation trigger
    pub max_file_count:      u32,        // default 10 — toplam max 500MB
    pub max_file_age_days:   u32,        // default 30 — yaşlı dosyalar otomatik silinir
    pub total_cap_bytes:     u64,        // hard cap: default 1GB
    pub crash_loop_dedup:    bool,       // default true — aynı stack trace 100 kez → "100× repeated" özet
}

impl DiagnosticsBuffer {
    pub async fn run(mut self, cancel: CancellationToken) {
        loop {
            tokio::select! {
                Some(event) = self.rx.recv() => {
                    let line = serde_json::to_string(&*event).unwrap_or_default();
                    let _ = self.writer.write_all(line.as_bytes()).await;
                    let _ = self.writer.write_all(b"\n").await;
                    self.maybe_rotate().await;
                }
                _ = cancel.cancelled() => {
                    let _ = self.writer.flush().await;
                    return;
                }
            }
        }
    }

    async fn maybe_rotate(&mut self) {
        let current_size = self.writer.get_ref().metadata().await.map(|m| m.len()).unwrap_or(0);
        if current_size < self.rotation_policy.max_file_size_bytes { return; }

        // 1. Mevcut dosyayı kapat, timestamp ile rename: events-2026-05-11T15-30-22.ndjson
        self.rotate_current_file().await;

        // 2. Count cap enforce: en eski dosyaları sil
        self.enforce_count_cap().await;

        // 3. Age cap enforce: 30+ gün eski dosyaları sil
        self.enforce_age_cap().await;

        // 4. Total cap enforce: toplam disk kullanımı > 1GB ise en eski(ler)i sil
        self.enforce_total_cap().await;
    }
}
```


**Aboneler:**

| Abone | İlgilendiği event'ler |
|---|---|
| EngineEventAggregator | Çoğu event (batch'lenir) |
| QueueScheduler | `TransferStateChanged`, `TransferFailed` |
| DiagnosticsBuffer | Tümü |
| Tauri emit (UI) | EngineEventAggregator çıktısı + urgent express lane |
| RateLimiter | `RateLimited` |
| InternalTracing | Tümü (span annotation) |

### 33.1 EngineEventAggregator — Batch + Urgent Express Lane

`tauri::emit` her çağrıda Rust → WebView IPC köprüsünü geçirir (JSON serialize + Tauri internal mutex + WebView postMessage). 1000 küçük dosya 5 saniyede tamamlandığında 1000 ayrı `TransferCompleted` emit = 200/sn IPC fırtınası → UI freeze.

`ProgressAggregator` (bölüm 6.4) sadece `TransferProgress`'i throttle eder. State değişimleri, tamamlanmalar, hatalar tek tek emit edilirse bypass olur. Çözüm: aggregator'ı **EngineEventAggregator**'a evrilt, **iki kanal** sun:

```rust
pub enum EmitChannel {
    /// Batch — 250ms throttle, UI gözle görülür gecikme tolere edebilir
    /// (progress, state change, completed, failed)
    Batched,
    /// Urgent — anlık emit, kullanıcı input'u bekleyen prompt'lar
    /// (TLS pin onayı, password expired, adapter auth flow, rate limit warning)
    Urgent,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineEventBatch {
    pub batch_id:     u64,                          // sıra korunması için
    pub progress:     Vec<ProgressUpdate>,          // bytes/speed/eta
    pub state_changes: Vec<StateChange>,            // Started/Paused/Completed/Failed
    pub errors:       Vec<TransferErrorEvent>,      // non-blocking errors
    pub queue:        Vec<QueueEvent>,              // queue drained/recovered
}

pub struct EngineEventAggregator {
    pending:   EngineEventBatch,
    batch_id:  u64,
    interval:  tokio::time::Interval,  // 250ms
    rx:        broadcast::Receiver<EngineEvent>,
}

impl EngineEventAggregator {
    pub async fn run(mut self, app: AppHandle) {
        loop {
            tokio::select! {
                _ = self.interval.tick() => {
                    self.flush_batch(&app).await;
                }
                event = self.rx.recv() => match event {
                    Ok(e) if e.is_urgent() => {
                        // Express lane: aggregator'ı bypass et, anında emit
                        app.emit("engine:urgent", &e).ok();
                    }
                    Ok(e) => {
                        // Batch'e ekle, 250ms tick'inde gönderilir
                        self.pending.push(e);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Subscriber yetişemedi, n event kaybı diagnostics'e log
                        tracing::warn!(lagged = n, "event_aggregator_lagged");
                    }
                    Err(_) => break,  // sender drop
                }
            }
        }
    }

    async fn flush_batch(&mut self, app: &AppHandle) {
        if self.pending.is_empty() { return; }
        self.batch_id += 1;
        self.pending.batch_id = self.batch_id;
        // Tek emit, tek IPC geçişi, tek JSON serialize
        app.emit("engine:batch", &self.pending).ok();
        self.pending.clear();
    }
}
```

**`is_urgent()` sınıflandırması:**

```rust
impl EngineEvent {
    pub fn is_urgent(&self) -> bool {
        matches!(self,
            EngineEvent::TlsPinPromptRequired { .. }
          | EngineEvent::PasswordExpired { .. }          | EngineEvent::RateLimited { .. }              // kullanıcı bilsin
          | EngineEvent::ConnectionLost { .. }           // session indicator          | EngineEvent::DiskSpaceLow { .. }
          | EngineEvent::CertificateMismatch { .. }      // güvenlik kritik
        )
    }
}
```

**UI tarafında iki listener:**

```typescript
// Vue store
import { listen } from '@tauri-apps/api/event';

// Batch — 250ms'de bir gelir, store'u tek seferde günceller
listen<EngineEventBatch>('engine:batch', ({ payload }) => {
  applyProgressUpdates(payload.progress);
  applyStateChanges(payload.state_changes);
  applyErrors(payload.errors);
  // tek render cycle, Vue reactivity tek seferde tetiklenir
});

// Urgent — anında modal/toast aç
listen<EngineEvent>('engine:urgent', ({ payload }) => {
  if (payload.type === 'TlsPinPromptRequired') openTlsPinModal(payload);
  else if (payload.type === 'RateLimited') showRateLimitToast(payload);
  // ...
});
```

**Performans karşılaştırması:**

| Senaryo | Eski (per-event emit) | Yeni (batch + urgent) |
|---|---|---|
| 1000 küçük dosya tamamlanma | ~1000 IPC geçişi, ~200/sn | ~20 batch IPC, ~4/sn |
| Tek transfer progress (1MB/s) | 4/sn (zaten throttled) | 4/sn (aynı) |
| TLS pin prompt | Anında | Anında (urgent) |
| Rate limit warning | Anında | Anında (urgent) |

UI güncellemeleri batch geldiğinde Vue'nun tek `nextTick` döngüsünde DOM'a yansır — 1000 ayrı reactivity tetiklemesi yerine 1 toplu update. Virtual scroller bundan ekstra yararlanır.

---

## 34. Config Migration Sistemi

DB migration mevcut. Config şeması da versiyonlanmalı — aksi halde adapter capability değişimi, crypto default güncellemesi veya scheduler policy eklenmesi mevcut kullanıcı config'ini bozar.

```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ConfigVersion {
    V1,  // v1.0 — başlangıç şeması
    V2,  // v1.1 — adapter capability alanları eklendi
    V3,  // v1.2 — audit consent şeması güncellendi
}

pub struct AppConfig {
    pub version:          ConfigVersion,
    pub transfer_defaults: TransferOptions,
    pub crypto_default:   CryptoAlgorithm,
    pub theme:            ThemePreference,
    pub locale:           String,
    pub queue_policy:     SchedulingPolicy,
    // ...
}

pub struct ConfigMigrator;

impl ConfigMigrator {
    pub fn migrate(raw: serde_json::Value) -> Result<AppConfig> {
        let version = detect_version(&raw)?;
        match version {
            ConfigVersion::V1 => Self::v1_to_v2(raw).and_then(Self::v2_to_v3),
            ConfigVersion::V2 => Self::v2_to_v3(raw),
            ConfigVersion::V3 => serde_json::from_value(raw).map_err(Into::into),
        }
    }

    fn v1_to_v2(mut raw: serde_json::Value) -> Result<serde_json::Value> {
        // Örnek: v1'de olmayan new_feature_flags alanı ekle
        raw["new_feature_flags"] = json!({});
        raw["config_version"] = json!("V2");
        Ok(raw)
    }

    fn v2_to_v3(mut raw: serde_json::Value) -> Result<serde_json::Value> {
        // Örnek: audit consent şeması güncellendi
        raw["audit_consent_version"] = json!(1);
        raw["config_version"] = json!("V3");
        Ok(raw)
    }
}
```

Uygulama başlangıcında config dosyası okunur, `ConfigMigrator::migrate()` çalışır, yeni şemaya dönüştürülür ve geri yazılır. Kullanıcı config'ini manuel güncellemek zorunda kalmaz.

---

## 35. Internal Tracing Spans

Telemetry hiçbir yere gönderilmez. Ama `tracing` crate zaten dependency'de — span'lar yerel diagnostics buffer'ına yazılır. Chunk lifecycle, retry chain, adapter IPC latency, DB stall gibi production sorunları diagnostics bundle'da görünür hale gelir.

```rust
// Tracing zaten Cargo.toml'da:
// tracing = "0.1"
// tracing-subscriber = { features = ["json"] }

pub async fn upload_chunk(
    adapter: &dyn ProtocolAdapter,
    chunk: &ChunkRange,
    data: &[u8],
) -> Result<(), TransferError> {
    let span = tracing::info_span!(
        "upload_chunk",
        transfer_id = %chunk.transfer_id,
        chunk_index  = chunk.index,
        chunk_size   = data.len(),
    );
    let _guard = span.enter();

    tracing::debug!("chunk_start");
    let result = adapter.write_chunk(chunk.offset, data).await;
    match &result {
        Ok(_)  => tracing::debug!("chunk_complete"),
        Err(e) => tracing::warn!(error = %e, "chunk_failed"),
    }
    result
}

// Adapter call latency
async fn call_adapter(method: &str, params: Value) -> Result<Value> {
    let span = tracing::info_span!("adapter_call", method);
    let _guard = span.enter();
    let t0 = Instant::now();
    let result = send_jsonrpc(method, params).await;
    tracing::debug!(latency_ms = t0.elapsed().as_millis(), "rpc_done");
    result
}

// DB stall tespiti
async fn queue_batch_write(tasks: &[PersistedTransferTask]) -> Result<()> {
    let span = tracing::info_span!("queue_batch_write", count = tasks.len());
    let _guard = span.enter();
    let t0 = Instant::now();
    do_write(tasks).await?;
    let elapsed = t0.elapsed().as_millis();
    if elapsed > 100 {
        tracing::warn!(elapsed_ms = elapsed, "db_write_slow");
    }
    Ok(())
}
```

**Diagnostics bundle'a yazma:**

```rust
// tracing-subscriber JSON formatter → diagnostics buffer
let subscriber = tracing_subscriber::fmt()
    .json()
    .with_writer(DiagnosticsBuffer::new())  // hiçbir yere gönderilmez
    .with_max_level(tracing::Level::DEBUG)
    .finish();
tracing::subscriber::set_global_default(subscriber)?;
```

**Span kategorileri:**

| Span | Bilgi |
|---|---|
| `upload_chunk` / `download_chunk` | Boyut, süre, offset, hata |
| `adapter_call` | Method, latency, hata |
| `queue_batch_write` | Kayıt sayısı, süre, yavaşlama uyarısı |
| `presigned_refresh` | Generation, TTL, provider |
| `sftp_capability_probe` | Banner, kanal limiti, süre |
| `retry_attempt` | Deneme sayısı, backoff, hata tipi |
| `crypto_chunk` | Algoritma, boyut, süre |

---

## 36. TLS Sertifika Yönetimi

WebDAV ve S3 (HTTPS) TLS üzerinden çalışır. Self-signed cert kullanan kurumsal sunucular yaygındır. Trust kararı sessizce verilemez — kullanıcı açık onay vermeli, sonuç profile cache'lenmeli.

### 36.1 Trust Modu

```rust
pub enum TlsTrustMode {
    SystemRoots,                            // Windows Cert Store (default)
    SystemRootsPlusPinned { fingerprints: Vec<[u8; 32]> },  // SHA-256 pin'li
    InsecureUnverified { acknowledged_at: DateTime<Utc> }, // sadece kullanıcı onayladıysa
}

pub struct TlsConfig {
    pub trust_mode:        TlsTrustMode,
    pub min_version:       TlsVersion,   // default: TLS 1.2
    pub allowed_ciphers:   Option<Vec<CipherSuite>>,
    pub sni_hostname:      Option<String>,
    pub client_cert:       Option<KeychainRef>,  // mTLS için
}
```

### 36.2 İlk Bağlantıda Sertifika Akışı

```
Bağlan → Sertifika doğrula
   │
   ├── Geçerli (System Roots) → bağlan, hiçbir şey sorma
   ├── Self-signed / unknown CA / hostname mismatch / expired
   │       ↓
   │   [Modal: Sertifika Doğrulanamadı]
   │   • Konu: CN=...
   │   • Veren: CN=... (Self-signed / Unknown CA / ...)
   │   • Geçerlilik: ... → ...
   │   • SHA-256 parmak izi: AB:CD:EF:...
   │   • Hata: HOSTNAME_MISMATCH | EXPIRED | UNTRUSTED_ROOT
   │
   │   [Bir kez kabul et]  [Pin'le ve hatırla]  [İptal]
   │
   └── Pin'le seçilirse → fingerprint profile'a yazılır
       Sonraki bağlantılarda fingerprint match şart; değişirse uyarı (MITM)
```

### 36.3 Pin Mismatch Davranışı

Pinlenmiş sertifikanın fingerprint'i değişirse (CA renew, MITM, sunucu değişikliği) bağlantı **hata verir, sessizce devam etmez**:

```rust
TransferError::TlsPinMismatch {
    expected: String,    // hex SHA-256
    actual:   String,
    host:     String,
}
```

UI bu hatada: "Sertifika değişti — eskisini sil ve yenisini pin'le?" şeklinde açık bir prompt gösterir. Otomatik kabul yok.

### 36.4 Test Kapsamı

```rust
#[test] fn self_signed_rejected_without_acknowledgement() { ... }
#[test] fn pinned_fingerprint_required_on_subsequent()    { ... }
#[test] fn pin_mismatch_blocks_connection()               { ... }
#[test] fn expired_cert_distinguishable_from_untrusted()  { ... }
#[test] fn hostname_mismatch_specific_error()             { ... }
```

### 36.5 Trust Lifecycle Policy (v1.16)

Pin/store mekanizması var, ama gerçek dünyada **trust kararı zaman içinde değişir**: cert rotation, intermediate CA değişimi, expiry approaching, kullanıcının "trust once" ile "trust permanently" arasındaki fark. v1.16'da bu lifecycle açıkça spec'lenir.

**Trust Decision Types:**

```rust
pub enum TrustDecision {
    /// Sistem root store + hostname match — varsayılan
    SystemRoots,
    /// Belirli bir cert fingerprint'i (SHA-256) pin'lendi
    /// Cert rotation olduğunda re-prompt
    PinnedFingerprint { sha256: [u8; 32], pinned_at: DateTime<Utc> },
    /// Belirli SPKI (Subject Public Key Info) pin'i — leaf cert rotate olsa bile key aynıysa OK
    /// Daha esnek, Let's Encrypt rotation'larında re-prompt yapmaz
    PinnedSpki { sha256: [u8; 32], pinned_at: DateTime<Utc> },
    /// "Bu session boyunca güven" — uygulama kapanınca silinir
    /// Test ortamında, geçici bağlantılar için
    TrustOnce,
    /// "Insecure mode" — TLS validation devre dışı, kullanıcı bilinçli kabul etti
    /// UI'da kalıcı kırmızı badge
    InsecureSkipVerify { acknowledged_at: DateTime<Utc> },
}
```

**Cert rotation handling:**

| Senaryo | Davranış |
|---|---|
| `PinnedFingerprint` + leaf cert rotated (yeni fingerprint, aynı domain) | **Re-prompt:** "Bu sunucunun sertifikası değişti. Bu beklenen mi?" + eski/yeni fingerprint karşılaştır + cert chain göster |
| `PinnedSpki` + leaf rotated, **aynı public key** (CSR re-sign) | **Silent OK** — kullanıcı görmez, log'a info notu |
| `PinnedSpki` + key rotated | **Re-prompt**, fingerprint pin'i ile aynı UX |
| `SystemRoots` + intermediate CA değişti (Let's Encrypt'in yaptığı gibi) | **Silent OK** — sistem zaten yeni chain'i doğrular |
| `SystemRoots` + cert revoke edildi (OCSP/CRL) | **Block** + error: "Sertifika iptal edilmiş" |
| Expired cert (clock-skew olabilir) | **Distinguishable error** + saat kontrolü uyarısı (Bölüm 28 Updater clock drift mantığı) |

**SPKI pinning neden default önerilir:**

Modern PKI'de cert rotation yaygın (Let's Encrypt 60-90 gün, AWS ACM otomatik). Leaf fingerprint pin'lemek = her rotation'da kullanıcıya prompt → "Trust fatigue", kullanıcı sonunda her şeyi onaylar → güvenlik teorik kalır. **SPKI pin** ise long-lived public key'i pin'ler, leaf cert change'leri silent geçer ama key compromise prompt çıkar.

Default UI behavior: "Pin this server" → SPKI mode (kullanıcı explicit "fingerprint pin" seçmedikçe). Advanced settings'te switch.

**Trust state persistence:**

```sql
-- ~/.dtransfer/trust.db (SQLite, separate from queue.db)
CREATE TABLE trust_decisions (
    profile_id        TEXT NOT NULL,
    host              TEXT NOT NULL,
    port              INTEGER NOT NULL,
    decision_type     TEXT NOT NULL,    -- 'system' / 'pin_fp' / 'pin_spki' / 'once' / 'insecure'
    fingerprint       BLOB,              -- 32 bytes SHA-256 (pin modes)
    spki_hash         BLOB,              -- 32 bytes SHA-256 (SPKI mode)
    pinned_at         INTEGER NOT NULL,
    last_seen_at      INTEGER NOT NULL,  -- son başarılı bağlantı
    expires_at        INTEGER,           -- "trust once" için; NULL = kalıcı
    PRIMARY KEY (profile_id, host, port)
);
```

**Expiry & cleanup:**

- `TrustOnce` kayıtları process exit'inde silinir (RAM-only veya `expires_at = now + 24h`)
- `InsecureSkipVerify` kayıtları kullanıcı manuel Settings'ten silmedikçe kalıcı; UI'da listelenir
- Pinned (FP veya SPKI) kayıtları son `last_seen_at`'tan 1 yıl sonra otomatik archive ("Bu sunucuyu 1 yıldır kullanmıyorsunuz — pin'i temizleyelim mi?")

**SHA1 / weak crypto cert reddi:**

```rust
pub struct TlsPolicy {
    pub min_tls_version:    rustls::version::TlsVersion,  // default: TLS 1.2
    pub allowed_signatures: Vec<SignatureScheme>,         // SHA1 excluded
    pub require_sct:        bool,                          // Certificate Transparency log proof
}

// Default: TLS 1.2+, SHA-256+, no SHA1, SCT optional (kurumsal CA'lar bazen sağlamaz)
```

SHA1 sertifika gören kullanıcıya **block + explicit override** gerekli — "Trust fatigue" pattern'i kırmak için subtle değil, modal düzeyinde uyarı.

**Operational recovery (Bölüm 28 ile coupling):**

Trust pin mismatch sırasında recovery event yayınlanır:

```rust
EngineEvent::TlsPinMismatch {
    profile_id,
    host,
    pinned_fp:    [u8; 32],
    presented_fp: [u8; 32],
    cert_chain:   Vec<X509Cert>,        // UI'da gösterilir
}
```

UI: Modal (kritik, dismiss yok) — "Sunucu sertifikası değişti. Devam etmeden önce kontrol edin." + fingerprint karşılaştırma + cert detail.

---

## 37. Proxy Desteği (HTTP / SOCKS5)

Kurumsal kullanıcıların büyük kısmı HTTP/SOCKS proxy arkasında. Proxy desteği olmazsa pazar yarısı kapanır.

### 37.1 ProxyConfig

```rust
pub enum ProxyKind { Http, Https, Socks5 }

pub struct ProxyConfig {
    pub kind:           ProxyKind,
    pub host:           String,
    pub port:           u16,
    pub credential_ref: Option<KeychainRef>,  // basic/bearer auth
    pub bypass_hosts:   Vec<String>,          // örn. ["*.local", "10.0.*"]
    pub dns_through_proxy: bool,              // SOCKS5 remote DNS
}

pub enum ProxySource {
    None,
    System,                       // Windows WinHTTP / WPAD
    Custom(ProxyConfig),
    PerProfile(Uuid, ProxyConfig), // sadece bu profile uygula
}
```

### 37.2 Protokol Bazlı Davranış

| Protokol | HTTP Proxy | SOCKS5 |
|---|---|---|
| HTTP / WebDAV / S3 / REST | reqwest `Proxy::http/https` | reqwest `Proxy::all` (socks feature) |

| SFTP / SCP | HTTP proxy: `CONNECT` tüneli (russh) | SOCKS5 native (russh-config) |

```rust
// SFTP için CONNECT tüneli
async fn open_connect_tunnel(proxy: &ProxyConfig, target: &SocketAddr) -> Result<TcpStream> {
    let mut stream = TcpStream::connect((proxy.host.as_str(), proxy.port)).await?;
    write_connect_request(&mut stream, target, proxy.credential_ref.as_ref()).await?;
    expect_200_connected(&mut stream).await?;
    Ok(stream)
}
```

### 37.3 Bypass Listesi

`bypass_hosts` glob pattern destekler — `*.local`, `192.168.*`, `10.*`, `localhost`. Match olan host proxy'siz çıkar. Windows System Proxy seçeneğinde Windows'un kendi bypass listesi okunur (`HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings\ProxyOverride`).

### 37.4 PAC / WPAD

v1.0 kapsamında **PAC scripting yok**. Sistem proxy modu Windows'un effective proxy'sini sorgular (WPAD dahil), ancak kendi PAC engine implement edilmez. Karmaşık PAC kuralları için kullanıcı manuel ProxyConfig girer veya v2 stub'ında `pac_url` alanı ile sonradan eklenir.

### 37.5 NTLM / Negotiate (Kerberos) Proxy Auth — v1.1 Defer (v1.15 documentation)

**v1.0 scope dışı, v1.1'de eklenmesi planlı.**

Kurumsal Windows ortamlarında ISA Server, Microsoft Forefront TMG, modern Microsoft Defender for Cloud Apps proxy'leri sıklıkla **NTLM veya Negotiate (Kerberos)** authentication zorlar. Aynı kullanıcının domain kimliği transparent olarak proxy'ye sunulur (SSO).

**v1.0'da neden yok:**

`reqwest` (DTransfer'ın HTTP client'i) ve altındaki `hyper` ekosistemi **proxy authentication scheme'leri için native NTLM/Negotiate desteği sunmuyor**. Sadece Basic ve Digest desteklenir (HTTP/1.1 standart). NTLM bir multi-round-trip challenge-response handshake'i — TCP connection persistence ve auth state tracking gerekir, reqwest mimarisi ile uyumsuz.

**WebDAV NTLM ile karıştırma:** Bölüm 11.1.1'deki WebDAV NTLM auth **server auth**'tur (target server kimlik istiyor). Bu farklı problem — burada söz konusu olan **proxy auth** (proxy CONNECT'i veya HTTP request'i NTLM ile auth gerektiriyor).

**v1.1 implementasyon planı:**

İki olası yol:

**A. WinHTTP API üzerinden delegation (Windows-only, v1.1):**

```rust
#[cfg(windows)]
mod winhttp_proxy {
    // windows-rs crate ile WinHTTP API
    // WinHttpOpen, WinHttpSetCredentials, WinHttpSendRequest
    // OS'un kendi NTLM/Negotiate SSP stack'ini kullan
    pub async fn http_request_via_winhttp(req: HttpRequest) -> Result<HttpResponse> {
        // WinHTTP otomatik olarak current user'ın Kerberos ticket'ını / NTLM hash'ini kullanır
        unimplemented!("v1.1 — Windows-only NTLM/Negotiate proxy")
    }
}
```

Avantajı: SSO native, kullanıcı şifre girmez, domain credential'lar otomatik.
Dezavantajı: Windows-only, Linux/macOS'ta farklı çözüm gerekli, codebase ikiye ayrılır.

**B. Sidecar local proxy (cross-platform, v1.1):**

NTLM-aware local proxy daemon (örn. `cntlm` veya custom Rust impl) DTransfer ile birlikte ship edilir. DTransfer trafiği `localhost:port`'a yönlendirir, sidecar kurumsal proxy'ye NTLM/Negotiate auth ile çıkar.

Avantajı: Cross-platform tek codebase.
Dezavantajı: Ek binary, ek port, kullanıcının NTLM credential'ı saklanması gerek (keychain).

**Karar:** v1.1'de **A (WinHTTP) Windows için**, **B (sidecar) Linux için** hibrit. macOS topluluk portu kendi `NSURLConnection`/`CFNetwork` stack'ini kullanabilir.

**v1.0 kullanıcılarına ne diyoruz:**

NTLM/Negotiate proxy gerektiren kurumsal kullanıcı v1.0'da:
- Basic veya Digest fallback'i varsa (bazı proxy admin'ler izin verir) → kullanır
- IT'den exception ister (`*.dtransfer.app` proxy bypass)
- v1.1'i bekler

Error message'da bu durum net açıklanır:

```rust
TransferError::ProxyAuthUnsupported {
    scheme: "NTLM",
    proxy_host: "proxy.corp.local",
    hint: "DTransfer v1.0 NTLM/Negotiate proxy auth desteklemiyor. v1.1 (Q3 2026) ile gelecek. \
           Geçici çözüm: IT yöneticinizden Basic auth veya whitelist isteyin.",
}
```

UI'da Profile → Proxy → "Test connection" yaparken bu hata yakalanır, kullanıcı kafa karışıklığı yaşamaz.

---

## 38. SSH / SFTP Keepalive ve Network Resilience

Pause edilmiş veya boşa çıkmış SFTP bağlantıları kurumsal firewall'lar (Fortinet, Palo Alto) tarafından sessizce drop edilir — genellikle 1800sn idle sonrası. Transfer "Devam et" tıklandığında yarım saat sonra kullanıcı patlar bir hata görür.

### 38.1 Keepalive Profili

```rust
pub struct SshKeepalive {
    pub server_alive_interval: Duration,  // default: 30s — packet gönder
    pub server_alive_count_max: u8,       // default: 3 — yanıt yoksa drop
    pub tcp_keepalive:         bool,      // default: true (SO_KEEPALIVE)
    pub tcp_keepalive_idle:    Duration,  // Windows: WSAIoctl ile
}

impl Default for SshKeepalive {
    fn default() -> Self {
        Self {
            server_alive_interval:  Duration::from_secs(30),
            server_alive_count_max: 3,
            tcp_keepalive:          true,
            tcp_keepalive_idle:     Duration::from_secs(60),
        }
    }
}
```

russh konfigürasyonuna `keepalive_interval` ve `keepalive_max` parametreleri geçilir. 90 saniye boyunca yanıt yoksa bağlantı `ConnectionLost` ile düşürülür ve transfer state machine reconnect denemesine girer.

### 38.2 Pause Davranışı

| Pause süresi | Davranış |
|---|---|
| < 5 dk | Bağlantı keepalive ile canlı tutulur |
| 5–30 dk | Bağlantı keepalive ile canlı tutulur (varsayılan) |
| > 30 dk | Bağlantı kapatılır, "resume" tıklanınca yeniden auth + capability probe |

Eşik `transfer_opts.keep_connection_secs` ile profil bazında ayarlanabilir. Cloud presigned URL transfer'lerinde keepalive gerekmez — URL expire'dan önce devam edilirse devam, sonrasında refresh.


### 38.4 Staggered Connect (Firewall/Fail2Ban Koruması)

Kullanıcı bağlantı limitini 8'e ayarlar. Klasik istemci aynı milisaniyede 8 paralel TCP/SSH handshake yollar. Kurumsal firewall (Fortinet, pfSense, FortiGate) bunu **SYN flood saldırısı** olarak algılar; Fail2Ban auth-attempt threshold'unu aşar; cloudflare-style anti-DDoS WAF rate limit tetikler. Sonuç: IP banlanır, kullanıcı bağlanamaz, hata mesajı belirsiz.

```rust
pub struct ConnectStrategy {
    /// İlk bağlantı sonrası kaç ms bekle, sonraki açılır
    pub stagger_initial_ms: u64,    // default: 250
    /// Sonraki her bağlantıda jitter aralığı [min, max] ms
    pub stagger_jitter_ms:  (u64, u64),  // default: (250, 500)
    /// Auth tamamlanana kadar bir sonraki bağlantıyı açma
    pub wait_for_auth:      bool,    // default: true
}

impl ConnectionPool {
    pub async fn acquire_n(&self, n: usize) -> Vec<PooledConn> {
        let mut conns = Vec::with_capacity(n);
        for i in 0..n {
            if i > 0 {
                let jitter = thread_rng().gen_range(
                    self.strategy.stagger_jitter_ms.0..=self.strategy.stagger_jitter_ms.1
                );
                tokio::time::sleep(Duration::from_millis(jitter)).await;
            }
            let conn = self.acquire_single().await?;
            // Auth tamamlanmadan diğerini açma
            if self.strategy.wait_for_auth {
                conn.wait_authenticated().await?;
            }
            conns.push(conn);
        }
        conns
    }
}
```

**Davranış:**
- 8 bağlantı isteği = ilk bağlantı T+0ms, sonraki T+250-500ms (jitter), 8'inci yaklaşık T+2-3.5sn
- Total ramp-up ~3 saniye — kullanıcı için fark edilmez (transfer 30sn+ sürüyor)
- Firewall'lar burst pattern yerine "natural client" gibi görür
- Fail2Ban "3 failed auth in 5sn" eşiğine takılmaz (auth aralarında 250-500ms boşluk)

**Override:** Profile bazında `connect_strategy.aggressive = true` set edilirse stagger devre dışı (intranet/güvenli ortam). Power user override.

### 38.5 Network Change Monitor (Sleep/Wake + WiFi Roaming)

Laptop kullanıcısı kafede indirme başlatır → kapağı kapatır → eve gider → kapağı açar (WiFi değişti, IP değişti). Klasik istemci eski TCP soket'in koptuğunu fark etmez, OS-level TCP timeout'u (varsayılan 2-15 dakika) bekler. Kullanıcı uygulamanın "donmuş" olduğunu zanneder, force-quit ile kapatır, baştan başlamak zorunda kalır.

DTransfer **OS network change event'lerini** dinler ve agresif tepki verir.

```rust
use if_watch::{IfEvent, IfWatcher};

pub struct NetworkChangeMonitor {
    pool: Arc<ConnectionPool>,
    event_tx: broadcast::Sender<NetworkEvent>,
}

pub enum NetworkEvent {
    /// Yeni interface up (yeni WiFi, ethernet plug-in)
    InterfaceUp { name: String, ip: IpAddr },
    /// Interface down (WiFi kayıp, ethernet unplug)
    InterfaceDown { name: String },
    /// Sleep/wake (Win/macOS suspend resume)
    SystemResumed,
    /// IP değişti (DHCP renewal yeni adres aldı)
    IpChanged { old: IpAddr, new: IpAddr },
}

impl NetworkChangeMonitor {
    pub async fn run(self) -> Result<()> {
        let mut watcher = IfWatcher::new()?;
        // System resume event listener (platform-specific)
        let mut resume_rx = platform::system_resume_signal();

        loop {
            tokio::select! {
                Some(event) = watcher.next() => {
                    match event {
                        IfEvent::Up(addr)   => self.handle_up(addr).await,
                        IfEvent::Down(addr) => self.handle_down(addr).await,
                    }
                }
                _ = resume_rx.recv() => {
                    self.handle_resume().await;
                }
            }
        }
    }

    async fn handle_resume(&self) {
        tracing::info!("system resumed from sleep, dropping all connections");
        // Tüm aktif soketleri agresif kapat — TCP timeout BEKLEMEZ
        self.pool.drop_all_aggressively().await;
        // ConnectionLost event tüm aktif transferlere
        self.event_tx.send(NetworkEvent::SystemResumed).ok();
        // Engine reconnect akışına geçer (Bölüm 10 retry policy)
    }

    async fn handle_up(&self, addr: IpAddr) {
        // 500ms bekle (DHCP/DNS settle), sonra reconnect dene
        tokio::time::sleep(Duration::from_millis(500)).await;
        // Pending transfer'leri trigger et
        self.event_tx.send(NetworkEvent::InterfaceUp { /* ... */ }).ok();
    }
}
```

**Platform-specific resume signal:**
- **Windows:** `WM_POWERBROADCAST` mesajı (`PBT_APMRESUMESUSPEND`)
- **Linux:** systemd-logind D-Bus `PrepareForSleep(false)` event
- **macOS (v2 port):** `IOPMScheduleNotificationPath` ile

**Davranış senaryoları:**

| Olay | Tepki |
|---|---|
| WiFi → Ethernet (cable plug) | Eski WiFi soket'leri 500ms sonra reconnect (yeni interface tercih) |
| Sleep 30dk → Wake | Tüm soketler aggressive close, full reconnect cycle (~3-5sn) |
| IP renewal (DHCP lease expire) | IP change event → mevcut soket reset, yeni IP'den reconnect |
| Tunnel/VPN connect | Yeni interface up → mevcut soketler korunur (parallel kullanım), ama yeni transfer'ler VPN üzerinden |
| Tunnel/VPN disconnect | VPN interface down → bu interface'teki soketler reset |

**Status bar UI:** Network change anında "Ağ değişti — yeniden bağlanılıyor…" toast (3-5sn). Reconnect başarısızsa "Ağ kayıp, manuel yeniden bağlanma gerekli" banner.

### 38.6 Application-Level Write Timeout (Liar NAT Koruması)

SSH keepalive 30sn'de yollanır, ama bu sadece bağlantının kontrol kanalıdır. Asıl sorun: **veri yazıyorken** TCP state'i kayıp olur. Senaryo:
1. Kullanıcı 1GB chunk yazıyor, socket'e 50MB yazıldı
2. Aradaki ucuz ISP router veya kurumsal NAT TCP state'i drop ediyor (idle timeout, tablo full)
3. RST paketi göndermiyor — sessizce yutuyor
4. Uygulama kendini bağlı sanıyor, write çağrıları return ediyor (OS buffer'a yazıyor)
5. Veri boşluğa gidiyor, sunucuya hiç ulaşmıyor
6. Transfer "%5" ekrana basılı, gerçekte hiçbir şey yok

OS-level keepalive bunu yakalamaz çünkü kontrol kanalı ayrı. Çözüm: **application-level write timeout**. Eğer chunk yazılıyor ve N saniye boyunca **karşıdan ACK / window update gelmezse**, soket app tarafından zorla kapatılır.

```rust
pub struct TransferOptions {
    /// Socket'e yazıldığı andan itibaren karşı taraftan progress
    /// (ACK / window update / response) gelmesi için max bekleme
    pub write_timeout_secs: u64,    // default: 15

    /// Read timeout — hiçbir byte gelmiyorsa max bekleme
    pub read_timeout_secs:  u64,    // default: 30

    /// Connection idle timeout (sıfır byte transfer)
    pub idle_timeout_secs:  u64,    // default: 60
    // ...
}

// Implementasyon: tokio::time::timeout wrap
async fn write_chunk_with_timeout(
    socket: &mut TcpStream,
    chunk: &[u8],
    write_timeout: Duration,
) -> Result<()> {
    match tokio::time::timeout(write_timeout, socket.write_all(chunk)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(TransferError::Io(e)),
        Err(_) => {
            // Timeout — socket'i ZORLA kapat
            // OS TCP timeout'unu BEKLEMEDEN
            socket.shutdown().await.ok();
            Err(TransferError::WriteTimeout {
                duration: write_timeout,
                bytes_pending: chunk.len(),
            })
        }
    }
}
```

**TCP_USER_TIMEOUT (Linux + Win 10+):** `setsockopt(TCP_USER_TIMEOUT)` ile OS-level write timeout da set edilir. Bu kernel-level paralel koruma — uygulama timeout'ı pratikte çalışmazsa OS yine de socket'i öldürür.

```rust
#[cfg(target_os = "linux")]
fn set_tcp_user_timeout(socket: &TcpStream, ms: u32) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    let fd = socket.as_raw_fd();
    let timeout: libc::c_uint = ms;
    unsafe {
        let ret = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_USER_TIMEOUT,
            &timeout as *const _ as *const _,
            std::mem::size_of_val(&timeout) as libc::socklen_t,
        );
        if ret == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
    }
}
```

**Davranış:**
- 1GB chunk yazılıyor, 50MB sonra Liar NAT drop yaptı
- 15sn boyunca yazma progress yok → write_timeout tetikleniyor
- Socket force-close, transfer `Failed::WriteTimeout(15s, 50MB pending)`
- Retry policy reconnect → kaldığı offset'ten resume (Bölüm 14.5)
- Kullanıcı ekranda "Bağlantı kayıp, yeniden deneniyor…" görür, "donmuş" hissi YOK

---

## 39. Geliştirme Yol Haritası

### v1.0 — Taş Motor (13–17 hafta)

**Faz 1 — İskelet (1–2 hafta)**
- [ ] Tauri 2 + Vue 3 + Pinia + Tailwind + Vite
- [ ] JetBrains Mono + tema sistemi
- [ ] i18n (TR/EN)
- [ ] ProtocolAdapter + TransferError taxonomy
- [ ] **Filesystem edge-case modülü** (NormalizedPath, SymlinkPolicy, sanitize_for_target, case conflict detect)
- [ ] ConnectionPool (semaphore)
- [ ] **CancellationToken hiyerarşisi** (App/Profile/Transfer/Chunk)
- [ ] **EngineEvent bus** (broadcast channel) + EngineEventAggregator (batch + urgent)
- [ ] **Config migration sistemi** (V1→V2→V3)
- [ ] **Internal tracing spans** (diagnostics buffer)
- [ ] **Stall watchdog** (dedicated OS thread)

**Faz 2 — Transfer Motoru (4–5 hafta)**
- [ ] SFTP + CapabilityProfile probe + max_inflight_bytes (russh-sftp) + keepalive + sparse
- [ ] S3 / R2 / B2 (aws-sdk-s3) + proxy
- [ ] WebDAV + TLS pin desteği
- [ ] XChaCha20-Poly1305 (Argon2id KDF + nonce yönetimi + chunk auth tag + zeroize)
- [ ] **Secret zeroization** (tüm credential/key path'leri)
- [ ] Multipart + atomic birleştirme + **fsync politikası** (DataOnly default + parent dir fsync Full mode)
- [ ] Resume (.dtresume atomic write, plaintext offset)
- [ ] **Per-chunk hash** + binary blob file (`{file_id}.chunkmap`, mmap zero-copy)
- [ ] PresignedContext generation (refresh race koruması)
- [ ] Rate Limiter + adaptive backoff (profile_id key)
- [ ] ProgressAggregator 250ms
- [ ] Memory backpressure
- [ ] Presigned URL log masking
- [ ] **TLS Sertifika yönetimi** (system roots / pin / insecure)
- [ ] **HTTP/SOCKS5 Proxy** (reqwest + russh + suppaftp tüneli)
- [ ] **SSH/SFTP keepalive** (server alive + TCP keepalive)
- [ ] **Staggered Connect with Jitter** (Fail2Ban / firewall guard)
- [ ] **Network Change Monitor** (if-watch + sleep/wake + WiFi roaming)
- [ ] **Application-Level Write Timeout** (Liar NAT koruması, TCP_USER_TIMEOUT setsockopt)
- [ ] **AV Lock Micro-Retry** (Windows atomic_rename: ACCESS_DENIED + SHARING_VIOLATION 50/150/300/600/1000ms)
- [ ] **Windows Shared Read flag** (FILE_SHARE_READ|WRITE|DELETE — kilitli log/PST yedekleme)
- [ ] **Absolute Symlink Sanitization** (kötü niyetli sunucu /etc/passwd hijack koruması, SymlinkTargetPolicy)
- [ ] **Mtime DST/Timezone heuristic** (3600/7200sn + size match → silent skip)

**Faz 3 — Queue + Persistence (1.5 hafta)**
- [ ] queue.db (SQLite WAL + tokio-rusqlite)
- [ ] PersistedTransferTask CRUD + TransferState machine
- [ ] **DbActor pattern** (mpsc serialize, oneshot ack)
- [ ] QueueScheduler (FIFO + priority)
- [ ] **Directory Traversal Stream** (node_modules problemi, BFS streaming, ilk 100 hemen kuyruğa)
- [ ] **Transfer Filter + .transferignore** (gitignore syntax, default OS metadata, Skipped vs Failed ayrımı)
- [ ] QueueProgressWriter (bytes_done 5s batch + flush_now on chunk)
- [ ] Startup recovery (yarım transfer devam) + **orphan cleaner**
- [ ] **Paginated list_dir Stream API** (ProtocolAdapter Vec→Stream, 2M dosya OOM koruması)
- [ ] **view_cache.db** (SQLite paginated remote listing, VirtualScroller LIMIT/OFFSET)

**Faz 4 — UI + UX (2.5 hafta)**
- [ ] Dual panel + vue-virtual-scroller
- [ ] Drag & Drop koordinat bazlı (tauri-plugin-drag-drop)
- [ ] Transfer kuyruğu UI (chunk bar + persist durumu, **ikon+sayı disiplini**)
- [ ] Bağlantı bilgi paneli + hız grafiği
- [ ] Profil yöneticisi + Windows Credential Manager / Linux Secret Service
- [ ] **TLS sertifika onay modal'ı** (pin & remember)
- [ ] **Proxy ayar UI'ı** (system / custom / per-profile)
- [ ] **Conflict Resolution UX** (modal + apply-to-all + checksum lazy compare)
- [ ] **Quick Connect Wizard** (S3 / SFTP / Synology preset templates) — onboarding için
- [ ] Reka UI headless components (Select, Calendar, Tooltip, Dialog)
- [ ] Tam klavye kısayolları (ARIA)

**Faz 5 — Recovery + Test + Benchmark + Paketleme (3 hafta)**
- [ ] Crash recovery (orphan chunk, DB backup)
- [ ] **Updater anti-rollback** (TUF benzeri: min_version, expires_at, monotonic)
- [ ] **Operational Recovery Playbooks** (recovery banner/modal/notification, i18n recovery namespace)
- [ ] **LimitProfile auto-detect** (LowMemory/Desktop/Workstation/Server, system probe)
- [ ] **Failure semantics chaos test suite** (CrashPoint enum, garanti tablosu doğrulama)
- [ ] Diagnostics Bundle (tracing span'lardan beslenir + RuntimeMetrics + RuntimeLimits + recovery_events.ndjson)
- [ ] Rust birim + entegrasyon testleri (unftp, wiremock, localstack, openssh-server)
- [ ] **Benchmark suite** (criterion, divan) + perf gate (≥%10 regression block)
- [ ] Vue Vitest + Playwright (functional)
- [ ] **Playwright visual regression** (Win + Linux baseline ayrı, %2 eşik)
- [ ] CI pipeline (windows-latest + ubuntu-latest matrix)
- [ ] Windows: MSI + portable ZIP + Authenticode imzalama
- [ ] Linux: AppImage + .deb (cargo-deb veya dpkg-deb)

### v1.1 — Genişletilmiş Motor (4–5 hafta)

- [ ] SCP
- [ ] Akıllı kısmi transfer optimizasyonu (S3 + WebDAV + SFTP)
- [ ] PresignedRequest + expiry + refresh
- [ ] GitHub Releases + Ed25519 + key rotation + revoke flow
- [ ] Cloud adapter'ları (Dropbox · OneDrive · GCS · Azure · Firebase · Custom REST)

### v1.2 — Premium Feature Katmanı (Opt-In) (2–3 hafta)

- [ ] Network Wizard adapter
- [ ] Audit Trail adapter (batch write + maskeleme + KVKK rızası)

**Toplam: 20–27 hafta**

---

## 40. Teknik Kararlar ve Gerekçeler

| Karar | Alternatif | Gerekçe |
|---|---|---|
| Rust core engine | Go, Node.js | Sıfır maliyet soyutlama, bellek güvenliği, tokio |
| Tauri 2 | Electron | ~10MB vs ~150MB, düşük RAM |
| Vue 3 | React | Admin Toolkit + D-Player tutarlılığı |
| Subprocess IPC adapter | libloading | Rust ABI yok, Gatekeeper dostu, crash isolation |
| JSON-RPC 2.0 | Raw JSON | Request ID, async correlation, structured error |
| XChaCha20 varsayılan | AES-256-GCM | Nonce misuse toleransı; parallel chunk güvenliği |
| PresignedRequest + expiry | URL'yi bir kez al | Uzun transfer/pause edge case önler |
| PresignedContext generation | Yok | Refresh race: stale completion ignore edilir |
| Presigned URL log masking | Plaintext log | SAS token / credential log'a düşmez |
| RemoteProvider trait | ProtocolAdapter | Cloud adapter için dar, IPC-friendly yüzey |
| SFTP CapabilityProfile | Sabit paralel | NAS/Synology/BusyBox channel limit önler |
| max_inflight_bytes SFTP | max_buffered_chunks | Byte bazlı RAM limiti; yüksek latency koruması |
| ProgressAggregator 250ms | Per-event emit | 100 transfer × 8 chunk → Vue render baskısı önlenir |
| TransferState machine | Bare enum | can_transition_to(): race/double-retry/zombie task önlenir |
| RateLimiter key = profile_id | host | Aynı servisteki farklı hesaplar birbirini etkilemez |
| bytes_done 5s batch write | Per-chunk write | WAL + rusqlite blocking; disk I/O felci önlenir |
| tokio-rusqlite | rusqlite direkt | tokio worker thread'leri DB I/O'dan bloke olmaz |
| D&D koordinat bazlı drop | DOM hedef | Virtual Scroller: DOM'da olmayan klasöre drop çalışır |
| Windows creation_flags | Stdio::piped() tek başına | Handle inheritance: credential leak önlenir |
| command-group | tokio::process | Zombie process: PR_SET_PDEATHSIG + Job Object |
| Key rotation yapısı | Tek statik key | Key compromise sonrası ekosistem kilitlenmez |
| Queue persistence (SQLite WAL) | Runtime only | Restart/crash sonrası kuyruk devam eder |
| Structured TransferError | anyhow only | UI retry/refresh/pause kararı için şart |
| Rate Limiter + adaptive backoff | Yok | Cloud API throttle: 429 sessiz hata olmaz |
| Vue Virtual Scroller | v-for | 10k+ dosyada UI donması önlenir |
| Diagnostics Bundle | Telemetry | Tamamen lokal; gizlilik korunur |
| v2 stub'ları şimdiden | Sonra ekle | SyncEngine/Sandbox/Scheduler eklerken yeniden yazma gerekmez |
| Delta → v1.1 | v1.0 | SCP ile birlikte; v1.0 scope odaklı kalır |
| SCP → v1.1 | v1.0 | SFTP/S3/WebDAV v1.0 için yeterli |
| Atomic write her yerde | Doğrudan yazma | Kesinti = bozulma yok |
| Telemetry = sıfır | Opt-in | KVKK/GDPR; kullanıcı güveni |
| Drag & Drop v1.0 | Sonra | Düşük efor, yüksek UX etkisi |
| **CancellationToken hiyerarşisi** | Ayrı mekanizmalar | App/Profile/Transfer/Chunk — iptal zinciri tek merkezde, race-free |
| **Unified EngineEvent bus** | Ayrı kanallar | Progress + log + adapter + queue tek enum; diagnostics buradan beslenir |
| **Config migration V1→V3** | Manuel güncelleme | Crypto/scheduler/feature flag değişimlerinde kullanıcı config bozulmaz |
| **Internal tracing spans** | Yok / harici telemetry | Chunk lifecycle + IPC latency + DB stall diagnostics'te görünür; hiçbir yere gönderilmez |
| **Windows + Linux v1.0** | Tek platform / cross-platform sonradan | Linux+Win WebKit+Blink ikilisi paralel test edilerek macOS portunu da kolaylaştırır; macOS topluluk portu GPL-3.0+ altında |
| **GPL-3.0-or-later lisans** | MIT/Apache/AGPL | Copyleft koruma: türev çalışmalar da aynı şartlarda açık kalır; D Brand felsefesi (Privacy First, Local by Default, Open Source) ile uyumlu; FileZilla'nın GPLv2 mirası ile aynı ailede, ama GPLv3 patent grant ve TiVo-clause ile modern |
| **TLS pin + insecure ack** | Sessiz kabul / katı red | Self-signed kurumsal NAS'lar yaygın; granüler trust + MITM koruması |
| **HTTP/SOCKS5 proxy desteği** | Sadece sistem proxy | Kurumsal pazar şart; per-profile + bypass + dns_through_proxy |
| **SSH ServerAliveInterval 30s** | Default keepalive | Firewall (Fortinet/PAN) 1800sn idle drop'unu önler; pause UX |
| **Argon2id m=19MiB t=2 p=1** | Default Argon2 / scrypt | OWASP 2024 desktop profili; ~250–500ms derivation, params_version ile migration-safe |
| **Plaintext offset resume** | Encrypted byte offset | chunk_size değişiminde resume bozulmaz; auth tag (16B) chunk_size üstüne yazılır |
| **bytes_done 5s batch + flush_now** | Per-chunk DB write | WAL fsync amplification önlenir; chunk tamamlanınca anlık flush; doğruluk chunk tablosundan |
| **Performans benchmark gate** | Ad-hoc test | criterion CI'da %10 regression PR bloklar; v1.0 release notlarında flamegraph zorunlu |
| **SLSA L2 + Azure Key Vault HSM** | Self-signed local key | Updater signing key runner'a inmez, provenance attestation |
| **Probe-only SFTP capability** | Hardcoded NAS listesi | Banner listesi yıllar içinde eskir; probe + default 4 her durumda doğru karar verir |
| **Linux birinci sınıf platform** | Sadece Windows / topluluk Linux | WebKit + Blink ikilisini paralel test etmek macOS portunu da otomatik kolaylaştırır; KVKK altyapısı Linux/Win ortak |
| **Tauri + CSS disiplini (Electron değil)** | Electron / Servo bundle | 50MB binary + 50MB RAM hedefi sürdürülür; 6 disiplin kuralı + Reka UI + visual regression ile WebView farkı yönetilir |
| **Reka UI (eski Radix Vue) headless lib** | Custom her component | Native HTML widget yasaklı (date/select/file picker) — headless lib ile davranış al, stil tema token'larından |
| **Playwright visual regression** | Manuel görsel test | Win + Linux baseline ayrı, CI'da %2 eşikli pixel diff; PR'larda screenshot artifact |
| **Font paketleme (sistem fallback yasak)** | system monospace fallback | Linux'ta JetBrains Mono garantili değil; FOIT yerine font-display:block ile tutarlı render |
| **EngineEventAggregator batch + urgent express lane** | Per-event tauri::emit | 1000 küçük dosya = 200 IPC/sn fırtınası → batch 4/sn; auth prompt'lar urgent ile bypass |
| **DbActor pattern (mpsc serialize)** | Concurrent flush_now | 16 paralel chunk completion = SQLITE_BUSY riski; tek tüketici actor + oneshot ack ile sıfırlanır |
| **CPU-bound spawn_blocking zorunlu** | tokio::fs sanılan koruma | tokio::fs zaten blocking pool kullanır; asıl risk SHA-256/crypto async worker'ı bloklaması — 80MB/s'de 30ms |
| **UI ikon+sayı+a11y+tooltip 4'lü disiplin** | Salt metin / salt ikon | Yüksek frekans güncellemede metin yorucu; ama salt ikon screen reader'ı öldürür — dördü zorunlu |
| **Filesystem Edge-Case modülü v1.0'da** | Sadece happy path | Production en sık bug burada: case insensitive collision, NFC/NFD duplicate, Windows reserved name. Linux+Win ikilisi sessiz veri kaybı yaratır |
| **NFC default internal compare** | Byte-exact compare | Türkçe ş/ğ/ı NFC vs NFD farklı binary; sync engine v2 zorunluluğu, conflict detection için şart |
| **Conflict Resolution UX kendi başına bölüm** | Generic modal | FileZilla'nın en sevilmeyen yanı; apply-to-all + checksum lazy compare + resume preference DTransfer diferansiyatörü |
| **Failure Semantics garanti tablosu + chaos test** | Implicit kontratlar | "Ne garanti ediyoruz" cevabı sözle değil regression test ile; CrashPoint enum production-grade rigor |
| **Per-chunk hash v1.0'a çekildi (lazy blob)** | Whole-file hash veya v2 | 10GB tek bit corruption = tüm dosya yeniden = 2026 standardı kabul etmez; lazy storage queue.db şişmesini önler |
| **fsync DataOnly default + parent dir Full** | None / Full | %95 user throughput ister; Full mode parent dir fsync olmadan POSIX rename durable değil — subtle production bug |
| **Sparse SFTP/Local v1.0** | S3 dahil dense kopya | VM image / DB dump için kritik; S3 zaten desteklemez ama Local+SFTP'de sparse 100GB → 10GB wire byte |
| **TUF benzeri Updater Anti-Rollback** | Sadece imzalı binary | Downgrade attack: vulnerable v1.0.5 imzalı eski binary kullanıcıya zorlamak; min_supported_version + manifest expiry + monotonic released_at |
| **Secret zeroization (zeroize + secrecy)** | Plain RAM bekleme | Argon2 derived key + plaintext password core dump'a sızar; secrecy::Secret compile-time Debug engelleyici |
| **Protocol Capability Tier Matrix** | Implicit assumption | "Neden burada rename atomic değil?" sorusunu sıfırlar; UI capability'ye göre seçenek disable eder, bug rapor azaltır |
| **Stall Watchdog dedicated OS thread** | Async runtime watchdog | Async runtime stall olduğunda async watchdog da donar — OS thread ile dışarıdan izle, 2sn+ blok = diagnostics dump |
| **RuntimeMetrics + RuntimeLimits structs** | "Yavaş" demekle debug | Cevaplanabilir sorular: db queue depth, adapter RPC p99, inflight bytes; soft/hard memory cap ile graceful degrade |
| **Chunk hash binary blob (SQLite ayrımı)** | SQLite chunk_hashes tablosu | Milyar mertebesi row WAL şişirir, VACUUM ağırlaşır; mmap zero-copy + per-transfer `.chunkmap` blob; UI sorguları DB'ye değmez |
| **LimitProfile (LowMemory/Desktop/Workstation/Server) auto-detect** | Statik default | 8GB laptop ile 128GB workstation aynı limit'lerle çalışmamalı; system probe (RAM, FD ulimit, headless) ile uygun profil |
| **Invalid UTF-8 path = base64 IPC + hex display** | Panic / lossy strip | Linux non-UTF-8 filename gerçek; PathTransport enum (Utf8 / RawBytes) JSON IPC'yi koruyor, raw bytes preserve |
| **Long path UNC + Explorer compat uyarı** | Sadece `\\?\` prefix | UNC `\\server\share` 260 limit ayrı; Win Explorer / robocopy / AV uyumsuzluk uyarısı UI badge ile |
| **Operational Recovery Playbooks** | Sadece engine-side garanti | "Kullanıcı ne görür" tarafı premium ürün farkı; banner/modal/toast hiyerarşisi + i18n recovery namespace + recovery_events.ndjson |
| **Directory Traversal Stream (BFS)** | Tüm ağacı tara → kuyruğa | 100K dosya FileZilla'yı dakikalarca dondurur; ilk 100 dosya stream → scheduler beslenir, UI hiç bloklanmaz |
| **.transferignore + Skipped vs Failed ayrımı** | Her dosya için modal | OS metadata (.DS_Store, Thumbs.db) sessiz skip; permission denied = Skipped (transfer durmaz), gitignore syntax kullanıcı override |
| **Staggered Connect with Jitter (250-500ms)** | Burst N paralel handshake | Fail2Ban / kurumsal firewall SYN flood algılar IP banlar; 250-500ms jitter natural client pattern, kullanıcı 3sn fark etmez |
| **Network Change Monitor (if-watch + sleep/wake)** | OS TCP timeout bekle | Kafe→ev WiFi değişiminde 2-15dk donma hissi; agresif soket atma + reconnect, premium laptop UX |
| **Application-Level Write Timeout (15sn) + TCP_USER_TIMEOUT** | Sadece SSH keepalive | Liar NAT TCP state'i drop eder, RST yollamaz, veri boşluğa gider — uygulama-level timeout sessiz hanging'i kökten çözer |
| **AV Lock Micro-Retry (Windows atomic_rename)** | Generic retry policy | Windows Defender atomic_rename anında lock koyar (ERROR_ACCESS_DENIED); transferin genel retry'ından bağımsız 50-1000ms micro-retry — "FileZilla failed, DTransfer succeeded" detayı |
| **Mtime granularity + DST/Timezone heuristic** | Strict mtime compare | FAT32 2sn granularity + WebDAV server localtime vs S3 UTC = sahte conflict modal yağmuru; tam saat katı + size match → silent skip |
| **list_dir Stream API + view_cache.db (paginated)** | Vec<RemoteEntry> | 2M dosyalı bucket OOM Killer; Stream + SQLite cache → UI 30-50MB sabit RAM, geniş listeleme desteği |
| **Windows Shared Read (FILE_SHARE_READ\|WRITE\|DELETE)** | Default exclusive read | Açık log/.pst/.sqlite yedekleme klasik Windows pain; Office VSS gerek (v1.1+) ama şimdiki shared read %80 kullanım senaryosunu kapsıyor |
| **Absolute Symlink Sanitization** | Olduğu gibi preserve | Kötü niyetli sunucu /etc/passwd → C:\Windows\... hijack saldırısı (CVE-class); SanitizeOrSkip default + power user PreserveAsIs override |
| **SQLite bundled feature** | System SQLite | Ubuntu 3.37, Debian 3.40, Alpine 3.41 — versiyon farkı WAL/JSON1/FTS5'te "user'da çalışmıyor" bug yatağı; rusqlite bundled = SQLite 3.45 sabit |
| **SQLite WAL checkpoint policy (autocheckpoint + 5dk Truncate)** | Default davranış | 16 paralel × 5sn batch write → WAL GB'a çıkar; periodic Truncate + journal_size_limit 64MB cap = production disk koruması |
| **Arc<EngineEvent> broadcast** | Clone-on-send | 6 subscriber × 1000 event/sec × String clone = glibc malloc contention; refcount++ ile sıfır deep clone |
| **AuthFailReason enum (static reason)** | String reason her zaman | %95 case'de allocation'sız variant; sadece beklenmedik durumlar Other(String) |
| **Profile-aware blocking pool** | Statik 256 thread | Workstation 64 transfer × 4 chunk = 256 peak; pool dolu, hash queue'da bekler; LimitProfile başına dinamik build |
| **chunk_size benchmark gate v0.1** | Textbook sayılarla lock-in | "80MB/s SHA-256" textbook, gerçek hardware'de farklı; criterion ile ölç, release notes'a yaz, kullanıcıya `dtransfer bench` |
| **Linux credential encrypted file fallback** | Sadece Secret Service | Alpine/WSL2/headless server'da D-Bus yok; Argon2id + XChaCha20 file backend = "Linux first-class" iddiası tutuyor |
| **WebDAV NTLM + Digest + Bearer + Negotiate** | Sadece Basic | SharePoint on-prem NTLM şart; ownCloud Bearer; auth probe + scheme negotiate olmadan WebDAV "implement edildi" denemez |
| **AV lock TransferActiveSubState (Verifying / Finalizing)** | Tek "Active" state | %100 progress sonrası 2sn nothing = "donmuş mu?" sorusu; sub-state ile "Antivirüs taraması bekleniyor..." UX premium hissi |
| **cargo-fuzz haftalık scheduled CI** | Sadece unit/integration | Protocol parser'lar ağdan untrusted input; DoS/memory exhaustion fuzz olmadan yakalanmaz; %95 coverage hedef |
| **Redacted<T> wrapper + diagnostics post-pass regex** | Default tracing Debug | RUST_LOG=debug ile diagnostics bundle'da home path/email/path sızar (KVKK/GDPR ihlali); compile-time wrapper + runtime regex defense-in-depth |
| **HTTP/2 pool tuning (S3 64 idle/host)** | hyper default 32 | 1000 küçük dosya S3 upload: default ~5dk vs tuned ~2dk; TLS handshake overhead her connection ~50ms |
| **Spec (Bölüm 1-40) vs Discovery Log (Bölüm 42) ayrımı** | Hepsini ana spec'e ekle | Spec'e prematür ekleme v1.0→v1.12 scope inflation'a yol açar; deliberate-after-review (spec, versiyonlu) ile reactive-during-coding (discovery, tarihli) farklı workflow'lar — tek dosyada iki net section |
| **Single Instance Lock (tauri-plugin-single-instance) v1.0 zorunlu** | Multi-instance permissive | DbActor + RateLimiter in-memory varsayar; iki paralel process aynı queue.db'ye yazınca SQLITE_BUSY + rate quota'lar habersiz tüketilir → 429 ban. v2 daemon mode'a kadar single instance şart |
| **Per-user install path (AppData\Local default)** | Program Files | C:\Program Files'a kurulu uygulama her auto-update'te UAC prompt; per-user install MSI seçeneği default + system-wide kurulu tespit edilirse manual update mode |
| **AV retry iki katmanlı (micro + file-size scaled macro)** | Tek sabit 2.1sn | 50GB dosya AV scan'i dakikalar sürer; 2.1sn limit yanlış Failed verir; file_size_bytes / 1GB başına +10sn macro retry + WaitingForAntivirus UX state |
| **Database path kolonları BLOB + path_kind** | TEXT (UTF-8 zorunlu) | Bölüm 12.7 Invalid UTF-8 policy ile schema inconsistency; rusqlite String conversion panics on raw bytes; BLOB + 'utf8'/'raw_bytes' kind sütunu schema bütünlüğü |
| **WAL checkpoint TRUNCATE → PASSIVE fallback** | Sadece TRUNCATE | Aktif reader varken TRUNCATE SQLITE_BUSY döner ve no-op; PASSIVE BUSY-immune ama daha az agresif; UI tarafında short transaction disiplini kombinasyonu |
| **view_cache.db startup orphan cleanup + sort lockout** | 1 saatlik cleanup tek başına | Crash anında in-process task çalışmaz, view_cache.db GB'lar büyür; startup'ta zorunlu sweep + is_complete=0 iken UI sort disable (zıplayan liste UX bug) |
| **Updater HTTP Date header clock drift check** | Sadece local Utc::now() | CMOS battery dead / yanlış set edilmiş sistem saati = updater brick; server Date header 24sn drift toleransı + kullanıcıya brick olmayan modal |
| **Headless master key injection (env var + keyfile)** | Sadece interactive prompt | Cron / CI / systemd / Docker = TTY yok = backend unlock imkansız; env var (convenience) + 0600 keyfile (security) + auto-lock disabled headless'ta |
| **Wayland D&D yanında File Picker prominence** | Sadece D&D | WebKitGTK Wayland D&D bug-prone (event'ler kaybolur); buton yan yana + Wayland session detect + "Force File Picker mode" toggle Linux güvenilirliği |
| **NTLM/Negotiate proxy v1.1'e defer (reqwest native desteği yok)** | v1.0'da implement | reqwest+hyper proxy auth scheme'lerinde NTLM/Negotiate native yok; v1.1'de WinHTTP (Windows) + sidecar (Linux) hibrit yaklaşım; v1.0'da net error message ile kullanıcıya bildirim |
| **Salt storage clarification (encrypted file / .dtransfer export / encrypt_at_rest)** | İmplicit, bölümler arası dağınık | Argon2 kullanılan 3 senaryoda salt artifact'in yanında saklanır (header'da, 16 byte rastgele, plaintext OK); OS keychain backend'lerinde Argon2 kullanılmaz, salt yönetimi relevant değil — açıkça yazılı |
| **Resume schema versioning (ResumeHeader v1)** | Implicit JSON evolution | `.dtresume` kalıcı artifact; haftalar sonra resume edilebilir; bu sürede DTransfer upgrade'de schema değişirse legacy blob; explicit schema_version + min_reader_version + crypto_suite contract ile backward/forward kompatibilite |
| **Unified backpressure model + bottleneck detection** | Parçalı backpressure (inflight/aggregator/mpsc) | Parçalar var ama uçtan uca flow control graph yok → memory ballooning / event storm / lagging UI risk; BackpressureMetrics struct + bottleneck() metodu + RuntimeMetrics entegrasyonu |
| **Cancellation guarantee per-stage table** | "Cancel immediate" varsayımı | Bazı stage'ler atomic (fsync, rename, DB commit) — mid-syscall cancel imkansız; immediate / cooperative / deferred / non-cancellable taxonomy + 10sn worst-case latency SLA |
| **S3 ETag ≠ MD5 explicit policy** | ETag = checksum varsayım | Multipart + SSE + MinIO'da ETag content MD5 değil; silent corruption riski; AWS Additional Checksums API default + LocalRehashVerify fallback + probe + UI uyarı |
| **TLS trust lifecycle (SPKI pin default)** | Sadece leaf cert pin | Modern cert rotation (Let's Encrypt 60-90 gün) leaf fingerprint pin'i useless yapar → trust fatigue; SPKI pin long-lived public key'i pin'ler, leaf rotation silent OK |
| **Log rotation + retention + crash-loop dedup** | Sadece 50MB rotation eşiği | max_file_count + max_file_age + total_cap (1GB hard) + crash-loop stack_trace_hash dedup → AppData'da GB'larca log birikimi engellenir |
| **Crypto agility contract (CryptoSuiteVersion + 2-release deprecation)** | Hardcoded current default | Argon2 params sıkılaşabilir, AEAD swap olabilir, post-quantum geçiş; her artifact'in suite_id + params_version metadata'sı; 2 major release deprecation window; lazy re-encrypt |
| **Update transaction (binary + config + DB atomic rollback)** | Sadece binary rollback | Yeni binary eski config/DB okuyamazsa app dead lock; UpdateTransaction struct ile 3'lü snapshot + 30sn IPC handshake + rollback chain |
| **Explicit Threat Model (Bölüm 41, A1-A15 + trust boundaries)** | İmplicit "her şeyden korur" varsayım | Hangi attacker'a karşı koruma + hangi alanlar out-of-scope (privileged malware, side-channel, APT) açıkça yazılı; spec kararlarının gerekçeleri mapping tablosunda izlenebilir |
| ❌ **Multi-process DB ownership (DISMISSED v1.16)** | Process ownership contract eklemek | Single Instance Lock (v1.15) + DbActor (v1.13) zaten kapsıyor; v1.0'da multi-process senaryo yok; CLI daemon mode v2 stub'da (sandbox + capability spec'i v2 scope) |
| ❌ **Runtime memory pressure adaptive (DISMISSED v1.16 → v2)** | OS PSI / memory notification entegrasyonu | v1.0'da startup-time LimitProfile detection yeterli; runtime adaptive degradation Windows API + Linux PSI integration karmaşık; v2 scope, v1.0'da statik limits + hard cap → diagnostics banner yeterli |
| ❌ **Third-party adapter sandbox (v2+ scope)** | v1.0'da implement | v1.0 sadece D Brand'ın derlediği in-process adapter'ları çalıştırır; üçüncü taraf adapter altyapısı (cgroups, Job Object, capability permissions, ABI stability) tek başına bir v2 ürün scope'u. v1.0 closed-core model bunu gereksizleştirir |

---

## 41. Threat Model & Trust Boundaries

Bu bölüm DTransfer'ın savunma kararlarının **arkasındaki gerekçeleri** formalize eder. Bölüm 1-42'deki spec kararlarının çoğu güvenlikle ilgili — symlink sanitize, secret zeroization, adapter signing, TLS pin, updater anti-rollback. Ama hangi attacker'a karşı hangi koruma? Bu netleşmeden ekosistem büyür, kararların gerekçesi unutulur.

### 41.1 Attacker Modelleri

DTransfer aşağıdaki saldırgan modellerine karşı **explicit** koruma sağlar veya sağlamaz:

| # | Saldırgan | Sınıf | Korunuyor mu | Nasıl |
|---|---|---|---|---|
| **A1** | Network MITM (passive eavesdropping) | Aktif tehdit | ✅ Tam | TLS 1.2+ + rustls + pin opsiyonu; client-side payload encryption opsiyonel |
| **A2** | Network MITM (active TLS strip / cert spoofing) | Aktif tehdit | ✅ Tam | Cert pin + SPKI pin + revocation check; pin mismatch hard fail |
| **A3** | Compromised remote server (SFTP/S3 hosting taraf) | Aktif tehdit | ⚠️ Kısmi | Sym link hijack sanitize, path traversal koruması, presigned URL TTL; server kötüye kullanırsa client-side payload encryption ile veri yine korunur |
| **A4** | Malicious adapter (kullanıcı kötü adapter install etti) | Aktif tehdit | ⚠️ Kısmi (v1.0); tam (v2.0 sandbox) | Ed25519 imza zorunlu (signed adapter'lar), Job Object zombie koruması, env_clear, stdout JSON-only kontrat; ama runtime CPU/RAM/FD limit yok (v2 sandbox) |
| **A5** | Local non-privileged attacker (aynı makinedeki başka kullanıcı) | Pasif/aktif | ✅ Tam | Credentials OS keychain veya 0600 encrypted file; queue.db kullanıcıya özel %APPDATA% / `~/.dtransfer`; secret zeroization (core dump leak koruması) |
| **A6** | Compromised local FS (ransomware encryptor, malware) | Aktif tehdit | ❌ Korunmuyor | Bu tehdit DTransfer scope'unda değil; "local FS güvenilir" varsayımı temel; endpoint security DTransfer'ın işi değil |
| **A7** | Compromised hostile AV (false positive, locking) | Pasif tehdit | ✅ Tam | AV Lock micro-retry + file-size scaled macro retry + WaitingForAntivirus state (Bölüm 14.2) |
| **A8** | Privileged local malware (root/SYSTEM seviyesinde) | Aktif tehdit | ❌ Korunmuyor | Privileged malware DPAPI'ye, keychain'e, RAM'e erişebilir; "OS güvenilir" varsayımı temel |
| **A9** | Update channel attacker (MITM update server) | Aktif tehdit | ✅ Tam | Ed25519 signed manifests, anchor pubkey compile-time, TUF benzeri anti-rollback (Bölüm 28) |
| **A10** | Update server compromise (release key çalındı) | Aktif tehdit | ⚠️ Kısmi | Anchor key rotation = forced major release (manual download); pre-compromise binary'ler etkilenmez; SLSA L2 + Azure Key Vault HSM ile signing key runner'a inmez (release pipeline güvenlik politikası) |
| **A11** | Surveillance / forensics (RAM imaging while running) | Pasif tehdit | ⚠️ Kısmi | Secrets zeroize after use; ama active session'da RAM'de plaintext duruyor — Page lock (VirtualLock/mlock) v1.1 |
| **A12** | Data leak via diagnostics bundle | Pasif tehdit | ✅ Tam | Tracing PII redaction (Redacted<T> wrapper + diagnostics post-pass regex); presigned URL masking; user paths sanitize (Bölüm 17.3.1) |
| **A13** | Cross-app data exfiltration (other apps reading DTransfer state) | Pasif tehdit | ✅ Tam (Windows/macOS); ⚠️ Linux (Secret Service paylaşımlı) | DPAPI per-user, Keychain per-user; Linux'ta D-Bus üzerinden Secret Service `dtransfer:*` namespace |
| **A14** | DoS via crafted file (zip bomb, recursive symlink, massive sparse file) | Aktif tehdit | ✅ Tam | Symlink max_depth=8, sparse file detection, list_dir stream + view_cache OOM koruması (Bölüm 9.1 + 19.8) |
| **A15** | DoS via crafted protocol response (malformed SFTP packet, huge XML) | Aktif tehdit | ✅ Tam | Fuzz testing (Bölüm 27), max_packet_size limits, XML lenient parser with limits |

### 41.2 Trust Boundaries

DTransfer'ın "güvenilir kabul ettiği" sistemler ve sınırları:

```
┌─────────────────────────────────────────────────────────────────┐
│  GÜVENİLİR ALAN (Trusted Computing Base)                       │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ OS Kernel + Bootloader                                  │   │
│  │ — Verilen tüm güvenlik garantileri buna güveniyor       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ OS Crypto Provider (DPAPI / Keychain / Secret Service)  │   │
│  │ — Master key derivation, secret storage                 │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ DTransfer Main Binary (imzalı, integrity verified)      │   │
│  │ — Tüm spec mantığını içerir                             │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Protocol Adapters (SFTP, S3, WebDAV, Local)             │   │
│  │ — Compile-time bağlı, in-process trait implementations │   │
│  │ — Data plane ana Rust motorunda                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              ║
                              ║ (trust boundary)
                              ║
┌─────────────────────────────────────────────────────────────────┐
│  GÜVENİLMEZ ALAN (Untrusted)                                    │
│                                                                  │
│  • Remote servers (SFTP/S3/WebDAV/cloud APIs)              │
│  • Network (LAN, Internet, intermediate proxies)                │
│  • Third-party adapter extension (v2+, henüz aktif değil)         │
│  • User-supplied files (.dtresume from external source)        │
│  • Update server (signed manifest gerekli)                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Güven kararı kuralları:**

1. **Trusted alana giren her input verify edilir.** Update manifest'inin anchor pubkey doğrulaması, TLS cert chain validation, `.dtresume` schema check.
2. **Trusted alandan çıkan her output sanitize edilir.** Diagnostics bundle PII redaction, log presigned URL masking, adapter'a env_clear.
3. **Trust boundary cross'larında explicit serialization.** JSON-RPC IPC adapter'le, signed manifest update server'la, normalized path remote server'la.

### 41.3 Güvenlik Kararlarının Gerekçeleri (Mapping)

Spec'teki major güvenlik kararlarının hangi attacker model'e karşı olduğunu açık şekilde göster:

| Spec Kararı | Bölüm | Karşı koruma | Attacker |
|---|---|---|---|
| TLS 1.2+ minimum, SHA-1 reject | 33 | MITM passive, SHA-1 collision | A1, A2 |
| Cert pin + SPKI pin | 33.5 | Cert spoofing, hostile CA | A2 |
| Updater Ed25519 + anti-rollback + clock drift | 25 | Downgrade attack, MITM update | A9, A10 |
| OS Credential Store (DPAPI/Keychain/SecretService) | 22 | Local non-priv attacker | A5, A13 |
| Encrypted file fallback (Argon2id + XChaCha20) | 22.1 | Local non-priv attacker (headless) | A5 |
| Secret zeroization (zeroize + secrecy) | 10.6 | Core dump leak, swap | A11 |
| Page lock (VirtualLock/mlock) v1.1 | 10.6 stub | RAM imaging | A11 |
| Symlink sanitize (SanitizeOrSkip default) | 9.1 | Hostile server | A3 |
| Path traversal koruması | 9 | Hostile server | A3 |
| Client-side encryption (XChaCha20-Poly1305) | 10 | Compromised storage | A3 |
| Tracing PII redaction + Redacted<T> | 14.3.1 | Bundle data leak | A12 |
| Presigned URL masking | 14.3 | Bundle credential leak | A12 |
| AV Lock micro-retry + WaitingForAntivirus | 11.2 | Hostile AV | A7 |
| Fuzz testing (cargo-fuzz weekly) | 24 | Malformed protocol input | A15 |
| Directory traversal stream + view_cache pagination | 6.1 + 19.8 | DoS via massive listing | A14 |
| Symlink max_depth=8, sparse detection | 9.1 | DoS via crafted file | A14 |
| SLSA L2 + Azure Key Vault HSM signing | 37 | Compromised release pipeline | A10 |
| Anchor key rotation = forced major | 25 | Compromised anchor key | A10 |

### 41.4 Explicitly Unsupported (Out-of-Scope)

DTransfer **şunlara karşı korumaz** ve bu bilinçli karar:

- **Privileged malware** (root/SYSTEM): OS güvenlik modeli zaten kırılmış; DTransfer bu seviyede correction yapmaya çalışmaz
- **Compromised hardware** (TPM bypass, cold boot attack): Endpoint security'nin işi
- **Side-channel attacks** (timing, cache, power analysis): General-purpose code; constant-time crypto crate'leri kullanılır (chacha20poly1305, argon2) ama side-channel hardening yok
- **Local FS compromise** (ransomware): Endpoint security'nin işi; DTransfer cloud backup pattern'i ile mitigation sağlayabilir ama doğrudan koruma yok
- **Nation-state actors with custom malware**: DTransfer "consumer / SMB grade" güvenlik sunar; APT-grade tehdit modellemesi out-of-scope
- **Insider threats with credential access**: Eğer attacker kullanıcının OS oturumuna sahipse, OS-trusted boundary'de — koruma yok

Bu sınırlar **explicit** olarak yazılır çünkü implicit "her şeyden korur" iddiası güven sağlamaz, yanlış güvende olmaya yol açar.

### 41.5 Threat Model Lifecycle

Yeni feature eklemeden önce **bu bölüme bak**: hangi attacker model'e etkisi var? Yeni attack surface açıyor mu? Mevcut korumalardan herhangi birini zayıflatıyor mu?

Spec'te yeni güvenlik kararı geldiğinde Bölüm 41.3 tablosuna satır eklenir — karar gerekçesi kalıcı olarak görünür.

---


### 41.6 Per-Component Trust Levels (Explicit Tablo)

Bölüm 41.2'deki genel diyagramın somut karşılığı — her component için **trust seviyesi** açıkça yazılı:

| Boundary | Trust Level | Davranış |
|---|---|---|
| Local process memory | **trusted** | OS güvenlik modeli içinde; secret'lar zeroize edilir (Bölüm 12.6) |
| Local config files | **trusted** | Kullanıcı kontrolünde; permission check + integrity validation |
| OS Credential Store (DPAPI/Keychain/Secret Service) | **trusted** | OS crypto provider'a delegasyon |
| Encrypted file fallback (`credentials.enc`) | **trusted** (Argon2id+XChaCha20) | Master password sınırı |
| Update manifest (Ed25519 signed) | **trusted after verify** | Anchor pubkey doğrulaması zorunlu |
| Remote server metadata (`mtime`, `size`, `etag`) | **untrusted** | Identity için tek başına yeterli değil (Bölüm 5.2) |
| Remote directory listings | **partially trusted** | Symlink sanitize edilir, path traversal reject (Bölüm 11) |
| Remote file content (in transit) | **untrusted until verified** | Hash check yapılabiliyorsa yapılır; yoksa best-effort |
| S3 ETag | **NOT treated as integrity proof** | Multipart/SSE'de content MD5 değil; Additional Checksums API kullanılır (Bölüm 10.6) |
| WebDAV `If-Match` ETag | **partially trusted** | Concurrency control için, integrity için değil |
| TLS server certificate | **trusted after pin/store match** | Pin mismatch hard fail; sistem root store fallback |
| Local filesystem | **trusted** (within OS) | Ransomware/malware DTransfer scope dışı (Bölüm 41.4) |
| User input (paths, profile data) | **validated** | NormalizedPath sanitize, reserved name reject, path traversal check |
| Third-party adapter input | **UNSUPPORTED (v1.0)** | v1.0'da üçüncü taraf adapter yok; v2+ sandbox spec'i ile birlikte |
| Update server (HTTPS endpoint) | **untrusted channel, trusted payload** | TLS + signed manifest çift katman |
| Diagnostics bundle (kullanıcı paylaşımı) | **PII redacted before share** | Redacted<T> wrapper + post-pass regex (Bölüm 16.3) |

**Trust ihlali davranışı:** Trusted alana giren input verify edilmezse → spec ihlali, security review trigger. Untrusted alandan gelen veri verify edilmeden trusted alana geçirilirse → spec ihlali.


---

## 42. Implementation Discovery Log

Bu bölüm **kod yazarken keşfedilen** edge-case'ler, gotchas ve workaround'lar için canlı kayıt. Bölüm 1-42 mimari spec — donmuş, sadece major versiyon bump'ı ile değişir. Bu bölüm ise tarihli, append-only — versiyon bump yok.

### 42.1 Workflow Kuralları

**Buraya eklenir:**
- Implementation sırasında karşılaşılan platform-specific bug (Windows API edge case, Linux distro quirk)
- Library quirk (`russh`, `suppaftp`, `aws-sdk-s3`, `vue-virtual-scroller` undocumented davranış)
- Undocumented protocol behavior (server-specific SFTP/FTP/S3 sapması)
- Performance trap (profiler ile yakalanan beklenmedik bottleneck)
- Debugging hikayesi (uzun süreli investigation'ın sonucu)

**Buraya eklenmez:**
- Yeni feature (v1.15+ spec değişikliği)
- Mimari karar değişikliği (Bölüm 40'a dokunmadan önce spec review)
- API yeniden tasarımı (v2.0 master plan'ına gider)

**Ayrım kuralı:** *"Bunu önceden bilebilir miydim?"* Cevap **hayır** ise → Discovery Log. **Evet** ise → ya zaten spec'te var ya yeni versiyon ihtiyacı.

### 42.2 Note Format

```md
### [YYYY-MM-DD] [Kategori] — Kısa Başlık

**Bağlam:** Hangi modülde / bölümde çalışırken?
**Bulgu:** Ne keşfedildi?
**Çözüm:** Nasıl handle edildi?
**Referans:** Commit hash, PR link, external doc link (varsa)
```

**Kategori etiketleri:** `Windows-Specific`, `Linux-Specific`, `Protocol-FTP`, `Protocol-SFTP`, `Protocol-S3`, `Protocol-WebDAV`, `Library-russh`, `Library-aws-sdk`, `Library-vue-virtual-scroller`, `Library-tokio`, `Library-rusqlite`, `Performance-Trap`, `Debugging-Story`, `Build-CI`.

Tek not birden fazla kategoride olabilir, virgülle ayır: `[Windows-Specific, Library-tokio]`.

### 42.3 Kategoriler (Boş — Faz 1 başlayınca dolacak)

#### Windows-Specific
*(Faz 1 başlangıcında boş; AV interaction, NTFS junction, UAC elevation, CreateFile flags gibi keşifler buraya)*

#### Linux-Specific
*(Distro farklılıkları, glibc quirks, AppImage runtime issues, sandbox interactions)*

#### Protocol Quirks
*(SFTP server-specific davranış, FTP MLSD vs LIST farkları, S3 region routing, WebDAV server quirks)*

#### Library Gotchas
*(russh undocumented behavior, suppaftp edge case, aws-sdk timeout patterns, vue-virtual-scroller scroll restoration)*

#### Performance Traps
*(Profiler bulguları, beklenmedik hotspot'lar, async runtime contention)*

#### Debugging Stories
*(Uzun süreli investigation'lar, "bu bug 3 günümüzü aldı" hikayeleri)*

#### Build / CI
*(GitHub Actions runner quirks, signtool davranışı, cargo-deb edge case, AppImage build issues)*

### 42.4 Örnek Format (gelecekte eklenecek not şablonu için)

```md
### [2026-08-12] [Windows-Specific, Library-tokio] — MoveFileEx Network Drive Edge Case

**Bağlam:** AV Lock micro-retry implementasyonu (Bölüm 14.2 atomic_rename_with_av_retry)
**Bulgu:** Network drive (`\\server\share\`) üzerinde `MoveFileEx` ERROR_GEN_FAILURE (31)
return ediyor — spec'teki ACCESS_DENIED (5) ve SHARING_VIOLATION (32) listesinde
yok. Local NTFS'te tetiklenmiyor; sadece SMB üzerinde. Microsoft docs'ta net
açıklama yok, ama SMB redirector'da AV scan interaction varmış.

**Çözüm:** Micro-retry whitelist'ine raw_os_error 31 eklendi, ek 2sn timeout ile
network drive heuristic (`is_network_path()` ile detect). Bölüm 14.2 ana spec
değişmedi — bu sadece edge case handling, retry kuralının genişletilmesi.

**Referans:** commit abc1234, https://learn.microsoft.com/...
```

Bu örnek henüz keşfedilmemiş hipotetik bir senaryo — format göstermek için. Gerçek not'lar yukarıdaki kategorilere tarih sırasıyla eklenir.

### 42.5 Spec-level Değişiklik mi, Discovery mi?

Karar verme rehberi:

| Soru | Discovery Log | Spec Bump (v1.15+) |
|---|---|---|
| Yeni bir capability mi geliyor? | Hayır | **Evet** |
| Mevcut bir capability'nin **gizli** davranışını mı keşfettim? | **Evet** | Hayır |
| Bu bilgi başka bir geliştiriciye "neden böyle yapmamız gerekti" sorusunu cevaplar mı? | **Evet** | (zaten spec'te) |
| Bu değişiklik public API contract'ını etkiliyor mu? | Hayır | **Evet** |
| Bu değişiklik kullanıcının config'ini migrate etmesini gerektiriyor mu? | Hayır | **Evet** |
| Halihazırda spec'te yazılı bir kuralı **netleştiriyor** mu (yeni kural eklemiyor)? | **Evet** | Hayır |

**Tereddütte kalırsan:** Discovery'e ekle, sprint sonunda team review'da "bu aslında spec değişikliği mi olmalı?" sorusu sor. Tersini yapmak (spec'e prematür ekleme) v1.0→v1.12 pattern'ine geri döner.

---

*DTransfer Teknik Dökümanı — v1.16 · 🔒 Implementation Reference Frozen (monolitik)*
*44 bölüm · 8 AI review turu (Gemini ×5, ChatGPT ×4) + v1.13 errata + v1.14 monolitik entegrasyon + v1.15 platform/operational completeness + v1.16 second-order completeness + Threat Model + Discovery Log workflow · v1.7→v1.8→v1.9→v1.10→v1.11→v1.12→v1.13→v1.14→v1.15→v1.16*
*Windows + Linux birinci sınıf · GPL-3.0-or-later · Tek-dosya referans · Bölüm 1-42 spec (donmuş, major bump ile değişir) · Bölüm 41 Threat Model (formalize) · Bölüm 42 Discovery Log (canlı, append-only, versiyon bump yok)*

***🔒 Spec dondu, Discovery Log canlı. Kod yazma zamanı.***


---

*DTransfer Teknik Dökümanı — v2.1 · 🔒 Semantic Contract Frozen*
*42 bölüm · Lisans: GPL-3.0-or-later · Tek-dosya referans*
*Mimari spec (Bölüm 1-40) donmuş · Implementation Discovery Log (Bölüm 42) canlı*

***🔒 Spec dondu, Discovery Log canlı. Şimdi gerçekten kod yazma zamanı.***
