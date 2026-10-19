import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createApplyGate } from '../applyGate';
import { applyBlockReason } from '../driverStatus';
import { describeApplyOutcome } from '../applyOutcome';
import { serializeSettingsDocument } from '../settingsDocument';
import { validateSettings } from '../validation';
import type { SettingsApplyReport } from '../types';
import type { ProfileEditor } from './useProfileEditor';
import type { useDriverStatus } from './useDriverStatus';
import type { Notify } from './useNotifications';

export function useApplySettings(editor: ProfileEditor, driverStatus: ReturnType<typeof useDriverStatus>, notify: Notify) {
  const [runApply] = useState(createApplyGate);
  const [isApplying, setIsApplying] = useState(false);
  const [appliedSnapshot, setAppliedSnapshot] = useState<string | null>(null);
  const { profiles, deviceConfig, settingsMetadata } = editor;
  const applyDisabledReason = applyBlockReason(driverStatus, profiles);
  const applySettings = () => runApply(async () => {
    setIsApplying(true);
    try {
      const block = applyBlockReason(driverStatus, profiles);
      if (block) {
        notify({ isOpen: true, title: 'Settings Were Not Applied', message: block });
        throw new Error(block);
      }
      const validationError = validateSettings(profiles, deviceConfig, settingsMetadata.default_device_config);
      if (validationError) {
        notify({ isOpen: true, title: 'Invalid Input', message: validationError });
        throw new Error(validationError);
      }
      try {
        console.log('Applying Settings...', profiles);
        const settingsJson = serializeSettingsDocument(settingsMetadata, profiles, deviceConfig);
        const result = await invoke<SettingsApplyReport>('apply_settings', { settingsJson });
        if (result.applied) setAppliedSnapshot(settingsJson);
        if (result.saved) editor.markSaved(settingsJson);
        const outcome = describeApplyOutcome(result);
        if (outcome.notification) notify(outcome.notification);
        if (outcome.succeeded) console.log('Settings applied and saved successfully.');
        return outcome.succeeded;
      } catch (error) {
        console.error('Failed to apply settings:', error);
        notify({ isOpen: true, title: 'Settings Were Not Applied', message: String(error) });
        throw error;
      }
    } finally {
      setIsApplying(false);
      // A status refresh failure cannot change the Apply/Save outcome.
      void driverStatus.afterApply();
    }
  });
  return { isApplying, applyDisabledReason, applySettings, appliedSnapshot };
}
