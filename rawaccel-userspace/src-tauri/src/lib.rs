mod input_buffers;
mod wire;
mod packet;
mod validation;
mod apply;
mod migration;
mod settings_defaults;
pub mod config;
pub mod devices;
pub mod driver;
pub mod math;
pub mod models;
pub mod mouse_tracker;
pub mod persistence;

#[tauri::command]
fn apply_settings(settings_json: String) -> Result<apply::ApplyReport, String> {
    apply::apply_with(
        &settings_json,
        driver::apply_config,
        config::save_settings_to_disk,
    )
}

#[tauri::command]
fn load_settings() -> Result<persistence::LoadReport, String> {
    config::load_settings_with_report()
}

#[tauri::command]
fn import_settings(raw_json: String) -> Result<String, String> {
    let document = config::normalize_settings_json(&raw_json)?;
    let serialized = serde_json::to_string_pretty(&document)
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
