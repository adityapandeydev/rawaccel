import type { Device, DeviceConfig } from './types';

// Zero numeric overrides inherit driver defaults. New devices stay unassigned.
export const defaultDeviceConfig = (id: string): DeviceConfig => ({
  id, profile_id: null, disable: false, set_extra_info: false,
  poll_time_lock: false, dpi: 0, polling_rate: 0, clamp_min: 0, clamp_max: 0,
});

export function resolveDeviceSelection(id: string | null, devices: Device[]): string | null {
  return devices.some(device => device.id === id) ? id : devices[0]?.id ?? null;
}

export function patchDevice(config: DeviceConfig[], id: string, patch: Partial<DeviceConfig>): DeviceConfig[] {
  const exists = config.some(device => device.id === id);
  return exists
    ? config.map(device => device.id === id ? { ...device, ...patch, id } : device)
    : [...config, { ...defaultDeviceConfig(id), ...patch, id }];
}
