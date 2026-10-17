use crate::models;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

fn parse_vec2(val: Option<&serde_json::Value>, default_x: f64, default_y: f64) -> AppVec2D {
    if let Some(v) = val {
        let x = v.get("x").and_then(|n| n.as_f64()).unwrap_or(default_x);
        let y = v.get("y").and_then(|n| n.as_f64()).unwrap_or(default_y);
        AppVec2D { x, y }
    } else {
        AppVec2D { x: default_x, y: default_y }
    }
}

fn parse_accel_args(val: Option<&serde_json::Value>) -> AppAccelArgs {
    let mut args = AppAccelArgs::default();
    if let Some(v) = val {
        if let Some(m) = v.get("mode").and_then(|m| m.as_str()) {
            args.mode = if m == "lut" { "lookup".to_string() } else { m.to_string() };
        }

        if let Some(gain) = v.get("Gain / Velocity").or_else(|| v.get("gain")).and_then(|g| g.as_bool()) {
            args.gain = gain;
        }

        if let Some(n) = v.get("inputOffset").or_else(|| v.get("input_offset")).and_then(|n| n.as_f64()) {
            args.input_offset = n;
        }
        if let Some(n) = v.get("outputOffset").or_else(|| v.get("output_offset")).and_then(|n| n.as_f64()) {
            args.output_offset = n;
        }
        if let Some(n) = v.get("acceleration").and_then(|n| n.as_f64()) {
            args.acceleration = n;
        }
        if let Some(n) = v.get("decayRate").or_else(|| v.get("decay_rate")).and_then(|n| n.as_f64()) {
            args.decay_rate = n;
        }
        if let Some(n) = v.get("gamma").and_then(|n| n.as_f64()) {
            args.gamma = n;
        }
        if let Some(n) = v.get("motivity").and_then(|n| n.as_f64()) {
            args.motivity = n;
        }
        if let Some(n) = v.get("exponentClassic").or_else(|| v.get("exponent_classic")).and_then(|n| n.as_f64()) {
            args.exponent_classic = n;
        }
        if let Some(n) = v.get("scale").and_then(|n| n.as_f64()) {
            args.scale = n;
        }
        if let Some(n) = v.get("exponentPower").or_else(|| v.get("exponent_power")).and_then(|n| n.as_f64()) {
            args.exponent_power = n;
        }
        if let Some(n) = v.get("limit").and_then(|n| n.as_f64()) {
            args.limit = n;
        }
        if let Some(n) = v.get("syncSpeed").or_else(|| v.get("sync_speed")).and_then(|n| n.as_f64()) {
            args.sync_speed = n;
        }
        if let Some(n) = v.get("smooth").and_then(|n| n.as_f64()) {
            args.smooth = n;
        }

        args.cap = parse_vec2(v.get("Cap / Jump").or_else(|| v.get("cap")), 0.0, 0.0);

        if let Some(cm) = v.get("Cap mode").or_else(|| v.get("capMode")).or_else(|| v.get("cap_mode")).and_then(|s| s.as_str()) {
            args.cap_mode = match cm {
                "in_out" | "io" => "io".to_string(),
                "input" | "in" => "in".to_string(),
                "output" | "out" => "out".to_string(),
                other => other.to_string(),
            };
        }

        // Parse legacy LUT data: flat array [x1, y1, x2, y2, ...]
        if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
            let mut pts = Vec::new();
            let mut i = 0;
            while i + 1 < arr.len() {
                if let (Some(x), Some(y)) = (arr[i].as_f64(), arr[i + 1].as_f64()) {
                    pts.push(AppVec2D { x, y });
                }
                i += 2;
            }
            if pts.len() >= 2 {
                args.lookup_table = pts;
            }
        }
    }
    args
}

fn parse_speed_args(val: Option<&serde_json::Value>) -> AppSpeedArgs {
    let mut args = AppSpeedArgs::default();
    if let Some(v) = val {
        if let Some(whole) = v.get("Whole/combined accel (set false for 'by component' mode)")
            .or_else(|| v.get("combineMagnitudes"))
            .or_else(|| v.get("whole"))
            .and_then(|b| b.as_bool())
        {
            args.whole = whole;
        }

        if let Some(n) = v.get("lpNorm").or_else(|| v.get("lp_norm")).and_then(|n| n.as_f64()) {
            args.lp_norm = n;
        }
        if let Some(n) = v.get("inputSmoothHalflife").or_else(|| v.get("input_speed_smooth_halflife")).and_then(|n| n.as_f64()) {
            args.input_speed_smooth_halflife = n;
        }
        if let Some(n) = v.get("scaleSmoothHalflife").or_else(|| v.get("scale_smooth_halflife")).and_then(|n| n.as_f64()) {
            args.scale_smooth_halflife = n;
        }
        if let Some(n) = v.get("outputSmoothHalflife").or_else(|| v.get("output_speed_smooth_halflife")).and_then(|n| n.as_f64()) {
            args.output_speed_smooth_halflife = n;
        }
    }
    args
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

/// Select the format by its keys, never by whether validation happens to pass.
pub fn parse_or_migrate_settings(raw_json: &str) -> Result<AppConfig, String> {
    let val: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;
    if detect_settings_format(&val)? == SettingsFormat::Modern {
        let config: AppConfig = serde_json::from_value(val)
            .map_err(|error| format!("Invalid modern settings: {error}"))?;
        config.validate()?;
        return Ok(config);
    }

    if let Some(profiles_val) = val.get("profiles").and_then(|p| p.as_array()) {
        let mut modern_profiles = Vec::new();

        for (idx, p_val) in profiles_val.iter().enumerate() {
            let name = p_val.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(&format!("Profile {}", idx + 1))
                .to_string();

            let domain_xy = parse_vec2(p_val.get("domainXY").or_else(|| p_val.get("domain_weights")), 1.0, 1.0);
            let range_xy = parse_vec2(p_val.get("rangeXY").or_else(|| p_val.get("range_weights")), 1.0, 1.0);

            let accel_x = parse_accel_args(p_val.get("argsX").or_else(|| p_val.get("accel_x")));
            let accel_y = parse_accel_args(p_val.get("argsY").or_else(|| p_val.get("accel_y")));

            let speed_processor_args = parse_speed_args(p_val.get("inputSpeedArgs").or_else(|| p_val.get("speed_processor_args")));

            let output_dpi = p_val.get("Output DPI")
                .or_else(|| p_val.get("outputDPI"))
                .or_else(|| p_val.get("output_dpi"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1600.0);

            let yx_output_dpi_ratio = p_val.get("Y/X output DPI ratio (vertical sens multiplier)")
                .or_else(|| p_val.get("yxOutputDPIRatio"))
                .or_else(|| p_val.get("yx_output_dpi_ratio"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);

            let lr_output_dpi_ratio = p_val.get("L/R output DPI ratio (left sens multiplier)")
                .or_else(|| p_val.get("lrOutputDPIRatio"))
                .or_else(|| p_val.get("lr_output_dpi_ratio"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);

            let ud_output_dpi_ratio = p_val.get("U/D output DPI ratio (up sens multiplier)")
                .or_else(|| p_val.get("udOutputDPIRatio"))
                .or_else(|| p_val.get("ud_output_dpi_ratio"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);

            let degrees_rotation = p_val.get("Degrees of rotation")
                .or_else(|| p_val.get("rotation"))
                .or_else(|| p_val.get("degrees_rotation"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let degrees_snap = p_val.get("Degrees of angle snapping")
                .or_else(|| p_val.get("snap"))
                .or_else(|| p_val.get("degrees_snap"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let speed_max = p_val.get("Input Speed Cap")
                .or_else(|| p_val.get("maximumSpeed"))
                .or_else(|| p_val.get("speed_max"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            modern_profiles.push(AppProfile {
                id: format!("profile-{}", idx + 1),
                name,
                domain_weights: domain_xy,
                range_weights: range_xy,
                accel_x,
                accel_y,
                speed_processor_args,
                output_dpi,
                yx_output_dpi_ratio,
                lr_output_dpi_ratio,
                ud_output_dpi_ratio,
                degrees_rotation,
                degrees_snap,
                speed_min: 0.0,
                speed_max,
            });
        }

        let mut modern_devices = Vec::new();
        if let Some(devices_val) = val.get("devices").and_then(|d| d.as_array()) {
            for dev_val in devices_val {
                if let Some(id) = dev_val.get("id").and_then(|i| i.as_str()) {
                    let profile_id = dev_val.get("profile")
                        .or_else(|| dev_val.get("profile_id"))
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string());

                    let cfg = dev_val.get("config").unwrap_or(dev_val);
                    let disable = cfg.get("disable").and_then(|v| v.as_bool()).unwrap_or(false);
                    let dpi = cfg.get("DPI (normalizes input speed unit: counts/ms -> in/s)")
                        .or_else(|| cfg.get("dpi"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        .max(0) as u32;
                    let polling_rate = cfg.get("Polling rate Hz (keep at 0 for automatic adjustment)")
                        .or_else(|| cfg.get("pollingRate"))
                        .or_else(|| cfg.get("polling_rate"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        .max(0) as u32;

                    modern_devices.push(AppDeviceConfig {
                        id: id.to_string(),
                        profile_id,
                        disable,
                        set_extra_info: false,
                        poll_time_lock: false,
                        dpi,
                        polling_rate,
                        clamp_min: 0.0,
                        clamp_max: 0.0,
                    });
                }
            }
        }

        if !modern_profiles.is_empty() {
            let config = AppConfig {
                profiles: modern_profiles,
                devices: modern_devices,
            };
            config.validate()?;
            return Ok(config);
        }
    }

    Err("Could not parse configuration. Ensure it is a valid Raw Accel JSON file.".to_string())
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
    let config = parse_or_migrate_settings(raw)?;
    match format {
        SettingsFormat::Modern => Ok(raw.to_string()),
        // Migration is in memory only; explicit saving retains the original.
        SettingsFormat::Legacy => serde_json::to_string_pretty(&config)
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
}

impl AppConfig {
    /// Validates data that is used directly by the driver.
    /// Ensures profiles exist, have unique non-empty names, and mapped devices reference existing profiles.
    pub fn validate(&self) -> Result<(), String> {
        if self.profiles.is_empty() {
            return Err("At least one profile is required".to_string());
        }

        let mut profile_names = std::collections::HashSet::new();
        for profile in &self.profiles {
            let trimmed = profile.name.trim();
            if trimmed.is_empty() {
                return Err("Profile name cannot be empty".to_string());
            }
            if !profile_names.insert(trimmed.to_lowercase()) {
                return Err(format!("Duplicate profile name '{}'", profile.name));
            }

            validate_accel_args(&profile.name, "X", &profile.accel_x)?;

            if !profile.speed_processor_args.whole {
                validate_accel_args(&profile.name, "Y", &profile.accel_y)?;
            }
        }

        for dev in &self.devices {
            if let Some(ref pid) = dev.profile_id {
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

fn validate_lookup_table(
    profile_name: &str,
    axis: &str,
    args: &AppAccelArgs,
) -> Result<(), String> {
    if args.mode != "lookup" {
        return Ok(());
    }

    let points = &args.lookup_table;
    let max_points = models::LUT_POINTS_CAPACITY;
    if points.len() < 2 {
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
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(format!(
                "Profile '{profile_name}' {axis} lookup point {} must contain finite numbers",
                index + 1
            ));
        }
        if point.x <= 0.0 {
            return Err(format!(
                "Profile '{profile_name}' {axis} lookup point {} must have an input speed above zero",
                index + 1
            ));
        }
        if index > 0 && point.x <= previous_x {
            return Err(format!(
                "Profile '{profile_name}' {axis} lookup input speeds must be strictly increasing"
            ));
        }
        previous_x = point.x;
    }

    Ok(())
}

fn validate_accel_args(
    profile_name: &str,
    axis: &str,
    args: &AppAccelArgs,
) -> Result<(), String> {
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
        "lookup" => {
            validate_lookup_table(profile_name, axis, args)?;
        }
        "tiered" => {
            if args.tiered_multiplier1 <= 0.0 || args.tiered_multiplier2 <= 0.0 || args.tiered_multiplier3 <= 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: tiered multipliers must be positive"));
            }
            if args.tiered_input_offset1 < 0.0 || args.tiered_input_offset2 < 0.0 {
                return Err(format!("Profile '{profile_name}' {axis}: offset can not be negative"));
            }
            if args.t_type == "linear" {
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppDeviceConfig {
    pub id: String,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
}

impl AppProfile {
    pub fn to_native(&self) -> models::profile {
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

        prof.accel_x = self.accel_x.to_native();
        prof.accel_y = self.accel_y.to_native();
        prof.degrees_rotation = self.degrees_rotation;
        prof.degrees_snap = self.degrees_snap;
        prof.speed_min = self.speed_min;
        prof.speed_max = self.speed_max;
        prof.lr_output_dpi_ratio = if self.lr_output_dpi_ratio <= 0.0 {
            1.0
        } else {
            self.lr_output_dpi_ratio
        };
        prof.ud_output_dpi_ratio = if self.ud_output_dpi_ratio <= 0.0 {
            1.0
        } else {
            self.ud_output_dpi_ratio
        };
        prof.output_dpi = if self.output_dpi <= 0.0 {
            1000.0
        } else {
            self.output_dpi
        };
        prof.yx_output_dpi_ratio = if self.yx_output_dpi_ratio <= 0.0 {
            1.0
        } else {
            self.yx_output_dpi_ratio
        };
        prof.range_weights = models::vec2d {
            x: if self.range_weights.x < 0.0 {
                1.0
            } else {
                self.range_weights.x
            },
            y: if self.range_weights.y < 0.0 {
                1.0
            } else {
                self.range_weights.y
            },
        };
        prof.domain_weights = models::vec2d {
            x: if self.domain_weights.x <= 0.0 {
                1.0
            } else {
                self.domain_weights.x
            },
            y: if self.domain_weights.y <= 0.0 {
                1.0
            } else {
                self.domain_weights.y
            },
        };

        prof.speed_processor_args = models::speed_args {
            whole: self.speed_processor_args.whole,
            lp_norm: if self.speed_processor_args.lp_norm <= 0.0 {
                2.0
            } else {
                self.speed_processor_args.lp_norm
            },
            input_speed_smooth_halflife: self.speed_processor_args.input_speed_smooth_halflife,
            scale_smooth_halflife: self.speed_processor_args.scale_smooth_halflife,
            output_speed_smooth_halflife: self.speed_processor_args.output_speed_smooth_halflife,
        };

        prof
    }
}

impl AppAccelArgs {
    pub fn to_native(&self) -> models::accel_args {
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
        args.acceleration = if self.acceleration <= 0.0 {
            0.001
        } else {
            self.acceleration
        };
        args.decay_rate = if self.decay_rate <= 0.0 {
            0.1
        } else {
            self.decay_rate
        };
        args.gamma = if self.gamma <= 0.0 { 1.0 } else { self.gamma };
        args.motivity = if self.motivity <= 1.0 {
            1.5
        } else {
            self.motivity
        };
        args.exponent_classic = if self.mode.as_str() == "linear" {
            2.0
        } else if self.exponent_classic <= 1.0 {
            2.0
        } else {
            self.exponent_classic
        };
        args.scale = if self.scale <= 0.0 { 1.0 } else { self.scale };
        args.exponent_power = if self.exponent_power <= 0.0 {
            0.05
        } else {
            self.exponent_power
        };
        args.limit = if self.limit <= 0.0 { 2.0 } else { self.limit };
        args.sync_speed = if self.sync_speed <= 0.0 {
            5.0
        } else {
            self.sync_speed
        };
        args.smooth = if self.smooth < 0.0 || self.smooth > 1.0 {
            0.0
        } else {
            self.smooth
        };
        args.t_type = match self.t_type.as_str() {
            "natural" => models::tiered_type::natural,
            _ => models::tiered_type::linear,
        };
        args.tiered_multiplier1 = if self.tiered_multiplier1 <= 0.0 { 1.0 } else { self.tiered_multiplier1 };
        args.tiered_input_offset1 = if self.tiered_input_offset1 < 0.0 { 0.0 } else { self.tiered_input_offset1 };
        args.tiered_multiplier2 = if self.tiered_multiplier2 <= 0.0 { 1.5 } else { self.tiered_multiplier2 };
        args.tiered_transition1 = if self.tiered_transition1 <= 0.0 { 10.0 } else { self.tiered_transition1 };
        args.tiered_input_offset2 = if self.tiered_input_offset2 <= self.tiered_input_offset1 { self.tiered_input_offset1 + 15.0 } else { self.tiered_input_offset2 };
        args.tiered_multiplier3 = if self.tiered_multiplier3 <= 0.0 { 2.0 } else { self.tiered_multiplier3 };
        args.tiered_transition2 = if self.tiered_transition2 <= 0.0 { 25.0 } else { self.tiered_transition2 };
        args.tiered_decay_rate1 = if self.tiered_decay_rate1 <= 0.0 { 0.1 } else { self.tiered_decay_rate1 };
        args.tiered_decay_rate2 = if self.tiered_decay_rate2 <= 0.0 { 0.1 } else { self.tiered_decay_rate2 };
        args.cap = models::vec2d {
            x: self.cap.x,
            y: self.cap.y,
        };
        args.cap_mode = match self.cap_mode.as_str() {
            "in" => models::cap_mode::in_,
            "io" => models::cap_mode::io,
            _ => models::cap_mode::out,
        };

        let max_points = models::LUT_RAW_DATA_CAPACITY / 2;
        let num_points = std::cmp::min(self.lookup_table.len(), max_points);

        // `length` is the number of raw f32 values, not the number of points.
        // The C++ lookup implementation derives point count with `length / 2`.
        args.length = (num_points * 2) as i32;
        for i in 0..num_points {
            args.data[i * 2] = self.lookup_table[i].x as f32;
            args.data[i * 2 + 1] = self.lookup_table[i].y as f32;
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                name: "Portable".to_string(),
                ..Default::default()
            }],
            devices: vec![],
        };
        let mut value = serde_json::to_value(config).unwrap();
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

        let native = args.to_native();
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
        };
        assert!(config.validate().is_ok());
    }
}
