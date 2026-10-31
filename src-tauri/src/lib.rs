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
            business_create
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
