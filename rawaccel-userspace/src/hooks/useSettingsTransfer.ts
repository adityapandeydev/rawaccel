import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SettingsLoadReport } from '../types';
import { serializeSettingsDocument } from '../settingsDocument';
import type { ProfileEditor } from './useProfileEditor';
import type { Notify } from './useNotifications';

export function useSettingsTransfer(editor: ProfileEditor, notify: Notify, confirmReplace: (replace: () => void) => void) {
  const { replaceDocument } = editor;
  const current = serializeSettingsDocument(editor.settingsMetadata, editor.profiles, editor.deviceConfig);
  const latest = useRef({ current, saved: editor.savedSnapshot, confirmReplace });
  latest.current = { current, saved: editor.savedSnapshot, confirmReplace };
  useEffect(() => {
    let active = true;
    const baseline = latest.current;
    async function load() {
      try {
        const result = await invoke<SettingsLoadReport>('load_settings');
        const parsed = JSON.parse(result.settings_json);
        if (!active) return;
        if (latest.current.current !== baseline.current || latest.current.saved !== baseline.saved) {
          notify({ isOpen: true, title: 'Your edits were kept',
            message: `Saved settings finished loading after you started editing. They have not replaced your changes. To load them, use Load and select ${result.source}.` });
          return;
        }
        replaceDocument(parsed);
        if (result.recovered || result.migrated) {
          notify({
            isOpen: true,
            title: result.recovered ? 'Settings Recovered' : 'Settings Upgraded',
            message: `Settings were loaded from ${result.source}.${result.migrated ? " They were upgraded in memory; use Apply Settings to save the upgrade." : ""} Your original files have not been changed.\n${result.warnings.join('\n')}`,
          });
        }
      } catch (error) {
        console.error('Failed to load settings on startup:', error);
        if (active && error !== 'No settings.json found') {
          notify({ isOpen: true, title: 'Settings Could Not Be Loaded', message: String(error) });
        }
      }
    }
    void load();
    return () => { active = false; };
  }, [replaceDocument, notify]);

  const importSettings = async (file: File) => {
    try {
      const text = await file.text();
      const migrated = await invoke<string>('import_settings', { rawJson: text });
      const document = JSON.parse(migrated);
      latest.current.confirmReplace(() => {
        replaceDocument(document);
        notify({ isOpen: true, kind: 'info', title: 'Settings Imported', message: 'Settings loaded into the editor. Use Apply & Save to update the driver and startup settings.' });
      });
    } catch (error) {
      notify({ isOpen: true, title: 'Import Error',
        message: typeof error === 'string' ? error : 'Failed to parse or migrate settings JSON.' });
    }
  };

  const exportSettings = () => {
    const currentData = serializeSettingsDocument(editor.settingsMetadata, editor.profiles, editor.deviceConfig, true);
    const blob = new Blob([currentData], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'settings.json';
    anchor.click();
    URL.revokeObjectURL(url);
  };
  return { importSettings, exportSettings };
}
