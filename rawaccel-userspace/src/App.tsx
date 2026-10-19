import { useDriverStatus } from "./hooks/useDriverStatus";
import { useDevices } from "./hooks/useDevices";
import { applyBlockReason } from "./driverStatus";
import { DriverStatusIndicator } from "./components/DriverStatus";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Moon, Sun, Monitor, Layers, Activity, Plus, Save, Download, Upload, Mouse, Menu, X, Settings, Maximize, Minimize, Check } from "lucide-react";
import "./index.css";
import { Profile, defaultProfile, AccelArgs, DeviceConfig, SettingsLoadReport, SettingsApplyReport, SETTINGS_SCHEMA_VERSION } from "./types";
import { validateSettings } from "./validation";
import { NumberInput } from "./components/NumberInput";
import { CurveGraph } from "./components/CurveGraph";
import { CustomSelect } from "./components/CustomSelect";
import { DevicesView } from "./components/DevicesView";
import { MappingsView } from "./components/MappingsView";
import { SettingsView } from "./components/SettingsView";
import { CustomModal } from "./components/CustomModal";
import { AnimatedButton } from "./components/AnimatedButton";

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

function App() {
  const driverStatus = useDriverStatus();
  const [profiles, setProfiles] = useState<Profile[]>([defaultProfile()]);
  const [activeProfileIndex, setActiveProfileIndex] = useState(0);
  const activeProfile = profiles[activeProfileIndex] || profiles[0];

  const { devices, loading: devicesLoading, error: devicesError, refresh: refreshDevices } = useDevices();
  const [deviceConfig, setDeviceConfig] = useState<DeviceConfig[]>([]);
  const [settingsMetadata, setSettingsMetadata] = useState<Record<string, unknown>>({ schema_version: SETTINGS_SCHEMA_VERSION });
  
  const [activeTab, setActiveTab] = useState<"devices" | "mappings" | "profiles" | "settings">("profiles");
  const [theme, setTheme] = useState<"dark" | "light" | "system">("dark");
  const [language, setLanguage] = useState("en-US");
  const [showToastNotifications, setShowToastNotifications] = useState(true);
  const [showConfirmModals, setShowConfirmModals] = useState(true);
  const [activeCurveAxis, setActiveCurveAxis] = useState<"x" | "y">("x");
  const [linkXY, setLinkXY] = useState(true);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [visibleGraphs, setVisibleGraphs] = useState<string[]>(["sensitivity"]);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isProfilesExpanded, setIsProfilesExpanded] = useState(false);
  const [modalState, setModalState] = useState({ isOpen: false, title: "", message: "" });
  
  const applyDisabledReason = applyBlockReason(driverStatus, profiles);
  const tieredPreview = !driverStatus.report?.tiered_apply_enabled && (activeProfile.accel_x.mode === "tiered" || activeProfile.accel_y.mode === "tiered");

  const mappedDevicesCount = deviceConfig.filter(d => !d.disable && d.profile_id).length;

  const toggleGraph = (graph: string) => {
    if (visibleGraphs.includes(graph)) {
      if (visibleGraphs.length > 1) {
        setVisibleGraphs(visibleGraphs.filter(g => g !== graph));
      }
    } else {
      setVisibleGraphs([...visibleGraphs, graph]);
    }
  };

  useEffect(() => {
    if (theme === "light") {
      document.body.setAttribute("data-theme", "light");
    } else {
      document.body.removeAttribute("data-theme");
    }
  }, [theme]);

  async function applySettings(overrideConfig?: DeviceConfig[]): Promise<boolean> {
    try {
      const block = applyBlockReason(driverStatus, profiles);
      if (block) {
        setModalState({ isOpen: true, title: "Settings Were Not Applied", message: block });
        throw new Error(block);
      }
      const configToUse = overrideConfig || deviceConfig;
      const validationError = validateSettings(profiles, configToUse, settingsMetadata.default_device_config);
      if (validationError) {
        setModalState({ isOpen: true, title: "Invalid Input", message: validationError });
        throw new Error(validationError);
      }

      try {
        console.log("Applying Settings...", profiles);
        const profileJson = JSON.stringify({
          ...settingsMetadata,
          profiles: profiles,
          devices: configToUse
        });
        const result = await invoke<SettingsApplyReport>('apply_settings', { settingsJson: profileJson });
        if (!result.saved) {
          setModalState({
            isOpen: true,
            title: "Applied, But Not Saved",
            message: `Your settings are active in the driver, but could not be saved. They may be lost after restarting.\n${result.save_error ?? "No settings location could be updated."}`,
          });
          return false;
        }
        if (result.warnings.length > 0) {
          setModalState({
            isOpen: true,
            title: "Applied and Saved With Warnings",
            message: `Your settings are active and were saved to:\n${result.saved_paths.join("\n")}\nSome locations could not be updated:\n${result.warnings.join("\n")}`,
          });
        }
        console.log("Settings applied and saved successfully.");
        return true;
      } catch (error) {
        console.error("Failed to apply settings:", error);
        setModalState({ isOpen: true, title: "Settings Were Not Applied", message: String(error) });
        throw error;
      }
    } finally {
      // Status failures never turn a successful Apply/Save into a failed operation.
      void driverStatus.afterApply();
    }
  }

  // Load settings on startup
  useEffect(() => {
    async function initSettings() {
      try {
        const result = await invoke<SettingsLoadReport>('load_settings');
        const parsed = JSON.parse(result.settings_json);
        const metadata = { ...parsed };
        delete metadata.profiles;
        delete metadata.devices;
        setSettingsMetadata(metadata);
        if (result.recovered || result.migrated) {
          setModalState({
            isOpen: true,
            title: result.recovered ? "Settings Recovered" : "Settings Upgraded",
            message: `Settings were loaded from ${result.source}.${result.migrated ? " They were upgraded in memory; use Apply Settings to save the upgrade." : ""} Your original files have not been changed.\n${result.warnings.join("\n")}`,
          });
        }
        if (parsed && parsed.profiles && parsed.profiles.length > 0) {
            console.log("Loaded settings from backend:", parsed.profiles);
            setProfiles(parsed.profiles);
            setActiveProfileIndex(0);
            if (parsed.profiles[0]?.speed_processor_args) {
              setLinkXY(parsed.profiles[0].speed_processor_args.whole);
            }
        }
        if (parsed && parsed.devices) {
            setDeviceConfig(parsed.devices);
        }
      } catch (error) {
        console.error("Failed to load settings or devices on startup:", error);
        if (error !== "No settings.json found") {
          setModalState({ isOpen: true, title: "Settings Could Not Be Loaded", message: String(error) });
        }
      }
    }
    initSettings();
  }, []);

  // State Update Helpers
  const updateActiveProfile = (updater: (p: Profile) => Profile) => {
    setProfiles(prev => prev.map((p, i) => i === activeProfileIndex ? updater(p) : p));
  };

  const updateProfile = (key: keyof Profile, value: any) => {
    updateActiveProfile(p => ({ ...p, [key]: value }));
  };

  const renameProfile = (newName: string) => {
    const oldName = activeProfile.name;
    updateActiveProfile(p => ({ ...p, name: newName }));
    if (oldName && oldName !== newName) {
      setDeviceConfig(prev => prev.map(d => d.profile_id === oldName ? { ...d, profile_id: newName } : d));
    }
  };

  const updateSpeed = (key: string, value: number) => {
    updateActiveProfile(p => ({ ...p, speed_processor_args: { ...p.speed_processor_args, [key]: value } }));
  };

  const updateAccel = (key: keyof AccelArgs, value: any) => {
    updateActiveProfile(p => {
      const next = { ...p };
      if (linkXY) {
        next.accel_x = { ...next.accel_x, [key]: value };
        next.accel_y = { ...next.accel_y, [key]: value };
      } else {
        if (activeCurveAxis === "x") next.accel_x = { ...next.accel_x, [key]: value };
        if (activeCurveAxis === "y") next.accel_y = { ...next.accel_y, [key]: value };
      }
      return next;
    });
  };

  const updateLookup = (updater: (args: AccelArgs) => AccelArgs) => {
    updateActiveProfile((profile) => {
      if (linkXY) {
        return {
          ...profile,
          accel_x: updater(profile.accel_x),
          accel_y: updater(profile.accel_y),
        };
      }

      return activeCurveAxis === "x"
        ? { ...profile, accel_x: updater(profile.accel_x) }
        : { ...profile, accel_y: updater(profile.accel_y) };
    });
  };

  const activeAccel = activeCurveAxis === "x" ? activeProfile.accel_x : activeProfile.accel_y;

  const navigateTo = (tab: typeof activeTab) => {
    setActiveTab(tab);
    setIsSidebarOpen(false);
  };

  const handleLinkXY = () => {
    setLinkXY(true);
    setActiveCurveAxis("x");
    updateActiveProfile(p => ({
      ...p,
      speed_processor_args: { ...p.speed_processor_args, whole: true },
      accel_y: JSON.parse(JSON.stringify(p.accel_x))
    }));
  };

  const handleSelectXAxis = () => {
    setLinkXY(false);
    setActiveCurveAxis("x");
    updateActiveProfile(p => ({
      ...p,
      speed_processor_args: { ...p.speed_processor_args, whole: false }
    }));
  };

  const handleSelectYAxis = () => {
    setLinkXY(false);
    setActiveCurveAxis("y");
    updateActiveProfile(p => ({
      ...p,
      speed_processor_args: { ...p.speed_processor_args, whole: false }
    }));
  };

  return (
    <div className="app-container">
      
      {/* ── OVERLAY SIDEBAR DRAWER ── */}
      {isSidebarOpen && (
        <div style={{ position: "fixed", inset: 0, zIndex: 40 }} onClick={() => setIsSidebarOpen(false)}>
          {/* Overlay Background */}
          <div style={{ position: "absolute", inset: 0, backgroundColor: "rgba(0,0,0,0.4)", backdropFilter: "blur(2px)" }} />
        </div>
      )}
      
      <aside className={`sidebar-drawer ${isSidebarOpen ? "open" : ""}`}>
        <div style={{ padding: "0.5rem 0", borderBottom: "1px solid var(--border)", display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.5rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            <Mouse size={24} color="var(--color-primary)" />
            <h2 style={{ fontSize: "1.2rem", margin: 0, fontWeight: 700, color: "var(--text-main)", marginRight: "0.5rem" }}>Raw Accel</h2>
            
            <button 
              className="btn" 
              onClick={() => setTheme(theme === "dark" ? "light" : "dark")} 
              style={{ padding: "0.25rem", background: "var(--bg-input)", border: "1px solid var(--border)", color: "var(--text-muted)", cursor: "pointer", borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center" }}
              title="Toggle Theme"
            >
              {theme === "dark" ? <Sun size={14} /> : <Moon size={14} />}
            </button>
          </div>
          <button className="btn" style={{ padding: "0.25rem", background: "transparent", border: "none", color: "var(--text-muted)" }} aria-label="Close navigation" onClick={() => setIsSidebarOpen(false)}>
            <X size={20} />
          </button>
        </div>
        
        <nav style={{ display: "flex", flexDirection: "column", flex: 1 }}>
          <span className="input-label" style={{ marginBottom: "0.5rem", paddingLeft: "0.5rem" }}>Navigation</span>
          
          <div>
            <div 
              className={`nav-link ${activeTab === "profiles" ? "active" : ""}`} 
              onClick={() => {
                if (profiles.length <= 1) {
                  navigateTo("profiles");
                } else {
                  setActiveTab("profiles");
                  setIsProfilesExpanded(!isProfilesExpanded);
                }
              }}
              style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}
            >
              <div style={{ display: "flex", alignItems: "center" }}>
                <Activity size={16} style={{ marginRight: "0.75rem" }}/> Profiles
              </div>
              <button 
                className="btn" 
                style={{ padding: "0.2rem", background: "transparent", border: "none", color: "inherit", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center" }} 
                onClick={(e) => {
                  e.stopPropagation();
                  if (profiles.length < 5) {
                    const newProfile = defaultProfile();
                    let nameIndex = profiles.length + 1;
                    let candidateName = `Profile ${nameIndex}`;
                    while (profiles.some(p => p.name.toLowerCase() === candidateName.toLowerCase())) {
                      nameIndex++;
                      candidateName = `Profile ${nameIndex}`;
                    }
                    newProfile.name = candidateName;
                    const nextProfiles = [...profiles, newProfile];
                    setProfiles(nextProfiles);
                    setActiveProfileIndex(nextProfiles.length - 1);
                    setIsProfilesExpanded(true);
                    setActiveTab("profiles");
                  } else {
                    setModalState({ isOpen: true, title: "Profile Limit Reached", message: "Maximum 5 profiles allowed." });
                  }
                }}
                title="Add New Profile"
              >
                <Plus size={16} />
              </button>
            </div>
            
            {isProfilesExpanded && (
              <div style={{ paddingLeft: "2.5rem", display: "flex", flexDirection: "column", gap: "0.25rem", marginTop: "0.25rem", marginBottom: "0.5rem" }}>
                {profiles.map((p, idx) => (
                  <div 
                    key={idx}
                    onClick={() => { 
                      setActiveProfileIndex(idx); 
                      setLinkXY(profiles[idx].speed_processor_args?.whole ?? true);
                      setActiveCurveAxis("x");
                      navigateTo("profiles"); 
                    }}
                    style={{ 
                      padding: "0.5rem", 
                      fontSize: "0.85rem",
                      background: activeProfileIndex === idx ? "var(--color-primary-bg)" : "transparent",
                      color: activeProfileIndex === idx ? "var(--color-primary)" : "var(--text-main)",
                      borderRadius: "var(--radius-sm)",
                      cursor: "pointer",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center"
                    }}
                  >
                    <div style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                      {p.name}
                    </div>
                    {profiles.length > 1 && (
                      <div 
                        onClick={(e) => {
                          e.stopPropagation();
                          const deletedName = profiles[idx].name;
                          const newProfiles = profiles.filter((_, i) => i !== idx);
                          setProfiles(newProfiles);
                          setDeviceConfig(prev => prev.map(d => d.profile_id === deletedName ? { ...d, profile_id: null } : d));
                          if (activeProfileIndex === idx) setActiveProfileIndex(Math.max(0, idx - 1));
                          else if (activeProfileIndex > idx) setActiveProfileIndex(activeProfileIndex - 1);
                        }}
                        style={{ padding: "0.15rem", color: "var(--text-muted)", display: "flex", alignItems: "center", justifyContent: "center" }}
                        title="Delete Profile"
                      >
                        <X size={12} />
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
          <div className={`nav-link ${activeTab === "mappings" ? "active" : ""}`} onClick={() => navigateTo("mappings")}>
            <Layers size={16} style={{ marginRight: "0.75rem" }}/> Mappings
          </div>
          <div className={`nav-link ${activeTab === "devices" ? "active" : ""}`} onClick={() => navigateTo("devices")}>
            <Monitor size={16} style={{ marginRight: "0.75rem" }}/> Devices
          </div>
          
          <div style={{ marginTop: "auto", borderTop: "1px solid var(--border)", paddingTop: "0.5rem" }}>
            <div className={`nav-link ${activeTab === "settings" ? "active" : ""}`} onClick={() => navigateTo("settings")}>
              <Settings size={16} style={{ marginRight: "0.75rem" }}/> Settings
            </div>
          </div>
        </nav>
        
        <div style={{ paddingTop: "1.25rem", borderTop: "1px solid var(--border)", display: "flex", flexDirection: "column", gap: "1rem" }}>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <input
              type="file"
              accept=".json"
              id="import-settings-upload"
              style={{ display: "none" }}
              onChange={async (e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                try {
                  const text = await file.text();
                  const migrated: string = await invoke('import_settings', { rawJson: text });
                  const parsed = JSON.parse(migrated);
                  const metadata = { ...parsed };
                  delete metadata.profiles;
                  delete metadata.devices;
                  setSettingsMetadata(metadata);
                  if (parsed && parsed.profiles && parsed.profiles.length > 0) {
                    setProfiles(parsed.profiles);
                    setActiveProfileIndex(0);
                    if (parsed.profiles[0]?.speed_processor_args) {
                      setLinkXY(parsed.profiles[0].speed_processor_args.whole);
                    }
                  }
                  if (parsed && parsed.devices) {
                    setDeviceConfig(parsed.devices);
                  }
                  setModalState({
                    isOpen: true,
                    title: "Settings Imported",
                    message: "Settings loaded and migrated successfully!"
                  });
                } catch (err: any) {
                  setModalState({
                    isOpen: true,
                    title: "Import Error",
                    message: typeof err === "string" ? err : "Failed to parse or migrate settings JSON."
                  });
                }
                e.target.value = "";
              }}
            />
            <button 
              className="btn" 
              style={{ flex: 1 }}
              onClick={() => document.getElementById("import-settings-upload")?.click()}
            >
              <Upload size={14} /> Load
            </button>
            <button 
              className="btn" 
              style={{ flex: 1 }}
              onClick={() => {
                const currentData = JSON.stringify({ ...settingsMetadata, profiles, devices: deviceConfig }, null, 2);
                const blob = new Blob([currentData], { type: "application/json" });
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = "settings.json";
                a.click();
                URL.revokeObjectURL(url);
              }}
            >
              <Download size={14} /> Save
            </button>
          </div>
        </div>
      </aside>

      {/* ── MAIN CONTENT ── */}
      <main className="main-content" style={{ display: "flex", flexDirection: "column", width: "100%" }}>
        
        {/* Global Header */}
        <header className="app-header" style={{ padding: "1rem 1.5rem", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center", backgroundColor: "var(--bg-app)", zIndex: 5 }}>
          
          <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <button className="btn" style={{ padding: "0.5rem", background: "transparent", border: "none", color: "var(--text-main)", cursor: "pointer", display: "flex", alignItems: "center" }} aria-label="Open navigation" onClick={() => setIsSidebarOpen(true)}>
              <Menu size={22} />
            </button>
            
            <h1 style={{ fontSize: "1.25rem", margin: 0, color: "var(--text-main)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
              {activeTab === "profiles" && (
                <input 
                  type="text" 
                  value={activeProfile.name}
                  onChange={(e) => renameProfile(e.target.value)}
                  style={{ background: "transparent", border: "none", color: "inherit", font: "inherit", outline: "none", width: "200px", padding: 0, margin: 0 }}
                  title="Click to rename profile"
                />
              )}
              {activeTab === "mappings" && "Device Mappings"}
              {activeTab === "devices" && "Devices"}
              {activeTab === "settings" && "Application Settings"}
            </h1>
            {activeTab === "profiles" && (
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                {(() => {
                  const mappedCount = deviceConfig.filter(d => !d.disable && d.profile_id === activeProfile.name).length;
                  if (mappedCount > 0) {
                    return (
                      <span style={{ padding: "0.25rem 0.6rem", backgroundColor: "var(--color-primary-bg)", color: "var(--color-primary)", borderRadius: "var(--radius-sm)", fontSize: "0.75rem", fontWeight: 600 }}>
                        Mapped ({mappedCount} {mappedCount === 1 ? "device" : "devices"})
                      </span>
                    );
                  }
                  return (
                    <span style={{ padding: "0.25rem 0.6rem", backgroundColor: "var(--bg-input)", color: "var(--text-muted)", borderRadius: "var(--radius-sm)", fontSize: "0.75rem", fontWeight: 500, border: "1px solid var(--border)" }}>
                      Unmapped
                    </span>
                  );
                })()}
              </div>
            )}
          </div>

          <div className="app-header-actions" style={{ display: "flex", alignItems: "center", gap: "1.5rem" }}>
            <DriverStatusIndicator snapshot={driverStatus} onRefresh={driverStatus.refresh} />
            {activeTab === "profiles" && (
              <>
                <div style={{ display: "flex", gap: "0.25rem", background: "var(--bg-input)", padding: "0.25rem", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
                  <button className="btn" style={{ padding: "0.25rem 0.75rem", fontSize: "0.75rem", background: visibleGraphs.includes("sensitivity") ? "var(--color-primary)" : "transparent", color: visibleGraphs.includes("sensitivity") ? "#fff" : "var(--text-muted)", border: "none" }} onClick={() => toggleGraph("sensitivity")}>Sensitivity</button>
                  <button className="btn" style={{ padding: "0.25rem 0.75rem", fontSize: "0.75rem", background: visibleGraphs.includes("velocity") ? "var(--color-primary)" : "transparent", color: visibleGraphs.includes("velocity") ? "#fff" : "var(--text-muted)", border: "none" }} onClick={() => toggleGraph("velocity")}>Velocity</button>
                  <button className="btn" style={{ padding: "0.25rem 0.75rem", fontSize: "0.75rem", background: visibleGraphs.includes("gain") ? "var(--color-primary)" : "transparent", color: visibleGraphs.includes("gain") ? "#fff" : "var(--text-muted)", border: "none" }} onClick={() => toggleGraph("gain")}>Gain</button>
                </div>
                
                <button 
                  className="btn"
                  onClick={() => setIsExpanded(!isExpanded)}
                  style={{ padding: "0.25rem", background: "var(--bg-input)", border: "1px solid var(--border)", color: "var(--text-muted)", cursor: "pointer", borderRadius: "var(--radius-sm)", display: "flex", alignItems: "center", justifyContent: "center" }}
                  title={isExpanded ? "Minimize Graph Domain" : "Expand Graph Domain"}
                >
                  {isExpanded ? <Minimize size={18} /> : <Maximize size={18} />}
                </button>

                <div style={{ display: "flex", gap: "0.25rem", background: "var(--bg-input)", padding: "0.25rem", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
                  <button className="btn" style={{ padding: "0.25rem 0.75rem", fontSize: "0.75rem", background: linkXY ? "var(--color-primary)" : "transparent", color: linkXY ? "#fff" : "var(--text-muted)", border: "none" }} onClick={handleLinkXY}>Link X/Y</button>
                  <button className="btn" style={{ padding: "0.25rem 0.75rem", fontSize: "0.75rem", background: !linkXY && activeCurveAxis === "x" ? "var(--color-primary)" : "transparent", color: !linkXY && activeCurveAxis === "x" ? "#fff" : "var(--text-muted)", border: "none" }} onClick={handleSelectXAxis}>X Axis</button>
                  <button className="btn" style={{ padding: "0.25rem 0.75rem", fontSize: "0.75rem", background: !linkXY && activeCurveAxis === "y" ? "var(--color-primary)" : "transparent", color: !linkXY && activeCurveAxis === "y" ? "#fff" : "var(--text-muted)", border: "none" }} onClick={handleSelectYAxis}>Y Axis</button>
                </div>
              </>
            )}

            {activeTab === "profiles" ? (
              <AnimatedButton
                disabled={applyDisabledReason !== null}
                title={applyDisabledReason ?? "Apply settings to the driver"}
                onClick={async () => {
                  if (profiles.length === 1 && mappedDevicesCount <= 1 && deviceConfig.length > 0) {
                    const targetDeviceIndex = deviceConfig.findIndex(d => !d.disable && d.profile_id);
                    const applyIndex = targetDeviceIndex !== -1 ? targetDeviceIndex : 0;
                    const newConfig = [...deviceConfig];
                    newConfig[applyIndex] = { ...newConfig[applyIndex], profile_id: activeProfile.name };
                    setDeviceConfig(newConfig);
                    return await applySettings(newConfig);
                  } else {
                    return await applySettings();
                  }
                }}
                defaultText={<><Save size={16} /> {profiles.length > 1 ? "Save All to Driver" : "Apply Settings"}</>}
                successText={<><Check size={16} /> {profiles.length > 1 ? "Saved!" : "Applied!"}</>}
              />
            ) : (
              activeTab !== "settings" && <AnimatedButton 
                disabled={applyDisabledReason !== null}
                title={applyDisabledReason ?? "Apply settings to the driver"}
                onClick={async () => await applySettings()} 
                defaultText={<><Save size={16} /> Apply Settings</>} 
                successText={<><Check size={16} /> Applied!</>} 
              />
            )}
          </div>
        </header>

        {activeTab === "settings" && (
          <SettingsView 
            driverStatus={driverStatus}
            onRefreshDriver={driverStatus.refresh}
            theme={theme}
            setTheme={setTheme}
            language={language}
            setLanguage={setLanguage}
            showToastNotifications={showToastNotifications}
            setShowToastNotifications={setShowToastNotifications}
            showConfirmModals={showConfirmModals}
            setShowConfirmModals={setShowConfirmModals}
          />
        )}

        {activeTab === "profiles" && (
          <div style={{ display: "grid", gridTemplateColumns: "340px 1fr", padding: "0 1.5rem 1.5rem 1.5rem", gap: "1.5rem", flex: 1, minHeight: 0, overflow: "hidden" }}>
            
            {/* Left Column: Mode Selection, Fields & Advanced Config */}
            <div className="custom-scrollbar" style={{ display: "flex", flexDirection: "column", gap: "1.5rem", height: "100%", overflowY: "auto", paddingRight: "0.5rem", paddingTop: "1.5rem" }}>
              
              {tieredPreview && <div className="tiered-preview-notice" role="note">
                <strong>Tiered · Preview only</strong>
                <p>Edit either submode and explore all graph views. Switch to a supported mode before applying to the stock driver.</p>
              </div>}

              {/* Mode Settings Pane */}
              <div className="panel" style={{ padding: "1.5rem" }}>
                <div className="panel-header" style={{ marginBottom: "1.5rem", display: "flex", alignItems: "center", gap: "1rem" }}>
                  <div style={{ flex: 1 }}>
                    <CustomSelect
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
                        { value: "tiered", label: driverStatus.report?.tiered_apply_enabled ? "Tiered" : "Tiered (Preview)" },
                        { value: "lookup", label: "Lookup Table" },
                        { value: "noaccel", label: "Off" },
                      ]}
                    />
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
                </div>
                
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
                  {activeAccel.mode === "tiered" && (
                    <>
                      <div className="input-group">
                        <label className="input-label">Type</label>
                        <CustomSelect
                          width="120px"
                          value={activeAccel.t_type || "linear"}
                          onChange={(val) => {
                            updateAccel("t_type", val);
                            if (val === "linear") {
                              updateAccel("gain", false);
                            }
                          }}
                          options={[
                            { value: "linear", label: "Linear" },
                            { value: "natural", label: "Natural" }
                          ]}
                        />
                      </div>
                      
                      {/* TIER 1 */}
                      <div className="input-group">
                        <label className="input-label">Tier 1 Multiplier</label>
                        <NumberInput className="input-field selectable" value={activeAccel.tiered_multiplier1} onChange={e => updateAccel("tiered_multiplier1", safeParseFloat(e.target.value))} step="0.1" />
                      </div>
                      <div className="input-group">
                        <label className="input-label">Tier 1 Input Offset</label>
                        <NumberInput className="input-field selectable" value={activeAccel.tiered_input_offset1} onChange={e => {
                          let val = safeParseFloat(e.target.value);
                          const maxOffset1 = activeAccel.tiered_input_offset2 - ((!activeAccel.t_type || activeAccel.t_type === "linear") ? activeAccel.tiered_transition1 : 0);
                          if (val > maxOffset1) val = maxOffset1;
                          if (val < 0) val = 0;
                          updateAccel("tiered_input_offset1", val);
                        }} step="0.1" />
                      </div>
                      {(activeAccel.t_type === "natural") && (
                        <div className="input-group">
                          <label className="input-label">Tier 1 Decay Rate</label>
                          <NumberInput className="input-field selectable" value={activeAccel.tiered_decay_rate1} onChange={e => updateAccel("tiered_decay_rate1", safeParseFloat(e.target.value))} step="0.01" />
                        </div>
                      )}

                      {/* TIER 2 */}
                      <div className="input-group">
                        <label className="input-label">Tier 2 Multiplier</label>
                        <NumberInput className="input-field selectable" value={activeAccel.tiered_multiplier2} onChange={e => updateAccel("tiered_multiplier2", safeParseFloat(e.target.value))} step="0.1" />
                      </div>
                      {((activeAccel.t_type || "linear") === "linear") && (
                        <div className="input-group">
                          <label className="input-label">Tier 2 Transition</label>
                          <NumberInput className="input-field selectable" value={activeAccel.tiered_transition1} onChange={e => {
                            let val = safeParseFloat(e.target.value);
                            const maxTransition = activeAccel.tiered_input_offset2 - activeAccel.tiered_input_offset1;
                            if (val > maxTransition) val = maxTransition;
                            if (val < 0) val = 0;
                            updateAccel("tiered_transition1", val);
                          }} step="0.1" />
                        </div>
                      )}
                      <div className="input-group">
                        <label className="input-label">Tier 2 Input Offset</label>
                        <NumberInput className="input-field selectable" value={activeAccel.tiered_input_offset2} onChange={e => {
                          let val = safeParseFloat(e.target.value);
                          const minOffset2 = activeAccel.tiered_input_offset1 + ((!activeAccel.t_type || activeAccel.t_type === "linear") ? activeAccel.tiered_transition1 : 0);
                          if (val < minOffset2) val = minOffset2;
                          updateAccel("tiered_input_offset2", val);
                        }} step="0.1" />
                      </div>
                      {(activeAccel.t_type === "natural") && (
                        <div className="input-group">
                          <label className="input-label">Tier 2 Decay Rate</label>
                          <NumberInput className="input-field selectable" value={activeAccel.tiered_decay_rate2} onChange={e => updateAccel("tiered_decay_rate2", safeParseFloat(e.target.value))} step="0.01" />
                        </div>
                      )}

                      {/* TIER 3 */}
                      <div className="input-group">
                        <label className="input-label">Tier 3 Multiplier</label>
                        <NumberInput className="input-field selectable" value={activeAccel.tiered_multiplier3} onChange={e => updateAccel("tiered_multiplier3", safeParseFloat(e.target.value))} step="0.1" />
                      </div>
                      {((activeAccel.t_type || "linear") === "linear") && (
                        <div className="input-group">
                          <label className="input-label">Tier 3 Transition</label>
                          <NumberInput className="input-field selectable" value={activeAccel.tiered_transition2} onChange={e => updateAccel("tiered_transition2", safeParseFloat(e.target.value))} step="0.1" />
                        </div>
                      )}
                    </>
                  )}

                  {activeAccel.mode === "noaccel" && (
                    <div style={{ textAlign: "center", color: "var(--text-muted)", padding: "1rem" }}>
                      Acceleration disabled.
                    </div>
                  )}

                </div>
              </div>

              {/* Advanced Parameters Panel */}
              <div className="panel" style={{ padding: "1.5rem" }}>
                <div className="panel-header" style={{ marginBottom: "1rem", borderBottom: "none", paddingBottom: 0 }}>Advanced Configuration</div>
                
                <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
                  
                  {/* General */}
                  <h3 style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", marginTop: "0.5rem", marginBottom: "-0.25rem" }}>General</h3>
                  <div className="input-group" style={{ margin: 0 }}>
                    <label className="input-label">Sens Multiplier</label>
                    <NumberInput className="input-field selectable" value={activeProfile.yx_output_dpi_ratio} onChange={e => updateProfile("yx_output_dpi_ratio", safeParseFloat(e.target.value))} step="0.01" />
                  </div>
                  <div className="input-group" style={{ margin: 0 }}>
                    <label className="input-label">Output DPI</label>
                    <NumberInput className="input-field selectable" value={activeProfile.output_dpi} onChange={e => updateProfile("output_dpi", safeParseFloat(e.target.value))} step="1" />
                  </div>

                  <div style={{ height: "1px", backgroundColor: "var(--border)", margin: "0.5rem 0" }}></div>

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
              </div>
              
            </div>
            
            <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden", paddingTop: "0.25rem" }}>
              <CurveGraph 
                argsX={activeProfile.accel_x} 
                argsY={activeProfile.accel_y} 
                visibleGraphs={visibleGraphs} 
                isExpanded={isExpanded} 
                onUpdateLookupPoint={(axis, newTable) => {
                  if (linkXY) {
                    updateActiveProfile(p => ({
                      ...p,
                      accel_x: { ...p.accel_x, lookup_table: copyLookupTable(newTable) },
                      accel_y: { ...p.accel_y, lookup_table: copyLookupTable(newTable) },
                    }));
                  } else {
                    const prop = axis === "x" ? "accel_x" : "accel_y";
                    updateProfile(prop, {
                      ...activeProfile[prop],
                      lookup_table: newTable
                    });
                  }
                }}
              />
            </div>
            
          </div>
        )}
        
        {activeTab === "devices" && <DevicesView devices={devices} deviceConfig={deviceConfig} setDeviceConfig={setDeviceConfig} loading={devicesLoading} error={devicesError} onRefresh={refreshDevices} />}
        {activeTab === "mappings" && <MappingsView 
            devices={devices} 
            deviceConfig={deviceConfig} 
            setDeviceConfig={setDeviceConfig}
            profiles={profiles.map(p => p.name)} 
        />}
        
      </main>
      <CustomModal 
        isOpen={modalState.isOpen} 
        title={modalState.title} 
        message={modalState.message} 
        onClose={() => setModalState({ ...modalState, isOpen: false })} 
      />
    </div>
  );
}

export default App;

