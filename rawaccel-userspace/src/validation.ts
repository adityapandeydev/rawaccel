import type { AccelArgs, Profile, DeviceConfig } from "./types";

// JS string length counts the UTF-16 units used by the Windows wire format.
function wireString(value: string, capacity: number, label: string): string | null {
  if (typeof value !== "string") return `${label} must be a string`;
  if (!value.trim() || value.includes("\0")) return `${label} must be nonempty and cannot contain NUL`;
  return value.length >= capacity ? `${label} exceeds ${capacity - 1} UTF-16 code units` : null;
}

function deviceNumbers(value: Record<string, unknown>, inherited: [number, number]): string | null {
  for (const key of ["dpi", "polling_rate"]) {
    const n = value[key] === undefined ? 0 : value[key];
    if (typeof n !== "number" || !Number.isInteger(n) || n < 0 || n > 2147483647) return "DPI and polling rate must fit a nonnegative signed 32-bit integer";
  }
  const min = value.clamp_min === undefined ? 0 : value.clamp_min, max = value.clamp_max === undefined ? 0 : value.clamp_max;
  if (typeof min !== "number" || typeof max !== "number" || !Number.isFinite(min) || !Number.isFinite(max) || min < 0 || max < 0) return "Time clamps must be finite and nonnegative; zero inherits the default";
  if ((min || inherited[0]) <= 0 || (max || inherited[1]) < (min || inherited[0])) return "Effective minimum time must be positive and no greater than maximum time";
  return null;
}

export function validateSettings(profiles: Profile[], devices: DeviceConfig[], defaults?: unknown): string | null {
  if (!profiles.length) return "At least one profile is required";
  let inherited: [number, number] = [0.0625, 100];
  if (defaults != null) {
    if (typeof defaults !== "object" || Array.isArray(defaults)) return "Invalid default device configuration";
    const d = defaults as Record<string, unknown>;
    const err = deviceNumbers(d, inherited);
    if (err) return `Default device: ${err}`;
    inherited = [(d.clamp_min as number) || inherited[0], (d.clamp_max as number) || inherited[1]];
  }
  const names = new Set<string>();
  for (const p of profiles) {
    const err = validateProfile(p);
    if (err) return `Profile '${p.name}': ${err}`;
    const name = p.name.trim().toLowerCase();
    if (names.has(name)) return `Duplicate profile name '${p.name}'`;
    names.add(name);
  }
  const ids = new Set<string>();
  for (const device of devices) {
    const err = wireString(device.id, 200, "Device ID") || deviceNumbers(device as unknown as Record<string, unknown>, inherited);
    if (err) return `Device '${device.id}': ${err}`;
    if (ids.has(device.id)) return `Duplicate device ID '${device.id}'`;
    ids.add(device.id);
    if (device.profile_id != null) {
      const mappingError = wireString(device.profile_id, 256, "Mapped profile name");
      if (mappingError) return mappingError;
      if (!profiles.some(p => p.name === device.profile_id)) return `Device '${device.id}' references non-existent profile '${device.profile_id}'`;
    }
  }
  return null;
}

function validateAccel(args: AccelArgs, isY: boolean): string | null {
  const axis = isY ? 'Y Axis' : 'X Axis';


  if (!["noaccel", "linear", "classic", "jump", "natural", "synchronous", "power", "lookup", "tiered"].includes(args.mode)) return `${axis}: Unknown acceleration mode`;
  if (!["in", "out", "io"].includes(args.cap_mode)) return `${axis}: Unknown cap mode`;
  if (!["linear", "natural"].includes(args.t_type)) return `${axis}: Unknown Tiered subtype`;
  if (![
    args.input_offset, args.output_offset, args.acceleration, args.decay_rate,
    args.gamma, args.motivity, args.exponent_classic, args.scale,
    args.exponent_power, args.limit, args.sync_speed, args.smooth,
    args.tiered_multiplier1, args.tiered_input_offset1, args.tiered_multiplier2, args.tiered_transition1,
    args.tiered_input_offset2, args.tiered_multiplier3, args.tiered_transition2, args.tiered_decay_rate1,
    args.tiered_decay_rate2, args.cap.x, args.cap.y,
  ].every(Number.isFinite)) return `${axis}: Curve values must be finite`;
  const points = args.lookup_table ?? [];
  if (args.mode === "lookup" && points.length < 2) return `${axis}: Lookup Table requires at least two points`;
  if (points.length > 257) return `${axis}: Lookup Table supports at most 257 points`;
  let previous = 0;
  for (const point of points) {
    const x = Math.fround(point.x), y = Math.fround(point.y);
    if (!Number.isFinite(x) || !Number.isFinite(y)) return `${axis}: Lookup values must remain finite when packed as f32`;
    if (x <= 0 || x <= previous) return `${axis}: Lookup input speeds must remain positive and strictly increasing when packed as f32`;
    previous = x;
  }
  if (["jump", "synchronous"].includes(args.mode) && (args.smooth < 0 || args.smooth > 1)) return `${axis}: Smooth must be between 0 and 1`;
  // Base validations for offset and cap
  if (args.input_offset < 0) return `${axis}: Input Offset cannot be negative`;
  if (args.output_offset < 0) return `${axis}: Output Offset cannot be negative`;

  const jump_or_io_cap = (args.mode === 'jump' || ((args.mode === 'classic' || args.mode === 'linear' || args.mode === 'power') && args.cap_mode === 'io'));
  if (args.cap.x < 0) return `${axis}: Cap cannot be negative`;
  else if (args.cap.x === 0 && jump_or_io_cap) return `${axis}: Cap cannot be 0 for this mode/cap style`;
  if (args.cap.y < 0) return `${axis}: Cap (output) cannot be negative`;
  else if (args.cap.y === 0 && jump_or_io_cap) return `${axis}: Cap (output) cannot be 0 for this mode/cap style`;

  if ((args.mode === 'classic' || args.mode === 'linear') && args.cap.x > 0 && args.cap.x < args.input_offset && args.cap_mode !== 'out') return `${axis}: Cap cannot be less than Input Offset`;
  if (args.mode === 'power' && args.cap.y > 0 && args.cap.y < args.output_offset && args.cap_mode !== 'in') return `${axis}: Cap cannot be less than Output Offset`;

  // Mode-specific validations
  switch (args.mode) {
    case 'classic':
      if (args.acceleration <= 0) return `${axis}: Acceleration must be positive (greater than 0)`;
      if (args.exponent_classic <= 1) return `${axis}: Classic Exponent must be strictly greater than 1`;
      break;
    case 'linear':
      if (args.acceleration <= 0) return `${axis}: Acceleration must be positive (greater than 0)`;
      break;
    case 'natural':
      if (args.decay_rate <= 0) return `${axis}: Decay Rate must be positive (greater than 0)`;
      if (args.limit <= 0) return `${axis}: Limit must be positive`;
      break;
    case 'power':
      if (args.scale <= 0) return `${axis}: Scale must be positive (greater than 0)`;
      if (args.exponent_power <= 0) return `${axis}: Power Exponent must be positive (greater than 0)`;
      break;
    case 'synchronous':
      if (args.sync_speed <= 0) return `${axis}: Synchronous speed must be positive`;
      if (args.motivity <= 1) return `${axis}: Motivity must be strictly greater than 1`;
      if (args.gamma <= 0) return `${axis}: Gamma must be positive`;
      break;
    case 'tiered':
      if (args.tiered_multiplier1 <= 0 || args.tiered_multiplier2 <= 0 || args.tiered_multiplier3 <= 0) {
        return `${axis}: Tiered multipliers must be positive (greater than 0)`;
      }
      if (args.tiered_input_offset1 < 0 || args.tiered_input_offset2 < 0) {
        return `${axis}: Offsets cannot be negative`;
      }
      if (args.t_type === 'linear') {
        if (!Number.isFinite(args.tiered_input_offset1 + args.tiered_transition1) || !Number.isFinite(args.tiered_input_offset2 + args.tiered_transition2)) return `${axis}: Transition endpoints must be finite`;
        if (args.tiered_transition1 <= 0 || args.tiered_transition2 <= 0) {
          return `${axis}: Transition widths must be positive (greater than 0)`;
        }
        if (args.tiered_input_offset2 < args.tiered_input_offset1 + args.tiered_transition1) {
          return `${axis}: Second offset must be greater than or equal to first transition end`;
        }
      } else if (args.t_type === 'natural') {
        if (args.tiered_input_offset2 <= args.tiered_input_offset1) {
          return `${axis}: Second offset must be strictly greater than first offset`;
        }
        if (args.tiered_decay_rate1 <= 0 || args.tiered_decay_rate2 <= 0) {
          return `${axis}: Decay rates must be positive (greater than 0)`;
        }
      }
      break;
  }

  return null;
}

export function validateProfile(profile: Profile): string | null {
  const nameError = wireString(profile.name, 256, "Profile name");
  if (nameError) return nameError;
  if (![profile.output_dpi, profile.yx_output_dpi_ratio, profile.lr_output_dpi_ratio, profile.ud_output_dpi_ratio, profile.degrees_rotation, profile.degrees_snap, profile.speed_min, profile.speed_max, profile.domain_weights.x, profile.domain_weights.y, profile.range_weights.x, profile.range_weights.y,
    profile.speed_processor_args.lp_norm, profile.speed_processor_args.input_speed_smooth_halflife, profile.speed_processor_args.scale_smooth_halflife, profile.speed_processor_args.output_speed_smooth_halflife].every(Number.isFinite)) return "Profile values must be finite";
  let err = validateAccel(profile.accel_x, false);
  if (err) return err;

  // Both axes are precomputed by the driver, even in Combined mode.
  err = validateAccel(profile.accel_y, true);
  if (err) return err;

  if (profile.speed_max < 0) return 'speed cap is negative';
  if (profile.speed_max < profile.speed_min) return 'max speed is less than min speed';
  if (profile.degrees_snap < 0 || profile.degrees_snap > 45) return 'snap angle must be between 0 and 45 degrees';
  if (profile.output_dpi === 0) return 'output DPI is 0';
  if (profile.yx_output_dpi_ratio === 0) return 'Y/X output DPI ratio is 0';
  if (profile.domain_weights.x <= 0 || profile.domain_weights.y <= 0) return 'domain weights must be positive';
  if (profile.lr_output_dpi_ratio <= 0 || profile.ud_output_dpi_ratio <= 0) return 'output DPI ratio must be positive';
  if (profile.speed_processor_args.lp_norm <= 0) return 'Lp norm must be positive (default=2)';
  if (profile.range_weights.x < 0 || profile.range_weights.y < 0) return 'range weights must be nonnegative';

  return null;
}
