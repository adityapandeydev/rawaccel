use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Explicit paths keep persistence tests away from personal settings.
pub(crate) struct SettingsPaths {
    pub primary: PathBuf,
    pub fallback: PathBuf,
    pub mirror: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct SaveReport {
    pub saved_paths: Vec<String>,
    pub warnings: Vec<String>,
}

pub(crate) fn save(settings_json: &str, paths: &SettingsPaths) -> Result<SaveReport, String> {
    save_with_writer(settings_json, paths, |path, content| {
        fs::write(path, content)
    })
}

fn save_with_writer(
    settings_json: &str,
    paths: &SettingsPaths,
    mut write: impl FnMut(&Path, &str) -> io::Result<()>,
) -> Result<SaveReport, String> {
    let mut report = SaveReport {
        saved_paths: Vec::new(),
        warnings: Vec::new(),
    };
    let mut attempt = |path: &Path, create_parent: bool| {
        let result = (|| {
            if create_parent {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
            }
            write(path, settings_json)
        })();
        match result {
            Ok(()) => {
                report.saved_paths.push(path.display().to_string());
                true
            }
            Err(error) => {
                report
                    .warnings
                    .push(format!("{}: {}", path.display(), error));
                false
            }
        }
    };

    // Portable location first; CWD is a fallback, not another mandatory copy.
    let primary_saved = attempt(&paths.primary, false);
    let fallback_attempted = !primary_saved && paths.fallback != paths.primary;
    if fallback_attempted {
        attempt(&paths.fallback, false);
    }
    if let Some(mirror) = &paths.mirror {
        if mirror != &paths.primary && (!fallback_attempted || mirror != &paths.fallback) {
            attempt(mirror, true);
        }
    }

    if report.saved_paths.is_empty() {
        Err(format!(
            "Failed to save settings to any location:\n{}",
            report.warnings.join("\n")
        ))
    } else {
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    pub(super) struct TestDirectory(pub PathBuf);

    impl TestDirectory {
        pub(super) fn new() -> Self {
            loop {
                let path = std::env::temp_dir().join(format!(
                    "rawaccel-persistence-{}-{}",
                    std::process::id(),
                    NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create test directory: {error}"),
                }
            }
        }

        fn paths(&self) -> SettingsPaths {
            SettingsPaths {
                primary: self.0.join("portable.json"),
                fallback: self.0.join("cwd.json"),
                mirror: Some(self.0.join("appdata").join("settings.json")),
            }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            // Only remove the unique directory created by this fixture.
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn existing_stale_backup_does_not_turn_failed_writes_into_success() {
        let dir = TestDirectory::new();
        let paths = dir.paths();
        let mirror = paths.mirror.as_ref().unwrap();
        fs::create_dir_all(mirror.parent().unwrap()).unwrap();
        fs::write(mirror, "old settings").unwrap();

        let error = save_with_writer("new settings", &paths, |_, _| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        })
        .unwrap_err();

        assert!(error.contains("Failed to save settings to any location"));
        assert_eq!(fs::read_to_string(mirror).unwrap(), "old settings");
    }

    #[test]
    fn successful_mirror_is_reported_when_both_portable_locations_fail() {
        let dir = TestDirectory::new();
        let mut paths = dir.paths();
        paths.primary = dir.0.join("missing").join("portable.json");
        paths.fallback = dir.0.join("also-missing").join("cwd.json");
        let report = save("new settings", &paths).unwrap();

        assert_eq!(report.saved_paths.len(), 1);
        assert_eq!(report.warnings.len(), 2);
        assert_eq!(
            fs::read_to_string(paths.mirror.unwrap()).unwrap(),
            "new settings"
        );
    }

    #[test]
    fn portable_success_skips_cwd_and_still_updates_mirror() {
        let dir = TestDirectory::new();
        let paths = dir.paths();
        let report = save("new settings", &paths).unwrap();

        assert_eq!(report.saved_paths.len(), 2);
        assert!(report.warnings.is_empty());
        assert!(!paths.fallback.exists());
        assert_eq!(fs::read_to_string(paths.primary).unwrap(), "new settings");
        assert_eq!(
            fs::read_to_string(paths.mirror.unwrap()).unwrap(),
            "new settings"
        );
    }

    #[test]
    fn mirror_matching_skipped_cwd_is_still_written() {
        let dir = TestDirectory::new();
        let mut paths = dir.paths();
        paths.mirror = Some(paths.fallback.clone());
        let report = save("new settings", &paths).unwrap();

        assert_eq!(report.saved_paths.len(), 2);
        assert_eq!(fs::read_to_string(paths.fallback).unwrap(), "new settings");
    }
}
