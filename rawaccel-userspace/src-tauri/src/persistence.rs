use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

#[derive(Debug, Serialize)]
pub struct LoadReport {
    pub settings_json: String,
    pub source: String,
    pub recovered: bool,
    pub warnings: Vec<String>,
}

pub(crate) fn load(
    paths: &SettingsPaths,
    parse: impl Fn(&str) -> Result<String, String>,
) -> Result<LoadReport, String> {
    let mut locations = vec![paths.primary.clone()];
    for path in std::iter::once(&paths.fallback).chain(paths.mirror.iter()) {
        if !locations.contains(path) {
            locations.push(path.clone());
        }
    }
    // Prefer current copies to older recovery files, retaining portable priority.
    let candidates: Vec<_> = locations
        .iter()
        .map(|path| (path.clone(), false))
        .chain(locations.iter().map(|path| (backup_path(path), true)))
        .collect();
    let mut warnings = Vec::new();
    for (path, is_backup) in candidates {
        let result = match fs::read_to_string(&path) {
            Ok(raw) => parse(&raw),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => Err(error.to_string()),
        };
        match result {
            Ok(settings_json) => {
                return Ok(LoadReport {
                    settings_json,
                    source: path.display().to_string(),
                    recovered: is_backup || !warnings.is_empty(),
                    warnings,
                });
            }
            Err(error) => warnings.push(format!("{}: {}", path.display(), error)),
        }
    }
    if warnings.is_empty() {
        Err("No settings.json found".to_string())
    } else {
        Err(format!(
            "No valid settings could be loaded:\n{}",
            warnings.join("\n")
        ))
    }
}

pub(crate) fn save(settings_json: &str, paths: &SettingsPaths) -> Result<SaveReport, String> {
    save_with_writer(settings_json, paths, atomic_write)
}

pub(crate) fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".bak");
    PathBuf::from(name)
}

fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    atomic_write_with_replace(path, content, |source, destination| {
        fs::rename(source, destination)
    })
}

fn atomic_write_with_replace(
    path: &Path,
    content: &str,
    replace: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let staged = StagedFile::new(path, content.as_bytes())?;
    match fs::read(path) {
        Ok(previous) => {
            let backup = backup_path(path);
            let previous_is_valid = std::str::from_utf8(&previous)
                .is_ok_and(|raw| crate::config::parse_settings_for_load(raw).is_ok());
            // Saving after recovery must not overwrite the last good backup
            // with the corrupt primary, especially if replacement then fails.
            if !backup.is_file() || previous_is_valid {
                let staged_backup = StagedFile::new(&backup, &previous)?;
                fs::rename(&staged_backup.path, &backup)?;
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    // Same-directory staging keeps rename on one volume. A failed replacement
    // leaves the original and the recovery copy available.
    replace(&staged.path, path)
}

static NEXT_STAGED_FILE: AtomicU64 = AtomicU64::new(0);

struct StagedFile {
    path: PathBuf,
    file: Option<File>,
}

impl StagedFile {
    fn new(destination: &Path, content: &[u8]) -> io::Result<Self> {
        let filename = destination
            .file_name()
            .ok_or_else(|| io::Error::other("Settings destination must be a file"))?;
        loop {
            let mut name = std::ffi::OsString::from(".");
            name.push(filename);
            name.push(format!(
                ".{}-{}.tmp",
                std::process::id(),
                NEXT_STAGED_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            let path = destination.with_file_name(name);
            let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            };
            let mut staged = Self {
                path,
                file: Some(file),
            };
            let file = staged.file.as_mut().unwrap();
            file.write_all(content)?;
            file.sync_all()?;
            staged.file.take();
            return Ok(staged);
        }
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        // Close first so Windows can delete the file after a write/sync error.
        self.file.take();
        let _ = fs::remove_file(&self.path);
    }
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

    #[test]
    fn replacement_preserves_previous_contents_and_leaves_no_temp_files() {
        let dir = TestDirectory::new();
        let path = dir.0.join("settings.json");
        let original = valid_settings("original");
        let replacement = valid_settings("replacement");
        let latest = valid_settings("latest");
        fs::write(&path, &original).unwrap();
        atomic_write(&path, &replacement).unwrap();
        atomic_write(&path, &latest).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), latest);
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), replacement);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }

    #[test]
    fn failed_replacement_preserves_original_and_recovery_copy() {
        let dir = TestDirectory::new();
        let path = dir.0.join("settings.json");
        fs::write(&path, "original").unwrap();
        atomic_write_with_replace(&path, "replacement", |_, _| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"))
        })
        .unwrap_err();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), "original");
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }

    #[test]
    fn failed_backup_preserves_original_and_cleans_staged_file() {
        let dir = TestDirectory::new();
        let path = dir.0.join("settings.json");
        fs::write(&path, "original").unwrap();
        fs::create_dir(backup_path(&path)).unwrap();
        atomic_write(&path, "replacement").unwrap_err();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }

    fn valid_settings(name: &str) -> String {
        serde_json::to_string(&crate::config::AppConfig {
            profiles: vec![crate::config::AppProfile {
                name: name.to_string(),
                ..Default::default()
            }],
            devices: vec![],
            default_device_config: None,
        })
        .unwrap()
    }

    #[test]
    fn failed_save_after_recovery_keeps_the_last_valid_backup() {
        let dir = TestDirectory::new();
        let path = dir.0.join("settings.json");
        let good = valid_settings("recoverable");
        fs::write(&path, "broken JSON").unwrap();
        fs::write(backup_path(&path), &good).unwrap();
        atomic_write_with_replace(&path, &good, |_, _| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"))
        })
        .unwrap_err();
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken JSON");
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), good);
    }

    fn parse_test_json(raw: &str) -> Result<String, String> {
        serde_json::from_str::<serde_json::Value>(raw)
            .map(|_| raw.to_string())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn corrupt_primary_recovers_current_mirror_without_rewriting_files() {
        let dir = TestDirectory::new();
        let paths = dir.paths();
        fs::write(&paths.primary, "broken JSON").unwrap();
        let mirror = paths.mirror.as_ref().unwrap();
        fs::create_dir_all(mirror.parent().unwrap()).unwrap();
        fs::write(mirror, "{\"current\":true}").unwrap();
        fs::write(backup_path(&paths.primary), "{\"older\":true}").unwrap();

        let report = load(&paths, parse_test_json).unwrap();

        assert_eq!(report.settings_json, "{\"current\":true}");
        assert_eq!(report.source, mirror.display().to_string());
        assert!(report.recovered);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(fs::read_to_string(&paths.primary).unwrap(), "broken JSON");
        assert_eq!(fs::read_to_string(mirror).unwrap(), "{\"current\":true}");
    }

    #[test]
    fn invalid_current_copies_recover_previous_backup() {
        let dir = TestDirectory::new();
        let paths = dir.paths();
        fs::write(&paths.primary, "broken JSON").unwrap();
        fs::write(backup_path(&paths.primary), "{\"older\":true}").unwrap();

        let report = load(&paths, parse_test_json).unwrap();

        assert_eq!(report.settings_json, "{\"older\":true}");
        assert!(report.recovered);
        assert!(report.source.ends_with(".bak"));
    }

    #[test]
    fn valid_portable_file_wins_and_keeps_its_formatting() {
        let dir = TestDirectory::new();
        let paths = dir.paths();
        let raw = "{ \"portable\" : true }\n";
        fs::write(&paths.primary, raw).unwrap();
        fs::write(&paths.fallback, "{\"cwd\":true}").unwrap();

        let report = load(&paths, parse_test_json).unwrap();

        assert_eq!(report.settings_json, raw);
        assert!(!report.recovered);
        assert!(report.warnings.is_empty());
        assert_eq!(fs::read_to_string(&paths.primary).unwrap(), raw);
        assert!(!backup_path(&paths.primary).exists());
    }

    #[test]
    fn all_invalid_files_return_diagnostics_instead_of_invalid_json() {
        let dir = TestDirectory::new();
        let paths = dir.paths();
        fs::write(&paths.primary, "broken JSON").unwrap();
        let error = load(&paths, parse_test_json).unwrap_err();
        assert!(error.contains("No valid settings"));
        assert!(error.contains("portable.json"));
    }
}
