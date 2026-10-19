import { NumberInput } from './NumberInput';
import type { ProfileEditor } from '../hooks/useProfileEditor';

export function SensitivityParameters({ editor }: { editor: ProfileEditor }) {
  return <details className="panel sensitivity-panel"><summary>Sensitivity &amp; DPI</summary><div className="accordion-fields">
    <div className="input-group"><label className="input-label">Sens Multiplier</label>
      <NumberInput className="input-field" value={editor.activeProfile.yx_output_dpi_ratio} step="0.01" onChange={event => editor.updateProfile('yx_output_dpi_ratio', parseFloat(event.target.value) || 0)} /></div>
    <div className="input-group"><label className="input-label">Output DPI</label>
      <NumberInput className="input-field" value={editor.activeProfile.output_dpi} step="1" onChange={event => editor.updateProfile('output_dpi', parseFloat(event.target.value) || 0)} /></div>
  </div></details>;
}
