//! Restore saved settings at sign-in without creating a window or starting Tauri.

use crate::{
    config::AppConfig,
    driver_status::{ConnectionState, DriverStatus},
    persistence::SettingsPaths,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_PROBE_ATTEMPTS: u32 = 10;

#[derive(Debug, Deserialize, Serialize)]
pub struct RestoreReport {
    pub restored: bool,
    pub timestamp_secs: u64,
    pub settings_path: String,
    pub attempts: u32,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct StartupInfo {
    pub enabled: Option<bool>,
    pub registration_error: Option<String>,
    pub settings_path: String,
    pub mirror_path: Option<String>,
    pub executable_path: String,
    pub portable: bool,
    pub last_result: Option<RestoreReport>,
    pub diagnostic_error: Option<String>,
}

fn registration_for(exe: &Path) -> Result<auto_launch::AutoLaunch, String> {
    if !exe.is_absolute() {
        return Err("Startup executable path must be absolute".into());
    }
    let path = exe
        .to_str()
        .ok_or("Startup executable path cannot be represented as Unicode")?;
    if path.contains(['"', '\r', '\n', '\0']) {
        return Err("Invalid startup executable path".into());
    }
    let quoted = format!("\"{path}\"");
    // Windows Run entries have a documented 260-character command-line limit.
    if format!("{quoted} --silent").encode_utf16().count() > 260 {
        return Err("This installation path is too long for Windows sign-in startup. Move RawAccel to a shorter path.".into());
    }
    Ok(auto_launch::AutoLaunch::new(
        "RawAccel",
        &quoted,
        &["--silent"],
    ))
}

pub fn info() -> Result<StartupInfo, String> {
    let paths = crate::config::settings_paths()?;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let (enabled, registration_error) = match registration_for(&exe)?.is_enabled() {
        Ok(enabled) => (Some(enabled), None),
        Err(auto_launch::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            (Some(false), None)
        }
        Err(error) => (
            None,
            Some(format!("Cannot read Windows startup registration: {error}")),
        ),
    };
    let (last_result, diagnostic_error) = match read_report(&report_path(&paths)) {
        Ok(report) => (report, None),
        Err(error) => (None, Some(error)),
    };
    Ok(StartupInfo {
        enabled,
        registration_error,
        settings_path: paths.primary.display().to_string(),
        mirror_path: paths.mirror.map(|path| path.display().to_string()),
        executable_path: exe.display().to_string(),
        portable: exe.with_file_name("portable.txt").is_file(),
        last_result,
        diagnostic_error,
    })
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let paths = crate::config::settings_paths()?;
        validate_saved_settings(&read_saved_settings(&paths)?)?;
    }
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let registration = registration_for(&exe)?;
    // Explicit enabling also repairs registration after moving a portable copy.
    if enabled {
        registration.enable()
    } else {
        registration.disable()
    }
    .map_err(|error| format!("Cannot update Windows startup: {error}"))
}

pub(crate) fn read_saved_settings(paths: &SettingsPaths) -> Result<String, String> {
    // Recoveries/imports are offered in the GUI. Automatic restoration must not
    // silently select an older backup or another portable copy's AppData mirror.
    fs::read_to_string(&paths.primary).map_err(|error| format!(
        "Cannot read saved settings at {}: {error}. Open RawAccel and use Apply & Save before enabling restoration.",
        paths.primary.display(),
    ))
}

pub(crate) fn validate_saved_settings(raw: &str) -> Result<AppConfig, String> {
    let config = crate::config::parse_or_migrate_settings(raw)?;
    crate::driver_contract::require_supported_modes(&config)?;
    Ok(config)
}

fn restore_with(
    raw: Result<String, String>,
    mut probe: impl FnMut() -> DriverStatus,
    apply: impl FnOnce(&AppConfig) -> Result<(), String>,
    mut wait: impl FnMut(),
) -> RestoreReport {
    let mut report = RestoreReport {
        restored: false,
        timestamp_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        settings_path: String::new(),
        attempts: 0,
        error: None,
    };
    let outcome = (|| {
        let config = validate_saved_settings(&raw?)?;
        loop {
            report.attempts += 1;
            let status = probe();
            if status.state == ConnectionState::Connected {
                let version = status.version.ok_or("Driver version was not returned")?;
                crate::driver_contract::require_supported_version(version)?;
                // apply_config rechecks the version on its own write handle.
                // Never retry a write: a failed request can have an uncertain outcome.
                return apply(&config);
            }
            if matches!(
                status.state,
                ConnectionState::Unavailable | ConnectionState::Busy
            ) && report.attempts < MAX_PROBE_ATTEMPTS
            {
                wait();
                continue;
            }
            return Err(status
                .diagnostic
                .map(|failure| failure.message)
                .unwrap_or_else(|| format!("Driver is {:?}", status.state)));
        }
    })();
    match outcome {
        Ok(()) => report.restored = true,
        Err(error) => report.error = Some(error),
    }
    report
}

pub(crate) fn report_path(paths: &SettingsPaths) -> PathBuf {
    paths.primary.with_file_name("startup-result.json")
}

pub(crate) fn read_report(path: &Path) -> Result<Option<RestoreReport>, String> {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(|error| format!("Cannot read startup result: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read startup result: {error}")),
    }
}

/// Called by the binary before Tauri initialization. This never opens a window.
pub fn run_silent() -> i32 {
    let paths = match crate::config::settings_paths() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("Startup restoration failed: {error}");
            return 1;
        }
    };
    let mut report = restore_with(
        read_saved_settings(&paths),
        crate::driver_status::get,
        crate::driver::apply_config,
        || std::thread::sleep(Duration::from_secs(1)),
    );
    report.settings_path = paths.primary.display().to_string();
    let exit_code = if report.restored { 0 } else { 1 };
    if let Some(error) = &report.error {
        eprintln!("Startup restoration failed: {error}");
    }
    let saved = serde_json::to_string_pretty(&report)
        .map_err(|error| error.to_string())
        .and_then(|raw| {
            crate::persistence::write_state(&report_path(&paths), &raw)
                .map_err(|error| error.to_string())
        });
    if let Err(error) = saved {
        eprintln!("Cannot save startup result: {error}");
        return 2;
    }
    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{AppConfig, AppProfile},
        driver_status,
        models::version_t,
    };
    use std::cell::Cell;

    fn settings() -> String {
        serde_json::to_string(&AppConfig {
            profiles: vec![AppProfile::default()],
            devices: vec![],
            default_device_config: None,
        })
        .unwrap()
    }

    #[test]
    fn missing_or_invalid_settings_fail_before_any_driver_access() {
        for raw in [
            Err("missing settings".into()),
            Ok("broken JSON".into()),
            Ok("{\"profiles\":[]}".into()),
        ] {
            let report = restore_with(
                raw,
                || panic!("Must not probe"),
                |_| panic!("Must not apply"),
                || panic!("Must not wait"),
            );
            assert!(!report.restored);
            assert!(report.error.is_some());
            assert_eq!(report.attempts, 0);
        }
    }

    #[test]
    fn failed_driver_apply_is_reported_and_is_not_retried() {
        let report = restore_with(
            Ok(settings()),
            || driver_status::status_with(|| Ok(crate::driver_contract::EXPECTED_VERSION)),
            |_| Err("write rejected".into()),
            || panic!("Must not retry a write"),
        );
        assert!(!report.restored);
        assert_eq!(report.error.as_deref(), Some("write rejected"));
        assert_eq!(report.attempts, 1);
    }

    #[test]
    fn temporary_unavailability_is_retried_before_one_successful_apply() {
        let probes = Cell::new(0);
        let waits = Cell::new(0);
        let report = restore_with(
            Ok(settings()),
            || {
                probes.set(probes.get() + 1);
                if probes.get() < 3 {
                    driver_status::status_with(|| {
                        Err(crate::driver::DriverFailure {
                            stage: crate::driver::FailureStage::Open,
                            win32_code: Some(2),
                            hresult: None,
                            message: "Driver not ready".into(),
                        })
                    })
                } else {
                    driver_status::status_with(|| Ok(crate::driver_contract::EXPECTED_VERSION))
                }
            },
            |_| Ok(()),
            || waits.set(waits.get() + 1),
        );
        assert!(report.restored);
        assert_eq!(report.attempts, 3);
        assert_eq!(waits.get(), 2);
    }

    #[test]
    fn incompatible_version_never_reaches_apply() {
        let report = restore_with(
            Ok(settings()),
            || {
                driver_status::status_with(|| {
                    Ok(version_t {
                        major: 99,
                        minor: 0,
                        patch: 0,
                    })
                })
            },
            |_| panic!("Must reject incompatible driver"),
            || panic!("Must not retry"),
        );
        assert!(!report.restored);
        assert!(report.error.unwrap().contains("unsupported"));
    }

    #[test]
    fn persistent_unavailability_stops_after_bounded_retries() {
        let waits = Cell::new(0);
        let report = restore_with(
            Ok(settings()),
            || {
                let mut status =
                    driver_status::status_with(|| Ok(crate::driver_contract::EXPECTED_VERSION));
                status.state = ConnectionState::Unavailable;
                status.version = None;
                status
            },
            |_| panic!("Must not apply"),
            || waits.set(waits.get() + 1),
        );
        assert!(!report.restored);
        assert_eq!(report.attempts, 10);
        assert_eq!(waits.get(), 9);
    }

    #[test]
    fn access_denied_does_not_retry_or_apply() {
        let report = restore_with(
            Ok(settings()),
            || {
                driver_status::status_with(|| {
                    Err(crate::driver::DriverFailure {
                        stage: crate::driver::FailureStage::Open,
                        win32_code: Some(5),
                        hresult: None,
                        message: "Access denied".into(),
                    })
                })
            },
            |_| panic!("Must not apply"),
            || panic!("Must not retry"),
        );
        assert_eq!(report.error.as_deref(), Some("Access denied"));
        assert_eq!(report.attempts, 1);
    }

    #[test]
    fn tiered_preview_is_rejected_before_startup_driver_access() {
        if crate::driver_contract::NATIVE_TIERED {
            return;
        }
        let mut value: serde_json::Value = serde_json::from_str(&settings()).unwrap();
        value["profiles"][0]["accel_y"]["mode"] = serde_json::json!("tiered");
        let report = restore_with(
            Ok(value.to_string()),
            || panic!("Must not probe"),
            |_| panic!("Must not apply"),
            || panic!("Must not wait"),
        );
        assert!(report.error.unwrap().contains("preview only"));
        assert_eq!(report.attempts, 0);
    }

    #[test]
    fn registration_quotes_custom_install_paths_and_uses_silent_mode() {
        let exe = Path::new(r"C:\Users\Example User\My Apps\RawAccel.exe");
        let registration = registration_for(exe).unwrap();
        assert_eq!(
            registration.get_app_path(),
            r#""C:\Users\Example User\My Apps\RawAccel.exe""#
        );
        assert_eq!(registration.get_args(), ["--silent"]);
        assert!(registration_for(Path::new("relative.exe")).is_err());
        let long = format!(r"C:\{}\RawAccel.exe", "x".repeat(250));
        assert!(registration_for(Path::new(&long))
            .unwrap_err()
            .contains("too long"));
    }
}
