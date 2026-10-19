export type ThemePreference = 'dark' | 'light' | 'system';
export const THEME_KEY = 'rawaccel.theme';
export function parseTheme(value: string | null): ThemePreference {
  return value === 'dark' || value === 'light' || value === 'system' ? value : 'system';
}
export function resolveTheme(preference: ThemePreference, systemDark: boolean): 'dark' | 'light' {
  return preference === 'system' ? (systemDark ? 'dark' : 'light') : preference;
}
