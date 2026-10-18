//! Stock v1.7.x packet layout, independently checked against pinned upstream C++.
//! Editor/native Tiered models remain intact; conversion rejects Tiered explicitly.
use crate::{models as ra, wire::wire_struct};
use ra::*;
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct StockAccelArgs {
    pub mode: i32,
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
    pub cap: vec2d,
    pub cap_mode: cap_mode,
    pub length: i32,
    pub data: [f32; LUT_RAW_DATA_CAPACITY],
}
wire_struct!(
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
);
impl TryFrom<&ra::accel_args> for StockAccelArgs {
    type Error = String;
    fn try_from(value: &ra::accel_args) -> Result<Self, String> {
        let mode = match value.mode {
            ra::accel_mode::tiered => return Err("Tiered is preview only with the stock driver; switch to a supported mode before applying".into()),
            ra::accel_mode::noaccel => 6,
            other => other as i32,
        };
        Ok(Self {
            mode: mode,
            gain: value.gain,
            input_offset: value.input_offset,
            output_offset: value.output_offset,
            acceleration: value.acceleration,
            decay_rate: value.decay_rate,
            gamma: value.gamma,
            motivity: value.motivity,
            exponent_classic: value.exponent_classic,
            scale: value.scale,
            exponent_power: value.exponent_power,
            limit: value.limit,
            sync_speed: value.sync_speed,
            smooth: value.smooth,
            cap: value.cap,
            cap_mode: value.cap_mode,
            length: value.length,
            data: value.data,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct StockProfile {
    pub name: [u16; MAX_NAME_LEN],
    pub domain_weights: vec2d,
    pub range_weights: vec2d,
    pub accel_x: StockAccelArgs,
    pub accel_y: StockAccelArgs,
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
wire_struct!(
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
);
impl TryFrom<&ra::profile> for StockProfile {
    type Error = String;
    fn try_from(value: &ra::profile) -> Result<Self, String> {
        Ok(Self {
            name: value.name,
            domain_weights: value.domain_weights,
            range_weights: value.range_weights,
            accel_x: StockAccelArgs::try_from(&value.accel_x)?,
            accel_y: StockAccelArgs::try_from(&value.accel_y)?,
            speed_processor_args: value.speed_processor_args,
            output_dpi: value.output_dpi,
            yx_output_dpi_ratio: value.yx_output_dpi_ratio,
            lr_output_dpi_ratio: value.lr_output_dpi_ratio,
            ud_output_dpi_ratio: value.ud_output_dpi_ratio,
            degrees_rotation: value.degrees_rotation,
            degrees_snap: value.degrees_snap,
            speed_min: value.speed_min,
            speed_max: value.speed_max,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct StockModifier {
    pub prof: StockProfile,
    pub data: modifier_settings_data,
}
wire_struct!(StockModifier, prof, data);
impl TryFrom<&ra::modifier_settings> for StockModifier {
    type Error = String;
    fn try_from(value: &ra::modifier_settings) -> Result<Self, String> {
        Ok(Self {
            prof: StockProfile::try_from(&value.prof)?,
            data: value.data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stock_enum_mapping_is_explicit_and_tiered_is_never_downgraded() {
        for (mode, expected) in [
            (ra::accel_mode::classic, 0),
            (ra::accel_mode::jump, 1),
            (ra::accel_mode::natural, 2),
            (ra::accel_mode::synchronous, 3),
            (ra::accel_mode::power, 4),
            (ra::accel_mode::lookup, 5),
            (ra::accel_mode::noaccel, 6),
        ] {
            let args = ra::accel_args {
                mode,
                ..Default::default()
            };
            assert_eq!(StockAccelArgs::try_from(&args).unwrap().mode, expected);
        }
        let args = ra::accel_args {
            mode: ra::accel_mode::tiered,
            ..Default::default()
        };
        assert!(StockAccelArgs::try_from(&args).is_err());
    }
}
