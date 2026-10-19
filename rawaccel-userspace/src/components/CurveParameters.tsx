import { NumberInput } from './NumberInput';
import { CustomSelect } from './CustomSelect';
import { TieredParameters } from './TieredParameters';
import { ProcessingControl } from './ProcessingControl';
import type { Notify } from '../hooks/useNotifications';
import type { ProfileEditor } from '../hooks/useProfileEditor';
const safeParseFloat = (val: string) => {
  const parsed = parseFloat(val);
  return isNaN(parsed) ? 0 : parsed;
};

const defaultLookupTable = () => [
  { x: 0.1, y: 1.0 },
  { x: 20.0, y: 1.0 },
];

const copyLookupTable = (table: { x: number; y: number }[]) =>
  table.map((point) => ({ ...point }));


export function CurveParameters({ editor, notify: setModalState }: {
  editor: ProfileEditor; notify: Notify;
}) {
  const { activeAccel, updateAccel, updateLookup } = editor;
  return (
    <details className="panel acceleration-panel" open>
      <summary>Acceleration</summary>
      <div className="acceleration-body">
      <div className="mode-row input-group">
        <span className="input-label">Mode</span>
        <div style={{ flex: 1 }}>
          <CustomSelect
            label="Acceleration mode"
            width="100%"
            value={activeAccel.mode}
            onChange={(val) => {
              updateLookup((args) => {
                if (val !== "lookup" || (args.lookup_table?.length ?? 0) >= 2) {
                  return { ...args, mode: val };
                }

                // A newly created LUT starts as a flat sensitivity curve.
                return {
                  ...args,
                  mode: "lookup",
                  gain: false,
                  lookup_table: defaultLookupTable(),
                };
              });
            }}
            options={[
              { value: "linear", label: "Linear" },
              { value: "classic", label: "Classic" },
              { value: "jump", label: "Jump" },
              { value: "natural", label: "Natural" },
              { value: "power", label: "Power" },
              { value: "synchronous", label: "Synchronous" },
              { value: "tiered", label: "Tiered" },
              { value: "lookup", label: "Lookup Table" },
              { value: "noaccel", label: "Off" },
            ]}
          />
        </div>

      </div>
        {/* Shared Gain Checkbox - Beside Mode Selector */}
        {activeAccel.mode !== "noaccel" && activeAccel.mode !== "lookup" && (
          <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", cursor: (activeAccel.mode === "tiered" && (!activeAccel.t_type || activeAccel.t_type === "linear")) ? "not-allowed" : "pointer", whiteSpace: "nowrap", opacity: (activeAccel.mode === "tiered" && (!activeAccel.t_type || activeAccel.t_type === "linear")) ? 0.5 : 1 }}>
            <input
              type="checkbox"
              checked={activeAccel.mode === "tiered" && (!activeAccel.t_type || activeAccel.t_type === "linear") ? false : activeAccel.gain}
              disabled={activeAccel.mode === "tiered" && (!activeAccel.t_type || activeAccel.t_type === "linear")}
              onChange={e => updateAccel("gain", e.target.checked)}
              style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px", margin: 0, cursor: (activeAccel.mode === "tiered" && (!activeAccel.t_type || activeAccel.t_type === "linear")) ? "not-allowed" : "pointer" }}
            />
            <span style={{ fontSize: "0.85rem", fontWeight: 500, color: "var(--text-main)" }}>Gain</span>
          </label>
        )}

      {activeAccel.mode !== "tiered" && <ProcessingControl editor={editor} />}

      <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>

        {/* ───────────────────────────────────────────────────────── */}
        {/* LINEAR MODE                                                 */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "linear" && (
          <>
            {activeAccel.cap_mode !== "io" && (
              <div className="input-group">
                <label className="input-label">Acceleration</label>
                <NumberInput className="input-field selectable" value={activeAccel.acceleration} onChange={e => updateAccel("acceleration", safeParseFloat(e.target.value))} step="0.001" />
              </div>
            )}

            <div className="input-group">
              <label className="input-label">Cap Type</label>
              <CustomSelect
                width="120px"
                value={activeAccel.cap_mode === "io" ? "both" : activeAccel.cap_mode}
                onChange={(val) => updateAccel("cap_mode", val === "both" ? "io" : val)}
                options={[
                  { value: "out", label: "Output" },
                  { value: "in", label: "Input" },
                  { value: "both", label: "Both" }
                ]}
              />
            </div>

            {activeAccel.cap_mode === "in" && (
              <div className="input-group">
                <label className="input-label">Cap: Input</label>
                <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
              </div>
            )}
            {activeAccel.cap_mode === "out" && (
              <div className="input-group">
                <label className="input-label">Cap: Output</label>
                <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
              </div>
            )}
            {activeAccel.cap_mode === "io" && (
              <>
                <div className="input-group">
                  <label className="input-label">Cap: Input</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
                <div className="input-group">
                  <label className="input-label">Cap: Output</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
              </>
            )}

            <div className="input-group">
              <label className="input-label">Input Offset</label>
              <NumberInput className="input-field selectable" value={activeAccel.input_offset} onChange={e => updateAccel("input_offset", safeParseFloat(e.target.value))} step="0.1" />
            </div>
          </>
        )}

        {/* ───────────────────────────────────────────────────────── */}
        {/* CLASSIC MODE                                                */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "classic" && (
          <>
            <div className="input-group">
              <label className="input-label">Cap Type</label>
              <CustomSelect
                width="120px"
                value={activeAccel.cap_mode === "io" ? "both" : activeAccel.cap_mode}
                onChange={(val) => updateAccel("cap_mode", val === "both" ? "io" : val)}
                options={[
                  { value: "out", label: "Output" },
                  { value: "in", label: "Input" },
                  { value: "both", label: "Both" }
                ]}
              />
            </div>

            {activeAccel.cap_mode === "in" && (
              <>
                <div className="input-group">
                  <label className="input-label">Cap: Input</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
                <div className="input-group">
                  <label className="input-label">Acceleration</label>
                  <NumberInput className="input-field selectable" value={activeAccel.acceleration} onChange={e => updateAccel("acceleration", safeParseFloat(e.target.value))} step="0.001" />
                </div>
              </>
            )}
            {activeAccel.cap_mode === "out" && (
              <>
                <div className="input-group">
                  <label className="input-label">Cap: Output</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
                <div className="input-group">
                  <label className="input-label">Acceleration</label>
                  <NumberInput className="input-field selectable" value={activeAccel.acceleration} onChange={e => updateAccel("acceleration", safeParseFloat(e.target.value))} step="0.001" />
                </div>
              </>
            )}
            {activeAccel.cap_mode === "io" && (
              <>
                <div className="input-group">
                  <label className="input-label">Cap: Input</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
                <div className="input-group">
                  <label className="input-label">Cap: Output</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
              </>
            )}

            <div className="input-group">
              <label className="input-label">Input Offset</label>
              <NumberInput className="input-field selectable" value={activeAccel.input_offset} onChange={e => updateAccel("input_offset", safeParseFloat(e.target.value))} step="0.1" />
            </div>

            <div className="input-group">
              <label className="input-label">Power</label>
              <NumberInput className="input-field selectable" value={activeAccel.exponent_classic} onChange={e => updateAccel("exponent_classic", safeParseFloat(e.target.value))} step="0.1" />
            </div>
          </>
        )}

        {/* ───────────────────────────────────────────────────────── */}
        {/* JUMP MODE                                                   */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "jump" && (
          <>
            <div className="input-group">
              <label className="input-label">Smooth</label>
              <NumberInput className="input-field selectable" value={activeAccel.smooth} onChange={e => updateAccel("smooth", safeParseFloat(e.target.value))} step="0.1" />
            </div>
            <div className="input-group">
              <label className="input-label">Input</label>
              <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
            </div>
            <div className="input-group">
              <label className="input-label">Output</label>
              <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
            </div>
          </>
        )}

        {/* ───────────────────────────────────────────────────────── */}
        {/* NATURAL MODE                                                */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "natural" && (
          <>
            <div className="input-group">
              <label className="input-label">Decay Rate</label>
              <NumberInput className="input-field selectable" value={activeAccel.decay_rate} onChange={e => updateAccel("decay_rate", safeParseFloat(e.target.value))} step="0.01" />
            </div>
            <div className="input-group">
              <label className="input-label">Input Offset</label>
              <NumberInput className="input-field selectable" value={activeAccel.input_offset} onChange={e => updateAccel("input_offset", safeParseFloat(e.target.value))} step="0.1" />
            </div>
            <div className="input-group">
              <label className="input-label">Limit</label>
              <NumberInput className="input-field selectable" value={activeAccel.limit} onChange={e => updateAccel("limit", safeParseFloat(e.target.value))} step="0.1" />
            </div>
          </>
        )}

        {/* ───────────────────────────────────────────────────────── */}
        {/* POWER MODE                                                  */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "power" && (
          <>
            {activeAccel.cap_mode !== "io" && (
              <div className="input-group">
                <label className="input-label">Scale</label>
                <NumberInput className="input-field selectable" value={activeAccel.scale} onChange={e => updateAccel("scale", safeParseFloat(e.target.value))} step="0.1" />
              </div>
            )}

            <div className="input-group">
              <label className="input-label">Cap Type</label>
              <CustomSelect
                width="120px"
                value={activeAccel.cap_mode === "io" ? "both" : activeAccel.cap_mode}
                onChange={(val) => updateAccel("cap_mode", val === "both" ? "io" : val)}
                options={[
                  { value: "out", label: "Output" },
                  { value: "in", label: "Input" },
                  { value: "both", label: "Both" }
                ]}
              />
            </div>

            {activeAccel.cap_mode === "in" && (
              <div className="input-group">
                <label className="input-label">Cap: Input</label>
                <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
              </div>
            )}
            {activeAccel.cap_mode === "out" && (
              <div className="input-group">
                <label className="input-label">Cap: Output</label>
                <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
              </div>
            )}
            {activeAccel.cap_mode === "io" && (
              <>
                <div className="input-group">
                  <label className="input-label">Cap: Input</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.x} onChange={e => updateAccel("cap", { ...activeAccel.cap, x: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
                <div className="input-group">
                  <label className="input-label">Cap: Output</label>
                  <NumberInput className="input-field selectable" value={activeAccel.cap.y} onChange={e => updateAccel("cap", { ...activeAccel.cap, y: safeParseFloat(e.target.value) })} step="0.1" />
                </div>
              </>
            )}

            <div className="input-group">
              <label className="input-label">Exponent</label>
              <NumberInput className="input-field selectable" value={activeAccel.exponent_power} onChange={e => updateAccel("exponent_power", safeParseFloat(e.target.value))} step="0.1" />
            </div>

            <div className="input-group">
              <label className="input-label">Output Offset</label>
              <NumberInput className="input-field selectable" value={activeAccel.output_offset} onChange={e => updateAccel("output_offset", safeParseFloat(e.target.value))} step="0.1" />
            </div>
          </>
        )}

        {/* ───────────────────────────────────────────────────────── */}
        {/* SYNCHRONOUS MODE                                            */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "synchronous" && (
          <>
            <div className="input-group">
              <label className="input-label">Gamma</label>
              <NumberInput className="input-field selectable" value={activeAccel.gamma} onChange={e => updateAccel("gamma", safeParseFloat(e.target.value))} step="0.1" />
            </div>
            <div className="input-group">
              <label className="input-label">Smooth</label>
              <NumberInput className="input-field selectable" value={activeAccel.smooth} onChange={e => updateAccel("smooth", safeParseFloat(e.target.value))} step="0.1" />
            </div>
            <div className="input-group">
              <label className="input-label">Motivity</label>
              <NumberInput className="input-field selectable" value={activeAccel.motivity} onChange={e => updateAccel("motivity", safeParseFloat(e.target.value))} step="0.1" />
            </div>
            <div className="input-group">
              <label className="input-label">Sync Speed</label>
              <NumberInput className="input-field selectable" value={activeAccel.sync_speed} onChange={e => updateAccel("sync_speed", safeParseFloat(e.target.value))} step="0.1" />
            </div>
          </>
        )}



        {/* ───────────────────────────────────────────────────────── */}
        {/* LOOKUP TABLE                                                */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "lookup" && (
          <>
            <div className="input-group">
              <label className="input-label">Apply As</label>
              <CustomSelect
                width="120px"
                value={activeAccel.gain ? "velocity" : "sensitivity"}
                onChange={(val) => {
                  const useVelocity = val === "velocity";
                  updateLookup((args) => {
                    if (args.gain === useVelocity) return args;

                    // Preserve the displayed sensitivity curve while changing
                    // the driver's stored representation.
                    const table = (args.lookup_table ?? []).map((point) => ({
                      x: point.x,
                      y: useVelocity ? point.y * point.x : point.y / point.x,
                    }));
                    return { ...args, gain: useVelocity, lookup_table: table };
                  });
                }}
                options={[
                  { value: "sensitivity", label: "Sensitivity" },
                  { value: "velocity", label: "Velocity" }
                ]}
              />
            </div>
            <input
              type="file"
              accept=".csv, .xlsx, .xls"
              id="csv-upload"
              style={{ display: "none" }}
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (!file) return;

                const fileName = file.name.toLowerCase();
                if (fileName.endsWith(".xlsx") || fileName.endsWith(".xls")) {
                  setModalState({
                    isOpen: true,
                    title: "Excel File Detected",
                    message: "Raw Accel requires a plain-text CSV file.\n\nHow to convert your Excel sheet to CSV:\n1. Open your file in Excel or Google Sheets\n2. Click File → Save As (or Download)\n3. Choose 'CSV (Comma delimited) (*.csv)'\n4. Upload the saved .csv file here."
                  });
                  e.target.value = "";
                  return;
                }

                const reader = new FileReader();
                reader.onload = (event) => {
                  const text = event.target?.result as string;
                  const lines = text.split(/\r?\n/).filter(line => line.trim().length > 0);
                  const parsedArray = [] as { x: number; y: number }[];

                  for (const [index, line] of lines.entries()) {
                    const [rawX, rawY] = line.split(",");
                    const x = Number(rawX?.trim());
                    const y = Number(rawY?.trim());
                    if (!rawX?.trim() || !rawY?.trim() || !Number.isFinite(x) || !Number.isFinite(y)) {
                      setModalState({ isOpen: true, title: "Invalid CSV", message: `Line ${index + 1} must contain two finite numbers: input speed, value.` });
                      e.target.value = "";
                      return;
                    }
                    parsedArray.push({ x, y });
                  }

                  parsedArray.sort((a, b) => a.x - b.x);
                  if (parsedArray.length < 2 || parsedArray.length > 257) {
                    setModalState({ isOpen: true, title: "Invalid CSV", message: "A lookup table must contain between 2 and 257 points." });
                  } else if (parsedArray.some((point, index) => point.x <= 0 || (index > 0 && point.x <= parsedArray[index - 1].x))) {
                    setModalState({ isOpen: true, title: "Invalid CSV", message: "Input speeds must be above zero and strictly increasing." });
                  } else {
                    updateLookup((args) => ({
                      ...args,
                      gain: activeAccel.gain,
                      lookup_table: copyLookupTable(parsedArray),
                      lookup_csv_backup: copyLookupTable(parsedArray),
                      lookup_csv_backup_gain: activeAccel.gain,
                    }));
                  }
                  // Reset input
                  e.target.value = "";
                };
                reader.readAsText(file);
              }}
            />
            <button
              className="btn btn-primary"
              style={{ marginTop: "0.5rem", width: "100%" }}
              onClick={() => document.getElementById("csv-upload")?.click()}
            >
              Upload CSV
            </button>
            <button
              className="btn"
              style={{ marginTop: "0.5rem", width: "100%" }}
              disabled={!activeAccel.lookup_csv_backup || activeAccel.lookup_csv_backup.length < 2}
              onClick={() => updateLookup((args) => {
                if (!args.lookup_csv_backup || args.lookup_csv_backup.length < 2) return args;
                return {
                  ...args,
                  lookup_table: copyLookupTable(args.lookup_csv_backup),
                  gain: args.lookup_csv_backup_gain ?? args.gain,
                };
              })}
            >
              Restore CSV Points
            </button>
          </>
        )}

        {/* ───────────────────────────────────────────────────────── */}
        {/* TIERED MODE                                                 */}
        {/* ───────────────────────────────────────────────────────── */}
        {activeAccel.mode === "tiered" && <TieredParameters editor={editor} />}

        {activeAccel.mode === "noaccel" && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", padding: "1rem" }}>
            Acceleration disabled.
          </div>
        )}

      </div>
      </div>
    </details>
  );
}
