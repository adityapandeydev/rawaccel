import { Device, DeviceConfig } from "../types";
import type { Dispatch, SetStateAction } from 'react';
import { patchDevice } from '../deviceState';
import { Monitor, Activity } from "lucide-react";
import { CustomSelect } from "./CustomSelect";

export function MappingsView({ 
  devices, 
  deviceConfig, 
  setDeviceConfig,
  profiles 
}: { 
  devices: Device[], 
  deviceConfig: DeviceConfig[], 
  setDeviceConfig: Dispatch<SetStateAction<DeviceConfig[]>>,
  profiles: string[]
}) {
  const assignProfile = (deviceId: string, profileName: string | null) => {
    setDeviceConfig(previous => patchDevice(previous, deviceId, { profile_id: profileName }));
  };

  const profileOptions = [
    { value: "", label: "-- No Profile (Raw Input) --" },
    ...profiles.map(p => ({ value: p, label: p }))
  ];

  return (
    <div style={{ padding: "1.5rem", height: "100%", width: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
        <div>
          <h2 style={{ fontSize: "1.1rem", margin: 0, fontWeight: 600, color: "var(--text-main)" }}>Hardware Device Bindings</h2>
          <p style={{ color: "var(--text-muted)", fontSize: "0.85rem", marginTop: "0.25rem" }}>
            Enable and bind specific acceleration profiles to individual physical mouse devices.
          </p>
        </div>
      </div>

      {devices.length === 0 ? (
        <div className="panel" style={{ padding: "3rem 1.5rem", textAlign: "center" }}>
          <Monitor size={48} style={{ margin: "0 auto 1rem auto", color: "var(--text-muted)", opacity: 0.5 }} />
          <h3 style={{ margin: "0 0 0.5rem 0", color: "var(--text-main)" }}>No Devices Detected</h3>
          <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.85rem" }}>
            No mouse hardware devices were found. Connect a mouse to bind acceleration profiles.
          </p>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          {devices.map(device => {
            const config = deviceConfig.find(c => c.id === device.id);
            const isMapped = !!config?.profile_id;
            const isIgnored = !!config?.disable;
            
            return (
              <div 
                key={device.id} 
                className="panel" 
                style={{ 
                  display: "flex", 
                  alignItems: "center", 
                  gap: "2rem", 
                  padding: "1.5rem", 
                  borderLeft: isMapped ? "4px solid var(--color-primary)" : "4px solid transparent",
                  opacity: isIgnored ? 0.6 : 1
                }}
              >
                
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <input 
                    type="checkbox" 
                    checked={isMapped}
                    onChange={(e) => {
                      if (e.target.checked && profiles.length > 0) {
                        assignProfile(device.id, profiles[0]);
                      } else {
                        assignProfile(device.id, null);
                      }
                    }}
                    style={{ width: "1.2rem", height: "1.2rem", accentColor: "var(--color-primary)", cursor: "pointer" }}
                    title="Enable mapping for this device"
                  />
                </div>

                <div style={{ flex: 1, display: "flex", alignItems: "flex-start", gap: "1rem" }}>
                  <div style={{ padding: "0.75rem", backgroundColor: "var(--bg-app)", borderRadius: "var(--radius-md)", color: isMapped ? "var(--color-primary)" : "var(--text-muted)" }}>
                    <Monitor size={24} />
                  </div>
                  <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                      <h3 style={{ fontSize: "1rem", margin: 0 }}>{device.name}</h3>
                      {isIgnored && (
                        <span style={{ fontSize: "0.7rem", padding: "0.15rem 0.5rem", borderRadius: "var(--radius-sm)", backgroundColor: "rgba(239, 68, 68, 0.15)", color: "#ef4444", fontWeight: 600 }}>
                          Ignored in Devices
                        </span>
                      )}
                    </div>
                    <span className="mono" style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>{device.id}</span>
                  </div>
                </div>

                <div style={{ fontSize: "1.5rem", color: "var(--border)" }}>→</div>

                <div style={{ flex: 1 }}>
                  <div className="input-group" style={{ margin: 0 }}>
                    <label className="input-label" style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                      <Activity size={14} /> Assigned Profile
                    </label>
                    <CustomSelect
                      width="100%"
                      value={config?.profile_id || ""}
                      onChange={(val) => assignProfile(device.id, val === "" ? null : String(val))}
                      options={profileOptions}
                    />
                  </div>
                </div>

              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}


