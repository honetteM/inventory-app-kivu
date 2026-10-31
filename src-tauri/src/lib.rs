mod db;

use tauri::Manager;

#[tauri::command]
fn db_info(state: tauri::State<db::DbState>) -> Result<db::DbInfo, String> {
    state.with(db::info).map_err(|e| e.to_string())
}

#[tauri::command]
fn business_list(state: tauri::State<db::DbState>) -> Result<Vec<db::Business>, String> {
    state.with(db::list_businesses).map_err(|e| e.to_string())
}

#[tauri::command]
fn business_create(
    state: tauri::State<db::DbState>,
    name: String,
    tax_id: Option<String>,
) -> Result<db::Business, String> {
    state
        .with(|conn| db::create_business(conn, &name, tax_id.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn category_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Category>, String> {
    state
        .with(|conn| db::list_categories(conn, &business_id))
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
        .with(|conn| {
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
        .with(|conn| db::list_units(conn, &business_id))
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
        .with(|conn| db::create_unit(conn, &business_id, &code, &name, decimals_ok))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn brand_list(
    state: tauri::State<db::DbState>,
    business_id: String,
) -> Result<Vec<db::Brand>, String> {
    state
        .with(|conn| db::list_brands(conn, &business_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn brand_create(
    state: tauri::State<db::DbState>,
    business_id: String,
    name: String,
) -> Result<db::Brand, String> {
    state
        .with(|conn| db::create_brand(conn, &business_id, &name))
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
        .with(|conn| db::list_products(conn, &business_id, search.as_deref(), offset, limit))
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
        .with(|conn| {
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
fn product_unit_list(
    state: tauri::State<db::DbState>,
    business_id: String,
    product_id: String,
) -> Result<Vec<db::ProductUnit>, String> {
    state
        .with(|conn| db::list_product_units(conn, &business_id, &product_id))
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
        .with(|conn| {
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
        .with(|conn| db::list_inventory(conn, &business_id, search.as_deref(), offset, limit))
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
        .with(|conn| {
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
        .with(|conn| {
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
        .with(|conn| db::correct_stock(conn, &business_id, &product_id, target_quantity, &reason))
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
) -> Result<db::StockMovement, String> {
    state
        .with(|conn| {
            db::receive_stock(
                conn,
                &business_id,
                &product_id,
                quantity,
                received_unit_cost,
                &receipt_reference,
                reason.as_deref(),
            )
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
            db_info,
            business_list,
            business_create,
            category_list,
            category_create,
            unit_list,
            unit_create,
            brand_list,
            brand_create,
            product_list,
            product_create,
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
