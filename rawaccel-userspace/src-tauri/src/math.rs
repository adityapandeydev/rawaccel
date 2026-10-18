#![allow(dead_code)]

use crate::models::{
    self, accel_args, accel_mode, accel_union, cap_mode, modifier_flags, modifier_settings,
    profile, vec2d,
};
use std::f64::consts::PI;

// ── Rotation ─────────────────────────────────────────────────────────────

fn direction(degrees: f64) -> vec2d {
    let rad = degrees * PI / 180.0;
    vec2d {
        x: rad.cos(),
        y: rad.sin(),
    }
}

// ── Modifier Flags ───────────────────────────────────────────────────────

fn compute_flags(prof: &profile) -> modifier_flags {
    let clamp_speed = prof.speed_max > 0.0 && prof.speed_min <= prof.speed_max;
    let apply_rotate = prof.degrees_rotation != 0.0;
    let apply_snap = prof.degrees_snap != 0.0;
    let apply_directional_weight =
        prof.speed_processor_args.whole && prof.range_weights.x != prof.range_weights.y;
    let compute_ref_angle = apply_snap || apply_directional_weight;

    modifier_flags {
        apply_rotate,
        compute_ref_angle,
        apply_snap,
        clamp_speed,
        apply_directional_weight,
        apply_dir_mul_x: prof.lr_output_dpi_ratio != 1.0,
        apply_dir_mul_y: prof.ud_output_dpi_ratio != 1.0,
    }
}

// ── Utility Functions ────────────────────────────────────────────────────

// ── Curve Structs ────────────────────────────────────────────────────────
// Each mirrors the C++ constructor logic that pre-computes constants
// written into the accel_union for the driver to evaluate at runtime.

// ── Classic ──────────────────────────────────────────────────────────────

#[repr(C)]
struct ClassicLegacy {
    accel_raised: f64,
    cap: f64,
    sign: f64,
}

impl ClassicLegacy {
    fn new(args: &accel_args) -> Self {
        let mut sign = 1.0;
        let mut cap = f64::MAX;
        let accel_raised;

        match args.cap_mode {
            cap_mode::io => {
                cap = args.cap.y - 1.0;
                if cap < 0.0 {
                    cap = -cap;
                    sign = -sign;
                }
                let a = classic_base_accel(args.cap.x, cap, args);
                accel_raised = a.powf(args.exponent_classic - 1.0);
            }
            cap_mode::in_ => {
                accel_raised = args.acceleration.powf(args.exponent_classic - 1.0);
                if args.cap.x > 0.0 {
                    cap = classic_base_fn(args.cap.x, accel_raised, args);
                }
            }
            _ => {
                accel_raised = args.acceleration.powf(args.exponent_classic - 1.0);
                if args.cap.y > 0.0 {
                    cap = args.cap.y - 1.0;
                    if cap < 0.0 {
                        cap = -cap;
                        sign = -sign;
                    }
                }
            }
        }

        Self {
            accel_raised,
            cap,
            sign,
        }
    }

    pub fn evaluate(&self, x: f64, args: &accel_args) -> f64 {
        if x <= args.input_offset {
            return 1.0;
        }
        self.sign * classic_base_fn(x, self.accel_raised, args).min(self.cap) + 1.0
    }
}

#[repr(C)]
struct ClassicGain {
    accel_raised: f64,
    cap: vec2d,
    constant: f64,
    sign: f64,
}

impl ClassicGain {
    fn new(args: &accel_args) -> Self {
        let mut sign = 1.0;
        let mut cap = vec2d {
            x: f64::MAX,
            y: f64::MAX,
        };
        let mut constant = 0.0;
        let accel_raised;

        match args.cap_mode {
            cap_mode::io => {
                cap.x = args.cap.x;
                cap.y = args.cap.y - 1.0;
                if cap.y < 0.0 {
                    cap.y = -cap.y;
                    sign = -sign;
                }
                let a = classic_gain_accel(cap.x, cap.y, args.exponent_classic, args.input_offset);
                accel_raised = a.powf(args.exponent_classic - 1.0);
                constant = (classic_base_fn(cap.x, accel_raised, args) - cap.y) * cap.x;
            }
            cap_mode::in_ => {
                accel_raised = args.acceleration.powf(args.exponent_classic - 1.0);
                if args.cap.x > 0.0 {
                    cap.x = args.cap.x;
                    cap.y = classic_gain(
                        cap.x,
                        args.acceleration,
                        args.exponent_classic,
                        args.input_offset,
                    );
                    constant = (classic_base_fn(cap.x, accel_raised, args) - cap.y) * cap.x;
                }
            }
            _ => {
                accel_raised = args.acceleration.powf(args.exponent_classic - 1.0);
                if args.cap.y > 0.0 {
                    cap.y = args.cap.y - 1.0;
                    if cap.y == 0.0 {
                        cap.x = 0.0;
                    } else {
                        if cap.y < 0.0 {
                            cap.y = -cap.y;
                            sign = -sign;
                        }
                        cap.x = classic_gain_inverse(
                            cap.y,
                            args.acceleration,
                            args.exponent_classic,
                            args.input_offset,
                        );
                        constant = (classic_base_fn(cap.x, accel_raised, args) - cap.y) * cap.x;
                    }
                }
            }
        }

        Self {
            accel_raised,
            cap,
            constant,
            sign,
        }
    }

    pub fn evaluate(&self, x: f64, args: &accel_args) -> f64 {
        if x <= args.input_offset {
            return 1.0;
        }
        let output = if x < self.cap.x {
            classic_base_fn(x, self.accel_raised, args)
        } else {
            self.constant / x + self.cap.y
        };
        self.sign * output + 1.0
    }
}

fn classic_base_fn(x: f64, accel_raised: f64, args: &accel_args) -> f64 {
    accel_raised * (x - args.input_offset).powf(args.exponent_classic) / x
}

fn classic_base_accel(x: f64, y: f64, args: &accel_args) -> f64 {
    let power = args.exponent_classic;
    (x * y * (x - args.input_offset).powf(-power)).powf(1.0 / (power - 1.0))
}

fn classic_gain(x: f64, accel: f64, power: f64, offset: f64) -> f64 {
    power * (accel * (x - offset)).powf(power - 1.0)
}

fn classic_gain_inverse(y: f64, accel: f64, power: f64, offset: f64) -> f64 {
    (accel * offset + (y / power).powf(1.0 / (power - 1.0))) / accel
}

fn classic_gain_accel(x: f64, y: f64, power: f64, offset: f64) -> f64 {
    -(y / power).powf(1.0 / (power - 1.0)) / (offset - x)
}

// ── Jump ─────────────────────────────────────────────────────────────────

const JUMP_SMOOTH_SCALE: f64 = 2.0 * PI;

fn jump_smooth_rate(args: &accel_args, step_x: f64) -> f64 {
    let rate_inverse = args.smooth * step_x;
    if rate_inverse < 1.0 {
        0.0
    } else {
        JUMP_SMOOTH_SCALE / rate_inverse
    }
}

fn jump_smooth_antideriv(x: f64, step: vec2d, smooth_rate: f64) -> f64 {
    let decay = (smooth_rate * (step.x - x)).exp();
    step.y * (x + (1.0 + decay).ln() / smooth_rate)
}

#[repr(C)]
struct JumpLegacy {
    step: vec2d,
    smooth_rate: f64,
}

impl JumpLegacy {
    fn new(args: &accel_args) -> Self {
        let step = vec2d {
            x: args.cap.x,
            y: args.cap.y - 1.0,
        };
        let smooth_rate = jump_smooth_rate(args, step.x);
        Self { step, smooth_rate }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if self.smooth_rate != 0.0 {
            let decay = (self.smooth_rate * (self.step.x - x)).exp();
            self.step.y / (1.0 + decay) + 1.0
        } else if x < self.step.x {
            1.0
        } else {
            1.0 + self.step.y
        }
    }
}

#[repr(C)]
struct JumpGain {
    step: vec2d,
    smooth_rate: f64,
    c: f64,
}

impl JumpGain {
    fn new(args: &accel_args) -> Self {
        let step = vec2d {
            x: args.cap.x,
            y: args.cap.y - 1.0,
        };
        let smooth_rate = jump_smooth_rate(args, step.x);
        let c = -jump_smooth_antideriv(0.0, step, smooth_rate);
        Self {
            step,
            smooth_rate,
            c,
        }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 1.0;
        }
        if self.smooth_rate != 0.0 {
            let antideriv = jump_smooth_antideriv(x, self.step, self.smooth_rate);
            1.0 + (antideriv + self.c) / x
        } else if x < self.step.x {
            1.0
        } else {
            1.0 + self.step.y * (x - self.step.x) / x
        }
    }
}

// ── Natural ──────────────────────────────────────────────────────────────

#[repr(C)]
struct NaturalLegacy {
    offset: f64,
    accel: f64,
    limit: f64,
}

impl NaturalLegacy {
    fn new(args: &accel_args) -> Self {
        let limit = args.limit - 1.0;
        let accel = args.decay_rate / limit.abs();
        Self {
            offset: args.input_offset,
            accel,
            limit,
        }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= self.offset {
            return 1.0;
        }
        let offset_x = self.offset - x;
        let decay = (self.accel * offset_x).exp();
        self.limit * (1.0 - (self.offset - decay * offset_x) / x) + 1.0
    }
}

#[repr(C)]
struct NaturalGain {
    offset: f64,
    accel: f64,
    limit: f64,
    constant: f64,
}

impl NaturalGain {
    fn new(args: &accel_args) -> Self {
        let limit = args.limit - 1.0;
        let accel = args.decay_rate / limit.abs();
        let constant = -limit / accel;
        Self {
            offset: args.input_offset,
            accel,
            limit,
            constant,
        }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= self.offset {
            return 1.0;
        }
        let offset_x = self.offset - x;
        let decay = (self.accel * offset_x).exp();
        let output = self.limit * (decay / self.accel - offset_x) + self.constant;
        output / x + 1.0
    }
}

// ── Power ────────────────────────────────────────────────────────────────

fn power_gain_fn(input: f64, power: f64, scale: f64) -> f64 {
    (power + 1.0) * (input * scale).powf(power)
}

fn power_gain_inverse(gain: f64, power: f64, scale: f64) -> f64 {
    (gain / (power + 1.0)).powf(1.0 / power) / scale
}

fn power_scale_from_gain_point(input: f64, gain: f64, power: f64) -> f64 {
    (gain / (power + 1.0)).powf(1.0 / power) / input
}

fn power_scale_from_output_point(input: f64, output: f64, power: f64, c: f64) -> f64 {
    (output - c / input).powf(1.0 / power) / input
}

fn power_base_fn(x: f64, args: &accel_args, offset: vec2d, scale: f64, constant: f64) -> f64 {
    if x <= offset.x {
        offset.y
    } else {
        (scale * x).powf(args.exponent_power) + constant / x
    }
}

fn power_integration_constant(input: f64, gain: f64, output: f64) -> f64 {
    (output - gain) * input
}

#[repr(C)]
struct PowerLegacy {
    offset: vec2d,
    scale: f64,
    constant: f64,
    cap: f64,
}

impl PowerLegacy {
    fn new(args: &accel_args) -> Self {
        let n = args.exponent_power;
        let offset;
        let scale;
        let constant;

        // Mirrors C++ power_base constructor: if/else-if/else on cap_mode and gain
        if args.cap_mode != cap_mode::io {
            scale = args.scale;
        } else if args.gain {
            scale = power_scale_from_gain_point(args.cap.x, args.cap.y, n);
        } else {
            // Legacy + io cap mode: offset ignored due to circular dependency
            let s = power_scale_from_output_point(args.cap.x, args.cap.y, n, 0.0);
            return Self {
                offset: vec2d::default(),
                scale: s,
                constant: 0.0,
                cap: args.cap.y,
            };
        }

        offset = vec2d {
            x: power_gain_inverse(args.output_offset, n, scale),
            y: args.output_offset,
        };
        constant = offset.x * offset.y * n / (n + 1.0);

        let mut cap = f64::MAX;
        match args.cap_mode {
            cap_mode::io => {
                cap = args.cap.y;
            }
            cap_mode::in_ => {
                if args.cap.x > 0.0 {
                    cap = power_base_fn(args.cap.x, args, offset, scale, constant);
                }
            }
            _ => {
                if args.cap.y > 0.0 {
                    cap = args.cap.y;
                }
            }
        }

        Self {
            offset,
            scale,
            constant,
            cap,
        }
    }

    pub fn evaluate(&self, x: f64, args: &accel_args) -> f64 {
        let base = power_base_fn(x, args, self.offset, self.scale, self.constant);
        base.min(self.cap)
    }
}

#[repr(C)]
struct PowerGain {
    offset: vec2d,
    scale: f64,
    constant: f64,
    cap: vec2d,
    constant_b: f64,
}

impl PowerGain {
    fn new(args: &accel_args) -> Self {
        let n = args.exponent_power;
        let mut offset = vec2d::default();
        let scale;
        let constant;

        if args.cap_mode != cap_mode::io {
            scale = args.scale;
        } else if args.gain {
            scale = power_scale_from_gain_point(args.cap.x, args.cap.y, n);
        } else {
            let s = power_scale_from_output_point(args.cap.x, args.cap.y, n, 0.0);
            return Self {
                offset: vec2d::default(),
                scale: s,
                constant: 0.0,
                cap: vec2d {
                    x: f64::MAX,
                    y: f64::MAX,
                },
                constant_b: 0.0,
            };
        }

        offset.x = power_gain_inverse(args.output_offset, n, scale);
        offset.y = args.output_offset;
        constant = offset.x * offset.y * n / (n + 1.0);

        let mut cap = vec2d {
            x: f64::MAX,
            y: f64::MAX,
        };
        let constant_b;

        match args.cap_mode {
            cap_mode::io => {
                cap = args.cap;
                constant_b = power_integration_constant(
                    cap.x,
                    cap.y,
                    power_base_fn(cap.x, args, offset, scale, constant),
                );
            }
            cap_mode::in_ => {
                if args.cap.x > 0.0 {
                    if args.cap.x <= offset.x {
                        return Self {
                            offset,
                            scale,
                            constant,
                            cap: vec2d {
                                x: 0.0,
                                y: offset.y,
                            },
                            constant_b: 0.0,
                        };
                    }
                    cap.x = args.cap.x;
                    cap.y = power_gain_fn(args.cap.x, n, scale);
                }
                constant_b = power_integration_constant(
                    cap.x,
                    cap.y,
                    power_base_fn(cap.x, args, offset, scale, constant),
                );
            }
            _ => {
                if args.cap.y > 0.0 {
                    cap.x = power_gain_inverse(args.cap.y, n, scale);
                    cap.y = args.cap.y;
                }
                constant_b = power_integration_constant(
                    cap.x,
                    cap.y,
                    power_base_fn(cap.x, args, offset, scale, constant),
                );
            }
        }

        Self {
            offset,
            scale,
            constant,
            cap,
            constant_b,
        }
    }

    pub fn evaluate(&self, x: f64, args: &accel_args) -> f64 {
        if x < self.cap.x {
            power_base_fn(x, args, self.offset, self.scale, self.constant)
        } else {
            self.cap.y + self.constant_b / x
        }
    }
}

// ── Tiered ───────────────────────────────────────────────────────────────

#[repr(C)]
struct TieredLinearLegacy {
    m1: f64,
    x1: f64,
    m2: f64,
    x1_end: f64,
    inv_trans1: f64,
    m3: f64,
    x2: f64,
    x2_end: f64,
    inv_trans2: f64,
}

impl TieredLinearLegacy {
    fn new(args: &accel_args) -> Self {
        let inv_trans1 = if args.tiered_transition1 > 0.0 {
            1.0 / args.tiered_transition1
        } else {
            0.0
        };
        let inv_trans2 = if args.tiered_transition2 > 0.0 {
            1.0 / args.tiered_transition2
        } else {
            0.0
        };
        Self {
            m1: args.tiered_multiplier1,
            x1: args.tiered_input_offset1,
            m2: args.tiered_multiplier2,
            x1_end: args.tiered_input_offset1 + args.tiered_transition1,
            inv_trans1,
            m3: args.tiered_multiplier3,
            x2: args.tiered_input_offset2,
            x2_end: args.tiered_input_offset2 + args.tiered_transition2,
            inv_trans2,
        }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= self.x1 {
            return self.m1;
        }
        if x < self.x1_end {
            if self.inv_trans1 <= 0.0 {
                return self.m2;
            }
            let t = (x - self.x1) * self.inv_trans1;
            return self.m1 + t * (self.m2 - self.m1);
        }
        if x <= self.x2 {
            return self.m2;
        }
        if x < self.x2_end {
            if self.inv_trans2 <= 0.0 {
                return self.m3;
            }
            let t = (x - self.x2) * self.inv_trans2;
            return self.m2 + t * (self.m3 - self.m2);
        }
        self.m3
    }
}

#[repr(C)]
struct TieredNaturalLegacy {
    m1: f64,
    x1: f64,
    l1: f64,
    a1: f64,
    x2: f64,
    v2: f64,
    m2_prime: f64,
    l2: f64,
    a2: f64,
}

impl TieredNaturalLegacy {
    fn new(args: &accel_args) -> Self {
        let m1 = args.tiered_multiplier1;
        let x1 = args.tiered_input_offset1;
        let v1 = m1 * x1;

        let l1 = args.tiered_multiplier2 - m1;
        let a1 = if l1 != 0.0 {
            args.tiered_decay_rate1 / l1.abs()
        } else {
            0.0
        };

        let x2 = args.tiered_input_offset2;
        let dx1 = x2 - x1;

        let decay1 = if l1 != 0.0 { (-a1 * dx1).exp() } else { 1.0 };
        let v2 = if l1 != 0.0 {
            v1 + m1 * dx1 + l1 * dx1 * (1.0 - decay1)
        } else {
            v1 + m1 * dx1
        };
        let m2_prime = if l1 != 0.0 {
            m1 + l1 * (1.0 - decay1 + dx1 * a1 * decay1)
        } else {
            m1
        };

        let l2 = args.tiered_multiplier3 - m2_prime;
        let a2 = if l2 != 0.0 {
            args.tiered_decay_rate2 / l2.abs()
        } else {
            0.0
        };

        Self {
            m1,
            x1,
            l1,
            a1,
            x2,
            v2,
            m2_prime,
            l2,
            a2,
        }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= self.x1 {
            return self.m1;
        }
        if x < self.x2 {
            if self.l1 == 0.0 {
                return self.m1;
            }
            let dx = x - self.x1;
            let decay = (-self.a1 * dx).exp();
            let v = self.m1 * self.x1 + self.m1 * dx + self.l1 * dx * (1.0 - decay);
            return v / x;
        }
        if self.l2 == 0.0 {
            let dx2 = x - self.x2;
            return (self.v2 + self.m2_prime * dx2) / x;
        }
        let dx2 = x - self.x2;
        let decay2 = (-self.a2 * dx2).exp();
        let v = self.v2 + self.m2_prime * dx2 + self.l2 * dx2 * (1.0 - decay2);
        v / x
    }
}

#[repr(C)]
struct TieredNaturalGain {
    m1: f64,
    x1: f64,
    l1: f64,
    a1: f64,
    x2: f64,
    v2: f64,
    m2_prime: f64,
    l2: f64,
    a2: f64,
}

impl TieredNaturalGain {
    fn new(args: &accel_args) -> Self {
        let m1 = args.tiered_multiplier1;
        let x1 = args.tiered_input_offset1;
        let v1 = m1 * x1;

        let l1 = args.tiered_multiplier2 - m1;
        let a1 = if l1 != 0.0 {
            args.tiered_decay_rate1 / l1.abs()
        } else {
            0.0
        };

        let x2 = args.tiered_input_offset2;
        let dx1 = x2 - x1;

        let decay1 = if l1 != 0.0 { (-a1 * dx1).exp() } else { 1.0 };
        let v2 = if l1 != 0.0 {
            v1 + m1 * dx1 + l1 * (dx1 - (1.0 - decay1) / a1)
        } else {
            v1 + m1 * dx1
        };
        let m2_prime = if l1 != 0.0 {
            m1 + l1 * (1.0 - decay1)
        } else {
            m1
        };

        let l2 = args.tiered_multiplier3 - m2_prime;
        let a2 = if l2 != 0.0 {
            args.tiered_decay_rate2 / l2.abs()
        } else {
            0.0
        };

        Self {
            m1,
            x1,
            l1,
            a1,
            x2,
            v2,
            m2_prime,
            l2,
            a2,
        }
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= self.x1 {
            return self.m1;
        }
        if x < self.x2 {
            if self.l1 == 0.0 {
                return self.m1;
            }
            let dx = x - self.x1;
            let decay = (-self.a1 * dx).exp();
            let v = self.m1 * self.x1 + self.m1 * dx + self.l1 * (dx - (1.0 - decay) / self.a1);
            return v / x;
        }
        if self.l2 == 0.0 {
            let dx2 = x - self.x2;
            return (self.v2 + self.m2_prime * dx2) / x;
        }
        let dx2 = x - self.x2;
        let decay2 = (-self.a2 * dx2).exp();
        let v = self.v2 + self.m2_prime * dx2 + self.l2 * (dx2 - (1.0 - decay2) / self.a2);
        v / x
    }
}

// ── Lookup ───────────────────────────────────────────────────────────────
// Lookup stores size and velocity flag. Points live in accel_args.data[].

#[repr(C)]
struct Lookup {
    size: i32,
    velocity: bool,
}

impl Lookup {
    fn new(args: &accel_args) -> Self {
        Self {
            size: args.length / 2,
            velocity: args.gain,
        }
    }

    pub fn evaluate(&self, x: f64, args: &accel_args) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if self.size < 2 {
            return 1.0;
        }
        let data = &args.data;
        let mut lo = 0usize;
        let mut hi = (self.size - 2) as isize;

        while (lo as isize) <= hi {
            let mid = ((lo as isize + hi) / 2) as usize;
            let px = data[mid * 2] as f64;
            let py = data[mid * 2 + 1] as f64;

            if x < px {
                hi = mid as isize - 1;
            } else if x > px {
                lo = mid + 1;
            } else {
                let mut y = py;
                if self.velocity {
                    y /= x;
                }
                return y;
            }
        }

        if lo > 0 && lo < self.size as usize {
            let ax = data[(lo - 1) * 2] as f64;
            let ay = data[(lo - 1) * 2 + 1] as f64;
            let by = data[lo * 2 + 1] as f64;
            // C++ subtracts the two packed float X values before promotion.
            let dx = (args.data[lo * 2] - args.data[(lo - 1) * 2]) as f64;
            let t = (x - ax) / dx;
            let interpolated = ay + t * (by - ay);
            let mut y = if (t > 1.0) == (ay < by) {
                if by < interpolated { interpolated } else { by }
            } else if interpolated < by { interpolated } else { by };
            if self.velocity {
                y /= x;
            }
            return y;
        }

        let y0 = data[1] as f64;
        let x0 = data[0] as f64;
        if self.velocity && x0 > 0.0 {
            y0 / x0
        } else {
            y0
        }
    }
}

// ── Synchronous (activation_framework) ───────────────────────────────────
// Legacy variant stores pre-computed log/gamma constants.
// Gain variant fills accel_args.data[] via numerical integration.

#[repr(C)]
struct SynchronousLegacy {
    log_motivity: f64,
    gamma_const: f64,
    log_syncspeed: f64,
    syncspeed: f64,
    sharpness: f64,
    sharpness_recip: f64,
    use_linear_clamp: bool,
    minimum_sens: f64,
    maximum_sens: f64,
}

impl SynchronousLegacy {
    fn new(args: &accel_args) -> Self {
        let log_motivity = args.motivity.ln();
        let gamma_const = args.gamma / log_motivity;
        let sharpness = if args.smooth == 0.0 {
            16.0
        } else {
            0.5 / args.smooth
        };

        Self {
            log_motivity,
            gamma_const,
            log_syncspeed: args.sync_speed.ln(),
            syncspeed: args.sync_speed,
            sharpness,
            sharpness_recip: 1.0 / sharpness,
            use_linear_clamp: sharpness >= 16.0,
            minimum_sens: 1.0 / args.motivity,
            maximum_sens: args.motivity,
        }
    }

    fn evaluate(&self, x: f64) -> f64 {
        if self.use_linear_clamp {
            let log_space = self.gamma_const * (x.ln() - self.log_syncspeed);
            if log_space < -1.0 {
                return self.minimum_sens;
            }
            if log_space > 1.0 {
                return self.maximum_sens;
            }
            return (log_space * self.log_motivity).exp();
        }

        if x == self.syncspeed {
            return 1.0;
        }

        let log_x = x.ln();
        let log_diff = log_x - self.log_syncspeed;

        if log_diff > 0.0 {
            let log_space = self.gamma_const * log_diff;
            let exponent = log_space
                .powf(self.sharpness)
                .tanh()
                .powf(self.sharpness_recip);
            (exponent * self.log_motivity).exp()
        } else {
            let log_space = -self.gamma_const * log_diff;
            let exponent = -(log_space
                .powf(self.sharpness)
                .tanh()
                .powf(self.sharpness_recip));
            (exponent * self.log_motivity).exp()
        }
    }
}

#[repr(C)]
struct SynchronousGain {
    velocity: bool,
    range_start: i32,
    range_stop: i32,
    range_num: i32,
    x_start: f64,
}

impl SynchronousGain {
    fn new(args: &mut accel_args) -> Self {
        let range_start = -3i32;
        let range_stop = 9i32;
        let range_num = 8i32;
        let velocity = true;
        let x_start = 2.0f64.powi(range_start);

        let sig = SynchronousLegacy::new(args);

        let mut sum = 0.0f64;
        let mut a = 0.0f64;
        let mut idx = 0usize;

        for e in 0..(range_stop - range_start) {
            let exp_scale = 2.0f64.powi(e + range_start) / range_num as f64;
            for i in 0..range_num {
                let x = (i + range_num) as f64 * exp_scale;

                let partitions = 2;
                let interval = (x - a) / partitions as f64;
                for p in 1..=partitions {
                    sum += sig.evaluate(a + p as f64 * interval) * interval;
                }
                a = x;

                let mut y = sum;
                if !velocity {
                    y /= x;
                }

                if idx < models::LUT_RAW_DATA_CAPACITY {
                    args.data[idx] = y as f32;
                    idx += 1;
                }
            }
        }

        // Final point at 2^stop
        let x_final = 2.0f64.powi(range_stop);
        let partitions = 2;
        let interval = (x_final - a) / partitions as f64;
        for p in 1..=partitions {
            sum += sig.evaluate(a + p as f64 * interval) * interval;
        }
        let mut y = sum;
        if !velocity {
            y /= x_final;
        }
        if idx < models::LUT_RAW_DATA_CAPACITY {
            args.data[idx] = y as f32;
        }

        Self {
            velocity,
            range_start,
            range_stop,
            range_num,
            x_start,
        }
    }

    fn evaluate(&self, x: f64, args: &accel_args) -> f64 {
        if x >= self.x_start {
            let e = (x.log2().floor() as i32).min(self.range_stop - 1);
            let idx_f = self.range_num as f64
                * ((e - self.range_start) as f64 + x * 2.0f64.powi(-e) - 1.0);
            let size = (self.range_stop - self.range_start) * self.range_num + 1;
            let idx = (idx_f as usize).min((size - 2) as usize);
            let a = args.data[idx] as f64;
            let b = args.data[idx + 1] as f64;
            let y = a + (idx_f - idx as f64) * (b - a);
            return if self.velocity { y / x } else { y };
        }
        let y = args.data[0] as f64;
        if self.velocity { y / self.x_start } else { y }
    }
}

// ── Noaccel ──────────────────────────────────────────────────────────────

#[repr(C)]
struct Noaccel;


// Explicit curve encoders retain C layout and leave padding initialized.
crate::wire::wire_struct!(ClassicLegacy, accel_raised, cap, sign);
crate::wire::wire_struct!(ClassicGain, accel_raised, cap, constant, sign);
crate::wire::wire_struct!(JumpLegacy, step, smooth_rate);
crate::wire::wire_struct!(JumpGain, step, smooth_rate, c);
crate::wire::wire_struct!(NaturalLegacy, offset, accel, limit);
crate::wire::wire_struct!(NaturalGain, offset, accel, limit, constant);
crate::wire::wire_struct!(PowerLegacy, offset, scale, constant, cap);
crate::wire::wire_struct!(PowerGain, offset, scale, constant, cap, constant_b);
crate::wire::wire_struct!(
    TieredLinearLegacy,
    m1,
    x1,
    m2,
    x1_end,
    inv_trans1,
    m3,
    x2,
    x2_end,
    inv_trans2
);
crate::wire::wire_struct!(
    TieredNaturalLegacy,
    m1,
    x1,
    l1,
    a1,
    x2,
    v2,
    m2_prime,
    l2,
    a2
);
crate::wire::wire_struct!(TieredNaturalGain, m1, x1, l1, a1, x2, v2, m2_prime, l2, a2);
crate::wire::wire_struct!(Lookup, size, velocity);
crate::wire::wire_struct!(
    SynchronousLegacy,
    log_motivity,
    gamma_const,
    log_syncspeed,
    syncspeed,
    sharpness,
    sharpness_recip,
    use_linear_clamp,
    minimum_sens,
    maximum_sens
);
crate::wire::wire_struct!(
    SynchronousGain,
    velocity,
    range_start,
    range_stop,
    range_num,
    x_start
);

// ── Union Initialization ─────────────────────────────────────────────────

fn init_accel_union(union: &mut accel_union, args: &mut accel_args) {
    union.memory.fill(0);
    match args.mode {
        accel_mode::classic => {
            if args.gain {
                let curve = ClassicGain::new(args);
                union.write(&curve);
            } else {
                let curve = ClassicLegacy::new(args);
                union.write(&curve);
            }
        }
        accel_mode::jump => {
            if args.gain {
                let curve = JumpGain::new(args);
                union.write(&curve);
            } else {
                let curve = JumpLegacy::new(args);
                union.write(&curve);
            }
        }
        accel_mode::natural => {
            if args.gain {
                let curve = NaturalGain::new(args);
                union.write(&curve);
            } else {
                let curve = NaturalLegacy::new(args);
                union.write(&curve);
            }
        }
        accel_mode::power => {
            if args.gain {
                let curve = PowerGain::new(args);
                union.write(&curve);
            } else {
                let curve = PowerLegacy::new(args);
                union.write(&curve);
            }
        }
        accel_mode::tiered => {
            if args.t_type == models::tiered_type::linear {
                let curve = TieredLinearLegacy::new(args);
                union.write(&curve);
            } else if args.gain {
                let curve = TieredNaturalGain::new(args);
                union.write(&curve);
            } else {
                let curve = TieredNaturalLegacy::new(args);
                union.write(&curve);
            }
        }
        accel_mode::synchronous => {
            if args.gain {
                let curve = SynchronousGain::new(args);
                union.write(&curve);
            } else {
                let curve = SynchronousLegacy::new(args);
                union.write(&curve);
            }
        }
        accel_mode::lookup => {
            let curve = Lookup::new(args);
            union.write(&curve);
        }
        accel_mode::noaccel => {
            // No-op.
        }
    }
}

// ── Public Entry Point ───────────────────────────────────────────────────

pub fn init_data(settings: &mut modifier_settings) {
    init_accel_union(&mut settings.data.accel_x, &mut settings.prof.accel_x);
    init_accel_union(&mut settings.data.accel_y, &mut settings.prof.accel_y);
    settings.data.rot_direction = direction(settings.prof.degrees_rotation);
    settings.data.flags = compute_flags(&settings.prof);
}

/// Evaluates any acceleration curve at an arbitrary input speed `x` (counts/ms).
/// Used for mathematical verification and userspace simulations.
pub fn evaluate_curve(args: &accel_args, x: f64) -> f64 {
    match args.mode {
        accel_mode::classic => {
            if args.gain {
                ClassicGain::new(args).evaluate(x, args)
            } else {
                ClassicLegacy::new(args).evaluate(x, args)
            }
        }
        accel_mode::jump => {
            if args.gain {
                JumpGain::new(args).evaluate(x)
            } else {
                JumpLegacy::new(args).evaluate(x)
            }
        }
        accel_mode::natural => {
            if args.gain {
                NaturalGain::new(args).evaluate(x)
            } else {
                NaturalLegacy::new(args).evaluate(x)
            }
        }
        accel_mode::power => {
            if args.gain {
                PowerGain::new(args).evaluate(x, args)
            } else {
                PowerLegacy::new(args).evaluate(x, args)
            }
        }
        accel_mode::tiered => {
            if args.t_type == models::tiered_type::linear {
                TieredLinearLegacy::new(args).evaluate(x)
            } else if args.gain {
                TieredNaturalGain::new(args).evaluate(x)
            } else {
                TieredNaturalLegacy::new(args).evaluate(x)
            }
        }
        accel_mode::synchronous => {
            if args.gain {
                let mut prepared = *args;
                let curve = SynchronousGain::new(&mut prepared);
                curve.evaluate(x, &prepared)
            } else {
                SynchronousLegacy::new(args).evaluate(x)
            }
        }
        accel_mode::lookup => {
            Lookup::new(args).evaluate(x, args)
        }
        accel_mode::noaccel => 1.0,
    }
}

// ── Mathematical Reference & Parity Test Suite ───────────────────────────


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{accel_args, accel_mode, cap_mode, tiered_type, ACCEL_UNION_SIZE};
    use std::mem::size_of;

    #[test]
    fn lookup_uses_packed_float_segment_width_and_zero_endpoint() {
        let mut args = accel_args::default();
        args.mode = accel_mode::lookup;
        args.gain = false;
        args.length = 4;
        args.data[..4].copy_from_slice(&[0.1, 1.0, 10.0, 1.5]);
        assert_eq!(evaluate_curve(&args, 0.0), 0.0);
        assert_eq!(evaluate_curve(&args, -1.0), 0.0);
        // Generated by the C++ operator, including the f32 denominator.
        assert!((evaluate_curve(&args, 5.0) - 1.2474747569352596).abs() < 1e-12);
    }

    #[test]
    fn synchronous_gain_uses_velocity_table_and_evaluates_its_integral() {
        let mut args = accel_args::default();
        args.mode = accel_mode::synchronous;
        args.gain = true;
        args.motivity = 2.0;
        args.gamma = 1.0;
        args.sync_speed = 10.0;
        args.smooth = 0.0;
        init_accel_union(&mut accel_union::default(), &mut args);
        // C++ integrates the constant minimum gain (0.5) from 0 to 0.125.
        assert_eq!(args.data[0], 0.0625);
        assert_eq!(evaluate_curve(&args, 0.0), 0.5);
        assert_eq!(evaluate_curve(&args, 0.125), 0.5);
        // Gain integrates the curve; Legacy is exactly 1 at sync_speed.
        assert!(evaluate_curve(&args, 10.0) < 0.9);
        assert!(evaluate_curve(&args, 1024.0).is_finite());
    }

    // ── 1. Binary Memory Layout & ABI Size Tests ─────────────────────────

    #[test]
    fn test_struct_sizes_fit_in_accel_union() {
        assert!(size_of::<ClassicLegacy>() <= ACCEL_UNION_SIZE, "ClassicLegacy exceeds 72 bytes");
        assert!(size_of::<ClassicGain>() <= ACCEL_UNION_SIZE, "ClassicGain exceeds 72 bytes");
        assert!(size_of::<JumpLegacy>() <= ACCEL_UNION_SIZE, "JumpLegacy exceeds 72 bytes");
        assert!(size_of::<JumpGain>() <= ACCEL_UNION_SIZE, "JumpGain exceeds 72 bytes");
        assert!(size_of::<NaturalLegacy>() <= ACCEL_UNION_SIZE, "NaturalLegacy exceeds 72 bytes");
        assert!(size_of::<NaturalGain>() <= ACCEL_UNION_SIZE, "NaturalGain exceeds 72 bytes");
        assert!(size_of::<PowerLegacy>() <= ACCEL_UNION_SIZE, "PowerLegacy exceeds 72 bytes");
        assert!(size_of::<PowerGain>() <= ACCEL_UNION_SIZE, "PowerGain exceeds 72 bytes");
        assert_eq!(size_of::<TieredLinearLegacy>(), 72, "TieredLinearLegacy must be exactly 72 bytes");
        assert_eq!(size_of::<TieredNaturalLegacy>(), 72, "TieredNaturalLegacy must be exactly 72 bytes");
        assert_eq!(size_of::<TieredNaturalGain>(), 72, "TieredNaturalGain must be exactly 72 bytes");
        assert_eq!(size_of::<SynchronousLegacy>(), 72, "SynchronousLegacy must be exactly 72 bytes");
        assert!(size_of::<SynchronousGain>() <= ACCEL_UNION_SIZE, "SynchronousGain exceeds 72 bytes");
        assert!(size_of::<Lookup>() <= ACCEL_UNION_SIZE, "Lookup exceeds 72 bytes");
        assert_eq!(size_of::<accel_union>(), 72, "accel_union buffer must be 72 bytes");
    }

    // ── 2. Classic Mode Tests ────────────────────────────────────────────

    #[test]
    fn test_classic_legacy_offset_and_scaling() {
        let args = accel_args {
            mode: accel_mode::classic,
            gain: false,
            acceleration: 0.01,
            exponent_classic: 2.0,
            input_offset: 10.0,
            cap: vec2d { x: 0.0, y: 0.0 }, // No cap
            cap_mode: cap_mode::out,
            ..Default::default()
        };

        // Below offset: sensitivity is strictly 1.0
        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        assert_eq!(evaluate_curve(&args, 5.0), 1.0);
        assert_eq!(evaluate_curve(&args, 10.0), 1.0);

        // Above offset: f(x) = (a * (x - offset)^exp) / x + 1
        // At x = 20: 0.01 * (10)^2 / 20 + 1 = 0.01 * 100 / 20 + 1 = 0.05 + 1 = 1.05
        let sens_20 = evaluate_curve(&args, 20.0);
        assert!((sens_20 - 1.05).abs() < 1e-6);

        // At x = 30: 0.01 * (20)^2 / 30 + 1 = 4 / 30 + 1 = 1.1333333
        let sens_30 = evaluate_curve(&args, 30.0);
        assert!((sens_30 - (1.0 + 4.0 / 30.0)).abs() < 1e-6);
        assert!(sens_30 > sens_20, "Classic acceleration must be strictly monotonically increasing");
    }

    #[test]
    fn test_classic_legacy_caps_in_out_io() {
        // Output Cap (cap_mode::out)
        let out_cap_args = accel_args {
            mode: accel_mode::classic,
            gain: false,
            acceleration: 0.05,
            exponent_classic: 2.0,
            input_offset: 0.0,
            cap: vec2d { x: 0.0, y: 1.5 }, // output cap at 1.5
            cap_mode: cap_mode::out,
            ..Default::default()
        };
        assert!(evaluate_curve(&out_cap_args, 100.0) <= 1.5 + 1e-6);
        assert!((evaluate_curve(&out_cap_args, 500.0) - 1.5).abs() < 1e-6);

        // Input Cap (cap_mode::in_)
        let in_cap_args = accel_args {
            mode: accel_mode::classic,
            gain: false,
            acceleration: 0.01,
            exponent_classic: 2.0,
            input_offset: 0.0,
            cap: vec2d { x: 20.0, y: 0.0 }, // input cap at speed 20.0
            cap_mode: cap_mode::in_,
            ..Default::default()
        };
        let val_at_cap = evaluate_curve(&in_cap_args, 20.0);
        let val_above_cap = evaluate_curve(&in_cap_args, 100.0);
        assert!((val_at_cap - val_above_cap).abs() < 1e-6, "Sensitivity must saturate beyond input cap threshold");

        // Input-Output Cap (cap_mode::io)
        let io_cap_args = accel_args {
            mode: accel_mode::classic,
            gain: false,
            exponent_classic: 2.0,
            input_offset: 0.0,
            cap: vec2d { x: 25.0, y: 1.75 }, // forced (25.0, 1.75) point
            cap_mode: cap_mode::io,
            ..Default::default()
        };
        let val_at_io = evaluate_curve(&io_cap_args, 25.0);
        assert!((val_at_io - 1.75).abs() < 1e-4, "IO cap curve must pass precisely through (cap.x, cap.y)");
    }

    #[test]
    fn test_classic_gain_mode_evaluation() {
        let args = accel_args {
            mode: accel_mode::classic,
            gain: true,
            acceleration: 0.02,
            exponent_classic: 2.0,
            input_offset: 5.0,
            cap: vec2d { x: 0.0, y: 2.0 },
            cap_mode: cap_mode::out,
            ..Default::default()
        };

        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        assert_eq!(evaluate_curve(&args, 5.0), 1.0);
        let sens = evaluate_curve(&args, 15.0);
        assert!(sens > 1.0);
        assert!(evaluate_curve(&args, 1000.0) <= 2.0 + 1e-6);
    }

    // ── 3. Natural Mode Tests ────────────────────────────────────────────

    #[test]
    fn test_natural_legacy_asymptotic_limit() {
        let args = accel_args {
            mode: accel_mode::natural,
            gain: false,
            decay_rate: 0.1,
            limit: 2.0,
            input_offset: 5.0,
            ..Default::default()
        };

        // Offset check
        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        assert_eq!(evaluate_curve(&args, 5.0), 1.0);

        // Monotonic growth bounded by limit
        let mut prev = 1.0;
        for speed in [6.0, 10.0, 20.0, 50.0, 100.0] {
            let current = evaluate_curve(&args, speed);
            assert!(current > prev, "Natural curve must increase monotonically");
            assert!(current < 2.0, "Natural curve must remain strictly below limit");
            prev = current;
        }

        // Asymptotic check at high speed
        let high_speed = evaluate_curve(&args, 5000.0);
        assert!((high_speed - 2.0).abs() < 0.01, "Natural curve must approach limit asymptotically");
    }

    #[test]
    fn test_natural_gain_mode() {
        let args = accel_args {
            mode: accel_mode::natural,
            gain: true,
            decay_rate: 0.1,
            limit: 2.5,
            input_offset: 0.0,
            ..Default::default()
        };

        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        let val_50 = evaluate_curve(&args, 50.0);
        let val_100 = evaluate_curve(&args, 100.0);
        assert!(val_100 > val_50);
        assert!(val_100 <= 2.5);
    }

    // ── 4. Jump Mode Tests ───────────────────────────────────────────────

    #[test]
    fn test_jump_legacy_instant_step() {
        let args = accel_args {
            mode: accel_mode::jump,
            gain: false,
            smooth: 0.0, // Instant step
            cap: vec2d { x: 15.0, y: 2.0 },
            ..Default::default()
        };

        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        assert_eq!(evaluate_curve(&args, 14.99), 1.0);
        assert_eq!(evaluate_curve(&args, 15.0), 2.0);
        assert_eq!(evaluate_curve(&args, 50.0), 2.0);
    }

    #[test]
    fn test_jump_legacy_smooth_sigmoid() {
        let args = accel_args {
            mode: accel_mode::jump,
            gain: false,
            smooth: 0.5,
            cap: vec2d { x: 20.0, y: 2.0 },
            ..Default::default()
        };

        let at_mid = evaluate_curve(&args, 20.0);
        // At threshold midpoint x = 20, sigmoid should evaluate exactly to 1.5
        assert!((at_mid - 1.5).abs() < 1e-6, "Jump smooth sigmoid must pass through midpoint at cap.x");

        let before_mid = evaluate_curve(&args, 15.0);
        let after_mid = evaluate_curve(&args, 25.0);
        assert!(before_mid > 1.0 && before_mid < 1.5);
        assert!(after_mid > 1.5 && after_mid < 2.0);
    }

    #[test]
    fn test_jump_gain_smooth() {
        let args = accel_args {
            mode: accel_mode::jump,
            gain: true,
            smooth: 0.5,
            cap: vec2d { x: 20.0, y: 2.0 },
            ..Default::default()
        };

        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        let val_10 = evaluate_curve(&args, 10.0);
        let val_20 = evaluate_curve(&args, 20.0);
        let val_40 = evaluate_curve(&args, 40.0);
        assert!(val_10 >= 1.0);
        assert!(val_20 > val_10);
        assert!(val_40 > val_20);
    }

    // ── 5. Power Mode Tests ──────────────────────────────────────────────

    #[test]
    fn test_power_legacy_scaling_and_offset() {
        let args = accel_args {
            mode: accel_mode::power,
            gain: false,
            scale: 0.8,
            exponent_power: 0.2,
            output_offset: 0.6,
            cap: vec2d { x: 0.0, y: 2.5 },
            cap_mode: cap_mode::out,
            ..Default::default()
        };

        // Output below offset point (offset.x = 0.0390625) must return output_offset
        let low_speed = evaluate_curve(&args, 0.02);
        assert!((low_speed - 0.6).abs() < 1e-4);
        assert!(evaluate_curve(&args, 0.1) > 0.6);

        // Monotonic growth
        let val_10 = evaluate_curve(&args, 10.0);
        let val_50 = evaluate_curve(&args, 50.0);
        assert!(val_50 > val_10);
        assert!(val_50 <= 2.5);
    }

    // ── 6. Synchronous Mode Tests ────────────────────────────────────────

    #[test]
    fn test_synchronous_legacy_symmetry() {
        let args = accel_args {
            mode: accel_mode::synchronous,
            gain: false,
            motivity: 2.0,
            sync_speed: 10.0,
            gamma: 1.0,
            smooth: 0.0, // linear clamp
            ..Default::default()
        };

        // At syncspeed: sensitivity is exactly 1.0
        let at_sync = evaluate_curve(&args, 10.0);
        assert!((at_sync - 1.0).abs() < 1e-6);

        // Below syncspeed: clamped to 1 / motivity = 0.5
        let min_sens = evaluate_curve(&args, 0.1);
        assert!((min_sens - 0.5).abs() < 1e-6);

        // Above syncspeed: clamped to motivity = 2.0
        let max_sens = evaluate_curve(&args, 100.0);
        assert!((max_sens - 2.0).abs() < 1e-6);
    }

    // ── 7. Tiered Mode Tests ─────────────────────────────────────────────

    #[test]
    fn test_tiered_linear_three_zones_and_plateaus() {
        let args = accel_args {
            mode: accel_mode::tiered,
            t_type: tiered_type::linear,
            tiered_multiplier1: 1.0,
            tiered_input_offset1: 10.0,
            tiered_multiplier2: 1.5,
            tiered_transition1: 10.0, // Transition 1: [10.0, 20.0]
            tiered_input_offset2: 30.0,
            tiered_multiplier3: 2.0,
            tiered_transition2: 10.0, // Transition 2: [30.0, 40.0]
            ..Default::default()
        };

        // Tier 1 plateau: x <= 10.0
        assert_eq!(evaluate_curve(&args, 0.0), 1.0);
        assert_eq!(evaluate_curve(&args, 5.0), 1.0);
        assert_eq!(evaluate_curve(&args, 10.0), 1.0);

        // Transition 1 linear interpolation: [10.0, 20.0]
        assert_eq!(evaluate_curve(&args, 15.0), 1.25); // midpoint: (1.0 + 1.5) / 2
        assert_eq!(evaluate_curve(&args, 20.0), 1.5);

        // Tier 2 plateau: [20.0, 30.0]
        assert_eq!(evaluate_curve(&args, 25.0), 1.5);
        assert_eq!(evaluate_curve(&args, 30.0), 1.5);

        // Transition 2 linear interpolation: [30.0, 40.0]
        assert_eq!(evaluate_curve(&args, 35.0), 1.75); // midpoint: (1.5 + 2.0) / 2
        assert_eq!(evaluate_curve(&args, 40.0), 2.0);

        // Tier 3 plateau: x >= 40.0
        assert_eq!(evaluate_curve(&args, 50.0), 2.0);
        assert_eq!(evaluate_curve(&args, 500.0), 2.0);
    }

    #[test]
    fn test_tiered_natural_continuity_at_boundary() {
        let args = accel_args {
            mode: accel_mode::tiered,
            t_type: tiered_type::natural,
            gain: false,
            tiered_multiplier1: 1.0,
            tiered_input_offset1: 10.0,
            tiered_multiplier2: 1.6,
            tiered_input_offset2: 30.0,
            tiered_multiplier3: 2.2,
            tiered_decay_rate1: 0.1,
            tiered_decay_rate2: 0.05,
            ..Default::default()
        };

        // Below offset 1: multiplier 1
        assert_eq!(evaluate_curve(&args, 5.0), 1.0);
        assert_eq!(evaluate_curve(&args, 10.0), 1.0);

        // In tier 1-2 interval: monotonic growth
        let v_15 = evaluate_curve(&args, 15.0);
        let v_25 = evaluate_curve(&args, 25.0);
        assert!(v_15 > 1.0 && v_15 < 1.6);
        assert!(v_25 > v_15 && v_25 < 1.6);

        // Continuity check at offset 2 (x = 30.0)
        let eps = 1e-4;
        let left_val = evaluate_curve(&args, 30.0 - eps);
        let exact_val = evaluate_curve(&args, 30.0);
        let right_val = evaluate_curve(&args, 30.0 + eps);
        assert!((left_val - exact_val).abs() < 1e-3, "Tiered natural must be continuous at boundary");
        assert!((right_val - exact_val).abs() < 1e-3, "Tiered natural must be continuous at boundary");

        // High speed tier 3 asymptotic progression towards multiplier 3
        let v_high = evaluate_curve(&args, 500.0);
        assert!(v_high > 1.6 && v_high <= 2.2);
    }

    #[test]
    fn test_tiered_natural_gain_mode() {
        let args = accel_args {
            mode: accel_mode::tiered,
            t_type: tiered_type::natural,
            gain: true,
            tiered_multiplier1: 1.0,
            tiered_input_offset1: 5.0,
            tiered_multiplier2: 1.5,
            tiered_input_offset2: 25.0,
            tiered_multiplier3: 2.0,
            tiered_decay_rate1: 0.1,
            tiered_decay_rate2: 0.1,
            ..Default::default()
        };

        assert_eq!(evaluate_curve(&args, 5.0), 1.0);
        let val_15 = evaluate_curve(&args, 15.0);
        let val_25 = evaluate_curve(&args, 25.0);
        let val_50 = evaluate_curve(&args, 50.0);
        assert!(val_15 > 1.0);
        assert!(val_25 > val_15);
        assert!(val_50 > val_25);
    }

    // ── 8. Lookup Table (LUT) Mode Tests ─────────────────────────────────

    #[test]
    fn test_lookup_piecewise_linear_interpolation() {
        let mut data = [0.0f32; models::LUT_RAW_DATA_CAPACITY];
        // Points: (10.0, 1.0), (20.0, 1.4), (40.0, 2.0)
        data[0] = 10.0;
        data[1] = 1.0;
        data[2] = 20.0;
        data[3] = 1.4;
        data[4] = 40.0;
        data[5] = 2.0;

        let args = accel_args {
            mode: accel_mode::lookup,
            gain: false,
            length: 6, // 3 points * 2
            data,
            ..Default::default()
        };

        // Exact points
        assert!((evaluate_curve(&args, 10.0) - 1.0).abs() < 1e-4);
        assert!((evaluate_curve(&args, 20.0) - 1.4).abs() < 1e-4);
        assert!((evaluate_curve(&args, 40.0) - 2.0).abs() < 1e-4);

        // Linear interpolation midpoints
        // Between 10 and 20 at x=15: lerp(1.0, 1.4, 0.5) = 1.2
        assert!((evaluate_curve(&args, 15.0) - 1.2).abs() < 1e-4);
        // Between 20 and 40 at x=30: lerp(1.4, 2.0, 0.5) = 1.7
        assert!((evaluate_curve(&args, 30.0) - 1.7).abs() < 1e-4);
    }

    // ── 9. Buffer Initialization and Flag Integrity ─────────────────────

    #[test]
    fn test_init_data_settings_buffer_integrity() {
        let mut settings = modifier_settings::default();
        settings.prof.degrees_rotation = 45.0;
        settings.prof.speed_max = 100.0;
        settings.prof.speed_min = 0.0;
        settings.prof.lr_output_dpi_ratio = 1.2;
        settings.prof.ud_output_dpi_ratio = 1.0;
        settings.prof.accel_x = accel_args {
            mode: accel_mode::tiered,
            t_type: tiered_type::linear,
            tiered_multiplier1: 1.0,
            tiered_input_offset1: 10.0,
            tiered_multiplier2: 1.5,
            tiered_transition1: 15.0,
            tiered_input_offset2: 30.0,
            tiered_multiplier3: 2.0,
            tiered_transition2: 20.0,
            ..Default::default()
        };
        settings.prof.accel_y = settings.prof.accel_x;

        init_data(&mut settings);

        // Verify computed flags
        assert!(settings.data.flags.apply_rotate);
        assert!(settings.data.flags.clamp_speed);
        assert!(settings.data.flags.apply_dir_mul_x);
        assert!(!settings.data.flags.apply_dir_mul_y);

        // Verify rotation direction unit vector: 45 degrees -> (cos 45, sin 45)
        let expected_rot = std::f64::consts::FRAC_1_SQRT_2;
        assert!((settings.data.rot_direction.x - expected_rot).abs() < 1e-6);
        assert!((settings.data.rot_direction.y - expected_rot).abs() < 1e-6);

        // Verify union memory is populated (not all zeroes)
        assert!(settings.data.accel_x.memory.iter().any(|&b| b != 0));
        assert!(settings.data.accel_y.memory.iter().any(|&b| b != 0));
    }
}


#[cfg(test)]
pub(crate) fn curve_layouts() -> serde_json::Map<String, serde_json::Value> {
    use std::mem::{align_of, offset_of, size_of};
    let mut layouts = serde_json::Map::new();
    layouts.insert("ClassicLegacy".into(), serde_json::json!({ "size": size_of::<ClassicLegacy>(), "align": align_of::<ClassicLegacy>(), "accel_raised": offset_of!(ClassicLegacy, accel_raised), "cap": offset_of!(ClassicLegacy, cap), "sign": offset_of!(ClassicLegacy, sign) }));
    layouts.insert("ClassicGain".into(), serde_json::json!({ "size": size_of::<ClassicGain>(), "align": align_of::<ClassicGain>(), "accel_raised": offset_of!(ClassicGain, accel_raised), "cap": offset_of!(ClassicGain, cap), "constant": offset_of!(ClassicGain, constant), "sign": offset_of!(ClassicGain, sign) }));
    layouts.insert("JumpLegacy".into(), serde_json::json!({ "size": size_of::<JumpLegacy>(), "align": align_of::<JumpLegacy>(), "step": offset_of!(JumpLegacy, step), "smooth_rate": offset_of!(JumpLegacy, smooth_rate) }));
    layouts.insert("JumpGain".into(), serde_json::json!({ "size": size_of::<JumpGain>(), "align": align_of::<JumpGain>(), "step": offset_of!(JumpGain, step), "smooth_rate": offset_of!(JumpGain, smooth_rate), "c": offset_of!(JumpGain, c) }));
    layouts.insert("NaturalLegacy".into(), serde_json::json!({ "size": size_of::<NaturalLegacy>(), "align": align_of::<NaturalLegacy>(), "offset": offset_of!(NaturalLegacy, offset), "accel": offset_of!(NaturalLegacy, accel), "limit": offset_of!(NaturalLegacy, limit) }));
    layouts.insert("NaturalGain".into(), serde_json::json!({ "size": size_of::<NaturalGain>(), "align": align_of::<NaturalGain>(), "offset": offset_of!(NaturalGain, offset), "accel": offset_of!(NaturalGain, accel), "limit": offset_of!(NaturalGain, limit), "constant": offset_of!(NaturalGain, constant) }));
    layouts.insert("PowerLegacy".into(), serde_json::json!({ "size": size_of::<PowerLegacy>(), "align": align_of::<PowerLegacy>(), "offset": offset_of!(PowerLegacy, offset), "scale": offset_of!(PowerLegacy, scale), "constant": offset_of!(PowerLegacy, constant), "cap": offset_of!(PowerLegacy, cap) }));
    layouts.insert("PowerGain".into(), serde_json::json!({ "size": size_of::<PowerGain>(), "align": align_of::<PowerGain>(), "offset": offset_of!(PowerGain, offset), "scale": offset_of!(PowerGain, scale), "constant": offset_of!(PowerGain, constant), "cap": offset_of!(PowerGain, cap), "constant_b": offset_of!(PowerGain, constant_b) }));
    layouts.insert("TieredLinearLegacy".into(), serde_json::json!({ "size": size_of::<TieredLinearLegacy>(), "align": align_of::<TieredLinearLegacy>(), "m1": offset_of!(TieredLinearLegacy, m1), "x1": offset_of!(TieredLinearLegacy, x1), "m2": offset_of!(TieredLinearLegacy, m2), "x1_end": offset_of!(TieredLinearLegacy, x1_end), "inv_trans1": offset_of!(TieredLinearLegacy, inv_trans1), "m3": offset_of!(TieredLinearLegacy, m3), "x2": offset_of!(TieredLinearLegacy, x2), "x2_end": offset_of!(TieredLinearLegacy, x2_end), "inv_trans2": offset_of!(TieredLinearLegacy, inv_trans2) }));
    layouts.insert("TieredNaturalLegacy".into(), serde_json::json!({ "size": size_of::<TieredNaturalLegacy>(), "align": align_of::<TieredNaturalLegacy>(), "m1": offset_of!(TieredNaturalLegacy, m1), "x1": offset_of!(TieredNaturalLegacy, x1), "l1": offset_of!(TieredNaturalLegacy, l1), "a1": offset_of!(TieredNaturalLegacy, a1), "x2": offset_of!(TieredNaturalLegacy, x2), "v2": offset_of!(TieredNaturalLegacy, v2), "m2_prime": offset_of!(TieredNaturalLegacy, m2_prime), "l2": offset_of!(TieredNaturalLegacy, l2), "a2": offset_of!(TieredNaturalLegacy, a2) }));
    layouts.insert("TieredNaturalGain".into(), serde_json::json!({ "size": size_of::<TieredNaturalGain>(), "align": align_of::<TieredNaturalGain>(), "m1": offset_of!(TieredNaturalGain, m1), "x1": offset_of!(TieredNaturalGain, x1), "l1": offset_of!(TieredNaturalGain, l1), "a1": offset_of!(TieredNaturalGain, a1), "x2": offset_of!(TieredNaturalGain, x2), "v2": offset_of!(TieredNaturalGain, v2), "m2_prime": offset_of!(TieredNaturalGain, m2_prime), "l2": offset_of!(TieredNaturalGain, l2), "a2": offset_of!(TieredNaturalGain, a2) }));
    layouts.insert("Lookup".into(), serde_json::json!({ "size": size_of::<Lookup>(), "align": align_of::<Lookup>(), "size": offset_of!(Lookup, size), "velocity": offset_of!(Lookup, velocity) }));
    layouts.insert("SynchronousLegacy".into(), serde_json::json!({ "size": size_of::<SynchronousLegacy>(), "align": align_of::<SynchronousLegacy>(), "log_motivity": offset_of!(SynchronousLegacy, log_motivity), "gamma_const": offset_of!(SynchronousLegacy, gamma_const), "log_syncspeed": offset_of!(SynchronousLegacy, log_syncspeed), "syncspeed": offset_of!(SynchronousLegacy, syncspeed), "sharpness": offset_of!(SynchronousLegacy, sharpness), "sharpness_recip": offset_of!(SynchronousLegacy, sharpness_recip), "use_linear_clamp": offset_of!(SynchronousLegacy, use_linear_clamp), "minimum_sens": offset_of!(SynchronousLegacy, minimum_sens), "maximum_sens": offset_of!(SynchronousLegacy, maximum_sens) }));
    layouts.insert("SynchronousGain".into(), serde_json::json!({ "size": size_of::<SynchronousGain>(), "align": align_of::<SynchronousGain>(), "velocity": offset_of!(SynchronousGain, velocity), "range_start": offset_of!(SynchronousGain, range_start), "range_stop": offset_of!(SynchronousGain, range_stop), "range_num": offset_of!(SynchronousGain, range_num), "x_start": offset_of!(SynchronousGain, x_start) }));
    layouts
}
