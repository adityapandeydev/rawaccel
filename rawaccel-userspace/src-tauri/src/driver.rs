use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

use crate::models as ra;

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
    let handle = open_driver()?;

    // Send ALL configured devices to the driver, not just ones with a profile.
    // A device with disable=true but no profile_id still needs to be sent so
    // the driver knows to skip acceleration for that mouse.
    let all_devices: Vec<_> = config.devices.iter().collect();
    let num_profiles = config.profiles.len();
    let num_devices = all_devices.len();

    let io_base_size = size_of::<ra::io_base>();
    let modifier_data_size = size_of::<ra::modifier_settings>() * num_profiles;
    let device_data_size = size_of::<ra::device_settings>() * num_devices;
    let total_size = io_base_size + modifier_data_size + device_data_size;

    let mut buffer = vec![0u8; total_size];

    unsafe {
        let base_ptr = buffer.as_mut_ptr() as *mut ra::io_base;
        std::ptr::write(base_ptr, ra::io_base::default());
        (*base_ptr).modifier_data_size = num_profiles as u32;
        (*base_ptr).device_data_size = num_devices as u32;
        if let Some(defaults) = &config.default_device_config {
            (*base_ptr).default_dev_cfg = defaults.to_native();
        }

        let mod_ptr = buffer.as_mut_ptr().add(io_base_size) as *mut ra::modifier_settings;
        for i in 0..num_profiles {
            let settings = &mut *mod_ptr.add(i);
            settings.prof = config.profiles[i].to_native();
            crate::math::init_data(settings);
        }

        let dev_ptr =
            buffer.as_mut_ptr().add(io_base_size + modifier_data_size) as *mut ra::device_settings;
        for i in 0..num_devices {
            let app_dev = all_devices[i];
            let dev = &mut *dev_ptr.add(i);

            // Write the profile name if assigned, otherwise leave it zeroed
            if let Some(profile_name) = &app_dev.profile_id {
                for (idx, c) in profile_name
                    .encode_utf16()
                    .enumerate()
                    .take(ra::MAX_NAME_LEN - 1)
                {
                    dev.profile[idx] = c;
                }
            }

            for (idx, c) in app_dev
                .id
                .encode_utf16()
                .enumerate()
                .take(ra::MAX_DEV_ID_LEN - 1)
            {
                dev.id[idx] = c;
            }

            let def_clamp = (*base_ptr).default_dev_cfg.clamp;
            dev.config = ra::device_config {
                disable: app_dev.disable,
                set_extra_info: app_dev.set_extra_info,
                poll_time_lock: app_dev.poll_time_lock,
                dpi: app_dev.dpi as i32,
                polling_rate: app_dev.polling_rate as i32,
                clamp: ra::time_clamp {
                    min: if app_dev.clamp_min > 0.0 {
                        app_dev.clamp_min
                    } else {
                        def_clamp.min
                    },
                    max: if app_dev.clamp_max > 0.0 {
                        app_dev.clamp_max
                    } else {
                        def_clamp.max
                    },
                },
            };
        }

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
