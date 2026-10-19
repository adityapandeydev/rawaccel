import { NumberInput } from "./NumberInput";
import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { defaultDeviceConfig, patchDevice, resolveDeviceSelection } from "../deviceState";
import { Device, DeviceConfig } from "../types";
import { Monitor } from "lucide-react";

export function DevicesView({ 
  devices,
  deviceConfig,
  setDeviceConfig, loading, error, onRefresh
}: { 
  devices: Device[],
  deviceConfig: DeviceConfig[],
  setDeviceConfig: Dispatch<SetStateAction<DeviceConfig[]>>,
  loading: boolean, error: string | null, onRefresh: () => Promise<void>
}) {
  const [requestedDeviceId, setSelectedDeviceId] = useState<string | null>(null);
  const selectedDeviceId = resolveDeviceSelection(requestedDeviceId, devices);
  useEffect(() => {
    // Commit fallback selection so reordering/reconnecting devices cannot jump
    // the editor back to a previously selected, disconnected mouse.
    setSelectedDeviceId(current => resolveDeviceSelection(current, devices));
  }, [devices]);

  const selectedDevice = devices.find(d => d.id === selectedDeviceId);
  const selectedConfig = deviceConfig.find(c => c.id === selectedDeviceId)
    ?? (selectedDeviceId ? defaultDeviceConfig(selectedDeviceId) : undefined);

  const updateConfig = <K extends keyof DeviceConfig>(key: K, value: DeviceConfig[K]) => {
    if (!selectedDeviceId) return;
    
    setDeviceConfig(previous => patchDevice(previous, selectedDeviceId, { [key]: value }));
  };

  return (
    <div className="page devices-page">
      
      {/* Device List */}
      <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
        <div className="panel" style={{ flex: 1, display: "flex", flexDirection: "column", padding: "1.5rem 0 0 0", overflow: "hidden" }}>
          <div style={{ padding: "0 1.5rem" }}>
            <div className="panel-header" style={{ marginBottom: "0.5rem" }}>
              System Devices
            </div>
            <button className="btn" disabled={loading} onClick={() => void onRefresh()}>{loading ? "Refreshing…" : "Refresh Devices"}</button>
            {error && <p role="alert">Could not refresh devices: {error}. Previous results are retained.</p>}
            {!loading && devices.length === 0 && <p>No mouse devices detected. Connect a mouse and refresh.</p>}
          </div>
          
          <div style={{ flex: 1, padding: "0.5rem", overflowY: "auto" }}>
            <ul className="sidebar-list">
              {devices.map(d => {
                const isSelected = selectedDeviceId === d.id;
                return (
                  <li key={d.id}><button type="button"
                    aria-pressed={isSelected}
                    className={`device-list-item ${isSelected ? "active" : ""}`}
                    onClick={() => setSelectedDeviceId(d.id)}
                    style={{ 
                      padding: "0.75rem 1rem", 
                      display: "flex", 
                      alignItems: "center", 
                      gap: "0.75rem",
                      borderRadius: "var(--radius-md)",
                      cursor: "pointer",
                      backgroundColor: isSelected ? "var(--color-primary-bg)" : "transparent",
                      color: isSelected ? "var(--color-primary)" : "var(--text-main)",
                      fontWeight: isSelected ? 600 : 500,
                      borderLeft: isSelected ? "3px solid var(--color-primary)" : "3px solid transparent",
                      transition: "all 0.15s ease",
                      marginBottom: "0.25rem"
                    }}
                  >
                    <Monitor size={18} style={{ color: isSelected ? "var(--color-primary)" : "var(--text-muted)", flexShrink: 0 }} />
                    <span style={{ fontSize: "0.9rem", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                      {d.name}
                    </span>
                  </button></li>
                );
              })}
            </ul>
          </div>
        </div>
      </div>

      {/* Device Settings */}
      {selectedDevice && selectedConfig ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem", overflowY: "auto", paddingRight: "1rem" }}>
          <div className="panel">
            <div className="panel-header" style={{ marginBottom: "0.5rem" }}>
              {selectedDevice.name}
            </div>
            <p className="mono" style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginBottom: "1.5rem" }}>ID: {selectedDevice.id}</p>

            <div className="device-fields">
              
              {/* Left Settings */}
              <div>
                <div className="input-group">
                  <label className="input-label">Hardware DPI</label>
                  <NumberInput className="input-field selectable" value={selectedConfig.dpi || 0} onChange={e => updateConfig("dpi", parseInt(e.target.value) || 0)} />
                </div>
                <div className="input-group">
                  <label className="input-label">Polling Rate (Hz)</label>
                  <NumberInput className="input-field selectable" value={selectedConfig.polling_rate || 0} onChange={e => updateConfig("polling_rate", parseInt(e.target.value) || 0)} />
                </div>
                
                <div style={{ marginTop: "2rem", display: "flex", flexDirection: "column", gap: "1rem" }}>
                  <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer", fontSize: "0.9rem" }}>
                    <input type="checkbox" checked={selectedConfig.disable} onChange={e => updateConfig("disable", e.target.checked)} style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} />
                    Ignore Device (Disable Accel)
                  </label>
                  <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer", fontSize: "0.9rem" }}>
                    <input type="checkbox" checked={selectedConfig.set_extra_info} onChange={e => updateConfig("set_extra_info", e.target.checked)} style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} />
                    Set Extra Info (Driver flag)
                  </label>
                  <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer", fontSize: "0.9rem" }}>
                    <input type="checkbox" checked={selectedConfig.poll_time_lock} onChange={e => updateConfig("poll_time_lock", e.target.checked)} style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} />
                    Poll Time Lock
                  </label>
                </div>
              </div>

              {/* Right Settings */}
              <div>
                <h4 style={{ fontSize: "0.85rem", color: "var(--text-muted)", marginBottom: "1rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Time Clamping</h4>
                <div className="input-group">
                  <label className="input-label">Min Time Clamp</label>
                  <NumberInput className="input-field selectable" value={selectedConfig.clamp_min || 0} onChange={e => updateConfig("clamp_min", parseFloat(e.target.value) || 0)} step="0.1" />
                </div>
                <div className="input-group">
                  <label className="input-label">Max Time Clamp</label>
                  <NumberInput className="input-field selectable" value={selectedConfig.clamp_max || 0} onChange={e => updateConfig("clamp_max", parseFloat(e.target.value) || 0)} step="0.1" />
                </div>
                <p style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "1rem", lineHeight: 1.5 }}>
                  Time clamping sets limits on the delta-time between polls. Useful for fixing anomalous polling rate spikes that cause acceleration jumps.
                </p>
              </div>

            </div>
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", height: "100%" }}>
          Select a device to view and edit its configuration.
        </div>
      )}
    </div>
  );
}

