//! Validation at the userspace/driver boundary. Never repair invalid values silently.
use crate::{
    config::{AppAccelArgs, AppProfile},
    models,
};

fn finite(values: &[(&str, f64)]) -> Result<(), String> {
    for (name, value) in values {
        if !value.is_finite() {
            return Err(format!("{name} must be finite"));
        }
    }
    Ok(())
}

pub(crate) fn accel_numbers(a: &AppAccelArgs) -> Result<(), String> {
    finite(&[
        ("input_offset", a.input_offset),
        ("output_offset", a.output_offset),
        ("acceleration", a.acceleration),
        ("decay_rate", a.decay_rate),
        ("gamma", a.gamma),
        ("motivity", a.motivity),
        ("exponent_classic", a.exponent_classic),
        ("scale", a.scale),
        ("exponent_power", a.exponent_power),
        ("limit", a.limit),
        ("sync_speed", a.sync_speed),
        ("smooth", a.smooth),
        ("tiered_multiplier1", a.tiered_multiplier1),
        ("tiered_input_offset1", a.tiered_input_offset1),
        ("tiered_multiplier2", a.tiered_multiplier2),
        ("tiered_transition1", a.tiered_transition1),
        ("tiered_input_offset2", a.tiered_input_offset2),
        ("tiered_multiplier3", a.tiered_multiplier3),
        ("tiered_transition2", a.tiered_transition2),
        ("tiered_decay_rate1", a.tiered_decay_rate1),
        ("tiered_decay_rate2", a.tiered_decay_rate2),
        ("cap.x", a.cap.x),
        ("cap.y", a.cap.y),
    ])
}

pub(crate) fn profile_numbers(p: &AppProfile) -> Result<(), String> {
    let s = &p.speed_processor_args;
    let check = || -> Result<(), String> {
        finite(&[
            ("output_dpi", p.output_dpi),
            ("yx_output_dpi_ratio", p.yx_output_dpi_ratio),
            ("lr_output_dpi_ratio", p.lr_output_dpi_ratio),
            ("ud_output_dpi_ratio", p.ud_output_dpi_ratio),
            ("degrees_rotation", p.degrees_rotation),
            ("degrees_snap", p.degrees_snap),
            ("speed_min", p.speed_min),
            ("speed_max", p.speed_max),
            ("lp_norm", s.lp_norm),
            ("input_speed_smooth_halflife", s.input_speed_smooth_halflife),
            ("scale_smooth_halflife", s.scale_smooth_halflife),
            (
                "output_speed_smooth_halflife",
                s.output_speed_smooth_halflife,
            ),
            ("domain_weights.x", p.domain_weights.x),
            ("domain_weights.y", p.domain_weights.y),
            ("range_weights.x", p.range_weights.x),
            ("range_weights.y", p.range_weights.y),
        ])?;
        if p.output_dpi == 0.0 || p.yx_output_dpi_ratio == 0.0 {
            return Err("Output DPI and Y/X ratio must be nonzero".into());
        }
        if p.lr_output_dpi_ratio <= 0.0 || p.ud_output_dpi_ratio <= 0.0 {
            return Err("Left/right and up/down DPI ratios must be positive".into());
        }
        if p.domain_weights.x <= 0.0 || p.domain_weights.y <= 0.0 {
            return Err("Domain weights must be positive".into());
        }
        if p.range_weights.x < 0.0 || p.range_weights.y < 0.0 {
            return Err("Range weights must be nonnegative".into());
        }
        if s.lp_norm <= 0.0 {
            return Err("Lp norm must be positive".into());
        }
        if !(0.0..=45.0).contains(&p.degrees_snap) {
            return Err("Snap angle must be between 0 and 45 degrees".into());
        }
        if p.speed_max < 0.0 || p.speed_max < p.speed_min {
            return Err("Maximum speed must be nonnegative and at least minimum speed".into());
        }
        // Negative half-lives disable smoothing in the C++ runtime; negative DPI flips axes.
        Ok(())
    };
    check().map_err(|error| format!("Profile '{}': {error}", p.name))
}

pub(crate) fn device_numbers(
    dpi: u32,
    rate: u32,
    min: f64,
    max: f64,
    inherited: models::time_clamp,
) -> Result<(), String> {
    if dpi > i32::MAX as u32 || rate > i32::MAX as u32 {
        return Err("DPI and polling rate must fit a nonnegative signed 32-bit integer".into());
    }
    finite(&[("Minimum time", min), ("Maximum time", max)])?;
    if min < 0.0 || max < 0.0 {
        return Err("Time clamps cannot be negative; zero inherits the default".into());
    }
    let min = if min == 0.0 { inherited.min } else { min };
    let max = if max == 0.0 { inherited.max } else { max };
    if min <= 0.0 || max < min {
        return Err(
            "Effective minimum time must be positive and no greater than maximum time".into(),
        );
    }
    Ok(())
}

/// Wire strings must fit with their terminating NUL, without splitting UTF-16 pairs.
pub(crate) fn wire_string(value: &str, capacity: usize, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.contains('\0') {
        return Err(format!("{label} must be nonempty and cannot contain NUL"));
    }
    if value.encode_utf16().count() >= capacity {
        return Err(format!(
            "{label} exceeds {} UTF-16 code units",
            capacity - 1
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, AppDeviceConfig, AppDeviceDefaults, AppVec2D};
    fn config() -> AppConfig {
        AppConfig {
            profiles: vec![AppProfile::default()],
            devices: vec![],
            default_device_config: None,
        }
    }
    #[test]
    fn wire_strings_are_bounded_in_utf16_without_truncation() {
        assert!(wire_string(&"a".repeat(255), 256, "Name").is_ok());
        assert!(wire_string(&"a".repeat(256), 256, "Name").is_err());
        assert!(wire_string(&("\u{1f600}".repeat(127) + "a"), 256, "Name").is_ok());
        assert!(wire_string(&"\u{1f600}".repeat(128), 256, "Name").is_err());
        assert!(wire_string("a\0b", 256, "Name").is_err());
        let mut c = config();
        c.profiles[0].name = "\u{1f600}".repeat(128);
        assert!(c.validate().is_err());
        c.profiles[0].name = "Valid".into();
        c.devices.push(AppDeviceConfig {
            id: "d".repeat(199),
            ..Default::default()
        });
        c.validate().unwrap();
        c.devices.push(c.devices[0].clone());
        assert!(c.validate().is_err());
        c.devices.pop();
        c.devices[0].id.push('d');
        assert!(c.validate().is_err());
    }
    #[test]
    fn rejects_nonfinite_numbers_including_inactive_curve_fields() {
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut c = config();
            c.profiles[0].output_dpi = number;
            assert!(c.validate().is_err());
            let mut c = config();
            c.profiles[0].accel_y.gamma = number;
            assert!(c.validate().is_err());
            let mut c = config();
            c.profiles[0].speed_processor_args.scale_smooth_halflife = number;
            assert!(c.validate().is_err());
        }
    }
    #[test]
    fn preserves_cpp_supported_boundary_values() {
        let mut c = config();
        let p = &mut c.profiles[0];
        p.output_dpi = -800.0;
        p.yx_output_dpi_ratio = -1.0;
        p.range_weights.x = 0.0;
        p.speed_min = -1.0;
        p.speed_processor_args.lp_norm = 32.0;
        p.speed_processor_args.input_speed_smooth_halflife = -1.0;
        p.accel_x.mode = "natural".into();
        p.accel_x.limit = 1.0;
        c.validate().unwrap();
    }
    #[test]
    fn validates_caps_and_both_precomputed_axes() {
        let mut c = config();
        c.profiles[0].accel_y.mode = "synchronous".into();
        c.profiles[0].accel_y.smooth = 1.1;
        assert!(c.validate().is_err());
        for mode in ["jump", "classic", "linear", "power"] {
            let mut c = config();
            let a = &mut c.profiles[0].accel_x;
            a.mode = mode.into();
            a.cap_mode = "io".into();
            a.cap.x = 0.0;
            assert!(c.validate().is_err(), "{mode}");
        }
        let mut c = config();
        let a = &mut c.profiles[0].accel_x;
        a.mode = "linear".into();
        a.cap_mode = "in".into();
        a.input_offset = a.cap.x + 1.0;
        assert!(c.validate().is_err());
    }
    #[test]
    fn checks_lut_after_float_packing_even_when_stored_in_another_mode() {
        for (x, y) in [(1e40, 1.0), (1.0, 1e40), (1e-50, 1.0), (1.0 + 1e-9, 1.0)] {
            let mut c = config();
            c.profiles[0].accel_x.lookup_table =
                vec![AppVec2D { x: 1.0, y: -1.0 }, AppVec2D { x, y }];
            assert!(c.validate().is_err(), "{x}, {y}");
        }
        let mut c = config();
        c.profiles[0].accel_x.mode = "lookup".into();
        c.profiles[0].accel_x.lookup_table =
            vec![AppVec2D { x: 1.0, y: -1.0 }, AppVec2D { x: 2.0, y: 1e-50 }];
        c.validate().unwrap();
    }
    #[test]
    fn checks_device_integer_conversion_and_effective_inherited_clamps() {
        let mut c = config();
        c.default_device_config = Some(AppDeviceDefaults {
            clamp_min: 2.0,
            clamp_max: 4.0,
            ..Default::default()
        });
        c.devices.push(AppDeviceConfig {
            id: "mouse".into(),
            clamp_max: 1.0,
            ..Default::default()
        });
        assert!(c.validate().is_err());
        c.devices[0].clamp_max = 0.0;
        c.validate().unwrap();
        c.devices[0].dpi = i32::MAX as u32;
        c.validate().unwrap();
        c.devices[0].dpi += 1;
        assert!(c.validate().is_err());
        c.devices[0].dpi = 0;
        c.devices[0].clamp_min = -1.0;
        assert!(c.validate().is_err());
    }
}
