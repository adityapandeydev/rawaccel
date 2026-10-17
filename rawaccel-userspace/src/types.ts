export const SETTINGS_SCHEMA_VERSION = 1;

export interface SettingsApplyReport {
  applied: boolean;
  saved: boolean;
  saved_paths: string[];
  warnings: string[];
  save_error: string | null;
}

export interface SettingsLoadReport {
  settings_json: string;
  source: string;
  recovered: boolean;
  migrated: boolean;
  warnings: string[];
}

export interface Vec2D {
  x: number;
  y: number;
}

export interface DeviceConfig {
  id: string;
  disable: boolean;
  set_extra_info: boolean;
  poll_time_lock: boolean;
  dpi: number;
  polling_rate: number;
  clamp_min: number;
  clamp_max: number;
  profile_id: string | null;
}

export interface Device {
  id: string;
  name: string;
}

export interface AccelArgs {
  mode: string; // "classic", "jump", "natural", "power", "synchronous", "lookup" | "tiered", "noaccel"
  gain: boolean;
  input_offset: number;
  output_offset: number;
  acceleration: number;
  decay_rate: number;
  gamma: number;
  motivity: number;
  exponent_classic: number;
  scale: number;
  exponent_power: number;
  limit: number;
  sync_speed: number;
  smooth: number;
  
  // Tiered Mode Fields
  t_type: "linear" | "natural";
  tiered_multiplier1: number;
  tiered_input_offset1: number;
  tiered_multiplier2: number;
  tiered_transition1: number;
  tiered_input_offset2: number;
  tiered_multiplier3: number;
  tiered_transition2: number;
  tiered_decay_rate1: number;
  tiered_decay_rate2: number;

  cap: Vec2D;
  cap_mode: "in" | "out" | "io";
  // The editable curve sent to the driver when mode is "lookup".
  lookup_table?: Vec2D[];
  // Optional CSV source snapshot. It is never sent to the driver.
  lookup_csv_backup?: Vec2D[];
  lookup_csv_backup_gain?: boolean;
}

export interface SpeedArgs {
  whole: boolean;
  lp_norm: number;
  input_speed_smooth_halflife: number;
  scale_smooth_halflife: number;
  output_speed_smooth_halflife: number;
}

export interface Profile {
  id: string; // Frontend only identifier
  name: string;
  domain_weights: Vec2D;
  range_weights: Vec2D;
  accel_x: AccelArgs;
  accel_y: AccelArgs;
  speed_processor_args: SpeedArgs;
  output_dpi: number;
  yx_output_dpi_ratio: number;
  lr_output_dpi_ratio: number;
  ud_output_dpi_ratio: number;
  degrees_rotation: number;
  degrees_snap: number;
  speed_min: number;
  speed_max: number;
}

// ── Default Generators ──────────────────────────────────────────────────

export const defaultAccelArgs = (): AccelArgs => ({
  mode: "noaccel",
  gain: true,
  input_offset: 0,
  output_offset: 0,
  acceleration: 0,
  decay_rate: 0.1,
  gamma: 1,
  motivity: 1.5,
  exponent_classic: 2,
  scale: 1,
  exponent_power: 2,
  limit: 2,
  sync_speed: 10,
  smooth: 0,
  t_type: "linear",
  tiered_multiplier1: 1.0,
  tiered_input_offset1: 0.0,
  tiered_multiplier2: 1.5,
  tiered_transition1: 10.0,
  tiered_input_offset2: 15.0,
  tiered_multiplier3: 2.0,
  tiered_transition2: 25.0,
  tiered_decay_rate1: 0.1,
  tiered_decay_rate2: 0.1,
  cap: { x: 15, y: 1.5 },
  cap_mode: "out",
  lookup_table: []
});

export const defaultSpeedArgs = (): SpeedArgs => ({
  whole: true,
  lp_norm: 2.0,
  input_speed_smooth_halflife: 0,
  scale_smooth_halflife: 0,
  output_speed_smooth_halflife: 0
});

export const defaultProfile = (name: string = "Default Profile"): Profile => ({
  id: crypto.randomUUID ? crypto.randomUUID() : Date.now().toString(),
  name,
  domain_weights: { x: 1, y: 1 },
  range_weights: { x: 1, y: 1 },
  accel_x: defaultAccelArgs(),
  accel_y: defaultAccelArgs(),
  speed_processor_args: defaultSpeedArgs(),
  output_dpi: 1000,
  yx_output_dpi_ratio: 1,
  lr_output_dpi_ratio: 1,
  ud_output_dpi_ratio: 1,
  degrees_rotation: 0,
  degrees_snap: 0,
  speed_min: 0,
  speed_max: 0
});




