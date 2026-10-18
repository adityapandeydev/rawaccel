//! Explicit Windows ABI encoding. Only named fields are copied; padding stays zero.
//! Byte buffers are never dereferenced as native structs. Windows is little endian.
use crate::models::*;

pub(crate) trait Wire {
    fn write(&self, bytes: &mut [u8]);
}
macro_rules! scalar {
    ($($ty:ty),*) => {$(impl Wire for $ty {
        fn write(&self, bytes: &mut [u8]) { bytes[..std::mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes()); }
    })*};
}
scalar!(u16, u32, i32, f32, f64);
impl Wire for u8 {
    fn write(&self, bytes: &mut [u8]) {
        bytes[0] = *self;
    }
}
impl Wire for bool {
    fn write(&self, bytes: &mut [u8]) {
        bytes[0] = u8::from(*self);
    }
}
impl<T: Wire, const N: usize> Wire for [T; N] {
    fn write(&self, bytes: &mut [u8]) {
        let size = std::mem::size_of::<T>();
        for (value, output) in self.iter().zip(bytes[..size * N].chunks_exact_mut(size)) {
            value.write(output);
        }
    }
}
macro_rules! wire_struct {
    ($ty:ty, $($field:ident),* $(,)?) => {
        impl $crate::wire::Wire for $ty {
            fn write(&self, bytes: &mut [u8]) {
                bytes[..std::mem::size_of::<Self>()].fill(0);
                $(let start = std::mem::offset_of!(Self, $field);
                $crate::wire::Wire::write(&self.$field, &mut bytes[start..start + std::mem::size_of_val(&self.$field)]);)*
            }
        }
    };
}
pub(crate) use wire_struct;
impl Wire for accel_mode {
    fn write(&self, bytes: &mut [u8]) {
        (*self as i32).write(bytes);
    }
}
impl Wire for cap_mode {
    fn write(&self, bytes: &mut [u8]) {
        (*self as i32).write(bytes);
    }
}
impl Wire for tiered_type {
    fn write(&self, bytes: &mut [u8]) {
        (*self as i32).write(bytes);
    }
}
wire_struct!(vec2d, x, y);
wire_struct!(
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
wire_struct!(
    speed_args,
    whole,
    lp_norm,
    input_speed_smooth_halflife,
    scale_smooth_halflife,
    output_speed_smooth_halflife
);
wire_struct!(
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
wire_struct!(
    modifier_flags,
    apply_rotate,
    compute_ref_angle,
    apply_snap,
    clamp_speed,
    apply_directional_weight,
    apply_dir_mul_x,
    apply_dir_mul_y
);
wire_struct!(accel_union, memory);
wire_struct!(
    modifier_settings_data,
    flags,
    rot_direction,
    accel_x,
    accel_y
);
wire_struct!(modifier_settings, prof, data);
wire_struct!(time_clamp, min, max);
wire_struct!(
    device_config,
    disable,
    set_extra_info,
    poll_time_lock,
    dpi,
    polling_rate,
    clamp
);
wire_struct!(device_settings, name, profile, id, config);
wire_struct!(
    io_base,
    default_dev_cfg,
    modifier_data_size,
    device_data_size
);

wire_struct!(version_t, major, minor, patch);
