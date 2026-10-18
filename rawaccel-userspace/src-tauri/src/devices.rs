use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_Interface_PropertyW, CR_BUFFER_SMALL, CR_SUCCESS,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_GetManufacturerString, HidD_GetProductString,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_InstanceId, DEVPROPTYPE, DEVPROP_TYPE_STRING,
};
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::UI::Input::{
    GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME,
    RIM_TYPEMOUSE,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
    pub id: String,
}

const HID_STR_MAX_LEN: usize = 127;

fn get_device_list() -> Option<Vec<RAWINPUTDEVICELIST>> {
    read_device_list(|output, count| unsafe {
        GetRawInputDeviceList(
            output.map(|b| b.as_mut_ptr()),
            count,
            std::mem::size_of::<RAWINPUTDEVICELIST>() as u32,
        )
    })
}

fn read_device_list(
    mut read: impl FnMut(Option<&mut [RAWINPUTDEVICELIST]>, &mut u32) -> u32,
) -> Option<Vec<RAWINPUTDEVICELIST>> {
    // Devices may arrive/disappear between the two API calls. Retry a fresh query.
    for _ in 0..3 {
        let mut count = 0;
        if read(None, &mut count) != 0 {
            return None;
        }
        if count == 0 {
            return Some(vec![]);
        }
        let mut list = Vec::new();
        list.try_reserve_exact(count as usize).ok()?;
        list.resize(count as usize, RAWINPUTDEVICELIST::default());
        let returned = read(Some(&mut list), &mut count);
        if returned == u32::MAX {
            continue;
        }
        if count as usize > list.len() || returned as usize > list.len() {
            return None;
        }
        // The return value is the number actually copied, not the original query count.
        list.truncate(returned as usize);
        return Some(list);
    }
    None
}

pub fn get_connected_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let dev_list = match get_device_list() {
        Some(list) => list,
        None => return devices,
    };

    for dev_list_entry in dev_list {
        if dev_list_entry.dwType != RIM_TYPEMOUSE {
            continue;
        }

        let handle = dev_list_entry.hDevice;

        let mut name_len = 0u32;
        unsafe {
            if GetRawInputDeviceInfoW(Some(handle), RIDI_DEVICENAME, None, &mut name_len)
                == u32::MAX
            {
                continue;
            }
        }

        let capacity = match name_len.checked_add(1) {
            Some(n) if n > 1 => n,
            _ => continue,
        };
        let mut interface_buffer = match capacity
            .checked_mul(2)
            .and_then(crate::input_buffers::InputBuffer::new)
        {
            Some(buffer) => buffer,
            None => continue,
        };
        name_len = capacity;
        let returned = unsafe {
            GetRawInputDeviceInfoW(
                Some(handle),
                RIDI_DEVICENAME,
                Some(interface_buffer.as_mut_ptr()),
                &mut name_len,
            )
        };
        if returned == u32::MAX || returned > capacity || name_len > capacity {
            continue;
        }
        let name_bytes = match name_len
            .checked_mul(2)
            .and_then(|n| interface_buffer.bytes(n))
        {
            Some(bytes) => bytes,
            None => continue,
        };
        let interface_name: Vec<u16> = name_bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if crate::input_buffers::terminated(&interface_name).is_none() {
            continue;
        }

        let mut name_str = String::new();
        unsafe {
            let hid_dev_object_res = CreateFileW(
                PCWSTR(interface_name.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            );

            if let Ok(hid_dev_object) = hid_dev_object_res {
                if hid_dev_object != INVALID_HANDLE_VALUE {
                    let mut product_buf = [0u16; HID_STR_MAX_LEN];
                    if HidD_GetProductString(
                        hid_dev_object,
                        product_buf.as_mut_ptr() as *mut _,
                        (HID_STR_MAX_LEN * 2) as u32,
                    ) {
                        let product_sv = OsString::from_wide(
                            crate::input_buffers::terminated(&product_buf).unwrap_or(&[]),
                        )
                        .to_string_lossy()
                        .trim_end_matches('\0')
                        .to_string();

                        let mut manufacturer_buf = [0u16; HID_STR_MAX_LEN];
                        if HidD_GetManufacturerString(
                            hid_dev_object,
                            manufacturer_buf.as_mut_ptr() as *mut _,
                            (HID_STR_MAX_LEN * 2) as u32,
                        ) {
                            let manufacturer_sv = OsString::from_wide(
                                crate::input_buffers::terminated(&manufacturer_buf).unwrap_or(&[]),
                            )
                            .to_string_lossy()
                            .trim_end_matches('\0')
                            .to_string();

                            if product_sv.starts_with(&manufacturer_sv) {
                                name_str = product_sv;
                            } else {
                                name_str = format!("{} {}", manufacturer_sv, product_sv);
                            }
                        } else {
                            name_str = product_sv;
                        }
                    }
                    let _ = CloseHandle(hid_dev_object);
                }
            }
        }

        let mut id_size = 0u32;
        let mut prop_type = DEVPROPTYPE(0);
        unsafe {
            let cm_res = CM_Get_Device_Interface_PropertyW(
                PCWSTR(interface_name.as_ptr()),
                &DEVPKEY_Device_InstanceId,
                &mut prop_type,
                None,
                &mut id_size,
                0,
            );

            if cm_res != CR_BUFFER_SMALL && cm_res != CR_SUCCESS {
                continue;
            }

            if prop_type != DEVPROP_TYPE_STRING || id_size < 2 {
                continue;
            }
            let mut id_buf = Vec::new();
            if id_buf.try_reserve_exact(id_size as usize).is_err() {
                continue;
            }
            id_buf.resize(id_size as usize, 0u8);
            let cm_res2 = CM_Get_Device_Interface_PropertyW(
                PCWSTR(interface_name.as_ptr()),
                &DEVPKEY_Device_InstanceId,
                &mut prop_type,
                Some(id_buf.as_mut_ptr()),
                &mut id_size,
                0,
            );

            if cm_res2 != CR_SUCCESS || prop_type != DEVPROP_TYPE_STRING {
                continue;
            }

            let mut id_str = match crate::input_buffers::property_string(&id_buf, id_size) {
                Some(id) => id,
                None => continue,
            };

            if let Some(idx) = id_str.rfind('\\') {
                id_str = id_str[..idx].to_string();
            }

            if !devices.iter().any(|d: &DeviceInfo| d.id == id_str) {
                devices.push(DeviceInfo {
                    name: name_str,
                    id: id_str,
                });
            }
        }
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_growth_retries_and_uses_actual_copied_count() {
        let mut calls = 0;
        let list = read_device_list(|output, count| {
            calls += 1;
            match calls {
                1 => {
                    assert!(output.is_none());
                    *count = 1;
                    0
                }
                2 => {
                    assert_eq!(output.unwrap().len(), 1);
                    *count = 3;
                    u32::MAX
                }
                3 => {
                    *count = 3;
                    0
                }
                4 => {
                    assert_eq!(output.unwrap().len(), 3);
                    2
                }
                _ => panic!("Unexpected retry"),
            }
        })
        .unwrap();
        assert_eq!(list.len(), 2);
    }
    #[test]
    fn list_reader_bounds_count_and_retries() {
        assert!(read_device_list(|out, count| {
            if out.is_none() {
                *count = 1;
                0
            } else {
                *count = 2;
                2
            }
        })
        .is_none());
        let mut calls = 0;
        assert!(read_device_list(|out, count| {
            calls += 1;
            if out.is_none() {
                *count = 1;
                0
            } else {
                u32::MAX
            }
        })
        .is_none());
        assert_eq!(calls, 6);
        assert!(read_device_list(|_, count| {
            *count = 0;
            0
        })
        .unwrap()
        .is_empty());
    }
}
