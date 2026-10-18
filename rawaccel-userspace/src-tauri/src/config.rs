use crate::models;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Settings document version, independent of the kernel driver's version/ABI.
pub const SETTINGS_SCHEMA_VERSION: u64 = 1;

pub(crate) fn settings_schema_version(value: &serde_json::Value) -> Result<u64, String> {
    let version = match value.get("schema_version") {
        None => 0, // The original Rust format had no schema marker.
        Some(value) => value.as_u64().ok_or("schema_version must be a non-negative integer")?,
    };
    if version > SETTINGS_SCHEMA_VERSION {
        return Err(format!("Unsupported settings schema version {version}; this app supports up to {SETTINGS_SCHEMA_VERSION}. The file has not been changed."));
    }
    Ok(version)
}

/// Gets the primary settings path: right next to the running executable.
pub fn get_local_settings_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join("settings.json");
        }
    }
    PathBuf::from("settings.json")
}

/// Gets the backup settings path in %LocalAppData%\RawAccel\settings.json
pub fn get_appdata_settings_path() -> Option<PathBuf> {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        return Some(PathBuf::from(local_app_data).join("RawAccel").join("settings.json"));
    }
    None
}

#[derive(Debug, PartialEq)]
enum SettingsFormat {
    Modern,
    Legacy,
}

fn detect_settings_format(value: &serde_json::Value) -> Result<SettingsFormat, String> {
    let object = value.as_object().ok_or("Settings must be a JSON object")?;
    let profiles = object
        .get("profiles")
        .and_then(|profiles| profiles.as_array())
        .ok_or("Settings must contain a profiles array")?;
    let mut legacy = false;
    let mut modern = object.contains_key("schema_version");
    for profile in profiles {
        let profile = profile.as_object().ok_or("Each profile must be an object")?;
        legacy |= [
            "argsX", "argsY", "domainXY", "rangeXY", "inputSpeedArgs",
            "Whole or horizontal accel parameters", "Vertical accel parameters",
            "Stretches domain for horizontal vs vertical inputs",
            "Stretches accel range for horizontal vs vertical inputs",
            "Input speed calculation parameters", "Output DPI",
        ].iter().any(|key| profile.contains_key(*key));
        modern |= [
            "accel_x", "accel_y", "domain_weights", "range_weights",
            "speed_processor_args", "output_dpi",
        ].iter().any(|key| profile.contains_key(*key));
    }
    if legacy && modern {
        return Err("Mixed modern and legacy settings are not supported; use one format per document".to_string());
    }
    Ok(if legacy { SettingsFormat::Legacy } else { SettingsFormat::Modern })
}

/// Normalize recognized aliases in memory while retaining all JSON metadata.
pub fn normalize_settings_json(raw_json: &str) -> Result<serde_json::Value, String> {
    let mut value: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|error| format!("Invalid JSON: {error}"))?;
    settings_schema_version(&value)?;
    let format = detect_settings_format(&value)?;
    if format == SettingsFormat::Legacy {
        value = crate::migration::migrate(value)?;
    }
    crate::settings_defaults::complete_profiles(&mut value)?;
    value.as_object_mut().unwrap().entry("devices").or_insert_with(|| serde_json::json!([]));
    let config: AppConfig = serde_json::from_value(value.clone())
        .map_err(|error| format!("Invalid {} settings: {error}", if format == SettingsFormat::Modern { "modern" } else { "legacy" }))?;
    config.validate()?;
    value.as_object_mut().unwrap().insert("schema_version".to_string(), serde_json::json!(SETTINGS_SCHEMA_VERSION));
    Ok(value)
}

/// Driver-facing models contain only supported fields; the document retains metadata.
pub fn parse_or_migrate_settings(raw_json: &str) -> Result<AppConfig, String> {
    serde_json::from_value(normalize_settings_json(raw_json)?)
        .map_err(|error| error.to_string())
}
/// Loads and validates candidates without rewriting originals or normalizing files.
pub fn load_settings_with_report() -> Result<crate::persistence::LoadReport, String> {
    crate::persistence::load(&settings_paths(), parse_settings_for_load)
}

/// The silent startup path needs only JSON; diagnostics remain available in logs.
pub fn load_settings_from_disk() -> Result<String, String> {
    let report = load_settings_with_report()?;
    for warning in &report.warnings {
        eprintln!("Settings recovery: {warning}");
    }
    Ok(report.settings_json)
}

pub(crate) fn parse_settings_for_load(raw: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| format!("Invalid JSON: {error}"))?;
    let format = detect_settings_format(&value)?;
    let document = normalize_settings_json(raw)?;
    match format {
        SettingsFormat::Modern if value == document => Ok(raw.to_string()),
        // Migration is in memory only; explicit saving retains the original.
        _ => serde_json::to_string_pretty(&document)
            .map_err(|error| error.to_string()),
    }
}

pub(crate) fn settings_paths() -> crate::persistence::SettingsPaths {
    crate::persistence::SettingsPaths {
        primary: get_local_settings_path(),
        fallback: std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("settings.json"),
        mirror: get_appdata_settings_path(),
    }
}

/// Saves portable settings (with a CWD fallback) and an AppData mirror.
/// Success means at least one write succeeded; partial failures are reported.
pub fn save_settings_to_disk(settings_json: &str) -> Result<crate::persistence::SaveReport, String> {
    crate::persistence::save(settings_json, &settings_paths())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub profiles: Vec<AppProfile>,
    #[serde(default)]
    pub devices: Vec<AppDeviceConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_device_config: Option<AppDeviceDefaults>,
}

impl AppConfig {
    /// Validates data that is used directly by the driver.
    /// Ensures profiles exist, have unique non-empty names, and mapped devices reference existing profiles.
    pub fn validate(&self) -> Result<(), String> {
        if self.profiles.is_empty() {
            return Err("At least one profile is required".to_string());
        }
        if let Some(defaults) = &self.default_device_config { defaults.to_native()?; }

        let mut profile_names = std::collections::HashSet::new();
        for profile in &self.profiles {
            crate::validation::wire_string(&profile.name, models::MAX_NAME_LEN, "Profile name")?;
            let trimmed = profile.name.trim();
            if trimmed.is_empty() {
                return Err("Profile name cannot be empty".to_string());
            }
            if !profile_names.insert(trimmed.to_lowercase()) {
                return Err(format!("Duplicate profile name '{}'", profile.name));
            }

            validate_profile(profile)?;
        }

        let inherited = self.default_device_config.as_ref().map(|d| d.to_native()).transpose()?.map(|d| d.clamp).unwrap_or_default();
        let mut device_ids = std::collections::HashSet::new();
        for dev in &self.devices {
            crate::validation::wire_string(&dev.id, models::MAX_DEV_ID_LEN, "Device ID")?;
            if !device_ids.insert(&dev.id) { return Err(format!("Duplicate device ID '{}'", dev.id)); }
            crate::validation::device_numbers(dev.dpi, dev.polling_rate, dev.clamp_min, dev.clamp_max, inherited)
                .map_err(|error| format!("Device '{}': {error}", dev.id))?;
            if let Some(ref pid) = dev.profile_id {
                crate::validation::wire_string(pid, models::MAX_NAME_LEN, "Mapped profile name")?;
                if !self.profiles.iter().any(|p| p.name == *pid) {
                    return Err(format!(
                        "Device '{}' references non-existent profile '{}'",
                        dev.id, pid
                    ));
                }
            }
        }

        Ok(())
    }
}

fn validate_profile(profile: &AppProfile) -> Result<(), String> {
    crate::validation::wire_string(&profile.name, models::MAX_NAME_LEN, "Profile name")?;
    validate_accel_args(&profile.name, "X", &profile.accel_x)?;
    // init_data precomputes Y even in Combined mode.
    validate_accel_args(&profile.name, "Y", &profile.accel_y)?;
    crate::validation::profile_numbers(profile)
}

fn validate_lookup_table(
    profile_name: &str,
    axis: &str,
    args: &AppAccelArgs,
) -> Result<(), String> {
    let points = &args.lookup_table;
    let max_points = models::LUT_POINTS_CAPACITY;
    if args.mode == "lookup" && points.len() < 2 {
        return Err(format!(
            "Profile '{profile_name}' {axis} lookup table needs at least two points"
        ));
    }
    if points.len() > max_points {
        return Err(format!(
            "Profile '{profile_name}' {axis} lookup table has too many points (maximum {max_points})"
        ));
    }

    let mut previous_x = 0.0;
    for (index, point) in points.iter().enumerate() {
        if !point.x.is_finite() || !point.y.is_finite() || !(point.x as f32).is_finite() || !(point.y as f32).is_finite() {
            return Err(format!(
                "Profile '{profile_name}' {axis} lookup point {} must contain finite numbers",
                index + 1
            ));
        }
        if (point.x as f32) <= 0.0 {
            return Err(format!(
                "Profile '{profile_name}' {axis} lookup point {} must have an input speed above zero",
                index + 1
            ));
        }
        if index > 0 && (point.x as f32) <= previous_x {
            return Err(format!(
                "Profile '{profile_name}' {axis} lookup input speeds must be strictly increasing"
            ));
        }
        previous_x = point.x as f32;
    }

    Ok(())
}

fn validate_accel_args(
    profile_name: &str,
    axis: &str,
    args: &AppAccelArgs,
) -> Result<(), String> {

    if !["noaccel", "linear", "classic", "jump", "natural", "synchronous", "power", "lookup", "tiered"].contains(&args.mode.as_str()) {
        return Err(format!("Profile '{}' {axis}: unknown acceleration mode '{}'", profile_name, args.mode));
    }
    if !["in", "out", "io"].contains(&args.cap_mode.as_str()) {
        return Err(format!("Profile '{}' {axis}: unknown cap mode '{}'", profile_name, args.cap_mode));
    }
    if !["linear", "natural"].contains(&args.t_type.as_str()) {
        return Err(format!("Profile '{}' {axis}: unknown Tiered subtype '{}'", profile_name, args.t_type));
    }
    crate::validation::accel_numbers(args).map_err(|error| format!("Profile '{profile_name}' {axis}: {error}"))?;
    validate_lookup_table(profile_name, axis, args)?;
    let requires_cap = args.mode == "jump" || (["classic", "linear", "power"].contains(&args.mode.as_str()) && args.cap_mode == "io");
    if requires_cap && (args.cap.x == 0.0 || args.cap.y == 0.0) {
        return Err(format!("Profile '{profile_name}' {axis}: both caps must be positive for this mode/cap style"));
    }
    if (["classic", "linear"].contains(&args.mode.as_str()) && args.cap_mode != "out" && args.cap.x > 0.0 && args.cap.x < args.input_offset)
        || (args.mode == "power" && args.cap_mode != "in" && args.cap.y > 0.0 && args.cap.y < args.output_offset) {
        return Err(format!("Profile '{profile_name}' {axis}: cap cannot be less than offset"));
    }
    if ["jump", "synchronous"].contains(&args.mode.as_str()) && !(0.0..=1.0).contains(&args.smooth) {
        return Err(format!("Profile '{profile_name}' {axis}: smooth must be between 0 and 1"));
    }
    if args.input_offset < 0.0 {
        return Err(format!("Profile '{profile_name}' {axis}: offset can not be negative"));
    }
    if args.output_offset < 0.0 {
        return Err(format!("Profile '{profile_name}' {axis}: offset can not be negative"));
    }
    if args.cap.x < 0.0 || args.cap.y < 0.0 {
        return Err(format!("Profile '{profile_name}' {axis}: cap can not be negative"));
    }

    match args.mode.as_str() {
        "tiered" => {
            if args.tiered_multiplier1 <= 0.0 || args.tiered_multiplier2 <= 0.0 || args.tiered_multiplier3 <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: tiered multipliers must be positive"));
            }
            if args.tiered_input_offset1 < 0.0 || args.tiered_input_offset2 < 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: offset can not be negative"));
            }
            if args.t_type == "linear" {
                if !(args.tiered_input_offset1 + args.tiered_transition1).is_finite() || !(args.tiered_input_offset2 + args.tiered_transition2).is_finite() {
                    return Err(format!("Profile '{profile_name}' {axis}: transition endpoints must be finite"));
                }
                if args.tiered_transition1 <= 0.0 || args.tiered_transition2 <= 0.0 {
                    return Err(format!("Profile '{profile_name}' {axis}: transition width must be positive"));
                }
                if args.tiered_input_offset2 < args.tiered_input_offset1 + args.tiered_transition1 {
                    return Err(format!("Profile '{profile_name}' {axis}: second offset must be greater than or equal to first transition end"));
                }
            } else if args.t_type == "natural" {
                if args.tiered_input_offset2 <= args.tiered_input_offset1 {
                    return Err(format!("Profile '{profile_name}' {axis}: second offset must be greater than first offset"));
                }
                if args.tiered_decay_rate1 <= 0.0 || args.tiered_decay_rate2 <= 0.0 {
                    return Err(format!("Profile '{profile_name}' {axis}: decay rate must be positive"));
                }
            }
        }
        "classic" => {
            if args.acceleration <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: acceleration must be positive"));
            }
            if args.exponent_classic <= 1.0 {
                return Err(format!("Profile '{profile_name}' {axis}: exponent must be greater than 1"));
            }
        }
        "linear" => {
            if args.acceleration <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: acceleration must be positive"));
            }
        }
        "natural" => {
            if args.decay_rate <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: decay rate must be positive"));
            }
            if args.limit <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: limit must be positive"));
            }
        }
        "power" => {
            if args.scale <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: scale must be positive"));
            }
            if args.exponent_power <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: exponent must be positive"));
            }
        }
        "jump" => {
            if args.smooth < 0.0 || args.smooth > 1.0 {
                return Err(format!("Profile '{profile_name}' {axis}: smooth must be between 0 and 1"));
            }
        }
        "synchronous" => {
            if args.sync_speed <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: synchronous speed must be positive"));
            }
            if args.motivity <= 1.0 {
                return Err(format!("Profile '{profile_name}' {axis}: motivity must be greater than 1"));
            }
            if args.gamma <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: gamma must be positive"));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Defaults for unmapped devices, separate from per-device identity and mapping.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct AppDeviceDefaults {
    pub disable: bool,
    pub set_extra_info: bool,
    pub poll_time_lock: bool,
    pub dpi: u32,
    pub polling_rate: u32,
    pub clamp_min: f64,
    pub clamp_max: f64,
}

impl AppDeviceDefaults {
    pub fn to_native(&self) -> Result<models::device_config, String> {
        crate::validation::device_numbers(self.dpi, self.polling_rate, self.clamp_min, self.clamp_max, models::time_clamp::default())?;
        let defaults = models::time_clamp::default();
        Ok(models::device_config {
            disable: self.disable,
            set_extra_info: self.set_extra_info,
            poll_time_lock: self.poll_time_lock,
            dpi: self.dpi as i32,
            polling_rate: self.polling_rate as i32,
            clamp: models::time_clamp {
                min: if self.clamp_min == 0.0 { defaults.min } else { self.clamp_min },
                max: if self.clamp_max == 0.0 { defaults.max } else { self.clamp_max },
            },
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppDeviceConfig {
    pub id: String,
    /// Driver profile name, not AppProfile.id; names are the wire-level mapping key.
    pub profile_id: Option<String>,
    #[serde(default)]
    pub disable: bool,
    #[serde(default)]
    pub set_extra_info: bool,
    #[serde(default = "default_poll_time_lock")]
    pub poll_time_lock: bool,
    #[serde(default)]
    pub dpi: u32,
    #[serde(default)]
    pub polling_rate: u32,
    #[serde(default)]
    pub clamp_min: f64,
    #[serde(default)]
    pub clamp_max: f64,
}

fn default_poll_time_lock() -> bool {
    false
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub domain_weights: AppVec2D,
    pub range_weights: AppVec2D,
    pub accel_x: AppAccelArgs,
    pub accel_y: AppAccelArgs,
    pub speed_processor_args: AppSpeedArgs,
    pub output_dpi: f64,
    pub yx_output_dpi_ratio: f64,
    pub lr_output_dpi_ratio: f64,
    pub ud_output_dpi_ratio: f64,
    pub degrees_rotation: f64,
    pub degrees_snap: f64,
    pub speed_min: f64,
    pub speed_max: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppVec2D {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSpeedArgs {
    #[serde(default)]
    pub whole: bool,
    #[serde(default)]
    pub lp_norm: f64,
    #[serde(default)]
    pub input_speed_smooth_halflife: f64,
    #[serde(default)]
    pub scale_smooth_halflife: f64,
    #[serde(default)]
    pub output_speed_smooth_halflife: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppAccelArgs {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub gain: bool,
    #[serde(default)]
    pub input_offset: f64,
    #[serde(default)]
    pub output_offset: f64,
    #[serde(default)]
    pub acceleration: f64,
    #[serde(default)]
    pub decay_rate: f64,
    #[serde(default)]
    pub gamma: f64,
    #[serde(default)]
    pub motivity: f64,
    #[serde(default)]
    pub exponent_classic: f64,
    #[serde(default)]
    pub scale: f64,
    #[serde(default)]
    pub exponent_power: f64,
    #[serde(default)]
    pub limit: f64,
    #[serde(default)]
    pub sync_speed: f64,
    #[serde(default)]
    pub smooth: f64,
    #[serde(default)]
    pub t_type: String,
    #[serde(default)]
    pub tiered_multiplier1: f64,
    #[serde(default)]
    pub tiered_input_offset1: f64,
    #[serde(default)]
    pub tiered_multiplier2: f64,
    #[serde(default)]
    pub tiered_transition1: f64,
    #[serde(default)]
    pub tiered_input_offset2: f64,
    #[serde(default)]
    pub tiered_multiplier3: f64,
    #[serde(default)]
    pub tiered_transition2: f64,
    #[serde(default)]
    pub tiered_decay_rate1: f64,
    #[serde(default)]
    pub tiered_decay_rate2: f64,
    #[serde(default)]
    pub cap: AppVec2D,
    #[serde(default)]
    pub cap_mode: String,
    #[serde(default)]
    pub lookup_table: Vec<AppVec2D>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookup_csv_backup: Option<Vec<AppVec2D>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookup_csv_backup_gain: Option<bool>,
}

impl Default for AppProfile {
    fn default() -> Self {
        serde_json::from_value(crate::settings_defaults::profile())
            .expect("Checked-in profile defaults must match the model")
    }
}

impl Default for AppSpeedArgs {
    fn default() -> Self {
        serde_json::from_value(crate::settings_defaults::document()["speed_args"].clone())
            .expect("Checked-in speed defaults must match the model")
    }
}

impl Default for AppAccelArgs {
    fn default() -> Self {
        serde_json::from_value(crate::settings_defaults::document()["accel_args"].clone())
            .expect("Checked-in acceleration defaults must match the model")
    }
}

impl AppProfile {
    pub fn to_native(&self) -> Result<models::profile, String> {
        validate_profile(self)?;
        let mut prof = models::profile::default();

        let mut name_u16 = [0u16; models::MAX_NAME_LEN];
        for (i, c) in self
            .name
            .encode_utf16()
            .enumerate()
            .take(models::MAX_NAME_LEN - 1)
        {
            name_u16[i] = c;
        }
        prof.name = name_u16;

        prof.accel_x = self.accel_x.to_native()?;
        prof.accel_y = self.accel_y.to_native()?;
        prof.degrees_rotation = self.degrees_rotation;
        prof.degrees_snap = self.degrees_snap;
        prof.speed_min = self.speed_min;
        prof.speed_max = self.speed_max;
        prof.lr_output_dpi_ratio = self.lr_output_dpi_ratio;
        prof.ud_output_dpi_ratio = self.ud_output_dpi_ratio;
        prof.output_dpi = self.output_dpi;
        prof.yx_output_dpi_ratio = self.yx_output_dpi_ratio;
        prof.range_weights = models::vec2d { x: self.range_weights.x, y: self.range_weights.y };
        prof.domain_weights = models::vec2d { x: self.domain_weights.x, y: self.domain_weights.y };

        prof.speed_processor_args = models::speed_args {
            whole: self.speed_processor_args.whole,
            lp_norm: self.speed_processor_args.lp_norm,
            input_speed_smooth_halflife: self.speed_processor_args.input_speed_smooth_halflife,
            scale_smooth_halflife: self.speed_processor_args.scale_smooth_halflife,
            output_speed_smooth_halflife: self.speed_processor_args.output_speed_smooth_halflife,
        };

        Ok(prof)
    }
}

impl AppAccelArgs {
    pub fn to_native(&self) -> Result<models::accel_args, String> {
        validate_accel_args("Curve", "", self)?;
        let mut args = models::accel_args::default();

        args.mode = match self.mode.as_str() {
            "classic" => models::accel_mode::classic,
            "linear" => models::accel_mode::classic,
            "jump" => models::accel_mode::jump,
            "natural" => models::accel_mode::natural,
            "synchronous" => models::accel_mode::synchronous,
            "power" => models::accel_mode::power,
            "lookup" => models::accel_mode::lookup,
            "tiered" => models::accel_mode::tiered,
            _ => models::accel_mode::noaccel,
        };
        args.gain = self.gain;
        args.input_offset = self.input_offset;
        args.output_offset = self.output_offset;
        args.acceleration = self.acceleration;
        args.decay_rate = self.decay_rate;
        args.gamma = self.gamma;
        args.motivity = self.motivity;
        args.scale = self.scale;
        args.exponent_power = self.exponent_power;
        args.limit = self.limit;
        args.sync_speed = self.sync_speed;
        args.smooth = self.smooth;
        // Linear is the UI alias for Classic with exponent 2.
        args.exponent_classic = if self.mode == "linear" { 2.0 } else { self.exponent_classic };
        args.t_type = match self.t_type.as_str() {
            "natural" => models::tiered_type::natural,
            _ => models::tiered_type::linear,
        };
        args.tiered_multiplier1 = self.tiered_multiplier1;
        args.tiered_input_offset1 = self.tiered_input_offset1;
        args.tiered_multiplier2 = self.tiered_multiplier2;
        args.tiered_transition1 = self.tiered_transition1;
        args.tiered_input_offset2 = self.tiered_input_offset2;
        args.tiered_multiplier3 = self.tiered_multiplier3;
        args.tiered_transition2 = self.tiered_transition2;
        args.tiered_decay_rate1 = self.tiered_decay_rate1;
        args.tiered_decay_rate2 = self.tiered_decay_rate2;
        args.cap = models::vec2d {
            x: self.cap.x,
            y: self.cap.y,
        };
        args.cap_mode = match self.cap_mode.as_str() {
            "in" => models::cap_mode::in_,
            "io" => models::cap_mode::io,
            _ => models::cap_mode::out,
        };

        let num_points = self.lookup_table.len();

        // `length` is the number of raw f32 values, not the number of points.
        // The C++ lookup implementation derives point count with `length / 2`.
        args.length = (num_points * 2) as i32;
        for i in 0..num_points {
            args.data[i * 2] = self.lookup_table[i].x as f32;
            args.data[i * 2 + 1] = self.lookup_table[i].y as f32;
        }

        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_defaults_match_native_acceleration_and_speed_defaults() {
        assert_eq!(crate::settings_defaults::document()["schema_version"], SETTINGS_SCHEMA_VERSION);
        let actual = AppAccelArgs::default().to_native().unwrap();
        let expected = models::accel_args::default();
        assert_eq!(actual.mode, expected.mode);
        assert_eq!(actual.gain, expected.gain);
        assert_eq!(actual.t_type, expected.t_type);
        assert_eq!(actual.cap_mode, expected.cap_mode);
        assert_eq!(actual.data, expected.data);
        assert_eq!(actual.length, expected.length);
        let values = |args: &models::accel_args| [
            args.input_offset, args.output_offset, args.acceleration, args.decay_rate,
            args.gamma, args.motivity, args.exponent_classic, args.scale,
            args.exponent_power, args.limit, args.sync_speed, args.smooth,
            args.tiered_multiplier1, args.tiered_input_offset1, args.tiered_multiplier2,
            args.tiered_transition1, args.tiered_input_offset2, args.tiered_multiplier3,
            args.tiered_transition2, args.tiered_decay_rate1, args.tiered_decay_rate2,
            args.cap.x, args.cap.y,
        ];
        assert_eq!(values(&actual), values(&expected));
        let speed = AppSpeedArgs::default();
        let native_speed = models::speed_args::default();
        assert_eq!(speed.whole, native_speed.whole);
        assert_eq!([speed.lp_norm, speed.input_speed_smooth_halflife, speed.scale_smooth_halflife, speed.output_speed_smooth_halflife],
            [native_speed.lp_norm, native_speed.input_speed_smooth_halflife, native_speed.scale_smooth_halflife, native_speed.output_speed_smooth_halflife]);
    }

    #[test]
    fn omitted_modern_fields_receive_the_same_canonical_defaults() {
        let value = normalize_settings_json(r#"{"profiles":[{"name":"Partial","accel_x":{"mode":"classic"}}]}"#).unwrap();
        let config: AppConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(config.profiles[0].accel_x.acceleration, 0.005);
        assert_eq!(config.profiles[0].accel_y.mode, "noaccel");
        assert!(config.profiles[0].speed_processor_args.whole);
        assert_eq!(config.profiles[0].output_dpi, 1000.0);
        assert_eq!(normalize_settings_json(&value.to_string()).unwrap(), value);
    }

    #[test]
    fn explicit_zero_false_and_existing_ids_are_not_replaced_by_defaults() {
        let raw = r#"{"profiles":[{"name":"Explicit","id":"keep-me","accel_x":{"mode":"noaccel","gain":false,"acceleration":0,"smooth":0},"speed_processor_args":{"whole":false}}]}"#;
        let value = normalize_settings_json(raw).unwrap();
        assert_eq!(value["profiles"][0]["id"], "keep-me");
        assert_eq!(value["profiles"][0]["accel_x"]["acceleration"], 0);
        assert_eq!(value["profiles"][0]["accel_x"]["smooth"], 0);
        assert_eq!(value["profiles"][0]["accel_x"]["gain"], false);
        assert_eq!(value["profiles"][0]["speed_processor_args"]["whole"], false);
    }

    #[test]
    fn generated_editor_ids_do_not_collide_or_change_name_based_mappings() {
        let raw = r#"{"profiles":[{"name":"First","id":"profile-2"},{"name":"Second"}],"devices":[{"id":"mouse","profile_id":"Second"}]}"#;
        let value = normalize_settings_json(raw).unwrap();
        assert_eq!(value["profiles"][0]["id"], "profile-2");
        assert_eq!(value["profiles"][1]["id"], "profile-3");
        assert_eq!(value["devices"][0]["profile_id"], "Second");
        assert_eq!(normalize_settings_json(&value.to_string()).unwrap(), value);
    }

    #[test]
    fn unknown_selectors_are_rejected_on_both_axes_even_when_combined() {
        for axis in ["accel_x", "accel_y"] {
            for field in ["mode", "cap_mode", "t_type"] {
                let mut value = normalize_settings_json(r#"{"profiles":[{"name":"Selectors"}]}"#).unwrap();
                value["profiles"][0][axis][field] = serde_json::json!("unsupported");
                assert!(normalize_settings_json(&value.to_string()).unwrap_err().contains("unknown"));
            }
        }
    }

    #[test]
    fn defaults_do_not_hide_wrong_types_nulls_or_missing_names() {
        for raw in [r#"{"profiles":[{}]}"#, r#"{"profiles":[{"name":"Bad","accel_x":null}]}"#,
            r#"{"profiles":[{"name":"Bad","speed_processor_args":"wrong"}]}"#,
            r#"{"profiles":[{"name":"Bad","accel_x":{"cap":null}}]}"#] {
            assert!(normalize_settings_json(raw).is_err(), "Accepted {raw}");
        }
    }

    #[test]
    fn unversioned_and_version_zero_documents_upgrade_idempotently() {
        let raw = include_str!("../tests/fixtures/settings/modern-complete.json");
        let expected = normalize_settings_json(raw).unwrap();
        assert_eq!(expected["schema_version"], SETTINGS_SCHEMA_VERSION);
        let mut zero: serde_json::Value = serde_json::from_str(raw).unwrap();
        zero["schema_version"] = serde_json::json!(0);
        assert_eq!(normalize_settings_json(&zero.to_string()).unwrap(), expected);
        assert_eq!(normalize_settings_json(&expected.to_string()).unwrap(), expected);
    }

    #[test]
    fn future_or_malformed_schema_markers_are_rejected() {
        let raw = include_str!("../tests/fixtures/settings/modern-complete.json");
        for marker in [serde_json::json!(2), serde_json::json!(-1), serde_json::json!(1.5), serde_json::json!("1"), serde_json::Value::Null] {
            let mut value: serde_json::Value = serde_json::from_str(raw).unwrap();
            value["schema_version"] = marker;
            assert!(normalize_settings_json(&value.to_string()).is_err());
        }
    }

    #[test]
    fn complete_modern_document_round_trip_preserves_tiered_axes_and_metadata() {
        let raw = include_str!("../tests/fixtures/settings/modern-complete.json");
        let mut original: serde_json::Value = serde_json::from_str(raw).unwrap();
        original["schema_version"] = serde_json::json!(SETTINGS_SCHEMA_VERSION);
        let document = normalize_settings_json(raw).unwrap();
        assert_eq!(document, original);
        let config = parse_or_migrate_settings(raw).unwrap();
        assert_eq!(config.profiles[0].accel_x.t_type, "linear");
        assert_eq!(config.profiles[1].accel_x.t_type, "natural");
        assert!(!config.profiles[0].speed_processor_args.whole);
        let args = &config.profiles[0].accel_y;
        assert_eq!(args.lookup_csv_backup.as_ref().unwrap()[0].y, 3.0);
        assert_eq!(args.lookup_csv_backup_gain, Some(false));
        assert_eq!(args.to_native().unwrap().data[..4], [1.0, 2.0, 8.0, 12.0]);
        assert_eq!(normalize_settings_json(&document.to_string()).unwrap(), original);
    }

    #[test]
    fn actual_wrapper_labels_preserve_device_options_and_independent_axes() {
        let raw = include_str!("../tests/fixtures/settings/legacy-wrapper.json");
        let document = normalize_settings_json(raw).unwrap();
        let config = parse_or_migrate_settings(raw).unwrap();
        let profile = &config.profiles[0];
        assert_eq!(profile.id, "keep-legacy-id");
        assert_eq!(profile.accel_x.mode, "natural");
        assert_eq!(profile.accel_x.input_offset, 3.0);
        assert_eq!(profile.accel_y.mode, "lookup");
        assert_eq!(profile.accel_y.to_native().unwrap().data[..4], [1.0, 2.0, 5.0, 3.0]);
        assert_eq!(profile.speed_processor_args.input_speed_smooth_halflife, 2.0);
        assert_eq!(profile.speed_processor_args.scale_smooth_halflife, 4.0);
        assert_eq!(profile.speed_processor_args.output_speed_smooth_halflife, 6.0);
        let device = &config.devices[0];
        assert!(device.disable && device.set_extra_info && device.poll_time_lock);
        assert_eq!(device.clamp_min, 0.125);
        assert_eq!(device.polling_rate, 1000);
        assert_eq!(device.profile_id.as_deref(), Some("Legacy Natural"));
        let defaults = config.default_device_config.unwrap().to_native().unwrap();
        assert_eq!(defaults.dpi, 800);
        assert!(defaults.set_extra_info && defaults.poll_time_lock);
        assert_eq!(defaults.clamp.max, 90.0);
        assert_eq!(document["editor_metadata"]["note"], "Preserved legacy extension");
        assert_eq!(normalize_settings_json(&document.to_string()).unwrap(), document);
    }

    #[test]
    fn legacy_wrong_types_and_incomplete_lut_pairs_are_rejected() {
        let original: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/settings/legacy-wrapper.json")).unwrap();
        for (field, value) in [("data", serde_json::json!([1,2,3])), ("data", serde_json::json!([1,"bad",3,4]))] {
            let mut invalid = original.clone();
            invalid["profiles"][0]["Vertical accel parameters"][field] = value;
            assert!(normalize_settings_json(&invalid.to_string()).is_err());
        }
        let mut invalid = original;
        invalid["devices"][0]["config"]["minimumTime"] = serde_json::json!("bad");
        assert!(normalize_settings_json(&invalid.to_string()).is_err());
    }

    #[test]
    fn invalid_modern_import_returns_validation_error_instead_of_migrating() {
        assert_eq!(
            parse_or_migrate_settings("{\"profiles\":[],\"devices\":[]}").unwrap_err(),
            "At least one profile is required"
        );
    }

    #[test]
    fn malformed_modern_fields_are_not_defaulted_by_legacy_migration() {
        let config = AppConfig {
            profiles: vec![AppProfile { name: "Modern".to_string(), ..Default::default() }],
            devices: vec![],

            default_device_config: None,
        };
        let mut value = serde_json::to_value(config).unwrap();
        value["profiles"][0]["accel_x"]["input_offset"] = serde_json::json!("bad number");
        let raw = value.to_string();
        let error = parse_or_migrate_settings(&raw).unwrap_err();
        assert!(error.starts_with("Invalid modern settings:"));
        assert!(error.contains("f64"));
        assert_eq!(parse_settings_for_load(&raw).unwrap_err(), error);
    }

    #[test]
    fn mixed_legacy_and_modern_documents_are_rejected() {
        let raw = r#"{"profiles":[{"argsX":{"mode":"natural"}}, {"accel_x":{"mode":"tiered"}}]}"#;
        assert!(parse_or_migrate_settings(raw).unwrap_err().contains("Mixed modern and legacy"));
    }

    #[test]
    fn unsupported_document_shapes_are_rejected() {
        for raw in ["[]", "{}", r#"{"profiles":null}"#, r#"{"profiles":[null]}"#] {
            assert!(parse_or_migrate_settings(raw).is_err(), "Accepted {raw}");
        }
    }

    #[test]
    fn loading_invalid_modern_settings_preserves_validation_error() {
        let raw = "{\"profiles\":[],\"devices\":[]}";
        assert_eq!(
            parse_settings_for_load(raw).unwrap_err(),
            "At least one profile is required"
        );
    }

    #[test]
    fn loading_modern_settings_preserves_unknown_editor_metadata_and_format() {
        let config = AppConfig {
            profiles: vec![AppProfile {
                id: "stable-id".to_string(),
                name: "Portable".to_string(),
                ..Default::default()
            }],
            devices: vec![],

            default_device_config: None,
        };
        let mut value = serde_json::to_value(config).unwrap();
        value["schema_version"] = serde_json::json!(SETTINGS_SCHEMA_VERSION);
        value["profiles"][0]["accel_x"]["lookup_csv_backup"] =
            serde_json::json!([{ "x": 1, "y": 2 }, { "x": 3, "y": 4 }]);
        let raw = serde_json::to_string_pretty(&value).unwrap();
        assert_eq!(parse_settings_for_load(&raw).unwrap(), raw);
    }

    #[test]
    fn lookup_points_are_written_as_driver_pairs() {
        let args = AppAccelArgs {
            mode: "lookup".to_string(),
            gain: true,
            lookup_table: vec![AppVec2D { x: 1.0, y: 2.0 }, AppVec2D { x: 4.0, y: 8.0 }],
            ..Default::default()
        };

        let native = args.to_native().unwrap();
        assert_eq!(native.length, 4);
        assert_eq!(native.data[..4], [1.0, 2.0, 4.0, 8.0]);
        assert!(native.gain);
    }

    #[test]
    fn lookup_validation_rejects_duplicate_input_speeds() {
        let args = AppAccelArgs {
            mode: "lookup".to_string(),
            lookup_table: vec![AppVec2D { x: 1.0, y: 1.0 }, AppVec2D { x: 1.0, y: 2.0 }],
            ..Default::default()
        };

        assert!(validate_lookup_table("test", "X", &args).is_err());
    }

    #[test]
    fn profile_validation_rejects_empty_profiles() {
        let config = AppConfig {
            profiles: vec![],
            devices: vec![],

            default_device_config: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn profile_validation_rejects_duplicate_names() {
        let config = AppConfig {
            profiles: vec![
                AppProfile {
                    name: "Gaming".to_string(),
                    ..Default::default()
                },
                AppProfile {
                    name: "gaming".to_string(),
                    ..Default::default()
                },
            ],
            devices: vec![],

            default_device_config: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn profile_validation_rejects_invalid_device_mapping() {
        let config = AppConfig {
            profiles: vec![AppProfile {
                name: "Gaming".to_string(),
                ..Default::default()
            }],
            devices: vec![AppDeviceConfig {
                id: "DEV123".to_string(),
                profile_id: Some("NonExistent".to_string()),
                ..Default::default()
            }],
            default_device_config: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn profile_validation_accepts_valid_multi_profile_mapping() {
        let config = AppConfig {
            profiles: vec![
                AppProfile {
                    name: "Gaming".to_string(),
                    ..Default::default()
                },
                AppProfile {
                    name: "Desktop".to_string(),
                    ..Default::default()
                },
            ],
            devices: vec![
                AppDeviceConfig {
                    id: "DEV_MOUSE1".to_string(),
                    profile_id: Some("Gaming".to_string()),
                    ..Default::default()
                },
                AppDeviceConfig {
                    id: "DEV_MOUSE2".to_string(),
                    profile_id: Some("Desktop".to_string()),
                    ..Default::default()
                },
                AppDeviceConfig {
                    id: "DEV_TRACKPAD".to_string(),
                    profile_id: None,
                    disable: true,
                    ..Default::default()
                },
            ],
            default_device_config: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn local_settings_path_resolves_json() {
        let path = super::get_local_settings_path();
        assert!(path.ends_with("settings.json"));
    }

    #[test]
    fn appdata_settings_path_resolves_rawaccel_subfolder() {
        if let Some(path) = super::get_appdata_settings_path() {
            assert!(path.ends_with("RawAccel\\settings.json") || path.ends_with("RawAccel/settings.json"));
        }
    }

    #[test]
    fn legacy_settings_migration_succeeds() {
        let legacy_json = r####"{
            "### Cap modes ###": "in_out | input | output",
            "### Accel modes ###": "classic | jump | natural | synchronous | power | lut | noaccel",
            "version": "v1.6.1",
            "defaultDeviceConfig": {
                "disable": false,
                "dpi": 1600,
                "pollingRate": 1000
            },
            "profiles": [
                {
                    "name": "Old Profile",
                    "domainXY": { "x": 1.0, "y": 1.0 },
                    "rangeXY": { "x": 1.0, "y": 1.0 },
                    "argsX": {
                        "mode": "natural",
                        "Gain / Velocity": true,
                        "decayRate": 0.2,
                        "limit": 2.0,
                        "inputOffset": 5.0,
                        "Cap / Jump": { "x": 15.0, "y": 1.5 },
                        "Cap mode": "output"
                    },
                    "argsY": {
                        "mode": "natural",
                        "Gain / Velocity": true,
                        "decayRate": 0.2,
                        "limit": 2.0,
                        "inputOffset": 5.0,
                        "Cap / Jump": { "x": 15.0, "y": 1.5 },
                        "Cap mode": "output"
                    },
                    "inputSpeedArgs": {
                        "Whole/combined accel (set false for 'by component' mode)": true,
                        "lpNorm": 2.0
                    },
                    "Output DPI": 1600.0,
                    "Y/X output DPI ratio (vertical sens multiplier)": 1.0,
                    "Degrees of rotation": 0.0,
                    "Degrees of angle snapping": 0.0,
                    "Input Speed Cap": 0.0
                }
            ],
            "devices": [
                {
                    "id": "HID\\VID_1234&PID_5678",
                    "profile": "Old Profile",
                    "config": {
                        "disable": false,
                        "dpi": 1600,
                        "pollingRate": 1000
                    }
                }
            ]
        }"####;

        let migrated = super::parse_or_migrate_settings(legacy_json);
        assert!(migrated.is_ok(), "Failed to migrate legacy settings: {:?}", migrated.err());
        let config = migrated.unwrap();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "Old Profile");
        assert_eq!(config.profiles[0].accel_x.mode, "natural");
        assert_eq!(config.profiles[0].accel_x.cap_mode, "out");
        assert_eq!(config.profiles[0].speed_processor_args.whole, true);
        assert_eq!(config.devices.len(), 1);
        assert_eq!(config.devices[0].profile_id, Some("Old Profile".to_string()));
    }

    #[test]
    fn tiered_validation_rejects_zero_decay_rate() {
        let config = AppConfig {
            profiles: vec![AppProfile {
                name: "Tiered Profile".to_string(),
                accel_x: AppAccelArgs {
                    mode: "tiered".to_string(),
                    t_type: "natural".to_string(),
                    tiered_multiplier1: 1.0,
                    tiered_input_offset1: 5.0,
                    tiered_multiplier2: 1.5,
                    tiered_input_offset2: 15.0,
                    tiered_multiplier3: 2.0,
                    tiered_decay_rate1: 0.0, // Invalid
                    tiered_decay_rate2: 0.1,
                    ..Default::default()
                },
                ..Default::default()
            }],
            devices: vec![],

            default_device_config: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn tiered_validation_rejects_backward_offsets() {
        let config = AppConfig {
            profiles: vec![AppProfile {
                name: "Tiered Profile".to_string(),
                accel_x: AppAccelArgs {
                    mode: "tiered".to_string(),
                    t_type: "natural".to_string(),
                    tiered_multiplier1: 1.0,
                    tiered_input_offset1: 20.0,
                    tiered_multiplier2: 1.5,
                    tiered_input_offset2: 10.0, // Backward
                    tiered_multiplier3: 2.0,
                    tiered_decay_rate1: 0.1,
                    tiered_decay_rate2: 0.1,
                    ..Default::default()
                },
                ..Default::default()
            }],
            devices: vec![],

            default_device_config: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn tiered_validation_accepts_valid_config() {
        let config = AppConfig {
            profiles: vec![AppProfile {
                name: "Tiered Profile".to_string(),
                accel_x: AppAccelArgs {
                    mode: "tiered".to_string(),
                    t_type: "natural".to_string(),
                    tiered_multiplier1: 1.0,
                    tiered_input_offset1: 5.0,
                    tiered_multiplier2: 1.5,
                    tiered_input_offset2: 15.0,
                    tiered_multiplier3: 2.0,
                    tiered_decay_rate1: 0.1,
                    tiered_decay_rate2: 0.1,
                    ..Default::default()
                },
                ..Default::default()
            }],
            devices: vec![],

            default_device_config: None,
        };
        assert!(config.validate().is_ok());
    }
}
