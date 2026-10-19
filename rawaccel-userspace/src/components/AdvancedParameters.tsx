import { NumberInput } from './NumberInput';
import type { ProfileEditor } from '../hooks/useProfileEditor';
const safeParseFloat = (val: string) => {
  const parsed = parseFloat(val);
  return isNaN(parsed) ? 0 : parsed;
};


export function AdvancedParameters({ editor }: { editor: ProfileEditor }) {
  const { activeProfile, updateProfile, updateSpeed } = editor;
  return (
    <details className="panel advanced-panel">
      <summary>Advanced</summary>

      <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>

        {/* Anisotropy */}
        <h3 style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "-0.25rem" }}>Anisotropy (X | Y)</h3>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">Domain Weights</label>
          <div style={{ display: "flex", gap: "0.25rem", width: "90px" }}>
            <NumberInput className="input-field selectable" style={{ width: "100%", padding: "0.25rem" }} value={activeProfile.domain_weights.x} onChange={e => updateProfile("domain_weights", { ...activeProfile.domain_weights, x: safeParseFloat(e.target.value) })} step="0.1" />
            <NumberInput className="input-field selectable" style={{ width: "100%", padding: "0.25rem" }} value={activeProfile.domain_weights.y} onChange={e => updateProfile("domain_weights", { ...activeProfile.domain_weights, y: safeParseFloat(e.target.value) })} step="0.1" />
          </div>
        </div>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">Range Weights</label>
          <div style={{ display: "flex", gap: "0.25rem", width: "90px" }}>
            <NumberInput className="input-field selectable" style={{ width: "100%", padding: "0.25rem" }} value={activeProfile.range_weights.x} onChange={e => updateProfile("range_weights", { ...activeProfile.range_weights, x: safeParseFloat(e.target.value) })} step="0.1" />
            <NumberInput className="input-field selectable" style={{ width: "100%", padding: "0.25rem" }} value={activeProfile.range_weights.y} onChange={e => updateProfile("range_weights", { ...activeProfile.range_weights, y: safeParseFloat(e.target.value) })} step="0.1" />
          </div>
        </div>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">LP Norm</label>
          <NumberInput className="input-field selectable" value={activeProfile.speed_processor_args.lp_norm} onChange={e => updateSpeed("lp_norm", safeParseFloat(e.target.value))} step="0.1" />
        </div>

        <div style={{ height: "1px", backgroundColor: "var(--border)", margin: "0.5rem 0" }}></div>

        {/* Coalescion */}
        <h3 style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "-0.25rem" }}>Coalescion (Smooth)</h3>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">Input Half-life</label>
          <NumberInput className="input-field selectable" value={activeProfile.speed_processor_args.input_speed_smooth_halflife} onChange={e => updateSpeed("input_speed_smooth_halflife", safeParseFloat(e.target.value))} step="0.1" />
        </div>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">Scale Half-life</label>
          <NumberInput className="input-field selectable" value={activeProfile.speed_processor_args.scale_smooth_halflife} onChange={e => updateSpeed("scale_smooth_halflife", safeParseFloat(e.target.value))} step="0.1" />
        </div>

        <div style={{ height: "1px", backgroundColor: "var(--border)", margin: "0.5rem 0" }}></div>

        {/* Geometry */}
        <h3 style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "-0.25rem" }}>Geometry</h3>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">Rotation (°)</label>
          <NumberInput className="input-field selectable" value={activeProfile.degrees_rotation} onChange={e => updateProfile("degrees_rotation", safeParseFloat(e.target.value))} step="0.1" />
        </div>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">Angle Snapping</label>
          <NumberInput className="input-field selectable" value={activeProfile.degrees_snap} onChange={e => updateProfile("degrees_snap", safeParseFloat(e.target.value))} step="0.1" />
        </div>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">L/R Ratio</label>
          <NumberInput className="input-field selectable" value={activeProfile.lr_output_dpi_ratio} onChange={e => updateProfile("lr_output_dpi_ratio", safeParseFloat(e.target.value))} step="0.01" />
        </div>
        <div className="input-group" style={{ margin: 0 }}>
          <label className="input-label">U/D Ratio</label>
          <NumberInput className="input-field selectable" value={activeProfile.ud_output_dpi_ratio} onChange={e => updateProfile("ud_output_dpi_ratio", safeParseFloat(e.target.value))} step="0.01" />
        </div>

      </div>
    </details>
  );
}
