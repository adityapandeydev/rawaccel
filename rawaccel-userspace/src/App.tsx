import { useEffect, useRef, useState } from 'react';
import { FolderOpen, Layers, Plus, Mouse, Menu, X, Settings, Moon, Sun, Check, ChevronDown, Undo2, RotateCcw } from 'lucide-react';
import { ProfileWorkspace } from './components/ProfileWorkspace';
import { DevicesView } from './components/DevicesView';
import { MappingsView } from './components/MappingsView';
import { SettingsView } from './components/SettingsView';
import { SettingsTransfer } from './components/SettingsTransfer';
import { DriverStatusIndicator } from './components/DriverStatus';
import { LiveInput } from './components/MouseTelemetry';
import { CustomModal } from './components/CustomModal';
import { AnimatedButton } from './components/AnimatedButton';
import { useApplySettings } from './hooks/useApplySettings';
import { useDriverStatus } from './hooks/useDriverStatus';
import { useDevices } from './hooks/useDevices';
import { useProfileEditor } from './hooks/useProfileEditor';
import { useSettingsTransfer } from './hooks/useSettingsTransfer';
import { usePreferences } from './hooks/usePreferences';
import { useNotifications } from './hooks/useNotifications';
import { useNavigation } from './hooks/useNavigation';
import { useWindowState } from './hooks/useWindowState';
import { serializeSettingsDocument } from './settingsDocument';
import { validateSettings } from './validation';
import { defaultProfile } from './types';
import { documentStatus } from './documentStatus';
import './index.css';
import './workspace.css';

type Page = 'profiles' | 'devices' | 'mappings' | 'settings';
type Confirmation = { title: string; message: string; action: () => void; label: string };

function App() {
  useWindowState();
  const driverStatus = useDriverStatus();
  const { devices, loading, error, refresh } = useDevices();
  const preferences = usePreferences();
  const { notification, notify, dismiss } = useNotifications();
  const editor = useProfileEditor(notify);
  const transfer = useSettingsTransfer(editor, notify);
  const apply = useApplySettings(editor, driverStatus, notify);
  const navigation = useNavigation();
  const [page, setPage] = useState<Page>('profiles');
  const [profilesExpanded, setProfilesExpanded] = useState(true);
  const [graphs, setGraphs] = useState(['sensitivity']);
  const [expanded, setExpanded] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const sidebar = useRef<HTMLElement>(null);
  const menu = useRef<HTMLButtonElement>(null);
  const current = serializeSettingsDocument(editor.settingsMetadata, editor.profiles, editor.deviceConfig);
  const status = documentStatus(current, editor.savedSnapshot, editor.snapshotOrigin, apply.appliedSnapshot);
  const validationError = validateSettings(editor.profiles, editor.deviceConfig, editor.settingsMetadata.default_device_config);
  const disabledReason = validationError ?? apply.applyDisabledReason;
  const tieredPreview = !driverStatus.report?.tiered_apply_enabled && [editor.activeProfile.accel_x, editor.activeProfile.accel_y].some(args => args.mode === 'tiered');
  const navigate = (next: Page) => { setPage(next); navigation.closeDrawer(); };

  useEffect(() => {
    if (!navigation.open || navigation.wide) return;
    sidebar.current?.querySelector<HTMLButtonElement>('button')?.focus();
    const key = (event: KeyboardEvent) => {
      if (document.querySelector('.modal-overlay')) return;
      if (event.key === 'Escape') { event.preventDefault(); navigation.closeDrawer(); }
      if (event.key !== 'Tab') return;
      const targets = Array.from(sidebar.current?.querySelectorAll<HTMLElement>('button, input, select') ?? [])
        .filter(element => !element.hasAttribute('disabled') && element.getClientRects().length > 0);
      const index = targets.indexOf(document.activeElement as HTMLElement);
      event.preventDefault();
      targets[(index + (event.shiftKey ? -1 : 1) + targets.length) % targets.length]?.focus();
    };
    document.addEventListener('keydown', key);
    return () => { document.removeEventListener('keydown', key); menu.current?.focus(); };
  }, [navigation.open, navigation.wide]);

  const reset = () => setConfirmation({ title: 'Reset this profile?', label: 'Reset profile',
    message: 'Restore the active profile’s curve and advanced parameters to their defaults. Its name and device mappings will stay in place. Apply & save when you are ready.',
    action: () => editor.updateActiveProfile(profile => ({ ...defaultProfile(profile.name), id: profile.id })) });
  const revert = () => setConfirmation({ title: 'Revert unsaved edits?', label: 'Revert edits',
    message: 'Restore all profiles and device settings to the last document loaded or saved in this session. This does not send anything to the driver.',
    action: () => {
      if (!editor.savedSnapshot) return;
      const snapshot = editor.savedSnapshot;
      const saved = editor.snapshotOrigin === 'saved';
      editor.replaceDocument(JSON.parse(snapshot));
      if (saved) editor.markSaved(snapshot);
    } });

  return <div className={`app-container ${navigation.open ? 'nav-open' : 'nav-closed'}`}>
    {!navigation.wide && navigation.open && <div className="navigation-backdrop" onClick={navigation.closeDrawer} />}
    {navigation.open && <aside ref={sidebar} className="app-sidebar" aria-label="Navigation"
      role={navigation.wide ? undefined : 'dialog'} aria-modal={navigation.wide ? undefined : true}>
      <div className="brand"><Mouse size={34} strokeWidth={1.5} /><strong>Raw Accel</strong>
        <button className="btn icon-button" aria-label="Close navigation" onClick={navigation.toggle}><X size={16} /></button>
      </div>
      <nav className="navigation" aria-label="Main navigation">
        {([{ id: 'profiles', title: 'Profiles', Icon: FolderOpen }, { id: 'devices', title: 'Devices', Icon: Mouse },
          { id: 'mappings', title: 'Mappings', Icon: Layers }, { id: 'settings', title: 'Settings', Icon: Settings }] as const).map(({id,title,Icon}) =>
          <button key={id} className={`nav-link ${page === id ? 'active' : ''}`} aria-current={page === id ? 'page' : undefined} onClick={() => navigate(id)}><Icon size={23} strokeWidth={1.5} />{title}</button>)}
        <div className="profile-nav-heading"><button className="profile-list-toggle" aria-expanded={profilesExpanded} aria-label="Toggle profile list" onClick={() => setProfilesExpanded(value => !value)}>Profiles <ChevronDown size={13} /></button>
          <button className="btn icon-button" title="Add New Profile" onClick={() => { if (editor.addProfile()) { setProfilesExpanded(true); navigate('profiles'); } }}><Plus size={19} /></button>
        </div>
        {profilesExpanded && <div className="profile-list">
          {editor.profiles.map((profile, index) => <div key={profile.id} className={`profile-item ${editor.activeProfileIndex === index ? 'selected' : ''}`}>
            <button onClick={() => { editor.selectProfile(index); navigate('profiles'); }}
              aria-current={page === 'profiles' && editor.activeProfileIndex === index ? 'page' : undefined} title={profile.name}>{profile.name}</button>
            {editor.profiles.length > 1 && <button className="delete-profile" title={`Delete ${profile.name}`} onClick={() => setConfirmation({
              title: `Delete ${profile.name}?`, label: 'Delete profile', message: 'Remove this profile and clear its device mappings. Apply & save when you are ready.', action: () => editor.deleteProfile(index),
            })}><X size={13} /></button>}
          </div>)}
        </div>}
        <div className="configuration-import"><SettingsTransfer {...transfer} variant="import" /></div>
      </nav>
      <div className="sidebar-footer"><DriverStatusIndicator snapshot={driverStatus} onRefresh={driverStatus.refresh} />
        <button className="btn icon-button theme-button" aria-label={`Switch to ${preferences.resolvedTheme === 'dark' ? 'light' : 'dark'} theme`}
          title={`Switch to ${preferences.resolvedTheme === 'dark' ? 'light' : 'dark'} theme`}
          onClick={() => preferences.setTheme(preferences.resolvedTheme === 'dark' ? 'light' : 'dark')}>
          {preferences.resolvedTheme === 'dark' ? <Sun size={19} /> : <Moon size={19} />}</button></div>
    </aside>}
    <main className="main-content" inert={!navigation.wide && navigation.open ? true : undefined}>
      <header className="workspace-header">
        <div className="page-heading">
          <button ref={menu} className="btn icon-button" aria-label={navigation.open ? 'Collapse navigation' : 'Open navigation'} aria-expanded={navigation.open} onClick={navigation.toggle}><Menu size={20} /></button>
          <div><h1>{page === 'profiles' ? <input className="profile-name" aria-label="Profile name" title="Rename profile" value={editor.activeProfile.name} onChange={event => editor.renameProfile(event.target.value)} />
              : {devices: 'Devices', mappings: 'Device mappings', settings: 'Settings'}[page]}</h1>
            <p className="page-subtitle">{page === 'profiles' ? editor.activeAccel.mode === 'tiered' ? 'Two-stage acceleration' : editor.activeAccel.mode === 'noaccel' ? 'Unmodified mouse input' : `${editor.activeAccel.mode === 'lookup' ? 'Lookup table' : editor.activeAccel.mode[0].toUpperCase() + editor.activeAccel.mode.slice(1)} acceleration`
              : {devices: 'Configure your mouse hardware', mappings: 'Connect your profiles to your devices', settings: 'Appearance, preferences & application information'}[page]}
              {page === 'profiles' && tieredPreview && <span className="preview-badge" role="note" title="Applying Tiered requires a compatible signed driver.">Preview only</span>}</p></div>
        </div>
        <div className="header-actions"><LiveInput whole={editor.linkXY} /></div>
      </header>
      {page !== 'settings' && validationError && <div className="inline-notice" role="alert">{validationError}</div>}
      {page === 'profiles' && <ProfileWorkspace editor={editor} visibleGraphs={graphs} isExpanded={expanded} notify={notify}
        toggleGraph={graph => setGraphs(previous => previous.includes(graph) ? previous.length > 1 ? previous.filter(value => value !== graph) : previous : [...previous, graph])}
        toggleExpanded={() => setExpanded(value => !value)} devices={devices} devicesLoading={loading} onMappings={() => navigate('mappings')} />}
      {page === 'devices' && <DevicesView devices={devices} deviceConfig={editor.deviceConfig} setDeviceConfig={editor.setDeviceConfig} loading={loading} error={error} onRefresh={refresh} />}
      {page === 'mappings' && <MappingsView devices={devices} deviceConfig={editor.deviceConfig} setDeviceConfig={editor.setDeviceConfig} profiles={editor.profiles.map(profile => profile.name)} />}
      {page === 'settings' && <SettingsView driverStatus={driverStatus} onRefreshDriver={driverStatus.refresh} {...preferences} />}
      {page !== 'settings' && <footer className="action-bar">
        <div className="document-status"><span className={`state-dot ${status.edited ? 'edited' : ''}`} /><div><strong className={status.edited ? 'unsaved-label' : ''}>{status.disk}</strong><span className="driver-document-state">{status.driver}</span></div></div>
        <div className="action-buttons">
          {status.edited && editor.savedSnapshot && <button className="btn quiet" title="Revert unsaved edits" onClick={revert}><Undo2 size={15} /><span>Revert</span></button>}
          {page === 'profiles' && <button className="btn" onClick={reset}><RotateCcw size={15} />Reset</button>}
          <SettingsTransfer {...transfer} variant="export" />
          <AnimatedButton disabled={apply.isApplying || disabledReason !== null} title={disabledReason ?? 'Apply all profiles and device settings, then save to disk'} onClick={apply.applySettings}
            defaultText={apply.isApplying ? 'Applying…' : 'Apply & Save'} successText={<><Check size={16} />Applied & saved</>} />
        </div>
      </footer>}
    </main>
    <CustomModal isOpen={confirmation !== null || notification.isOpen} title={confirmation?.title ?? notification.title} message={confirmation?.message ?? notification.message}
      onClose={() => { if (confirmation) setConfirmation(null); else dismiss(); }} onConfirm={confirmation?.action} confirmLabel={confirmation?.label} />
  </div>;
}
export default App;
