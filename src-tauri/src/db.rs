use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashSet, VecDeque};
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
    (
        5,
        "005_receipt_cost_totals",
        include_str!("../migrations/005_receipt_cost_totals.sql"),
    ),
    (6, "006_sales", include_str!("../migrations/006_sales.sql")),
    (
        7,
        "007_suppliers",
        include_str!("../migrations/007_suppliers.sql"),
    ),
    (
        8,
        "008_sale_price_overrides",
        include_str!("../migrations/008_sale_price_overrides.sql"),
    ),
    (
        9,
        "009_purchases",
        include_str!("../migrations/009_purchases.sql"),
    ),
    (
        10,
        "010_supplier_payments",
        include_str!("../migrations/010_supplier_payments.sql"),
    ),
    (
        11,
        "011_inventory_cost_layers",
        include_str!("../migrations/011_inventory_cost_layers.sql"),
    ),
    (
        12,
        "012_customers",
        include_str!("../migrations/012_customers.sql"),
    ),
    (
        13,
        "013_customer_credit",
        include_str!("../migrations/013_customer_credit.sql"),
    ),
    (
        14,
        "014_expenses",
        include_str!("../migrations/014_expenses.sql"),
    ),
    (
        15,
        "015_cash_ledger",
        include_str!("../migrations/015_cash_ledger.sql"),
    ),
    (
        16,
        "016_role_permissions",
        include_str!("../migrations/016_role_permissions.sql"),
    ),
    (
        17,
        "017_manager_permissions",
        include_str!("../migrations/017_manager_permissions.sql"),
    ),
    (
        18,
        "018_manager_operational_access",
        include_str!("../migrations/018_manager_operational_access.sql"),
    ),
    (
        19,
        "019_expense_and_balance_permissions",
        include_str!("../migrations/019_expense_and_balance_permissions.sql"),
    ),
    (
        20,
        "020_opening_balances_and_cash_posting",
        include_str!("../migrations/020_opening_balances_and_cash_posting.sql"),
    ),
    (
        21,
        "021_account_backed_expenses",
        include_str!("../migrations/021_account_backed_expenses.sql"),
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

pub const PERMISSION_SYSTEM_READ: &str = "system.read";
pub const PERMISSION_BUSINESS_READ: &str = "business.read";
pub const PERMISSION_BUSINESS_MANAGE: &str = "business.manage";
pub const PERMISSION_EMPLOYEES_MANAGE: &str = "employees.manage";
pub const PERMISSION_CASH_ACCOUNTS_MANAGE: &str = "cash_accounts.manage";
pub const PERMISSION_CATALOG_READ: &str = "catalog.read";
pub const PERMISSION_CATALOG_MANAGE: &str = "catalog.manage";
pub const PERMISSION_INVENTORY_READ: &str = "inventory.read";
pub const PERMISSION_INVENTORY_MANAGE: &str = "inventory.manage";
pub const PERMISSION_SUPPLIERS_READ: &str = "suppliers.read";
pub const PERMISSION_SUPPLIERS_MANAGE: &str = "suppliers.manage";
pub const PERMISSION_CUSTOMERS_READ: &str = "customers.read";
pub const PERMISSION_CUSTOMERS_MANAGE: &str = "customers.manage";
pub const PERMISSION_CUSTOMER_DEBTS_READ: &str = "customer_debts.read";
pub const PERMISSION_CUSTOMER_PAYMENTS_MANAGE: &str = "customer_payments.manage";
pub const PERMISSION_POS_USE: &str = "pos.use";
pub const PERMISSION_SALES_READ: &str = "sales.read";
pub const PERMISSION_SALES_MANAGE: &str = "sales.manage";
pub const PERMISSION_PURCHASES_READ: &str = "purchases.read";
pub const PERMISSION_PURCHASES_MANAGE: &str = "purchases.manage";
pub const PERMISSION_EXPENSES_READ: &str = "expenses.read";
pub const PERMISSION_EXPENSES_MANAGE: &str = "expenses.manage";
pub const PERMISSION_CASH_ACCOUNT_BALANCE_READ: &str = "cash_accounts.balance.read";
pub const PERMISSION_REPORTS_READ: &str = "reports.read";
pub const PERMISSION_ROLES_MANAGE: &str = "roles.manage";

const MANAGER_PERMISSION_IDS: &[&str] = &[
    PERMISSION_SYSTEM_READ,
    PERMISSION_BUSINESS_READ,
    PERMISSION_CASH_ACCOUNT_BALANCE_READ,
    PERMISSION_CATALOG_READ,
    PERMISSION_CATALOG_MANAGE,
    PERMISSION_INVENTORY_READ,
    PERMISSION_INVENTORY_MANAGE,
    PERMISSION_SUPPLIERS_READ,
    PERMISSION_SUPPLIERS_MANAGE,
    PERMISSION_CUSTOMERS_READ,
    PERMISSION_CUSTOMERS_MANAGE,
    PERMISSION_CUSTOMER_DEBTS_READ,
    PERMISSION_CUSTOMER_PAYMENTS_MANAGE,
    PERMISSION_POS_USE,
    PERMISSION_SALES_READ,
    PERMISSION_SALES_MANAGE,
    PERMISSION_PURCHASES_READ,
    PERMISSION_PURCHASES_MANAGE,
    PERMISSION_EXPENSES_READ,
    PERMISSION_EXPENSES_MANAGE,
];

const DEFAULT_MANAGER_PERMISSION_IDS: &[&str] = &[
    PERMISSION_SYSTEM_READ,
    PERMISSION_BUSINESS_READ,
    PERMISSION_CASH_ACCOUNT_BALANCE_READ,
    PERMISSION_CATALOG_READ,
    PERMISSION_CATALOG_MANAGE,
    PERMISSION_INVENTORY_READ,
    PERMISSION_INVENTORY_MANAGE,
    PERMISSION_SUPPLIERS_READ,
    PERMISSION_SUPPLIERS_MANAGE,
    PERMISSION_CUSTOMERS_READ,
    PERMISSION_CUSTOMERS_MANAGE,
    PERMISSION_CUSTOMER_DEBTS_READ,
    PERMISSION_CUSTOMER_PAYMENTS_MANAGE,
    PERMISSION_POS_USE,
    PERMISSION_SALES_READ,
    PERMISSION_SALES_MANAGE,
    PERMISSION_PURCHASES_READ,
    PERMISSION_PURCHASES_MANAGE,
    PERMISSION_EXPENSES_READ,
    PERMISSION_EXPENSES_MANAGE,
];

/// Application state holding the single SQLite connection.
pub struct DbState {
    conn: Mutex<Connection>,
    session: Mutex<Option<AuthUser>>,
}

impl DbState {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            session: Mutex::new(None),
        }
    }

    pub fn with_public<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        f(&conn)
    }

    pub fn with_authenticated<T>(
        &self,
        f: impl FnOnce(&Connection, &AuthUser) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        let session = self
            .session
            .lock()
            .map_err(|_| DbError::LockPoisoned)?
            .clone()
            .ok_or_else(|| DbError::InvalidInput("sign in to continue".to_string()))?;
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        if !Self::is_active_session(&conn, &session)? {
            self.clear_session()?;
            return Err(DbError::InvalidInput(
                "your account is inactive or no longer authorized; sign in again".to_string(),
            ));
        }
        f(&conn, &session)
    }

    fn is_active_session(conn: &Connection, session: &AuthUser) -> Result<bool, DbError> {
        conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM users u
                JOIN user_roles ur ON ur.user_id = u.id
                JOIN roles r ON r.id = ur.role_id AND r.business_id = u.business_id
                WHERE u.id = ?1 AND u.business_id = ?2 AND u.is_active = 1
                  AND r.name IN ('owner', 'manager') AND r.name = ?3
            )",
            params![session.id, session.business_id, session.role],
            |row| row.get(0),
        )
        .map_err(DbError::from)
    }

    pub fn current_session_user(&self) -> Result<Option<AuthUser>, DbError> {
        let Some(session) = self
            .session
            .lock()
            .map_err(|_| DbError::LockPoisoned)?
            .clone()
        else {
            return Ok(None);
        };
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        if !Self::is_active_session(&conn, &session)? {
            self.clear_session()?;
            return Ok(None);
        }
        Ok(Some(session))
    }

    pub fn with_authorized<T>(
        &self,
        permission: &str,
        f: impl FnOnce(&Connection, &AuthUser) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        self.with_authenticated(|conn, user| {
            require_permission(conn, user, permission)?;
            f(conn, user)
        })
    }

    pub fn with_business_permission<T>(
        &self,
        business_id: &str,
        permission: &str,
        f: impl FnOnce(&Connection, &AuthUser) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        self.with_authorized(permission, |conn, user| {
            if user.business_id != business_id {
                return Err(DbError::InvalidInput(
                    "you are not signed in to this business".to_string(),
                ));
            }
            f(conn, user)
        })
    }

    pub fn set_session(&self, user: AuthUser) -> Result<(), DbError> {
        *self.session.lock().map_err(|_| DbError::LockPoisoned)? = Some(user);
        Ok(())
    }

    pub fn clear_session(&self) -> Result<(), DbError> {
        *self.session.lock().map_err(|_| DbError::LockPoisoned)? = None;
        Ok(())
    }
}

fn require_permission(conn: &Connection, user: &AuthUser, permission: &str) -> Result<(), DbError> {
    let allowed: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM user_roles ur
            JOIN roles r ON r.id = ur.role_id AND r.business_id = ?2
            JOIN role_permissions rp ON rp.role_id = r.id
            JOIN permissions p ON p.id = rp.permission_id
            WHERE ur.user_id = ?1 AND p.id = ?3
        )",
        params![user.id, user.business_id, permission],
        |row| row.get(0),
    )?;
    if allowed {
        Ok(())
    } else {
        Err(DbError::InvalidInput(format!(
            "your role does not have permission: {permission}"
        )))
    }
}

pub fn list_user_permissions(conn: &Connection, user: &AuthUser) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT p.id
         FROM user_roles ur
         JOIN roles r ON r.id = ur.role_id AND r.business_id = ?2
         JOIN role_permissions rp ON rp.role_id = r.id
         JOIN permissions p ON p.id = rp.permission_id
         WHERE ur.user_id = ?1
         ORDER BY p.id",
    )?;
    let permissions = stmt
        .query_map(params![user.id, user.business_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(permissions)
}

fn grant_owner_permissions(conn: &Connection, role_id: &str) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
         SELECT ?1, id FROM permissions",
        params![role_id],
    )?;
    Ok(())
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
        if *version == 11 {
            backfill_inventory_cost_layers(&tx)
                .map_err(|e| DbError::Migration(*version, e.to_string()))?;
        }
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![version, name, now_utc()],
        )
        .map_err(|e| DbError::Migration(*version, e.to_string()))?;
        tx.commit().map_err(DbError::Sqlite)?;
    }
    Ok(())
}

struct BackfillCostLayer {
    movement_id: Option<String>,
    source_type: String,
    quantity_received: i64,
    quantity_remaining: i64,
    unit_cost: i64,
    created_at: String,
}

fn backfill_inventory_cost_layers(conn: &Connection) -> Result<(), DbError> {
    let balances = {
        let mut stmt = conn.prepare(
            "SELECT ib.business_id, ib.product_id, ib.quantity_on_hand,
                    ib.average_unit_cost, p.purchase_price, ib.updated_at
             FROM inventory_balances ib
             JOIN products p ON p.id = ib.product_id AND p.business_id = ib.business_id
             ORDER BY ib.business_id, ib.product_id",
        )?;
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
    };

    for (business_id, product_id, actual_quantity, average_cost, product_cost, updated_at) in
        balances
    {
        let movements = {
            let mut stmt = conn.prepare(
                "SELECT id, movement_type, quantity_delta, unit_cost, created_at
                 FROM stock_movements
                 WHERE business_id = ?1 AND product_id = ?2
                 ORDER BY created_at, rowid",
            )?;
            stmt.query_map(params![business_id, product_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        let mut layers = VecDeque::new();
        for (movement_id, movement_type, quantity_delta, movement_cost, created_at) in movements {
            if quantity_delta > 0 {
                let unit_cost = movement_cost.unwrap_or(if average_cost > 0 {
                    average_cost
                } else {
                    product_cost
                });
                layers.push_back(BackfillCostLayer {
                    movement_id: Some(movement_id),
                    source_type: movement_type,
                    quantity_received: quantity_delta,
                    quantity_remaining: quantity_delta,
                    unit_cost,
                    created_at,
                });
            } else if quantity_delta < 0 {
                let mut to_consume = quantity_delta.checked_abs().ok_or_else(|| {
                    DbError::InvalidInput("legacy stock movement quantity is invalid".into())
                })?;
                while to_consume > 0 {
                    let Some(layer) = layers.front_mut() else {
                        break;
                    };
                    let consumed = to_consume.min(layer.quantity_remaining);
                    layer.quantity_remaining -= consumed;
                    to_consume -= consumed;
                    if layer.quantity_remaining == 0 {
                        layers.pop_front();
                    }
                }
            }
        }

        let reconstructed_quantity = layers.iter().try_fold(0_i64, |total, layer| {
            total.checked_add(layer.quantity_remaining).ok_or_else(|| {
                DbError::InvalidInput("legacy stock quantity exceeds the supported limit".into())
            })
        })?;
        if reconstructed_quantity > actual_quantity {
            let mut excess = reconstructed_quantity - actual_quantity;
            while excess > 0 {
                let Some(layer) = layers.front_mut() else {
                    break;
                };
                let removed = excess.min(layer.quantity_remaining);
                layer.quantity_remaining -= removed;
                excess -= removed;
                if layer.quantity_remaining == 0 {
                    layers.pop_front();
                }
            }
        } else if reconstructed_quantity < actual_quantity {
            layers.push_back(BackfillCostLayer {
                movement_id: None,
                source_type: "legacy_remainder".to_string(),
                quantity_received: actual_quantity - reconstructed_quantity,
                quantity_remaining: actual_quantity - reconstructed_quantity,
                unit_cost: if average_cost > 0 {
                    average_cost
                } else {
                    product_cost
                },
                created_at: updated_at,
            });
        }

        for layer in layers {
            conn.execute(
                "INSERT INTO inventory_cost_layers
                    (id, business_id, product_id, source_movement_id, source_type,
                     quantity_received, quantity_remaining, unit_cost, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    business_id,
                    product_id,
                    layer.movement_id,
                    layer.source_type,
                    layer.quantity_received,
                    layer.quantity_remaining,
                    layer.unit_cost,
                    layer.created_at
                ],
            )?;
        }
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

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AuthUser {
    pub id: String,
    pub business_id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AuthBusinessStatus {
    pub id: String,
    pub name: String,
    pub requires_owner_setup: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct CashAccount {
    pub id: String,
    pub business_id: String,
    pub account_type: String,
    pub name: String,
    pub responsible_user_id: Option<String>,
    pub responsible_name: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashAccountPaymentOption {
    pub id: String,
    pub account_type: String,
    pub name: String,
    pub available_balance: i64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashTransfer {
    pub id: String,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount: i64,
    pub status: String,
    pub confirmation_reference: Option<String>,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Employee {
    pub id: String,
    pub business_id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub is_active: bool,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionSetting {
    pub permission_id: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionGrant {
    pub permission_id: String,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct AuthCredentials {
    pub user: AuthUser,
    pub password_hash: String,
}

pub fn auth_business_status(conn: &Connection) -> Result<Vec<AuthBusinessStatus>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT b.id, b.name,
                NOT EXISTS (SELECT 1 FROM users u WHERE u.business_id = b.id)
         FROM businesses b WHERE b.is_active = 1
         ORDER BY b.name COLLATE NOCASE, b.id",
    )?;
    let businesses = stmt
        .query_map([], |row| {
            Ok(AuthBusinessStatus {
                id: row.get(0)?,
                name: row.get(1)?,
                requires_owner_setup: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(businesses)
}

fn validate_auth_identity(username: &str, full_name: &str) -> Result<(String, String), DbError> {
    let username = username.trim().to_ascii_lowercase();
    if !(3..=32).contains(&username.len())
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(DbError::InvalidInput(
            "username must be 3–32 characters using letters, numbers, dots, underscores or hyphens"
                .to_string(),
        ));
    }
    let full_name = full_name.trim();
    if full_name.is_empty() || full_name.chars().count() > 120 {
        return Err(DbError::InvalidInput(
            "full name is required and must be 120 characters or fewer".to_string(),
        ));
    }
    Ok((username, full_name.to_string()))
}

pub fn bootstrap_owner(
    conn: &Connection,
    business_id: Option<&str>,
    new_business_name: Option<&str>,
    username: &str,
    full_name: &str,
    password_hash: &str,
) -> Result<AuthUser, DbError> {
    let (username, full_name) = validate_auth_identity(username, full_name)?;
    if password_hash.trim().is_empty() {
        return Err(DbError::InvalidInput(
            "a valid password hash is required".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let business_id = if let Some(business_id) = business_id {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM businesses WHERE id = ?1 AND is_active = 1)",
            params![business_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DbError::InvalidInput("business does not exist".to_string()));
        }
        business_id.to_string()
    } else {
        let business_count: i64 =
            tx.query_row("SELECT COUNT(*) FROM businesses", [], |row| row.get(0))?;
        if business_count != 0 {
            return Err(DbError::InvalidInput(
                "select an existing business to set up its first owner".to_string(),
            ));
        }
        let name = new_business_name.map(str::trim).unwrap_or_default();
        if name.is_empty() || name.chars().count() > 120 {
            return Err(DbError::InvalidInput(
                "business name is required and must be 120 characters or fewer".to_string(),
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = now_utc();
        tx.execute(
            "INSERT INTO businesses (id, name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)",
            params![id, name, timestamp],
        )?;
        id
    };

    let existing_user_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM users WHERE business_id = ?1",
        params![business_id],
        |row| row.get(0),
    )?;
    if existing_user_count != 0 {
        return Err(DbError::InvalidInput(
            "an owner account has already been created for this business".to_string(),
        ));
    }

    let role_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO roles (id, business_id, name, display_name, is_system, created_at, updated_at)
         VALUES (?1, ?2, 'owner', 'Owner', 1, ?3, ?3)
         ON CONFLICT (business_id, name) DO NOTHING",
        params![role_id, business_id, now_utc()],
    )?;
    let role_id: String = tx.query_row(
        "SELECT id FROM roles WHERE business_id = ?1 AND name = 'owner'",
        params![business_id],
        |row| row.get(0),
    )?;
    grant_owner_permissions(&tx, &role_id)?;
    let user_id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO users
            (id, business_id, username, full_name, password_hash, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            user_id,
            business_id,
            username,
            full_name,
            password_hash,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO user_roles (id, user_id, role_id, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            user_id,
            role_id,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'owner_bootstrapped', 'user', ?3,
                 'Initial owner account created', ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            user_id,
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(AuthUser {
        id: user_id,
        business_id,
        username,
        full_name,
        role: "owner".to_string(),
    })
}

pub fn create_business_for_owner(
    conn: &Connection,
    current_user: &AuthUser,
    name: &str,
    tax_id: Option<&str>,
) -> Result<Business, DbError> {
    if current_user.role != "owner" {
        return Err(DbError::InvalidInput(
            "an active owner account is required".to_string(),
        ));
    }
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(DbError::InvalidInput(
            "business name is required and must be 120 characters or fewer".to_string(),
        ));
    }
    let tax_id = tax_id.map(str::trim).filter(|value| !value.is_empty());
    if tax_id.is_some_and(|value| value.chars().count() > 80) {
        return Err(DbError::InvalidInput(
            "tax identification number must be 80 characters or fewer".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let (username, full_name, password_hash): (String, String, String) = tx
        .query_row(
            "SELECT u.username, u.full_name, u.password_hash
             FROM users u
             JOIN user_roles ur ON ur.user_id = u.id
             JOIN roles r ON r.id = ur.role_id AND r.business_id = u.business_id
             WHERE u.id = ?1 AND u.business_id = ?2 AND u.is_active = 1
               AND r.name = 'owner'",
            params![current_user.id, current_user.business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| DbError::InvalidInput("an active owner account is required".to_string()))?;

    let business_id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO businesses (id, name, tax_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![business_id, name, tax_id, timestamp],
    )?;
    let role_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO roles (id, business_id, name, display_name, is_system, created_at, updated_at)
         VALUES (?1, ?2, 'owner', 'Owner', 1, ?3, ?3)",
        params![role_id, business_id, timestamp],
    )?;
    grant_owner_permissions(&tx, &role_id)?;
    let user_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO users
            (id, business_id, username, full_name, password_hash, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            user_id,
            business_id,
            username,
            full_name,
            password_hash,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO user_roles (id, user_id, role_id, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            user_id,
            role_id,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'business_created', 'business', ?2,
                 'Business and owner access created', ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            user_id,
            timestamp
        ],
    )?;
    let business = tx.query_row(
        "SELECT id, name, tax_id, currency, timezone, date_format, language
         FROM businesses WHERE id = ?1",
        params![business_id],
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
    )?;
    tx.commit()?;
    Ok(business)
}

pub fn auth_credentials(
    conn: &Connection,
    business_id: &str,
    username: &str,
) -> Result<Option<AuthCredentials>, DbError> {
    let username = username.trim().to_ascii_lowercase();
    let result = conn
        .query_row(
            "SELECT u.id, u.business_id, u.username, u.full_name, r.name, u.password_hash
             FROM users u
             JOIN user_roles ur ON ur.user_id = u.id
             JOIN roles r ON r.id = ur.role_id AND r.business_id = u.business_id
             WHERE u.business_id = ?1 AND u.username = ?2 AND u.is_active = 1
               AND r.name IN ('owner', 'manager')
             ORDER BY CASE r.name WHEN 'owner' THEN 0 ELSE 1 END
             LIMIT 1",
            params![business_id, username],
            |row| {
                Ok(AuthCredentials {
                    user: AuthUser {
                        id: row.get(0)?,
                        business_id: row.get(1)?,
                        username: row.get(2)?,
                        full_name: row.get(3)?,
                        role: row.get(4)?,
                    },
                    password_hash: row.get(5)?,
                })
            },
        )
        .optional()?;
    Ok(result)
}

pub fn record_successful_login(conn: &Connection, user: &AuthUser) -> Result<(), DbError> {
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "UPDATE users SET last_login_at = ?1
         WHERE id = ?2 AND business_id = ?3 AND is_active = 1",
        params![timestamp, user.id, user.business_id],
    )?;
    if updated != 1 {
        return Err(DbError::InvalidInput(
            "user account is inactive; sign in again".to_string(),
        ));
    }
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, created_at)
         VALUES (?1, ?2, ?3, 'user_signed_in', 'user', ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            user.business_id,
            user.id,
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list_employees(conn: &Connection, business_id: &str) -> Result<Vec<Employee>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.business_id, u.username, u.full_name, r.name, u.is_active,
                u.last_login_at, u.created_at
         FROM users u
         JOIN user_roles ur ON ur.user_id = u.id
         JOIN roles r ON r.id = ur.role_id AND r.business_id = u.business_id
         WHERE u.business_id = ?1 AND r.name IN ('manager', 'cashier')
         ORDER BY u.is_active DESC, u.full_name COLLATE NOCASE",
    )?;
    let employees = stmt
        .query_map(params![business_id], |row| {
            Ok(Employee {
                id: row.get(0)?,
                business_id: row.get(1)?,
                username: row.get(2)?,
                full_name: row.get(3)?,
                role: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
                last_login_at: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(employees)
}

fn ensure_manager_role(
    conn: &Connection,
    business_id: &str,
    timestamp: &str,
) -> Result<(String, bool), DbError> {
    let created = conn.execute(
        "INSERT INTO roles (id, business_id, name, display_name, is_system, created_at, updated_at)
         VALUES (?1, ?2, 'manager', 'Manager', 1, ?3, ?3)
         ON CONFLICT (business_id, name) DO NOTHING",
        params![uuid::Uuid::new_v4().to_string(), business_id, timestamp],
    )? == 1;
    let role_id = conn.query_row(
        "SELECT id FROM roles WHERE business_id = ?1 AND name = 'manager'",
        params![business_id],
        |row| row.get(0),
    )?;
    Ok((role_id, created))
}

fn grant_default_manager_permissions(conn: &Connection, role_id: &str) -> Result<(), DbError> {
    for permission_id in DEFAULT_MANAGER_PERMISSION_IDS {
        conn.execute(
            "INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
             SELECT ?1, id FROM permissions WHERE id = ?2",
            params![role_id, permission_id],
        )?;
    }
    Ok(())
}

pub fn list_manager_permissions(
    conn: &Connection,
    business_id: &str,
) -> Result<Vec<RolePermissionSetting>, DbError> {
    let mut settings = Vec::with_capacity(MANAGER_PERMISSION_IDS.len());
    let manager_role_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM roles WHERE business_id = ?1 AND name = 'manager')",
        params![business_id],
        |row| row.get(0),
    )?;
    for permission_id in MANAGER_PERMISSION_IDS {
        let setting = conn
            .query_row(
                "SELECT p.description,
                        EXISTS(
                            SELECT 1 FROM roles r
                            JOIN role_permissions rp ON rp.role_id = r.id
                            WHERE r.business_id = ?1 AND r.name = 'manager'
                              AND rp.permission_id = p.id
                        )
                 FROM permissions p WHERE p.id = ?2",
                params![business_id, permission_id],
                |row| -> rusqlite::Result<RolePermissionSetting> {
                    Ok(RolePermissionSetting {
                        permission_id: (*permission_id).to_string(),
                        description: row.get(0)?,
                        enabled: if manager_role_exists {
                            row.get(1)?
                        } else {
                            DEFAULT_MANAGER_PERMISSION_IDS.contains(permission_id)
                        },
                    })
                },
            )
            .optional()?
            .ok_or_else(|| {
                DbError::InvalidInput(format!("permission catalogue is missing {permission_id}"))
            })?;
        settings.push(setting);
    }
    Ok(settings)
}

pub fn update_manager_permissions(
    conn: &Connection,
    business_id: &str,
    actor: &AuthUser,
    grants: &[RolePermissionGrant],
) -> Result<Vec<RolePermissionSetting>, DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "only the owner/admin can configure Manager permissions".to_string(),
        ));
    }
    require_permission(conn, actor, PERMISSION_ROLES_MANAGE)?;
    if grants.len() != MANAGER_PERMISSION_IDS.len() {
        return Err(DbError::InvalidInput(
            "submit the complete Manager permission set".to_string(),
        ));
    }
    let unique_ids = grants
        .iter()
        .map(|grant| grant.permission_id.as_str())
        .collect::<HashSet<_>>();
    if unique_ids.len() != MANAGER_PERMISSION_IDS.len()
        || MANAGER_PERMISSION_IDS
            .iter()
            .any(|permission_id| !unique_ids.contains(permission_id))
    {
        return Err(DbError::InvalidInput(
            "Manager permissions contain unknown or duplicate capabilities".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let timestamp = now_utc();
    let (manager_role_id, _) = ensure_manager_role(&tx, business_id, &timestamp)?;
    tx.execute(
        "DELETE FROM role_permissions WHERE role_id = ?1",
        params![manager_role_id],
    )?;
    let mut enabled = Vec::new();
    for grant in grants.iter().filter(|grant| grant.enabled) {
        tx.execute(
            "INSERT INTO role_permissions (role_id, permission_id) VALUES (?1, ?2)",
            params![manager_role_id, grant.permission_id],
        )?;
        enabled.push(grant.permission_id.as_str());
    }
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'manager_permissions_updated', 'role', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            manager_role_id,
            format!("enabled={}", enabled.join(",")),
            timestamp
        ],
    )?;
    tx.commit()?;
    list_manager_permissions(conn, business_id)
}

pub fn create_employee(
    conn: &Connection,
    business_id: &str,
    actor: &AuthUser,
    username: &str,
    full_name: &str,
    role: &str,
    password_hash: &str,
) -> Result<Employee, DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "an owner signed in to this business is required".to_string(),
        ));
    }
    if role != "manager" {
        return Err(DbError::InvalidInput(
            "only Manager profiles can be created; Owner/Admin is the full-access account"
                .to_string(),
        ));
    }
    if password_hash.trim().is_empty() {
        return Err(DbError::InvalidInput(
            "a valid password hash is required".to_string(),
        ));
    }
    let (username, full_name) = validate_auth_identity(username, full_name)?;
    let tx = conn.unchecked_transaction()?;
    let timestamp = now_utc();
    let (role_id, manager_role_created) = ensure_manager_role(&tx, business_id, &timestamp)?;
    if manager_role_created {
        grant_default_manager_permissions(&tx, &role_id)?;
    }
    let user_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO users
            (id, business_id, username, full_name, password_hash, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            user_id,
            business_id,
            username,
            full_name,
            password_hash,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO user_roles (id, user_id, role_id, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            user_id,
            role_id,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'employee_created', 'user', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            user_id,
            format!("role={role}"),
            timestamp
        ],
    )?;
    tx.commit()?;
    list_employees(conn, business_id)?
        .into_iter()
        .find(|employee| employee.id == user_id)
        .ok_or_else(|| DbError::InvalidInput("created employee could not be loaded".to_string()))
}

pub fn set_employee_active(
    conn: &Connection,
    business_id: &str,
    employee_id: &str,
    is_active: bool,
    actor: &AuthUser,
) -> Result<(), DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "an owner signed in to this business is required".to_string(),
        ));
    }
    if actor.id == employee_id {
        return Err(DbError::InvalidInput(
            "you cannot change your own account status here".to_string(),
        ));
    }
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "UPDATE users SET is_active = ?1, updated_at = ?2
         WHERE id = ?3 AND business_id = ?4 AND is_active != ?1
           AND EXISTS (
               SELECT 1 FROM user_roles ur
               JOIN roles r ON r.id = ur.role_id AND r.business_id = ?4
               WHERE ur.user_id = users.id AND r.name IN ('manager', 'cashier')
           )
           AND NOT EXISTS (
               SELECT 1 FROM user_roles ur
               JOIN roles r ON r.id = ur.role_id AND r.business_id = ?4
               WHERE ur.user_id = users.id AND r.name = 'owner'
           )",
        params![i64::from(is_active), timestamp, employee_id, business_id],
    )?;
    if updated == 0 {
        let is_employee: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM users u
                JOIN user_roles ur ON ur.user_id = u.id
                JOIN roles r ON r.id = ur.role_id AND r.business_id = u.business_id
                WHERE u.id = ?1 AND u.business_id = ?2
                  AND r.name IN ('manager', 'cashier')
                  AND NOT EXISTS (
                      SELECT 1 FROM user_roles owner_ur
                      JOIN roles owner_role ON owner_role.id = owner_ur.role_id
                          AND owner_role.business_id = u.business_id
                      WHERE owner_ur.user_id = u.id AND owner_role.name = 'owner'
                  )
            )",
            params![employee_id, business_id],
            |row| row.get(0),
        )?;
        if !is_employee {
            return Err(DbError::InvalidInput(
                "employee does not exist in this business".to_string(),
            ));
        }
        return Ok(());
    }
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, ?4, 'user', ?5, ?6, ?7)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            if is_active {
                "employee_activated"
            } else {
                "employee_deactivated"
            },
            employee_id,
            format!("is_active={is_active}"),
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn reset_employee_password(
    conn: &Connection,
    business_id: &str,
    employee_id: &str,
    password_hash: &str,
    actor: &AuthUser,
) -> Result<(), DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "an owner signed in to this business is required".to_string(),
        ));
    }
    if password_hash.trim().is_empty() {
        return Err(DbError::InvalidInput(
            "a valid password hash is required".to_string(),
        ));
    }
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2
         WHERE id = ?3 AND business_id = ?4
           AND EXISTS (
               SELECT 1 FROM user_roles ur
               JOIN roles r ON r.id = ur.role_id AND r.business_id = ?4
               WHERE ur.user_id = users.id AND r.name = 'manager'
           )",
        params![password_hash, timestamp, employee_id, business_id],
    )?;
    if updated != 1 {
        return Err(DbError::InvalidInput(
            "Manager profile does not exist in this business".to_string(),
        ));
    }
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'manager_password_reset', 'user', ?4, 'password_changed', ?5)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            employee_id,
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list_cash_accounts(
    conn: &Connection,
    business_id: &str,
) -> Result<Vec<CashAccount>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.business_id, a.account_type, a.name, a.responsible_user_id,
                u.full_name, a.is_active, a.created_at, a.updated_at
         FROM cash_accounts a
         LEFT JOIN users u ON u.id = a.responsible_user_id
             AND u.business_id = a.business_id
         WHERE a.business_id = ?1
         ORDER BY a.is_active DESC, a.account_type, a.name COLLATE NOCASE",
    )?;
    let accounts = stmt
        .query_map(params![business_id], |row| {
            Ok(CashAccount {
                id: row.get(0)?,
                business_id: row.get(1)?,
                account_type: row.get(2)?,
                name: row.get(3)?,
                responsible_user_id: row.get(4)?,
                responsible_name: row.get(5)?,
                is_active: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(accounts)
}

pub fn cash_account_balance(
    conn: &Connection,
    business_id: &str,
    account_id: &str,
) -> Result<i64, DbError> {
    let opening: Option<(i64, String)> = conn
        .query_row(
            "SELECT opening_amount, recorded_at FROM cash_account_openings
             WHERE business_id = ?1 AND account_id = ?2",
            params![business_id, account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (opening_amount, opened_at) = opening.ok_or_else(|| {
        DbError::InvalidInput(
            "this account has no authorized opening balance; ask the Owner/Admin to set one"
                .to_string(),
        )
    })?;
    let mut stmt = conn.prepare(
        "SELECT direction, amount FROM cash_movements
         WHERE business_id = ?1 AND account_id = ?2 AND recorded_at >= ?3
         ORDER BY recorded_at, id",
    )?;
    let movements = stmt
        .query_map(params![business_id, account_id, opened_at], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    movements
        .into_iter()
        .try_fold(opening_amount, |balance, (direction, amount)| {
            let next = if direction == "in" {
                balance.checked_add(amount)
            } else {
                balance.checked_sub(amount)
            };
            next.ok_or_else(|| {
                DbError::InvalidInput(
                    "cash account balance exceeds the supported range".to_string(),
                )
            })
        })
}

pub fn list_cash_account_payment_options(
    conn: &Connection,
    business_id: &str,
) -> Result<Vec<CashAccountPaymentOption>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.account_type, a.name
         FROM cash_accounts a
         JOIN cash_account_openings o
           ON o.business_id = a.business_id AND o.account_id = a.id
         WHERE a.business_id = ?1 AND a.is_active = 1
         ORDER BY a.account_type, a.name COLLATE NOCASE",
    )?;
    let accounts = stmt
        .query_map(params![business_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    accounts
        .into_iter()
        .map(|(id, account_type, name)| {
            Ok(CashAccountPaymentOption {
                available_balance: cash_account_balance(conn, business_id, &id)?,
                id,
                account_type,
                name,
            })
        })
        .collect()
}

pub fn create_cash_account_opening(
    conn: &Connection,
    business_id: &str,
    account_id: &str,
    opening_amount: i64,
    actor: &AuthUser,
) -> Result<(), DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "only the Owner/Admin can authorize an account opening balance".to_string(),
        ));
    }
    require_permission(conn, actor, PERMISSION_CASH_ACCOUNTS_MANAGE)?;
    if opening_amount < 0 {
        return Err(DbError::InvalidInput(
            "opening balance cannot be negative".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let account_is_active: bool = tx
        .query_row(
            "SELECT is_active FROM cash_accounts WHERE id = ?1 AND business_id = ?2",
            params![account_id, business_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )
        .optional()?
        .ok_or_else(|| {
            DbError::InvalidInput("cash account does not belong to this business".to_string())
        })?;
    if !account_is_active {
        return Err(DbError::InvalidInput(
            "activate the cash account before opening its balance".to_string(),
        ));
    }
    let has_movement: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM cash_movements
         WHERE business_id = ?1 AND account_id = ?2)",
        params![business_id, account_id],
        |row| row.get(0),
    )?;
    if has_movement {
        return Err(DbError::InvalidInput(
            "an account with existing movements cannot receive an opening balance; use an authorized correction".to_string(),
        ));
    }
    let already_opened: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM cash_account_openings
         WHERE business_id = ?1 AND account_id = ?2)",
        params![business_id, account_id],
        |row| row.get(0),
    )?;
    if already_opened {
        return Err(DbError::InvalidInput(
            "this account already has an authorized opening balance; use an authorized correction"
                .to_string(),
        ));
    }

    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO cash_account_openings
            (id, business_id, account_id, opening_amount, opened_at,
             entered_by_user_id, authorized_by_user_id, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?5)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            account_id,
            opening_amount,
            timestamp,
            actor.id
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'cash_account_opened', 'cash_account', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            account_id,
            serde_json::json!({
                "opening_amount": opening_amount,
                "authorized_by": actor.id
            })
            .to_string(),
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn create_cash_transfer(
    conn: &Connection,
    business_id: &str,
    from_account_id: &str,
    to_account_id: &str,
    amount: i64,
    actor: &AuthUser,
) -> Result<CashTransfer, DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "only the Owner/Admin can authorize account transfers".to_string(),
        ));
    }
    require_permission(conn, actor, PERMISSION_CASH_ACCOUNTS_MANAGE)?;
    if from_account_id == to_account_id {
        return Err(DbError::InvalidInput(
            "select two different cash accounts".to_string(),
        ));
    }
    if amount <= 0 {
        return Err(DbError::InvalidInput(
            "transfer amount must be greater than zero".to_string(),
        ));
    }
    let tx = conn.unchecked_transaction()?;
    for account_id in [from_account_id, to_account_id] {
        let is_active: bool = tx
            .query_row(
                "SELECT is_active FROM cash_accounts
                 WHERE business_id = ?1 AND id = ?2",
                params![business_id, account_id],
                |row| row.get::<_, i64>(0).map(|value| value != 0),
            )
            .optional()?
            .ok_or_else(|| {
                DbError::InvalidInput("cash account does not belong to this business".to_string())
            })?;
        if !is_active {
            return Err(DbError::InvalidInput(
                "both transfer accounts must be active".to_string(),
            ));
        }
        cash_account_balance(&tx, business_id, account_id)?;
    }
    let available = cash_account_balance(&tx, business_id, from_account_id)?;
    if amount > available {
        return Err(DbError::InvalidInput(format!(
            "transfer exceeds the available balance of {available} in the source account"
        )));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO cash_transfers
            (id, business_id, from_account_id, to_account_id, amount,
             occurred_at, initiated_by_user_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6)",
        params![
            id,
            business_id,
            from_account_id,
            to_account_id,
            amount,
            timestamp,
            actor.id
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'cash_transfer_created', 'cash_transfer', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            id,
            serde_json::json!({
                "from_account_id": from_account_id,
                "to_account_id": to_account_id,
                "amount": amount
            })
            .to_string(),
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(CashTransfer {
        id,
        from_account_id: from_account_id.to_string(),
        to_account_id: to_account_id.to_string(),
        amount,
        status: "pending".to_string(),
        confirmation_reference: None,
        occurred_at: timestamp.clone(),
        created_at: timestamp,
    })
}

pub fn confirm_cash_transfer(
    conn: &Connection,
    business_id: &str,
    transfer_id: &str,
    confirmation_reference: Option<&str>,
    actor: &AuthUser,
) -> Result<(), DbError> {
    if actor.role != "owner" || actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "only the Owner/Admin can confirm account transfers".to_string(),
        ));
    }
    require_permission(conn, actor, PERMISSION_CASH_ACCOUNTS_MANAGE)?;
    let reference = confirmation_reference
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if reference.is_some_and(|value| value.chars().count() > 200) {
        return Err(DbError::InvalidInput(
            "transfer confirmation reference must be 200 characters or fewer".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let transfer: Option<(String, String, i64, String)> = tx
        .query_row(
            "SELECT from_account_id, to_account_id, amount, status
             FROM cash_transfers WHERE id = ?1 AND business_id = ?2",
            params![transfer_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let (from_account_id, to_account_id, amount, status) = transfer.ok_or_else(|| {
        DbError::InvalidInput("cash transfer does not belong to this business".to_string())
    })?;
    if status != "pending" {
        return Err(DbError::InvalidInput(
            "only a pending transfer can be confirmed".to_string(),
        ));
    }
    for account_id in [&from_account_id, &to_account_id] {
        let is_active: bool = tx.query_row(
            "SELECT is_active FROM cash_accounts
             WHERE business_id = ?1 AND id = ?2",
            params![business_id, account_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )?;
        if !is_active {
            return Err(DbError::InvalidInput(
                "both transfer accounts must be active when the transfer is confirmed".to_string(),
            ));
        }
    }
    let available = cash_account_balance(&tx, business_id, &from_account_id)?;
    if amount > available {
        return Err(DbError::InvalidInput(format!(
            "transfer exceeds the available balance of {available} in the source account"
        )));
    }
    let timestamp = now_utc();
    let updated = tx.execute(
        "UPDATE cash_transfers
         SET status = 'confirmed', confirmed_by_user_id = ?1, confirmed_at = ?2,
             confirmation_reference = ?3
         WHERE id = ?4 AND business_id = ?5 AND status = 'pending'",
        params![actor.id, timestamp, reference, transfer_id, business_id],
    )?;
    if updated != 1 {
        return Err(DbError::InvalidInput(
            "cash transfer changed before it could be confirmed".to_string(),
        ));
    }
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'cash_transfer_confirmed', 'cash_transfer', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            transfer_id,
            serde_json::json!({
                "from_account_id": from_account_id,
                "to_account_id": to_account_id,
                "amount": amount,
                "confirmation_reference": reference
            })
            .to_string(),
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list_cash_transfers(
    conn: &Connection,
    business_id: &str,
) -> Result<Vec<CashTransfer>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, from_account_id, to_account_id, amount, status,
                confirmation_reference, occurred_at, created_at
         FROM cash_transfers
         WHERE business_id = ?1
         ORDER BY created_at DESC, id DESC
         LIMIT 100",
    )?;
    let transfers = stmt
        .query_map(params![business_id], |row| {
            Ok(CashTransfer {
                id: row.get(0)?,
                from_account_id: row.get(1)?,
                to_account_id: row.get(2)?,
                amount: row.get(3)?,
                status: row.get(4)?,
                confirmation_reference: row.get(5)?,
                occurred_at: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(transfers)
}

fn expected_account_type(payment_method: &str) -> Option<&'static str> {
    match payment_method {
        "cash" => Some("cash_drawer"),
        "bank_transfer" => Some("bank"),
        "mtn_mobile_money" => Some("mtn_wallet"),
        "airtel_money" => Some("airtel_wallet"),
        _ => None,
    }
}

fn validate_payment_account(
    conn: &Connection,
    business_id: &str,
    account_id: &str,
    payment_method: &str,
) -> Result<i64, DbError> {
    let expected_type = expected_account_type(payment_method)
        .ok_or_else(|| DbError::InvalidInput("select a supported payment method".to_string()))?;
    let account_type: Option<(String, bool)> = conn
        .query_row(
            "SELECT account_type, is_active FROM cash_accounts
             WHERE business_id = ?1 AND id = ?2",
            params![business_id, account_id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()?;
    let (account_type, active) = account_type.ok_or_else(|| {
        DbError::InvalidInput("select a cash account belonging to this business".to_string())
    })?;
    if account_type != expected_type {
        return Err(DbError::InvalidInput(
            "selected cash account does not match the payment method".to_string(),
        ));
    }
    if !active {
        return Err(DbError::InvalidInput(
            "selected cash account is inactive".to_string(),
        ));
    }
    cash_account_balance(conn, business_id, account_id)
}

fn insert_cash_movement(
    conn: &Connection,
    business_id: &str,
    account_id: &str,
    movement_type: &str,
    direction: &str,
    amount: i64,
    source_type: &str,
    source_id: &str,
    responsible_user_id: &str,
    timestamp: &str,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO cash_movements
            (id, business_id, account_id, movement_type, direction, amount,
             source_type, source_id, source_leg, responsible_user_id,
             occurred_at, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'single', ?9, ?10, ?10)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            account_id,
            movement_type,
            direction,
            amount,
            source_type,
            source_id,
            responsible_user_id,
            timestamp
        ],
    )?;
    Ok(())
}

#[cfg(test)]
fn test_payment_account(
    conn: &Connection,
    business_id: &str,
    payment_method: &str,
) -> Result<(String, AuthUser), DbError> {
    let account_type = expected_account_type(payment_method)
        .ok_or_else(|| DbError::InvalidInput("select a supported payment method".to_string()))?;
    let actor: Option<AuthUser> = conn
        .query_row(
            "SELECT u.id, u.business_id, u.username, u.full_name, r.name
             FROM users u
             JOIN user_roles ur ON ur.user_id = u.id
             JOIN roles r ON r.id = ur.role_id AND r.business_id = u.business_id
             WHERE u.business_id = ?1
             ORDER BY CASE r.name WHEN 'owner' THEN 0 ELSE 1 END, u.created_at
             LIMIT 1",
            params![business_id],
            |row| {
                Ok(AuthUser {
                    id: row.get(0)?,
                    business_id: row.get(1)?,
                    username: row.get(2)?,
                    full_name: row.get(3)?,
                    role: row.get(4)?,
                })
            },
        )
        .optional()?;
    let actor = match actor {
        Some(actor) => actor,
        None => bootstrap_owner(
            conn,
            Some(business_id),
            None,
            "test.owner",
            "Test Owner",
            "test-password-hash",
        )?,
    };
    let account: Option<String> = conn
        .query_row(
            "SELECT id FROM cash_accounts
             WHERE business_id = ?1 AND account_type = ?2 AND is_active = 1
             ORDER BY created_at LIMIT 1",
            params![business_id, account_type],
            |row| row.get(0),
        )
        .optional()?;
    let account_id = if let Some(account_id) = account {
        account_id
    } else {
        create_cash_account(
            conn,
            business_id,
            &actor,
            account_type,
            &format!("Test {account_type}"),
        )?
        .id
    };
    let has_opening: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM cash_account_openings
         WHERE business_id = ?1 AND account_id = ?2)",
        params![business_id, account_id],
        |row| row.get(0),
    )?;
    if !has_opening {
        let timestamp = now_utc();
        conn.execute(
            "INSERT INTO cash_account_openings
                (id, business_id, account_id, opening_amount, opened_at,
                 entered_by_user_id, authorized_by_user_id, recorded_at)
             VALUES (?1, ?2, ?3, 1000000000, ?4, ?5, ?5, ?4)",
            params![
                uuid::Uuid::new_v4().to_string(),
                business_id,
                account_id,
                timestamp,
                actor.id
            ],
        )?;
    }
    Ok((account_id, actor))
}

pub fn create_cash_account(
    conn: &Connection,
    business_id: &str,
    responsible_user: &AuthUser,
    account_type: &str,
    name: &str,
) -> Result<CashAccount, DbError> {
    if !matches!(
        account_type,
        "cash_drawer" | "bank" | "mtn_wallet" | "airtel_wallet" | "other"
    ) {
        return Err(DbError::InvalidInput(
            "select a supported cash account type".to_string(),
        ));
    }
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(DbError::InvalidInput(
            "account name is required and must be 120 characters or fewer".to_string(),
        ));
    }
    if responsible_user.business_id != business_id {
        return Err(DbError::InvalidInput(
            "you are not signed in to this business".to_string(),
        ));
    }

    let account_id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO cash_accounts
            (id, business_id, account_type, name, responsible_user_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            account_id,
            business_id,
            account_type,
            name,
            if account_type == "cash_drawer" {
                Some(responsible_user.id.as_str())
            } else {
                None
            },
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'cash_account_created', 'cash_account', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            responsible_user.id,
            account_id,
            account_type,
            timestamp
        ],
    )?;
    tx.commit()?;
    list_cash_accounts(conn, business_id)?
        .into_iter()
        .find(|account| account.id == account_id)
        .ok_or_else(|| {
            DbError::InvalidInput("created cash account could not be loaded".to_string())
        })
}

pub fn set_cash_account_active(
    conn: &Connection,
    business_id: &str,
    account_id: &str,
    is_active: bool,
    actor: &AuthUser,
) -> Result<(), DbError> {
    if actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "you are not signed in to this business".to_string(),
        ));
    }
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "UPDATE cash_accounts SET is_active = ?1, updated_at = ?2
         WHERE id = ?3 AND business_id = ?4 AND is_active != ?1",
        params![i64::from(is_active), timestamp, account_id, business_id],
    )?;
    if updated == 0 {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM cash_accounts WHERE id = ?1 AND business_id = ?2)",
            params![account_id, business_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DbError::InvalidInput(
                "cash account does not exist in this business".to_string(),
            ));
        }
        return Ok(());
    }
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, ?4, 'cash_account', ?5, ?6, ?7)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            if is_active {
                "cash_account_activated"
            } else {
                "cash_account_deactivated"
            },
            account_id,
            format!("is_active={is_active}"),
            timestamp
        ],
    )?;
    tx.commit()?;
    Ok(())
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

#[cfg(test)]
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
pub struct Supplier {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub is_active: bool,
}

fn clean_supplier_field(
    value: Option<&str>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, DbError> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if value.is_some_and(|value| value.chars().count() > max_length) {
        return Err(DbError::InvalidInput(format!(
            "{field} must be {max_length} characters or fewer"
        )));
    }
    Ok(value.map(str::to_string))
}

fn supplier_name(name: &str) -> Result<String, DbError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::InvalidInput(
            "supplier name must not be empty".to_string(),
        ));
    }
    if name.chars().count() > 160 {
        return Err(DbError::InvalidInput(
            "supplier name must be 160 characters or fewer".to_string(),
        ));
    }
    Ok(name.to_string())
}

fn map_supplier(row: &rusqlite::Row<'_>) -> rusqlite::Result<Supplier> {
    Ok(Supplier {
        id: row.get(0)?,
        business_id: row.get(1)?,
        name: row.get(2)?,
        contact_person: row.get(3)?,
        phone: row.get(4)?,
        email: row.get(5)?,
        address: row.get(6)?,
        tax_id: row.get(7)?,
        is_active: row.get::<_, i64>(8)? != 0,
    })
}

pub fn list_suppliers(conn: &Connection, business_id: &str) -> Result<Vec<Supplier>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, name, contact_person, phone, email, address, tax_id, is_active
         FROM suppliers WHERE business_id = ?1
         ORDER BY is_active DESC, name COLLATE NOCASE, id",
    )?;
    let suppliers = stmt
        .query_map(params![business_id], map_supplier)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(suppliers)
}

pub fn create_supplier(
    conn: &Connection,
    business_id: &str,
    name: &str,
    contact_person: Option<&str>,
    phone: Option<&str>,
    email: Option<&str>,
    address: Option<&str>,
    tax_id: Option<&str>,
) -> Result<Supplier, DbError> {
    let name = supplier_name(name)?;
    let contact_person = clean_supplier_field(contact_person, "contact person", 160)?;
    let phone = clean_supplier_field(phone, "phone", 80)?;
    let email = clean_supplier_field(email, "email", 254)?;
    let address = clean_supplier_field(address, "address", 500)?;
    let tax_id = clean_supplier_field(tax_id, "tax ID", 100)?;
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
        "INSERT INTO suppliers
            (id, business_id, name, contact_person, phone, email, address, tax_id,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            business_id,
            name,
            contact_person,
            phone,
            email,
            address,
            tax_id,
            timestamp
        ],
    )?;
    conn.query_row(
        "SELECT id, business_id, name, contact_person, phone, email, address, tax_id, is_active
         FROM suppliers WHERE id = ?1 AND business_id = ?2",
        params![id, business_id],
        map_supplier,
    )
    .map_err(DbError::Sqlite)
}

pub fn update_supplier(
    conn: &Connection,
    business_id: &str,
    supplier_id: &str,
    name: &str,
    contact_person: Option<&str>,
    phone: Option<&str>,
    email: Option<&str>,
    address: Option<&str>,
    tax_id: Option<&str>,
    is_active: bool,
) -> Result<Supplier, DbError> {
    let name = supplier_name(name)?;
    let contact_person = clean_supplier_field(contact_person, "contact person", 160)?;
    let phone = clean_supplier_field(phone, "phone", 80)?;
    let email = clean_supplier_field(email, "email", 254)?;
    let address = clean_supplier_field(address, "address", 500)?;
    let tax_id = clean_supplier_field(tax_id, "tax ID", 100)?;
    let timestamp = now_utc();
    let updated = conn.execute(
        "UPDATE suppliers
         SET name = ?1, contact_person = ?2, phone = ?3, email = ?4,
             address = ?5, tax_id = ?6, is_active = ?7, updated_at = ?8
         WHERE id = ?9 AND business_id = ?10",
        params![
            name,
            contact_person,
            phone,
            email,
            address,
            tax_id,
            i64::from(is_active),
            timestamp,
            supplier_id,
            business_id
        ],
    )?;
    if updated != 1 {
        return Err(DbError::InvalidInput(
            "supplier does not belong to this business".to_string(),
        ));
    }
    conn.query_row(
        "SELECT id, business_id, name, contact_person, phone, email, address, tax_id, is_active
         FROM suppliers WHERE id = ?1 AND business_id = ?2",
        params![supplier_id, business_id],
        map_supplier,
    )
    .map_err(DbError::Sqlite)
}

#[derive(Debug, serde::Serialize)]
pub struct Customer {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub customer_type: String,
    pub credit_limit: i64,
    pub outstanding_balance: i64,
    pub is_active: bool,
}

fn customer_name(name: &str) -> Result<String, DbError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DbError::InvalidInput(
            "customer name must not be empty".to_string(),
        ));
    }
    if name.chars().count() > 160 {
        return Err(DbError::InvalidInput(
            "customer name must be 160 characters or fewer".to_string(),
        ));
    }
    Ok(name.to_string())
}

fn validate_customer_type(customer_type: &str) -> Result<(), DbError> {
    if !matches!(customer_type, "individual" | "business") {
        return Err(DbError::InvalidInput(
            "customer type must be individual or business".to_string(),
        ));
    }
    Ok(())
}

fn map_customer(row: &rusqlite::Row<'_>) -> rusqlite::Result<Customer> {
    Ok(Customer {
        id: row.get(0)?,
        business_id: row.get(1)?,
        name: row.get(2)?,
        phone: row.get(3)?,
        address: row.get(4)?,
        customer_type: row.get(5)?,
        credit_limit: row.get(6)?,
        outstanding_balance: 0,
        is_active: row.get::<_, i64>(7)? != 0,
    })
}

pub fn list_customers(conn: &Connection, business_id: &str) -> Result<Vec<Customer>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, name, phone, address, customer_type, credit_limit, is_active
         FROM customers WHERE business_id = ?1
         ORDER BY is_active DESC, name COLLATE NOCASE, id",
    )?;
    let mut customers = stmt
        .query_map(params![business_id], map_customer)?
        .collect::<Result<Vec<_>, _>>()?;
    for customer in &mut customers {
        customer.outstanding_balance =
            customer_outstanding_balance(conn, business_id, &customer.id)?;
    }
    Ok(customers)
}

fn customer_outstanding_balance(
    conn: &Connection,
    business_id: &str,
    customer_id: &str,
) -> Result<i64, DbError> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(
            d.original_amount
            - COALESCE((
                SELECT SUM(sp.amount) FROM sale_payments sp
                JOIN sales s ON s.id = sp.sale_id
                WHERE s.business_id = d.business_id AND s.id = d.sale_id
            ), 0)
            - COALESCE((
                SELECT SUM(dp.amount) FROM customer_debt_payments dp
                WHERE dp.business_id = d.business_id AND dp.debt_id = d.id
            ), 0)
         ), 0)
         FROM customer_debts d
         WHERE d.business_id = ?1 AND d.customer_id = ?2",
        params![business_id, customer_id],
        |row| row.get(0),
    )?)
}

fn load_customer(
    conn: &Connection,
    business_id: &str,
    customer_id: &str,
) -> Result<Customer, DbError> {
    let mut customer = conn.query_row(
        "SELECT id, business_id, name, phone, address, customer_type, credit_limit, is_active
         FROM customers WHERE id = ?1 AND business_id = ?2",
        params![customer_id, business_id],
        map_customer,
    )?;
    customer.outstanding_balance = customer_outstanding_balance(conn, business_id, customer_id)?;
    Ok(customer)
}

pub fn create_customer(
    conn: &Connection,
    business_id: &str,
    name: &str,
    phone: Option<&str>,
    address: Option<&str>,
    customer_type: &str,
    credit_limit: i64,
) -> Result<Customer, DbError> {
    let name = customer_name(name)?;
    let phone = clean_supplier_field(phone, "phone", 80)?;
    let address = clean_supplier_field(address, "address", 500)?;
    validate_customer_type(customer_type)?;
    if credit_limit < 0 {
        return Err(DbError::InvalidInput(
            "credit limit must not be negative".to_string(),
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
        "INSERT INTO customers
            (id, business_id, name, phone, address, customer_type, credit_limit,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            business_id,
            name,
            phone,
            address,
            customer_type,
            credit_limit,
            timestamp
        ],
    )?;
    load_customer(conn, business_id, &id)
}

pub fn update_customer(
    conn: &Connection,
    business_id: &str,
    customer_id: &str,
    name: &str,
    phone: Option<&str>,
    address: Option<&str>,
    customer_type: &str,
    credit_limit: i64,
    is_active: bool,
) -> Result<Customer, DbError> {
    let name = customer_name(name)?;
    let phone = clean_supplier_field(phone, "phone", 80)?;
    let address = clean_supplier_field(address, "address", 500)?;
    validate_customer_type(customer_type)?;
    if credit_limit < 0 {
        return Err(DbError::InvalidInput(
            "credit limit must not be negative".to_string(),
        ));
    }
    let timestamp = now_utc();
    let updated = conn.execute(
        "UPDATE customers
         SET name = ?1, phone = ?2, address = ?3, customer_type = ?4,
             credit_limit = ?5, is_active = ?6, updated_at = ?7
         WHERE id = ?8 AND business_id = ?9",
        params![
            name,
            phone,
            address,
            customer_type,
            credit_limit,
            i64::from(is_active),
            timestamp,
            customer_id,
            business_id
        ],
    )?;
    if updated != 1 {
        return Err(DbError::InvalidInput(
            "customer does not belong to this business".to_string(),
        ));
    }
    load_customer(conn, business_id, customer_id)
}

#[derive(Debug, serde::Serialize)]
pub struct CustomerDebt {
    pub id: String,
    pub business_id: String,
    pub customer_id: String,
    pub customer_name: String,
    pub sale_id: String,
    pub transaction_number: String,
    pub original_amount: i64,
    pub amount_paid: i64,
    pub balance_due: i64,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CustomerDebtPayment {
    pub id: String,
    pub debt_id: String,
    pub customer_name: String,
    pub payment_method: String,
    pub amount: i64,
    pub balance_due: i64,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CustomerStatementEntry {
    pub id: String,
    pub transaction_number: String,
    pub entry_type: String,
    pub payment_method: Option<String>,
    pub amount: i64,
    pub balance_after: i64,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CustomerStatement {
    pub customer: Customer,
    pub total_sales: i64,
    pub total_payments: i64,
    pub outstanding_balance: i64,
    pub entries: Vec<CustomerStatementEntry>,
}

fn valid_sale_payment_method(payment_method: &str) -> bool {
    matches!(
        payment_method,
        "cash" | "mtn_mobile_money" | "airtel_money" | "bank_transfer"
    )
}

pub fn customer_statement(
    conn: &Connection,
    business_id: &str,
    customer_id: &str,
) -> Result<CustomerStatement, DbError> {
    let customer = load_customer(conn, business_id, customer_id)?;
    let mut stmt = conn.prepare(
        "SELECT id, transaction_number, entry_type, payment_method, amount, created_at, order_rank
         FROM (
             SELECT s.id, s.transaction_number, 'sale' AS entry_type,
                    NULL AS payment_method, s.total AS amount, s.created_at, 0 AS order_rank
             FROM sales s
             WHERE s.business_id = ?1 AND s.customer_id = ?2 AND s.status = 'completed'
             UNION ALL
             SELECT sp.id, s.transaction_number, 'sale_payment' AS entry_type,
                    sp.payment_method, sp.amount, sp.created_at, 1 AS order_rank
             FROM sale_payments sp
             JOIN sales s ON s.id = sp.sale_id
             WHERE s.business_id = ?1 AND s.customer_id = ?2 AND s.status = 'completed'
             UNION ALL
             SELECT dp.id, s.transaction_number, 'repayment' AS entry_type,
                    dp.payment_method, dp.amount, dp.created_at, 2 AS order_rank
             FROM customer_debt_payments dp
             JOIN customer_debts d
               ON d.id = dp.debt_id AND d.business_id = dp.business_id
             JOIN sales s ON s.id = d.sale_id AND s.business_id = d.business_id
             WHERE dp.business_id = ?1 AND d.customer_id = ?2 AND s.status = 'completed'
         )
         ORDER BY created_at, order_rank, id",
    )?;
    let raw_entries = stmt
        .query_map(params![business_id, customer_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut total_sales = 0_i64;
    let mut total_payments = 0_i64;
    let mut balance = 0_i64;
    let mut entries = Vec::with_capacity(raw_entries.len());
    for (id, transaction_number, entry_type, payment_method, amount, created_at) in raw_entries {
        let is_sale = entry_type == "sale";
        if is_sale {
            total_sales = total_sales.checked_add(amount).ok_or_else(|| {
                DbError::InvalidInput("customer sales total exceeds the supported limit".into())
            })?;
            balance = balance.checked_add(amount).ok_or_else(|| {
                DbError::InvalidInput(
                    "customer statement balance exceeds the supported limit".into(),
                )
            })?;
        } else {
            total_payments = total_payments.checked_add(amount).ok_or_else(|| {
                DbError::InvalidInput("customer payment total exceeds the supported limit".into())
            })?;
            balance = balance
                .checked_sub(amount)
                .filter(|value| *value >= 0)
                .ok_or_else(|| {
                    DbError::InvalidInput(
                        "customer statement payments exceed recorded sales".into(),
                    )
                })?;
        }
        entries.push(CustomerStatementEntry {
            id,
            transaction_number,
            entry_type,
            payment_method,
            amount,
            balance_after: balance,
            created_at,
        });
    }

    if balance != customer.outstanding_balance {
        return Err(DbError::InvalidInput(
            "customer statement does not match the outstanding balance".to_string(),
        ));
    }

    Ok(CustomerStatement {
        outstanding_balance: customer.outstanding_balance,
        customer,
        total_sales,
        total_payments,
        entries,
    })
}

fn load_customer_debt(
    conn: &Connection,
    business_id: &str,
    debt_id: &str,
) -> Result<CustomerDebt, DbError> {
    let mut debt = conn
        .query_row(
            "SELECT d.id, d.business_id, d.customer_id, c.name, d.sale_id,
                s.transaction_number, d.original_amount, d.created_at,
                COALESCE(sp.amount_paid, 0) + COALESCE(dp.amount_paid, 0)
         FROM customer_debts d
         JOIN customers c ON c.id = d.customer_id AND c.business_id = d.business_id
         JOIN sales s ON s.id = d.sale_id AND s.business_id = d.business_id
         LEFT JOIN (
             SELECT sale_id, SUM(amount) AS amount_paid
             FROM sale_payments GROUP BY sale_id
         ) sp ON sp.sale_id = d.sale_id
         LEFT JOIN (
             SELECT business_id, debt_id, SUM(amount) AS amount_paid
             FROM customer_debt_payments GROUP BY business_id, debt_id
         ) dp ON dp.business_id = d.business_id AND dp.debt_id = d.id
         WHERE d.id = ?1 AND d.business_id = ?2",
            params![debt_id, business_id],
            |row| {
                Ok(CustomerDebt {
                    id: row.get(0)?,
                    business_id: row.get(1)?,
                    customer_id: row.get(2)?,
                    customer_name: row.get(3)?,
                    sale_id: row.get(4)?,
                    transaction_number: row.get(5)?,
                    original_amount: row.get(6)?,
                    amount_paid: row.get(8)?,
                    balance_due: 0,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            DbError::InvalidInput("customer debt does not belong to this business".to_string())
        })?;
    debt.balance_due = debt
        .original_amount
        .checked_sub(debt.amount_paid)
        .filter(|balance| *balance >= 0)
        .ok_or_else(|| DbError::InvalidInput("customer payments exceed the debt".to_string()))?;
    Ok(debt)
}

#[derive(Debug, serde::Serialize)]
pub struct Expense {
    pub id: String,
    pub business_id: String,
    pub category: String,
    pub amount: i64,
    pub expense_date: String,
    pub payment_method: String,
    pub cash_account_id: Option<String>,
    pub description: String,
    pub responsible_name: String,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ExpensePage {
    pub items: Vec<Expense>,
    pub total_count: i64,
}

fn validate_expense_category(category: &str) -> Result<(), DbError> {
    if !matches!(
        category,
        "rent"
            | "electricity"
            | "transport"
            | "salaries"
            | "internet"
            | "repairs"
            | "security"
            | "other"
    ) {
        return Err(DbError::InvalidInput(
            "select a supported expense category".to_string(),
        ));
    }
    Ok(())
}

fn validate_expense_date(expense_date: &str) -> Result<(), DbError> {
    let parsed = chrono::NaiveDate::parse_from_str(expense_date, "%Y-%m-%d")
        .map_err(|_| DbError::InvalidInput("enter a valid expense date".to_string()))?;
    if parsed.format("%Y-%m-%d").to_string() != expense_date {
        return Err(DbError::InvalidInput(
            "enter a valid expense date".to_string(),
        ));
    }
    Ok(())
}

fn map_expense(row: &rusqlite::Row<'_>) -> rusqlite::Result<Expense> {
    Ok(Expense {
        id: row.get(0)?,
        business_id: row.get(1)?,
        category: row.get(2)?,
        amount: row.get(3)?,
        expense_date: row.get(4)?,
        payment_method: row.get(5)?,
        cash_account_id: row.get(6)?,
        description: row.get(7)?,
        responsible_name: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn list_expenses(
    conn: &Connection,
    business_id: &str,
    offset: i64,
    limit: i64,
) -> Result<ExpensePage, DbError> {
    if offset < 0 || !(1..=200).contains(&limit) {
        return Err(DbError::InvalidInput(
            "expense pagination must use a non-negative offset and a limit from 1 to 200"
                .to_string(),
        ));
    }
    let total_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM expenses WHERE business_id = ?1",
        params![business_id],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT id, business_id, category, amount, expense_date, payment_method,
                cash_account_id, description, responsible_name, created_at
         FROM expenses WHERE business_id = ?1
         ORDER BY expense_date DESC, created_at DESC, id
         LIMIT ?2 OFFSET ?3",
    )?;
    let items = stmt
        .query_map(params![business_id, limit, offset], map_expense)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ExpensePage { items, total_count })
}

pub fn create_expense_as_user(
    conn: &Connection,
    business_id: &str,
    category: &str,
    amount: i64,
    expense_date: &str,
    payment_method: &str,
    description: &str,
    responsible_name: &str,
    account_id: &str,
    actor: &AuthUser,
) -> Result<Expense, DbError> {
    if actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "you are not signed in to this business".to_string(),
        ));
    }
    require_permission(conn, actor, PERMISSION_EXPENSES_MANAGE)?;
    validate_expense_category(category)?;
    validate_expense_date(expense_date)?;
    if !valid_sale_payment_method(payment_method) {
        return Err(DbError::InvalidInput(
            "select a supported payment method".to_string(),
        ));
    }
    if amount <= 0 {
        return Err(DbError::InvalidInput(
            "expense amount must be greater than zero".to_string(),
        ));
    }
    let description = description.trim();
    if description.is_empty() || description.chars().count() > 1000 {
        return Err(DbError::InvalidInput(
            "description must contain 1 to 1000 characters".to_string(),
        ));
    }
    let responsible_name = responsible_name.trim();
    if responsible_name.is_empty() || responsible_name.chars().count() > 160 {
        return Err(DbError::InvalidInput(
            "responsible person must contain 1 to 160 characters".to_string(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let available_balance = validate_payment_account(&tx, business_id, account_id, payment_method)?;
    if amount > available_balance {
        return Err(DbError::InvalidInput(format!(
            "expense exceeds the available balance of {available_balance} in the selected account"
        )));
    }
    tx.execute(
        "INSERT INTO expenses
            (id, business_id, category, amount, expense_date, payment_method,
             description, responsible_name, created_at, cash_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            business_id,
            category,
            amount,
            expense_date,
            payment_method,
            description,
            responsible_name,
            timestamp,
            account_id
        ],
    )?;
    insert_cash_movement(
        &tx,
        business_id,
        account_id,
        "expense_payment",
        "out",
        amount,
        "expense_payment",
        &id,
        &actor.id,
        &timestamp,
    )?;
    tx.execute(
        "INSERT INTO audit_logs
            (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, 'expense_created', 'expense', ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            actor.id,
            id,
            serde_json::json!({
                "category": category,
                "amount": amount,
                "expense_date": expense_date,
                "payment_method": payment_method,
                "responsible_name": responsible_name,
                "cash_account_id": account_id
            })
            .to_string(),
            timestamp
        ],
    )?;
    tx.commit()?;

    conn.query_row(
        "SELECT id, business_id, category, amount, expense_date, payment_method,
                cash_account_id, description, responsible_name, created_at
         FROM expenses WHERE id = ?1 AND business_id = ?2",
        params![id, business_id],
        map_expense,
    )
    .map_err(DbError::Sqlite)
}

#[cfg(test)]
pub fn create_expense(
    conn: &Connection,
    business_id: &str,
    category: &str,
    amount: i64,
    expense_date: &str,
    payment_method: &str,
    description: &str,
    responsible_name: &str,
) -> Result<Expense, DbError> {
    let (account_id, actor) = test_payment_account(conn, business_id, payment_method)?;
    create_expense_as_user(
        conn,
        business_id,
        category,
        amount,
        expense_date,
        payment_method,
        description,
        responsible_name,
        &account_id,
        &actor,
    )
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct DailyAmountBreakdown {
    pub category: String,
    pub amount: i64,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct DailyCountBreakdown {
    pub movement_type: String,
    pub count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DailyCashOutflowBreakdown {
    pub category: String,
    pub description: String,
    pub amount: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DailyBusinessReport {
    pub business_name: String,
    pub report_date: String,
    pub money_received: i64,
    pub net_position: i64,
    pub opening_physical_cash: i64,
    pub cash_received: i64,
    pub cash_expenses: i64,
    pub cash_transferred_to_bank: i64,
    pub cash_transferred_to_mobile_money: i64,
    pub other_verified_cash_outflows: i64,
    pub other_cash_outflows: Vec<DailyCashOutflowBreakdown>,
    pub physical_closing_cash: i64,
    pub total_inflow: i64,
    pub total_outflow: i64,
    pub ending_cash: i64,
    pub loan_taken: i64,
    pub loan_paid: i64,
    pub bank: i64,
    pub momo: i64,
    pub opening_customer_outstanding: i64,
    pub closing_customer_outstanding: i64,
    pub calculated_physical_cash: i64,
    pub cash_reconciliation_difference: i64,
    pub loan_received: i64,
    pub loan_repayments: i64,
    pub other_receipts: i64,
    pub sale_count: i64,
    pub sales_total: i64,
    pub sales_by_payment_method: Vec<DailyAmountBreakdown>,
    pub credit_sales: i64,
    pub customer_repayments: i64,
    pub expenses_total: i64,
    pub expenses_by_category: Vec<DailyAmountBreakdown>,
    pub supplier_payments: i64,
    pub purchases_received: i64,
    pub cost_of_goods_sold: i64,
    pub gross_profit: i64,
    pub operating_result: i64,
    pub sale_line_count: i64,
    pub customer_outstanding: i64,
    pub supplier_outstanding: i64,
    pub stock_adjustment_counts: Vec<DailyCountBreakdown>,
}

fn report_amount(
    conn: &Connection,
    sql: &str,
    business_id: &str,
    report_date: &str,
) -> Result<i64, DbError> {
    Ok(conn.query_row(sql, params![business_id, report_date], |row| row.get(0))?)
}

fn report_business_amount(conn: &Connection, sql: &str, business_id: &str) -> Result<i64, DbError> {
    Ok(conn.query_row(sql, params![business_id], |row| row.get(0))?)
}

pub fn daily_business_report(
    conn: &Connection,
    business_id: &str,
    report_date: &str,
) -> Result<DailyBusinessReport, DbError> {
    validate_expense_date(report_date)?;
    let business_name: Option<String> = conn
        .query_row(
            "SELECT name FROM businesses WHERE id = ?1",
            params![business_id],
            |row| row.get(0),
        )
        .optional()?;
    let business_name = business_name
        .ok_or_else(|| DbError::InvalidInput("business does not exist".to_string()))?;

    let opening_physical_cash: i64 = conn.query_row(
        "SELECT COALESCE(SUM(
            o.opening_amount + COALESCE((
            SELECT SUM(CASE WHEN m.direction = 'in' THEN m.amount ELSE -m.amount END)
            FROM cash_movements m
            WHERE m.business_id = o.business_id AND m.account_id = o.account_id
              AND m.recorded_at >= o.recorded_at
              AND date(m.occurred_at, 'localtime') < ?2
        ), 0)
        ), 0)
         FROM cash_account_openings o
         JOIN cash_accounts a ON a.id = o.account_id AND a.business_id = o.business_id
         WHERE o.business_id = ?1 AND a.account_type = 'cash_drawer'",
        params![business_id, report_date],
        |row| row.get(0),
    )?;
    let cash_movement_sum = |condition: &str| -> Result<i64, DbError> {
        let sql = format!(
            "SELECT COALESCE(SUM(m.amount), 0)
             FROM cash_movements m
             JOIN cash_accounts a ON a.id = m.account_id AND a.business_id = m.business_id
             WHERE m.business_id = ?1 AND a.account_type = 'cash_drawer'
               AND date(m.occurred_at, 'localtime') = ?2 AND {condition}"
        );
        Ok(conn.query_row(&sql, params![business_id, report_date], |row| row.get(0))?)
    };
    let cash_received =
        cash_movement_sum("m.direction = 'in' AND m.movement_type != 'transfer_in'")?;
    let cash_expenses =
        cash_movement_sum("m.direction = 'out' AND m.movement_type = 'expense_payment'")?;
    let cash_transferred_to_bank = conn.query_row(
        "SELECT COALESCE(SUM(m.amount), 0)
         FROM cash_movements m
         JOIN cash_accounts from_account ON from_account.id = m.account_id
             AND from_account.business_id = m.business_id
         JOIN cash_transfers t ON t.id = m.transfer_id AND t.business_id = m.business_id
         JOIN cash_accounts to_account ON to_account.id = t.to_account_id
             AND to_account.business_id = t.business_id
         WHERE m.business_id = ?1 AND from_account.account_type = 'cash_drawer'
           AND to_account.account_type = 'bank' AND m.movement_type = 'transfer_out'
           AND date(m.occurred_at, 'localtime') = ?2",
        params![business_id, report_date],
        |row| row.get(0),
    )?;
    let cash_transferred_to_mobile_money = conn.query_row(
        "SELECT COALESCE(SUM(m.amount), 0)
         FROM cash_movements m
         JOIN cash_accounts from_account ON from_account.id = m.account_id
             AND from_account.business_id = m.business_id
         JOIN cash_transfers t ON t.id = m.transfer_id AND t.business_id = m.business_id
         JOIN cash_accounts to_account ON to_account.id = t.to_account_id
             AND to_account.business_id = t.business_id
         WHERE m.business_id = ?1 AND from_account.account_type = 'cash_drawer'
           AND to_account.account_type IN ('mtn_wallet', 'airtel_wallet')
           AND m.movement_type = 'transfer_out'
           AND date(m.occurred_at, 'localtime') = ?2",
        params![business_id, report_date],
        |row| row.get(0),
    )?;
    let other_verified_cash_outflows = cash_movement_sum(
        "m.direction = 'out' AND m.movement_type NOT IN ('expense_payment', 'transfer_out')",
    )?;
    let mut outflow_stmt = conn.prepare(
        "SELECT
            CASE m.movement_type
                WHEN 'supplier_payment' THEN 'Supplier payment'
                WHEN 'loan_repayment' THEN 'Loan repayment'
                WHEN 'other_payment' THEN 'Other payment'
                ELSE 'Other cash outflow'
            END,
            CASE
                WHEN m.movement_type = 'supplier_payment'
                    THEN 'Invoice ' || COALESCE(p.supplier_invoice_number, 'unknown')
                WHEN m.description IS NOT NULL AND trim(m.description) != ''
                    THEN trim(m.description)
                WHEN m.movement_type = 'loan_repayment' THEN 'Loan repayment'
                WHEN m.movement_type = 'other_payment' THEN 'Other payment'
                ELSE 'Verified cash outflow'
            END,
            m.amount
         FROM cash_movements m
         JOIN cash_accounts a ON a.id = m.account_id AND a.business_id = m.business_id
         LEFT JOIN purchase_payments pp
            ON pp.id = m.source_id AND pp.business_id = m.business_id
         LEFT JOIN purchases p
            ON p.id = pp.purchase_id AND p.business_id = m.business_id
         WHERE m.business_id = ?1 AND a.account_type = 'cash_drawer'
           AND date(m.occurred_at, 'localtime') = ?2
           AND m.direction = 'out'
           AND m.movement_type NOT IN ('expense_payment', 'transfer_out')
         ORDER BY m.occurred_at, m.id",
    )?;
    let other_cash_outflows = outflow_stmt
        .query_map(params![business_id, report_date], |row| {
            Ok(DailyCashOutflowBreakdown {
                category: row.get(0)?,
                description: row.get(1)?,
                amount: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let physical_closing_cash: i64 = conn.query_row(
        "SELECT COALESCE(SUM(
            o.opening_amount + COALESCE((
            SELECT SUM(CASE WHEN m.direction = 'in' THEN m.amount ELSE -m.amount END)
            FROM cash_movements m
            WHERE m.business_id = o.business_id AND m.account_id = o.account_id
              AND m.recorded_at >= o.recorded_at
              AND date(m.occurred_at, 'localtime') <= ?2
        ), 0)
        ), 0)
         FROM cash_account_openings o
         JOIN cash_accounts a ON a.id = o.account_id AND a.business_id = o.business_id
         WHERE o.business_id = ?1 AND a.account_type = 'cash_drawer'",
        params![business_id, report_date],
        |row| row.get(0),
    )?;
    let loan_received = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM cash_movements
         WHERE business_id = ?1 AND movement_type = 'loan_issued'
           AND date(occurred_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let loan_repayments = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM cash_movements
         WHERE business_id = ?1 AND movement_type = 'loan_repayment'
           AND date(occurred_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let other_receipts = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM cash_movements
         WHERE business_id = ?1 AND movement_type = 'other_receipt'
           AND date(occurred_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let money_received = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM cash_movements
         WHERE business_id = ?1 AND direction = 'in'
           AND movement_type != 'transfer_in'
           AND date(occurred_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;

    let sale_count = report_amount(
        conn,
        "SELECT COUNT(*) FROM sales
         WHERE business_id = ?1 AND status = 'completed' AND date(created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let sales_total = report_amount(
        conn,
        "SELECT COALESCE(SUM(total), 0) FROM sales
         WHERE business_id = ?1 AND status = 'completed' AND date(created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let mut payment_stmt = conn.prepare(
        "SELECT sp.payment_method, SUM(sp.amount)
         FROM sale_payments sp
         JOIN sales s ON s.id = sp.sale_id AND s.business_id = ?1
         WHERE s.status = 'completed' AND date(sp.created_at, 'localtime') = ?2
         GROUP BY sp.payment_method ORDER BY sp.payment_method",
    )?;
    let sales_by_payment_method = payment_stmt
        .query_map(params![business_id, report_date], |row| {
            Ok(DailyAmountBreakdown {
                category: row.get(0)?,
                amount: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let credit_sales = report_amount(
        conn,
        "SELECT COALESCE(SUM(
            d.original_amount - COALESCE((
                SELECT SUM(sp.amount) FROM sale_payments sp
                WHERE sp.sale_id = d.sale_id
            ), 0)
         ), 0)
         FROM customer_debts d
         JOIN sales s ON s.id = d.sale_id AND s.business_id = d.business_id
         WHERE d.business_id = ?1 AND s.status = 'completed'
           AND date(s.created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let customer_repayments = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM customer_debt_payments
         WHERE business_id = ?1 AND date(created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let expenses_total = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM expenses
         WHERE business_id = ?1 AND expense_date = ?2",
        business_id,
        report_date,
    )?;
    let total_inflow = sales_total;
    let total_outflow = expenses_total;
    let net_position = total_inflow.checked_sub(total_outflow).ok_or_else(|| {
        DbError::InvalidInput("daily net position exceeds the supported range".to_string())
    })?;
    let mut expense_stmt = conn.prepare(
        "SELECT category, SUM(amount) FROM expenses
         WHERE business_id = ?1 AND expense_date = ?2
         GROUP BY category ORDER BY category",
    )?;
    let expenses_by_category = expense_stmt
        .query_map(params![business_id, report_date], |row| {
            Ok(DailyAmountBreakdown {
                category: row.get(0)?,
                amount: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let supplier_payments = report_amount(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM purchase_payments
         WHERE business_id = ?1 AND date(created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let purchases_received = report_amount(
        conn,
        "SELECT COALESCE(SUM(pri.total_cost), 0)
         FROM purchase_receipt_items pri
         JOIN purchase_receipts pr
           ON pr.id = pri.receipt_id AND pr.business_id = pri.business_id
         WHERE pr.business_id = ?1 AND date(pr.created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let cost_of_goods_sold = report_amount(
        conn,
        "SELECT COALESCE(SUM(si.total_cost), 0)
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id AND s.business_id = si.business_id
         WHERE s.business_id = ?1 AND s.status = 'completed'
           AND date(s.created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let gross_profit = sales_total.checked_sub(cost_of_goods_sold).ok_or_else(|| {
        DbError::InvalidInput("gross profit exceeds the supported range".to_string())
    })?;
    let operating_result = gross_profit.checked_sub(expenses_total).ok_or_else(|| {
        DbError::InvalidInput("operating result exceeds the supported range".to_string())
    })?;
    let sale_line_count = report_amount(
        conn,
        "SELECT COUNT(*) FROM sale_items si
         JOIN sales s ON s.id = si.sale_id AND s.business_id = si.business_id
         WHERE s.business_id = ?1 AND s.status = 'completed'
           AND date(s.created_at, 'localtime') = ?2",
        business_id,
        report_date,
    )?;
    let customer_outstanding = report_business_amount(
        conn,
        "SELECT COALESCE(SUM(
            d.original_amount
            - COALESCE((SELECT SUM(sp.amount) FROM sale_payments sp
                        WHERE sp.sale_id = d.sale_id), 0)
            - COALESCE((SELECT SUM(dp.amount) FROM customer_debt_payments dp
                        WHERE dp.business_id = d.business_id AND dp.debt_id = d.id), 0)
         ), 0)
         FROM customer_debts d WHERE d.business_id = ?1",
        business_id,
    )?;
    let opening_customer_outstanding: i64 = conn.query_row(
        "SELECT COALESCE(SUM(
            d.original_amount
            - COALESCE((SELECT SUM(sp.amount) FROM sale_payments sp
                        WHERE sp.sale_id = d.sale_id
                          AND date(sp.created_at, 'localtime') < ?2), 0)
            - COALESCE((SELECT SUM(dp.amount) FROM customer_debt_payments dp
                        WHERE dp.business_id = d.business_id AND dp.debt_id = d.id
                          AND date(dp.created_at, 'localtime') < ?2), 0)
         ), 0)
         FROM customer_debts d
         JOIN sales s ON s.id = d.sale_id AND s.business_id = d.business_id
         WHERE d.business_id = ?1 AND date(s.created_at, 'localtime') < ?2",
        params![business_id, report_date],
        |row| row.get(0),
    )?;
    let loan_taken = credit_sales;
    let loan_paid = customer_repayments;
    let ending_cash = opening_physical_cash
        .checked_add(net_position)
        .ok_or_else(|| {
            DbError::InvalidInput("ending cash exceeds the supported range".to_string())
        })?;
    let account_inflow = |account_type: &str| -> Result<i64, DbError> {
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(m.amount), 0)
             FROM cash_movements m
             JOIN cash_accounts a ON a.id = m.account_id AND a.business_id = m.business_id
             WHERE m.business_id = ?1 AND a.account_type = ?2
               AND m.direction = 'in'
               AND date(m.occurred_at, 'localtime') = ?3",
            params![business_id, account_type, report_date],
            |row| row.get(0),
        )?)
    };
    let bank = account_inflow("bank")?;
    let momo = account_inflow("mtn_wallet")?
        .checked_add(account_inflow("airtel_wallet")?)
        .ok_or_else(|| {
            DbError::InvalidInput("MOMO inflow exceeds the supported range".to_string())
        })?;
    let calculated_physical_cash = ending_cash
        .checked_sub(loan_taken)
        .and_then(|value| value.checked_add(loan_paid))
        .and_then(|value| value.checked_sub(bank))
        .and_then(|value| value.checked_sub(momo))
        .and_then(|value| value.checked_sub(supplier_payments))
        .and_then(|value| value.checked_sub(other_verified_cash_outflows))
        .ok_or_else(|| {
            DbError::InvalidInput(
                "calculated physical cash exceeds the supported range".to_string(),
            )
        })?;
    let cash_reconciliation_difference = calculated_physical_cash
        .checked_sub(physical_closing_cash)
        .ok_or_else(|| {
            DbError::InvalidInput(
                "cash reconciliation difference exceeds the supported range".to_string(),
            )
        })?;
    let supplier_outstanding = report_business_amount(
        conn,
        "SELECT COALESCE(SUM(
            p.subtotal - COALESCE((SELECT SUM(pp.amount) FROM purchase_payments pp
                                   WHERE pp.business_id = p.business_id
                                     AND pp.purchase_id = p.id), 0)
         ), 0)
         FROM purchases p WHERE p.business_id = ?1",
        business_id,
    )?;
    let mut adjustment_stmt = conn.prepare(
        "SELECT movement_type, COUNT(*)
         FROM stock_movements
        WHERE business_id = ?1 AND date(created_at, 'localtime') = ?2
           AND movement_type IN (
             'customer_return', 'supplier_return', 'damaged_stock',
             'stock_correction', 'stock_transfer', 'manual_adjustment'
           )
         GROUP BY movement_type ORDER BY movement_type",
    )?;
    let stock_adjustment_counts = adjustment_stmt
        .query_map(params![business_id, report_date], |row| {
            Ok(DailyCountBreakdown {
                movement_type: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DailyBusinessReport {
        business_name,
        report_date: report_date.to_string(),
        money_received,
        net_position,
        opening_physical_cash,
        cash_received,
        cash_expenses,
        cash_transferred_to_bank,
        cash_transferred_to_mobile_money,
        other_verified_cash_outflows,
        other_cash_outflows,
        physical_closing_cash,
        total_inflow,
        total_outflow,
        ending_cash,
        loan_taken,
        loan_paid,
        bank,
        momo,
        opening_customer_outstanding,
        closing_customer_outstanding: customer_outstanding,
        calculated_physical_cash,
        cash_reconciliation_difference,
        loan_received,
        loan_repayments,
        other_receipts,
        sale_count,
        sales_total,
        sales_by_payment_method,
        credit_sales,
        customer_repayments,
        expenses_total,
        expenses_by_category,
        supplier_payments,
        purchases_received,
        cost_of_goods_sold,
        gross_profit,
        operating_result,
        sale_line_count,
        customer_outstanding,
        supplier_outstanding,
        stock_adjustment_counts,
    })
}

pub fn list_customer_debts(
    conn: &Connection,
    business_id: &str,
) -> Result<Vec<CustomerDebt>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT d.id FROM customer_debts d
         WHERE d.business_id = ?1
         ORDER BY d.created_at DESC, d.id",
    )?;
    let ids = stmt
        .query_map(params![business_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    ids.iter()
        .map(|debt_id| load_customer_debt(conn, business_id, debt_id))
        .collect()
}

pub fn record_customer_debt_payment_as_user(
    conn: &Connection,
    business_id: &str,
    debt_id: &str,
    payment_method: &str,
    amount: i64,
    account_id: &str,
    actor: &AuthUser,
) -> Result<CustomerDebtPayment, DbError> {
    if actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "you are not signed in to this business".to_string(),
        ));
    }
    if !valid_sale_payment_method(payment_method) {
        return Err(DbError::InvalidInput(
            "select a supported payment method".to_string(),
        ));
    }
    if amount <= 0 {
        return Err(DbError::InvalidInput(
            "repayment amount must be greater than zero".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    validate_payment_account(&tx, business_id, account_id, payment_method)?;
    let debt = load_customer_debt(&tx, business_id, debt_id)?;
    if amount > debt.balance_due {
        return Err(DbError::InvalidInput(
            "repayment cannot exceed the outstanding balance".to_string(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO customer_debt_payments
            (id, business_id, debt_id, payment_method, amount, created_at, cash_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            business_id,
            debt_id,
            payment_method,
            amount,
            timestamp,
            account_id
        ],
    )?;
    insert_cash_movement(
        &tx,
        business_id,
        account_id,
        "customer_repayment",
        "in",
        amount,
        "customer_debt_payment",
        &id,
        &actor.id,
        &timestamp,
    )?;
    tx.commit()?;

    Ok(CustomerDebtPayment {
        id,
        debt_id: debt_id.to_string(),
        customer_name: debt.customer_name,
        payment_method: payment_method.to_string(),
        amount,
        balance_due: debt.balance_due - amount,
        created_at: timestamp,
    })
}

#[cfg(test)]
pub fn record_customer_debt_payment(
    conn: &Connection,
    business_id: &str,
    debt_id: &str,
    payment_method: &str,
    amount: i64,
) -> Result<CustomerDebtPayment, DbError> {
    let (account_id, actor) = test_payment_account(conn, business_id, payment_method)?;
    record_customer_debt_payment_as_user(
        conn,
        business_id,
        debt_id,
        payment_method,
        amount,
        &account_id,
        &actor,
    )
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
    pub minimum_selling_price: i64,
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
                COALESCE(ib.quantity_on_hand, 0), u.decimals_ok,
                COALESCE(
                    (SELECT MAX(icl.unit_cost) FROM inventory_cost_layers icl
                     WHERE icl.business_id = p.business_id AND icl.product_id = p.id
                       AND icl.quantity_remaining > 0),
                    p.purchase_price
                )
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
                minimum_selling_price: row.get(8)?,
                quantity_on_hand: row.get(6)?,
                base_unit_allows_decimals: row.get::<_, i64>(7)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PosProductPage { items, total_count })
}

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaleItemInput {
    pub product_id: String,
    pub quantity: i64,
    pub selling_price: Option<i64>,
    pub price_change_reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SaleReceiptItem {
    pub product_name: String,
    pub sku: String,
    pub quantity: i64,
    pub unit_name: String,
    pub listed_unit_price: i64,
    pub unit_price: i64,
    pub line_total: i64,
    pub price_change_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalePaymentInput {
    pub payment_method: String,
    pub amount: i64,
    pub account_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SalePaymentReceipt {
    pub payment_method: String,
    pub amount: i64,
    pub account_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SaleReceipt {
    pub id: String,
    pub transaction_number: String,
    pub payment_method: String,
    pub total: i64,
    pub amount_paid: i64,
    pub balance_due: i64,
    pub customer_name: Option<String>,
    pub created_at: String,
    pub items: Vec<SaleReceiptItem>,
    pub payments: Vec<SalePaymentReceipt>,
}

#[derive(Debug, serde::Serialize)]
pub struct SaleHistoryItem {
    pub product_name: String,
    pub sku: String,
    pub quantity: i64,
    pub unit_name: String,
    pub unit_price: i64,
    pub line_total: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct SaleHistoryPayment {
    pub payment_method: String,
    pub amount: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct SaleHistoryEntry {
    pub id: String,
    pub transaction_number: String,
    pub status: String,
    pub customer_name: Option<String>,
    pub sold_by: Option<String>,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub amount_paid: i64,
    pub balance_due: i64,
    pub item_quantity: i64,
    pub created_at: String,
    pub items: Vec<SaleHistoryItem>,
    pub payments: Vec<SaleHistoryPayment>,
}

#[derive(Debug, serde::Serialize)]
pub struct SaleHistoryPage {
    pub items: Vec<SaleHistoryEntry>,
    pub total_count: i64,
    pub completed_sales_total: i64,
    pub collected_at_sale_total: i64,
}

pub fn list_sales(
    conn: &Connection,
    business_id: &str,
    search: Option<&str>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<SaleHistoryPage, DbError> {
    if offset < 0 || !(1..=100).contains(&limit) {
        return Err(DbError::InvalidInput(
            "sales pagination must use a non-negative offset and a limit from 1 to 100".to_string(),
        ));
    }
    for date in [from_date, to_date].into_iter().flatten() {
        let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| DbError::InvalidInput("enter a valid sales date".to_string()))?;
        if parsed.format("%Y-%m-%d").to_string() != date {
            return Err(DbError::InvalidInput(
                "enter a valid sales date".to_string(),
            ));
        }
    }
    if matches!((from_date, to_date), (Some(from), Some(to)) if from > to) {
        return Err(DbError::InvalidInput(
            "sales start date cannot be after its end date".to_string(),
        ));
    }

    let pattern = search
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            format!(
                "%{}%",
                value
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        });
    let filter = "s.business_id = ?1
        AND (?2 IS NULL OR
             s.transaction_number LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
             COALESCE(c.name, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
             EXISTS (
                 SELECT 1 FROM sale_items si
                 WHERE si.sale_id = s.id AND si.business_id = s.business_id
                   AND (si.product_name LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                        OR si.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE)
             ))
        AND (?3 IS NULL OR substr(s.created_at, 1, 10) >= ?3)
        AND (?4 IS NULL OR substr(s.created_at, 1, 10) <= ?4)";
    let total_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM sales s
             LEFT JOIN customers c ON c.id = s.customer_id AND c.business_id = s.business_id
             WHERE {filter}"
        ),
        params![business_id, pattern, from_date, to_date],
        |row| row.get(0),
    )?;
    let (completed_sales_total, collected_at_sale_total): (i64, i64) = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(CASE WHEN s.status = 'completed' THEN s.total ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN s.status = 'completed' THEN
                        (SELECT COALESCE(SUM(sp.amount), 0) FROM sale_payments sp
                         WHERE sp.sale_id = s.id)
                        ELSE 0 END), 0)
             FROM sales s
             LEFT JOIN customers c ON c.id = s.customer_id AND c.business_id = s.business_id
             WHERE {filter}"
        ),
        params![business_id, pattern, from_date, to_date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut stmt = conn.prepare(&format!(
        "SELECT s.id, s.transaction_number, s.status, c.name, u.full_name,
                s.subtotal, s.discount, s.total,
                COALESCE((SELECT SUM(sp.amount) FROM sale_payments sp
                          WHERE sp.sale_id = s.id), 0), s.created_at
         FROM sales s
         LEFT JOIN customers c ON c.id = s.customer_id AND c.business_id = s.business_id
         LEFT JOIN users u ON u.id = s.sold_by AND u.business_id = s.business_id
         WHERE {filter}
         ORDER BY s.created_at DESC, s.id DESC
         LIMIT ?5 OFFSET ?6"
    ))?;
    let rows = stmt
        .query_map(
            params![business_id, pattern, from_date, to_date, limit, offset],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let mut items = Vec::with_capacity(rows.len());
    let mut item_stmt = conn.prepare(
        "SELECT product_name, sku, quantity, unit_name, unit_price, line_total
         FROM sale_items WHERE sale_id = ?1 ORDER BY id",
    )?;
    let mut payment_stmt = conn.prepare(
        "SELECT payment_method, amount
         FROM sale_payments WHERE sale_id = ?1 ORDER BY created_at, id",
    )?;
    let mut quantity_stmt =
        conn.prepare("SELECT COALESCE(SUM(quantity), 0) FROM sale_items WHERE sale_id = ?1")?;
    for (
        id,
        transaction_number,
        status,
        customer_name,
        sold_by,
        subtotal,
        discount,
        total,
        amount_paid,
        created_at,
    ) in rows
    {
        let sale_items = item_stmt
            .query_map(params![id], |row| {
                Ok(SaleHistoryItem {
                    product_name: row.get(0)?,
                    sku: row.get(1)?,
                    quantity: row.get(2)?,
                    unit_name: row.get(3)?,
                    unit_price: row.get(4)?,
                    line_total: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let payments = payment_stmt
            .query_map(params![id], |row| {
                Ok(SaleHistoryPayment {
                    payment_method: row.get(0)?,
                    amount: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let balance_due = total
            .checked_sub(amount_paid)
            .filter(|balance| *balance >= 0)
            .ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "recorded sale payments exceed the total for {transaction_number}"
                ))
            })?;
        let item_quantity: i64 = quantity_stmt.query_row(params![id], |row| row.get(0))?;
        items.push(SaleHistoryEntry {
            id,
            transaction_number,
            status,
            customer_name,
            sold_by,
            subtotal,
            discount,
            total,
            amount_paid,
            balance_due,
            item_quantity,
            created_at,
            items: sale_items,
            payments,
        });
    }

    Ok(SaleHistoryPage {
        items,
        total_count,
        completed_sales_total,
        collected_at_sale_total,
    })
}

struct SaleLine {
    id: String,
    product_id: String,
    product_name: String,
    sku: String,
    quantity: i64,
    unit_name: String,
    listed_unit_price: i64,
    unit_price: i64,
    line_total: i64,
    price_change_reason: Option<String>,
    unit_cost: i64,
    total_cost: i64,
    previous_quantity: i64,
    new_quantity: i64,
}

#[cfg(test)]
pub fn checkout_sale(
    conn: &Connection,
    business_id: &str,
    items: &[SaleItemInput],
    payment_method: &str,
) -> Result<SaleReceipt, DbError> {
    if !valid_sale_payment_method(payment_method) {
        return Err(DbError::InvalidInput(
            "select a supported payment method".to_string(),
        ));
    }
    let (account_id, actor) = test_payment_account(conn, business_id, payment_method)?;
    checkout_sale_core(
        conn,
        business_id,
        items,
        None,
        &[],
        Some(payment_method),
        Some(&account_id),
        &actor,
    )
}

pub fn checkout_sale_with_payments_as_user(
    conn: &Connection,
    business_id: &str,
    items: &[SaleItemInput],
    customer_id: Option<&str>,
    payments: &[SalePaymentInput],
    actor: &AuthUser,
) -> Result<SaleReceipt, DbError> {
    checkout_sale_core(
        conn,
        business_id,
        items,
        customer_id,
        payments,
        None,
        None,
        actor,
    )
}

#[cfg(test)]
pub fn checkout_sale_with_payments(
    conn: &Connection,
    business_id: &str,
    items: &[SaleItemInput],
    customer_id: Option<&str>,
    payments: &[SalePaymentInput],
) -> Result<SaleReceipt, DbError> {
    let (_, actor) = test_payment_account(conn, business_id, "cash")?;
    let payments_with_accounts = payments
        .iter()
        .map(|payment| {
            let (account_id, _) = test_payment_account(conn, business_id, &payment.payment_method)?;
            Ok(SalePaymentInput {
                payment_method: payment.payment_method.clone(),
                amount: payment.amount,
                account_id,
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    checkout_sale_with_payments_as_user(
        conn,
        business_id,
        items,
        customer_id,
        &payments_with_accounts,
        &actor,
    )
}

fn checkout_sale_core(
    conn: &Connection,
    business_id: &str,
    items: &[SaleItemInput],
    customer_id: Option<&str>,
    payment_inputs: &[SalePaymentInput],
    default_payment_method: Option<&str>,
    default_cash_account_id: Option<&str>,
    actor: &AuthUser,
) -> Result<SaleReceipt, DbError> {
    if actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "you are not signed in to this business".to_string(),
        ));
    }
    if items.is_empty() || items.len() > 100 {
        return Err(DbError::InvalidInput(
            "a sale must contain between 1 and 100 products".to_string(),
        ));
    }
    if payment_inputs.len() > 10 {
        return Err(DbError::InvalidInput(
            "a sale can contain no more than 10 payment entries".to_string(),
        ));
    }

    let mut seen_products = HashSet::with_capacity(items.len());
    for item in items {
        if item.quantity <= 0 {
            return Err(DbError::InvalidInput(
                "sale quantity must be greater than zero".to_string(),
            ));
        }
        if !seen_products.insert(item.product_id.as_str()) {
            return Err(DbError::InvalidInput(
                "a product can only appear once in a sale".to_string(),
            ));
        }
    }

    let sale_id = uuid::Uuid::new_v4().to_string();
    let transaction_number = format!(
        "SALE-{}-{}",
        chrono::Utc::now().format("%Y%m%d"),
        uuid::Uuid::new_v4().simple()
    );
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let mut lines = Vec::with_capacity(items.len());
    let mut total = 0_i64;

    for item in items {
        let product: Option<(String, String, String, i64, i64, i64, i64)> = tx
            .query_row(
                "SELECT p.name, p.sku, u.name, u.decimals_ok, p.selling_price,
                        COALESCE(ib.quantity_on_hand, 0),
                        p.is_active
                 FROM products p
                 JOIN units u ON u.id = p.base_unit_id
                 LEFT JOIN inventory_balances ib
                   ON ib.product_id = p.id AND ib.business_id = p.business_id
                 WHERE p.id = ?1 AND p.business_id = ?2",
                params![item.product_id, business_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?;
        let (
            product_name,
            sku,
            unit_name,
            base_unit_allows_decimals,
            listed_unit_price,
            previous_quantity,
            is_active,
        ) = product.ok_or_else(|| {
            DbError::InvalidInput("a selected product does not belong to this business".to_string())
        })?;
        if is_active == 0 {
            return Err(DbError::InvalidInput(
                "a selected product is no longer active".to_string(),
            ));
        }
        if base_unit_allows_decimals == 0 && item.quantity % 1000 != 0 {
            return Err(DbError::InvalidInput(
                "this product only allows whole base units".to_string(),
            ));
        }
        if item.selling_price.is_some_and(|price| price < 0) {
            return Err(DbError::InvalidInput(
                "sale price must not be negative".to_string(),
            ));
        }
        let requested_price_change_reason = item
            .price_change_reason
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty());
        if requested_price_change_reason.is_some_and(|reason| reason.chars().count() > 500) {
            return Err(DbError::InvalidInput(
                "price change reason must be 500 characters or fewer".to_string(),
            ));
        }
        let unit_price = item.selling_price.unwrap_or(listed_unit_price);
        let price_change_reason = if unit_price != listed_unit_price {
            let reason = requested_price_change_reason.ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "provide a reason when changing the selling price for {product_name}"
                ))
            })?;
            Some(reason.to_string())
        } else {
            None
        };
        let new_quantity = previous_quantity
            .checked_sub(item.quantity)
            .filter(|quantity| *quantity >= 0)
            .ok_or_else(|| DbError::InvalidInput(format!("not enough stock for {product_name}")))?;
        let line_total = i64::try_from(
            i128::from(item.quantity)
                .checked_mul(i128::from(unit_price))
                .and_then(|amount| amount.checked_add(500))
                .ok_or_else(|| {
                    DbError::InvalidInput("sale total exceeds the supported limit".to_string())
                })?
                / 1000,
        )
        .map_err(|_| DbError::InvalidInput("sale total exceeds the supported limit".to_string()))?;
        total = total.checked_add(line_total).ok_or_else(|| {
            DbError::InvalidInput("sale total exceeds the supported limit".to_string())
        })?;
        let (unit_cost, total_cost) =
            consume_inventory_cost_layers(&tx, business_id, &item.product_id, item.quantity)?;
        let is_below_cost = line_total < total_cost;
        let price_change_reason = if is_below_cost {
            let reason = requested_price_change_reason.ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "BELOW_FIFO_COST:{}: sale price for {} is below the cost of the FIFO stock; enter a reason to authorize this loss",
                    item.product_id, product_name
                ))
            })?;
            Some(reason.to_string())
        } else {
            price_change_reason
        };
        lines.push(SaleLine {
            id: uuid::Uuid::new_v4().to_string(),
            product_id: item.product_id.clone(),
            product_name,
            sku,
            quantity: item.quantity,
            unit_name,
            listed_unit_price,
            unit_price,
            line_total,
            price_change_reason,
            unit_cost,
            total_cost,
            previous_quantity,
            new_quantity,
        });
    }

    let payments = if let Some(method) = default_payment_method {
        vec![SalePaymentReceipt {
            payment_method: method.to_string(),
            amount: total,
            account_id: default_cash_account_id
                .ok_or_else(|| {
                    DbError::InvalidInput("select an opened cash account for the sale".to_string())
                })?
                .to_string(),
        }]
    } else {
        let mut payments = Vec::with_capacity(payment_inputs.len());
        for payment in payment_inputs {
            if !valid_sale_payment_method(&payment.payment_method) {
                return Err(DbError::InvalidInput(
                    "select a supported payment method".to_string(),
                ));
            }
            if payment.amount <= 0 {
                return Err(DbError::InvalidInput(
                    "payment amounts must be greater than zero".to_string(),
                ));
            }
            payments.push(SalePaymentReceipt {
                payment_method: payment.payment_method.clone(),
                amount: payment.amount,
                account_id: payment.account_id.clone(),
            });
        }
        payments
    };
    let amount_paid = payments.iter().try_fold(0_i64, |sum, payment| {
        sum.checked_add(payment.amount).ok_or_else(|| {
            DbError::InvalidInput("sale payment total exceeds the supported limit".to_string())
        })
    })?;
    let balance_due = total
        .checked_sub(amount_paid)
        .filter(|due| *due >= 0)
        .ok_or_else(|| {
            DbError::InvalidInput("payments cannot exceed the sale total".to_string())
        })?;
    for payment in &payments {
        validate_payment_account(
            &tx,
            business_id,
            &payment.account_id,
            &payment.payment_method,
        )?;
    }

    let customer_name = if let Some(customer_id) = customer_id {
        let customer: Option<(String, i64)> = tx
            .query_row(
                "SELECT name, credit_limit FROM customers
                 WHERE id = ?1 AND business_id = ?2 AND is_active = 1",
                params![customer_id, business_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (name, credit_limit) = customer.ok_or_else(|| {
            DbError::InvalidInput("select an active customer from this business".to_string())
        })?;
        if balance_due > 0 {
            let current_balance = customer_outstanding_balance(&tx, business_id, customer_id)?;
            let new_balance = current_balance.checked_add(balance_due).ok_or_else(|| {
                DbError::InvalidInput("customer balance exceeds the supported limit".to_string())
            })?;
            if new_balance > credit_limit {
                return Err(DbError::InvalidInput(
                    "customer credit limit exceeded".to_string(),
                ));
            }
        }
        Some(name)
    } else {
        if balance_due > 0 {
            return Err(DbError::InvalidInput(
                "select a customer to record the unpaid balance as credit".to_string(),
            ));
        }
        None
    };

    tx.execute(
        "INSERT INTO sales
            (id, business_id, transaction_number, status, subtotal, discount, total,
             sold_by, customer_id, created_at)
         VALUES (?1, ?2, ?3, 'completed', ?4, 0, ?4, ?5, ?6, ?7)",
        params![
            sale_id,
            business_id,
            transaction_number,
            total,
            actor.id,
            customer_id,
            timestamp
        ],
    )?;

    let mut receipt_items = Vec::with_capacity(lines.len());
    for line in &lines {
        let updated = tx.execute(
            "UPDATE inventory_balances
             SET quantity_on_hand = ?1, updated_at = ?2
             WHERE product_id = ?3 AND business_id = ?4 AND quantity_on_hand >= ?5",
            params![
                line.new_quantity,
                timestamp,
                line.product_id,
                business_id,
                line.quantity
            ],
        )?;
        if updated != 1 {
            return Err(DbError::InvalidInput(format!(
                "stock changed while completing the sale for {}",
                line.product_name
            )));
        }
        tx.execute(
            "INSERT INTO sale_items
                (id, sale_id, business_id, product_id, product_name, sku, quantity,
                 unit_name, listed_unit_price, unit_price, line_total, price_change_reason,
                 unit_cost, total_cost)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                line.id,
                sale_id,
                business_id,
                line.product_id,
                line.product_name,
                line.sku,
                line.quantity,
                line.unit_name,
                line.listed_unit_price,
                line.unit_price,
                line.line_total,
                line.price_change_reason,
                line.unit_cost,
                line.total_cost
            ],
        )?;
        tx.execute(
            "INSERT INTO stock_movements
                (id, business_id, product_id, movement_type, quantity_delta, unit_cost,
                 total_cost, previous_quantity, new_quantity, reference_type, reference_id,
                 reason, created_at)
             VALUES (?1, ?2, ?3, 'sale', ?4, ?5, ?6, ?7, ?8, 'sale', ?9,
                     'POS sale', ?10)",
            params![
                uuid::Uuid::new_v4().to_string(),
                business_id,
                line.product_id,
                -line.quantity,
                line.unit_cost,
                line.total_cost,
                line.previous_quantity,
                line.new_quantity,
                sale_id,
                timestamp
            ],
        )?;
        receipt_items.push(SaleReceiptItem {
            product_name: line.product_name.clone(),
            sku: line.sku.clone(),
            quantity: line.quantity,
            unit_name: line.unit_name.clone(),
            listed_unit_price: line.listed_unit_price,
            unit_price: line.unit_price,
            line_total: line.line_total,
            price_change_reason: line.price_change_reason.clone(),
        });
    }
    for payment in &payments {
        let payment_id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO sale_payments
                (id, sale_id, payment_method, amount, created_at, cash_account_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                payment_id,
                sale_id,
                payment.payment_method,
                payment.amount,
                timestamp,
                payment.account_id
            ],
        )?;
        insert_cash_movement(
            &tx,
            business_id,
            &payment.account_id,
            "sale_collection",
            "in",
            payment.amount,
            "sale_payment",
            &payment_id,
            &actor.id,
            &timestamp,
        )?;
    }
    if balance_due > 0 {
        tx.execute(
            "INSERT INTO customer_debts
                (id, business_id, customer_id, sale_id, original_amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                business_id,
                customer_id,
                sale_id,
                total,
                timestamp
            ],
        )?;
    }
    tx.commit()?;

    let payment_method = if let Some(payment) = payments.first().filter(|_| payments.len() == 1) {
        payment.payment_method.clone()
    } else if payments.is_empty() {
        "credit".to_string()
    } else {
        "split".to_string()
    };
    Ok(SaleReceipt {
        id: sale_id,
        transaction_number,
        payment_method,
        total,
        amount_paid,
        balance_due,
        customer_name,
        created_at: timestamp,
        items: receipt_items,
        payments,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct PurchaseProduct {
    pub id: String,
    pub name: String,
    pub sku: String,
    pub base_unit_name: String,
    pub purchase_price: i64,
    pub base_unit_allows_decimals: bool,
}

pub fn list_purchase_products(
    conn: &Connection,
    business_id: &str,
) -> Result<Vec<PurchaseProduct>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.sku, u.name, p.purchase_price, u.decimals_ok
         FROM products p
         JOIN units u ON u.id = p.base_unit_id
         WHERE p.business_id = ?1 AND p.is_active = 1
         ORDER BY p.name COLLATE NOCASE, p.id
         LIMIT 500",
    )?;
    let products = stmt
        .query_map(params![business_id], |row| {
            Ok(PurchaseProduct {
                id: row.get(0)?,
                name: row.get(1)?,
                sku: row.get(2)?,
                base_unit_name: row.get(3)?,
                purchase_price: row.get(4)?,
                base_unit_allows_decimals: row.get::<_, i64>(5)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(products)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseItemInput {
    pub product_id: String,
    pub ordered_quantity: i64,
    pub unit_cost: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PurchaseItem {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub sku: String,
    pub unit_name: String,
    pub ordered_quantity: i64,
    pub received_quantity: i64,
    pub unit_cost: i64,
    pub line_total: i64,
    pub base_unit_allows_decimals: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct PurchaseDeliveryItem {
    pub product_name: String,
    pub quantity: i64,
    pub unit_name: String,
    pub total_cost: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PurchaseDelivery {
    pub id: String,
    pub delivery_reference: Option<String>,
    pub notes: Option<String>,
    pub total_cost: i64,
    pub created_at: String,
    pub items: Vec<PurchaseDeliveryItem>,
}

#[derive(Debug, serde::Serialize)]
pub struct PurchasePayment {
    pub id: String,
    pub payment_method: String,
    pub amount: i64,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Purchase {
    pub id: String,
    pub supplier_id: String,
    pub supplier_name: String,
    pub supplier_invoice_number: String,
    pub status: String,
    pub subtotal: i64,
    pub amount_paid: i64,
    pub balance_due: i64,
    pub duplicate_authorized: bool,
    pub duplicate_of_purchase_id: Option<String>,
    pub created_at: String,
    pub items: Vec<PurchaseItem>,
    pub deliveries: Vec<PurchaseDelivery>,
    pub payments: Vec<PurchasePayment>,
}

fn load_purchase(
    conn: &Connection,
    business_id: &str,
    purchase_id: &str,
) -> Result<Purchase, DbError> {
    let mut purchase = conn.query_row(
        "SELECT p.id, p.supplier_id, s.name, p.supplier_invoice_number, p.status,
                p.subtotal, p.duplicate_authorized, p.duplicate_of_purchase_id, p.created_at
         FROM purchases p
         JOIN suppliers s ON s.id = p.supplier_id AND s.business_id = p.business_id
         WHERE p.id = ?1 AND p.business_id = ?2",
        params![purchase_id, business_id],
        |row| {
            Ok(Purchase {
                id: row.get(0)?,
                supplier_id: row.get(1)?,
                supplier_name: row.get(2)?,
                supplier_invoice_number: row.get(3)?,
                status: row.get(4)?,
                subtotal: row.get(5)?,
                amount_paid: 0,
                balance_due: row.get(5)?,
                duplicate_authorized: row.get::<_, i64>(6)? != 0,
                duplicate_of_purchase_id: row.get(7)?,
                created_at: row.get(8)?,
                items: Vec::new(),
                deliveries: Vec::new(),
                payments: Vec::new(),
            })
        },
    )?;
    let mut payment_stmt = conn.prepare(
        "SELECT id, payment_method, amount, created_at
         FROM purchase_payments
         WHERE business_id = ?1 AND purchase_id = ?2
         ORDER BY created_at DESC, id",
    )?;
    purchase.payments = payment_stmt
        .query_map(params![business_id, purchase_id], |row| {
            Ok(PurchasePayment {
                id: row.get(0)?,
                payment_method: row.get(1)?,
                amount: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    purchase.amount_paid = purchase.payments.iter().try_fold(0_i64, |total, payment| {
        total.checked_add(payment.amount).ok_or_else(|| {
            DbError::InvalidInput("supplier payment total exceeds the supported limit".into())
        })
    })?;
    purchase.balance_due = purchase
        .subtotal
        .checked_sub(purchase.amount_paid)
        .ok_or_else(|| {
            DbError::InvalidInput("supplier payments exceed the invoice total".into())
        })?;
    let mut stmt = conn.prepare(
        "SELECT pi.id, pi.product_id, pi.product_name, pi.sku, pi.unit_name,
                pi.ordered_quantity, pi.received_quantity, pi.unit_cost, pi.line_total,
                u.decimals_ok
         FROM purchase_items pi
         JOIN products p ON p.id = pi.product_id AND p.business_id = pi.business_id
         JOIN units u ON u.id = p.base_unit_id
         WHERE pi.business_id = ?1 AND pi.purchase_id = ?2
         ORDER BY pi.product_name COLLATE NOCASE, pi.id",
    )?;
    purchase.items = stmt
        .query_map(params![business_id, purchase_id], |row| {
            Ok(PurchaseItem {
                id: row.get(0)?,
                product_id: row.get(1)?,
                product_name: row.get(2)?,
                sku: row.get(3)?,
                unit_name: row.get(4)?,
                ordered_quantity: row.get(5)?,
                received_quantity: row.get(6)?,
                unit_cost: row.get(7)?,
                line_total: row.get(8)?,
                base_unit_allows_decimals: row.get::<_, i64>(9)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut receipt_stmt = conn.prepare(
        "SELECT id, delivery_reference, notes, created_at
         FROM purchase_receipts
         WHERE business_id = ?1 AND purchase_id = ?2
         ORDER BY created_at DESC, id",
    )?;
    let receipts = receipt_stmt
        .query_map(params![business_id, purchase_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (receipt_id, delivery_reference, notes, created_at) in receipts {
        let mut item_stmt = conn.prepare(
            "SELECT pi.product_name, pri.quantity, pi.unit_name, pri.total_cost
             FROM purchase_receipt_items pri
             JOIN purchase_items pi
               ON pi.id = pri.purchase_item_id AND pi.business_id = pri.business_id
             WHERE pri.business_id = ?1 AND pri.receipt_id = ?2
             ORDER BY pi.product_name COLLATE NOCASE",
        )?;
        let items = item_stmt
            .query_map(params![business_id, receipt_id], |row| {
                Ok(PurchaseDeliveryItem {
                    product_name: row.get(0)?,
                    quantity: row.get(1)?,
                    unit_name: row.get(2)?,
                    total_cost: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let total_cost = items.iter().try_fold(0_i64, |total, item| {
            total.checked_add(item.total_cost).ok_or_else(|| {
                DbError::InvalidInput("delivery total exceeds the supported limit".into())
            })
        })?;
        purchase.deliveries.push(PurchaseDelivery {
            id: receipt_id,
            delivery_reference,
            notes,
            total_cost,
            created_at,
            items,
        });
    }
    Ok(purchase)
}

pub fn list_purchases(conn: &Connection, business_id: &str) -> Result<Vec<Purchase>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM purchases
         WHERE business_id = ?1
           AND (
               status != 'received'
               OR COALESCE(
                   (SELECT SUM(pp.amount) FROM purchase_payments pp
                    WHERE pp.business_id = purchases.business_id
                      AND pp.purchase_id = purchases.id),
                   0
               ) < subtotal
           )
         ORDER BY created_at DESC, id LIMIT 200",
    )?;
    let purchase_ids = stmt
        .query_map(params![business_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    purchase_ids
        .iter()
        .map(|purchase_id| load_purchase(conn, business_id, purchase_id))
        .collect()
}

#[derive(Debug, serde::Serialize)]
pub struct PurchaseHistoryPage {
    pub items: Vec<Purchase>,
    pub total_count: i64,
    pub purchase_total: i64,
    pub paid_total: i64,
    pub balance_due_total: i64,
}

pub fn list_purchase_history(
    conn: &Connection,
    business_id: &str,
    search: Option<&str>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<PurchaseHistoryPage, DbError> {
    if offset < 0 || !(1..=100).contains(&limit) {
        return Err(DbError::InvalidInput(
            "purchase history pagination must use a non-negative offset and a limit from 1 to 100"
                .to_string(),
        ));
    }
    for date in [from_date, to_date].into_iter().flatten() {
        let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| DbError::InvalidInput("enter a valid purchase date".to_string()))?;
        if parsed.format("%Y-%m-%d").to_string() != date {
            return Err(DbError::InvalidInput(
                "enter a valid purchase date".to_string(),
            ));
        }
    }
    if matches!((from_date, to_date), (Some(from), Some(to)) if from > to) {
        return Err(DbError::InvalidInput(
            "purchase start date cannot be after its end date".to_string(),
        ));
    }
    let pattern = search
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            format!(
                "%{}%",
                value
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        });
    let filter = "p.business_id = ?1 AND p.status = 'received'
        AND COALESCE(
            (SELECT SUM(pp.amount) FROM purchase_payments pp
             WHERE pp.business_id = p.business_id AND pp.purchase_id = p.id),
            0
        ) >= p.subtotal
        AND (?2 IS NULL OR
             s.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
             p.supplier_invoice_number LIKE ?2 ESCAPE '\\' COLLATE NOCASE OR
             EXISTS (SELECT 1 FROM purchase_items pi
                     WHERE pi.purchase_id = p.id AND pi.business_id = p.business_id
                       AND (pi.product_name LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                            OR pi.sku LIKE ?2 ESCAPE '\\' COLLATE NOCASE)) OR
             EXISTS (SELECT 1 FROM purchase_receipts pr
                     WHERE pr.purchase_id = p.id AND pr.business_id = p.business_id
                       AND COALESCE(pr.delivery_reference, '') LIKE ?2 ESCAPE '\\' COLLATE NOCASE))
        AND (?3 IS NULL OR substr(p.created_at, 1, 10) >= ?3)
        AND (?4 IS NULL OR substr(p.created_at, 1, 10) <= ?4)";
    let total_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM purchases p JOIN suppliers s ON s.id = p.supplier_id AND s.business_id = p.business_id WHERE {filter}"),
        params![business_id, pattern, from_date, to_date],
        |row| row.get(0),
    )?;
    let totals: (i64, i64) = conn.query_row(
        &format!("SELECT COALESCE(SUM(p.subtotal), 0), COALESCE(SUM((SELECT COALESCE(SUM(pp.amount), 0) FROM purchase_payments pp WHERE pp.purchase_id = p.id)), 0)
                  FROM purchases p JOIN suppliers s ON s.id = p.supplier_id AND s.business_id = p.business_id WHERE {filter}"),
        params![business_id, pattern, from_date, to_date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT p.id FROM purchases p
         JOIN suppliers s ON s.id = p.supplier_id AND s.business_id = p.business_id
         WHERE {filter}
         ORDER BY p.created_at DESC, p.id DESC LIMIT ?5 OFFSET ?6"
    ))?;
    let ids = stmt
        .query_map(
            params![business_id, pattern, from_date, to_date, limit, offset],
            |row| row.get::<_, String>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let items = ids
        .iter()
        .map(|id| load_purchase(conn, business_id, id))
        .collect::<Result<Vec<_>, _>>()?;
    let paid_total = totals.1;
    Ok(PurchaseHistoryPage {
        items,
        total_count,
        purchase_total: totals.0,
        paid_total,
        balance_due_total: totals.0.checked_sub(paid_total).ok_or_else(|| {
            DbError::InvalidInput("supplier payments exceed purchase totals".to_string())
        })?,
    })
}

pub fn record_purchase_payment_as_user(
    conn: &Connection,
    business_id: &str,
    purchase_id: &str,
    amount: i64,
    payment_method: &str,
    account_id: &str,
    actor: &AuthUser,
) -> Result<Purchase, DbError> {
    if actor.business_id != business_id {
        return Err(DbError::InvalidInput(
            "you are not signed in to this business".to_string(),
        ));
    }
    if amount <= 0 {
        return Err(DbError::InvalidInput(
            "payment amount must be greater than zero".to_string(),
        ));
    }
    if !matches!(
        payment_method,
        "cash" | "mtn_mobile_money" | "airtel_money" | "bank_transfer"
    ) {
        return Err(DbError::InvalidInput(
            "select a supported payment method".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let available_balance = validate_payment_account(&tx, business_id, account_id, payment_method)?;
    if amount > available_balance {
        return Err(DbError::InvalidInput(format!(
            "payment exceeds the available balance of {available_balance} in the selected account"
        )));
    }
    let subtotal = tx
        .query_row(
            "SELECT subtotal FROM purchases WHERE id = ?1 AND business_id = ?2",
            params![purchase_id, business_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .ok_or_else(|| {
            DbError::InvalidInput("purchase does not belong to this business".to_string())
        })?;
    let amount_paid: i64 = tx.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM purchase_payments
         WHERE business_id = ?1 AND purchase_id = ?2",
        params![business_id, purchase_id],
        |row| row.get(0),
    )?;
    let balance_due = subtotal.checked_sub(amount_paid).ok_or_else(|| {
        DbError::InvalidInput("supplier payments exceed the invoice total".into())
    })?;
    if amount > balance_due {
        return Err(DbError::InvalidInput(format!(
            "payment exceeds the outstanding balance of {balance_due}"
        )));
    }
    let payment_id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO purchase_payments
            (id, business_id, purchase_id, payment_method, amount, created_at, cash_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            payment_id,
            business_id,
            purchase_id,
            payment_method,
            amount,
            timestamp,
            account_id
        ],
    )?;
    insert_cash_movement(
        &tx,
        business_id,
        account_id,
        "supplier_payment",
        "out",
        amount,
        "purchase_payment",
        &payment_id,
        &actor.id,
        &timestamp,
    )?;
    tx.commit()?;
    load_purchase(conn, business_id, purchase_id)
}

#[cfg(test)]
pub fn record_purchase_payment(
    conn: &Connection,
    business_id: &str,
    purchase_id: &str,
    amount: i64,
    payment_method: &str,
) -> Result<Purchase, DbError> {
    let (account_id, actor) = test_payment_account(conn, business_id, payment_method)?;
    record_purchase_payment_as_user(
        conn,
        business_id,
        purchase_id,
        amount,
        payment_method,
        &account_id,
        &actor,
    )
}

pub fn create_purchase(
    conn: &Connection,
    business_id: &str,
    supplier_id: &str,
    supplier_invoice_number: &str,
    items: &[PurchaseItemInput],
    authorize_duplicate: bool,
) -> Result<Purchase, DbError> {
    let invoice_number = supplier_invoice_number.trim();
    if invoice_number.is_empty() || invoice_number.chars().count() > 200 {
        return Err(DbError::InvalidInput(
            "supplier invoice number is required and must be 200 characters or fewer".to_string(),
        ));
    }
    if items.is_empty() || items.len() > 100 {
        return Err(DbError::InvalidInput(
            "a purchase must contain between 1 and 100 products".to_string(),
        ));
    }
    let mut seen_products = HashSet::with_capacity(items.len());
    for item in items {
        if item.ordered_quantity <= 0 {
            return Err(DbError::InvalidInput(
                "ordered quantity must be greater than zero".to_string(),
            ));
        }
        if item.unit_cost < 0 {
            return Err(DbError::InvalidInput(
                "unit cost must not be negative".to_string(),
            ));
        }
        if !seen_products.insert(item.product_id.as_str()) {
            return Err(DbError::InvalidInput(
                "a product can only appear once on a purchase invoice".to_string(),
            ));
        }
    }

    let tx = conn.unchecked_transaction()?;
    let supplier: Option<i64> = tx
        .query_row(
            "SELECT is_active FROM suppliers WHERE id = ?1 AND business_id = ?2",
            params![supplier_id, business_id],
            |row| row.get(0),
        )
        .optional()?;
    match supplier {
        Some(1) => {}
        Some(_) => {
            return Err(DbError::InvalidInput(
                "the selected supplier is inactive".to_string(),
            ));
        }
        None => {
            return Err(DbError::InvalidInput(
                "supplier does not belong to this business".to_string(),
            ));
        }
    }

    let duplicate_of: Option<String> = tx
        .query_row(
            "SELECT id FROM purchases
             WHERE business_id = ?1 AND supplier_id = ?2
               AND supplier_invoice_number = ?3 COLLATE NOCASE
             ORDER BY created_at, id LIMIT 1",
            params![business_id, supplier_id, invoice_number],
            |row| row.get(0),
        )
        .optional()?;
    if duplicate_of.is_some() && !authorize_duplicate {
        return Err(DbError::InvalidInput(format!(
            "DUPLICATE_PURCHASE_INVOICE: invoice {invoice_number} already exists for this supplier; confirm authorization to record it again"
        )));
    }

    let mut lines = Vec::with_capacity(items.len());
    let mut subtotal = 0_i64;
    for item in items {
        let product: Option<(String, String, String, i64, i64)> = tx
            .query_row(
                "SELECT p.name, p.sku, u.name, u.decimals_ok, p.is_active
                 FROM products p JOIN units u ON u.id = p.base_unit_id
                 WHERE p.id = ?1 AND p.business_id = ?2",
                params![item.product_id, business_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;
        let (product_name, sku, unit_name, allows_decimals, is_active) =
            product.ok_or_else(|| {
                DbError::InvalidInput("a selected product does not belong to this business".into())
            })?;
        if is_active == 0 {
            return Err(DbError::InvalidInput(
                "a selected product is no longer active".to_string(),
            ));
        }
        if allows_decimals == 0 && item.ordered_quantity % 1000 != 0 {
            return Err(DbError::InvalidInput(format!(
                "{product_name} only allows whole base units"
            )));
        }
        let line_total = stock_value(item.ordered_quantity, item.unit_cost)?;
        subtotal = subtotal.checked_add(line_total).ok_or_else(|| {
            DbError::InvalidInput("purchase total exceeds the supported limit".into())
        })?;
        lines.push((item, product_name, sku, unit_name, line_total));
    }

    let purchase_id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let duplicate_authorized = duplicate_of.is_some();
    tx.execute(
        "INSERT INTO purchases
            (id, business_id, supplier_id, supplier_invoice_number, status, subtotal,
             duplicate_authorized, duplicate_of_purchase_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?8)",
        params![
            purchase_id,
            business_id,
            supplier_id,
            invoice_number,
            subtotal,
            i64::from(duplicate_authorized),
            duplicate_of,
            timestamp
        ],
    )?;
    for (item, product_name, sku, unit_name, line_total) in lines {
        tx.execute(
            "INSERT INTO purchase_items
                (id, business_id, purchase_id, product_id, product_name, sku, unit_name,
                 ordered_quantity, unit_cost, line_total)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                uuid::Uuid::new_v4().to_string(),
                business_id,
                purchase_id,
                item.product_id,
                product_name,
                sku,
                unit_name,
                item.ordered_quantity,
                item.unit_cost,
                line_total
            ],
        )?;
    }
    tx.commit()?;
    load_purchase(conn, business_id, &purchase_id)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReceiptItemInput {
    pub purchase_item_id: String,
    pub quantity: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PurchaseReceipt {
    pub id: String,
    pub purchase_id: String,
    pub status: String,
    pub delivery_reference: Option<String>,
    pub total_cost: i64,
    pub created_at: String,
}

pub fn receive_purchase_delivery(
    conn: &Connection,
    business_id: &str,
    purchase_id: &str,
    items: &[PurchaseReceiptItemInput],
    delivery_reference: Option<&str>,
    notes: Option<&str>,
) -> Result<PurchaseReceipt, DbError> {
    if items.is_empty() || items.len() > 100 {
        return Err(DbError::InvalidInput(
            "receive between 1 and 100 purchase lines at a time".to_string(),
        ));
    }
    let delivery_reference = delivery_reference
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if delivery_reference.is_some_and(|value| value.chars().count() > 200) {
        return Err(DbError::InvalidInput(
            "delivery reference must be 200 characters or fewer".to_string(),
        ));
    }
    let notes = notes.map(str::trim).filter(|value| !value.is_empty());
    if notes.is_some_and(|value| value.chars().count() > 500) {
        return Err(DbError::InvalidInput(
            "receiving notes must be 500 characters or fewer".to_string(),
        ));
    }
    let mut seen_items = HashSet::with_capacity(items.len());
    for item in items {
        if item.quantity <= 0 {
            return Err(DbError::InvalidInput(
                "received quantity must be greater than zero".to_string(),
            ));
        }
        if !seen_items.insert(item.purchase_item_id.as_str()) {
            return Err(DbError::InvalidInput(
                "a purchase line can only appear once per delivery".to_string(),
            ));
        }
    }

    let tx = conn.unchecked_transaction()?;
    let status: Option<String> = tx
        .query_row(
            "SELECT status FROM purchases WHERE id = ?1 AND business_id = ?2",
            params![purchase_id, business_id],
            |row| row.get(0),
        )
        .optional()?;
    if status.is_none() {
        return Err(DbError::InvalidInput(
            "purchase does not belong to this business".to_string(),
        ));
    }
    let invoice_number: String = tx.query_row(
        "SELECT supplier_invoice_number FROM purchases WHERE id = ?1",
        params![purchase_id],
        |row| row.get(0),
    )?;
    let receipt_id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    tx.execute(
        "INSERT INTO purchase_receipts
            (id, business_id, purchase_id, delivery_reference, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            receipt_id,
            business_id,
            purchase_id,
            delivery_reference,
            notes,
            timestamp
        ],
    )?;

    let mut total_cost = 0_i64;
    for item in items {
        let line: Option<(String, String, i64, i64, i64, i64, i64)> = tx
            .query_row(
                "SELECT pi.product_id, pi.product_name, pi.ordered_quantity,
                        pi.received_quantity, pi.unit_cost, u.decimals_ok,
                        COALESCE(ib.quantity_on_hand, 0)
                 FROM purchase_items pi
                 JOIN products p ON p.id = pi.product_id AND p.business_id = pi.business_id
                 JOIN units u ON u.id = p.base_unit_id
                 LEFT JOIN inventory_balances ib
                   ON ib.product_id = pi.product_id AND ib.business_id = pi.business_id
                 WHERE pi.id = ?1 AND pi.business_id = ?2 AND pi.purchase_id = ?3",
                params![item.purchase_item_id, business_id, purchase_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?;
        let (
            product_id,
            product_name,
            ordered_quantity,
            previously_received,
            unit_cost,
            allows_decimals,
            previous_quantity,
        ) = line.ok_or_else(|| {
            DbError::InvalidInput("a selected purchase line does not belong to this invoice".into())
        })?;
        if allows_decimals == 0 && item.quantity % 1000 != 0 {
            return Err(DbError::InvalidInput(format!(
                "{product_name} only allows whole base units"
            )));
        }
        let remaining_quantity = ordered_quantity - previously_received;
        if item.quantity > remaining_quantity {
            return Err(DbError::InvalidInput(format!(
                "received quantity exceeds the {remaining_quantity} remaining for {product_name}"
            )));
        }
        let new_quantity = previous_quantity
            .checked_add(item.quantity)
            .ok_or_else(|| {
                DbError::InvalidInput("received quantity exceeds the supported stock limit".into())
            })?;
        let previous_average_unit_cost: i64 = tx
            .query_row(
                "SELECT average_unit_cost FROM inventory_balances
                 WHERE product_id = ?1 AND business_id = ?2",
                params![product_id, business_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(unit_cost);
        let average_unit_cost = if previous_quantity == 0 {
            unit_cost
        } else {
            let previous_value = i128::from(previous_quantity)
                .checked_mul(i128::from(previous_average_unit_cost))
                .ok_or_else(|| {
                    DbError::InvalidInput("stock valuation exceeds the supported limit".into())
                })?;
            let received_value = i128::from(item.quantity)
                .checked_mul(i128::from(unit_cost))
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
        let line_cost = stock_value(item.quantity, unit_cost)?;
        total_cost = total_cost.checked_add(line_cost).ok_or_else(|| {
            DbError::InvalidInput("delivery total exceeds the supported limit".into())
        })?;
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
        let new_received = previously_received
            .checked_add(item.quantity)
            .ok_or_else(|| {
                DbError::InvalidInput("received quantity exceeds the supported limit".into())
            })?;
        let updated = tx.execute(
            "UPDATE purchase_items SET received_quantity = ?1
             WHERE id = ?2 AND business_id = ?3 AND purchase_id = ?4
               AND received_quantity = ?5 AND ordered_quantity >= ?1",
            params![
                new_received,
                item.purchase_item_id,
                business_id,
                purchase_id,
                previously_received
            ],
        )?;
        if updated != 1 {
            return Err(DbError::InvalidInput(format!(
                "purchase quantity changed while receiving {product_name}"
            )));
        }
        tx.execute(
            "INSERT INTO purchase_receipt_items
                (id, business_id, receipt_id, purchase_item_id, quantity, unit_cost, total_cost)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                uuid::Uuid::new_v4().to_string(),
                business_id,
                receipt_id,
                item.purchase_item_id,
                item.quantity,
                unit_cost,
                line_cost
            ],
        )?;
        let movement_reason = notes
            .map(str::to_string)
            .unwrap_or_else(|| format!("Supplier invoice {invoice_number}"));
        let movement_id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO stock_movements
                (id, business_id, product_id, movement_type, quantity_delta, unit_cost,
                 total_cost, previous_quantity, new_quantity, reference_type, reference_id,
                 reason, created_at)
             VALUES (?1, ?2, ?3, 'purchase_received', ?4, ?5, ?6, ?7, ?8,
                     'purchase_receipt', ?9, ?10, ?11)",
            params![
                movement_id,
                business_id,
                product_id,
                item.quantity,
                unit_cost,
                line_cost,
                previous_quantity,
                new_quantity,
                receipt_id,
                movement_reason,
                timestamp
            ],
        )?;
        add_inventory_cost_layer(
            &tx,
            business_id,
            &product_id,
            Some(&movement_id),
            "purchase_received",
            item.quantity,
            unit_cost,
            &timestamp,
        )?;
    }
    let (all_received, any_received): (i64, i64) = tx.query_row(
        "SELECT
            SUM(CASE WHEN received_quantity = ordered_quantity THEN 1 ELSE 0 END),
            SUM(CASE WHEN received_quantity > 0 THEN 1 ELSE 0 END)
         FROM purchase_items WHERE purchase_id = ?1",
        params![purchase_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let item_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM purchase_items WHERE purchase_id = ?1",
        params![purchase_id],
        |row| row.get(0),
    )?;
    let purchase_status = if all_received == item_count {
        "received"
    } else if any_received > 0 {
        "partially_received"
    } else {
        "pending"
    };
    tx.execute(
        "UPDATE purchases SET status = ?1, updated_at = ?2 WHERE id = ?3 AND business_id = ?4",
        params![purchase_status, timestamp, purchase_id, business_id],
    )?;
    tx.commit()?;

    Ok(PurchaseReceipt {
        id: receipt_id,
        purchase_id: purchase_id.to_string(),
        status: purchase_status.to_string(),
        delivery_reference: delivery_reference.map(str::to_string),
        total_cost,
        created_at: timestamp,
    })
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
    if selling_price < purchase_price {
        return Err(DbError::InvalidInput(
            "selling price must not be below the product's purchase cost".to_string(),
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

pub fn update_base_selling_price(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    selling_price: i64,
) -> Result<(), DbError> {
    if selling_price < 0 {
        return Err(DbError::InvalidInput(
            "selling price must not be negative".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_utc();
    let tx = conn.unchecked_transaction()?;
    let product: Option<(String, i64)> = tx
        .query_row(
            "SELECT p.base_unit_id, p.selling_price
             FROM products p
             WHERE p.id = ?1 AND p.business_id = ?2 AND p.is_active = 1",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (base_unit_id, current_price) = product.ok_or_else(|| {
        DbError::InvalidInput("active product does not belong to this business".to_string())
    })?;
    let price_floor = selling_price_floor(conn, business_id, product_id)?;
    if selling_price < price_floor {
        return Err(DbError::InvalidInput(format!(
            "selling price must not be below the highest purchase cost of stock on hand"
        )));
    }
    if current_price == selling_price {
        tx.commit()?;
        return Ok(());
    }

    let updated = tx.execute(
        "UPDATE products SET selling_price = ?1, updated_at = ?2
         WHERE id = ?3 AND business_id = ?4",
        params![selling_price, timestamp, product_id, business_id],
    )?;
    if updated != 1 {
        return Err(DbError::InvalidInput(
            "product does not belong to this business".to_string(),
        ));
    }
    let base_unit_updated = tx.execute(
        "UPDATE product_units SET selling_price = ?1, updated_at = ?2
         WHERE product_id = ?3 AND unit_id = ?4 AND is_base = 1",
        params![selling_price, timestamp, product_id, base_unit_id],
    )?;
    if base_unit_updated != 1 {
        return Err(DbError::InvalidInput(
            "product base selling unit is missing".to_string(),
        ));
    }

    tx.execute(
        "UPDATE product_prices SET effective_to = ?1
         WHERE product_id = ?2 AND unit_id = ?3
           AND price_type = 'selling' AND effective_to IS NULL",
        params![timestamp, product_id, base_unit_id],
    )?;
    tx.execute(
        "INSERT INTO product_prices
            (id, product_id, price_type, unit_id, price, effective_from, created_at)
         VALUES (?1, ?2, 'selling', ?3, ?4, ?5, ?5)",
        params![id, product_id, base_unit_id, selling_price, timestamp],
    )?;
    tx.commit()?;
    Ok(())
}

fn selling_price_floor(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
) -> Result<i64, DbError> {
    conn.query_row(
        "SELECT COALESCE(
             (SELECT MAX(icl.unit_cost)
              FROM inventory_cost_layers icl
              WHERE icl.business_id = p.business_id AND icl.product_id = p.id
                AND icl.quantity_remaining > 0),
             p.purchase_price
         )
         FROM products p
         WHERE p.business_id = ?1 AND p.id = ?2",
        params![business_id, product_id],
        |row| row.get(0),
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
    pub stock_value: i64,
    pub selling_price: i64,
    pub minimum_selling_price: i64,
    pub min_stock_quantity: i64,
    pub base_unit_allows_decimals: bool,
    pub has_movements: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct InventoryPage {
    pub items: Vec<InventoryItem>,
    pub total_count: i64,
    pub total_stock_value: i64,
    pub low_stock_count: i64,
    pub out_of_stock_count: i64,
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
    pub total_cost: Option<i64>,
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

fn add_inventory_cost_layer(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    source_movement_id: Option<&str>,
    source_type: &str,
    quantity: i64,
    unit_cost: i64,
    created_at: &str,
) -> Result<(), DbError> {
    if quantity <= 0 {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO inventory_cost_layers
            (id, business_id, product_id, source_movement_id, source_type,
             quantity_received, quantity_remaining, unit_cost, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)",
        params![
            uuid::Uuid::new_v4().to_string(),
            business_id,
            product_id,
            source_movement_id,
            source_type,
            quantity,
            unit_cost,
            created_at
        ],
    )?;
    Ok(())
}

fn consume_inventory_cost_layers(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    quantity: i64,
) -> Result<(i64, i64), DbError> {
    let layers = {
        let mut stmt = conn.prepare(
            "SELECT rowid, id, quantity_remaining, unit_cost
             FROM inventory_cost_layers
             WHERE business_id = ?1 AND product_id = ?2 AND quantity_remaining > 0
             ORDER BY created_at, rowid",
        )?;
        stmt.query_map(params![business_id, product_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
    };
    let mut remaining = quantity;
    let mut raw_cost = 0_i128;
    for (rowid, id, available, unit_cost) in layers {
        if remaining == 0 {
            break;
        }
        let consumed = remaining.min(available);
        raw_cost = raw_cost
            .checked_add(
                i128::from(consumed)
                    .checked_mul(i128::from(unit_cost))
                    .ok_or_else(|| {
                        DbError::InvalidInput(
                            "FIFO stock valuation exceeds the supported limit".into(),
                        )
                    })?,
            )
            .ok_or_else(|| {
                DbError::InvalidInput("FIFO stock valuation exceeds the supported limit".into())
            })?;
        let updated = conn.execute(
            "UPDATE inventory_cost_layers
             SET quantity_remaining = ?1
             WHERE rowid = ?2 AND id = ?3 AND business_id = ?4
               AND product_id = ?5 AND quantity_remaining = ?6",
            params![
                available - consumed,
                rowid,
                id,
                business_id,
                product_id,
                available
            ],
        )?;
        if updated != 1 {
            return Err(DbError::InvalidInput(
                "stock cost layers changed while recording the transaction".into(),
            ));
        }
        remaining -= consumed;
    }
    if remaining > 0 {
        return Err(DbError::InvalidInput(
            "stock cost layers do not cover the quantity on hand; correct inventory before continuing"
                .into(),
        ));
    }
    let total_cost = i64::try_from(
        raw_cost.checked_add(500).ok_or_else(|| {
            DbError::InvalidInput("FIFO stock valuation exceeds the supported limit".into())
        })? / 1000,
    )
    .map_err(|_| {
        DbError::InvalidInput("FIFO stock valuation exceeds the supported limit".into())
    })?;
    let unit_cost = i64::try_from(
        raw_cost
            .checked_add(i128::from(quantity / 2))
            .ok_or_else(|| {
                DbError::InvalidInput("FIFO stock valuation exceeds the supported limit".into())
            })?
            / i128::from(quantity),
    )
    .map_err(|_| DbError::InvalidInput("FIFO unit cost exceeds the supported limit".into()))?;
    Ok((unit_cost, total_cost))
}

fn inventory_layers_summary(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
) -> Result<(i64, i64, Option<i64>), DbError> {
    let mut stmt = conn.prepare(
        "SELECT quantity_remaining, unit_cost
         FROM inventory_cost_layers
         WHERE business_id = ?1 AND product_id = ?2 AND quantity_remaining > 0",
    )?;
    let mut rows = stmt.query_map(params![business_id, product_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    let (quantity, raw_value, max_unit_cost) = rows.try_fold(
        (0_i64, 0_i128, None),
        |(total_quantity, total_value, max_cost), row| {
            let (quantity, cost) = row?;
            let layer_value = i128::from(quantity)
                .checked_mul(i128::from(cost))
                .ok_or_else(|| {
                    DbError::InvalidInput("stock valuation exceeds the supported limit".into())
                })?;
            Ok::<_, DbError>((
                total_quantity.checked_add(quantity).ok_or_else(|| {
                    DbError::InvalidInput("stock quantity exceeds the supported limit".into())
                })?,
                total_value.checked_add(layer_value).ok_or_else(|| {
                    DbError::InvalidInput("stock valuation exceeds the supported limit".into())
                })?,
                Some(max_cost.map_or(cost, |current: i64| current.max(cost))),
            ))
        },
    )?;
    let stock_value = i64::try_from(
        raw_value.checked_add(500).ok_or_else(|| {
            DbError::InvalidInput("stock valuation exceeds the supported limit".into())
        })? / 1000,
    )
    .map_err(|_| DbError::InvalidInput("stock valuation exceeds the supported limit".into()))?;
    Ok((quantity, stock_value, max_unit_cost))
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
                COALESCE(ib.quantity_on_hand, 0),
                p.selling_price, p.min_stock_quantity, u.decimals_ok,
                EXISTS(SELECT 1 FROM stock_movements sm
                       WHERE sm.business_id = p.business_id AND sm.product_id = p.id),
                p.purchase_price
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
            Ok(InventoryItem {
                product_id: row.get(0)?,
                name: row.get(1)?,
                sku: row.get(2)?,
                base_unit_name: row.get(3)?,
                quantity_on_hand: row.get(4)?,
                stock_value: 0,
                selling_price: row.get(5)?,
                minimum_selling_price: row.get(9)?,
                min_stock_quantity: row.get(6)?,
                base_unit_allows_decimals: row.get::<_, i64>(7)? != 0,
                has_movements: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for item in &mut items {
        let (layer_quantity, stock_value, max_unit_cost) =
            inventory_layers_summary(conn, business_id, &item.product_id)?;
        if layer_quantity != item.quantity_on_hand {
            return Err(DbError::InvalidInput(format!(
                "inventory quantity and purchase-cost layers do not match for {}",
                item.name
            )));
        }
        item.stock_value = stock_value;
        if let Some(max_unit_cost) = max_unit_cost {
            item.minimum_selling_price = max_unit_cost;
        }
    }

    let total_stock_value = {
        let mut stmt = conn.prepare(
            "SELECT quantity_remaining, unit_cost
             FROM inventory_cost_layers WHERE business_id = ?1 AND quantity_remaining > 0",
        )?;
        let mut rows = stmt.query_map(params![business_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        let raw_value = rows.try_fold(0_i128, |total, row| {
            let (quantity, cost) = row?;
            let layer_value = i128::from(quantity)
                .checked_mul(i128::from(cost))
                .ok_or_else(|| {
                    DbError::InvalidInput(
                        "business stock valuation exceeds the supported limit".into(),
                    )
                })?;
            total.checked_add(layer_value).ok_or_else(|| {
                DbError::InvalidInput("business stock valuation exceeds the supported limit".into())
            })
        })?;
        i64::try_from(
            raw_value.checked_add(500).ok_or_else(|| {
                DbError::InvalidInput("business stock valuation exceeds the supported limit".into())
            })? / 1000,
        )
        .map_err(|_| {
            DbError::InvalidInput("business stock valuation exceeds the supported limit".into())
        })?
    };
    let (low_stock_count, out_of_stock_count): (i64, i64) = conn.query_row(
        "SELECT
             SUM(CASE WHEN COALESCE(ib.quantity_on_hand, 0) > 0
                       AND COALESCE(ib.quantity_on_hand, 0) <= p.min_stock_quantity
                      THEN 1 ELSE 0 END),
             SUM(CASE WHEN COALESCE(ib.quantity_on_hand, 0) = 0 THEN 1 ELSE 0 END)
         FROM products p
         LEFT JOIN inventory_balances ib ON ib.product_id = p.id AND ib.business_id = p.business_id
         WHERE p.business_id = ?1 AND p.is_active = 1",
        params![business_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
            ))
        },
    )?;

    Ok(InventoryPage {
        items,
        total_count,
        total_stock_value,
        low_stock_count,
        out_of_stock_count,
    })
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
                sm.quantity_delta, sm.unit_cost, sm.total_cost, sm.previous_quantity, sm.new_quantity,
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
                total_cost: row.get(7)?,
                previous_quantity: row.get(8)?,
                new_quantity: row.get(9)?,
                reference_type: row.get(10)?,
                reference_id: row.get(11)?,
                reason: row.get(12)?,
                user_id: row.get(13)?,
                created_at: row.get(14)?,
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
    let total_cost = stock_value(quantity, unit_cost)?;

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
            (id, business_id, product_id, movement_type, quantity_delta, unit_cost, total_cost,
             previous_quantity, new_quantity, reference_type, reference_id, reason, created_at)
         VALUES (?1, ?2, ?3, 'opening_stock', ?4, ?5, ?6, 0, ?4, 'product', ?3, ?7, ?8)",
        params![
            id,
            business_id,
            product_id,
            quantity,
            unit_cost,
            total_cost,
            reason,
            timestamp
        ],
    )?;
    add_inventory_cost_layer(
        &tx,
        business_id,
        product_id,
        Some(&id),
        "opening_stock",
        quantity,
        unit_cost,
        &timestamp,
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
        total_cost: Some(total_cost),
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
    let (previous_quantity, stored_average_unit_cost) =
        existing_balance.unwrap_or((0, product_unit_cost));
    let average_unit_cost = if previous_quantity == 0 {
        product_unit_cost
    } else {
        stored_average_unit_cost
    };
    if target_quantity == previous_quantity {
        return Err(DbError::InvalidInput(
            "corrected quantity must differ from the current stock".to_string(),
        ));
    }
    let quantity_delta = target_quantity - previous_quantity;
    let (movement_unit_cost, total_cost) = if quantity_delta > 0 {
        (
            average_unit_cost,
            stock_value(quantity_delta, average_unit_cost)?,
        )
    } else {
        consume_inventory_cost_layers(
            &tx,
            business_id,
            product_id,
            quantity_delta.checked_abs().ok_or_else(|| {
                DbError::InvalidInput("corrected quantity exceeds the supported limit".into())
            })?,
        )?
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
            target_quantity,
            average_unit_cost,
            timestamp
        ],
    )?;
    tx.execute(
        "INSERT INTO stock_movements
            (id, business_id, product_id, movement_type, quantity_delta, unit_cost, total_cost,
             previous_quantity, new_quantity, reference_type, reference_id, reason, created_at)
         VALUES (?1, ?2, ?3, 'stock_correction', ?4, ?5, ?6, ?7, ?8, 'product', ?3, ?9, ?10)",
        params![
            id,
            business_id,
            product_id,
            quantity_delta,
            movement_unit_cost,
            total_cost,
            previous_quantity,
            target_quantity,
            reason,
            timestamp
        ],
    )?;
    if quantity_delta > 0 {
        add_inventory_cost_layer(
            &tx,
            business_id,
            product_id,
            Some(&id),
            "stock_correction",
            quantity_delta,
            movement_unit_cost,
            &timestamp,
        )?;
    }
    tx.commit()?;

    Ok(StockMovement {
        id,
        business_id: business_id.to_string(),
        product_id: product_id.to_string(),
        product_name,
        movement_type: "stock_correction".to_string(),
        quantity_delta,
        unit_cost: Some(movement_unit_cost),
        total_cost: Some(total_cost),
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
    receive_stock_with_selling_price(
        conn,
        business_id,
        product_id,
        quantity,
        received_unit_cost,
        receipt_reference,
        reason,
        None,
    )
}

pub fn receive_stock_with_selling_price(
    conn: &Connection,
    business_id: &str,
    product_id: &str,
    quantity: i64,
    received_unit_cost: i64,
    receipt_reference: &str,
    reason: Option<&str>,
    new_selling_price: Option<i64>,
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
    if new_selling_price.is_some_and(|price| price < 0) {
        return Err(DbError::InvalidInput(
            "selling price must not be negative".to_string(),
        ));
    }
    let total_cost = stock_value(quantity, received_unit_cost)?;
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
    let product: Option<(String, i64, String, i64)> = tx
        .query_row(
            "SELECT p.name, u.decimals_ok, p.base_unit_id, p.selling_price FROM products p
             JOIN units u ON u.id = p.base_unit_id
             WHERE p.id = ?1 AND p.business_id = ?2 AND p.is_active = 1",
            params![product_id, business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let (product_name, base_unit_allows_decimals, base_unit_id, current_selling_price) = product
        .ok_or_else(|| {
            DbError::InvalidInput("active product does not belong to this business".to_string())
        })?;
    if let Some(price) = new_selling_price {
        let price_floor =
            selling_price_floor(&tx, business_id, product_id)?.max(received_unit_cost);
        if price < price_floor {
            return Err(DbError::InvalidInput(
                "selling price must not be below the highest purchase cost of stock on hand"
                    .to_string(),
            ));
        }
    }
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
            (id, business_id, product_id, movement_type, quantity_delta, unit_cost, total_cost,
             previous_quantity, new_quantity, reference_type, reference_id, reason, created_at)
         VALUES (?1, ?2, ?3, 'purchase_received', ?4, ?5, ?6, ?7, ?8,
                 'supplier_document', ?9, ?10, ?11)",
        params![
            id,
            business_id,
            product_id,
            quantity,
            received_unit_cost,
            total_cost,
            previous_quantity,
            new_quantity,
            receipt_reference,
            reason,
            timestamp
        ],
    )?;
    add_inventory_cost_layer(
        &tx,
        business_id,
        product_id,
        Some(&id),
        "purchase_received",
        quantity,
        received_unit_cost,
        &timestamp,
    )?;
    if let Some(new_selling_price) =
        new_selling_price.filter(|price| *price != current_selling_price)
    {
        let updated = tx.execute(
            "UPDATE products SET selling_price = ?1, updated_at = ?2
             WHERE id = ?3 AND business_id = ?4",
            params![new_selling_price, timestamp, product_id, business_id],
        )?;
        if updated != 1 {
            return Err(DbError::InvalidInput(
                "product does not belong to this business".to_string(),
            ));
        }
        let base_unit_updated = tx.execute(
            "UPDATE product_units SET selling_price = ?1, updated_at = ?2
             WHERE product_id = ?3 AND unit_id = ?4 AND is_base = 1",
            params![new_selling_price, timestamp, product_id, base_unit_id],
        )?;
        if base_unit_updated != 1 {
            return Err(DbError::InvalidInput(
                "product base selling unit is missing".to_string(),
            ));
        }
        tx.execute(
            "UPDATE product_prices SET effective_to = ?1
             WHERE product_id = ?2 AND unit_id = ?3
               AND price_type = 'selling' AND effective_to IS NULL",
            params![timestamp, product_id, base_unit_id],
        )?;
        tx.execute(
            "INSERT INTO product_prices
                (id, product_id, price_type, unit_id, price, effective_from, created_at)
             VALUES (?1, ?2, 'selling', ?3, ?4, ?5, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                product_id,
                base_unit_id,
                new_selling_price,
                timestamp
            ],
        )?;
    }
    tx.commit()?;

    Ok(StockMovement {
        id,
        business_id: business_id.to_string(),
        product_id: product_id.to_string(),
        product_name,
        movement_type: "purchase_received".to_string(),
        quantity_delta: quantity,
        unit_cost: Some(received_unit_cost),
        total_cost: Some(total_cost),
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
    fn role_permission_migration_grants_existing_owner_roles() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply_pragmas(&conn).unwrap();
        applied_versions(&conn).unwrap();
        for (version, name, sql) in MIGRATIONS
            .iter()
            .take_while(|(version, _, _)| *version < 16)
        {
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

        let business = create_business(&conn, "Existing shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();
        let employee = create_employee(
            &conn,
            &business.id,
            &owner,
            "manager.one",
            "Manager One",
            "manager",
            "$argon2id$manager-hash",
        )
        .unwrap();
        let manager = AuthUser {
            id: employee.id,
            business_id: employee.business_id,
            username: employee.username,
            full_name: employee.full_name,
            role: employee.role,
        };
        let (version, name, migration) = MIGRATIONS
            .iter()
            .find(|(version, _, _)| *version == 16)
            .unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute_batch(migration).unwrap();
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
            params![version, name, now_utc()],
        )
        .unwrap();
        tx.commit().unwrap();
        let cashier_role_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO roles (id, business_id, name, display_name, is_system, created_at, updated_at)
             VALUES (?1, ?2, 'cashier', 'Cashier', 1, ?3, ?3)",
            params![cashier_role_id, business.id, now_utc()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO role_permissions (role_id, permission_id)
             VALUES (?1, ?2)",
            params![cashier_role_id, PERMISSION_INVENTORY_READ],
        )
        .unwrap();
        run_migrations(&mut conn).unwrap();

        require_permission(&conn, &owner, PERMISSION_REPORTS_READ).unwrap();
        require_permission(&conn, &owner, PERMISSION_CASH_ACCOUNTS_MANAGE).unwrap();
        require_permission(&conn, &owner, PERMISSION_EXPENSES_READ).unwrap();
        require_permission(&conn, &owner, PERMISSION_EXPENSES_MANAGE).unwrap();
        require_permission(&conn, &owner, PERMISSION_CASH_ACCOUNT_BALANCE_READ).unwrap();
        require_permission(&conn, &manager, PERMISSION_INVENTORY_MANAGE).unwrap();
        require_permission(&conn, &manager, PERMISSION_CATALOG_MANAGE).unwrap();
        require_permission(&conn, &manager, PERMISSION_SALES_READ).unwrap();
        assert!(matches!(
            require_permission(&conn, &manager, PERMISSION_REPORTS_READ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            require_permission(&conn, &manager, "finance.read"),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            require_permission(&conn, &manager, PERMISSION_CASH_ACCOUNTS_MANAGE),
            Err(DbError::InvalidInput(_))
        ));
        require_permission(&conn, &manager, PERMISSION_EXPENSES_READ).unwrap();
        require_permission(&conn, &manager, PERMISSION_EXPENSES_MANAGE).unwrap();
        require_permission(&conn, &manager, PERMISSION_CASH_ACCOUNT_BALANCE_READ).unwrap();
        assert!(
            list_manager_permissions(&conn, &business.id)
                .unwrap()
                .iter()
                .any(|setting| {
                    setting.permission_id == PERMISSION_EXPENSES_READ && setting.enabled
                })
        );
        assert!(
            list_manager_permissions(&conn, &business.id)
                .unwrap()
                .iter()
                .any(|setting| {
                    setting.permission_id == PERMISSION_EXPENSES_MANAGE && setting.enabled
                })
        );
        assert!(
            list_manager_permissions(&conn, &business.id)
                .unwrap()
                .iter()
                .any(|setting| setting.permission_id == PERMISSION_CASH_ACCOUNT_BALANCE_READ)
        );
        let cashier_permissions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM role_permissions WHERE role_id = ?1",
                params![cashier_role_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cashier_permissions, 0);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM role_permissions rp
                 JOIN user_roles ur ON ur.role_id = rp.role_id
                 WHERE ur.user_id = ?1",
                params![owner.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            conn.query_row("SELECT COUNT(*) FROM permissions", [], |row| row
                .get::<_, i64>(0))
                .unwrap()
        );
    }

    #[test]
    fn owner_bootstrap_is_atomic_business_scoped_and_audited() {
        let conn = open_in_memory().unwrap();

        assert!(auth_business_status(&conn).unwrap().is_empty());
        let owner = bootstrap_owner(
            &conn,
            None,
            Some("Hardware shop"),
            "  OWNER.Admin  ",
            "  Aline Uwase  ",
            "$argon2id$test-hash",
        )
        .unwrap();

        assert_eq!(owner.username, "owner.admin");
        assert_eq!(owner.full_name, "Aline Uwase");
        assert_eq!(owner.role, "owner");
        assert_eq!(
            auth_business_status(&conn).unwrap()[0].requires_owner_setup,
            false
        );
        let credentials = auth_credentials(&conn, &owner.business_id, "OWNER.ADMIN")
            .unwrap()
            .unwrap();
        assert_eq!(credentials.user, owner);
        assert_eq!(credentials.password_hash, "$argon2id$test-hash");
        assert!(matches!(
            bootstrap_owner(
                &conn,
                Some(&owner.business_id),
                None,
                "second.owner",
                "Second Owner",
                "$argon2id$other"
            ),
            Err(DbError::InvalidInput(_))
        ));
        record_successful_login(&conn, &owner).unwrap();
        assert!(
            auth_credentials(&conn, &owner.business_id, "missing")
                .unwrap()
                .is_none()
        );
        let audit_action: String = conn
            .query_row(
                "SELECT action FROM audit_logs WHERE entity_id = ?1",
                params![owner.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_action, "owner_bootstrapped");
        let last_login_at: Option<String> = conn
            .query_row(
                "SELECT last_login_at FROM users WHERE id = ?1",
                params![owner.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(last_login_at.is_some());
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action = 'user_signed_in'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn employee_profiles_are_owner_created_audited_and_manager_can_sign_in() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();

        let employee = create_employee(
            &conn,
            &business.id,
            &owner,
            "  manager.one ",
            " Jean Niyonzima ",
            "manager",
            "$argon2id$employee-hash",
        )
        .unwrap();
        assert_eq!(employee.username, "manager.one");
        assert_eq!(employee.full_name, "Jean Niyonzima");
        assert_eq!(employee.role, "manager");
        assert!(employee.is_active);
        assert_eq!(
            list_employees(&conn, &business.id)
                .unwrap()
                .iter()
                .map(|listed| listed.id.as_str())
                .collect::<Vec<_>>(),
            vec![employee.id.as_str()]
        );
        assert!(
            list_employees(&conn, &other_business.id)
                .unwrap()
                .is_empty()
        );
        let manager_credentials = auth_credentials(&conn, &business.id, &employee.username)
            .unwrap()
            .expect("active Manager profiles can sign in");
        assert_eq!(manager_credentials.user.role, "manager");
        assert_eq!(manager_credentials.password_hash, "$argon2id$employee-hash");
        assert!(DbState::is_active_session(&conn, &manager_credentials.user).unwrap());
        let manager_permissions = list_user_permissions(&conn, &manager_credentials.user).unwrap();
        assert!(manager_permissions.contains(&PERMISSION_CATALOG_MANAGE.to_string()));
        assert!(!manager_permissions.contains(&PERMISSION_REPORTS_READ.to_string()));
        assert!(matches!(
            create_employee(
                &conn,
                &business.id,
                &owner,
                "cashier.one",
                "Cashier",
                "cashier",
                "$argon2id$manager-hash"
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_employee(
                &conn,
                &business.id,
                &owner,
                "owner.two",
                "Invalid role",
                "owner",
                "$argon2id$invalid"
            ),
            Err(DbError::InvalidInput(_))
        ));

        set_employee_active(&conn, &business.id, &employee.id, false, &owner).unwrap();
        assert!(!list_employees(&conn, &business.id).unwrap()[0].is_active);
        assert!(
            auth_credentials(&conn, &business.id, &employee.username)
                .unwrap()
                .is_none(),
            "deactivated Manager profiles cannot sign in"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE business_id = ?1 AND user_id = ?2
                   AND action IN ('employee_created', 'employee_deactivated')",
                params![business.id, owner.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            2
        );
        reset_employee_password(
            &conn,
            &business.id,
            &employee.id,
            "$argon2id$new-manager-hash",
            &owner,
        )
        .unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                params![employee.id],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "$argon2id$new-manager-hash"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE business_id = ?1 AND user_id = ?2
                   AND action = 'manager_password_reset' AND entity_id = ?3",
                params![business.id, owner.id, employee.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
        assert!(
            reset_employee_password(
                &conn,
                &other_business.id,
                &employee.id,
                "$argon2id$wrong-business",
                &owner,
            )
            .is_err()
        );
        assert!(
            reset_employee_password(
                &conn,
                &business.id,
                &owner.id,
                "$argon2id$owner-target",
                &owner,
            )
            .is_err()
        );
        assert!(set_employee_active(&conn, &business.id, &owner.id, false, &owner).is_err());
    }

    #[test]
    fn manager_permissions_are_configurable_without_granting_sensitive_access() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();
        require_permission(&conn, &owner, PERMISSION_REPORTS_READ).unwrap();

        require_permission(&conn, &owner, PERMISSION_ROLES_MANAGE).unwrap();
        let employee = create_employee(
            &conn,
            &business.id,
            &owner,
            "manager",
            "Jean Niyonzima",
            "manager",
            "$argon2id$manager-hash",
        )
        .unwrap();
        let manager = AuthUser {
            id: employee.id,
            business_id: employee.business_id,
            username: employee.username,
            full_name: employee.full_name,
            role: employee.role,
        };
        require_permission(&conn, &manager, PERMISSION_POS_USE).unwrap();
        assert!(matches!(
            require_permission(&conn, &manager, PERMISSION_REPORTS_READ),
            Err(DbError::InvalidInput(_))
        ));

        let mut grants = list_manager_permissions(&conn, &business.id)
            .unwrap()
            .into_iter()
            .map(|setting| RolePermissionGrant {
                enabled: setting.enabled && setting.permission_id != PERMISSION_POS_USE,
                permission_id: setting.permission_id,
            })
            .collect::<Vec<_>>();
        let updated = update_manager_permissions(&conn, &business.id, &owner, &grants).unwrap();
        assert!(
            !updated
                .iter()
                .find(|setting| setting.permission_id == PERMISSION_POS_USE)
                .unwrap()
                .enabled
        );
        assert!(matches!(
            require_permission(&conn, &manager, PERMISSION_POS_USE),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            require_permission(&conn, &manager, PERMISSION_REPORTS_READ),
            Err(DbError::InvalidInput(_))
        ));
        let second_manager = create_employee(
            &conn,
            &business.id,
            &owner,
            "manager.two",
            "Second Manager",
            "manager",
            "$argon2id$manager-two-hash",
        )
        .unwrap();
        assert!(matches!(
            require_permission(
                &conn,
                &AuthUser {
                    id: second_manager.id,
                    business_id: second_manager.business_id,
                    username: second_manager.username,
                    full_name: second_manager.full_name,
                    role: second_manager.role,
                },
                PERMISSION_POS_USE
            ),
            Err(DbError::InvalidInput(_))
        ));

        grants.push(RolePermissionGrant {
            permission_id: PERMISSION_REPORTS_READ.to_string(),
            enabled: true,
        });
        assert!(matches!(
            update_manager_permissions(&conn, &business.id, &owner, &grants),
            Err(DbError::InvalidInput(_))
        ));
        let non_owner = AuthUser {
            role: "manager".to_string(),
            ..owner.clone()
        };
        let current_grants = list_manager_permissions(&conn, &business.id).unwrap();
        let current_grants = current_grants
            .into_iter()
            .map(|setting| RolePermissionGrant {
                permission_id: setting.permission_id,
                enabled: setting.enabled,
            })
            .collect::<Vec<_>>();
        assert!(
            update_manager_permissions(&conn, &business.id, &non_owner, &current_grants).is_err()
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE business_id = ?1
                 AND user_id = ?2 AND action = 'manager_permissions_updated'",
                params![business.id, owner.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn authenticated_owner_can_create_another_business_with_the_same_login() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "First shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$test-hash",
        )
        .unwrap();

        let second_business =
            create_business_for_owner(&conn, &owner, "Second shop", Some("TIN-123")).unwrap();
        let second_owner = auth_credentials(&conn, &second_business.id, "owner")
            .unwrap()
            .unwrap();

        assert_eq!(second_business.name, "Second shop");
        assert_eq!(second_business.tax_id.as_deref(), Some("TIN-123"));
        assert_eq!(second_owner.user.full_name, owner.full_name);
        assert_eq!(second_owner.user.role, "owner");
        assert_eq!(second_owner.password_hash, "$argon2id$test-hash");
        require_permission(&conn, &second_owner.user, PERMISSION_REPORTS_READ).unwrap();
        let non_owner = AuthUser {
            role: "cashier".to_string(),
            ..owner
        };
        assert!(matches!(
            create_business_for_owner(&conn, &non_owner, "Unauthorized shop", None),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn authentication_session_is_required_and_scoped_to_its_business() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$test-hash",
        )
        .unwrap();
        let state = DbState::new(conn);

        assert!(matches!(
            state.with_authenticated(|_, user| Ok(user.clone())),
            Err(DbError::InvalidInput(_))
        ));
        assert!(state.current_session_user().unwrap().is_none());
        state.set_session(owner.clone()).unwrap();
        assert_eq!(state.current_session_user().unwrap().unwrap().id, owner.id);
        assert_eq!(
            state
                .with_business_permission(&business.id, PERMISSION_SYSTEM_READ, |_, _| Ok(
                    "authorized"
                ),)
                .unwrap(),
            "authorized"
        );
        assert_eq!(
            state
                .with_business_permission(&business.id, PERMISSION_REPORTS_READ, |_, _| Ok(
                    "permitted"
                ),)
                .unwrap(),
            "permitted"
        );
        assert!(
            state
                .with_business_permission(&business.id, "unknown.permission", |_, _| Ok(()))
                .is_err()
        );
        assert!(
            state
                .with_business_permission(
                    &other_business.id,
                    PERMISSION_SYSTEM_READ,
                    |_, _| Ok(()),
                )
                .is_err()
        );
        assert_eq!(
            state
                .with_authenticated(|_, user| Ok(user.id.clone()))
                .unwrap(),
            owner.id
        );
        state.clear_session().unwrap();
        assert!(state.current_session_user().unwrap().is_none());
        assert!(state.with_authenticated(|_, _| Ok(())).is_err());
    }

    #[test]
    fn manager_session_stays_authenticated_and_receives_operational_permissions() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();
        let employee = create_employee(
            &conn,
            &business.id,
            &owner,
            "manager.one",
            "Manager One",
            "manager",
            "$argon2id$manager-hash",
        )
        .unwrap();
        let manager = AuthUser {
            id: employee.id,
            business_id: employee.business_id,
            username: employee.username,
            full_name: employee.full_name,
            role: employee.role,
        };
        let state = DbState::new(conn);
        state.set_session(manager).unwrap();

        let permissions = state.with_authenticated(list_user_permissions).unwrap();
        assert!(permissions.contains(&PERMISSION_CATALOG_MANAGE.to_string()));
        assert!(permissions.contains(&PERMISSION_PURCHASES_MANAGE.to_string()));
        assert!(permissions.contains(&PERMISSION_CUSTOMERS_READ.to_string()));
        assert!(permissions.contains(&PERMISSION_CUSTOMER_PAYMENTS_MANAGE.to_string()));
        assert!(!permissions.contains(&PERMISSION_REPORTS_READ.to_string()));
        assert!(!permissions.contains(&"finance.read".to_string()));
        assert!(permissions.contains(&PERMISSION_EXPENSES_READ.to_string()));
        assert!(permissions.contains(&PERMISSION_EXPENSES_MANAGE.to_string()));
        assert!(permissions.contains(&PERMISSION_CASH_ACCOUNT_BALANCE_READ.to_string()));
        state
            .with_authorized(PERMISSION_CATALOG_MANAGE, |_, _| Ok(()))
            .unwrap();
        state
            .with_authorized(PERMISSION_CUSTOMERS_READ, |_, _| Ok(()))
            .unwrap();
        assert!(
            state
                .with_authorized(PERMISSION_REPORTS_READ, |_, _| Ok(()))
                .is_err()
        );
    }

    #[test]
    fn cash_ledger_migration_preserves_legacy_transactions_without_guessing_custody() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply_pragmas(&conn).unwrap();
        applied_versions(&conn).unwrap();
        for (version, name, sql) in MIGRATIONS
            .iter()
            .take_while(|(version, _, _)| *version < 15)
        {
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
        conn.execute(
            "INSERT INTO expenses
                (id, business_id, category, amount, expense_date, payment_method,
                 description, responsible_name, created_at)
             VALUES (?1, ?2, 'rent', 250000, '2026-11-02', 'cash',
                     'Existing rent expense', 'Manager', ?3)",
            params![uuid::Uuid::new_v4().to_string(), business.id, now_utc()],
        )
        .unwrap();

        run_migrations(&mut conn).unwrap();

        let expenses = list_expenses(&conn, &business.id, 0, 20).unwrap();
        assert_eq!(expenses.total_count, 1);
        assert_eq!(expenses.items[0].amount, 250_000);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM cash_movements", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0,
            "legacy transactions have no reliable custody-account mapping"
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM cash_accounts", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn cash_account_management_scopes_accounts_assigns_cash_drawer_and_audits_status() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$test-hash",
        )
        .unwrap();

        let drawer = create_cash_account(
            &conn,
            &business.id,
            &owner,
            "cash_drawer",
            " Front counter ",
        )
        .unwrap();
        assert_eq!(drawer.name, "Front counter");
        assert_eq!(
            drawer.responsible_user_id.as_deref(),
            Some(owner.id.as_str())
        );
        assert_eq!(drawer.responsible_name.as_deref(), Some("Aline Uwase"));

        let bank = create_cash_account(&conn, &business.id, &owner, "bank", "Shop bank").unwrap();
        assert_eq!(bank.responsible_user_id, None);
        assert!(
            list_cash_accounts(&conn, &other_business.id)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            create_cash_account(&conn, &business.id, &owner, "unsupported", "Invalid"),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_cash_account(&conn, &other_business.id, &owner, "bank", "Wrong business"),
            Err(DbError::InvalidInput(_))
        ));

        set_cash_account_active(&conn, &business.id, &drawer.id, false, &owner).unwrap();
        let listed = list_cash_accounts(&conn, &business.id).unwrap();
        assert!(
            !listed
                .iter()
                .find(|account| account.id == drawer.id)
                .unwrap()
                .is_active
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE business_id = ?1 AND user_id = ?2
                   AND action IN ('cash_account_created', 'cash_account_deactivated')",
                params![business.id, owner.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            3
        );
        set_cash_account_active(&conn, &business.id, &drawer.id, true, &owner).unwrap();
        assert!(
            list_cash_accounts(&conn, &business.id)
                .unwrap()
                .iter()
                .find(|account| account.id == drawer.id)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn cash_account_opening_is_owner_authorized_immutable_and_drives_available_balance() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();
        let manager = create_employee(
            &conn,
            &business.id,
            &owner,
            "manager.one",
            "Manager One",
            "manager",
            "$argon2id$manager-hash",
        )
        .unwrap();
        let manager = AuthUser {
            id: manager.id,
            business_id: manager.business_id,
            username: manager.username,
            full_name: manager.full_name,
            role: manager.role,
        };
        let account =
            create_cash_account(&conn, &business.id, &owner, "cash_drawer", "Main drawer").unwrap();

        assert!(cash_account_balance(&conn, &business.id, &account.id).is_err());
        assert!(
            create_cash_account_opening(&conn, &business.id, &account.id, 10_000, &manager)
                .is_err()
        );
        create_cash_account_opening(&conn, &business.id, &account.id, 10_000, &owner).unwrap();
        assert!(create_cash_account_opening(&conn, &business.id, &account.id, 1, &owner).is_err());
        assert_eq!(
            cash_account_balance(&conn, &business.id, &account.id).unwrap(),
            10_000
        );

        let timestamp = now_utc();
        insert_cash_movement(
            &conn,
            &business.id,
            &account.id,
            "other_receipt",
            "in",
            1_500,
            "other",
            &uuid::Uuid::new_v4().to_string(),
            &owner.id,
            &timestamp,
        )
        .unwrap();
        assert_eq!(
            cash_account_balance(&conn, &business.id, &account.id).unwrap(),
            11_500
        );
        assert!(
            conn.execute(
                "UPDATE cash_account_openings SET opening_amount = 1
                 WHERE business_id = ?1 AND account_id = ?2",
                params![business.id, account.id],
            )
            .is_err()
        );
        assert!(
            conn.execute(
                "DELETE FROM cash_account_openings
                 WHERE business_id = ?1 AND account_id = ?2",
                params![business.id, account.id],
            )
            .is_err()
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE business_id = ?1 AND action = 'cash_account_opened'",
                params![business.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn cash_transfers_require_funds_and_post_both_legs_atomically() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();
        let cash =
            create_cash_account(&conn, &business.id, &owner, "cash_drawer", "Main drawer").unwrap();
        let bank =
            create_cash_account(&conn, &business.id, &owner, "bank", "Business bank").unwrap();
        create_cash_account_opening(&conn, &business.id, &cash.id, 25_000, &owner).unwrap();
        create_cash_account_opening(&conn, &business.id, &bank.id, 0, &owner).unwrap();

        let transfer =
            create_cash_transfer(&conn, &business.id, &cash.id, &bank.id, 20_000, &owner).unwrap();
        assert_eq!(transfer.status, "pending");
        assert_eq!(
            cash_account_balance(&conn, &business.id, &cash.id).unwrap(),
            25_000
        );
        assert!(
            create_cash_transfer(&conn, &business.id, &cash.id, &bank.id, 26_000, &owner).is_err()
        );

        confirm_cash_transfer(
            &conn,
            &business.id,
            &transfer.id,
            Some("BANK-REF-01"),
            &owner,
        )
        .unwrap();
        assert_eq!(
            cash_account_balance(&conn, &business.id, &cash.id).unwrap(),
            5_000
        );
        assert_eq!(
            cash_account_balance(&conn, &business.id, &bank.id).unwrap(),
            20_000
        );
        let stored = list_cash_transfers(&conn, &business.id).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status, "confirmed");
        assert_eq!(
            stored[0].confirmation_reference.as_deref(),
            Some("BANK-REF-01")
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM cash_movements WHERE transfer_id = ?1",
                params![transfer.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            2
        );
        assert!(confirm_cash_transfer(&conn, &business.id, &transfer.id, None, &owner).is_err());

        let pending =
            create_cash_transfer(&conn, &business.id, &cash.id, &bank.id, 1_000, &owner).unwrap();
        set_cash_account_active(&conn, &business.id, &bank.id, false, &owner).unwrap();
        assert!(confirm_cash_transfer(&conn, &business.id, &pending.id, None, &owner).is_err());
        assert_eq!(
            cash_account_balance(&conn, &business.id, &cash.id).unwrap(),
            5_000
        );
    }

    #[test]
    fn cash_accounts_and_movements_enforce_custody_and_prevent_duplicate_postings() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let branch_id = uuid::Uuid::new_v4().to_string();
        let user_id = uuid::Uuid::new_v4().to_string();
        let timestamp = now_utc();
        conn.execute(
            "INSERT INTO branches (id, business_id, name, created_at, updated_at)
             VALUES (?1, ?2, 'Main branch', ?3, ?3)",
            params![branch_id, business.id, timestamp],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users
                (id, business_id, branch_id, username, full_name, password_hash,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, 'cashier', 'Cashier', 'test-hash', ?4, ?4)",
            params![user_id, business.id, branch_id, timestamp],
        )
        .unwrap();
        let cash_account_id = uuid::Uuid::new_v4().to_string();
        let bank_account_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO cash_accounts
                (id, business_id, branch_id, account_type, name, responsible_user_id,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, 'cash_drawer', 'Cashier drawer', ?4, ?5, ?5)",
            params![cash_account_id, business.id, branch_id, user_id, timestamp],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cash_accounts
                (id, business_id, branch_id, account_type, name, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'bank', 'Shop bank account', ?4, ?4)",
            params![bank_account_id, business.id, branch_id, timestamp],
        )
        .unwrap();
        for account_id in [&cash_account_id, &bank_account_id] {
            conn.execute(
                "INSERT INTO cash_account_openings
                    (id, business_id, account_id, opening_amount, opened_at,
                     entered_by_user_id, authorized_by_user_id, recorded_at)
                 VALUES (?1, ?2, ?3, 100000, ?4, ?5, ?5, ?4)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    business.id,
                    account_id,
                    timestamp,
                    user_id
                ],
            )
            .unwrap();
        }

        let movement_id = uuid::Uuid::new_v4().to_string();
        let source_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO cash_movements
                (id, business_id, account_id, movement_type, direction, amount,
                 source_type, source_id, responsible_user_id, occurred_at, recorded_at)
             VALUES (?1, ?2, ?3, 'other_receipt', 'in', 10_000,
                     'other', ?4, ?5, ?6, ?6)",
            params![
                movement_id,
                business.id,
                cash_account_id,
                source_id,
                user_id,
                timestamp
            ],
        )
        .unwrap();

        assert!(
            conn.execute(
                "INSERT INTO cash_movements
                    (id, business_id, account_id, movement_type, direction, amount,
                     source_type, source_id, responsible_user_id, occurred_at, recorded_at)
                 VALUES (?1, ?2, ?3, 'other_receipt', 'in', 10_000,
                         'other', ?4, ?5, ?6, ?6)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    business.id,
                    cash_account_id,
                    source_id,
                    user_id,
                    timestamp
                ],
            )
            .is_err(),
            "the same source payment must not post twice"
        );
        assert!(
            conn.execute(
                "INSERT INTO cash_accounts
                    (id, business_id, account_type, name, created_at, updated_at)
                 VALUES (?1, ?2, 'cash_drawer', 'Unassigned drawer', ?3, ?3)",
                params![uuid::Uuid::new_v4().to_string(), business.id, timestamp],
            )
            .is_err(),
            "a worker cash drawer must identify its responsible user"
        );
        assert!(
            conn.execute(
                "INSERT INTO cash_accounts
                    (id, business_id, branch_id, account_type, name, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'bank', 'Cross-business branch', ?4, ?4)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    other_business.id,
                    branch_id,
                    timestamp
                ],
            )
            .is_err(),
            "an account cannot use another business's branch"
        );
        assert!(
            conn.execute(
                "UPDATE cash_movements SET amount = 9_000 WHERE id = ?1",
                params![movement_id],
            )
            .is_err(),
            "ledger movements must be immutable"
        );
    }

    #[test]
    fn cash_transfer_confirmation_posts_paired_immutable_movements() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let branch_id = uuid::Uuid::new_v4().to_string();
        let initiator_id = uuid::Uuid::new_v4().to_string();
        let confirmer_id = uuid::Uuid::new_v4().to_string();
        let timestamp = now_utc();
        conn.execute(
            "INSERT INTO branches (id, business_id, name, created_at, updated_at)
             VALUES (?1, ?2, 'Main branch', ?3, ?3)",
            params![branch_id, business.id, timestamp],
        )
        .unwrap();
        for (id, username) in [(&initiator_id, "initiator"), (&confirmer_id, "confirmer")] {
            conn.execute(
                "INSERT INTO users
                    (id, business_id, branch_id, username, full_name, password_hash,
                     created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4, 'test-hash', ?5, ?5)",
                params![id, business.id, branch_id, username, timestamp],
            )
            .unwrap();
        }
        let from_account = uuid::Uuid::new_v4().to_string();
        let to_account = uuid::Uuid::new_v4().to_string();
        for (account_id, account_type, name, responsible_user_id) in [
            (
                &from_account,
                "cash_drawer",
                "Cash drawer",
                Some(&initiator_id),
            ),
            (&to_account, "mtn_wallet", "MTN wallet", None),
        ] {
            conn.execute(
                "INSERT INTO cash_accounts
                    (id, business_id, branch_id, account_type, name, responsible_user_id,
                     created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    account_id,
                    business.id,
                    branch_id,
                    account_type,
                    name,
                    responsible_user_id,
                    timestamp
                ],
            )
            .unwrap();
        }
        for (account_id, opening_amount, user_id) in [
            (&from_account, 100_000_i64, &initiator_id),
            (&to_account, 0_i64, &initiator_id),
        ] {
            conn.execute(
                "INSERT INTO cash_account_openings
                    (id, business_id, account_id, opening_amount, opened_at,
                     entered_by_user_id, authorized_by_user_id, recorded_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?5)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    business.id,
                    account_id,
                    opening_amount,
                    timestamp,
                    user_id
                ],
            )
            .unwrap();
        }
        let transfer_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO cash_transfers
                (id, business_id, from_account_id, to_account_id, amount,
                 occurred_at, initiated_by_user_id, created_at)
             VALUES (?1, ?2, ?3, ?4, 25_000, ?5, ?6, ?5)",
            params![
                transfer_id,
                business.id,
                from_account,
                to_account,
                timestamp,
                initiator_id
            ],
        )
        .unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM cash_movements WHERE transfer_id = ?1",
                params![transfer_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0,
            "pending transfers must not affect account balances"
        );

        conn.execute(
            "UPDATE cash_transfers
             SET status = 'confirmed', confirmed_by_user_id = ?1, confirmed_at = ?2
             WHERE id = ?3",
            params![confirmer_id, timestamp, transfer_id],
        )
        .unwrap();

        let movements = conn
            .prepare(
                "SELECT account_id, movement_type, direction, amount
                 FROM cash_movements WHERE transfer_id = ?1 ORDER BY source_leg",
            )
            .unwrap()
            .query_map(params![transfer_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            movements,
            vec![
                (
                    to_account,
                    "transfer_in".to_string(),
                    "in".to_string(),
                    25_000
                ),
                (
                    from_account,
                    "transfer_out".to_string(),
                    "out".to_string(),
                    25_000
                )
            ]
        );
        assert!(
            conn.execute(
                "UPDATE cash_transfers SET amount = 30_000 WHERE id = ?1",
                params![transfer_id],
            )
            .is_err(),
            "a confirmed transfer must not be overwritten"
        );
    }

    #[test]
    fn inventory_cost_layer_migration_reconstructs_fifo_and_uses_average_for_unmatched_stock() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply_pragmas(&conn).unwrap();
        applied_versions(&conn).unwrap();
        for (version, name, sql) in MIGRATIONS
            .iter()
            .take_while(|(version, _, _)| *version < 11)
        {
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
        let reconstructed = create_test_product(&conn, &business.id, "FIFO-MIGRATION-01");
        let unmatched = create_test_product(&conn, &business.id, "FIFO-MIGRATION-02");
        let timestamp = now_utc();
        conn.execute(
            "INSERT INTO inventory_balances
                (product_id, business_id, quantity_on_hand, average_unit_cost, updated_at)
             VALUES (?1, ?2, 2500, 6500, ?3), (?4, ?2, 2000, 7000, ?3)",
            params![reconstructed.id, business.id, timestamp, unmatched.id],
        )
        .unwrap();
        for (id, product_id, movement_type, quantity, unit_cost, previous, new, created_at) in [
            (
                "fifo-opening",
                reconstructed.id.as_str(),
                "opening_stock",
                3000,
                5000,
                0,
                3000,
                "2026-01-01T00:00:00Z",
            ),
            (
                "fifo-receipt",
                reconstructed.id.as_str(),
                "purchase_received",
                3000,
                8000,
                3000,
                6000,
                "2026-01-02T00:00:00Z",
            ),
            (
                "fifo-sale",
                reconstructed.id.as_str(),
                "sale",
                -3500,
                6500,
                6000,
                2500,
                "2026-01-03T00:00:00Z",
            ),
        ] {
            conn.execute(
                "INSERT INTO stock_movements
                    (id, business_id, product_id, movement_type, quantity_delta, unit_cost,
                     previous_quantity, new_quantity, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    business.id,
                    product_id,
                    movement_type,
                    quantity,
                    unit_cost,
                    previous,
                    new,
                    created_at
                ],
            )
            .unwrap();
        }

        run_migrations(&mut conn).unwrap();

        let reconstructed_layer: (i64, i64, String) = conn
            .query_row(
                "SELECT quantity_remaining, unit_cost, source_type
                 FROM inventory_cost_layers WHERE product_id = ?1",
                params![reconstructed.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            reconstructed_layer,
            (2500, 8000, "purchase_received".into())
        );
        let fallback_layer: (i64, i64, String) = conn
            .query_row(
                "SELECT quantity_remaining, unit_cost, source_type
                 FROM inventory_cost_layers WHERE product_id = ?1",
                params![unmatched.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(fallback_layer, (2000, 7000, "legacy_remainder".into()));
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
        let movement_total: i64 = conn
            .query_row(
                "SELECT total_cost FROM stock_movements WHERE id = 'legacy-movement'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(balance_cost, product.purchase_price);
        assert_eq!(movement_cost, product.purchase_price);
        assert_eq!(movement_total, 20_000);
    }

    #[test]
    fn sale_price_override_migration_backfills_listed_prices_for_existing_sales() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply_pragmas(&conn).unwrap();
        applied_versions(&conn).unwrap();
        for (version, name, sql) in MIGRATIONS.iter().take(7) {
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
        let product = create_test_product(&conn, &business.id, "SALE-PRICE-MIGRATION-01");
        let timestamp = now_utc();
        conn.execute(
            "INSERT INTO sales
                (id, business_id, transaction_number, subtotal, discount, total, created_at)
             VALUES ('legacy-sale', ?1, 'SALE-LEGACY-01', 12000, 0, 12000, ?2)",
            params![business.id, timestamp],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sale_items
                (id, sale_id, business_id, product_id, product_name, sku, quantity,
                 unit_name, unit_price, line_total, unit_cost, total_cost)
             VALUES ('legacy-sale-item', 'legacy-sale', ?1, ?2, ?3, ?4, 1000,
                     ?5, 12000, 12000, 10000, 10000)",
            params![
                business.id,
                product.id,
                product.name,
                product.sku,
                product.base_unit_name
            ],
        )
        .unwrap();

        run_migrations(&mut conn).unwrap();

        let (listed_price, reason): (i64, Option<String>) = conn
            .query_row(
                "SELECT listed_unit_price, price_change_reason
                 FROM sale_items WHERE id = 'legacy-sale-item'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(listed_price, 12_000);
        assert_eq!(reason, None);
    }

    #[test]
    fn selling_price_update_preserves_purchase_cost_and_records_price_history() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "PRICE-UPDATE-01");
        let new_price = 70_000;

        update_base_selling_price(&conn, &business.id, &product.id, new_price).unwrap();
        let updated = list_products(&conn, &business.id, Some("PRICE-UPDATE-01"), 0, 20).unwrap();
        let base_unit_price: i64 = conn
            .query_row(
                "SELECT pu.selling_price
                 FROM product_units pu
                 JOIN products p ON p.id = pu.product_id
                 WHERE p.id = ?1 AND pu.is_base = 1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();
        let history: Vec<(i64, Option<String>)> = conn
            .prepare(
                "SELECT price, effective_to FROM product_prices
                 WHERE product_id = ?1 AND price_type = 'selling' AND unit_id = ?2
                 ORDER BY effective_to IS NULL",
            )
            .unwrap()
            .query_map(params![product.id, product.base_unit_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(updated.items[0].purchase_price, product.purchase_price);
        assert_eq!(updated.items[0].selling_price, new_price);
        assert_eq!(base_unit_price, new_price);
        assert_eq!(history.len(), 2);
        assert!(
            history
                .iter()
                .any(|(price, ended)| { *price == product.selling_price && ended.is_some() })
        );
        assert!(
            history
                .iter()
                .any(|(price, ended)| { *price == new_price && ended.is_none() })
        );
        let pos_products =
            search_pos_products(&conn, &business.id, Some("PRICE-UPDATE-01"), 0, 10).unwrap();
        assert_eq!(pos_products.items[0].selling_price, new_price);

        update_base_selling_price(&conn, &business.id, &product.id, new_price).unwrap();
        assert!(matches!(
            update_base_selling_price(&conn, &business.id, &product.id, -1),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            update_base_selling_price(&conn, &other_business.id, &product.id, 80_000),
            Err(DbError::InvalidInput(_))
        ));
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
            "CEM-2",
            None,
            None,
            None,
            None,
            "sys-unit-bag",
            10_000,
            9_999,
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
    fn suppliers_are_business_scoped_and_contact_fields_are_normalized() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();

        let supplier = create_supplier(
            &conn,
            &business.id,
            "  Kigali Building Supplies  ",
            Some("  Alice Uwase  "),
            Some("  +250 788 000 000  "),
            Some("  sales@example.rw  "),
            Some("  Kigali  "),
            Some("  TIN-123  "),
        )
        .unwrap();
        create_supplier(
            &conn,
            &other_business.id,
            "Kigali Building Supplies",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(supplier.name, "Kigali Building Supplies");
        assert_eq!(supplier.contact_person.as_deref(), Some("Alice Uwase"));
        assert_eq!(supplier.phone.as_deref(), Some("+250 788 000 000"));
        assert_eq!(supplier.email.as_deref(), Some("sales@example.rw"));
        assert_eq!(supplier.address.as_deref(), Some("Kigali"));
        assert_eq!(supplier.tax_id.as_deref(), Some("TIN-123"));
        assert_eq!(list_suppliers(&conn, &business.id).unwrap().len(), 1);
        assert_eq!(list_suppliers(&conn, &other_business.id).unwrap().len(), 1);
        assert!(matches!(
            update_supplier(
                &conn,
                &other_business.id,
                &supplier.id,
                "Unauthorized edit",
                None,
                None,
                None,
                None,
                None,
                true
            ),
            Err(DbError::InvalidInput(_))
        ));

        let updated = update_supplier(
            &conn,
            &business.id,
            &supplier.id,
            "Kigali Building Supplies Ltd",
            None,
            Some("  "),
            None,
            None,
            None,
            false,
        )
        .unwrap();
        assert_eq!(updated.name, "Kigali Building Supplies Ltd");
        assert_eq!(updated.phone, None);
        assert!(!updated.is_active);
        assert_eq!(
            list_suppliers(&conn, &business.id).unwrap()[0].id,
            supplier.id
        );
    }

    #[test]
    fn customers_are_business_scoped_and_profile_fields_are_normalized() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();

        let customer = create_customer(
            &conn,
            &business.id,
            "  Aline Mukamana  ",
            Some("  +250 788 000 000  "),
            Some("  Kigali  "),
            "individual",
            1_250_500,
        )
        .unwrap();
        create_customer(
            &conn,
            &other_business.id,
            "Aline Mukamana",
            None,
            None,
            "business",
            0,
        )
        .unwrap();

        assert_eq!(customer.name, "Aline Mukamana");
        assert_eq!(customer.phone.as_deref(), Some("+250 788 000 000"));
        assert_eq!(customer.address.as_deref(), Some("Kigali"));
        assert_eq!(customer.customer_type, "individual");
        assert_eq!(customer.credit_limit, 1_250_500);
        assert_eq!(list_customers(&conn, &business.id).unwrap().len(), 1);
        assert_eq!(list_customers(&conn, &other_business.id).unwrap().len(), 1);
        assert!(matches!(
            update_customer(
                &conn,
                &other_business.id,
                &customer.id,
                "Unauthorized edit",
                None,
                None,
                "individual",
                0,
                true
            ),
            Err(DbError::InvalidInput(_))
        ));

        let updated = update_customer(
            &conn,
            &business.id,
            &customer.id,
            "Aline M.",
            Some("  "),
            None,
            "business",
            2_500_000,
            false,
        )
        .unwrap();
        assert_eq!(updated.name, "Aline M.");
        assert_eq!(updated.phone, None);
        assert_eq!(updated.customer_type, "business");
        assert_eq!(updated.credit_limit, 2_500_000);
        assert!(!updated.is_active);
        assert_eq!(
            list_customers(&conn, &business.id).unwrap()[0].id,
            customer.id
        );
    }

    #[test]
    fn customer_creation_rejects_invalid_name_business_type_limit_and_field_lengths() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();

        assert!(matches!(
            create_customer(&conn, &business.id, " ", None, None, "individual", 0),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_customer(
                &conn,
                "missing-business",
                "Customer",
                None,
                None,
                "individual",
                0
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_customer(&conn, &business.id, "Customer", None, None, "supplier", 0),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_customer(
                &conn,
                &business.id,
                "Customer",
                None,
                None,
                "individual",
                -1
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_customer(
                &conn,
                &business.id,
                "Customer",
                Some(&"x".repeat(81)),
                None,
                "individual",
                0
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_customer(
                &conn,
                &business.id,
                "Customer",
                None,
                Some(&"x".repeat(501)),
                "individual",
                0
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(list_customers(&conn, &business.id).unwrap().is_empty());
    }

    #[test]
    fn expenses_are_business_scoped_and_create_audit_records() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let (_, owner) = test_payment_account(&conn, &business.id, "mtn_mobile_money").unwrap();
        let employee = create_employee(
            &conn,
            &business.id,
            &owner,
            "manager.one",
            "Manager One",
            "manager",
            "$argon2id$manager-hash",
        )
        .unwrap();
        let manager = AuthUser {
            id: employee.id,
            business_id: employee.business_id,
            username: employee.username,
            full_name: employee.full_name,
            role: employee.role,
        };
        let (account_id, _) =
            test_payment_account(&conn, &business.id, "mtn_mobile_money").unwrap();

        let expense = create_expense_as_user(
            &conn,
            &business.id,
            "electricity",
            125_050,
            "2026-11-02",
            "mtn_mobile_money",
            "  Monthly electricity bill  ",
            "  Aline Uwase  ",
            &account_id,
            &manager,
        )
        .unwrap();
        create_expense(
            &conn,
            &other_business.id,
            "rent",
            500_000,
            "2026-11-01",
            "bank_transfer",
            "November rent",
            "Manager",
        )
        .unwrap();

        assert_eq!(expense.category, "electricity");
        assert_eq!(expense.amount, 125_050);
        assert_eq!(expense.description, "Monthly electricity bill");
        assert_eq!(expense.responsible_name, "Aline Uwase");
        let expenses = list_expenses(&conn, &business.id, 0, 20).unwrap();
        assert_eq!(expenses.total_count, 1);
        assert_eq!(expenses.items.len(), 1);
        assert_eq!(expenses.items[0].id, expense.id);
        assert_eq!(
            list_expenses(&conn, &other_business.id, 0, 20)
                .unwrap()
                .total_count,
            1
        );
        let audit_record: (String, String, String, String) = conn
            .query_row(
                "SELECT action, entity_type, entity_id, user_id
                 FROM audit_logs WHERE entity_id = ?1",
                params![expense.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            audit_record,
            (
                "expense_created".to_string(),
                "expense".to_string(),
                expense.id,
                manager.id.clone()
            )
        );
        assert_eq!(
            cash_account_balance(&conn, &business.id, &account_id).unwrap(),
            1_000_000_000 - 125_050
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM cash_movements
                 WHERE business_id = ?1 AND source_type = 'expense_payment'
                   AND responsible_user_id = ?2",
                params![business.id, manager.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn expense_recording_rejects_unopened_mismatched_and_insufficient_accounts() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let (_, owner) = test_payment_account(&conn, &business.id, "cash").unwrap();
        let (cash_account_id, _) = test_payment_account(&conn, &business.id, "cash").unwrap();
        let (bank_account_id, _) =
            test_payment_account(&conn, &business.id, "bank_transfer").unwrap();

        assert!(
            create_expense_as_user(
                &conn,
                &business.id,
                "rent",
                100,
                "2026-11-02",
                "cash",
                "Rent",
                "Manager",
                &bank_account_id,
                &owner,
            )
            .is_err()
        );
        assert!(
            create_expense_as_user(
                &conn,
                &business.id,
                "rent",
                1_000_000_001,
                "2026-11-02",
                "cash",
                "Rent",
                "Manager",
                &cash_account_id,
                &owner,
            )
            .is_err()
        );
        let (expense_count, movement_count): (i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM expenses WHERE business_id = ?1),
                        (SELECT COUNT(*) FROM cash_movements
                         WHERE business_id = ?1 AND source_type = 'expense_payment')",
                params![business.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(expense_count, 0);
        assert_eq!(movement_count, 0);
        assert_eq!(
            cash_account_balance(&conn, &business.id, &cash_account_id).unwrap(),
            1_000_000_000
        );
    }

    #[test]
    fn daily_business_report_reconciles_sales_credit_costs_expenses_and_balances() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let customer = create_customer(
            &conn,
            &business.id,
            "Aline Mukamana",
            None,
            None,
            "individual",
            20_000,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "REPORT-DAILY-01");
        set_opening_stock(&conn, &business.id, &product.id, 2_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 15_000).unwrap();

        let sale = checkout_sale_with_payments(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id,
                quantity: 1_000,
                selling_price: None,
                price_change_reason: None,
            }],
            Some(&customer.id),
            &[SalePaymentInput {
                payment_method: "cash".to_string(),
                amount: 5_000,
                account_id: String::new(),
            }],
        )
        .unwrap();
        let report_date = chrono::DateTime::parse_from_rfc3339(&sale.created_at)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d")
            .to_string();
        let debt = list_customer_debts(&conn, &business.id).unwrap().remove(0);
        record_customer_debt_payment(&conn, &business.id, &debt.id, "mtn_mobile_money", 6_000)
            .unwrap();
        create_expense(
            &conn,
            &business.id,
            "electricity",
            500,
            &report_date,
            "cash",
            "Daily report test expense",
            "Manager",
        )
        .unwrap();
        create_expense(
            &conn,
            &other_business.id,
            "rent",
            900,
            &report_date,
            "cash",
            "Other business expense",
            "Manager",
        )
        .unwrap();

        let report = daily_business_report(&conn, &business.id, &report_date).unwrap();

        assert_eq!(report.business_name, "Hardware shop");
        assert_eq!(report.report_date, report_date);
        assert_eq!(report.money_received, 11_000);
        assert_eq!(report.total_inflow, 15_000);
        assert_eq!(report.total_outflow, 500);
        assert_eq!(report.net_position, 14_500);
        assert_eq!(report.loan_taken, 10_000);
        assert_eq!(report.loan_paid, 6_000);
        assert_eq!(report.bank, 0);
        assert_eq!(report.momo, 6_000);
        assert_eq!(report.opening_customer_outstanding, 0);
        assert_eq!(report.closing_customer_outstanding, 4_000);
        assert_eq!(
            report.calculated_physical_cash,
            report.physical_closing_cash
        );
        assert_eq!(report.cash_reconciliation_difference, 0);
        assert_eq!(report.ending_cash, report.opening_physical_cash + 14_500);
        assert_eq!(report.cash_received, 5_000);
        assert_eq!(report.cash_expenses, 500);
        assert_eq!(report.cash_transferred_to_bank, 0);
        assert_eq!(report.cash_transferred_to_mobile_money, 0);
        assert_eq!(report.other_verified_cash_outflows, 0);
        assert_eq!(
            report.physical_closing_cash,
            report.opening_physical_cash + 5_000 - 500
        );
        assert_eq!(report.sale_count, 1);
        assert_eq!(report.sales_total, 15_000);
        assert_eq!(
            report.sales_by_payment_method,
            vec![DailyAmountBreakdown {
                category: "cash".to_string(),
                amount: 5_000,
            }]
        );
        assert_eq!(report.credit_sales, 10_000);
        assert_eq!(report.customer_repayments, 6_000);
        assert_eq!(report.expenses_total, 500);
        assert_eq!(
            report.expenses_by_category,
            vec![DailyAmountBreakdown {
                category: "electricity".to_string(),
                amount: 500,
            }]
        );
        assert_eq!(report.cost_of_goods_sold, 10_000);
        assert_eq!(report.gross_profit, 5_000);
        assert_eq!(report.operating_result, 4_500);
        assert_eq!(report.sale_line_count, 1);
        assert_eq!(report.customer_outstanding, 4_000);
        assert_eq!(report.supplier_outstanding, 0);
        assert!(report.stock_adjustment_counts.is_empty());
    }

    #[test]
    fn daily_business_report_rejects_invalid_date_and_unknown_business() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();

        assert!(matches!(
            daily_business_report(&conn, &business.id, "2026-02-30"),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            daily_business_report(&conn, "missing-business", "2026-02-28"),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn expense_creation_rejects_invalid_category_amount_date_payment_and_text() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let valid = (
            "rent",
            100_i64,
            "2026-11-02",
            "cash",
            "November shop rent",
            "Manager",
        );
        let attempts = [
            (
                "inventory_purchase",
                valid.1,
                valid.2,
                valid.3,
                valid.4,
                valid.5,
            ),
            (valid.0, 0, valid.2, valid.3, valid.4, valid.5),
            (valid.0, valid.1, "2026-02-30", valid.3, valid.4, valid.5),
            (valid.0, valid.1, valid.2, "credit", valid.4, valid.5),
            (valid.0, valid.1, valid.2, valid.3, " ", valid.5),
            (valid.0, valid.1, valid.2, valid.3, valid.4, " "),
        ];
        for (category, amount, date, method, description, responsible) in attempts {
            assert!(matches!(
                create_expense(
                    &conn,
                    &business.id,
                    category,
                    amount,
                    date,
                    method,
                    description,
                    responsible
                ),
                Err(DbError::InvalidInput(_))
            ));
        }
        assert!(matches!(
            list_expenses(&conn, &business.id, -1, 20),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            list_expenses(&conn, &business.id, 0, 201),
            Err(DbError::InvalidInput(_))
        ));
        assert!(
            list_expenses(&conn, &business.id, 0, 20)
                .unwrap()
                .items
                .is_empty()
        );
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action = 'expense_created'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 0);
    }

    #[test]
    fn supplier_creation_rejects_invalid_name_business_and_field_lengths() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();

        assert!(matches!(
            create_supplier(&conn, &business.id, " ", None, None, None, None, None),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_supplier(
                &conn,
                "missing-business",
                "Supplier",
                None,
                None,
                None,
                None,
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            create_supplier(
                &conn,
                &business.id,
                "Supplier",
                None,
                Some(&"x".repeat(81)),
                None,
                None,
                None
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(list_suppliers(&conn, &business.id).unwrap().is_empty());
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
    fn checkout_sale_records_receipt_payment_and_stock_movement_atomically() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "POS-SALE-01");
        set_opening_stock(&conn, &business.id, &product.id, 2_000, None).unwrap();
        let current_selling_price = 15_000;
        update_base_selling_price(&conn, &business.id, &product.id, current_selling_price).unwrap();

        let receipt = checkout_sale(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id.clone(),
                quantity: 1_000,
                selling_price: None,
                price_change_reason: None,
            }],
            "cash",
        )
        .unwrap();
        let quantity: i64 = conn
            .query_row(
                "SELECT quantity_on_hand FROM inventory_balances WHERE product_id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();
        let payment: (String, i64) = conn
            .query_row(
                "SELECT payment_method, amount FROM sale_payments WHERE sale_id = ?1",
                params![receipt.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let movement: (i64, i64, i64, String) = conn
            .query_row(
                "SELECT quantity_delta, unit_cost, total_cost, reference_id
                 FROM stock_movements WHERE reference_type = 'sale' AND reference_id = ?1",
                params![receipt.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert!(receipt.transaction_number.starts_with("SALE-"));
        assert_eq!(receipt.total, current_selling_price);
        assert_eq!(receipt.items.len(), 1);
        assert_eq!(receipt.items[0].product_name, product.name);
        assert_eq!(receipt.items[0].unit_price, current_selling_price);
        assert_eq!(quantity, 1_000);
        assert_eq!(payment, ("cash".to_string(), current_selling_price));
        assert_eq!(
            movement,
            (
                -1_000,
                product.purchase_price,
                product.purchase_price,
                receipt.id
            )
        );
    }

    #[test]
    fn sales_history_is_scoped_searchable_paginated_and_uses_sale_snapshots() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "HISTORY-SKU-01");
        let other_product = create_test_product(&conn, &other_business.id, "HISTORY-SKU-02");
        set_opening_stock(&conn, &business.id, &product.id, 3_000, None).unwrap();
        set_opening_stock(&conn, &other_business.id, &other_product.id, 1_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 10_000).unwrap();
        update_base_selling_price(&conn, &other_business.id, &other_product.id, 10_000).unwrap();
        let sale_item = |product_id: &str| SaleItemInput {
            product_id: product_id.to_string(),
            quantity: 1_000,
            selling_price: None,
            price_change_reason: None,
        };
        let sale = checkout_sale_with_payments(
            &conn,
            &business.id,
            &[sale_item(&product.id)],
            None,
            &[
                SalePaymentInput {
                    payment_method: "cash".to_string(),
                    amount: 6_000,
                    account_id: String::new(),
                },
                SalePaymentInput {
                    payment_method: "mtn_mobile_money".to_string(),
                    amount: 4_000,
                    account_id: String::new(),
                },
            ],
        )
        .unwrap();
        checkout_sale(&conn, &business.id, &[sale_item(&product.id)], "cash").unwrap();
        checkout_sale(
            &conn,
            &other_business.id,
            &[sale_item(&other_product.id)],
            "cash",
        )
        .unwrap();

        let date = sale.created_at[..10].to_string();
        let first_page =
            list_sales(&conn, &business.id, None, Some(&date), Some(&date), 0, 1).unwrap();
        assert_eq!(first_page.total_count, 2);
        assert_eq!(first_page.completed_sales_total, 20_000);
        assert_eq!(first_page.collected_at_sale_total, 20_000);
        assert_eq!(first_page.items.len(), 1);
        assert_eq!(first_page.items[0].total, 10_000);
        assert_eq!(first_page.items[0].amount_paid, 10_000);
        assert_eq!(first_page.items[0].balance_due, 0);
        assert_eq!(first_page.items[0].item_quantity, 1_000);
        assert_eq!(first_page.items[0].sold_by.as_deref(), Some("Test Owner"));
        assert_eq!(first_page.items[0].items[0].product_name, product.name);

        let matched = list_sales(
            &conn,
            &business.id,
            Some("history-sku-01"),
            Some(&date),
            Some(&date),
            0,
            20,
        )
        .unwrap();
        assert_eq!(matched.total_count, 2);
        assert_eq!(matched.completed_sales_total, 20_000);
        assert_eq!(matched.collected_at_sale_total, 20_000);
        let split_sale = matched
            .items
            .iter()
            .find(|entry| entry.id == sale.id)
            .unwrap();
        assert_eq!(split_sale.payments.len(), 2);
        assert_eq!(
            split_sale
                .payments
                .iter()
                .map(|payment| payment.amount)
                .sum::<i64>(),
            split_sale.total
        );
        let unmatched = list_sales(
            &conn,
            &business.id,
            Some("HISTORY-SKU-02"),
            None,
            None,
            0,
            20,
        )
        .unwrap();
        assert_eq!(unmatched.total_count, 0);
        assert!(matches!(
            list_sales(&conn, &business.id, None, None, None, -1, 20),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            list_sales(
                &conn,
                &business.id,
                None,
                Some("2026-11-03"),
                Some("2026-11-02"),
                0,
                20
            ),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn credit_sale_tracks_partial_payment_and_repayments_without_creating_sales() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let customer = create_customer(
            &conn,
            &business.id,
            "Aline Mukamana",
            None,
            None,
            "individual",
            20_000,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "POS-CREDIT-01");
        set_opening_stock(&conn, &business.id, &product.id, 2_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 15_000).unwrap();

        let receipt = checkout_sale_with_payments(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id.clone(),
                quantity: 1_000,
                selling_price: None,
                price_change_reason: None,
            }],
            Some(&customer.id),
            &[
                SalePaymentInput {
                    payment_method: "cash".to_string(),
                    amount: 3_000,
                    account_id: String::new(),
                },
                SalePaymentInput {
                    payment_method: "mtn_mobile_money".to_string(),
                    amount: 2_000,
                    account_id: String::new(),
                },
            ],
        )
        .unwrap();

        assert_eq!(receipt.customer_name.as_deref(), Some("Aline Mukamana"));
        assert_eq!(receipt.total, 15_000);
        assert_eq!(receipt.amount_paid, 5_000);
        assert_eq!(receipt.balance_due, 10_000);
        assert_eq!(receipt.payment_method, "split");
        assert_eq!(receipt.payments.len(), 2);
        let debt = list_customer_debts(&conn, &business.id).unwrap().remove(0);
        assert_eq!(debt.customer_id, customer.id);
        assert_eq!(debt.sale_id, receipt.id);
        assert_eq!(debt.original_amount, 15_000);
        assert_eq!(debt.amount_paid, 5_000);
        assert_eq!(debt.balance_due, 10_000);
        assert_eq!(
            list_customers(&conn, &business.id).unwrap()[0].outstanding_balance,
            10_000
        );

        let first_payment =
            record_customer_debt_payment(&conn, &business.id, &debt.id, "mtn_mobile_money", 6_000)
                .unwrap();
        assert_eq!(first_payment.balance_due, 4_000);
        let final_payment =
            record_customer_debt_payment(&conn, &business.id, &debt.id, "cash", 4_000).unwrap();
        assert_eq!(final_payment.balance_due, 0);
        assert_eq!(
            list_customer_debts(&conn, &business.id).unwrap()[0].balance_due,
            0
        );
        assert_eq!(
            list_customers(&conn, &business.id).unwrap()[0].outstanding_balance,
            0
        );
        let (cash_movement_count, cash_inflow_total): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount), 0)
                 FROM cash_movements
                 WHERE business_id = ?1 AND direction = 'in'",
                params![business.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(cash_movement_count, 4);
        assert_eq!(cash_inflow_total, 15_000);
        let sales_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sales_count, 1);
    }

    #[test]
    fn customer_statement_reconciles_sales_initial_payments_and_later_repayments() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let customer = create_customer(
            &conn,
            &business.id,
            "Aline Mukamana",
            None,
            None,
            "individual",
            20_000,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "POS-STATEMENT-01");
        set_opening_stock(&conn, &business.id, &product.id, 3_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 15_000).unwrap();
        let item = SaleItemInput {
            product_id: product.id.clone(),
            quantity: 1_000,
            selling_price: None,
            price_change_reason: None,
        };
        let receipt = checkout_sale_with_payments(
            &conn,
            &business.id,
            &[item],
            Some(&customer.id),
            &[SalePaymentInput {
                payment_method: "cash".to_string(),
                amount: 5_000,
                account_id: String::new(),
            }],
        )
        .unwrap();
        let debt = list_customer_debts(&conn, &business.id).unwrap().remove(0);
        record_customer_debt_payment(&conn, &business.id, &debt.id, "mtn_mobile_money", 1_000)
            .unwrap();

        let statement = customer_statement(&conn, &business.id, &customer.id).unwrap();
        assert_eq!(statement.customer.id, customer.id);
        assert_eq!(statement.total_sales, 15_000);
        assert_eq!(statement.total_payments, 6_000);
        assert_eq!(statement.outstanding_balance, 9_000);
        assert_eq!(statement.entries.len(), 3);
        assert_eq!(statement.entries[0].entry_type, "sale");
        assert_eq!(
            statement.entries[0].transaction_number,
            receipt.transaction_number
        );
        assert_eq!(statement.entries[0].amount, 15_000);
        assert_eq!(statement.entries[0].balance_after, 15_000);
        assert_eq!(statement.entries[1].entry_type, "sale_payment");
        assert_eq!(statement.entries[1].payment_method.as_deref(), Some("cash"));
        assert_eq!(statement.entries[1].amount, 5_000);
        assert_eq!(statement.entries[1].balance_after, 10_000);
        assert_eq!(statement.entries[2].entry_type, "repayment");
        assert_eq!(
            statement.entries[2].payment_method.as_deref(),
            Some("mtn_mobile_money")
        );
        assert_eq!(statement.entries[2].amount, 1_000);
        assert_eq!(statement.entries[2].balance_after, 9_000);
        assert!(matches!(
            customer_statement(&conn, "other-business", &customer.id),
            Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
                | Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn credit_sale_enforces_customer_and_credit_limit_before_stock_changes() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let customer = create_customer(
            &conn,
            &business.id,
            "Aline Mukamana",
            None,
            None,
            "individual",
            5_000,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "POS-CREDIT-LIMIT-01");
        set_opening_stock(&conn, &business.id, &product.id, 2_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 15_000).unwrap();
        let item = SaleItemInput {
            product_id: product.id.clone(),
            quantity: 1_000,
            selling_price: None,
            price_change_reason: None,
        };

        assert!(matches!(
            checkout_sale_with_payments(&conn, &business.id, &[item.clone()], None, &[]),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            checkout_sale_with_payments(
                &conn,
                &business.id,
                &[SaleItemInput {
                    product_id: product.id.clone(),
                    quantity: 1_000,
                    selling_price: None,
                    price_change_reason: None,
                }],
                Some(&customer.id),
                &[],
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            checkout_sale_with_payments(
                &conn,
                &business.id,
                &[SaleItemInput {
                    product_id: product.id.clone(),
                    quantity: 1_000,
                    selling_price: None,
                    price_change_reason: None,
                }],
                Some("customer-from-another-business"),
                &[],
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            checkout_sale_with_payments(
                &conn,
                &other_business.id,
                &[item],
                Some(&customer.id),
                &[],
            ),
            Err(DbError::InvalidInput(_))
        ));

        let sales_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
            .unwrap();
        let quantity: i64 = conn
            .query_row(
                "SELECT quantity_on_hand FROM inventory_balances WHERE product_id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sales_count, 0);
        assert_eq!(quantity, 2_000);
    }

    #[test]
    fn credit_limit_includes_customer_debt_from_prior_sales() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let customer = create_customer(
            &conn,
            &business.id,
            "Aline Mukamana",
            None,
            None,
            "individual",
            20_000,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "POS-CREDIT-CUMULATIVE-01");
        set_opening_stock(&conn, &business.id, &product.id, 3_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 15_000).unwrap();
        let item = SaleItemInput {
            product_id: product.id.clone(),
            quantity: 1_000,
            selling_price: None,
            price_change_reason: None,
        };

        checkout_sale_with_payments(
            &conn,
            &business.id,
            &[item.clone()],
            Some(&customer.id),
            &[],
        )
        .unwrap();
        assert_eq!(
            list_customers(&conn, &business.id).unwrap()[0].outstanding_balance,
            15_000
        );
        assert!(matches!(
            checkout_sale_with_payments(&conn, &business.id, &[item], Some(&customer.id), &[],),
            Err(DbError::InvalidInput(_))
        ));

        let sales_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
            .unwrap();
        let quantity: i64 = conn
            .query_row(
                "SELECT quantity_on_hand FROM inventory_balances WHERE product_id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sales_count, 1);
        assert_eq!(quantity, 2_000);
    }

    #[test]
    fn customer_debt_repayments_reject_overpayment_invalid_methods_and_other_businesses() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let customer = create_customer(
            &conn,
            &business.id,
            "Aline Mukamana",
            None,
            None,
            "individual",
            20_000,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "POS-CREDIT-REPAY-01");
        set_opening_stock(&conn, &business.id, &product.id, 2_000, None).unwrap();
        update_base_selling_price(&conn, &business.id, &product.id, 15_000).unwrap();
        checkout_sale_with_payments(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id.clone(),
                quantity: 1_000,
                selling_price: None,
                price_change_reason: None,
            }],
            Some(&customer.id),
            &[],
        )
        .unwrap();
        let debt = list_customer_debts(&conn, &business.id).unwrap().remove(0);

        assert!(matches!(
            record_customer_debt_payment(&conn, &business.id, &debt.id, "credit", 1),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            record_customer_debt_payment(&conn, &business.id, &debt.id, "cash", 0),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            record_customer_debt_payment(&conn, &business.id, &debt.id, "cash", 15_001),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            record_customer_debt_payment(&conn, "other-business", &debt.id, "cash", 1),
            Err(DbError::InvalidInput(_))
        ));
        assert_eq!(
            list_customer_debts(&conn, &business.id).unwrap()[0].balance_due,
            15_000
        );
    }

    #[test]
    fn checkout_sale_records_overridden_price_and_reason_without_changing_product_price() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "POS-SALE-PRICE-01");
        set_opening_stock(&conn, &business.id, &product.id, 2_000, None).unwrap();
        let listed_price = 15_000;
        update_base_selling_price(&conn, &business.id, &product.id, listed_price).unwrap();
        let override_price = 13_500;

        let receipt = checkout_sale(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id.clone(),
                quantity: 1_000,
                selling_price: Some(override_price),
                price_change_reason: Some("  Customer negotiated  ".to_string()),
            }],
            "cash",
        )
        .unwrap();
        let saved_prices: (i64, i64, Option<String>) = conn
            .query_row(
                "SELECT listed_unit_price, unit_price, price_change_reason
                 FROM sale_items WHERE sale_id = ?1",
                params![receipt.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let current_product_price: i64 = conn
            .query_row(
                "SELECT selling_price FROM products WHERE id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(receipt.total, override_price);
        assert_eq!(receipt.items[0].listed_unit_price, listed_price);
        assert_eq!(receipt.items[0].unit_price, override_price);
        assert_eq!(
            receipt.items[0].price_change_reason.as_deref(),
            Some("Customer negotiated")
        );
        assert_eq!(
            saved_prices,
            (
                listed_price,
                override_price,
                Some("Customer negotiated".to_string())
            )
        );
        assert_eq!(current_product_price, listed_price);
    }

    #[test]
    fn fifo_valuation_uses_remaining_purchase_cost_and_requires_reason_for_losses() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "FIFO-SALE-01");
        set_opening_stock(&conn, &business.id, &product.id, 1_000, None).unwrap();
        receive_stock(
            &conn,
            &business.id,
            &product.id,
            1_000,
            20_000,
            "FIFO-RECEIPT-01",
            None,
        )
        .unwrap();

        let inventory = list_inventory(&conn, &business.id, Some("FIFO-SALE-01"), 0, 20).unwrap();
        assert_eq!(inventory.items[0].stock_value, 30_000);
        assert_eq!(inventory.items[0].minimum_selling_price, 20_000);
        assert_eq!(inventory.total_stock_value, 30_000);
        assert!(update_base_selling_price(&conn, &business.id, &product.id, 19_999).is_err());

        let first_sale = checkout_sale(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id.clone(),
                quantity: 1_000,
                selling_price: None,
                price_change_reason: None,
            }],
            "cash",
        )
        .unwrap();
        assert_eq!(first_sale.total, 12_000);
        let remaining_inventory =
            list_inventory(&conn, &business.id, Some("FIFO-SALE-01"), 0, 20).unwrap();
        assert_eq!(remaining_inventory.items[0].stock_value, 20_000);

        assert!(matches!(
            checkout_sale(
                &conn,
                &business.id,
                &[SaleItemInput {
                    product_id: product.id.clone(),
                    quantity: 1_000,
                    selling_price: None,
                    price_change_reason: None,
                }],
                "cash"
            ),
            Err(DbError::InvalidInput(message)) if message.contains("BELOW_FIFO_COST:")
        ));
        let after_rejected_sale =
            list_inventory(&conn, &business.id, Some("FIFO-SALE-01"), 0, 20).unwrap();
        assert_eq!(after_rejected_sale.items[0].quantity_on_hand, 1_000);
        assert_eq!(after_rejected_sale.items[0].stock_value, 20_000);

        let loss_sale = checkout_sale(
            &conn,
            &business.id,
            &[SaleItemInput {
                product_id: product.id.clone(),
                quantity: 1_000,
                selling_price: None,
                price_change_reason: Some("Approved clearance".to_string()),
            }],
            "cash",
        )
        .unwrap();
        let (unit_cost, total_cost, reason): (i64, i64, Option<String>) = conn
            .query_row(
                "SELECT unit_cost, total_cost, price_change_reason
                 FROM sale_items WHERE sale_id = ?1",
                params![loss_sale.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(unit_cost, 20_000);
        assert_eq!(total_cost, 20_000);
        assert_eq!(reason.as_deref(), Some("Approved clearance"));
        assert_eq!(
            list_inventory(&conn, &business.id, Some("FIFO-SALE-01"), 0, 20)
                .unwrap()
                .items[0]
                .stock_value,
            0
        );
    }

    #[test]
    fn checkout_sale_rejects_insufficient_stock_and_invalid_payment_without_writes() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "POS-SALE-02");
        set_opening_stock(&conn, &business.id, &product.id, 1_000, None).unwrap();
        let item = SaleItemInput {
            product_id: product.id.clone(),
            quantity: 2_000,
            selling_price: None,
            price_change_reason: None,
        };

        assert!(matches!(
            checkout_sale(&conn, &business.id, &[item], "cash"),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            checkout_sale(
                &conn,
                &business.id,
                &[SaleItemInput {
                    product_id: product.id.clone(),
                    quantity: 1_000,
                    selling_price: Some(10_000),
                    price_change_reason: None,
                }],
                "cash"
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            checkout_sale(
                &conn,
                &business.id,
                &[SaleItemInput {
                    product_id: product.id.clone(),
                    quantity: 1_000,
                    selling_price: Some(10_000),
                    price_change_reason: Some("x".repeat(501)),
                }],
                "cash"
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert!(matches!(
            checkout_sale(
                &conn,
                &business.id,
                &[SaleItemInput {
                    product_id: product.id.clone(),
                    quantity: 1_000,
                    selling_price: None,
                    price_change_reason: None,
                }],
                "credit"
            ),
            Err(DbError::InvalidInput(_))
        ));
        let sales_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
            .unwrap();
        let quantity: i64 = conn
            .query_row(
                "SELECT quantity_on_hand FROM inventory_balances WHERE product_id = ?1",
                params![product.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(sales_count, 0);
        assert_eq!(quantity, 1_000);
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
        assert_eq!(inventory.items[0].minimum_selling_price, 10_000);
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
        assert_eq!(inventory.items[0].minimum_selling_price, 10_000);
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
    fn purchase_invoice_does_not_add_stock_and_duplicate_requires_authorization() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let supplier = create_supplier(
            &conn,
            &business.id,
            "Kigali Building Supplies",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "PURCHASE-INVOICE-01");
        let items = [PurchaseItemInput {
            product_id: product.id.clone(),
            ordered_quantity: 5_000,
            unit_cost: 20_000,
        }];

        let purchase = create_purchase(
            &conn,
            &business.id,
            &supplier.id,
            " INV-2026-001 ",
            &items,
            false,
        )
        .unwrap();
        assert_eq!(purchase.status, "pending");
        assert_eq!(purchase.supplier_invoice_number, "INV-2026-001");
        assert_eq!(purchase.subtotal, 100_000);
        assert_eq!(
            list_inventory(&conn, &business.id, Some("PURCHASE-INVOICE-01"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            0
        );
        assert!(matches!(
            create_purchase(
                &conn,
                &business.id,
                &supplier.id,
                "inv-2026-001",
                &items,
                false
            ),
            Err(DbError::InvalidInput(message)) if message.contains("DUPLICATE_PURCHASE_INVOICE:")
        ));

        let duplicate = create_purchase(
            &conn,
            &business.id,
            &supplier.id,
            "INV-2026-001",
            &items,
            true,
        )
        .unwrap();
        assert!(duplicate.duplicate_authorized);
        assert_eq!(
            duplicate.duplicate_of_purchase_id.as_deref(),
            Some(purchase.id.as_str())
        );
        assert_eq!(list_purchases(&conn, &business.id).unwrap().len(), 2);
    }

    #[test]
    fn supplier_payments_track_partial_and_full_invoice_balances() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let supplier = create_supplier(
            &conn,
            &business.id,
            "Kigali Building Supplies",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "SUPPLIER-PAYMENT-01");
        let purchase = create_purchase(
            &conn,
            &business.id,
            &supplier.id,
            "INV-PAYMENT-01",
            &[PurchaseItemInput {
                product_id: product.id.clone(),
                ordered_quantity: 2_000,
                unit_cost: 50_000,
            }],
            false,
        )
        .unwrap();
        assert_eq!(purchase.subtotal, 100_000);
        assert_eq!(purchase.amount_paid, 0);
        assert_eq!(purchase.balance_due, purchase.subtotal);

        let partial = record_purchase_payment(
            &conn,
            &business.id,
            &purchase.id,
            40_000,
            "mtn_mobile_money",
        )
        .unwrap();
        assert_eq!(partial.amount_paid, 40_000);
        assert_eq!(partial.balance_due, 60_000);
        assert_eq!(partial.payments.len(), 1);
        assert_eq!(partial.payments[0].payment_method, "mtn_mobile_money");
        assert_eq!(partial.status, "pending");

        let paid =
            record_purchase_payment(&conn, &business.id, &purchase.id, 60_000, "cash").unwrap();
        assert_eq!(paid.amount_paid, 100_000);
        assert_eq!(paid.balance_due, 0);
        assert_eq!(paid.payments.len(), 2);
        assert_eq!(paid.status, "pending");
        let (movement_count, supplier_outflow_total): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount), 0)
                 FROM cash_movements
                 WHERE business_id = ?1 AND source_type = 'purchase_payment'",
                params![business.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(movement_count, 2);
        assert_eq!(supplier_outflow_total, 100_000);
        assert_eq!(
            list_inventory(&conn, &business.id, Some("SUPPLIER-PAYMENT-01"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            0
        );
    }

    #[test]
    fn supplier_payment_cannot_spend_more_than_the_verified_account_balance() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "owner",
            "Aline Uwase",
            "$argon2id$owner-hash",
        )
        .unwrap();
        let supplier = create_supplier(
            &conn,
            &business.id,
            "Kigali Building Supplies",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "SUPPLIER-FUNDS-01");
        let purchase = create_purchase(
            &conn,
            &business.id,
            &supplier.id,
            "INV-FUNDS-01",
            &[PurchaseItemInput {
                product_id: product.id,
                ordered_quantity: 1_000,
                unit_cost: 100_000,
            }],
            false,
        )
        .unwrap();
        let account =
            create_cash_account(&conn, &business.id, &owner, "cash_drawer", "Main drawer").unwrap();
        create_cash_account_opening(&conn, &business.id, &account.id, 50_000, &owner).unwrap();

        assert!(
            record_purchase_payment_as_user(
                &conn,
                &business.id,
                &purchase.id,
                60_000,
                "cash",
                &account.id,
                &owner,
            )
            .is_err()
        );
        assert_eq!(
            load_purchase(&conn, &business.id, &purchase.id)
                .unwrap()
                .amount_paid,
            0
        );
        assert_eq!(
            cash_account_balance(&conn, &business.id, &account.id).unwrap(),
            50_000
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM cash_movements
                 WHERE business_id = ?1 AND source_type = 'purchase_payment'",
                params![business.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn supplier_payments_reject_invalid_amount_method_and_business() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let other_business = create_business(&conn, "Other shop", None).unwrap();
        let supplier = create_supplier(
            &conn,
            &business.id,
            "Kigali Building Supplies",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let product = create_test_product(&conn, &business.id, "SUPPLIER-PAYMENT-02");
        let purchase = create_purchase(
            &conn,
            &business.id,
            &supplier.id,
            "INV-PAYMENT-02",
            &[PurchaseItemInput {
                product_id: product.id,
                ordered_quantity: 1_000,
                unit_cost: 50_000,
            }],
            false,
        )
        .unwrap();
        for (business_id, amount, method) in [
            (business.id.as_str(), 0, "cash"),
            (business.id.as_str(), 50_001, "cash"),
            (business.id.as_str(), 1, "unsupported"),
            (other_business.id.as_str(), 1, "cash"),
        ] {
            assert!(matches!(
                record_purchase_payment(&conn, business_id, &purchase.id, amount, method),
                Err(DbError::InvalidInput(_))
            ));
        }
        assert_eq!(
            load_purchase(&conn, &business.id, &purchase.id)
                .unwrap()
                .payments
                .len(),
            0
        );
    }

    #[test]
    fn purchase_delivery_is_atomic_and_supports_partial_receipts() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let supplier = create_supplier(
            &conn,
            &business.id,
            "Kigali Building Supplies",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let first = create_test_product(&conn, &business.id, "PURCHASE-RECEIPT-01");
        let second = create_test_product(&conn, &business.id, "PURCHASE-RECEIPT-02");
        let purchase = create_purchase(
            &conn,
            &business.id,
            &supplier.id,
            "INV-RECEIPT-01",
            &[
                PurchaseItemInput {
                    product_id: first.id.clone(),
                    ordered_quantity: 5_000,
                    unit_cost: 20_000,
                },
                PurchaseItemInput {
                    product_id: second.id.clone(),
                    ordered_quantity: 3_000,
                    unit_cost: 30_000,
                },
            ],
            false,
        )
        .unwrap();
        let first_item = purchase
            .items
            .iter()
            .find(|item| item.product_id == first.id)
            .unwrap();
        let second_item = purchase
            .items
            .iter()
            .find(|item| item.product_id == second.id)
            .unwrap();

        assert!(matches!(
            receive_purchase_delivery(
                &conn,
                &business.id,
                &purchase.id,
                &[
                    PurchaseReceiptItemInput {
                        purchase_item_id: first_item.id.clone(),
                        quantity: 2_000,
                    },
                    PurchaseReceiptItemInput {
                        purchase_item_id: second_item.id.clone(),
                        quantity: 4_000,
                    },
                ],
                None,
                None,
            ),
            Err(DbError::InvalidInput(_))
        ));
        assert_eq!(
            list_inventory(&conn, &business.id, Some("PURCHASE-RECEIPT-01"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            0
        );
        let receipt_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM purchase_receipts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(receipt_count, 0);

        let first_delivery = receive_purchase_delivery(
            &conn,
            &business.id,
            &purchase.id,
            &[PurchaseReceiptItemInput {
                purchase_item_id: first_item.id.clone(),
                quantity: 2_000,
            }],
            Some("DN-01"),
            Some("First partial delivery"),
        )
        .unwrap();
        assert_eq!(first_delivery.status, "partially_received");
        assert_eq!(first_delivery.total_cost, 40_000);
        assert_eq!(
            list_inventory(&conn, &business.id, Some("PURCHASE-RECEIPT-01"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            2_000
        );

        let final_delivery = receive_purchase_delivery(
            &conn,
            &business.id,
            &purchase.id,
            &[
                PurchaseReceiptItemInput {
                    purchase_item_id: first_item.id.clone(),
                    quantity: 3_000,
                },
                PurchaseReceiptItemInput {
                    purchase_item_id: second_item.id.clone(),
                    quantity: 3_000,
                },
            ],
            None,
            None,
        )
        .unwrap();
        assert_eq!(final_delivery.status, "received");
        assert_eq!(final_delivery.total_cost, 150_000);
        let updated = load_purchase(&conn, &business.id, &purchase.id).unwrap();
        assert_eq!(updated.status, "received");
        assert!(
            updated
                .items
                .iter()
                .all(|item| item.received_quantity == item.ordered_quantity)
        );
        assert_eq!(updated.deliveries.len(), 2);
        assert_eq!(list_purchases(&conn, &business.id).unwrap().len(), 1);
        assert_eq!(
            list_purchase_history(&conn, &business.id, None, None, None, 0, 20)
                .unwrap()
                .total_count,
            0
        );
        let owner = bootstrap_owner(
            &conn,
            Some(&business.id),
            None,
            "history.owner",
            "History Owner",
            "test-password-hash",
        )
        .unwrap();
        let account =
            create_cash_account(&conn, &business.id, &owner, "cash_drawer", "History drawer")
                .unwrap();
        create_cash_account_opening(&conn, &business.id, &account.id, 190_000, &owner).unwrap();
        record_purchase_payment_as_user(
            &conn,
            &business.id,
            &purchase.id,
            190_000,
            "cash",
            &account.id,
            &owner,
        )
        .unwrap();
        assert!(list_purchases(&conn, &business.id).unwrap().is_empty());
        let history =
            list_purchase_history(&conn, &business.id, Some("DN-01"), None, None, 0, 20).unwrap();
        assert_eq!(history.total_count, 1);
        assert_eq!(history.purchase_total, 190_000);
        assert_eq!(history.paid_total, 190_000);
        assert_eq!(history.balance_due_total, 0);
        assert_eq!(
            history.items[0]
                .items
                .iter()
                .map(|item| item.received_quantity)
                .sum::<i64>(),
            8_000
        );
        let first_delivery_log = updated
            .deliveries
            .iter()
            .find(|delivery| delivery.delivery_reference.as_deref() == Some("DN-01"))
            .unwrap();
        assert_eq!(first_delivery_log.total_cost, 40_000);
        assert_eq!(
            list_inventory(&conn, &business.id, Some("PURCHASE-RECEIPT-01"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            5_000
        );
        assert_eq!(
            list_inventory(&conn, &business.id, Some("PURCHASE-RECEIPT-02"), 0, 20)
                .unwrap()
                .items[0]
                .quantity_on_hand,
            3_000
        );
        assert_eq!(
            list_stock_movements(&conn, &business.id, Some(&first.id), 0, 20)
                .unwrap()
                .iter()
                .filter(|movement| movement.movement_type == "purchase_received")
                .count(),
            2
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
        assert_eq!(receipt.total_cost, Some(60_000));
        assert_eq!(receipt.previous_quantity, 5_000);
        assert_eq!(receipt.new_quantity, 8_000);
        assert_eq!(receipt.reference_type.as_deref(), Some("supplier_document"));
        assert_eq!(receipt.reference_id.as_deref(), Some("INV-2026-0042"));
        assert_eq!(receipt.reason.as_deref(), Some("Partial delivery"));
        assert_eq!(inventory.items[0].quantity_on_hand, 8_000);
        assert_eq!(inventory.items[0].minimum_selling_price, 20_000);
        assert_eq!(inventory.items[0].stock_value, 110_000);
        assert_eq!(movements.len(), 2);
        assert!(movements.iter().any(|movement| movement.id == opening.id));
        assert!(movements.iter().any(|movement| movement.id == receipt.id));
        assert_eq!(
            movements
                .iter()
                .find(|movement| movement.id == receipt.id)
                .unwrap()
                .total_cost,
            Some(60_000)
        );
    }

    #[test]
    fn receiving_can_update_selling_price_and_pos_uses_the_new_price() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-RECEIPT-PRICE-01");
        let receipt = receive_stock_with_selling_price(
            &conn,
            &business.id,
            &product.id,
            30_000,
            80_000,
            "INV-PRICE-01",
            None,
            Some(95_000),
        )
        .unwrap();
        let products =
            list_products(&conn, &business.id, Some("STOCK-RECEIPT-PRICE-01"), 0, 20).unwrap();
        let pos_products =
            search_pos_products(&conn, &business.id, Some("STOCK-RECEIPT-PRICE-01"), 0, 20)
                .unwrap();
        let price_history: Vec<(i64, Option<String>)> = conn
            .prepare(
                "SELECT price, effective_to FROM product_prices
                 WHERE product_id = ?1 AND price_type = 'selling' AND unit_id = ?2
                 ORDER BY effective_to IS NULL",
            )
            .unwrap()
            .query_map(params![product.id, product.base_unit_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let movement = list_stock_movements(&conn, &business.id, Some(&product.id), 0, 20)
            .unwrap()
            .into_iter()
            .find(|movement| movement.id == receipt.id)
            .unwrap();

        assert_eq!(products.items[0].purchase_price, product.purchase_price);
        assert_eq!(products.items[0].selling_price, 95_000);
        assert_eq!(pos_products.items[0].selling_price, 95_000);
        assert_eq!(price_history.len(), 2);
        assert!(
            price_history
                .iter()
                .any(|(price, ended)| { *price == product.selling_price && ended.is_some() })
        );
        assert!(
            price_history
                .iter()
                .any(|(price, ended)| { *price == 95_000 && ended.is_none() })
        );
        assert_eq!(movement.unit_cost, Some(80_000));
        assert_eq!(movement.total_cost, Some(2_400_000));
    }

    #[test]
    fn receiving_without_new_selling_price_keeps_current_price() {
        let conn = open_in_memory().unwrap();
        let business = create_business(&conn, "Hardware shop", None).unwrap();
        let product = create_test_product(&conn, &business.id, "STOCK-RECEIPT-PRICE-02");

        receive_stock_with_selling_price(
            &conn,
            &business.id,
            &product.id,
            1_000,
            50_000,
            "INV-PRICE-02",
            None,
            None,
        )
        .unwrap();
        let updated =
            list_products(&conn, &business.id, Some("STOCK-RECEIPT-PRICE-02"), 0, 20).unwrap();

        assert_eq!(updated.items[0].selling_price, product.selling_price);
        assert_eq!(updated.items[0].purchase_price, product.purchase_price);
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
