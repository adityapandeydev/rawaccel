fn main() {
    let settings_json = std::fs::read_to_string("settings.json").unwrap();
    let config: rawaccel_userspace_lib::config::AppConfig =
        serde_json::from_str(&settings_json).unwrap();

    let native_prof = config.profiles[0]
        .to_native()
        .expect("Profile validation failed");
    println!("output_dpi: {}", native_prof.output_dpi);
}
