import { useRef } from 'react';
import { Download, Upload } from 'lucide-react';

export function SettingsTransfer({ importSettings, exportSettings }: {
  importSettings: (file: File) => Promise<void>;
  exportSettings: () => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  return (
    <div style={{ display: 'flex', gap: '0.5rem' }}>
      <input type="file" accept=".json" id="import-settings-upload" ref={input} style={{ display: 'none' }}
        onChange={async event => {
          const control = event.currentTarget;
          const file = control.files?.[0];
          if (!file) return;
          await importSettings(file);
          control.value = '';
        }} />
      <button className="btn" style={{ flex: 1 }} onClick={() => input.current?.click()}>
        <Upload size={14} /> Load
      </button>
      <button className="btn" style={{ flex: 1 }} onClick={exportSettings}>
        <Download size={14} /> Save
      </button>
    </div>
  );
}
