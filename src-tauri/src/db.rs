use rusqlite::{Connection, params};
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
    #[error("invalid input: {0}")]
    InvalidInput(String),
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
pub struct Business {
    pub id: String,
    pub name: String,
    pub tax_id: Option<String>,
    pub currency: String,
    pub timezone: String,
    pub date_format: String,
    pub language: String,
}

pub fn list_businesses(conn: &Connection) -> Result<Vec<Business>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, tax_id, currency, timezone, date_format, language
         FROM businesses ORDER BY name",
    )?;
    let businesses = stmt
        .query_map([], |row| {
            Ok(Business {
                id: row.get(0)?,
                name: row.get(1)?,
                tax_id: row.get(2)?,
                currency: row.get(3)?,
                timezone: row.get(4)?,
                date_format: row.get(5)?,
                language: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(businesses)
}

pub fn create_business(
    conn: &Connection,
    name: &str,
    tax_id: Option<&str>,
) -> Result<Business, DbError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::InvalidInput(
            "business name must not be empty".to_string(),
        ));
    }

    let tax_id = tax_id.map(str::trim).filter(|value| !value.is_empty());
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();

    conn.execute(
        "INSERT INTO businesses (id, name, tax_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, tax_id, timestamp],
    )?;

    Ok(conn.query_row(
        "SELECT id, name, tax_id, currency, timezone, date_format, language
         FROM businesses WHERE id = ?1",
        params![id],
        |row| {
            Ok(Business {
                id: row.get(0)?,
                name: row.get(1)?,
                tax_id: row.get(2)?,
                currency: row.get(3)?,
                timezone: row.get(4)?,
                date_format: row.get(5)?,
                language: row.get(6)?,
            })
        },
    )?)
}

#[derive(Debug, serde::Serialize)]
pub struct Category {
    pub id: String,
    pub business_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

pub fn list_categories(conn: &Connection, business_id: &str) -> Result<Vec<Category>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, parent_id, name, description, is_active
         FROM categories WHERE business_id = ?1 ORDER BY sort_order, name",
    )?;
    let categories = stmt
        .query_map(params![business_id], |row| {
            Ok(Category {
                id: row.get(0)?,
                business_id: row.get(1)?,
                parent_id: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(categories)
}

pub fn create_category(
    conn: &Connection,
    business_id: &str,
    parent_id: Option<&str>,
    name: &str,
    description: Option<&str>,
) -> Result<Category, DbError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::InvalidInput(
            "category name must not be empty".to_string(),
        ));
    }

    let business_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM businesses WHERE id = ?1)",
        params![business_id],
        |row| row.get(0),
    )?;
    if !business_exists {
        return Err(DbError::InvalidInput("business does not exist".to_string()));
    }

    if let Some(parent_id) = parent_id {
        let parent_belongs_to_business: bool = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM categories WHERE id = ?1 AND business_id = ?2
             )",
            params![parent_id, business_id],
            |row| row.get(0),
        )?;
        if !parent_belongs_to_business {
            return Err(DbError::InvalidInput(
                "parent category must belong to the same business".to_string(),
            ));
        }
    }

    let description = description.map(str::trim).filter(|value| !value.is_empty());
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();

    conn.execute(
        "INSERT INTO categories
            (id, business_id, parent_id, name, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, business_id, parent_id, name, description, timestamp],
    )?;

    Ok(conn.query_row(
        "SELECT id, business_id, parent_id, name, description, is_active
         FROM categories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Category {
                id: row.get(0)?,
                business_id: row.get(1)?,
                parent_id: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
            })
        },
    )?)
}

#[derive(Debug, serde::Serialize)]
pub struct Unit {
    pub id: String,
    pub business_id: Option<String>,
    pub code: String,
    pub name: String,
    pub decimals_ok: bool,
    pub is_system: bool,
    pub is_active: bool,
}

pub fn list_units(conn: &Connection, business_id: &str) -> Result<Vec<Unit>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, code, name, decimals_ok, is_system, is_active
         FROM units
         WHERE business_id IS NULL OR business_id = ?1
         ORDER BY is_system DESC, name",
    )?;
    let units = stmt
        .query_map(params![business_id], |row| {
            Ok(Unit {
                id: row.get(0)?,
                business_id: row.get(1)?,
                code: row.get(2)?,
                name: row.get(3)?,
                decimals_ok: row.get::<_, i64>(4)? != 0,
                is_system: row.get::<_, i64>(5)? != 0,
                is_active: row.get::<_, i64>(6)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(units)
}

pub fn create_unit(
    conn: &Connection,
    business_id: &str,
    code: &str,
    name: &str,
    decimals_ok: bool,
) -> Result<Unit, DbError> {
    let code = code.trim().to_ascii_lowercase();
    let name = name.trim();
    if code.is_empty() || name.is_empty() {
        return Err(DbError::InvalidInput(
            "unit code and name must not be empty".to_string(),
        ));
    }

    let business_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM businesses WHERE id = ?1)",
        params![business_id],
        |row| row.get(0),
    )?;
    if !business_exists {
        return Err(DbError::InvalidInput("business does not exist".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    conn.execute(
        "INSERT INTO units
            (id, business_id, code, name, decimals_ok, is_system, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
        params![
            id,
            business_id,
            code,
            name,
            i64::from(decimals_ok),
            timestamp
        ],
    )?;

    Ok(conn.query_row(
        "SELECT id, business_id, code, name, decimals_ok, is_system, is_active
         FROM units WHERE id = ?1",
        params![id],
        |row| {
            Ok(Unit {
                id: row.get(0)?,
                business_id: row.get(1)?,
                code: row.get(2)?,
                name: row.get(3)?,
                decimals_ok: row.get::<_, i64>(4)? != 0,
                is_system: row.get::<_, i64>(5)? != 0,
                is_active: row.get::<_, i64>(6)? != 0,
            })
        },
    )?)
}

#[derive(Debug, serde::Serialize)]
pub struct Product {
    pub id: String,
    pub business_id: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub brand_id: Option<String>,
    pub brand_name: Option<String>,
    pub name: String,
    pub sku: String,
    pub barcode: Option<String>,
    pub description: Option<String>,
    pub size_dimensions: Option<String>,
    pub colour: Option<String>,
    pub base_unit_id: String,
    pub base_unit_name: String,
    pub base_unit_code: String,
    pub purchase_price: i64,
    pub selling_price: i64,
    pub min_stock_quantity: i64,
    pub vat_class: String,
    pub is_active: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct ProductPage {
    pub items: Vec<Product>,
    pub total_count: i64,
}

pub fn list_products(
    conn: &Connection,
    business_id: &str,
    search: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<ProductPage, DbError> {
    if offset < 0 || !(1..=100).contains(&limit) {
        return Err(DbError::InvalidInput(
            "product page requires a non-negative offset and a limit between 1 and 100".to_string(),
        ));
    }

    let search = search.map(str::trim).filter(|value| !value.is_empty());
    let pattern = search.map(|value| {
        format!(
            "%{}%",
            value
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        )
    });
    let total_count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM products p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN brands b ON b.id = p.brand_id
         WHERE p.business_id = ?1
           AND (?2 IS NULL OR
                p.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                p.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.barcode, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.description, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.size_dimensions, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.colour, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(b.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(c.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE)",
        params![business_id, pattern],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT p.id, p.business_id, p.category_id, c.name, p.brand_id, b.name,
                p.name, p.sku, p.barcode, p.description, p.size_dimensions, p.colour,
                p.base_unit_id, u.name, u.code, p.purchase_price, p.selling_price,
                p.min_stock_quantity, p.vat_class, p.is_active
         FROM products p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN brands b ON b.id = p.brand_id
         JOIN units u ON u.id = p.base_unit_id
         WHERE p.business_id = ?1
           AND (?2 IS NULL OR
                p.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                p.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.barcode, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.description, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.size_dimensions, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.colour, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(b.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(c.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE)
         ORDER BY p.name COLLATE NOCASE, p.id
         LIMIT ?3 OFFSET ?4",
    )?;
    let items = stmt
        .query_map(params![business_id, pattern, limit, offset], |row| {
            Ok(Product {
                id: row.get(0)?,
                business_id: row.get(1)?,
                category_id: row.get(2)?,
                category_name: row.get(3)?,
                brand_id: row.get(4)?,
                brand_name: row.get(5)?,
                name: row.get(6)?,
                sku: row.get(7)?,
                barcode: row.get(8)?,
                description: row.get(9)?,
                size_dimensions: row.get(10)?,
                colour: row.get(11)?,
                base_unit_id: row.get(12)?,
                base_unit_name: row.get(13)?,
                base_unit_code: row.get(14)?,
                purchase_price: row.get(15)?,
                selling_price: row.get(16)?,
                min_stock_quantity: row.get(17)?,
                vat_class: row.get(18)?,
                is_active: row.get::<_, i64>(19)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProductPage { items, total_count })
}

#[allow(clippy::too_many_arguments)]
pub fn create_product(
    conn: &Connection,
    business_id: &str,
    category_id: Option<&str>,
    brand_id: Option<&str>,
    name: &str,
    sku: &str,
    barcode: Option<&str>,
    description: Option<&str>,
    size_dimensions: Option<&str>,
    colour: Option<&str>,
    base_unit_id: &str,
    purchase_price: i64,
    selling_price: i64,
    min_stock_quantity: i64,
    vat_class: &str,
) -> Result<Product, DbError> {
    let name = name.trim();
    let sku = sku.trim().to_ascii_uppercase();
    if name.is_empty() || sku.is_empty() {
        return Err(DbError::InvalidInput(
            "product name and SKU must not be empty".to_string(),
        ));
    }
    if purchase_price < 0 || selling_price < 0 || min_stock_quantity < 0 {
        return Err(DbError::InvalidInput(
            "prices and minimum stock quantity must not be negative".to_string(),
        ));
    }
    if !matches!(vat_class, "standard" | "zero" | "exempt") {
        return Err(DbError::InvalidInput(
            "VAT class must be standard, zero, or exempt".to_string(),
        ));
    }

    let category_id = category_id.map(str::trim).filter(|value| !value.is_empty());
    let brand_id = brand_id.map(str::trim).filter(|value| !value.is_empty());
    let barcode = barcode.map(str::trim).filter(|value| !value.is_empty());
    let description = description.map(str::trim).filter(|value| !value.is_empty());
    let size_dimensions = size_dimensions
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let colour = colour.map(str::trim).filter(|value| !value.is_empty());

    let business_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM businesses WHERE id = ?1)",
        params![business_id],
        |row| row.get(0),
    )?;
    if !business_exists {
        return Err(DbError::InvalidInput("business does not exist".to_string()));
    }

    let unit_is_available: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM units
            WHERE id = ?1 AND (business_id IS NULL OR business_id = ?2) AND is_active = 1
         )",
        params![base_unit_id, business_id],
        |row| row.get(0),
    )?;
    if !unit_is_available {
        return Err(DbError::InvalidInput(
            "base unit must be active and available to this business".to_string(),
        ));
    }

    if let Some(category_id) = category_id {
        let category_is_available: bool = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM categories WHERE id = ?1 AND business_id = ?2 AND is_active = 1
             )",
            params![category_id, business_id],
            |row| row.get(0),
        )?;
        if !category_is_available {
            return Err(DbError::InvalidInput(
                "category must be active and belong to this business".to_string(),
            ));
        }
    }

    if let Some(brand_id) = brand_id {
        let brand_is_available: bool = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM brands WHERE id = ?1 AND business_id = ?2 AND is_active = 1
             )",
            params![brand_id, business_id],
            |row| row.get(0),
        )?;
        if !brand_is_available {
            return Err(DbError::InvalidInput(
                "brand must be active and belong to this business".to_string(),
            ));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO products
            (id, business_id, category_id, brand_id, name, sku, barcode, description,
             size_dimensions, colour, base_unit_id, purchase_price, selling_price,
             min_stock_quantity, vat_class, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
        params![
            id,
            business_id,
            category_id,
            brand_id,
            name,
            sku,
            barcode,
            description,
            size_dimensions,
            colour,
            base_unit_id,
            purchase_price,
            selling_price,
            min_stock_quantity,
            vat_class,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO product_units
            (id, product_id, unit_id, conversion_factor, is_base, selling_price, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1000, 1, ?4, ?5, ?5)",
        params![uuid::Uuid::new_v4().to_string(), id, base_unit_id, selling_price, timestamp],
    )?;
    tx.execute(
        "INSERT INTO product_prices (id, product_id, price_type, unit_id, price, effective_from, created_at)
         VALUES (?1, ?2, 'purchase', ?3, ?4, ?5, ?5),
                (?6, ?2, 'selling', ?3, ?7, ?5, ?5)",
        params![
            uuid::Uuid::new_v4().to_string(),
            id,
            base_unit_id,
            purchase_price,
            timestamp,
            uuid::Uuid::new_v4().to_string(),
            selling_price
        ],
    )?;
    tx.commit()?;

    conn.query_row(
        "SELECT p.id, p.business_id, p.category_id, c.name, p.brand_id, b.name,
                p.name, p.sku, p.barcode, p.description, p.size_dimensions, p.colour,
                p.base_unit_id, u.name, u.code, p.purchase_price, p.selling_price,
                p.min_stock_quantity, p.vat_class, p.is_active
         FROM products p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN brands b ON b.id = p.brand_id
         JOIN units u ON u.id = p.base_unit_id
         WHERE p.id = ?1",
        params![id],
        |row| {
            Ok(Product {
                id: row.get(0)?,
                business_id: row.get(1)?,
                category_id: row.get(2)?,
                category_name: row.get(3)?,
                brand_id: row.get(4)?,
                brand_name: row.get(5)?,
                name: row.get(6)?,
                sku: row.get(7)?,
                barcode: row.get(8)?,
                description: row.get(9)?,
                size_dimensions: row.get(10)?,
                colour: row.get(11)?,
                base_unit_id: row.get(12)?,
                base_unit_name: row.get(13)?,
                base_unit_code: row.get(14)?,
                purchase_price: row.get(15)?,
                selling_price: row.get(16)?,
                min_stock_quantity: row.get(17)?,
                vat_class: row.get(18)?,
                is_active: row.get::<_, i64>(19)? != 0,
            })
        },
    )
    .map_err(DbError::from)
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
        conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
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
        assert!(
            info.table_count >= 15,
            "expected core tables, got {}",
            info.table_count
        );
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
            .query_row("SELECT COUNT(*) FROM units WHERE is_system = 1", [], |r| {
                r.get(0)
            })
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

    #[test]
    fn business_creation_trims_fields_and_uses_configured_defaults() {
        let conn = open_in_memory().unwrap();

        let business = create_business(&conn, "  Kigali Hardware  ", Some("  TAX-123  ")).unwrap();

        assert_eq!(business.name, "Kigali Hardware");
        assert_eq!(business.tax_id.as_deref(), Some("TAX-123"));
        assert_eq!(business.currency, "RWF");
        assert_eq!(business.timezone, "Africa/Kigali");
        assert_eq!(business.date_format, "DD/MM/YYYY");
        assert_eq!(business.language, "en");
        assert_eq!(list_businesses(&conn).unwrap().len(), 1);
    }

    #[test]
    fn business_creation_rejects_an_empty_name() {
        let conn = open_in_memory().unwrap();

        let result = create_business(&conn, "  ", None);

        assert!(matches!(result, Err(DbError::InvalidInput(_))));
        assert!(list_businesses(&conn).unwrap().is_empty());
    }

    #[test]
    fn categories_are_scoped_to_business_and_parent_categories() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let parent = create_category(&conn, &business.id, None, "  Building  ", None).unwrap();
        let child = create_category(
            &conn,
            &business.id,
            Some(&parent.id),
            "Cement",
            Some("  Bagged cement  "),
        )
        .unwrap();
        create_category(&conn, &other_business.id, None, "Building", None).unwrap();

        assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(child.description.as_deref(), Some("Bagged cement"));
        assert_eq!(list_categories(&conn, &business.id).unwrap().len(), 2);
        assert_eq!(list_categories(&conn, &other_business.id).unwrap().len(), 1);
    }

    #[test]
    fn category_creation_rejects_empty_names_and_cross_business_parents() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let parent = create_category(&conn, &other_business.id, None, "Building", None).unwrap();

        assert!(matches!(
            create_category(&conn, &business.id, None, "  ", None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_category(&conn, &business.id, Some(&parent.id), "Cement", None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(list_categories(&conn, &business.id).unwrap().is_empty());
    }

    #[test]
    fn unit_list_includes_system_and_current_business_units_only() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        create_unit(&conn, &business.id, "box", "Box", false).unwrap();
        create_unit(&conn, &other_business.id, "case", "Case", false).unwrap();

        let units = list_units(&conn, &business.id).unwrap();

        assert_eq!(units.len(), 15);
        assert!(
            units
                .iter()
                .any(|unit| unit.code == "bag" && unit.is_system)
        );
        assert!(
            units
                .iter()
                .any(|unit| unit.code == "box" && !unit.is_system)
        );
        assert!(!units.iter().any(|unit| unit.code == "case"));
    }

    #[test]
    fn custom_unit_normalizes_code_and_persists_decimal_setting() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();

        let unit = create_unit(&conn, &business.id, "  BOX  ", "  Box  ", true).unwrap();

        assert_eq!(unit.code, "box");
        assert_eq!(unit.name, "Box");
        assert!(unit.decimals_ok);
        assert!(!unit.is_system);
        assert_eq!(unit.business_id.as_deref(), Some(business.id.as_str()));
    }

    #[test]
    fn custom_unit_rejects_empty_fields_and_duplicate_code() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        create_unit(&conn, &business.id, "box", "Box", false).unwrap();

        assert!(matches!(
            create_unit(&conn, &business.id, " ", "Box", false),
            Err(DbError::InvalidInput(_))
        ));
        assert!(create_unit(&conn, &business.id, "BOX", "Another box", false).is_err());
        assert_eq!(list_units(&conn, &business.id).unwrap().len(), 15);
    }

    #[test]
    fn product_creation_records_base_unit_and_initial_prices() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let category = create_category(&conn, &business.id, None, "Building", None).unwrap();

        let product = create_product(
            &conn,
            &business.id,
            Some(&category.id),
            None,
            "  Cement 50kg  ",
            " cem-50 ",
            Some(" 123456 "),
            Some("  Portland cement  "),
            Some("50 kg"),
            None,
            "sys-unit-bag",
            125_000,
            150_000,
            5_000,
            "standard",
        )
        .unwrap();

        assert_eq!(product.name, "Cement 50kg");
        assert_eq!(product.sku, "CEM-50");
        assert_eq!(product.barcode.as_deref(), Some("123456"));
        assert_eq!(product.category_name.as_deref(), Some("Building"));
        assert_eq!(product.base_unit_code, "bag");
        assert_eq!(product.purchase_price, 125_000);
        assert_eq!(product.selling_price, 150_000);
        assert_eq!(product.min_stock_quantity, 5_000);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM product_units WHERE product_id = ?1 AND is_base = 1",
                params![product.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM product_prices WHERE product_id = ?1",
                params![product.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn product_creation_rejects_invalid_fields_and_foreign_categories_atomically() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let category = create_category(&conn, &other_business.id, None, "Building", None).unwrap();

        let result = create_product(
            &conn,
            &business.id,
            Some(&category.id),
            None,
            "Cement",
            "CEM-1",
            None,
            None,
            None,
            None,
            "sys-unit-bag",
            10_000,
            12_000,
            0,
            "standard",
        );
        assert!(matches!(result, Err(DbError::InvalidInput(_))));

        let result = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Cement",
            "CEM-1",
            None,
            None,
            None,
            None,
            "sys-unit-bag",
            -1,
            12_000,
            0,
            "standard",
        );
        assert!(matches!(result, Err(DbError::InvalidInput(_))));
        assert_eq!(
            list_products(&conn, &business.id, None, 0, 20)
                .unwrap()
                .total_count,
            0
        );
    }

    #[test]
    fn product_search_is_business_scoped_and_paginates_results() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        for (shop, sku, name) in [
            (&business.id, "CEM-1", "Cement"),
            (&business.id, "CEM-2", "Cement"),
            (&other_business.id, "CEM-3", "Cement"),
        ] {
            create_product(
                &conn,
                shop,
                None,
                None,
                name,
                sku,
                None,
                None,
                None,
                None,
                "sys-unit-bag",
                10_000,
                12_000,
                0,
                "standard",
            )
            .unwrap();
        }

        let first_page = list_products(&conn, &business.id, Some("cem"), 0, 1).unwrap();
        let second_page = list_products(&conn, &business.id, Some("CEM"), 1, 1).unwrap();
        let no_match = list_products(&conn, &business.id, Some("absent"), 0, 10).unwrap();

        assert_eq!(first_page.total_count, 2);
        assert_eq!(first_page.items.len(), 1);
        assert_eq!(second_page.total_count, 2);
        assert_eq!(second_page.items.len(), 1);
        assert_ne!(first_page.items[0].id, second_page.items[0].id);
        assert_eq!(no_match.total_count, 0);
    }

    #[test]
    fn product_list_rejects_invalid_pagination() {
        let conn = open_in_memory().unwrap();

        assert!(matches!(
            list_products(&conn, "biz", None, -1, 10),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            list_products(&conn, "biz", None, 0, 101),
            Err(DbError::InvalidInput(_))
        ));
    }
}
