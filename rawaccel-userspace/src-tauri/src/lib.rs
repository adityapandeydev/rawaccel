pub mod config;
pub mod devices;
pub mod driver;
pub mod math;
pub mod models;
pub mod mouse_tracker;
pub mod persistence;

#[tauri::command]
fn apply_settings(settings_json: String) -> Result<(), String> {
    let app_config: config::AppConfig = serde_json::from_str(&settings_json)
        .map_err(|e| format!("Failed to parse settings JSON: {}", e))?;

    app_config.validate()?;

    // Apply to driver
    driver::apply_config(&app_config)?;

    // Persist settings in local executable directory with mirrored AppData backup
    config::save_settings_to_disk(&settings_json)?;

    Ok(())
}

#[tauri::command]
fn load_settings() -> Result<persistence::LoadReport, String> {
    config::load_settings_with_report()
}

#[tauri::command]
fn import_settings(raw_json: String) -> Result<String, String> {
    let config = config::parse_or_migrate_settings(&raw_json)?;
    let serialized = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize migrated settings: {}", e))?;
    Ok(serialized)
}

#[tauri::command]
fn get_devices() -> Vec<devices::DeviceInfo> {
    devices::get_connected_devices()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().args(vec!["--silent"]).build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            mouse_tracker::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apply_settings,
            load_settings,
            import_settings,
            get_devices
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
