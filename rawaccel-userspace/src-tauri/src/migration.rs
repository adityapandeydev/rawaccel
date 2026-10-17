//! In-memory migration of the C# wrapper's settings format. Unknown metadata is
//! retained; malformed values and conflicting aliases are never defaulted away.
use serde_json::{json, Map, Value};

type Object = Map<String, Value>;

fn object(value: &mut Value) -> Result<&mut Object, String> {
    value
        .as_object_mut()
        .ok_or_else(|| "Expected an object".to_string())
}

fn aliases(map: &mut Object, target: &str, names: &[&str]) -> Result<(), String> {
    for name in names {
        if let Some(value) = map.remove(*name) {
            if let Some(existing) = map.get(target) {
                if existing != &value {
                    return Err(format!("Conflicting values for '{target}' and '{name}'"));
                }
            } else {
                map.insert(target.to_string(), value);
            }
        }
    }
    Ok(())
}

fn fill(map: &mut Object, defaults: Value) {
    for (key, value) in defaults.as_object().expect("Object defaults") {
        map.entry(key.clone()).or_insert_with(|| value.clone());
    }
}

fn accel(value: &mut Value) -> Result<(), String> {
    let map = object(value)?;
    for (key, names) in [
        ("gain", &["Gain / Velocity"][..]),
        ("input_offset", &["inputOffset"][..]),
        ("output_offset", &["outputOffset"][..]),
        ("decay_rate", &["decayRate"][..]),
        ("exponent_classic", &["exponentClassic"][..]),
        ("exponent_power", &["exponentPower"][..]),
        ("sync_speed", &["syncSpeed"][..]),
        ("cap", &["Cap / Jump"][..]),
        ("cap_mode", &["Cap mode", "capMode"][..]),
        ("t_type", &["tieredType", "tType"][..]),
    ] {
        aliases(map, key, names)?;
    }
    if map.get("mode").and_then(Value::as_str) == Some("lut") {
        map.insert("mode".to_string(), json!("lookup"));
    }
    if let Some(mode) = map.get_mut("cap_mode") {
        match mode.as_str() {
            Some("in_out") => *mode = json!("io"),
            Some("input") => *mode = json!("in"),
            Some("output") => *mode = json!("out"),
            _ => {}
        }
    }
    if let Some(data) = map.remove("data") {
        let values = data.as_array().ok_or("Legacy LUT data must be an array")?;
        let length = match map.remove("length") {
            Some(length) => usize::try_from(
                length
                    .as_u64()
                    .ok_or("Legacy LUT length must be an integer")?,
            )
            .map_err(|_| "Legacy LUT length exceeds the supported range")?,
            None => values.len(),
        };
        if length > values.len() || length % 2 != 0 {
            return Err(
                "Legacy LUT data must contain complete X/Y pairs within its declared length"
                    .to_string(),
            );
        }
        let mut points = Vec::new();
        for pair in values[..length].chunks_exact(2) {
            if !pair[0].is_number() || !pair[1].is_number() {
                return Err("Legacy LUT points must contain numbers".to_string());
            }
            points.push(json!({"x": pair[0], "y": pair[1]}));
        }
        let table = Value::Array(points);
        if let Some(existing) = map.get("lookup_table") {
            if existing != &table {
                return Err("Conflicting legacy data and lookup_table".to_string());
            }
        } else {
            map.insert("lookup_table".to_string(), table);
        }
    }
    fill(
        map,
        crate::settings_defaults::document()["accel_args"].clone(),
    );
    Ok(())
}

fn speed(value: &mut Value) -> Result<(), String> {
    let map = object(value)?;
    for (key, names) in [
        (
            "whole",
            &[
                "Whole/combined accel (set false for 'by component' mode)",
                "combineMagnitudes",
            ][..],
        ),
        ("lp_norm", &["lpNorm"][..]),
        (
            "input_speed_smooth_halflife",
            &[
                "inputSmoothHalflife",
                "Time in ms after which an input is weighted at half its original value.",
            ][..],
        ),
        (
            "scale_smooth_halflife",
            &[
                "scaleSmoothHalflife",
                "Time in ms after which scale is weighted at half its original value.",
            ][..],
        ),
        (
            "output_speed_smooth_halflife",
            &[
                "outputSmoothHalflife",
                "Time in ms after which an output is weighted at half its original value.",
            ][..],
        ),
    ] {
        aliases(map, key, names)?;
    }
    fill(
        map,
        crate::settings_defaults::document()["speed_args"].clone(),
    );
    Ok(())
}

fn device_options(map: &mut Object) -> Result<(), String> {
    for (key, names) in [
        ("set_extra_info", &["setExtraInfo"][..]),
        (
            "poll_time_lock",
            &[
                "pollTimeLock",
                "Use constant time interval based on polling rate",
            ][..],
        ),
        (
            "dpi",
            &["DPI (normalizes input speed unit: counts/ms -> in/s)"][..],
        ),
        (
            "polling_rate",
            &[
                "pollingRate",
                "Polling rate Hz (keep at 0 for automatic adjustment)",
            ][..],
        ),
        ("clamp_min", &["minimumTime"][..]),
        ("clamp_max", &["maximumTime"][..]),
    ] {
        aliases(map, key, names)?;
    }
    Ok(())
}

pub(crate) fn migrate(mut value: Value) -> Result<Value, String> {
    let root = object(&mut value)?;
    aliases(root, "default_device_config", &["defaultDeviceConfig"])?;
    if let Some(defaults) = root.get_mut("default_device_config") {
        device_options(object(defaults)?)?;
    }
    let profiles = root
        .get_mut("profiles")
        .and_then(Value::as_array_mut)
        .ok_or("Legacy settings must contain a profiles array")?;
    for (index, profile) in profiles.iter_mut().enumerate() {
        let map = object(profile)?;
        for (key, names) in [
            (
                "domain_weights",
                &[
                    "domainXY",
                    "Stretches domain for horizontal vs vertical inputs",
                ][..],
            ),
            (
                "range_weights",
                &[
                    "rangeXY",
                    "Stretches accel range for horizontal vs vertical inputs",
                ][..],
            ),
            (
                "accel_x",
                &["argsX", "Whole or horizontal accel parameters"][..],
            ),
            ("accel_y", &["argsY", "Vertical accel parameters"][..]),
            (
                "speed_processor_args",
                &["inputSpeedArgs", "Input speed calculation parameters"][..],
            ),
            ("output_dpi", &["outputDPI", "Output DPI"][..]),
            (
                "yx_output_dpi_ratio",
                &[
                    "yxOutputDPIRatio",
                    "Y/X output DPI ratio (vertical sens multiplier)",
                ][..],
            ),
            (
                "lr_output_dpi_ratio",
                &[
                    "lrOutputDPIRatio",
                    "L/R output DPI ratio (left sens multiplier)",
                ][..],
            ),
            (
                "ud_output_dpi_ratio",
                &[
                    "udOutputDPIRatio",
                    "U/D output DPI ratio (up sens multiplier)",
                ][..],
            ),
            ("degrees_rotation", &["rotation", "Degrees of rotation"][..]),
            ("degrees_snap", &["snap", "Degrees of angle snapping"][..]),
            ("speed_min", &["minimumSpeed"][..]),
            ("speed_max", &["maximumSpeed", "Input Speed Cap"][..]),
        ] {
            aliases(map, key, names)
                .map_err(|error| format!("Legacy profile {}: {error}", index + 1))?;
        }
        if !map.contains_key("accel_x") {
            return Err(format!(
                "Legacy profile {} is missing horizontal acceleration parameters",
                index + 1
            ));
        }
        let mut defaults = crate::settings_defaults::profile();
        defaults["name"] = json!(format!("Profile {}", index + 1));
        fill(map, defaults);
        accel(map.get_mut("accel_x").unwrap())?;
        accel(map.get_mut("accel_y").unwrap())?;
        speed(map.get_mut("speed_processor_args").unwrap())?;
    }
    if let Some(devices) = root.get_mut("devices") {
        for device in devices
            .as_array_mut()
            .ok_or("Legacy devices must be an array")?
        {
            let map = object(device)?;
            aliases(map, "profile_id", &["profile"])?;
            if map.get("profile_id").and_then(Value::as_str) == Some("") {
                map.insert("profile_id".to_string(), Value::Null);
            }
            if let Some(config) = map.remove("config") {
                let config = config
                    .as_object()
                    .ok_or("Legacy device config must be an object")?;
                for (key, value) in config {
                    if let Some(existing) = map.get(key) {
                        if existing != value {
                            return Err(format!("Conflicting device option '{key}'"));
                        }
                    } else {
                        map.insert(key.clone(), value.clone());
                    }
                }
            }
            device_options(map)?;
        }
    }
    Ok(value)
}
