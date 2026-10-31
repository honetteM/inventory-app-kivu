use rusqlite::{Connection, OptionalExtension, params};
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
    (
        3,
        "003_inventory",
        include_str!("../migrations/003_inventory.sql"),
    ),
    (
        4,
        "004_inventory_costing",
        include_str!("../migrations/004_inventory_costing.sql"),
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
pub struct Brand {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub is_active: bool,
}

pub fn list_brands(conn: &Connection, business_id: &str) -> Result<Vec<Brand>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, name, is_active
         FROM brands WHERE business_id = ?1 ORDER BY name COLLATE NOCASE",
    )?;
    let brands = stmt
        .query_map(params![business_id], |row| {
            Ok(Brand {
                id: row.get(0)?,
                business_id: row.get(1)?,
                name: row.get(2)?,
                is_active: row.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(brands)
}

pub fn create_brand(conn: &Connection, business_id: &str, name: &str) -> Result<Brand, DbError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::InvalidInput(
            "brand name must not be empty".to_string(),
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
        "INSERT INTO brands (id, business_id, name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, business_id, name, timestamp],
    )?;

    Ok(conn.query_row(
        "SELECT id, business_id, name, is_active FROM brands WHERE id = ?1",
        params![id],
        |row| {
            Ok(Brand {
                id: row.get(0)?,
                business_id: row.get(1)?,
                name: row.get(2)?,
                is_active: row.get::<_, i64>(3)? != 0,
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

#[derive(Debug, serde::Serialize)]
pub struct PosProduct {
    pub id: String,
    pub name: String,
    pub sku: String,
    pub barcode: Option<String>,
    pub base_unit_name: String,
    pub selling_price: i64,
    pub quantity_on_hand: i64,
    pub base_unit_allows_decimals: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct PosProductPage {
    pub items: Vec<PosProduct>,
    pub total_count: i64,
}

pub fn search_pos_products(
    conn: &Connection,
    business_id: &str,
    search: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<PosProductPage, DbError> {
    if offset < 0 || !(1..=100).contains(&limit) {
        return Err(DbError::InvalidInput(
            "POS search requires a non-negative offset and a limit between 1 and 100".to_string(),
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
         WHERE p.business_id = ?1 AND p.is_active = 1
           AND (?2 IS NULL OR p.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                OR p.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                OR COALESCE(p.barcode, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE)",
        params![business_id, pattern],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.sku, p.barcode, u.name, p.selling_price,
                COALESCE(ib.quantity_on_hand, 0), u.decimals_ok
         FROM products p
         JOIN units u ON u.id = p.base_unit_id
         LEFT JOIN inventory_balances ib
           ON ib.product_id = p.id AND ib.business_id = p.business_id
         WHERE p.business_id = ?1 AND p.is_active = 1
           AND (?2 IS NULL OR p.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                OR p.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                OR COALESCE(p.barcode, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE)
         ORDER BY p.name COLLATE NOCASE, p.id
         LIMIT ?3 OFFSET ?4",
    )?;
    let items = stmt
        .query_map(params![business_id, pattern, limit, offset], |row| {
            Ok(PosProduct {
                id: row.get(0)?,
                name: row.get(1)?,
                sku: row.get(2)?,
                barcode: row.get(3)?,
                base_unit_name: row.get(4)?,
                selling_price: row.get(5)?,
                quantity_on_hand: row.get(6)?,
                base_unit_allows_decimals: row.get::<_, i64>(7)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PosProductPage { items, total_count })
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
pub struct ProductUnit {
    pub id: String,
    pub product_id: String,
    pub unit_id: String,
    pub unit_name: String,
    pub unit_code: String,
    pub conversion_factor: i64,
    pub is_base: bool,
    pub selling_price: Option<i64>,
}

pub fn list_product_units(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
) -> Result<Vec<ProductUnit>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT pu.id, pu.product_id, pu.unit_id, u.name, u.code,
                pu.conversion_factor, pu.is_base, pu.selling_price
         FROM product_units pu
         JOIN products p ON p.id = pu.product_id
         JOIN units u ON u.id = pu.unit_id
         WHERE p.id = ?1 AND p.business_id = ?2
         ORDER BY pu.is_base DESC, pu.sort_order, u.name COLLATE NOCASE",
    )?;
    let units = stmt
        .query_map(params![product_id, business_id], |row| {
            Ok(ProductUnit {
                id: row.get(0)?,
                product_id: row.get(1)?,
                unit_id: row.get(2)?,
                unit_name: row.get(3)?,
                unit_code: row.get(4)?,
                conversion_factor: row.get(5)?,
                is_base: row.get::<_, i64>(6)? != 0,
                selling_price: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(units)
}

pub fn create_product_unit(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    unit_id: &str,
    conversion_factor: i64,
    selling_price: Option<i64>,
) -> Result<ProductUnit, DbError> {
    if conversion_factor <= 0 {
        return Err(DbError::InvalidInput(
            "conversion factor must be greater than zero".to_string(),
        ));
    }
    if selling_price.is_some_and(|price| price < 0) {
        return Err(DbError::InvalidInput(
            "selling price must not be negative".to_string(),
        ));
    }

    let product: Option<(String, String, i64)> = conn
        .query_row(
            "SELECT p.id, p.base_unit_id, base_unit.decimals_ok
             FROM products p
             JOIN units base_unit ON base_unit.id = p.base_unit_id
             WHERE p.id = ?1 AND p.business_id = ?2",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (_, base_unit_id, base_unit_decimals_ok) = product
        .ok_or_else(|| DbError::InvalidInput("product does not belong to this business".into()))?;

    if unit_id == base_unit_id {
        return Err(DbError::InvalidInput(
            "the base unit is already configured for this product".to_string(),
        ));
    }

    let unit_is_available: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM units
            WHERE id = ?1 AND (business_id IS NULL OR business_id = ?2) AND is_active = 1
         )",
        params![unit_id, business_id],
        |row| row.get(0),
    )?;
    if !unit_is_available {
        return Err(DbError::InvalidInput(
            "selling unit must be active and available to this business".to_string(),
        ));
    }

    if base_unit_decimals_ok == 0 && conversion_factor % 1000 != 0 {
        return Err(DbError::InvalidInput(
            "fractional base-unit conversions require a base unit that allows decimals".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO product_units
            (id, product_id, unit_id, conversion_factor, is_base, selling_price, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)",
        params![
            id,
            product_id,
            unit_id,
            conversion_factor,
            selling_price,
            timestamp
        ],
    )?;
    if let Some(price) = selling_price {
        tx.execute(
            "INSERT INTO product_prices (id, product_id, price_type, unit_id, price, effective_from, created_at)
             VALUES (?1, ?2, 'selling', ?3, ?4, ?5, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                product_id,
                unit_id,
                price,
                timestamp
            ],
        )?;
    }
    tx.commit()?;

    conn.query_row(
        "SELECT pu.id, pu.product_id, pu.unit_id, u.name, u.code,
                pu.conversion_factor, pu.is_base, pu.selling_price
         FROM product_units pu
         JOIN units u ON u.id = pu.unit_id
         WHERE pu.id = ?1",
        params![id],
        |row| {
            Ok(ProductUnit {
                id: row.get(0)?,
                product_id: row.get(1)?,
                unit_id: row.get(2)?,
                unit_name: row.get(3)?,
                unit_code: row.get(4)?,
                conversion_factor: row.get(5)?,
                is_base: row.get::<_, i64>(6)? != 0,
                selling_price: row.get(7)?,
            })
        },
    )
    .map_err(DbError::from)
}

#[derive(Debug, serde::Serialize)]
pub struct InventoryItem {
    pub product_id: String,
    pub name: String,
    pub sku: String,
    pub base_unit_name: String,
    pub quantity_on_hand: i64,
    pub average_unit_cost: i64,
    pub stock_value: i64,
    pub min_stock_quantity: i64,
    pub base_unit_allows_decimals: bool,
    pub has_movements: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct InventoryPage {
    pub items: Vec<InventoryItem>,
    pub total_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct StockMovement {
    pub id: String,
    pub business_id: String,
    pub product_id: String,
    pub product_name: String,
    pub movement_type: String,
    pub quantity_delta: i64,
    pub unit_cost: Option<i64>,
    pub previous_quantity: i64,
    pub new_quantity: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub reason: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
}

fn stock_value(quantity: i64, average_unit_cost: i64) -> Result<i64, DbError> {
    let value = i128::from(quantity)
        .checked_mul(i128::from(average_unit_cost))
        .and_then(|amount| amount.checked_add(500))
        .ok_or_else(|| {
            DbError::InvalidInput("stock valuation exceeds the supported limit".into())
        })?
        / 1000;
    i64::try_from(value)
        .map_err(|_| DbError::InvalidInput("stock valuation exceeds the supported limit".into()))
}

pub fn list_inventory(
    conn: &Connection,
    business_id: &str,
    search: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<InventoryPage, DbError> {
    if offset < 0 || !(1..=100).contains(&limit) {
        return Err(DbError::InvalidInput(
            "inventory page requires a non-negative offset and a limit between 1 and 100"
                .to_string(),
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
         JOIN units u ON u.id = p.base_unit_id
         WHERE p.business_id = ?1
           AND p.is_active = 1
           AND (?2 IS NULL OR
                p.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                p.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.barcode, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(b.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(c.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                u.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE)",
        params![business_id, pattern],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.sku, u.name,
                COALESCE(ib.quantity_on_hand, 0), COALESCE(ib.average_unit_cost, p.purchase_price),
                p.min_stock_quantity, u.decimals_ok,
                EXISTS(SELECT 1 FROM stock_movements sm WHERE sm.product_id = p.id)
         FROM products p
         JOIN units u ON u.id = p.base_unit_id
         LEFT JOIN inventory_balances ib
           ON ib.product_id = p.id AND ib.business_id = p.business_id
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN brands b ON b.id = p.brand_id
         WHERE p.business_id = ?1
           AND p.is_active = 1
           AND (?2 IS NULL OR
                p.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                p.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(p.barcode, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(b.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                COALESCE(c.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
                u.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE)
         ORDER BY p.name COLLATE NOCASE, p.id
         LIMIT ?3 OFFSET ?4",
    )?;
    let mut items = stmt
        .query_map(params![business_id, pattern, limit, offset], |row| {
            let quantity_on_hand: i64 = row.get(4)?;
            let average_unit_cost: i64 = row.get(5)?;
            Ok(InventoryItem {
                product_id: row.get(0)?,
                name: row.get(1)?,
                sku: row.get(2)?,
                base_unit_name: row.get(3)?,
                quantity_on_hand,
                average_unit_cost,
                stock_value: 0,
                min_stock_quantity: row.get(6)?,
                base_unit_allows_decimals: row.get::<_, i64>(7)? != 0,
                has_movements: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for item in &mut items {
        item.stock_value = stock_value(item.quantity_on_hand, item.average_unit_cost)?;
    }

    Ok(InventoryPage { items, total_count })
}

pub fn list_stock_movements(
    conn: &Connection,
    business_id: &str,
    product_id: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<Vec<StockMovement>, DbError> {
    if offset < 0 || !(1..=100).contains(&limit) {
        return Err(DbError::InvalidInput(
            "movement page requires a non-negative offset and a limit between 1 and 100"
                .to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT sm.id, sm.business_id, sm.product_id, p.name, sm.movement_type,
                sm.quantity_delta, sm.unit_cost, sm.previous_quantity, sm.new_quantity,
                sm.reference_type, sm.reference_id, sm.reason, sm.user_id, sm.created_at
         FROM stock_movements sm
         JOIN products p ON p.id = sm.product_id
         WHERE sm.business_id = ?1 AND (?2 IS NULL OR sm.product_id = ?2)
         ORDER BY sm.created_at DESC, sm.id DESC
         LIMIT ?3 OFFSET ?4",
    )?;
    let movements = stmt
        .query_map(params![business_id, product_id, limit, offset], |row| {
            Ok(StockMovement {
                id: row.get(0)?,
                business_id: row.get(1)?,
                product_id: row.get(2)?,
                product_name: row.get(3)?,
                movement_type: row.get(4)?,
                quantity_delta: row.get(5)?,
                unit_cost: row.get(6)?,
                previous_quantity: row.get(7)?,
                new_quantity: row.get(8)?,
                reference_type: row.get(9)?,
                reference_id: row.get(10)?,
                reason: row.get(11)?,
                user_id: row.get(12)?,
                created_at: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(movements)
}

pub fn set_opening_stock(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    quantity: i64,
    reason: Option<&str>,
) -> Result<StockMovement, DbError> {
    if quantity < 0 {
        return Err(DbError::InvalidInput(
            "opening quantity must not be negative".to_string(),
        ));
    }

    let reason = reason.map(str::trim).filter(|value| !value.is_empty());
    let product: Option<(String, i64, i64)> = conn
        .query_row(
            "SELECT p.name, u.decimals_ok, p.purchase_price FROM products p
             JOIN units u ON u.id = p.base_unit_id
             WHERE p.id = ?1 AND p.business_id = ?2 AND p.is_active = 1",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (product_name, base_unit_allows_decimals, unit_cost) = product.ok_or_else(|| {
        DbError::InvalidInput("active product does not belong to this business".to_string())
    })?;
    if base_unit_allows_decimals == 0 && quantity % 1000 != 0 {
        return Err(DbError::InvalidInput(
            "opening quantity must be a whole base unit".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let existing_balance: Option<i64> = tx
        .query_row(
            "SELECT quantity_on_hand FROM inventory_balances
             WHERE product_id = ?1 AND business_id = ?2",
            params![product_id, business_id],
            |row| row.get(0),
        )
        .optional()?;
    let has_movement: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM stock_movements WHERE product_id = ?1)",
        params![product_id],
        |row| row.get(0),
    )?;
    if existing_balance.unwrap_or(0) != 0 || has_movement {
        return Err(DbError::InvalidInput(
            "opening stock can only be recorded once, before other stock movements".to_string(),
        ));
    }

    tx.execute(
        "INSERT INTO inventory_balances
            (product_id, business_id, quantity_on_hand, average_unit_cost, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![product_id, business_id, quantity, unit_cost, timestamp],
    )?;
    tx.execute(
        "INSERT INTO stock_movements
            (id, business_id, product_id, movement_type, quantity_delta, unit_cost,
             previous_quantity, new_quantity, reference_type, reference_id, reason, created_at)
         VALUES (?1, ?2, ?3, 'opening_stock', ?4, ?5, 0, ?4, 'product', ?3, ?6, ?7)",
        params![
            id,
            business_id,
            product_id,
            quantity,
            unit_cost,
            reason,
            timestamp
        ],
    )?;
    tx.commit()?;

    Ok(StockMovement {
        id,
        business_id: business_id.to_string(),
        product_id: product_id.to_string(),
        product_name,
        movement_type: "opening_stock".to_string(),
        quantity_delta: quantity,
        unit_cost: Some(unit_cost),
        previous_quantity: 0,
        new_quantity: quantity,
        reference_type: Some("product".to_string()),
        reference_id: Some(product_id.to_string()),
        reason: reason.map(str::to_string),
        user_id: None,
        created_at: timestamp,
    })
}

pub fn correct_stock(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    target_quantity: i64,
    reason: &str,
) -> Result<StockMovement, DbError> {
    if target_quantity < 0 {
        return Err(DbError::InvalidInput(
            "corrected quantity must not be negative".to_string(),
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(DbError::InvalidInput(
            "a reason is required for stock correction".to_string(),
        ));
    }
    if reason.chars().count() > 500 {
        return Err(DbError::InvalidInput(
            "stock correction reason must be 500 characters or fewer".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let product: Option<(String, i64, i64)> = tx
        .query_row(
            "SELECT p.name, u.decimals_ok, p.purchase_price FROM products p
             JOIN units u ON u.id = p.base_unit_id
             WHERE p.id = ?1 AND p.business_id = ?2 AND p.is_active = 1",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (product_name, base_unit_allows_decimals, product_unit_cost) =
        product.ok_or_else(|| {
            DbError::InvalidInput("active product does not belong to this business".to_string())
        })?;
    if base_unit_allows_decimals == 0 && target_quantity % 1000 != 0 {
        return Err(DbError::InvalidInput(
            "corrected quantity must be a whole base unit".to_string(),
        ));
    }

    let existing_balance: Option<(i64, i64)> = tx
        .query_row(
            "SELECT quantity_on_hand, average_unit_cost FROM inventory_balances
             WHERE product_id = ?1 AND business_id = ?2",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (previous_quantity, average_unit_cost) = existing_balance.unwrap_or((0, product_unit_cost));
    if target_quantity == previous_quantity {
        return Err(DbError::InvalidInput(
            "corrected quantity must differ from the current stock".to_string(),
        ));
    }
    let quantity_delta = target_quantity - previous_quantity;

    tx.execute(
        "INSERT INTO inventory_balances
            (product_id, business_id, quantity_on_hand, average_unit_cost, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(product_id) DO UPDATE SET
             quantity_on_hand = excluded.quantity_on_hand,
             average_unit_cost = excluded.average_unit_cost,
             updated_at = excluded.updated_at",
        params![
            product_id,
            business_id,
            target_quantity,
            average_unit_cost,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO stock_movements
            (id, business_id, product_id, movement_type, quantity_delta, unit_cost,
             previous_quantity, new_quantity, reference_type, reference_id, reason, created_at)
         VALUES (?1, ?2, ?3, 'stock_correction', ?4, ?5, ?6, ?7, 'product', ?3, ?8, ?9)",
        params![
            id,
            business_id,
            product_id,
            quantity_delta,
            average_unit_cost,
            previous_quantity,
            target_quantity,
            reason,
            timestamp
        ],
    )?;
    tx.commit()?;

    Ok(StockMovement {
        id,
        business_id: business_id.to_string(),
        product_id: product_id.to_string(),
        product_name,
        movement_type: "stock_correction".to_string(),
        quantity_delta,
        unit_cost: Some(average_unit_cost),
        previous_quantity,
        new_quantity: target_quantity,
        reference_type: Some("product".to_string()),
        reference_id: Some(product_id.to_string()),
        reason: Some(reason.to_string()),
        user_id: None,
        created_at: timestamp,
    })
}

pub fn receive_stock(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    quantity: i64,
    received_unit_cost: i64,
    receipt_reference: &str,
    reason: Option<&str>,
) -> Result<StockMovement, DbError> {
    if quantity <= 0 {
        return Err(DbError::InvalidInput(
            "received quantity must be greater than zero".to_string(),
        ));
    }
    if received_unit_cost < 0 {
        return Err(DbError::InvalidInput(
            "received unit cost must not be negative".to_string(),
        ));
    }
    let receipt_reference = receipt_reference.trim();
    if receipt_reference.is_empty() || receipt_reference.chars().count() > 200 {
        return Err(DbError::InvalidInput(
            "delivery or invoice reference is required and must be 200 characters or fewer"
                .to_string(),
        ));
    }
    let reason = reason.map(str::trim).filter(|value| !value.is_empty());
    if reason.is_some_and(|value| value.chars().count() > 500) {
        return Err(DbError::InvalidInput(
            "receiving notes must be 500 characters or fewer".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let product: Option<(String, i64)> = tx
        .query_row(
            "SELECT p.name, u.decimals_ok FROM products p
             JOIN units u ON u.id = p.base_unit_id
             WHERE p.id = ?1 AND p.business_id = ?2 AND p.is_active = 1",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (product_name, base_unit_allows_decimals) = product.ok_or_else(|| {
        DbError::InvalidInput("active product does not belong to this business".to_string())
    })?;
    if base_unit_allows_decimals == 0 && quantity % 1000 != 0 {
        return Err(DbError::InvalidInput(
            "received quantity must be a whole base unit".to_string(),
        ));
    }

    let existing_balance: Option<(i64, i64)> = tx
        .query_row(
            "SELECT quantity_on_hand, average_unit_cost FROM inventory_balances
             WHERE product_id = ?1 AND business_id = ?2",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (previous_quantity, previous_average_unit_cost) =
        existing_balance.unwrap_or((0, received_unit_cost));
    let new_quantity = previous_quantity.checked_add(quantity).ok_or_else(|| {
        DbError::InvalidInput("received quantity exceeds the supported stock limit".to_string())
    })?;
    let average_unit_cost = if previous_quantity == 0 {
        received_unit_cost
    } else {
        let previous_value = i128::from(previous_quantity)
            .checked_mul(i128::from(previous_average_unit_cost))
            .ok_or_else(|| {
                DbError::InvalidInput("stock valuation exceeds the supported limit".into())
            })?;
        let received_value = i128::from(quantity)
            .checked_mul(i128::from(received_unit_cost))
            .ok_or_else(|| {
                DbError::InvalidInput("stock valuation exceeds the supported limit".into())
            })?;
        let weighted_value = previous_value.checked_add(received_value).ok_or_else(|| {
            DbError::InvalidInput("stock valuation exceeds the supported limit".into())
        })?;
        let average = weighted_value
            .checked_add(i128::from(new_quantity / 2))
            .ok_or_else(|| {
                DbError::InvalidInput("stock valuation exceeds the supported limit".into())
            })?
            / i128::from(new_quantity);
        i64::try_from(average).map_err(|_| {
            DbError::InvalidInput("average unit cost exceeds the supported limit".into())
        })?
    };

    tx.execute(
        "INSERT INTO inventory_balances
            (product_id, business_id, quantity_on_hand, average_unit_cost, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(product_id) DO UPDATE SET
             quantity_on_hand = excluded.quantity_on_hand,
             average_unit_cost = excluded.average_unit_cost,
             updated_at = excluded.updated_at",
        params![
            product_id,
            business_id,
            new_quantity,
            average_unit_cost,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO stock_movements
            (id, business_id, product_id, movement_type, quantity_delta, unit_cost,
             previous_quantity, new_quantity, reference_type, reference_id, reason, created_at)
         VALUES (?1, ?2, ?3, 'purchase_received', ?4, ?5, ?6, ?7,
                 'supplier_document', ?8, ?9, ?10)",
        params![
            id,
            business_id,
            product_id,
            quantity,
            received_unit_cost,
            previous_quantity,
            new_quantity,
            receipt_reference,
            reason,
            timestamp
        ],
    )?;
    tx.commit()?;

    Ok(StockMovement {
        id,
        business_id: business_id.to_string(),
        product_id: product_id.to_string(),
        product_name,
        movement_type: "purchase_received".to_string(),
        quantity_delta: quantity,
        unit_cost: Some(received_unit_cost),
        previous_quantity,
        new_quantity,
        reference_type: Some("supplier_document".to_string()),
        reference_id: Some(receipt_reference.to_string()),
        reason: reason.map(str::to_string),
        user_id: None,
        created_at: timestamp,
    })
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

    fn create_test_product(conn: &Connection, business_id: &str, sku: &str) -> Product {
        create_product(
            conn,
            business_id,
            None,
            None,
            "Test product",
            sku,
            None,
            None,
            None,
            None,
            "sys-unit-bag",
            10_000,
            12_000,
            5_000,
            "standard",
        )
        .unwrap()
    }

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
    fn inventory_costing_migration_backfills_existing_stock_costs() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply_pragmas(&conn).unwrap();
        applied_versions(&conn).unwrap();
        for (version, name, sql) in MIGRATIONS.iter().take(3) {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, ?3)",
                params![version, name, now_utc()],
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "COST-MIGRATION-01");
        conn.execute(
            "INSERT INTO inventory_balances (product_id, business_id, quantity_on_hand, updated_at)
             VALUES (?1, ?2, 2_000, ?3)",
            params![product.id, business.id, now_utc()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements
                (id, business_id, product_id, movement_type, quantity_delta,
                 previous_quantity, new_quantity, created_at)
             VALUES ('legacy-movement', ?1, ?2, 'opening_stock', 2_000, 0, 2_000, ?3)",
            params![business.id, product.id, now_utc()],
        )
        .unwrap();

        run_migrations(&mut conn).unwrap();

        let balance_cost: i64 = conn
            .query_row(
                "SELECT average_unit_cost FROM inventory_balances WHERE product_id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();
        let movement_cost: i64 = conn
            .query_row(
                "SELECT unit_cost FROM stock_movements WHERE id = 'legacy-movement'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(balance_cost, product.purchase_price);
        assert_eq!(movement_cost, product.purchase_price);
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
    fn pos_search_returns_selling_data_and_stock_without_purchase_costs() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "POS-01");
        let foreign = create_test_product(&conn, &other_business.id, "POS-02");
        conn.execute(
            "UPDATE products SET barcode = 'BARCODE-01' WHERE id = ?1",
            params![product.id],
        )
        .unwrap();
        set_opening_stock(&conn, &business.id, &product.id, 12_000, None).unwrap();
        set_opening_stock(&conn, &other_business.id, &foreign.id, 99_000, None).unwrap();

        let result = search_pos_products(&conn, &business.id, Some("BARCODE-01"), 0, 20).unwrap();
        let serialized = serde_json::to_value(&result.items[0]).unwrap();
        let no_match = search_pos_products(&conn, &business.id, Some("missing"), 0, 20).unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.items[0].id, product.id);
        assert_eq!(result.items[0].selling_price, product.selling_price);
        assert_eq!(result.items[0].quantity_on_hand, 12_000);
        assert_eq!(result.items[0].base_unit_name, "Bag");
        assert!(serialized.get("purchase_price").is_none());
        assert_eq!(no_match.total_count, 0);
        assert!(no_match.items.is_empty());
        assert!(matches!(
            search_pos_products(&conn, &business.id, None, -1, 20),
            Err(DbError::InvalidInput(_))
        ));
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

    #[test]
    fn brands_are_scoped_to_business_and_names_are_trimmed() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();

        let brand = create_brand(&conn, &business.id, "  BuildPro  ").unwrap();
        create_brand(&conn, &other_business.id, "BuildPro").unwrap();

        assert_eq!(brand.name, "BuildPro");
        assert_eq!(brand.business_id, business.id);
        assert_eq!(list_brands(&conn, &business.id).unwrap().len(), 1);
        assert_eq!(list_brands(&conn, &other_business.id).unwrap().len(), 1);
    }

    #[test]
    fn brand_creation_rejects_empty_names_and_duplicates() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        create_brand(&conn, &business.id, "BuildPro").unwrap();

        assert!(matches!(
            create_brand(&conn, &business.id, " "),
            Err(DbError::InvalidInput(_))
        ));
        assert!(create_brand(&conn, &business.id, "BuildPro").is_err());
        assert_eq!(list_brands(&conn, &business.id).unwrap().len(), 1);
    }

    #[test]
    fn product_units_store_conversion_and_optional_price_history() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let unit = create_unit(&conn, &business.id, "roll", "Roll", false).unwrap();
        let product = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Electrical cable",
            "CABLE-01",
            None,
            None,
            None,
            None,
            "sys-unit-m",
            1_000,
            1_500,
            0,
            "standard",
        )
        .unwrap();

        let product_unit = create_product_unit(
            &conn,
            &business.id,
            &product.id,
            &unit.id,
            100_000,
            Some(120_000),
        )
        .unwrap();
        let units = list_product_units(&conn, &business.id, &product.id).unwrap();

        assert_eq!(product_unit.conversion_factor, 100_000);
        assert_eq!(product_unit.unit_code, "roll");
        assert_eq!(product_unit.selling_price, Some(120_000));
        assert_eq!(units.len(), 2);
        assert!(units[0].is_base);
        assert!(!units[1].is_base);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM product_prices
                 WHERE product_id = ?1 AND price_type = 'selling' AND unit_id = ?2 AND price = 120000",
                params![product.id, unit.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn product_unit_validation_prevents_invalid_or_cross_business_conversions() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let bag = create_unit(&conn, &business.id, "bagx", "Bag custom", false).unwrap();
        let foreign_unit = create_unit(&conn, &other_business.id, "box", "Box", false).unwrap();
        let product = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Cement",
            "CEM-01",
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
        let decimal_base_product = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Cable",
            "CAB-01",
            None,
            None,
            None,
            None,
            "sys-unit-m",
            1_000,
            1_500,
            0,
            "standard",
        )
        .unwrap();

        assert!(matches!(
            create_product_unit(&conn, &business.id, &product.id, &bag.id, 0, None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_product_unit(&conn, &business.id, &product.id, &bag.id, 1_500, None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_product_unit(
                &conn,
                &business.id,
                &product.id,
                &foreign_unit.id,
                2_000,
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_product_unit(
                &conn,
                &other_business.id,
                &product.id,
                &foreign_unit.id,
                2_000,
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_product_unit(
                &conn,
                &business.id,
                &decimal_base_product.id,
                &bag.id,
                -1_000,
                Some(-1)
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(
            list_product_units(&conn, &business.id, &product.id)
                .unwrap()
                .len()
                == 1
        );
        assert!(
            list_product_units(&conn, &other_business.id, &product.id)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn product_units_allow_fractional_conversion_when_base_unit_supports_decimals() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let half_roll = create_unit(&conn, &business.id, "halfroll", "Half roll", false).unwrap();
        let product = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Cable",
            "CAB-02",
            None,
            None,
            None,
            None,
            "sys-unit-m",
            1_000,
            1_500,
            0,
            "standard",
        )
        .unwrap();

        let product_unit =
            create_product_unit(&conn, &business.id, &product.id, &half_roll.id, 1_500, None)
                .unwrap();

        assert_eq!(product_unit.conversion_factor, 1_500);
        assert_eq!(product_unit.selling_price, None);
    }

    #[test]
    fn opening_stock_updates_balance_and_writes_traceable_movement() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-01");

        let movement = set_opening_stock(
            &conn,
            &business.id,
            &product.id,
            125_000,
            Some("  Initial stock count  "),
        )
        .unwrap();
        let inventory = list_inventory(&conn, &business.id, Some("STOCK-01"), 0, 20).unwrap();
        let movements =
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20).unwrap();

        assert_eq!(movement.movement_type, "opening_stock");
        assert_eq!(movement.previous_quantity, 0);
        assert_eq!(movement.new_quantity, 125_000);
        assert_eq!(movement.quantity_delta, 125_000);
        assert_eq!(movement.reason.as_deref(), Some("Initial stock count"));
        assert_eq!(inventory.total_count, 1);
        assert_eq!(inventory.items[0].quantity_on_hand, 125_000);
        assert_eq!(inventory.items[0].average_unit_cost, 10_000);
        assert_eq!(inventory.items[0].stock_value, 1_250_000);
        assert_eq!(inventory.items[0].min_stock_quantity, 5_000);
        assert!(!inventory.items[0].base_unit_allows_decimals);
        assert!(inventory.items[0].has_movements);
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].id, movement.id);
        assert_eq!(
            movements[0].reference_id.as_deref(),
            Some(product.id.as_str())
        );
    }

    #[test]
    fn opening_stock_rejects_negative_repeat_and_cross_business_entries() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-02");

        assert!(matches!(
            set_opening_stock(&conn, &business.id, &product.id, -1, None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            set_opening_stock(&conn, &other_business.id, &product.id, 1000, None),
            Err(DbError::InvalidInput(_))
        ));
        set_opening_stock(&conn, &business.id, &product.id, 0, None).unwrap();
        assert!(matches!(
            set_opening_stock(&conn, &business.id, &product.id, 1000, None),
            Err(DbError::InvalidInput(_))
        ));
        assert_eq!(
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn opening_stock_obeys_base_unit_decimal_rules() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let bags = create_test_product(&conn, &business.id, "STOCK-DECIMAL-BAG");
        let cable = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Cable",
            "STOCK-DECIMAL-CABLE",
            None,
            None,
            None,
            None,
            "sys-unit-m",
            1_000,
            1_500,
            0,
            "standard",
        )
        .unwrap();

        assert!(matches!(
            set_opening_stock(&conn, &business.id, &bags.id, 1_500, None),
            Err(DbError::InvalidInput(_))
        ));
        let accepted = set_opening_stock(&conn, &business.id, &cable.id, 1_500, None).unwrap();
        assert_eq!(accepted.new_quantity, 1_500);
    }

    #[test]
    fn stock_correction_updates_balance_and_records_signed_traceable_movements() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-CORRECTION-01");

        let initial = correct_stock(
            &conn,
            &business.id,
            &product.id,
            8_000,
            "  First physical count  ",
        )
        .unwrap();
        let correction =
            correct_stock(&conn, &business.id, &product.id, 6_000, "Counted again").unwrap();
        let inventory =
            list_inventory(&conn, &business.id, Some("STOCK-CORRECTION-01"), 0, 20).unwrap();
        let movements =
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20).unwrap();

        assert_eq!(initial.movement_type, "stock_correction");
        assert_eq!(initial.quantity_delta, 8_000);
        assert_eq!(initial.previous_quantity, 0);
        assert_eq!(initial.new_quantity, 8_000);
        assert_eq!(initial.reason.as_deref(), Some("First physical count"));
        assert_eq!(correction.quantity_delta, -2_000);
        assert_eq!(correction.previous_quantity, 8_000);
        assert_eq!(correction.new_quantity, 6_000);
        assert_eq!(inventory.items[0].quantity_on_hand, 6_000);
        assert_eq!(inventory.items[0].average_unit_cost, 10_000);
        assert_eq!(inventory.items[0].stock_value, 60_000);
        assert_eq!(movements.len(), 2);
        assert!(
            movements
                .iter()
                .any(|movement| movement.id == correction.id)
        );
        assert!(movements.iter().any(|movement| movement.id == initial.id));
    }

    #[test]
    fn stock_correction_rejects_invalid_quantities_reasons_and_businesses() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-CORRECTION-02");

        for (requested_business, target, reason) in [
            (business.id.as_str(), -1, "Negative count"),
            (business.id.as_str(), 500, "Fractional bag"),
            (business.id.as_str(), 0, "No change"),
            (business.id.as_str(), 1_000, "   "),
            (business.id.as_str(), 1_000, &"x".repeat(501)),
            (other_business.id.as_str(), 1_000, "Wrong business"),
        ] {
            assert!(matches!(
                correct_stock(&conn, requested_business, &product.id, target, reason),
                Err(DbError::InvalidInput(_))
            ));
        }

        assert!(
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            list_inventory(&conn, &business.id, Some("STOCK-CORRECTION-02"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            0
        );
    }

    #[test]
    fn stock_receipt_increases_balance_and_records_supplier_document_reference() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-RECEIPT-01");
        let opening = set_opening_stock(&conn, &business.id, &product.id, 5_000, None).unwrap();
        let receipt = receive_stock(
            &conn,
            &business.id,
            &product.id,
            3_000,
            20_000,
            "  INV-2026-0042  ",
            Some("  Partial delivery  "),
        )
        .unwrap();
        let inventory =
            list_inventory(&conn, &business.id, Some("STOCK-RECEIPT-01"), 0, 20).unwrap();
        let movements =
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20).unwrap();

        assert_eq!(receipt.movement_type, "purchase_received");
        assert_eq!(receipt.quantity_delta, 3_000);
        assert_eq!(receipt.unit_cost, Some(20_000));
        assert_eq!(receipt.previous_quantity, 5_000);
        assert_eq!(receipt.new_quantity, 8_000);
        assert_eq!(receipt.reference_type.as_deref(), Some("supplier_document"));
        assert_eq!(receipt.reference_id.as_deref(), Some("INV-2026-0042"));
        assert_eq!(receipt.reason.as_deref(), Some("Partial delivery"));
        assert_eq!(inventory.items[0].quantity_on_hand, 8_000);
        assert_eq!(inventory.items[0].average_unit_cost, 13_750);
        assert_eq!(inventory.items[0].stock_value, 110_000);
        assert_eq!(movements.len(), 2);
        assert!(movements.iter().any(|movement| movement.id == opening.id));
        assert!(movements.iter().any(|movement| movement.id == receipt.id));
    }

    #[test]
    fn stock_receipt_can_initialize_balance_and_obeys_decimal_unit_rules() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let bags = create_test_product(&conn, &business.id, "STOCK-RECEIPT-BAG");
        let cable = create_product(
            &conn,
            &business.id,
            None,
            None,
            "Cable",
            "STOCK-RECEIPT-CABLE",
            None,
            None,
            None,
            None,
            "sys-unit-m",
            1_000,
            1_500,
            0,
            "standard",
        )
        .unwrap();

        let initial_receipt =
            receive_stock(&conn, &business.id, &bags.id, 2_000, 12_000, "DEL-01", None).unwrap();
        assert_eq!(initial_receipt.previous_quantity, 0);
        assert_eq!(initial_receipt.new_quantity, 2_000);
        assert_eq!(initial_receipt.unit_cost, Some(12_000));
        assert!(matches!(
            set_opening_stock(&conn, &business.id, &bags.id, 1_000, None),
            Err(DbError::InvalidInput(_))
        ));
        let decimal_receipt = receive_stock(
            &conn,
            &business.id,
            &cable.id,
            1_500,
            15_000,
            "DEL-02",
            None,
        )
        .unwrap();
        assert_eq!(decimal_receipt.new_quantity, 1_500);
        assert!(matches!(
            receive_stock(&conn, &business.id, &bags.id, 1_500, 12_000, "DEL-03", None),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn stock_receipt_rejects_invalid_input_and_prevents_quantity_overflow() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-RECEIPT-02");

        assert!(matches!(
            receive_stock(&conn, &business.id, &product.id, 0, 10_000, "DEL-01", None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            receive_stock(
                &conn,
                &business.id,
                &product.id,
                -1_000,
                10_000,
                "DEL-01",
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            receive_stock(&conn, &business.id, &product.id, 1_000, -1, "DEL-01", None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            receive_stock(&conn, &business.id, &product.id, 1_000, 10_000, "   ", None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            receive_stock(
                &conn,
                &business.id,
                &product.id,
                1_000,
                10_000,
                &"x".repeat(201),
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            receive_stock(
                &conn,
                &business.id,
                &product.id,
                1_000,
                10_000,
                "DEL-01",
                Some(&"x".repeat(501))
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            receive_stock(
                &conn,
                &other_business.id,
                &product.id,
                1_000,
                10_000,
                "DEL-01",
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20)
                .unwrap()
                .is_empty()
        );

        conn.execute(
            "INSERT INTO inventory_balances (product_id, business_id, quantity_on_hand, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![product.id, business.id, i64::MAX, now_utc()],
        )
        .unwrap();
        assert!(matches!(
            receive_stock(
                &conn,
                &business.id,
                &product.id,
                1_000,
                10_000,
                "DEL-02",
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        let balance: i64 = conn
            .query_row(
                "SELECT quantity_on_hand FROM inventory_balances WHERE product_id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(balance, i64::MAX);
        assert!(
            list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn inventory_search_and_movement_history_are_business_scoped_and_paginated() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let first = create_test_product(&conn, &business.id, "STOCK-03");
        let second = create_test_product(&conn, &business.id, "STOCK-04");
        let foreign = create_test_product(&conn, &other_business.id, "STOCK-05");
        set_opening_stock(&conn, &business.id, &first.id, 10_000, None).unwrap();
        set_opening_stock(&conn, &business.id, &second.id, 20_000, None).unwrap();
        set_opening_stock(&conn, &other_business.id, &foreign.id, 30_000, None).unwrap();

        let inventory = list_inventory(&conn, &business.id, Some("STOCK"), 1, 1).unwrap();
        let movements = list_stock_movements(&conn, &business.id, None, 0, 1).unwrap();
        let no_cross_business_movements =
            list_stock_movements(&conn, &business.id, Some(&foreign.id), 0, 10).unwrap();

        assert_eq!(inventory.total_count, 2);
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(movements.len(), 1);
        assert_ne!(movements[0].product_id, foreign.id);
        assert!(no_cross_business_movements.is_empty());
    }

    #[test]
    fn inventory_migration_enforces_tenant_and_movement_quantity_integrity() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-06");

        let mismatched_balance = conn.execute(
            "INSERT INTO inventory_balances (product_id, business_id, quantity_on_hand, updated_at)
             VALUES (?1, ?2, 1000, ?3)",
            params![product.id, other_business.id, now_utc()],
        );
        assert!(mismatched_balance.is_err());

        let invalid_movement = conn.execute(
            "INSERT INTO stock_movements
                (id, business_id, product_id, movement_type, quantity_delta,
                 previous_quantity, new_quantity, created_at)
             VALUES ('movement-1', ?1, ?2, 'opening_stock', 1000, 0, 999, ?3)",
            params![business.id, product.id, now_utc()],
        );
        assert!(invalid_movement.is_err());
    }
}
