//! Ship the stock signed-driver protocol by default. Tiered remains fully editable offline.
use crate::{config::AppConfig, models::version_t};
// Opt in at build time only after a matching signed native-Tiered driver is available.
pub const NATIVE_TIERED: bool = cfg!(feature = "custom-tiered-driver");
pub const CUSTOM_SOURCE_VERSION: version_t = version_t {
    major: 1,
    minor: 8,
    patch: 0,
};
pub const STOCK_VERSION: version_t = version_t {
    major: 1,
    minor: 7,
    patch: 0,
};
pub const EXPECTED_VERSION: version_t = if NATIVE_TIERED {
    CUSTOM_SOURCE_VERSION
} else {
    STOCK_VERSION
};
pub const CONTRACT: &str = if NATIVE_TIERED {
    "rawaccel-tiered-v1"
} else {
    "rawaccel-stock-1.7"
};

pub(crate) fn require_supported_modes(config: &AppConfig) -> Result<(), String> {
    if !NATIVE_TIERED {
        require_stock_modes(config)?;
    }
    Ok(())
}
pub(crate) fn require_stock_modes(config: &AppConfig) -> Result<(), String> {
    for profile in &config.profiles {
        for (axis, args) in [("X", &profile.accel_x), ("Y", &profile.accel_y)] {
            if args.mode == "tiered" {
                return Err(format!("Profile '{}' {axis}: Tiered is preview only with the stock driver. Its graphs, editing, import and export remain available; switch to a supported mode before applying settings.", profile.name));
            }
        }
    }
    Ok(())
}
pub(crate) fn require_supported_version(actual: version_t) -> Result<(), String> {
    if actual != EXPECTED_VERSION {
        let expected = EXPECTED_VERSION;
        return Err(format!("Settings were not sent: driver {}.{}.{} is unsupported by this build ({CONTRACT}, expected {}.{}.{}).", actual.major, actual.minor, actual.patch, expected.major, expected.minor, expected.patch));
    }
    Ok(())
}
