export const PREFERENCES_KEY = 'rawaccel.preferences';
export interface Preferences { showToastNotifications: boolean; showConfirmModals: boolean; }

export function parsePreferences(raw: string | null): Preferences {
  let value: Partial<Preferences> | null = null;
  try { value = raw ? JSON.parse(raw) : null; } catch { /* Recover defaults. */ }
  return {
    showToastNotifications: typeof value?.showToastNotifications === 'boolean' ? value.showToastNotifications : true,
    showConfirmModals: typeof value?.showConfirmModals === 'boolean' ? value.showConfirmModals : true,
  };
}

export function notificationChannel(kind: 'info' | 'error' | undefined, showToasts: boolean) {
  return kind === 'info' ? showToasts ? 'toast' : 'none' : 'dialog';
}
