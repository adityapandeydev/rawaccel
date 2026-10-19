import { ProfileWorkspace } from "./components/ProfileWorkspace";
import { useApplySettings } from "./hooks/useApplySettings";
import { useDriverStatus } from "./hooks/useDriverStatus";
import { useDevices } from "./hooks/useDevices";
import { useProfileEditor } from "./hooks/useProfileEditor";
import { useSettingsTransfer } from "./hooks/useSettingsTransfer";
import { SettingsTransfer } from "./components/SettingsTransfer";
import { DriverStatusIndicator } from "./components/DriverStatus";
import { usePreferences } from "./hooks/usePreferences";
import { useNotifications } from "./hooks/useNotifications";
import { useState } from "react";
import { Moon, Sun, Monitor, Layers, Activity, Plus, Save, Mouse, Menu, X, Settings, Maximize, Minimize, Check } from "lucide-react";
import "./index.css";
import { DevicesView } from "./components/DevicesView";
import { MappingsView } from "./components/MappingsView";
import { SettingsView } from "./components/SettingsView";
import { CustomModal } from "./components/CustomModal";
import { AnimatedButton } from "./components/AnimatedButton";

function App() {
  const driverStatus = useDriverStatus();
  const { devices, loading: devicesLoading, error: devicesError, refresh: refreshDevices } = useDevices();
  const [activeTab, setActiveTab] = useState<"devices" | "mappings" | "profiles" | "settings">("profiles");
  const { theme, setTheme, language, setLanguage, showToastNotifications,
    setShowToastNotifications, showConfirmModals, setShowConfirmModals } = usePreferences();
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [visibleGraphs, setVisibleGraphs] = useState<string[]>(["sensitivity"]);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isProfilesExpanded, setIsProfilesExpanded] = useState(false);
  const { notification: modalState, notify: setModalState, dismiss: dismissNotification } = useNotifications();
  const editor = useProfileEditor(setModalState);
  const { profiles, activeProfileIndex, activeProfile,
    deviceConfig, setDeviceConfig, activeCurveAxis,
    linkXY, renameProfile, handleLinkXY, handleSelectXAxis, handleSelectYAxis } = editor;
  const transfer = useSettingsTransfer(editor, setModalState);
  
  const { isApplying, applyDisabledReason, applySettings } = useApplySettings(editor, driverStatus, setModalState);

  const toggleGraph = (graph: string) => {
    if (visibleGraphs.includes(graph)) {
      if (visibleGraphs.length > 1) {
        setVisibleGraphs(visibleGraphs.filter(g => g !== graph));
      }
    } else {
      setVisibleGraphs([...visibleGraphs, graph]);
    }
  };

  const navigateTo = (tab: typeof activeTab) => {
    setActiveTab(tab);
    setIsSidebarOpen(false);
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
                  if (editor.addProfile()) {
                    setIsProfilesExpanded(true);
                    setActiveTab("profiles");
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
                      editor.selectProfile(idx);
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
                          editor.deleteProfile(idx);
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
          <SettingsTransfer {...transfer} />
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
                disabled={isApplying || applyDisabledReason !== null}
                title={applyDisabledReason ?? "Apply settings to the driver"}
                onClick={applySettings}
                defaultText={<><Save size={16} /> {profiles.length > 1 ? "Save All to Driver" : "Apply Settings"}</>}
                successText={<><Check size={16} /> {profiles.length > 1 ? "Saved!" : "Applied!"}</>}
              />
            ) : (
              activeTab !== "settings" && <AnimatedButton 
                disabled={isApplying || applyDisabledReason !== null}
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

        {activeTab === "profiles" && <ProfileWorkspace editor={editor}
          tieredApplyEnabled={driverStatus.report?.tiered_apply_enabled ?? false}
          visibleGraphs={visibleGraphs} isExpanded={isExpanded} notify={setModalState} />}

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
        onClose={dismissNotification}
      />
    </div>
  );
}

export default App;

