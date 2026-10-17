//! Shared application defaults. Existing explicit values, including zero/false,
//! are never overwritten. This template is also imported by TypeScript.
use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

pub(crate) fn document() -> &'static Value {
    static DEFAULTS: OnceLock<Value> = OnceLock::new();
    DEFAULTS.get_or_init(|| {
        serde_json::from_str(include_str!("../../src/settings-defaults.json"))
            .expect("Checked-in settings defaults must be valid JSON")
    })
}

pub(crate) fn profile() -> Value {
    let mut profile = document()["profile"].clone();
    profile["id"] = Value::String(String::new());
    profile["accel_x"] = document()["accel_args"].clone();
    profile["accel_y"] = document()["accel_args"].clone();
    profile["speed_processor_args"] = document()["speed_args"].clone();
    profile
}

pub(crate) fn fill_missing(value: &mut Value, defaults: &Value) {
    if let (Some(object), Some(defaults)) = (value.as_object_mut(), defaults.as_object()) {
        for (key, default) in defaults {
            if let Some(value) = object.get_mut(key) {
                fill_missing(value, default);
            } else {
                object.insert(key.clone(), default.clone());
            }
        }
    }
}

pub(crate) fn complete_profiles(value: &mut Value) -> Result<(), String> {
    let profiles = value
        .get_mut("profiles")
        .and_then(Value::as_array_mut)
        .ok_or("Settings must contain a profiles array")?;
    let mut ids: HashSet<String> = profiles
        .iter()
        .filter_map(|profile| profile.get("id").and_then(Value::as_str))
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect();
    let defaults = profile();
    for (index, profile) in profiles.iter_mut().enumerate() {
        if profile.get("name").and_then(Value::as_str).is_none() {
            return Err(format!("Profile {} must have a string name", index + 1));
        }
        fill_missing(profile, &defaults);
        if profile.get("id").and_then(Value::as_str) == Some("") {
            let mut suffix = index + 1;
            let id = loop {
                let candidate = format!("profile-{suffix}");
                if ids.insert(candidate.clone()) {
                    break candidate;
                }
                suffix += 1;
            };
            profile["id"] = Value::String(id);
        }
    }
    Ok(())
}
