#![allow(dead_code, non_camel_case_types, non_snake_case)]

// ── Constants ────────────────────────────────────────────────────────────
// Mirrored from common/rawaccel-base.hpp

pub const MAX_NAME_LEN: usize = 256;
pub const MAX_DEV_ID_LEN: usize = 200;
pub const LUT_RAW_DATA_CAPACITY: usize = 514;
pub const LUT_POINTS_CAPACITY: usize = LUT_RAW_DATA_CAPACITY / 2;
pub const MAX_NORM: f64 = 16.0;
pub const NORMALIZED_DPI: f64 = 1000.0;

// ── Enums ────────────────────────────────────────────────────────────────
// Must match the integer values in the C driver header.

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum accel_mode {
    classic = 0,
    jump = 1,
    natural = 2,
    synchronous = 3,
    power = 4,
    lookup = 5,
    tiered = 6,
    noaccel = 7,
}

impl Default for accel_mode {
    fn default() -> Self {
        accel_mode::noaccel
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum tiered_type {
    linear = 0,
    natural = 1,
}

impl Default for tiered_type {
    fn default() -> Self {
        tiered_type::linear
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum cap_mode {
    io = 0,
    in_ = 1,
    out = 2,
}

impl Default for cap_mode {
    fn default() -> Self {
        cap_mode::out
    }
}

// ── Math Primitives ──────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct vec2d {
    pub x: f64,
    pub y: f64,
}

// ── User-Facing Argument Structs ─────────────────────────────────────────
// These are the structs the user fills in the UI / settings.json.

#[repr(C)]
#[derive(Copy, Clone)]
pub struct accel_args {
    pub mode: accel_mode,
    pub gain: bool,

    pub input_offset: f64,
    pub output_offset: f64,
    pub acceleration: f64,
    pub decay_rate: f64,
    pub gamma: f64,
    pub motivity: f64,
    pub exponent_classic: f64,
    pub scale: f64,
    pub exponent_power: f64,
    pub limit: f64,
    pub sync_speed: f64,
    pub smooth: f64,

    // Tiered Mode specific parameters
    pub t_type: tiered_type,
    pub tiered_multiplier1: f64,
    pub tiered_input_offset1: f64,
    pub tiered_multiplier2: f64,
    pub tiered_transition1: f64,
    pub tiered_input_offset2: f64,
    pub tiered_multiplier3: f64,
    pub tiered_transition2: f64,
    pub tiered_decay_rate1: f64,
    pub tiered_decay_rate2: f64,

    pub cap: vec2d,
    pub cap_mode: cap_mode,

    pub length: i32,
    pub data: [f32; LUT_RAW_DATA_CAPACITY],
}

impl Default for accel_args {
    fn default() -> Self {
        Self {
            mode: accel_mode::noaccel,
            gain: true,
            input_offset: 0.0,
            output_offset: 0.0,
            acceleration: 0.005,
            decay_rate: 0.1,
            gamma: 1.0,
            motivity: 1.5,
            exponent_classic: 2.0,
            scale: 1.0,
            exponent_power: 0.05,
            limit: 1.5,
            sync_speed: 5.0,
            smooth: 0.5,
            t_type: tiered_type::linear,
            tiered_multiplier1: 1.0,
            tiered_input_offset1: 0.0,
            tiered_multiplier2: 1.5,
            tiered_transition1: 10.0,
            tiered_input_offset2: 15.0,
            tiered_multiplier3: 2.0,
            tiered_transition2: 25.0,
            tiered_decay_rate1: 0.1,
            tiered_decay_rate2: 0.1,

            cap: vec2d { x: 15.0, y: 1.5 },
            cap_mode: cap_mode::out,
            length: 0,
            data: [0.0; LUT_RAW_DATA_CAPACITY],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct speed_args {
    pub whole: bool,
    pub lp_norm: f64,
    pub input_speed_smooth_halflife: f64,
    pub scale_smooth_halflife: f64,
    pub output_speed_smooth_halflife: f64,
}

impl Default for speed_args {
    fn default() -> Self {
        Self {
            whole: true,
            lp_norm: 2.0,
            input_speed_smooth_halflife: 0.0,
            scale_smooth_halflife: 0.0,
            output_speed_smooth_halflife: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct profile {
    pub name: [u16; MAX_NAME_LEN],
    pub domain_weights: vec2d,
    pub range_weights: vec2d,
    pub accel_x: accel_args,
    pub accel_y: accel_args,
    pub speed_processor_args: speed_args,
    pub output_dpi: f64,
    pub yx_output_dpi_ratio: f64,
    pub lr_output_dpi_ratio: f64,
    pub ud_output_dpi_ratio: f64,
    pub degrees_rotation: f64,
    pub degrees_snap: f64,
    pub speed_min: f64,
    pub speed_max: f64,
}

impl Default for profile {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

// ── Driver Memory Structs ────────────────────────────────────────────────
// These match the C++ layout that the driver expects via DeviceIoControl.

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct modifier_flags {
    pub apply_rotate: bool,
    pub compute_ref_angle: bool,
    pub apply_snap: bool,
    pub clamp_speed: bool,
    pub apply_directional_weight: bool,
    pub apply_dir_mul_x: bool,
    pub apply_dir_mul_y: bool,
}

/// Union of all acceleration curve structs, sized to fit the largest variant.
/// Represented as a raw byte buffer to allow placement of any curve type.
#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct accel_union {
    pub memory: [u8; ACCEL_UNION_SIZE],
}

/// Sized to match the largest C++ union member: activation_framework<LEGACY> at 72 bytes.
/// Must exactly match the C++ sizeof(accel_union) for correct IOCTL buffer layout.
pub const ACCEL_UNION_SIZE: usize = 72;

impl Default for accel_union {
    fn default() -> Self {
        Self {
            memory: [0; ACCEL_UNION_SIZE],
        }
    }
}

impl accel_union {
    /// Encode only named curve fields; never copy uninitialized Rust padding.
    pub(crate) fn write<T: crate::wire::Wire>(&mut self, value: &T) {
        assert!(std::mem::size_of::<T>() <= ACCEL_UNION_SIZE, "curve struct exceeds union capacity");
        self.memory.fill(0);
        value.write(&mut self.memory);
    }

}

/// Inner data_t struct inside modifier_settings.
/// Layout must match C++: modifier_flags, then vec2d rot_direction, then two accel_unions.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct modifier_settings_data {
    pub flags: modifier_flags,
    pub rot_direction: vec2d,
    pub accel_x: accel_union,
    pub accel_y: accel_union,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct modifier_settings {
    pub prof: profile,
    pub data: modifier_settings_data,
}

// ── Device Structs ───────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct time_clamp {
    pub min: f64,
    pub max: f64,
}

impl Default for time_clamp {
    fn default() -> Self {
        Self {
            min: 1000.0 / 8000.0 / 2.0, // DEFAULT_TIME_MIN
            max: 100.0,                 // DEFAULT_TIME_MAX
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct device_config {
    pub disable: bool,
    pub set_extra_info: bool,
    pub poll_time_lock: bool,
    pub dpi: i32,
    pub polling_rate: i32,
    pub clamp: time_clamp,
}

impl Default for device_config {
    fn default() -> Self {
        Self {
            disable: false,
            set_extra_info: false,
            poll_time_lock: false,
            dpi: 0,
            polling_rate: 0,
            clamp: time_clamp::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct device_settings {
    pub name: [u16; MAX_NAME_LEN],
    pub profile: [u16; MAX_NAME_LEN],
    pub id: [u16; MAX_DEV_ID_LEN],
    pub config: device_config,
}

impl Default for device_settings {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct io_base {
    pub default_dev_cfg: device_config,
    pub modifier_data_size: u32,
    pub device_data_size: u32,
}

/// Exact response written by common/rawaccel-version.h (three C int fields).
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct version_t {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}
