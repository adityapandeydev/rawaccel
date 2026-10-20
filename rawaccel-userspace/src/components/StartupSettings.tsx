import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Power } from 'lucide-react';

interface StartupInfo {
  enabled: boolean | null;
  registration_error: string | null;
  settings_path: string;
  mirror_path: string | null;
  executable_path: string;
  portable: boolean;
  last_result: {
    restored: boolean;
    timestamp_secs: number;
    settings_path: string;
    attempts: number;
    error: string | null;
  } | null;
  diagnostic_error: string | null;
}

export function StartupSettings() {
  const [info, setInfo] = useState<StartupInfo | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    invoke<StartupInfo>('get_startup_info')
      .then(result => { if (active) setInfo(result); })
      .catch(error => { if (active) setError(String(error)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, []);

  const update = async (enabled?: boolean) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      if (enabled !== undefined) await invoke('set_startup_enabled', { enabled });
      const result = await invoke<StartupInfo>('get_startup_info');
      setInfo(result);
      if (enabled !== undefined && result.enabled !== enabled) {
        setError('Windows did not confirm the requested startup state. Check Startup apps in Windows Settings.');
      }
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const openFolder = async () => {
    try { await invoke('open_settings_folder'); }
    catch (error) { setError(String(error)); }
  };
  const last = info?.last_result;

  return <section className="startup-settings" aria-labelledby="startup-heading" aria-busy={busy}>
    <h3 id="startup-heading"><Power size={18} /> Startup & storage</h3>
    <label className="settings-checkbox">
      <input type="checkbox" checked={info?.enabled === true} disabled={busy || info?.enabled == null}
        onChange={event => void update(event.target.checked)} aria-describedby="startup-description" />
      Restore settings at sign-in
    </label>
    <p id="startup-description" className="settings-help">Restores saved settings without opening a window. Use Apply & Save to update what will be restored. Unsaved edits and Tiered previews are not sent to the stock driver.</p>
    {(error || info?.registration_error) && <p className="inline-notice" role="alert">{error ?? info?.registration_error}</p>}
    <div className="settings-actions">
      <button className="btn" disabled={busy} onClick={() => void update()}>{busy ? 'Checking…' : 'Refresh status'}</button>
      {info?.enabled && <button className="btn" disabled={busy} onClick={() => void update(true)}>Update startup path</button>}
    </div>
    {info && <>
      <dl className="settings-paths">
        <dt>Storage mode</dt><dd>{info.portable ? 'Portable · portable.txt present' : 'Installed · Local AppData'}</dd>
        <dt>Authoritative settings</dt><dd>{info.settings_path}</dd>
        <dt>Recovery mirror</dt><dd>{info.mirror_path ?? 'Not available'}</dd>
        <dt>This executable</dt><dd>{info.executable_path}</dd>
      </dl>
      <button className="btn" onClick={() => void openFolder()}>Open settings folder</button>
      <p className="settings-help">Custom install folders are supported. After moving a portable copy, update its startup path here. Add portable.txt beside the executable to use adjacent settings; restart RawAccel after changing storage mode.</p>
      <div className="startup-result" role="status">
        <strong>{last ? last.restored ? 'Last startup restoration succeeded' : 'Last startup restoration failed' : 'No startup result recorded'}</strong>
        {last && <><span>{new Date(last.timestamp_secs * 1000).toLocaleString()}</span><span>{last.settings_path}</span>
          {last.error && <span>{last.error}</span>}
          <span className="settings-help">This historical result does not verify the driver’s current settings.</span></>}
        {info.diagnostic_error && <span>{info.diagnostic_error}</span>}
      </div>
    </>}
    <p className="settings-help">Disable an existing writer.exe startup shortcut before enabling this option, so two startup entries do not overwrite each other’s settings. For development builds, startup points to the development executable; configure it again from your final installation.</p>
  </section>;
}
