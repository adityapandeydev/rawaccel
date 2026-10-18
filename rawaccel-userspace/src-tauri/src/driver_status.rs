//! Connection diagnostics and the supported custom-driver contract are distinct.
use crate::{
    driver::{DriverFailure, FailureStage},
    models::version_t,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connected,
    Unavailable,
    AccessDenied,
    Busy,
    OpenFailed,
    QueryFailed,
    MalformedResponse,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    Supported,
    Unverified,
    Unsupported,
    Unknown,
}
#[derive(Clone, Debug, Serialize)]
pub struct DriverStatus {
    pub state: ConnectionState,
    pub version: Option<version_t>,
    pub supported_version: version_t,
    pub driver_contract: &'static str,
    pub tiered_apply_enabled: bool,
    pub compatibility: Compatibility,
    pub diagnostic: Option<DriverFailure>,
}

pub fn get() -> DriverStatus {
    status_with(crate::driver::probe)
}

pub(crate) fn status_with(
    probe: impl FnOnce() -> Result<version_t, DriverFailure>,
) -> DriverStatus {
    let mut status = DriverStatus {
        state: ConnectionState::Connected,
        version: None,
        supported_version: crate::driver_contract::EXPECTED_VERSION,
        driver_contract: crate::driver_contract::CONTRACT,
        tiered_apply_enabled: crate::driver_contract::NATIVE_TIERED,
        compatibility: Compatibility::Unknown,
        diagnostic: None,
    };
    match probe() {
        Ok(version) => {
            status.version = Some(version);
            // Version selects the tested packet protocol, not a cryptographic identity of the installed binary.
            status.compatibility = if version == crate::driver_contract::EXPECTED_VERSION {
                if crate::driver_contract::NATIVE_TIERED {
                    Compatibility::Unverified
                } else {
                    Compatibility::Supported
                }
            } else {
                Compatibility::Unsupported
            };
        }
        Err(error) => {
            status.state = classify(&error);
            status.diagnostic = Some(error);
        }
    }
    status
}
fn classify(error: &DriverFailure) -> ConnectionState {
    if error.stage == FailureStage::VersionResponse {
        return ConnectionState::MalformedResponse;
    }
    match error.win32_code {
        Some(5 | 1314) => ConnectionState::AccessDenied,
        Some(32 | 33) => ConnectionState::Busy,
        Some(2 | 3 | 55 | 433 | 1167) if error.stage == FailureStage::Open => {
            ConnectionState::Unavailable
        }
        _ if error.stage == FailureStage::Open => ConnectionState::OpenFailed,
        _ => ConnectionState::QueryFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_contract_matches_the_actual_custom_cpp_header() {
        let header = include_str!("../../../common/rawaccel-version.h");
        let part = |name: &str| {
            header
                .lines()
                .find_map(|line| {
                    let fields: Vec<_> = line.split_whitespace().collect();
                    (fields.len() == 3 && fields[0] == "#define" && fields[1] == name)
                        .then(|| fields[2].parse::<i32>().unwrap())
                })
                .unwrap()
        };
        assert_eq!(
            crate::driver_contract::CUSTOM_SOURCE_VERSION,
            version_t {
                major: part("RA_VER_MAJOR"),
                minor: part("RA_VER_MINOR"),
                patch: part("RA_VER_PATCH")
            }
        );
    }
    #[test]
    fn connection_and_custom_abi_compatibility_are_distinct() {
        let status = status_with(|| Ok(crate::driver_contract::EXPECTED_VERSION));
        assert_eq!(status.state, ConnectionState::Connected);
        assert_eq!(
            status.version,
            Some(crate::driver_contract::EXPECTED_VERSION)
        );
        assert_eq!(
            status.compatibility,
            if crate::driver_contract::NATIVE_TIERED {
                Compatibility::Unverified
            } else {
                Compatibility::Supported
            }
        );
        for major in [0, 2] {
            let actual = version_t {
                major,
                minor: 0,
                patch: 0,
            };
            let status = status_with(|| Ok(actual));
            assert_eq!(status.version, Some(actual));
            assert_eq!(status.state, ConnectionState::Connected);
            assert_eq!(status.compatibility, Compatibility::Unsupported);
        }
    }
    #[test]
    fn windows_errors_keep_stage_codes_and_useful_connection_states() {
        for (stage, code, expected) in [
            (FailureStage::Open, 2, ConnectionState::Unavailable),
            (FailureStage::Open, 1167, ConnectionState::Unavailable),
            (FailureStage::Open, 5, ConnectionState::AccessDenied),
            (FailureStage::VersionQuery, 5, ConnectionState::AccessDenied),
            (FailureStage::Open, 32, ConnectionState::Busy),
            (FailureStage::Open, 87, ConnectionState::OpenFailed),
            (FailureStage::VersionQuery, 2, ConnectionState::QueryFailed),
            (FailureStage::VersionQuery, 50, ConnectionState::QueryFailed),
        ] {
            let error = DriverFailure::windows(
                stage,
                "query",
                windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(code)),
            );
            let status = status_with(|| Err(error));
            assert_eq!(status.state, expected);
            assert_eq!(status.diagnostic.as_ref().unwrap().win32_code, Some(code));
            assert_eq!(status.diagnostic.as_ref().unwrap().stage, stage);
            assert!(status.version.is_none());
            assert_eq!(status.compatibility, Compatibility::Unknown);
        }
    }
    #[test]
    fn malformed_and_non_win32_failures_are_not_reported_as_uninstalled() {
        let bad = DriverFailure {
            stage: FailureStage::VersionResponse,
            win32_code: None,
            hresult: None,
            message: "Expected 12 bytes, received 4".into(),
        };
        let status = status_with(|| Err(bad));
        assert_eq!(status.state, ConnectionState::MalformedResponse);
        assert!(status.diagnostic.unwrap().message.contains("received 4"));
        let error = DriverFailure::windows(
            FailureStage::VersionQuery,
            "query",
            windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005u32 as i32)),
        );
        assert_eq!(error.win32_code, None);
        assert_eq!(
            status_with(|| Err(error)).state,
            ConnectionState::QueryFailed
        );
    }
}
