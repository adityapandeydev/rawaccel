use std::mem::size_of;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

static DRIVER_IO: std::sync::Mutex<()> = std::sync::Mutex::new(());

const DEVICE_NAME: &str = r"\\.\rawaccel";

// IOCTLs defined in rawaccel-io-def.h
#[allow(dead_code)] // Mirrored/read-tested for ABI verification; no read command yet.
pub(crate) const IOCTL_READ: u32 = 0x88882220;
pub(crate) const IOCTL_WRITE: u32 = 0x88882224;
pub(crate) const IOCTL_GET_VERSION: u32 = 0x88882228;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStage {
    Open,
    VersionQuery,
    VersionResponse,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DriverFailure {
    pub stage: FailureStage,
    pub win32_code: Option<u32>,
    pub hresult: Option<String>,
    pub message: String,
}
impl DriverFailure {
    pub(crate) fn windows(stage: FailureStage, action: &str, error: windows::core::Error) -> Self {
        let code = error.code().0 as u32;
        Self {
            stage,
            win32_code: ((code & 0xffff0000) == 0x80070000).then_some(code & 0xffff),
            hresult: Some(format!("0x{code:08X}")),
            message: ioctl_error(action, error),
        }
    }
    fn malformed(message: String) -> Self {
        Self {
            stage: FailureStage::VersionResponse,
            win32_code: None,
            hresult: None,
            message,
        }
    }
}

pub fn open_driver() -> Result<OwnedHandle, String> {
    open_checked().map_err(|error| error.message)
}

fn open_checked() -> Result<OwnedHandle, DriverFailure> {
    let mut wide_name: Vec<u16> = DEVICE_NAME.encode_utf16().collect();
    wide_name.push(0);

    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_name.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
    .map_err(|error| DriverFailure::windows(FailureStage::Open, "open driver", error))?;

    if handle.is_invalid() {
        Err(DriverFailure {
            stage: FailureStage::Open,
            win32_code: None,
            hresult: None,
            message: "Windows returned an invalid RawAccel device handle".into(),
        })
    } else {
        // CreateFileW transferred ownership of this valid handle to us.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
    }
}

pub fn get_version() -> Result<crate::models::version_t, String> {
    probe().map_err(|error| error.message)
}

pub(crate) fn probe() -> Result<crate::models::version_t, DriverFailure> {
    let _guard = DRIVER_IO.lock().map_err(|_| DriverFailure {
        stage: FailureStage::Open,
        win32_code: None,
        hresult: None,
        message: "The driver connection lock is unavailable".into(),
    })?;
    let handle = open_checked()?;
    query_version(&handle)
}

fn query_version(handle: &OwnedHandle) -> Result<crate::models::version_t, DriverFailure> {
    let mut version = crate::models::version_t::default();
    let mut bytes_returned = 0u32;

    let success = unsafe {
        DeviceIoControl(
            HANDLE(handle.as_raw_handle()),
            IOCTL_GET_VERSION,
            None,
            0,
            Some(&mut version as *mut _ as *mut _),
            size_of::<crate::models::version_t>() as u32,
            Some(&mut bytes_returned),
            None,
        )
    };

    success.map_err(|error| {
        DriverFailure::windows(FailureStage::VersionQuery, "get driver version", error)
    })?;
    checked_version(version, bytes_returned).map_err(DriverFailure::malformed)
}

pub fn apply_config(config: &crate::config::AppConfig) -> Result<(), String> {
    // Includes preview-only mode validation, before even opening the driver.
    let buffer = crate::packet::encode_for_driver(config)?;
    let _guard = DRIVER_IO
        .lock()
        .map_err(|_| "The driver connection lock is unavailable")?;
    let handle = open_checked().map_err(|error| error.message)?;
    apply_packet_with(
        &buffer,
        || query_version(&handle).map_err(|error| error.message),
        |buffer| {
            let mut returned = 0;
            unsafe {
                DeviceIoControl(
                    HANDLE(handle.as_raw_handle()),
                    IOCTL_WRITE,
                    Some(buffer.as_ptr().cast()),
                    buffer.len() as u32,
                    None,
                    0,
                    Some(&mut returned),
                    None,
                )
            }
            .map_err(|error| ioctl_error("write settings to driver", error))
        },
    )
}

fn apply_packet_with(
    buffer: &[u8],
    probe: impl FnOnce() -> Result<crate::models::version_t, String>,
    write: impl FnOnce(&[u8]) -> Result<(), String>,
) -> Result<(), String> {
    // Probe and write use the SAME owned handle, serialized with our status queries.
    crate::driver_contract::require_supported_version(probe()?)?;
    write(buffer)
}

fn checked_version(
    version: crate::models::version_t,
    returned: u32,
) -> Result<crate::models::version_t, String> {
    let expected = size_of::<crate::models::version_t>() as u32;
    if returned != expected {
        return Err(format!(
            "Malformed driver version response: expected {expected} bytes, received {returned}"
        ));
    }
    if version.major < 0 || version.minor < 0 || version.patch < 0 {
        return Err("Malformed driver version response: negative version component".into());
    }
    Ok(version)
}

fn ioctl_error(action: &str, error: windows::core::Error) -> String {
    format!("Failed to {action}: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_versions_and_failed_probes_never_write() {
        let expected = crate::driver_contract::EXPECTED_VERSION;
        assert!(apply_packet_with(
            &[1, 2],
            || Err("query failed".into()),
            |_| panic!("Must not write")
        )
        .is_err());
        assert!(apply_packet_with(
            &[1, 2],
            || Ok(crate::models::version_t {
                major: 99,
                ..expected
            }),
            |_| panic!("Must not write")
        )
        .unwrap_err()
        .contains("unsupported"));
        apply_packet_with(
            &[1, 2],
            || Ok(expected),
            |bytes| {
                assert_eq!(bytes, &[1, 2]);
                Ok(())
            },
        )
        .unwrap();
    }
    #[test]
    fn stock_build_rejects_tiered_before_opening_a_driver_handle() {
        let mut c = crate::config::AppConfig {
            profiles: vec![crate::config::AppProfile::default()],
            devices: vec![],
            default_device_config: None,
        };
        for axis in ["X", "Y"] {
            c.profiles[0].accel_x.mode = "noaccel".into();
            c.profiles[0].accel_y.mode = "noaccel".into();
            if axis == "X" {
                c.profiles[0].accel_x.mode = "tiered".into();
            } else {
                c.profiles[0].accel_y.mode = "tiered".into();
            }
            assert!(crate::packet::encode_stock(&c)
                .unwrap_err()
                .contains("preview only"));
            if !crate::driver_contract::NATIVE_TIERED {
                assert!(apply_config(&c).unwrap_err().contains("preview only"));
            }
            // Offline native Tiered encoding remains available in both builds.
            crate::packet::encode(&c).unwrap();
        }
    }
    #[test]
    fn version_response_requires_the_complete_c_layout_record() {
        use crate::models::version_t;
        assert_eq!(size_of::<version_t>(), 12);
        assert_eq!(std::mem::align_of::<version_t>(), 4);
        let v = version_t {
            major: 1,
            minor: 8,
            patch: 0,
        };
        assert_eq!(checked_version(v, 12).unwrap(), v);
        for size in (0..12).chain([13, u32::MAX]) {
            assert!(checked_version(v, size).is_err(), "{size}");
        }
        assert!(checked_version(version_t { major: -1, ..v }, 12).is_err());
    }
    #[test]
    fn ioctl_error_retains_the_original_windows_error() {
        let captured = windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(5));
        unsafe {
            windows::Win32::Foundation::SetLastError(windows::Win32::Foundation::WIN32_ERROR(6));
        }
        let message = ioctl_error("write settings to driver", captured);
        assert!(message.contains("0x80070005"), "{message}");
        assert!(!message.contains("0x80070006"));
    }
    #[test]
    fn invalid_direct_apply_returns_before_opening_driver() {
        let c = crate::config::AppConfig {
            profiles: vec![],
            devices: vec![],
            default_device_config: None,
        };
        assert_eq!(
            apply_config(&c).unwrap_err(),
            "At least one profile is required"
        );
    }
}
