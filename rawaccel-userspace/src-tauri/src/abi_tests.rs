//! ABI snapshots are produced by tests/abi/reference.cpp using MSVC and actual common headers.
//! Refresh/check with: python scripts/check_driver_abi.py [--update]
//! These are byte/layout checks, not a claim of complete curve-math parity.
use crate::{models::*, wire::Wire};
use serde_json::{json, Value};
use std::mem::{align_of, offset_of, size_of};
macro_rules! layout {
    ($ty:ty, $($field:ident),* $(,)?) => {{ let mut value = json!({ "size": size_of::<$ty>(), "align": align_of::<$ty>() }); $(value[stringify!($field)] = json!(offset_of!($ty, $field));)* value }};
}
fn reference() -> Value {
    serde_json::from_str(include_str!("../tests/fixtures/abi/layout-x64.json")).unwrap()
}
#[test]
fn all_wire_fields_match_the_custom_cpp_contract() {
    let expected = reference();
    assert_eq!(layout!(vec2d, x, y), expected["vec2d"], "vec2d");
    assert_eq!(
        layout!(
            accel_args,
            mode,
            gain,
            input_offset,
            output_offset,
            acceleration,
            decay_rate,
            gamma,
            motivity,
            exponent_classic,
            scale,
            exponent_power,
            limit,
            sync_speed,
            smooth,
            t_type,
            tiered_multiplier1,
            tiered_input_offset1,
            tiered_multiplier2,
            tiered_transition1,
            tiered_input_offset2,
            tiered_multiplier3,
            tiered_transition2,
            tiered_decay_rate1,
            tiered_decay_rate2,
            cap,
            cap_mode,
            length,
            data
        ),
        expected["accel_args"],
        "accel_args"
    );
    assert_eq!(
        layout!(
            speed_args,
            whole,
            lp_norm,
            input_speed_smooth_halflife,
            scale_smooth_halflife,
            output_speed_smooth_halflife
        ),
        expected["speed_args"],
        "speed_args"
    );
    assert_eq!(
        layout!(
            profile,
            name,
            domain_weights,
            range_weights,
            accel_x,
            accel_y,
            speed_processor_args,
            output_dpi,
            yx_output_dpi_ratio,
            lr_output_dpi_ratio,
            ud_output_dpi_ratio,
            degrees_rotation,
            degrees_snap,
            speed_min,
            speed_max
        ),
        expected["profile"],
        "profile"
    );
    assert_eq!(
        layout!(
            modifier_flags,
            apply_rotate,
            compute_ref_angle,
            apply_snap,
            clamp_speed,
            apply_directional_weight,
            apply_dir_mul_x,
            apply_dir_mul_y
        ),
        expected["modifier_flags"],
        "modifier_flags"
    );
    assert_eq!(
        layout!(accel_union, memory),
        expected["accel_union"],
        "accel_union"
    );
    assert_eq!(
        layout!(
            modifier_settings_data,
            flags,
            rot_direction,
            accel_x,
            accel_y
        ),
        expected["modifier_settings_data"],
        "modifier_settings_data"
    );
    assert_eq!(
        layout!(modifier_settings, prof, data),
        expected["modifier_settings"],
        "modifier_settings"
    );
    assert_eq!(
        layout!(time_clamp, min, max),
        expected["time_clamp"],
        "time_clamp"
    );
    assert_eq!(
        layout!(
            device_config,
            disable,
            set_extra_info,
            poll_time_lock,
            dpi,
            polling_rate,
            clamp
        ),
        expected["device_config"],
        "device_config"
    );
    assert_eq!(
        layout!(device_settings, name, profile, id, config),
        expected["device_settings"],
        "device_settings"
    );
    assert_eq!(
        layout!(
            io_base,
            default_dev_cfg,
            modifier_data_size,
            device_data_size
        ),
        expected["io_base"],
        "io_base"
    );
    assert_eq!(
        layout!(version_t, major, minor, patch),
        expected["version_t"],
        "version_t"
    );
    assert_eq!(
        json!({ "size": size_of::<accel_mode>(), "align": align_of::<accel_mode>() }),
        expected["accel_mode"],
        "accel_mode"
    );
    assert_eq!(
        json!({ "size": size_of::<cap_mode>(), "align": align_of::<cap_mode>() }),
        expected["cap_mode"],
        "cap_mode"
    );
    assert_eq!(
        json!({ "size": size_of::<tiered_type>(), "align": align_of::<tiered_type>() }),
        expected["tiered_type"],
        "tiered_type"
    );
    assert_eq!(
        json!({ "size": size_of::<bool>(), "align": align_of::<bool>() }),
        expected["cpp_bool"],
        "cpp_bool"
    );
    assert_eq!(
        layout!(
            windows::Win32::UI::Input::RAWINPUTHEADER,
            dwType,
            dwSize,
            hDevice,
            wParam
        ),
        expected["RAWINPUTHEADER"],
        "RAWINPUTHEADER"
    );
    assert_eq!(
        layout!(
            windows::Win32::UI::Input::RAWMOUSE,
            usFlags,
            ulRawButtons,
            lLastX,
            lLastY,
            ulExtraInformation
        ),
        expected["RAWMOUSE"],
        "RAWMOUSE"
    );
    assert_eq!(
        layout!(windows::Win32::UI::Input::RAWINPUT, header, data),
        expected["RAWINPUT"],
        "RAWINPUT"
    );
    assert_eq!(
        layout!(
            windows::Win32::UI::Input::RAWINPUTDEVICELIST,
            hDevice,
            dwType
        ),
        expected["RAWINPUTDEVICELIST"],
        "RAWINPUTDEVICELIST"
    );
    assert_eq!(
        crate::math::curve_layouts(),
        expected
            .as_object()
            .unwrap()
            .iter()
            .filter(|(k, _)| k.chars().next().unwrap().is_uppercase() && !k.starts_with("RAW"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<serde_json::Map<_, _>>()
    );
    assert_eq!(
        json!(MAX_NAME_LEN),
        expected["constants"]["MAX_NAME_LEN"],
        "MAX_NAME_LEN"
    );
    assert_eq!(
        json!(MAX_DEV_ID_LEN),
        expected["constants"]["MAX_DEV_ID_LEN"],
        "MAX_DEV_ID_LEN"
    );
    assert_eq!(
        json!(LUT_RAW_DATA_CAPACITY),
        expected["constants"]["LUT_RAW_DATA_CAPACITY"],
        "LUT_RAW_DATA_CAPACITY"
    );
    assert_eq!(
        json!(crate::driver::IOCTL_READ),
        expected["constants"]["READ"],
        "READ"
    );
    assert_eq!(
        json!(crate::driver::IOCTL_WRITE),
        expected["constants"]["WRITE"],
        "WRITE"
    );
    assert_eq!(
        json!(crate::driver::IOCTL_GET_VERSION),
        expected["constants"]["GET_VERSION"],
        "GET_VERSION"
    );
    assert_eq!(
        json!(accel_mode::classic as i32),
        expected["constants"]["accel_mode.classic"],
        "accel_mode.classic"
    );
    assert_eq!(
        json!(accel_mode::jump as i32),
        expected["constants"]["accel_mode.jump"],
        "accel_mode.jump"
    );
    assert_eq!(
        json!(accel_mode::natural as i32),
        expected["constants"]["accel_mode.natural"],
        "accel_mode.natural"
    );
    assert_eq!(
        json!(accel_mode::synchronous as i32),
        expected["constants"]["accel_mode.synchronous"],
        "accel_mode.synchronous"
    );
    assert_eq!(
        json!(accel_mode::power as i32),
        expected["constants"]["accel_mode.power"],
        "accel_mode.power"
    );
    assert_eq!(
        json!(accel_mode::lookup as i32),
        expected["constants"]["accel_mode.lookup"],
        "accel_mode.lookup"
    );
    assert_eq!(
        json!(accel_mode::tiered as i32),
        expected["constants"]["accel_mode.tiered"],
        "accel_mode.tiered"
    );
    assert_eq!(
        json!(accel_mode::noaccel as i32),
        expected["constants"]["accel_mode.noaccel"],
        "accel_mode.noaccel"
    );
    assert_eq!(
        json!(cap_mode::io as i32),
        expected["constants"]["cap_mode.io"],
        "cap_mode.io"
    );
    assert_eq!(
        json!(cap_mode::in_ as i32),
        expected["constants"]["cap_mode.in"],
        "cap_mode.in"
    );
    assert_eq!(
        json!(cap_mode::out as i32),
        expected["constants"]["cap_mode.out"],
        "cap_mode.out"
    );
    assert_eq!(
        json!(tiered_type::linear as i32),
        expected["constants"]["tiered_type.linear"],
        "tiered_type.linear"
    );
    assert_eq!(
        json!(tiered_type::natural as i32),
        expected["constants"]["tiered_type.natural"],
        "tiered_type.natural"
    );
}

trait Fill {
    fn fill(&mut self, tag: &mut u32);
}
macro_rules! scalar_fill {
    ($($ty:ty),*) => {$(impl Fill for $ty { fn fill(&mut self, tag: &mut u32) { *tag += 1; *self = *tag as Self; } })*};
}
scalar_fill!(u8, u16, u32, i32);
impl Fill for f32 {
    fn fill(&mut self, tag: &mut u32) {
        *tag += 1;
        *self = *tag as f32 / 8.0;
    }
}
impl Fill for f64 {
    fn fill(&mut self, tag: &mut u32) {
        *tag += 1;
        *self = *tag as f64 / 8.0;
    }
}
impl Fill for bool {
    fn fill(&mut self, tag: &mut u32) {
        *tag += 1;
        *self = *tag % 2 != 0;
    }
}
impl<T: Fill, const N: usize> Fill for [T; N] {
    fn fill(&mut self, tag: &mut u32) {
        for v in self {
            v.fill(tag);
        }
    }
}
impl Fill for accel_mode {
    fn fill(&mut self, tag: &mut u32) {
        *tag += 1;
        *self = [
            accel_mode::classic,
            accel_mode::jump,
            accel_mode::natural,
            accel_mode::synchronous,
            accel_mode::power,
            accel_mode::lookup,
            accel_mode::tiered,
            accel_mode::noaccel,
        ][(*tag % 8) as usize];
    }
}
impl Fill for cap_mode {
    fn fill(&mut self, tag: &mut u32) {
        *tag += 1;
        *self = [cap_mode::io, cap_mode::in_, cap_mode::out][(*tag % 3) as usize];
    }
}
impl Fill for tiered_type {
    fn fill(&mut self, tag: &mut u32) {
        *tag += 1;
        *self = [tiered_type::linear, tiered_type::natural][(*tag % 2) as usize];
    }
}
macro_rules! fill_struct {
    ($ty:ty, $($field:ident),* $(,)?) => { impl Fill for $ty { fn fill(&mut self, tag: &mut u32) { $(self.$field.fill(tag);)* } } };
}
fill_struct!(vec2d, x, y);
fill_struct!(
    accel_args,
    mode,
    gain,
    input_offset,
    output_offset,
    acceleration,
    decay_rate,
    gamma,
    motivity,
    exponent_classic,
    scale,
    exponent_power,
    limit,
    sync_speed,
    smooth,
    t_type,
    tiered_multiplier1,
    tiered_input_offset1,
    tiered_multiplier2,
    tiered_transition1,
    tiered_input_offset2,
    tiered_multiplier3,
    tiered_transition2,
    tiered_decay_rate1,
    tiered_decay_rate2,
    cap,
    cap_mode,
    length,
    data
);
fill_struct!(
    speed_args,
    whole,
    lp_norm,
    input_speed_smooth_halflife,
    scale_smooth_halflife,
    output_speed_smooth_halflife
);
fill_struct!(
    profile,
    name,
    domain_weights,
    range_weights,
    accel_x,
    accel_y,
    speed_processor_args,
    output_dpi,
    yx_output_dpi_ratio,
    lr_output_dpi_ratio,
    ud_output_dpi_ratio,
    degrees_rotation,
    degrees_snap,
    speed_min,
    speed_max
);
fill_struct!(
    modifier_flags,
    apply_rotate,
    compute_ref_angle,
    apply_snap,
    clamp_speed,
    apply_directional_weight,
    apply_dir_mul_x,
    apply_dir_mul_y
);
fill_struct!(accel_union, memory);
fill_struct!(
    modifier_settings_data,
    flags,
    rot_direction,
    accel_x,
    accel_y
);
fill_struct!(modifier_settings, prof, data);
fill_struct!(time_clamp, min, max);
fill_struct!(
    device_config,
    disable,
    set_extra_info,
    poll_time_lock,
    dpi,
    polling_rate,
    clamp
);
fill_struct!(device_settings, name, profile, id, config);
fill_struct!(
    io_base,
    default_dev_cfg,
    modifier_data_size,
    device_data_size
);
fill_struct!(version_t, major, minor, patch);
#[test]
fn complete_packet_bytes_match_native_cpp_objects_including_padding() {
    let mut tag = 0;
    let mut base = io_base::default();
    base.fill(&mut tag);
    base.modifier_data_size = 2;
    base.device_data_size = 2;
    let mut modifiers = [modifier_settings::default(); 2];
    for value in &mut modifiers {
        value.fill(&mut tag);
    }
    let mut devices = [device_settings::default(); 2];
    for value in &mut devices {
        value.fill(&mut tag);
    }
    let mut bytes = vec![
        0xa5;
        size_of::<io_base>()
            + 2 * size_of::<modifier_settings>()
            + 2 * size_of::<device_settings>()
    ];
    base.write(&mut bytes);
    let mut start = size_of::<io_base>();
    for value in &modifiers {
        value.write(&mut bytes[start..]);
        start += size_of::<modifier_settings>();
    }
    for value in &devices {
        value.write(&mut bytes[start..]);
        start += size_of::<device_settings>();
    }
    let hex: String = include_str!("../tests/fixtures/abi/sentinel-packet-x64.hex")
        .split_whitespace()
        .collect();
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes.len(), expected.len());
    if let Some(i) = bytes.iter().zip(&expected).position(|(a, b)| a != b) {
        panic!(
            "Packet differs at byte {i}: Rust {}, C++ {}",
            bytes[i], expected[i]
        );
    }
}

#[test]
fn validated_tiered_and_device_settings_packet_matches_cpp_reference() {
    let config = crate::config::parse_or_migrate_settings(include_str!(
        "../tests/fixtures/abi/settings.json"
    ))
    .unwrap();
    let actual = crate::packet::encode(&config).unwrap();
    let hex: String = include_str!("../tests/fixtures/abi/settings-packet-x64.hex")
        .split_whitespace()
        .collect();
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(actual.len(), expected.len());
    if let Some(i) = actual.iter().zip(&expected).position(|(a, b)| a != b) {
        panic!(
            "Settings packet differs at byte {i}: Rust {}, C++ {}",
            actual[i], expected[i]
        );
    }
}

#[test]
fn stock_layout_matches_the_pinned_signed_release() {
    use crate::stock_wire::*;
    let expected: Value =
        serde_json::from_str(include_str!("../tests/fixtures/abi/stock-layout-x64.json")).unwrap();
    assert_eq!(
        layout!(
            StockAccelArgs,
            mode,
            gain,
            input_offset,
            output_offset,
            acceleration,
            decay_rate,
            gamma,
            motivity,
            exponent_classic,
            scale,
            exponent_power,
            limit,
            sync_speed,
            smooth,
            cap,
            cap_mode,
            length,
            data
        ),
        expected["StockAccelArgs"],
        "StockAccelArgs"
    );
    assert_eq!(
        layout!(
            StockProfile,
            name,
            domain_weights,
            range_weights,
            accel_x,
            accel_y,
            speed_processor_args,
            output_dpi,
            yx_output_dpi_ratio,
            lr_output_dpi_ratio,
            ud_output_dpi_ratio,
            degrees_rotation,
            degrees_snap,
            speed_min,
            speed_max
        ),
        expected["StockProfile"],
        "StockProfile"
    );
    assert_eq!(
        layout!(StockModifier, prof, data),
        expected["StockModifier"],
        "StockModifier"
    );
    assert_eq!(expected["constants"]["noaccel"], 6);
    assert_eq!(expected["constants"]["union_size"], ACCEL_UNION_SIZE);
    let v = crate::driver_contract::STOCK_VERSION;
    assert_eq!(
        expected["constants"]["version"],
        json!([v.major, v.minor, v.patch])
    );
}
#[test]
fn stock_settings_packet_matches_actual_upstream_cpp_bytes() {
    let config = crate::config::parse_or_migrate_settings(include_str!(
        "../tests/fixtures/abi/stock-settings.json"
    ))
    .unwrap();
    let actual = crate::packet::encode_stock(&config).unwrap();
    let hex: String = include_str!("../tests/fixtures/abi/stock-settings-packet-x64.hex")
        .split_whitespace()
        .collect();
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(actual.len(), expected.len());
    if let Some(i) = actual.iter().zip(&expected).position(|(a, b)| a != b) {
        panic!(
            "Stock packet differs at byte {i}: Rust {}, C++ {}",
            actual[i], expected[i]
        );
    }
    if !crate::driver_contract::NATIVE_TIERED {
        assert_eq!(crate::packet::encode_for_driver(&config).unwrap(), actual);
    }
}
