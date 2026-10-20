mod driver_contract;
mod stock_wire;
mod driver_status;
#[cfg(test)]
mod abi_tests;
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
pub mod startup;

#[tauri::command]
fn get_startup_info() -> Result<startup::StartupInfo, String> {
    startup::info()
}

#[tauri::command]
fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    startup::set_enabled(enabled)
}

#[tauri::command]
fn open_settings_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = config::settings_paths()?.primary;
    let folder = path.parent().ok_or("Settings directory is unavailable")?;
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    app.opener().open_path(folder.display().to_string(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_driver_status() -> Result<driver_status::DriverStatus, String> {
    tauri::async_runtime::spawn_blocking(driver_status::get).await
        .map_err(|error| format!("Driver status worker failed: {error}"))
}

#[tauri::command]
async fn apply_settings(settings_json: String) -> Result<apply::ApplyReport, String> {
    tauri::async_runtime::spawn_blocking(move || apply::apply_with(
        &settings_json, driver::apply_config, config::save_settings_to_disk,
    )).await.map_err(|error| format!("Apply worker failed: {error}"))?
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
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            mouse_tracker::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apply_settings,
            load_settings,
            import_settings,
            get_devices,
            get_driver_status,
            get_startup_info,
            set_startup_enabled,
            open_settings_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
