import { DriverStatusCard } from "./DriverStatus";
import type { DriverStatusSnapshot } from "../driverStatus";
import React from "react";
import { Globe, Bell, Info } from "lucide-react";
import { CustomSelect } from "./CustomSelect";
import { StartupSettings } from './StartupSettings';

interface SettingsViewProps {
  driverStatus: DriverStatusSnapshot;
  onRefreshDriver: () => Promise<void>;
  theme: "dark" | "light" | "system";
  setTheme: (theme: "dark" | "light" | "system") => void;
  showToastNotifications: boolean;
  setShowToastNotifications: (val: boolean) => void;
  showConfirmModals: boolean;
  setShowConfirmModals: (val: boolean) => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  driverStatus,
  onRefreshDriver,
  theme,
  setTheme,
  showToastNotifications,
  setShowToastNotifications,
  showConfirmModals,
  setShowConfirmModals
}) => {
  return (
    <div className="page settings-page">
      

      <div className="settings-grid">
        
        {/* General Settings */}
        <section>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1rem" }}>
            <Globe size={18} color="var(--text-muted)" />
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 600 }}>General</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", paddingLeft: "1.5rem" }}>
            <div className="input-group">
              <label className="input-label">Language</label>
              <CustomSelect
                width="200px"
                value="en-US"
                onChange={() => {}}
                options={[
                  { value: "en-US", label: "English (US)" },
                  { value: "ja-JP", label: "Japanese (日本語) — planned", disabled: true }
                ]}
              />
            </div>
            
            <div className="input-group">
              <label className="input-label">Theme</label>
              <CustomSelect
                width="200px"
                value={theme}
                onChange={(val) => setTheme(val as any)}
                options={[
                  { value: "system", label: "System Default" },
                  { value: "dark", label: "Dark" },
                  { value: "light", label: "Light" }
                ]}
              />
            </div>
          </div>
        </section>

        <div style={{ height: "1px", backgroundColor: "var(--border)" }} />

        {/* Notifications */}
        <section>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1rem" }}>
            <Bell size={18} color="var(--text-muted)" />
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 600 }}>Notifications</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", paddingLeft: "1.5rem" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer" }}>
              <input 
                type="checkbox" 
                checked={showToastNotifications} 
                onChange={e => setShowToastNotifications(e.target.checked)} 
                style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} 
              />
              <span style={{ fontSize: "0.95rem", color: "var(--text-main)" }}>Show Toast Notifications</span>
            </label>
            <p className="settings-help">Informational updates only. Errors and save warnings remain visible.</p>
            <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer" }}>
              <input 
                type="checkbox" 
                checked={showConfirmModals} 
                onChange={e => setShowConfirmModals(e.target.checked)} 
                style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} 
              />
              <span style={{ fontSize: "0.95rem", color: "var(--text-main)" }}>Confirm profile actions</span>
            </label>
            <p className="settings-help">Ask before resetting, reverting, or deleting profiles.</p>
          </div>
        </section>

        <div style={{ height: "1px", backgroundColor: "var(--border)" }} />

        <StartupSettings />

        <div style={{ height: "1px", backgroundColor: "var(--border)" }} />

        {/* Support */}
        <section>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1rem" }}>
            <Info size={18} color="var(--text-muted)" />
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 600 }}>Support & About</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", paddingLeft: "1.5rem" }}>
            <DriverStatusCard snapshot={driverStatus} onRefresh={onRefreshDriver} />
            <a href="https://github.com/a1xd/rawaccel" target="_blank" rel="noreferrer" style={{ color: "var(--color-primary)", textDecoration: "none", fontSize: "0.95rem" }}>
              GitHub Repository
            </a>
            <a href="https://discord.gg/Q3EebmD" target="_blank" rel="noreferrer" style={{ color: "var(--color-primary)", textDecoration: "none", fontSize: "0.95rem" }}>
              Join the Discord
            </a>
            <a href="https://github.com/a1xd/rawaccel/blob/master/doc/Guide.md" target="_blank" rel="noreferrer" style={{ color: "var(--color-primary)", textDecoration: "none", fontSize: "0.95rem" }}>
              User Guide
            </a>
          </div>
        </section>

      </div>
    </div>
  );
};


