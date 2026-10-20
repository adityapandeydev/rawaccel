//! Exercise the actual silent binary only with invalid/missing configurations.
//! These cases must terminate before ANY driver access or registry changes.
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "RawAccel silent test {} {}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("portable.txt"), "").unwrap();
        fs::copy(
            env!("CARGO_BIN_EXE_rawaccel-userspace"),
            path.join("RawAccel.exe"),
        )
        .unwrap();
        Self(path)
    }

    fn run(&self) -> serde_json::Value {
        let working_dir = self.0.join("unrelated working directory");
        fs::create_dir(&working_dir).unwrap();
        fs::write(
            working_dir.join("settings.json"),
            r#"{"schema_version":999,"profiles":[]}"#,
        )
        .unwrap();
        let output = Command::new(self.0.join("RawAccel.exe"))
            .arg("--silent")
            .current_dir(working_dir)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let raw = fs::read_to_string(self.0.join("startup-result.json")).unwrap();
        let report: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(report["restored"], false);
        assert_eq!(report["attempts"], 0);
        assert_eq!(
            report["settings_path"],
            self.0.join("settings.json").display().to_string()
        );
        report
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn silent_binary_reports_invalid_settings_without_opening_the_ui() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("settings.json"), "broken JSON").unwrap();
    let report = fixture.run();
    assert!(report["error"].as_str().unwrap().contains("Invalid JSON"));
    assert_eq!(
        fs::read_to_string(fixture.0.join("settings.json")).unwrap(),
        "broken JSON"
    );
}

#[test]
fn silent_binary_does_not_use_cwd_or_an_older_backup_when_primary_is_missing() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("settings.json.bak"), "broken backup").unwrap();
    let report = fixture.run();
    assert!(report["error"]
        .as_str()
        .unwrap()
        .contains("Cannot read saved settings"));
    assert!(!fixture.0.join("settings.json").exists());
}
