import { AccelArgs } from "./types";

// ── Generic Curve Evaluator Framework ───────────────────────────────────

// ── Generic Curve Evaluator Framework ───────────────────────────────────

export type Evaluator = (x: number) => number;

// common/accel-lookup.hpp clamps interpolation roundoff at the segment endpoint.
function lerp(a: number, b: number, t: number): number {
  const x = a + t * (b - a);
  return (t > 1) === (a < b) ? Math.max(x, b) : Math.min(x, b);
}

export function createEvaluator(args: AccelArgs): Evaluator {
  switch (args.mode) {
    case "tiered":
      return createTieredEvaluator(args);
    case "linear":
      return createClassicEvaluator(args, true);
    case "classic":
      return createClassicEvaluator(args, false);
    case "jump":
      return createJumpEvaluator(args);
    case "natural":
      return createNaturalEvaluator(args);
    case "power":
      return createPowerEvaluator(args);
    case "synchronous":
      return createSynchronousEvaluator(args);
    case "lookup":
      return createLookupEvaluator(args);
    case "noaccel":
    default:
      return () => 1.0;
  }
}

// ── Lookup Mode ──────────────────────────────────────────────────────────

function createLookupEvaluator(args: AccelArgs): Evaluator {
  if (!args.lookup_table || args.lookup_table.length < 2) {
    return () => 1.0;
  }

  const points = args.lookup_table.map(({ x, y }) => ({ x: Math.fround(x), y: Math.fround(y) }));

  const sensitivityFromStoredValue = (storedY: number, x: number) =>
    args.gain ? storedY / x : storedY;

  return (x: number) => {
    // Mirrors common/accel-lookup.hpp. A velocity LUT stores output velocity,
    // while the graph and driver modifier consume sensitivity (output / input).
    if (x <= 0) return 0;
    if (x < points[0].x) {
      return sensitivityFromStoredValue(points[0].y, points[0].x);
    }

    for (let i = 0; i < points.length - 1; i++) {
      if (x === points[i].x) return sensitivityFromStoredValue(points[i].y, x);
      if (x < points[i + 1].x || i === points.length - 2) {
        const dx = Math.fround(points[i + 1].x - points[i].x);
        if (dx <= 0) return 1.0;
        const t = (x - points[i].x) / dx;
        const y = lerp(points[i].y, points[i + 1].y, t);
        return sensitivityFromStoredValue(y, x);
      }
    }

    return 1.0;
  };
}

// ── Jump Mode ────────────────────────────────────────────────────────────

function createJumpEvaluator(args: AccelArgs): Evaluator {
  const smooth_scale = 2 * Math.PI;
  const step = { x: args.cap.x, y: args.cap.y - 1 };
  const rate_inverse = args.smooth * step.x;
  
  let smooth_rate = 0;
  if (rate_inverse >= 1) {
    smooth_rate = smooth_scale / rate_inverse;
  }
  
  const is_smooth = smooth_rate !== 0;
  
  const decay = (x: number) => Math.exp(smooth_rate * (step.x - x));
  const smooth = (x: number) => step.y / (1 + decay(x));
  const smooth_antideriv = (x: number) => step.y * (x + Math.log(1 + decay(x)) / smooth_rate);

  if (!args.gain) {
    return (x: number) => {
      if (is_smooth) return smooth(x) + 1;
      if (x < step.x) return 1;
      return 1 + step.y;
    };
  } else {
    const C = -smooth_antideriv(0);
    return (x: number) => {
      if (x <= 0) return 1;
      if (is_smooth) return 1 + (smooth_antideriv(x) + C) / x;
      if (x < step.x) return 1;
      return 1 + step.y * (x - step.x) / x;
    };
  }
}

// ── Natural Mode ─────────────────────────────────────────────────────────

function createNaturalEvaluator(args: AccelArgs): Evaluator {
  const offset = args.input_offset;
  const limit = args.limit - 1;
  const accel = args.decay_rate / Math.abs(limit);

  if (!args.gain) {
    return (x: number) => {
      if (x <= offset) return 1;
      const offset_x = offset - x;
      const decay = Math.exp(accel * offset_x);
      return limit * (1 - (offset - decay * offset_x) / x) + 1;
    };
  } else {
    const constant = -limit / accel;
    return (x: number) => {
      if (x <= offset) return 1;
      const offset_x = offset - x;
      const decay = Math.exp(accel * offset_x);
      const output = limit * (decay / accel - offset_x) + constant;
      return output / x + 1;
    };
  }
}

// ── Power Mode ───────────────────────────────────────────────────────────

function createPowerEvaluator(args: AccelArgs): Evaluator {
  const n = args.exponent_power;
  
  const power_gain = (input: number, power: number, scale: number) => (power + 1) * Math.pow(input * scale, power);
  const power_gain_inverse = (gain: number, power: number, scale: number) => Math.pow(gain / (power + 1), 1 / power) / scale;
  const scale_from_gain = (input: number, gain: number, power: number) => Math.pow(gain / (power + 1), 1 / power) / input;
  const scale_from_output = (input: number, output: number, power: number, C: number) => Math.pow(output - C / input, 1 / power) / input;
  const integration_constant = (input: number, gain: number, output: number) => (output - gain) * input;

  let scale = args.scale;
  if (args.cap_mode === "io") {
    if (args.gain) {
      scale = scale_from_gain(args.cap.x, args.cap.y, n);
    } else {
      scale = scale_from_output(args.cap.x, args.cap.y, n, 0);
      return (x: number) => Math.min(Math.pow(scale * x, n), args.cap.y);
    }
  }

  const offset = {
    x: power_gain_inverse(args.output_offset, n, scale),
    y: args.output_offset
  };
  const constant = offset.x * offset.y * n / (n + 1);

  const base_fn = (x: number) => {
    if (x <= offset.x) return offset.y;
    return Math.pow(scale * x, n) + constant / x;
  };

  if (!args.gain) {
    let cap = Number.MAX_VALUE;
    if (args.cap_mode === "io") cap = args.cap.y;
    else if (args.cap_mode === "in") { if (args.cap.x > 0) cap = base_fn(args.cap.x); }
    else { if (args.cap.y > 0) cap = args.cap.y; }

    return (x: number) => Math.min(base_fn(x), cap);
  } else {
    let cap = { x: Number.MAX_VALUE, y: Number.MAX_VALUE };
    if (args.cap_mode === "io") {
      cap = { ...args.cap };
    } else if (args.cap_mode === "in") {
      if (args.cap.x > 0) {
        if (args.cap.x <= offset.x) {
          cap.x = 0; cap.y = offset.y;
        } else {
          cap.x = args.cap.x;
          cap.y = power_gain(args.cap.x, n, scale);
        }
      }
    } else {
      if (args.cap.y > 0) {
        cap.x = power_gain_inverse(args.cap.y, n, scale);
        cap.y = args.cap.y;
      }
    }
    
    let constant_b = 0;
    if (cap.x !== 0 || cap.y !== offset.y) {
      constant_b = integration_constant(cap.x, cap.y, base_fn(cap.x));
    }

    return (x: number) => {
      if (x < cap.x) return base_fn(x);
      return cap.y + constant_b / x;
    };
  }
}

// ── Synchronous Mode ─────────────────────────────────────────────────────

function createSynchronousEvaluator(args: AccelArgs): Evaluator {
  const log_motivity = Math.log(args.motivity);
  const gamma_const = args.gamma / log_motivity;
  const sharpness = args.smooth === 0 ? 16 : 0.5 / args.smooth;
  const sharpness_recip = 1 / sharpness;
  const use_linear_clamp = sharpness >= 16;
  const minimum_sens = 1 / args.motivity;
  const maximum_sens = args.motivity;
  const log_syncspeed = Math.log(args.sync_speed);

  const evaluateLegacy = (x: number) => {
    if (x === 0) return minimum_sens;
    if (use_linear_clamp) {
      const log_space = gamma_const * (Math.log(x) - log_syncspeed);
      if (log_space < -1) return minimum_sens;
      if (log_space > 1) return maximum_sens;
      return Math.exp(log_space * log_motivity);
    }
    if (x === args.sync_speed) return 1.0;
    
    const log_x = Math.log(x);
    const log_diff = log_x - log_syncspeed;
    if (log_diff > 0) {
      const log_space = gamma_const * log_diff;
      const exponent = Math.pow(Math.tanh(Math.pow(log_space, sharpness)), sharpness_recip);
      return Math.exp(exponent * log_motivity);
    } else {
      const log_space = -gamma_const * log_diff;
      const exponent = -Math.pow(Math.tanh(Math.pow(log_space, sharpness)), sharpness_recip);
      return Math.exp(exponent * log_motivity);
    }
  };

  if (!args.gain) {
    return evaluateLegacy;
  }

  // common/accel-synchronous.hpp: two right-endpoint partitions per sample,
  // 8 samples per octave over [2^-3, 2^9], stored as f32 output velocities.
  const data: number[] = [];
  let sum = 0;
  let a = 0;
  const sample = (b: number) => {
    const interval = (b - a) / 2;
    for (let i = 1; i <= 2; i++) {
      sum += evaluateLegacy(a + i * interval) * interval;
    }
    a = b;
    data.push(Math.fround(sum));
  };
  for (let e = -3; e < 9; e++) {
    const scale = 2 ** e / 8;
    for (let i = 0; i < 8; i++) sample((i + 8) * scale);
  }
  sample(2 ** 9);

  return (x: number) => {
    if (x < 0.125) return data[0] / 0.125;
    const e = Math.min(Math.floor(Math.log2(x)), 8);
    const idxF = 8 * ((e + 3) + (x * 2 ** -e - 1));
    const idx = Math.min(Math.trunc(idxF), data.length - 2);
    const y = data[idx] + (idxF - idx) * (data[idx + 1] - data[idx]);
    return y / x;
  };
}

function classic_base_fn(x: number, accel_raised: number, args: AccelArgs): number {
  return accel_raised * Math.pow(x - args.input_offset, args.exponent_classic) / x;
}

function classic_base_accel(x: number, y: number, args: AccelArgs): number {
  const power = args.exponent_classic;
  return Math.pow(x * y * Math.pow(x - args.input_offset, -power), 1 / (power - 1));
}

function classic_gain(x: number, accel: number, power: number, offset: number): number {
  return power * Math.pow(accel * (x - offset), power - 1);
}

function classic_gain_inverse(y: number, accel: number, power: number, offset: number): number {
  return (accel * offset + Math.pow(y / power, 1 / (power - 1))) / accel;
}

function classic_gain_accel(x: number, y: number, power: number, offset: number): number {
  return -Math.pow(y / power, 1 / (power - 1)) / (offset - x);
}

function createClassicEvaluator(originalArgs: AccelArgs, isLinear: boolean): Evaluator {
  const power = isLinear ? 2.0 : originalArgs.exponent_classic;
  const args = { ...originalArgs, exponent_classic: power };

  if (!args.gain) {
    let sign = 1;
    let cap = Number.MAX_VALUE;
    let accel_raised = 0;

    if (args.cap_mode === "io") {
      cap = args.cap.y - 1;
      if (cap < 0) { cap = -cap; sign = -sign; }
      const a = classic_base_accel(args.cap.x, cap, args);
      accel_raised = Math.pow(a, power - 1);
    } else if (args.cap_mode === "in") {
      accel_raised = Math.pow(args.acceleration, power - 1);
      if (args.cap.x > 0) cap = classic_base_fn(args.cap.x, accel_raised, args);
    } else {
      accel_raised = Math.pow(args.acceleration, power - 1);
      if (args.cap.y > 0) {
        cap = args.cap.y - 1;
        if (cap < 0) { cap = -cap; sign = -sign; }
      }
    }

    return (x: number) => {
      if (x <= args.input_offset) return 1.0;
      return sign * Math.min(classic_base_fn(x, accel_raised, args), cap) + 1.0;
    };
  } else {
    let sign = 1;
    let cap = { x: Number.MAX_VALUE, y: Number.MAX_VALUE };
    let constant = 0;
    let accel_raised = 0;

    if (args.cap_mode === "io") {
      cap.x = args.cap.x;
      cap.y = args.cap.y - 1;
      if (cap.y < 0) { cap.y = -cap.y; sign = -sign; }
      const a = classic_gain_accel(cap.x, cap.y, power, args.input_offset);
      accel_raised = Math.pow(a, power - 1);
      constant = (classic_base_fn(cap.x, accel_raised, args) - cap.y) * cap.x;
    } else if (args.cap_mode === "in") {
      accel_raised = Math.pow(args.acceleration, power - 1);
      if (args.cap.x > 0) {
        cap.x = args.cap.x;
        cap.y = classic_gain(cap.x, args.acceleration, power, args.input_offset);
        constant = (classic_base_fn(cap.x, accel_raised, args) - cap.y) * cap.x;
      }
    } else {
      accel_raised = Math.pow(args.acceleration, power - 1);
      if (args.cap.y > 0) {
        cap.y = args.cap.y - 1;
        if (cap.y === 0) {
          cap.x = 0;
        } else {
          if (cap.y < 0) { cap.y = -cap.y; sign = -sign; }
          cap.x = classic_gain_inverse(cap.y, args.acceleration, power, args.input_offset);
          constant = (classic_base_fn(cap.x, accel_raised, args) - cap.y) * cap.x;
        }
      }
    }

    return (x: number) => {
      if (x <= args.input_offset) return 1.0;
      let output;
      if (x < cap.x) {
        output = classic_base_fn(x, accel_raised, args);
      } else {
        output = constant / x + cap.y;
      }
      return sign * output + 1.0;
    };
  }
}

// ── Tiered Mode ──────────────────────────────────────────────────────────

function createTieredEvaluator(args: AccelArgs): Evaluator {
  const evaluateLinear = (x: number) => {
    if (x <= args.tiered_input_offset1) return args.tiered_multiplier1;
    
    if (x < args.tiered_input_offset1 + args.tiered_transition1) {
      const denom = args.tiered_transition1;
      if (denom <= 0) return args.tiered_multiplier2;
      const t = (x - args.tiered_input_offset1) / denom;
      return args.tiered_multiplier1 + t * (args.tiered_multiplier2 - args.tiered_multiplier1);
    }
    
    if (x <= args.tiered_input_offset2) return args.tiered_multiplier2;
    
    if (x < args.tiered_input_offset2 + args.tiered_transition2) {
      const denom = args.tiered_transition2;
      if (denom <= 0) return args.tiered_multiplier3;
      const t = (x - args.tiered_input_offset2) / denom;
      return args.tiered_multiplier2 + t * (args.tiered_multiplier3 - args.tiered_multiplier2);
    }
    
    return args.tiered_multiplier3;
  };

  return (x: number) => {
    if (args.t_type === "linear") {
      return evaluateLinear(x);
    }

    const m1 = args.tiered_multiplier1;
    const x1 = args.tiered_input_offset1;
    const v1 = m1 * x1;
    const l1 = args.tiered_multiplier2 - m1;
    const a1 = l1 !== 0 ? args.tiered_decay_rate1 / Math.abs(l1) : 0;
    
    const x2 = args.tiered_input_offset2;
    const dx1 = x2 - x1;
    const decay1 = l1 !== 0 ? Math.exp(-a1 * dx1) : 1.0;
    
    let v2 = v1 + m1 * dx1;
    let m2_prime = m1;
    if (l1 !== 0) {
      if (args.gain) {
        v2 = v1 + m1 * dx1 + l1 * (dx1 - (1.0 - decay1) / a1);
        m2_prime = m1 + l1 * (1.0 - decay1);
      } else {
        v2 = v1 + m1 * dx1 + l1 * dx1 * (1.0 - decay1);
        m2_prime = m1 + l1 * (1.0 - decay1 + dx1 * a1 * decay1);
      }
    }
    
    const l2 = args.tiered_multiplier3 - m2_prime;
    const a2 = l2 !== 0 ? args.tiered_decay_rate2 / Math.abs(l2) : 0;

    if (x <= x1) return m1;

    if (x < x2) {
      if (l1 === 0) return m1;
      const dx = x - x1;
      const decay = Math.exp(-a1 * dx);
      if (args.gain) {
        const v = v1 + m1 * dx + l1 * (dx - (1.0 - decay) / a1);
        return v / x;
      } else {
        const v = v1 + m1 * dx + l1 * dx * (1.0 - decay);
        return v / x;
      }
    }

    if (l2 === 0) {
      const dx2 = x - x2;
      return (v2 + m2_prime * dx2) / x;
    }

    const dx2 = x - x2;
    const decay2 = Math.exp(-a2 * dx2);
    if (args.gain) {
      const v = v2 + m2_prime * dx2 + l2 * (dx2 - (1.0 - decay2) / a2);
      return v / x;
    } else {
      const v = v2 + m2_prime * dx2 + l2 * dx2 * (1.0 - decay2);
      return v / x;
    }
  };
}

// ── Graph Data Generation ───────────────────────────────────────────────

export interface GraphPoint {
  x: number;
  sensitivity: number;
  velocity: number;
  gain: number;
}

export function generateGraphData(args: AccelArgs, maxSpeed: number = 50, resolution: number = 200): GraphPoint[] {
  const evaluator = createEvaluator(args);
  const data: GraphPoint[] = [];
  const step = maxSpeed / resolution;
  
  for (let x = 0.001; x <= maxSpeed; x += step) {
    const sensitivity = evaluator(x);
    const velocity = x * sensitivity;
    data.push({ x, sensitivity, velocity, gain: 0 });
  }

  // Calculate Gain (derivative of velocity)
  for (let i = 0; i < data.length; i++) {
    if (i === 0) {
      data[i].gain = data[i].sensitivity; // Approximation at 0
    } else {
      const dx = data[i].x - data[i - 1].x;
      const dy = data[i].velocity - data[i - 1].velocity;
      data[i].gain = dx > 0 ? dy / dx : data[i].sensitivity;
    }
  }

  return data;
}
