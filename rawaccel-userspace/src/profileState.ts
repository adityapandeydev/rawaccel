import type { Profile, DeviceConfig } from './types';

export function renameProfileState(profiles: Profile[], devices: DeviceConfig[], index: number, name: string) {
  if (profiles.some((profile, i) => i !== index && profile.name.trim().toLowerCase() === name.trim().toLowerCase())) {
    throw new Error('A profile with that name already exists.');
  }
  const oldName = profiles[index].name;
  return {
    profiles: profiles.map((profile, i) => i === index ? { ...profile, name } : profile),
    devices: oldName !== name
      ? devices.map(device => device.profile_id === oldName ? { ...device, profile_id: name } : device)
      : devices,
  };
}

export function deleteProfileState(profiles: Profile[], devices: DeviceConfig[], activeIndex: number, index: number) {
  if (profiles.length <= 1) throw new Error('Cannot delete the last profile.');
  const name = profiles[index].name;
  return {
    profiles: profiles.filter((_, i) => i !== index),
    devices: devices.map(device => device.profile_id === name ? { ...device, profile_id: null } : device),
    activeIndex: activeIndex === index ? Math.max(0, index - 1) : activeIndex > index ? activeIndex - 1 : activeIndex,
  };
}
