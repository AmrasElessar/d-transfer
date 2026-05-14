//! `audit.db` schema + migration runner.
//!
//! Queue/transfer veri tabanından **ayrı** tutulur — KVKK/GDPR silme hakkı
//! (Bölüm 17.4) kullanıcı talebinde audit verisini siler ama transfer
//! kuyruğunu etkilemez. WAL mode, normal sync — durability transfer DB'si
//! kadar kritik değil (kayıp 500ms'lik son batch olur).

use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 1;

pub fn initialize(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    let tx = conn.transaction()?;
    tx.execute(
        "CREATE TABLE IF NOT EXISTS audit_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
         )",
        [],
    )?;
    let current: Option<i64> = tx
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM audit_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .ok();

    if current.is_none() {
        tx.execute(
            "INSERT INTO audit_meta(key, value) VALUES ('schema_version', ?1)",
            rusqlite::params![SCHEMA_VERSION.to_string()],
        )?;
    } else if current.unwrap() > SCHEMA_VERSION {
        return Err(rusqlite::Error::QueryReturnedNoRows); // futureschema
    }

    tx.execute(
        "CREATE TABLE IF NOT EXISTS audit_events (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            occurred_at   TEXT NOT NULL,
            kind          TEXT NOT NULL,
            profile_id    TEXT,
            transfer_id   TEXT,
            payload_json  TEXT NOT NULL,
            masked_flag   INTEGER NOT NULL DEFAULT 1
         )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_occurred ON audit_events(occurred_at)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_kind ON audit_events(kind)",
        [],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_gets_schema_v1() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&mut conn).unwrap();
        let version: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM audit_meta WHERE key='schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn idempotent_initialize() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&mut conn).unwrap();
        initialize(&mut conn).unwrap(); // ikinci çağrı bozmasın
    }
}
