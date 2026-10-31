use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

pub const MIGRATIONS: &[(i64, &str, &str)] = &[
    (1, "001_core", include_str!("../migrations/001_core.sql")),
    (
        2,
        "002_products",
        include_str!("../migrations/002_products.sql"),
    ),
];

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("migration {0} failed: {1}")]
    Migration(i64, String),
    #[error("internal error: state lock poisoned")]
    LockPoisoned,
}

/// Application state holding the single SQLite connection.
pub struct DbState {
    conn: Mutex<Connection>,
}

impl DbState {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T, DbError>) -> Result<T, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        f(&conn)
    }
}

pub fn now_utc() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn open(path: &Path) -> Result<Connection, DbError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    apply_pragmas(&conn)?;
    run_migrations(&mut conn)?;
    Ok(conn)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn open_in_memory() -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory()?;
    apply_pragmas(&conn)?;
    run_migrations(&mut conn)?;
    Ok(conn)
}

fn apply_pragmas(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}

fn applied_versions(conn: &Connection) -> Result<HashSet<i64>, DbError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;
    let mut stmt = conn.prepare("SELECT version FROM schema_migrations")?;
    let versions = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<HashSet<i64>, _>>()?;
    Ok(versions)
}

/// Applies pending migrations inside transactions. Safe to call repeatedly.
pub fn run_migrations(conn: &mut Connection) -> Result<(), DbError> {
    let applied = applied_versions(conn)?;
    for (version, name, sql) in MIGRATIONS {
        if applied.contains(version) {
            continue;
        }
        let tx = conn.transaction().map_err(DbError::Sqlite)?;
        tx.execute_batch(sql)
            .map_err(|e| DbError::Migration(*version, e.to_string()))?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![version, name, now_utc()],
        )
        .map_err(|e| DbError::Migration(*version, e.to_string()))?;
        tx.commit().map_err(DbError::Sqlite)?;
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct DbInfo {
    pub schema_version: i64,
    pub applied_migrations: i64,
    pub table_count: i64,
}

pub fn info(conn: &Connection) -> Result<DbInfo, DbError> {
    let schema_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    let applied_migrations: i64 =
        conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))?;
    let table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    Ok(DbInfo {
        schema_version,
        applied_migrations,
        table_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_on_fresh_database() {
        let conn = open_in_memory().unwrap();

        let info = info(&conn).unwrap();
        assert_eq!(info.schema_version, MIGRATIONS.last().unwrap().0);
        assert_eq!(info.applied_migrations, MIGRATIONS.len() as i64);
        assert!(info.table_count >= 15, "expected core tables, got {}", info.table_count);
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn system_units_are_seeded() {
        let conn = open_in_memory().unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM units WHERE is_system = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 14);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let conn = open_in_memory().unwrap();

        let result = conn.execute(
            "INSERT INTO branches (id, business_id, name, created_at, updated_at)
             VALUES ('b1', 'missing-business', 'Main', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(result.is_err(), "FK violation must fail");
    }

    #[test]
    fn product_requires_unique_sku_per_business() {
        let conn = open_in_memory().unwrap();

        let now = now_utc();
        conn.execute(
            "INSERT INTO businesses (id, name, created_at, updated_at) VALUES ('biz1', 'Shop', ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (id, business_id, name, sku, base_unit_id, created_at, updated_at)
             VALUES ('p1', 'biz1', 'Cement', 'CEM-01', 'sys-unit-bag', ?1, ?1)",
            params![now],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO products (id, business_id, name, sku, base_unit_id, created_at, updated_at)
             VALUES ('p2', 'biz1', 'Cement 2', 'CEM-01', 'sys-unit-bag', ?1, ?1)",
            params![now],
        );
        assert!(dup.is_err(), "duplicate SKU must fail");
    }
}
