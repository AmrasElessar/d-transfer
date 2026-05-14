//! Queue DB şeması ve migration runner — Bölüm 15.2.
//!
//! Faz 3 scope: queue.db skeleton. v1.15'teki BLOB path kolonları henüz aktif
//! değil (Bölüm 12.7 PathTransport tipi Faz 4'te eklenecek); şimdilik TEXT
//! kolonlar + invariant: kayıt edilen path'ler UTF-8 valid. Migration v2
//! geldiğinde TEXT → BLOB upgrade ayrı bir migration ile yapılır.

use rusqlite::{params, Connection};

/// Tüm yeni bağlantılarda çalıştırılması gereken pragmalar.
///
/// **Neden her connection için**: SQLite PRAGMA'ları çoğunlukla connection-local
/// kapsam taşır. `journal_mode = WAL` DB seviyesinde persiste olur ama
/// `busy_timeout`, `synchronous`, `foreign_keys` her yeni handle'da default'a
/// döner — bu yüzden bağlantı açılışında tek noktadan uygulanır.
pub fn configure_connection(conn: &Connection) -> rusqlite::Result<()> {
    // `journal_mode = WAL` DB dosyasının kalıcı modunu WAL'a alır. Reader'lar
    // writer'ı bloklamaz; concurrent okuma writer + tek seri yazar (DbActor)
    // ile birleşince SQLITE_BUSY pratikte sıfırlanır.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // NORMAL: WAL ile durability/perf dengesi — fsync her commit yerine
    // checkpoint anında. Power-loss penceresi son ~bir kaç işlem; queue.db
    // için kabul edilebilir (Bölüm 15.7 trade-off).
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // 5 sn: WAL checkpoint yazıcısı writer lock tutarken reader/writer'a
    // yeterli zaman tanır. Default 0 ms olduğundan WAL ile sık SQLITE_BUSY
    // üretir; 5000 ms hem CI hem yavaş HDD'de stabil.
    conn.pragma_update(None, "busy_timeout", 5_000)?;
    Ok(())
}

/// Migration tablosunu hazırlar, sonra eksik migration'ları sırayla uygular.
///
/// Idempotent: yeniden çağrılırsa `migrations` tablosundaki kayıtlara göre
/// already-applied versionları atlar.
pub fn apply_migrations(conn: &mut Connection) -> rusqlite::Result<()> {
    configure_connection(conn)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS migrations (
            version    INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let applied: bool = conn.query_row(
            "SELECT 1 FROM migrations WHERE version = ?1",
            params![version],
            |_| Ok(true),
        ).unwrap_or(false);

        if applied {
            continue;
        }

        // Tek migration = tek transaction. Yarı uygulanmış şema riski yok.
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO migrations (version, applied_at) VALUES (?1, ?2)",
            params![version, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
    }

    Ok(())
}

/// Versiyon → SQL eşlemesi. Yeni alanlar her zaman ALTER TABLE / yeni tablo
/// olarak EKLENİR, mevcut migration'ların SQL'i değiştirilmez.
const MIGRATIONS: &[(u32, &str)] = &[
    (
        1,
        "CREATE TABLE IF NOT EXISTS transfers (
            id              TEXT PRIMARY KEY,
            profile_id      TEXT NOT NULL,
            direction       TEXT NOT NULL CHECK (direction IN ('upload', 'download')),
            state           TEXT NOT NULL,
            priority        INTEGER NOT NULL DEFAULT 0,
            local_path      TEXT NOT NULL,
            remote_path     TEXT NOT NULL,
            bytes_total     INTEGER NOT NULL DEFAULT 0,
            bytes_done      INTEGER NOT NULL DEFAULT 0,
            chunk_size      INTEGER NOT NULL,
            retry_count     INTEGER NOT NULL DEFAULT 0,
            last_error      TEXT,
            schema_version  INTEGER NOT NULL DEFAULT 1,
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL,
            started_at      TEXT,
            completed_at    TEXT
        );
        CREATE INDEX IF NOT EXISTS transfers_state_idx
            ON transfers(state, priority DESC, created_at);
        CREATE INDEX IF NOT EXISTS transfers_profile_idx
            ON transfers(profile_id);",
    ),
    // v2: ConnectionProfile kalıcı kaydı (Bölüm 25). Sırlar OS keystore'da;
    // bu tabloda sadece bağlantı meta'sı. `transfers.profile_id` ileride bu
    // tabloya FK olarak bağlanabilir — şimdilik soft-link (legacy in-memory
    // local profilleriyle birlikte yaşadığı için zorlamıyoruz).
    (
        2,
        "CREATE TABLE IF NOT EXISTS profiles (
            id            TEXT PRIMARY KEY,
            name          TEXT NOT NULL,
            protocol      TEXT NOT NULL CHECK (protocol IN ('local','sftp','s3','webdav')),
            host          TEXT,
            port          INTEGER,
            username      TEXT,
            remote_root   TEXT,
            local_root    TEXT,
            auth_method   TEXT NOT NULL CHECK (auth_method IN ('none','password','publicKey')),
            options_json  TEXT NOT NULL DEFAULT '{}',
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS profiles_name_idx ON profiles(name);",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn open_temp() -> (NamedTempFile, Connection) {
        let file = NamedTempFile::new().expect("tempfile");
        let conn = Connection::open(file.path()).expect("open conn");
        (file, conn)
    }

    #[test]
    fn apply_migrations_creates_schema() {
        let (_f, mut conn) = open_temp();
        apply_migrations(&mut conn).unwrap();

        // transfers tablosu var mı?
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='transfers'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "transfers table must exist");

        // index'ler var mı?
        let idx_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='transfers_state_idx'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1, "state index must exist");
    }

    #[test]
    fn apply_migrations_is_idempotent() {
        let (_f, mut conn) = open_temp();
        apply_migrations(&mut conn).unwrap();
        // İkinci çağrı no-op olmalı, hata vermemeli
        apply_migrations(&mut conn).unwrap();

        let migration_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM migrations", [], |r| r.get(0))
            .unwrap();
        // MIGRATIONS dizisinin uzunluğu kadar kayıt olmalı, her biri tek kez.
        assert_eq!(
            migration_rows as usize,
            MIGRATIONS.len(),
            "each migration recorded exactly once"
        );
    }

    #[test]
    fn profiles_table_created_in_v2() {
        let (_f, mut conn) = open_temp();
        apply_migrations(&mut conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='profiles'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "profiles table must exist after v2 migration");

        let idx_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='profiles_name_idx'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1, "profiles_name_idx must exist");
    }

    #[test]
    fn wal_mode_is_enabled() {
        let (_f, mut conn) = open_temp();
        apply_migrations(&mut conn).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn busy_timeout_is_configured() {
        let (_f, mut conn) = open_temp();
        apply_migrations(&mut conn).unwrap();
        let timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(timeout, 5_000);
    }
}
