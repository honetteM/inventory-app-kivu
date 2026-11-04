mod db;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand_core::OsRng;
use std::fs;
use std::path::Path;
use tauri::Manager;

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("could not secure the password: {error}"))
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), String> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|error| format!("stored password hash is invalid: {error}"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| "business, username or password is incorrect".to_string())
}

fn normalize_recovery_answer(answer: &str) -> String {
    answer.trim().to_ascii_lowercase()
}

fn validate_recovery_input(value: &str, label: &str) -> Result<(), String> {
    let length = value.trim().chars().count();
    if !(3..=128).contains(&length) {
        return Err(format!("{label} must be 3-128 characters long"));
    }
    Ok(())
}

#[tauri::command]
fn auth_business_status(
    state: tauri::State<db::DbState>,
) -> Result<Vec<db::AuthBusinessStatus>, String> {
    state
        .with_public(db::auth_business_status)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn auth_bootstrap_owner(
    state: tauri::State<db::DbState>,
    business_id: Option<String>,
    new_business_name: Option<String>,
    username: String,
    full_name: String,
    password: String,
    recovery_code: String,
    recovery_question_one: String,
    recovery_answer_one: String,
    recovery_question_two: String,
    recovery_answer_two: String,
) -> Result<db::AuthUser, String> {
    if password.chars().count() < 12 || password.chars().count() > 128 {
        return Err("password must be 12-128 characters long".to_string());
    }

    validate_recovery_input(&recovery_code, "recovery code")?;
    validate_recovery_input(&recovery_answer_one, "first answer")?;
    validate_recovery_input(&recovery_answer_two, "second answer")?;
    let recovery_code_hash = hash_password(&recovery_code)?;
    let recovery_answer_one_hash = hash_password(&normalize_recovery_answer(&recovery_answer_one))?;
    let recovery_answer_two_hash = hash_password(&normalize_recovery_answer(&recovery_answer_two))?;
    let password_hash = hash_password(&password)?;
    let user = state
        .with_public(|conn| {
            db::bootstrap_owner_with_recovery(
                conn,
                business_id.as_deref(),
                new_business_name.as_deref(),
                &username,
                &full_name,
                &password_hash,
                Some(&recovery_code_hash),
                Some(&recovery_question_one),
                Some(&recovery_answer_one_hash),
                Some(&recovery_question_two),
                Some(&recovery_answer_two_hash),
            )
        })
        .map_err(|error| error.to_string())?;
    state
        .set_session(user.clone())
        .map_err(|error| error.to_string())?;
    Ok(user)
}

#[tauri::command]
fn auth_recovery_questions() -> Vec<db::RecoveryQuestion> {
    db::recovery_questions()
}

#[tauri::command]
fn auth_recovery_prompts(
    state: tauri::State<db::DbState>,
    business_id: String,
    username: String,
) -> Result<Option<db::RecoveryPrompts>, String> {
    state
        .with_public(|conn| db::owner_recovery_prompts(conn, &business_id, &username))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn auth_recover_owner(
    state: tauri::State<db::DbState>,
    business_id: String,
    username: String,
    recovery_code: String,
    question_one_answer: String,
    question_two_answer: String,
    new_password: String,
) -> Result<(), String> {
    if new_password.chars().count() < 12 || new_password.chars().count() > 128 {
        return Err("password must be 12-128 characters long".to_string());
    }
    validate_recovery_input(&recovery_code, "recovery code")?;
    validate_recovery_input(&question_one_answer, "first answer")?;
    validate_recovery_input(&question_two_answer, "second answer")?;
    let credentials = state
        .with_public(|conn| db::owner_recovery_credentials(conn, &business_id, &username))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "recovery details are incorrect".to_string())?;
    let Some(recovery_code_hash) = credentials.recovery_code_hash.as_deref() else {
        return Err("offline recovery has not been configured for this Owner account".to_string());
    };
    verify_password(&recovery_code, recovery_code_hash)
        .map_err(|_| "recovery details are incorrect".to_string())?;
    verify_password(
        &normalize_recovery_answer(&question_one_answer),
        credentials.answer_one_hash.as_deref().unwrap_or_default(),
    )
    .map_err(|_| "recovery details are incorrect".to_string())?;
    verify_password(
        &normalize_recovery_answer(&question_two_answer),
        credentials.answer_two_hash.as_deref().unwrap_or_default(),
    )
    .map_err(|_| "recovery details are incorrect".to_string())?;
    let password_hash = hash_password(&new_password)?;
    state
        .with_public(|conn| {
            db::reset_owner_password_after_recovery(conn, &credentials.user, &password_hash)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn auth_login(
    state: tauri::State<db::DbState>,
    business_id: String,
    username: String,
    password: String,
) -> Result<db::AuthUser, String> {
    state.clear_session().map_err(|error| error.to_string())?;
    if password.chars().count() > 128 {
        return Err("business, username or password is incorrect".to_string());
    }

    let credentials = state
        .with_public(|conn| db::auth_credentials(conn, &business_id, &username))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "business, username or password is incorrect".to_string())?;
    verify_password(&password, &credentials.password_hash)?;
    state
        .with_public(|conn| db::record_successful_login(conn, &credentials.user))
        .map_err(|error| error.to_string())?;
    state
        .set_session(credentials.user.clone())
        .map_err(|error| error.to_string())?;
    Ok(credentials.user)
}

fn validate_sqlite_backup(path: &Path) -> Result<(), String> {
    let connection = rusqlite::Connection::open(path)
        .map_err(|error| format!("could not open the backup: {error}"))?;
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("could not validate the backup: {error}"))?;
    if result != "ok" {
        return Err(format!("backup integrity check failed: {result}"));
    }
    Ok(())
}

#[tauri::command]
fn database_backup(
    app: tauri::AppHandle,
    state: tauri::State<db::DbState>,
    destination: String,
) -> Result<(), String> {
    let destination_path = Path::new(&destination);
    if destination_path
        .extension()
        .and_then(|value| value.to_str())
        != Some("db")
    {
        return Err("backup files must use the .db extension".to_string());
    }
    let source = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("could not locate application data: {error}"))?
        .join("quincaillerie.db");
    state
        .with_authenticated(|conn, user| {
            if user.role != "owner" {
                return Err(db::DbError::InvalidInput(
                    "only an Owner can create a database backup".to_string(),
                ));
            }
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            fs::copy(&source, destination_path)?;
            conn.execute(
                "INSERT INTO audit_logs
                 (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
                 VALUES (?1, ?2, ?3, 'database_backup_created', 'database', ?1, ?4, ?5)",
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    user.business_id,
                    user.id,
                    destination,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )?;
            Ok(())
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn database_restore(
    app: tauri::AppHandle,
    state: tauri::State<db::DbState>,
    source: String,
) -> Result<(), String> {
    let source_path = Path::new(&source);
    if !source_path.is_file() {
        return Err("the selected backup file does not exist".to_string());
    }
    validate_sqlite_backup(source_path)?;
    let destination = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("could not locate application data: {error}"))?
        .join("quincaillerie.db");
    state
        .with_authenticated(|conn, user| {
            if user.role != "owner" {
                return Err(db::DbError::InvalidInput(
                    "only an Owner can restore the database".to_string(),
                ));
            }
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            fs::copy(source_path, &destination)?;
            conn.execute(
                "INSERT INTO audit_logs
                 (id, business_id, user_id, action, entity_type, entity_id, details, created_at)
                 VALUES (?1, ?2, ?3, 'database_restore_completed', 'database', ?1, ?4, ?5)",
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    user.business_id,
                    user.id,
                    source,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )?;
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn employee_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Employee>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_EMPLOYEES_MANAGE, |conn, _| {
            db::list_employees(conn, &business_id)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn employee_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    username: String,
    full_name: String,
    role: String,
    password: String,
) -> Result<db::Employee, String> {
    if password.chars().count() < 12 || password.chars().count() > 128 {
        return Err("password must be 12-128 characters long".to_string());
    }
    let password_hash = hash_password(&password)?;
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_EMPLOYEES_MANAGE,
            |conn, actor| {
                db::create_employee(
                    conn,
                    &business_id,
                    actor,
                    &username,
                    &full_name,
                    &role,
                    &password_hash,
                )
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn employee_set_active(
    state: tauri::State<db::DbState>,
    business_id: String,
    employee_id: String,
    is_active: bool,
) -> Result<(), String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_EMPLOYEES_MANAGE,
            |conn, actor| {
                db::set_employee_active(conn, &business_id, &employee_id, is_active, actor)
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn employee_password_reset(
    state: tauri::State<db::DbState>,
    business_id: String,
    employee_id: String,
    password: String,
) -> Result<(), String> {
    if password.chars().count() < 12 || password.chars().count() > 128 {
        return Err("password must be 12-128 characters long".to_string());
    }
    let password_hash = hash_password(&password)?;
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_EMPLOYEES_MANAGE,
            |conn, actor| {
                db::reset_employee_password(conn, &business_id, &employee_id, &password_hash, actor)
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn manager_permission_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::RolePermissionSetting>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_ROLES_MANAGE, |conn, _| {
            db::list_manager_permissions(conn, &business_id)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn manager_permission_update(
    state: tauri::State<db::DbState>,
    business_id: String,
    grants: Vec<db::RolePermissionGrant>,
) -> Result<Vec<db::RolePermissionSetting>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_ROLES_MANAGE, |conn, actor| {
            db::update_manager_permissions(conn, &business_id, actor, &grants)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn auth_logout(state: tauri::State<db::DbState>) -> Result<(), String> {
    state.clear_session().map_err(|error| error.to_string())
}

#[tauri::command]
fn auth_current_user(state: tauri::State<db::DbState>) -> Result<Option<db::AuthUser>, String> {
    state
        .current_session_user()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn auth_current_permissions(state: tauri::State<db::DbState>) -> Result<Vec<String>, String> {
    state
        .with_authenticated(db::list_user_permissions)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn db_info(state: tauri::State<db::DbState>) -> Result<db::DbInfo, String> {
    state
        .with_authorized(db::PERMISSION_SYSTEM_READ, |conn, _| db::info(conn))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn business_list(state: tauri::State<db::DbState>) -> Result<Vec<db::Business>, String> {
    state
        .with_authorized(db::PERMISSION_BUSINESS_READ, |conn, user| {
            Ok(db::list_businesses(conn)?
                .into_iter()
                .filter(|business| business.id == user.business_id)
                .collect())
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn business_create(
    state: tauri::State<db::DbState>,
    name: String,
    tax_id: Option<String>,
) -> Result<db::Business, String> {
    state
        .with_authorized(db::PERMISSION_BUSINESS_MANAGE, |conn, user| {
            db::create_business_for_owner(conn, user, &name, tax_id.as_deref())
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_account_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::CashAccount>, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, _| db::list_cash_accounts(conn, &business_id),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_account_payment_options(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::CashAccountPaymentOption>, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNT_BALANCE_READ,
            |conn, _| db::list_cash_account_payment_options(conn, &business_id),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_account_opening_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    account_id: String,
    opening_amount: i64,
) -> Result<(), String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, user| {
                db::create_cash_account_opening(
                    conn,
                    &business_id,
                    &account_id,
                    opening_amount,
                    user,
                )
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_transfer_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    from_account_id: String,
    to_account_id: String,
    amount: i64,
) -> Result<db::CashTransfer, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, user| {
                db::create_cash_transfer(
                    conn,
                    &business_id,
                    &from_account_id,
                    &to_account_id,
                    amount,
                    user,
                )
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_transfer_confirm(
    state: tauri::State<db::DbState>,
    business_id: String,
    transfer_id: String,
    confirmation_reference: Option<String>,
) -> Result<(), String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, user| {
                db::confirm_cash_transfer(
                    conn,
                    &business_id,
                    &transfer_id,
                    confirmation_reference.as_deref(),
                    user,
                )
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_transfer_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::CashTransfer>, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, _| db::list_cash_transfers(conn, &business_id),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_account_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    account_type: String,
    name: String,
) -> Result<db::CashAccount, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, user| db::create_cash_account(conn, &business_id, user, &account_type, &name),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cash_account_set_active(
    state: tauri::State<db::DbState>,
    business_id: String,
    account_id: String,
    is_active: bool,
) -> Result<(), String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CASH_ACCOUNTS_MANAGE,
            |conn, user| {
                db::set_cash_account_active(conn, &business_id, &account_id, is_active, user)
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn category_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Category>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_READ, |conn, _| {
            db::list_categories(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn category_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    parent_id: Option<String>,
    name: String,
    description: Option<String>,
) -> Result<db::Category, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_MANAGE, |conn, _| {
            db::create_category(
                conn,
                &business_id,
                parent_id.as_deref(),
                &name,
                description.as_deref(),
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn unit_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Unit>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_READ, |conn, _| {
            db::list_units(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn unit_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    code: String,
    name: String,
    decimals_ok: bool,
) -> Result<db::Unit, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_MANAGE, |conn, _| {
            db::create_unit(conn, &business_id, &code, &name, decimals_ok)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn brand_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Brand>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_READ, |conn, _| {
            db::list_brands(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn brand_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    name: String,
) -> Result<db::Brand, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_MANAGE, |conn, _| {
            db::create_brand(conn, &business_id, &name)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn supplier_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Supplier>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_SUPPLIERS_READ, |conn, _| {
            db::list_suppliers(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn supplier_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    name: String,
    contact_person: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<db::Supplier, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_SUPPLIERS_MANAGE, |conn, _| {
            db::create_supplier(
                conn,
                &business_id,
                &name,
                contact_person.as_deref(),
                phone.as_deref(),
                email.as_deref(),
                address.as_deref(),
                tax_id.as_deref(),
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn supplier_update(
    state: tauri::State<db::DbState>,
    business_id: String,
    supplier_id: String,
    name: String,
    contact_person: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    tax_id: Option<String>,
    is_active: bool,
) -> Result<db::Supplier, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_SUPPLIERS_MANAGE, |conn, _| {
            db::update_supplier(
                conn,
                &business_id,
                &supplier_id,
                &name,
                contact_person.as_deref(),
                phone.as_deref(),
                email.as_deref(),
                address.as_deref(),
                tax_id.as_deref(),
                is_active,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Customer>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CUSTOMERS_READ, |conn, _| {
            db::list_customers(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    name: String,
    phone: Option<String>,
    address: Option<String>,
    customer_type: String,
    credit_limit: i64,
) -> Result<db::Customer, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CUSTOMERS_MANAGE, |conn, _| {
            db::create_customer(
                conn,
                &business_id,
                &name,
                phone.as_deref(),
                address.as_deref(),
                &customer_type,
                credit_limit,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_update(
    state: tauri::State<db::DbState>,
    business_id: String,
    customer_id: String,
    name: String,
    phone: Option<String>,
    address: Option<String>,
    customer_type: String,
    credit_limit: i64,
    is_active: bool,
) -> Result<db::Customer, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CUSTOMERS_MANAGE, |conn, _| {
            db::update_customer(
                conn,
                &business_id,
                &customer_id,
                &name,
                phone.as_deref(),
                address.as_deref(),
                &customer_type,
                credit_limit,
                is_active,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purchase_product_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::PurchaseProduct>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_PURCHASES_READ, |conn, _| {
            db::list_purchase_products(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purchase_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Purchase>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_PURCHASES_READ, |conn, _| {
            db::list_purchases(conn, &business_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purchase_history_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    search: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<db::PurchaseHistoryPage, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_PURCHASES_READ, |conn, _| {
            db::list_purchase_history(
                conn,
                &business_id,
                search.as_deref(),
                from_date.as_deref(),
                to_date.as_deref(),
                offset,
                limit,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purchase_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    supplier_id: String,
    supplier_invoice_number: String,
    items: Vec<db::PurchaseItemInput>,
    authorize_duplicate: bool,
) -> Result<db::Purchase, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_PURCHASES_MANAGE, |conn, _| {
            db::create_purchase(
                conn,
                &business_id,
                &supplier_id,
                &supplier_invoice_number,
                &items,
                authorize_duplicate,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purchase_record_payment(
    state: tauri::State<db::DbState>,
    business_id: String,
    purchase_id: String,
    amount: i64,
    payment_method: String,
    account_id: String,
) -> Result<db::Purchase, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_PURCHASES_MANAGE,
            |conn, user| {
                db::record_purchase_payment_as_user(
                    conn,
                    &business_id,
                    &purchase_id,
                    amount,
                    &payment_method,
                    &account_id,
                    user,
                )
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purchase_receive_delivery(
    state: tauri::State<db::DbState>,
    business_id: String,
    purchase_id: String,
    items: Vec<db::PurchaseReceiptItemInput>,
    delivery_reference: Option<String>,
    notes: Option<String>,
) -> Result<db::PurchaseReceipt, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_PURCHASES_MANAGE, |conn, _| {
            db::receive_purchase_delivery(
                conn,
                &business_id,
                &purchase_id,
                &items,
                delivery_reference.as_deref(),
                notes.as_deref(),
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn product_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    search: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<db::ProductPage, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_READ, |conn, _| {
            db::list_products(conn, &business_id, search.as_deref(), offset, limit)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn pos_product_search(
    state: tauri::State<db::DbState>,
    business_id: String,
    search: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<db::PosProductPage, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_POS_USE, |conn, _| {
            db::search_pos_products(conn, &business_id, search.as_deref(), offset, limit)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn pos_checkout(
    state: tauri::State<db::DbState>,
    business_id: String,
    items: Vec<db::SaleItemInput>,
    customer_id: Option<String>,
    payments: Vec<db::SalePaymentInput>,
) -> Result<db::SaleReceipt, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_SALES_MANAGE, |conn, user| {
            db::checkout_sale_with_payments_as_user(
                conn,
                &business_id,
                &items,
                customer_id.as_deref(),
                &payments,
                user,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn sales_history_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    search: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<db::SaleHistoryPage, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_SALES_READ, |conn, _| {
            db::list_sales(
                conn,
                &business_id,
                search.as_deref(),
                from_date.as_deref(),
                to_date.as_deref(),
                offset,
                limit,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_debt_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::CustomerDebt>, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CUSTOMER_DEBTS_READ,
            |conn, _| db::list_customer_debts(conn, &business_id),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_statement(
    state: tauri::State<db::DbState>,
    business_id: String,
    customer_id: String,
) -> Result<db::CustomerStatement, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CUSTOMERS_READ, |conn, _| {
            db::customer_statement(conn, &business_id, &customer_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn expense_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    offset: i64,
    limit: i64,
) -> Result<db::ExpensePage, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_EXPENSES_READ, |conn, _| {
            db::list_expenses(conn, &business_id, offset, limit)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn daily_business_report(
    state: tauri::State<db::DbState>,
    business_id: String,
    report_date: String,
) -> Result<db::DailyBusinessReport, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_REPORTS_READ, |conn, _| {
            db::daily_business_report(conn, &business_id, &report_date)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn expense_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    category: String,
    amount: i64,
    expense_date: String,
    payment_method: String,
    description: String,
    responsible_name: String,
    account_id: String,
) -> Result<db::Expense, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_EXPENSES_MANAGE,
            |conn, user| {
                db::create_expense_as_user(
                    conn,
                    &business_id,
                    &category,
                    amount,
                    &expense_date,
                    &payment_method,
                    &description,
                    &responsible_name,
                    &account_id,
                    user,
                )
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_debt_record_payment(
    state: tauri::State<db::DbState>,
    business_id: String,
    debt_id: String,
    payment_method: String,
    amount: i64,
    account_id: String,
) -> Result<db::CustomerDebtPayment, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CUSTOMER_PAYMENTS_MANAGE,
            |conn, user| {
                db::record_customer_debt_payment_as_user(
                    conn,
                    &business_id,
                    &debt_id,
                    &payment_method,
                    amount,
                    &account_id,
                    user,
                )
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn product_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    category_id: Option<String>,
    brand_id: Option<String>,
    name: String,
    sku: String,
    barcode: Option<String>,
    description: Option<String>,
    size_dimensions: Option<String>,
    colour: Option<String>,
    base_unit_id: String,
    purchase_price: i64,
    selling_price: i64,
    min_stock_quantity: i64,
    vat_class: String,
) -> Result<db::Product, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_MANAGE, |conn, _| {
            db::create_product(
                conn,
                &business_id,
                category_id.as_deref(),
                brand_id.as_deref(),
                &name,
                &sku,
                barcode.as_deref(),
                description.as_deref(),
                size_dimensions.as_deref(),
                colour.as_deref(),
                &base_unit_id,
                purchase_price,
                selling_price,
                min_stock_quantity,
                &vat_class,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn product_selling_price_update(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
    selling_price: i64,
) -> Result<(), String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_MANAGE, |conn, _| {
            db::update_base_selling_price(conn, &business_id, &product_id, selling_price)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn product_unit_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
) -> Result<Vec<db::ProductUnit>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_READ, |conn, _| {
            db::list_product_units(conn, &business_id, &product_id)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn product_unit_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
    unit_id: String,
    conversion_factor: i64,
    selling_price: Option<i64>,
) -> Result<db::ProductUnit, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_CATALOG_MANAGE, |conn, _| {
            db::create_product_unit(
                conn,
                &business_id,
                &product_id,
                &unit_id,
                conversion_factor,
                selling_price,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn inventory_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    search: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<db::InventoryPage, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_INVENTORY_READ, |conn, _| {
            db::list_inventory(conn, &business_id, search.as_deref(), offset, limit)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stock_movement_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<Vec<db::StockMovement>, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_INVENTORY_READ, |conn, _| {
            db::list_stock_movements(conn, &business_id, product_id.as_deref(), offset, limit)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn inventory_set_opening_stock(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
    quantity: i64,
    reason: Option<String>,
) -> Result<db::StockMovement, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_INVENTORY_MANAGE, |conn, _| {
            db::set_opening_stock(conn, &business_id, &product_id, quantity, reason.as_deref())
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn inventory_correct_stock(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
    target_quantity: i64,
    reason: String,
) -> Result<db::StockMovement, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_INVENTORY_MANAGE, |conn, _| {
            db::correct_stock(conn, &business_id, &product_id, target_quantity, &reason)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn inventory_receive_stock(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
    quantity: i64,
    received_unit_cost: i64,
    receipt_reference: String,
    reason: Option<String>,
    new_selling_price: Option<i64>,
) -> Result<db::StockMovement, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_INVENTORY_MANAGE, |conn, _| {
            if let Some(new_selling_price) = new_selling_price {
                db::receive_stock_with_selling_price(
                    conn,
                    &business_id,
                    &product_id,
                    quantity,
                    received_unit_cost,
                    &receipt_reference,
                    reason.as_deref(),
                    Some(new_selling_price),
                )
            } else {
                db::receive_stock(
                    conn,
                    &business_id,
                    &product_id,
                    quantity,
                    received_unit_cost,
                    &receipt_reference,
                    reason.as_deref(),
                )
            }
        })
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let conn = db::open(&data_dir.join("quincaillerie.db"))?;
            app.manage(db::DbState::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth_business_status,
            auth_bootstrap_owner,
            auth_recovery_questions,
            auth_recovery_prompts,
            auth_recover_owner,
            auth_login,
            database_backup,
            database_restore,
            employee_list,
            employee_create,
            employee_set_active,
            employee_password_reset,
            manager_permission_list,
            manager_permission_update,
            auth_logout,
            auth_current_user,
            auth_current_permissions,
            db_info,
            business_list,
            business_create,
            cash_account_list,
            cash_account_create,
            cash_account_payment_options,
            cash_account_opening_create,
            cash_transfer_create,
            cash_transfer_confirm,
            cash_transfer_list,
            cash_account_set_active,
            category_list,
            category_create,
            unit_list,
            unit_create,
            brand_list,
            brand_create,
            supplier_list,
            supplier_create,
            supplier_update,
            customer_list,
            customer_create,
            customer_update,
            customer_debt_list,
            customer_statement,
            expense_list,
            daily_business_report,
            expense_create,
            customer_debt_record_payment,
            purchase_product_list,
            purchase_list,
            purchase_history_list,
            purchase_create,
            purchase_record_payment,
            purchase_receive_delivery,
            product_list,
            pos_product_search,
            pos_checkout,
            sales_history_list,
            product_create,
            product_selling_price_update,
            product_unit_list,
            product_unit_create,
            inventory_list,
            stock_movement_list,
            inventory_set_opening_stock,
            inventory_correct_stock,
            inventory_receive_stock
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod auth_tests {
    use super::*;

    #[test]
    fn passwords_are_argon2id_hashed_and_verified() {
        let hash = hash_password("correct horse battery staple").unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery staple", &hash).is_ok());
        assert!(verify_password("incorrect password", &hash).is_err());
    }
}
