import type { SettingsApplyReport } from './types';
import type { Notification } from './hooks/useNotifications';

export function describeApplyOutcome(result: SettingsApplyReport): { succeeded: boolean; notification?: Notification } {
  if (!result.saved) return {
    succeeded: false,
    notification: { isOpen: true, title: 'Applied, But Not Saved',
      message: `Your settings are active in the driver, but could not be saved. They may be lost after restarting.\n${result.save_error ?? 'No settings location could be updated.'}` },
  };
  if (result.warnings.length > 0) return {
    succeeded: true,
    notification: { isOpen: true, title: 'Applied and Saved With Warnings',
      message: `Your settings are active and were saved to:\n${result.saved_paths.join('\n')}\nSome locations could not be updated:\n${result.warnings.join('\n')}` },
  };
  return { succeeded: true };
}
