# D-Transfer · Repo Standards

> **Hedef konum:** `C:\Projeler\d-transfer\REPO_STANDARDS.md`
> Reponun köğüne kopyalayıp commit'leyin. Sonraki düzenlemeler bu dosyaya bağlı kalmalı — değişiklik gerekirse burayı da güncelleyin.
>
> **Snapshot:** 2026-05-14 (D Brand README + about/topics align sonrası)

---

## 1. Locked GitHub metadata

| Alan | Değer |
|---|---|
| **Owner/Repo** | `AmrasElessar/d-transfer` |
| **Visibility** | public |
| **Default branch** | `main` |
| **License (SPDX)** | `GPL-3.0` (or-later — README'de açık) |
| **Description** | `Deterministic, crash-resilient file-transfer client unifying SFTP/S3/WebDAV/Local under one Rust engine. Tauri v2 + Vue 3 + Rust. Queue persistence, atomic finalization, OS-native credential vault. Pre-alpha · GPL-3.0+ · Windows + Linux.` |
| **Homepage** | (boş — pre-alpha, release yok henüz; v1.0'da releases URL'i girilecek) |
| **Topics (19)** | `crash-resilient, d-brand, desktop, deterministic, file-transfer, gpl, linux, open-source, pre-alpha, queue, rust, s3, sftp, tauri, tauri-v2, transfer, vue, webdav, windows` |

Değişiklik yaparsanız bu tabloyu güncelleyin + remote'a yansıtın.

---

## 2. README iskeleti (D Brand template — kaynak: d-terminal)

### 2.1 Bölüm sırası (kanonik)

1. **Header** — center-aligned, başlık + İngilizce tagline + TR/EN alt-tagline + bilingual notice
2. **🎬 Demo** — video / screenshot / "coming soon" placeholder
3. **Badge row** — License → Status → Platform → Tech → D Brand → Spec (donmuş ise)
4. **📌 Kısaca** (TR) + collapsible `🇬🇧 At a glance` (EN)
5. **🆕 Şu ana kadar yapılanlar / What's done so far** (bullet list)
6. **🎯 Vizyon / Vision** (opsiyonel)
7. **✨ Öne Çıkan Özellikler / Key Features**
8. **🛠️ Teknoloji / Tech Stack** + mimari döküman linkleri
9. **🗺️ Yol Haritası / Roadmap**
10. **📥 Kurulum / Installation** + **🚀 İlk Adımlar / Quick Start** (release varsa)
11. **🛡️ Güvenlik Tarama / Security Scan Results** (release varsa)
12. **🤝 Katkı / Contributing**
13. **🎨 D Brand Ailesi / D Brand Family**
14. **💖 Sponsorlar / Sponsors**
15. **❤️ Destekle / Support**
16. **📜 Lisans / License**

Pre-alpha'da 10-11 düşülebilir; **sıralama bozulmaz**.

### 2.2 Header pattern

```markdown
<div align="center">

# D-Transfer

**Deterministic, crash-resilient transfer infrastructure client**

*SFTP · S3 · WebDAV · Local — tek motor, kuyruk dayanıklılığı, doğrulanabilir transfer*
*SFTP · S3 · WebDAV · Local — single engine, queue durability, verifiable transfers*

🌐 **TR · EN** — Bu README iki dillidir / This README is bilingual (English collapsibles below each section)

</div>
```

### 2.3 Badge row

```
[License: GPL-3.0+]   (mavi)
[Status: pre-alpha]   (turuncu)
[Platform: Windows 10/11 · Linux]   (mavi)
[Tauri v2] [Vue 3] [Rust stable]
[D Brand]
[Spec: v2.1 🔒 frozen]   (success yeşil — spec dondurulmuş olduğundan özel)
```

### 2.4 Bilingual yapı

- Ana akış TR + `<details><summary>🇬🇧 ...</summary>` ile EN
- Spec referansları kısaltılmadan tam isimle: "Implementation Discovery Log (Bölüm 42)"

---

## 3. Tech stack & status

- **Status:** pre-alpha (release yok — mimari spec v2.1 donmuş, Faz 1-4 + sistem katmanları kod tarafında mevcut)
- **Core:** Tauri v2 (Rust core + WebView2 / WebKitGTK)
- **Frontend:** Vue 3
- **Storage:** SQLite WAL + DbActor pattern
- **Protokoller:** SFTP (russh 0.54 + russh-sftp 2.1), Local, S3 (planlı), WebDAV (planlı)
- **Target:** Windows 10/11 + Linux (Ubuntu 22.04+, Fedora 38+, Debian 12+) first-class; macOS topluluk katkısı
- **Architecture:** spec dondurulmuş — `dtransfer-teknik-dokuman-v2_1.md`

---

## 4. Lisans

- **GPL-3.0-or-later** (SPDX: `GPL-3.0`). README badge'i ve `LICENSE` dosyası tutarlı kalmalı.
- Spec/mimari dökümanları repo içinde aynı lisans altında.

---

## 5. Commit mesaj stili

Conventional commits. Recent log'dan gözlemlenen prefix'ler:

- `feat(readme): ...`, `fix(readme): ...`, `docs(readme): ...`
- `chore: ...` — config / FUNDING / dependency bump
- `feat(<area>): ...`, `fix(<area>): ...` — kod (engine, adapter, queue, ...)
- `docs(spec): ...` — spec dökümanı revizyonu (major bump'la coupled)

Dil: TR veya EN; tutarlı olunmalı. "Why" 1 cümleyle.

---

## 6. Dosya hijyeni

- Adı `:` veya `\` içeren dosyalar **commit'lenmez** — yol parse hatası kalıntısı.
- **Zorunlu:** `README.md`, `LICENSE`, `.github/FUNDING.yml`
- **Tercih edilen:** `.gitignore` (IDE/build), `docs/` (uzun teknik döküman)
- Spec dökümanı (`dtransfer-teknik-dokuman-v2_1.md`) repo köğünde tutulur — README'den referanslıdır.
- Push öncesi `git status` kontrol.

---

## 7. Repo-spesifik notlar

- **Spec v2.1 donmuş** — Bölüm 1-40 major bump olmadan değişmez. Implementation Discovery Log (Bölüm 42) canlı; her sprint güncellenir.
- **State machine validatörü** — `Active → Queued`, `Verifying/Finalizing → Failed` crash recovery rotaları; README'deki claim'lerle koddaki davranış senkron tutulmalı.
- **`profile_id` keyed rate limit** — host keyed değil; aynı servisin farklı hesapları birbirini etkilemez (README'de vurgulu, kodda korunmalı).
- **OS-native credential vault** — Windows Credential Manager / macOS Keychain / Linux Secret Service; sırlar UI'a düşmez (claim README'de — değişirse README de güncellenir).
- **Pre-alpha rozeti** — release çıkana kadar kalır; v1.0'da `Status` badge `stable`'a + `Homepage` releases URL'ine güncellenir.
