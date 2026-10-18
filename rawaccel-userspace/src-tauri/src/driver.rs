use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

const DEVICE_NAME: &str = r"\\.\rawaccel";

// IOCTLs defined in rawaccel-io-def.h
const _IOCTL_READ: u32 = 0x88882220;
const IOCTL_WRITE: u32 = 0x88882224;
const IOCTL_GET_VERSION: u32 = 0x88882228;

pub fn open_driver() -> Result<HANDLE, String> {
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
    .map_err(|e| format!("Failed to open driver: {}", e))?;

    if handle.is_invalid() {
        Err("Failed to open driver. Is RawAccel installed and running?".to_string())
    } else {
        Ok(handle)
    }
}

pub fn get_version() -> Result<(i32, i32, i32), String> {
    let handle = open_driver()?;
    let mut version = (0i32, 0i32, 0i32);
    let mut bytes_returned = 0u32;

    let success = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_GET_VERSION,
            None,
            0,
            Some(&mut version as *mut _ as *mut _),
            size_of::<(i32, i32, i32)>() as u32,
            Some(&mut bytes_returned),
            None,
        )
    };

    unsafe {
        CloseHandle(handle).ok();
    }

    if success.is_ok() && bytes_returned > 0 {
        Ok(version)
    } else {
        Err("Failed to get driver version".to_string())
    }
}

pub fn apply_config(config: &crate::config::AppConfig) -> Result<(), String> {
    let buffer = crate::packet::encode(config)?;
    let total_size = buffer.len();
    unsafe {
        // Validate and pack everything before opening the kernel device.
        let handle = open_driver()?;
        let mut bytes_returned = 0u32;
        let success = DeviceIoControl(
            handle,
            IOCTL_WRITE,
            Some(buffer.as_ptr() as *const _),
            total_size as u32,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        );

        CloseHandle(handle).ok();

        if success.is_ok() {
            println!("driver.rs: Successfully applied config.");
            Ok(())
        } else {
            let err = windows::Win32::Foundation::GetLastError();
            println!("driver.rs: DeviceIoControl failed with error {:?}", err);
            Err(format!(
                "Failed to write to driver via DeviceIoControl. Error code: {:?}",
                err
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_direct_apply_returns_before_opening_driver() {
        let c = crate::config::AppConfig { profiles: vec![], devices: vec![], default_device_config: None };
        assert_eq!(apply_config(&c).unwrap_err(), "At least one profile is required");
    }
}
