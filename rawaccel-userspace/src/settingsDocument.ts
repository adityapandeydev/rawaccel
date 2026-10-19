import type { DeviceConfig, Profile } from './types';

export interface SettingsDocument extends Record<string, unknown> {
  profiles?: Profile[];
  devices?: DeviceConfig[];
}

// Backend load/import normalizes and validates. Keep unknown document metadata
// separate from the editable fields so Apply and export use the same document.
export function splitSettingsDocument(document: SettingsDocument) {
  const { profiles, devices, ...metadata } = document;
  return { profiles, devices, metadata };
}

export function serializeSettingsDocument(metadata: Record<string, unknown>, profiles: Profile[], devices: DeviceConfig[], pretty = false) {
  return JSON.stringify({ ...metadata, profiles, devices }, null, pretty ? 2 : undefined);
}
