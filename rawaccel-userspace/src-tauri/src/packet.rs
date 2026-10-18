//! Offline deterministic stock/custom packet encoding; only the selected contract reaches Apply.
use crate::{config::AppConfig, models as ra, wire::Wire};
use std::mem::size_of;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn encode(config: &AppConfig) -> Result<Vec<u8>, String> {
    encode_layout(config, true)
}

pub(crate) fn encode_for_driver(config: &AppConfig) -> Result<Vec<u8>, String> {
    crate::driver_contract::require_supported_modes(config)?;
    encode_layout(config, crate::driver_contract::NATIVE_TIERED)
}

#[cfg(test)]
pub(crate) fn encode_stock(config: &AppConfig) -> Result<Vec<u8>, String> {
    encode_layout(config, false)
}

fn encode_layout(config: &AppConfig, native_tiered: bool) -> Result<Vec<u8>, String> {
    config.validate()?;
    if !native_tiered {
        crate::driver_contract::require_stock_modes(config)?;
    }
    let stride = if native_tiered {
        size_of::<ra::modifier_settings>()
    } else {
        size_of::<crate::stock_wire::StockModifier>()
    };
    let (modifier_bytes, total) =
        packet_size_for(config.profiles.len(), config.devices.len(), stride)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(total)
        .map_err(|_| "Unable to allocate settings packet")?;
    bytes.resize(total, 0);
    let mut base = ra::io_base::default();
    base.modifier_data_size =
        u32::try_from(config.profiles.len()).map_err(|_| "Too many profiles")?;
    base.device_data_size = u32::try_from(config.devices.len()).map_err(|_| "Too many devices")?;
    if let Some(defaults) = &config.default_device_config {
        base.default_dev_cfg = defaults.to_native()?;
    }
    base.write(&mut bytes[..size_of::<ra::io_base>()]);
    for (i, profile) in config.profiles.iter().enumerate() {
        let mut settings = ra::modifier_settings {
            prof: profile.to_native()?,
            ..Default::default()
        };
        crate::math::init_data(&mut settings);
        let start = size_of::<ra::io_base>() + i * stride;
        if native_tiered {
            settings.write(&mut bytes[start..start + stride]);
        } else {
            crate::stock_wire::StockModifier::try_from(&settings)?
                .write(&mut bytes[start..start + stride]);
        }
    }
    for (i, input) in config.devices.iter().enumerate() {
        let mut device = ra::device_settings::default();
        // Length and terminator safety were checked before allocation.
        for (out, unit) in device.id.iter_mut().zip(input.id.encode_utf16()) {
            *out = unit;
        }
        if let Some(name) = &input.profile_id {
            for (out, unit) in device.profile.iter_mut().zip(name.encode_utf16()) {
                *out = unit;
            }
        }
        let inherited = base.default_dev_cfg.clamp;
        device.config = ra::device_config {
            disable: input.disable,
            set_extra_info: input.set_extra_info,
            poll_time_lock: input.poll_time_lock,
            dpi: i32::try_from(input.dpi).map_err(|_| "Device DPI exceeds i32")?,
            polling_rate: i32::try_from(input.polling_rate)
                .map_err(|_| "Polling rate exceeds i32")?,
            clamp: ra::time_clamp {
                min: if input.clamp_min == 0.0 {
                    inherited.min
                } else {
                    input.clamp_min
                },
                max: if input.clamp_max == 0.0 {
                    inherited.max
                } else {
                    input.clamp_max
                },
            },
        };
        let start =
            size_of::<ra::io_base>() + modifier_bytes + i * size_of::<ra::device_settings>();
        device.write(&mut bytes[start..start + size_of::<ra::device_settings>()]);
    }
    Ok(bytes)
}

/// Check all arithmetic before allocating or narrowing counts for DeviceIoControl.
#[cfg(test)]
fn packet_size(profiles: usize, devices: usize) -> Result<(usize, usize), String> {
    packet_size_for(profiles, devices, size_of::<ra::modifier_settings>())
}
fn packet_size_for(
    profiles: usize,
    devices: usize,
    stride: usize,
) -> Result<(usize, usize), String> {
    let error = || "Settings packet exceeds the driver's 32-bit size/count limit".to_string();
    u32::try_from(profiles).map_err(|_| error())?;
    u32::try_from(devices).map_err(|_| error())?;
    let modifiers = stride.checked_mul(profiles).ok_or_else(error)?;
    let device_bytes = size_of::<ra::device_settings>()
        .checked_mul(devices)
        .ok_or_else(error)?;
    let total = size_of::<ra::io_base>()
        .checked_add(modifiers)
        .and_then(|n| n.checked_add(device_bytes))
        .ok_or_else(error)?;
    u32::try_from(total).map_err(|_| error())?;
    Ok((modifiers, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;
    #[test]
    fn packet_limits_are_checked_without_allocating() {
        let (modifiers, total) = packet_size(2, 3).unwrap();
        assert_eq!(modifiers, 2 * size_of::<ra::modifier_settings>());
        assert_eq!(
            total,
            size_of::<ra::io_base>() + modifiers + 3 * size_of::<ra::device_settings>()
        );
        for (p, d) in [
            (usize::MAX, 0),
            (0, usize::MAX),
            (u32::MAX as usize, 0),
            (0, u32::MAX as usize),
        ] {
            assert!(packet_size(p, d).is_err());
        }
    }
    #[test]
    fn packet_is_repeatable_and_device_defaults_are_inherited() {
        let mut c = AppConfig {
            profiles: vec![crate::config::AppProfile::default()],
            devices: vec![],
            default_device_config: Some(crate::config::AppDeviceDefaults {
                clamp_min: 2.0,
                clamp_max: 4.0,
                ..Default::default()
            }),
        };
        c.devices.push(crate::config::AppDeviceConfig {
            id: "mouse".into(),
            disable: true,
            dpi: 800,
            ..Default::default()
        });
        let bytes = encode(&c).unwrap();
        assert_eq!(bytes, encode(&c).unwrap());
        assert_eq!(
            bytes.len(),
            size_of::<ra::io_base>()
                + size_of::<ra::modifier_settings>()
                + size_of::<ra::device_settings>()
        );
        let start = size_of::<ra::io_base>()
            + size_of::<ra::modifier_settings>()
            + offset_of!(ra::device_settings, config);
        assert_eq!(bytes[start], 1); // Unmapped disabled mice must still be transmitted.
        assert_eq!(&bytes[start + 3..start + 4], &[0]);
        let clamp = start + offset_of!(ra::device_config, clamp);
        assert_eq!(&bytes[clamp..clamp + 8], &2.0f64.to_le_bytes());
        assert_eq!(&bytes[clamp + 8..clamp + 16], &4.0f64.to_le_bytes());
        let mut misaligned = vec![0xa5; bytes.len() + 1];
        let base = ra::io_base::default();
        base.write(&mut misaligned[1..]);
        assert_eq!(misaligned[0], 0xa5); // Encoding does not need aligned byte storage.
    }
    #[test]
    fn union_padding_and_unused_tail_are_zero_after_reinitialization() {
        let mut settings = ra::modifier_settings::default();
        settings.prof = crate::config::AppProfile::default().to_native().unwrap();
        settings.prof.accel_x.mode = ra::accel_mode::synchronous;
        settings.prof.accel_x.gain = false;
        settings.data.accel_x.memory.fill(0xa5);
        crate::math::init_data(&mut settings);
        assert_eq!(&settings.data.accel_x.memory[49..56], &[0; 7]);
        settings.prof.accel_x.mode = ra::accel_mode::lookup;
        settings.prof.accel_x.length = 4;
        crate::math::init_data(&mut settings);
        assert!(settings.data.accel_x.memory[5..].iter().all(|b| *b == 0));
        settings.prof.accel_x.mode = ra::accel_mode::noaccel;
        crate::math::init_data(&mut settings);
        assert_eq!(settings.data.accel_x.memory, [0; ra::ACCEL_UNION_SIZE]);
    }
}
