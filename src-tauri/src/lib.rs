mod db;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand_core::OsRng;
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
) -> Result<db::AuthUser, String> {
    if password.chars().count() < 12 || password.chars().count() > 128 {
        return Err("password must be 12-128 characters long".to_string());
    }
    let password_hash = hash_password(&password)?;
    let user = state
        .with_public(|conn| {
            db::bootstrap_owner(
                conn,
                business_id.as_deref(),
                new_business_name.as_deref(),
                &username,
                &full_name,
                &password_hash,
            )
        })
        .map_err(|error| error.to_string())?;
    state
        .set_session(user.clone())
        .map_err(|error| error.to_string())?;
    Ok(user)
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
) -> Result<db::Purchase, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_PURCHASES_MANAGE, |conn, _| {
            db::record_purchase_payment(conn, &business_id, &purchase_id, amount, &payment_method)
        })
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
        .with_business_permission(&business_id, db::PERMISSION_SALES_MANAGE, |conn, _| {
            db::checkout_sale_with_payments(
                conn,
                &business_id,
                &items,
                customer_id.as_deref(),
                &payments,
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
        .with_business_permission(&business_id, db::PERMISSION_FINANCE_READ, |conn, _| {
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
        .with_business_permission(&business_id, db::PERMISSION_FINANCE_READ, |conn, _| {
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
) -> Result<db::Expense, String> {
    state
        .with_business_permission(&business_id, db::PERMISSION_FINANCE_MANAGE, |conn, _| {
            db::create_expense(
                conn,
                &business_id,
                &category,
                amount,
                &expense_date,
                &payment_method,
                &description,
                &responsible_name,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn customer_debt_record_payment(
    state: tauri::State<db::DbState>,
    business_id: String,
    debt_id: String,
    payment_method: String,
    amount: i64,
) -> Result<db::CustomerDebtPayment, String> {
    state
        .with_business_permission(
            &business_id,
            db::PERMISSION_CUSTOMER_PAYMENTS_MANAGE,
            |conn, _| {
                db::record_customer_debt_payment(
                    conn,
                    &business_id,
                    &debt_id,
                    &payment_method,
                    amount,
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
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let conn = db::open(&data_dir.join("quincaillerie.db"))?;
            app.manage(db::DbState::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth_business_status,
            auth_bootstrap_owner,
            auth_login,
            employee_list,
            employee_create,
            employee_set_active,
            manager_permission_list,
            manager_permission_update,
            auth_logout,
            auth_current_user,
            db_info,
            business_list,
            business_create,
            cash_account_list,
            cash_account_create,
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
            purchase_create,
            purchase_record_payment,
            purchase_receive_delivery,
            product_list,
            pos_product_search,
            pos_checkout,
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
