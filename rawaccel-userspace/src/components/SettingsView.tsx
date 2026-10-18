import { DriverStatusCard } from "./DriverStatus";
import type { DriverStatusSnapshot } from "../driverStatus";
import React from "react";
import { Globe, Bell, Info } from "lucide-react";
import { CustomSelect } from "./CustomSelect";
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';

interface SettingsViewProps {
  driverStatus: DriverStatusSnapshot;
  onRefreshDriver: () => Promise<void>;
  theme: "dark" | "light" | "system";
  setTheme: (theme: "dark" | "light" | "system") => void;
  language: string;
  setLanguage: (lang: string) => void;
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
  language,
  setLanguage,
  showToastNotifications,
  setShowToastNotifications,
  showConfirmModals,
  setShowConfirmModals
}) => {
  const [runOnStartup, setRunOnStartup] = React.useState(false);

  React.useEffect(() => {
    isEnabled().then(setRunOnStartup).catch(console.error);
  }, []);

  const handleStartupToggle = async (checked: boolean) => {
    try {
      if (checked) {
        await enable();
      } else {
        await disable();
      }
      setRunOnStartup(await isEnabled());
    } catch (e) {
      console.error("Failed to toggle startup:", e);
    }
  };
  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", padding: "1.5rem" }}>
      

      <div style={{ flex: 1, overflowY: "auto", padding: "1.5rem", display: "flex", flexDirection: "column", gap: "2rem" }}>
        
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
                value={language}
                onChange={setLanguage}
                options={[
                  { value: "en-US", label: "English (US)" },
                  { value: "ja-JP", label: "Japanese (日本語)" }
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
            <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer" }}>
              <input 
                type="checkbox" 
                checked={showConfirmModals} 
                onChange={e => setShowConfirmModals(e.target.checked)} 
                style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} 
              />
              <span style={{ fontSize: "0.95rem", color: "var(--text-main)" }}>Show Confirmation Modals</span>
            </label>
          </div>
        </section>

        <div style={{ height: "1px", backgroundColor: "var(--border)" }} />

        {/* System Settings */}
        <section>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1rem" }}>
            <Globe size={18} color="var(--text-muted)" />
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 600 }}>System</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", paddingLeft: "1.5rem" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "0.75rem", cursor: "pointer" }}>
              <input 
                type="checkbox" 
                checked={runOnStartup} 
                onChange={e => handleStartupToggle(e.target.checked)} 
                style={{ accentColor: "var(--color-primary)", width: "16px", height: "16px" }} 
              />
              <span style={{ fontSize: "0.95rem", color: "var(--text-main)" }}>Run on Startup</span>
            </label>
          </div>
        </section>

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


