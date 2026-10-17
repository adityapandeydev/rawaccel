use crate::config::AppConfig;
use crate::persistence::SaveReport;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApplyReport {
    pub applied: bool,
    pub saved: bool,
    pub saved_paths: Vec<String>,
    pub warnings: Vec<String>,
    pub save_error: Option<String>,
}

/// Validation and driver failures reject the command. Once the driver accepts
/// settings, persistence failures must not be reported as an Apply failure.
pub(crate) fn apply_with(
    settings_json: &str,
    apply: impl FnOnce(&AppConfig) -> Result<(), String>,
    save: impl FnOnce(&str) -> Result<SaveReport, String>,
) -> Result<ApplyReport, String> {
    let document = crate::config::normalize_settings_json(settings_json)?;
    let config: AppConfig = serde_json::from_value(document.clone())
        .map_err(|error| format!("Failed to parse settings JSON: {error}"))?;
    let settings_json = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("Failed to serialize settings: {error}"))?;
    apply(&config)?;

    Ok(match save(&settings_json) {
        Ok(report) => ApplyReport {
            applied: true,
            saved: true,
            saved_paths: report.saved_paths,
            warnings: report.warnings,
            save_error: None,
        },
        Err(error) => ApplyReport {
            applied: true,
            saved: false,
            saved_paths: Vec::new(),
            warnings: Vec::new(),
            save_error: Some(error),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn settings() -> String {
        serde_json::to_string(&AppConfig {
            profiles: vec![crate::config::AppProfile {
                name: "Portable".to_string(),
                ..Default::default()
            }],
            devices: vec![],
            default_device_config: None,
        })
        .unwrap()
    }

    #[test]
    fn unsupported_schema_never_reaches_driver_or_disk() {
        let raw = format!(
            "{{\"schema_version\":{},\"profiles\":[]}}",
            crate::config::SETTINGS_SCHEMA_VERSION + 1
        );
        let result = apply_with(
            &raw,
            |_| panic!("Must reject before driver access"),
            |_| panic!("Must not save a future schema"),
        );
        assert!(result
            .unwrap_err()
            .contains("Unsupported settings schema version"));
    }

    #[test]
    fn explicit_apply_saves_current_schema_and_retains_document_extensions() {
        let mut value: serde_json::Value = serde_json::from_str(&settings()).unwrap();
        value["editor_metadata"] = serde_json::json!({"note":"preserve"});
        apply_with(
            &value.to_string(),
            |_| Ok(()),
            |raw| {
                let saved: serde_json::Value = serde_json::from_str(raw).unwrap();
                assert_eq!(
                    saved["schema_version"],
                    crate::config::SETTINGS_SCHEMA_VERSION
                );
                assert_eq!(saved["editor_metadata"]["note"], "preserve");
                Ok(SaveReport {
                    saved_paths: vec!["fixture.json".to_string()],
                    warnings: vec![],
                })
            },
        )
        .unwrap();
    }

    #[test]
    fn failed_driver_apply_does_not_persist_settings() {
        let result = apply_with(
            &settings(),
            |_| Err("Driver unavailable".to_string()),
            |_| panic!("Must not persist an unsuccessful Apply"),
        );
        assert_eq!(result.unwrap_err(), "Driver unavailable");
    }

    #[test]
    fn invalid_settings_do_not_reach_driver_or_disk() {
        let result = apply_with(
            "{\"profiles\":[]}",
            |_| panic!("Must validate before driver access"),
            |_| panic!("Must validate before persistence"),
        );
        assert_eq!(result.unwrap_err(), "At least one profile is required");
    }

    #[test]
    fn disk_failure_after_apply_returns_partial_success() {
        let applied = Cell::new(false);
        let report = apply_with(
            &settings(),
            |_| {
                applied.set(true);
                Ok(())
            },
            |_| {
                assert!(applied.get());
                Err("All settings locations are read-only".to_string())
            },
        )
        .unwrap();
        assert!(report.applied);
        assert!(!report.saved);
        assert!(report.saved_paths.is_empty());
        assert_eq!(
            report.save_error.as_deref(),
            Some("All settings locations are read-only")
        );
    }

    #[test]
    fn successful_save_preserves_partial_destination_warnings() {
        let report = apply_with(
            &settings(),
            |_| Ok(()),
            |_| {
                Ok(SaveReport {
                    saved_paths: vec!["AppData/settings.json".to_string()],
                    warnings: vec!["Portable location was read-only".to_string()],
                })
            },
        )
        .unwrap();
        assert!(report.applied && report.saved);
        assert!(report.save_error.is_none());
        assert_eq!(report.saved_paths, ["AppData/settings.json"]);
        assert_eq!(report.warnings, ["Portable location was read-only"]);
    }
}
