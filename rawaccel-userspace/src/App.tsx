import { useEffect, useRef, useState } from 'react';
import { Activity, Layers, Plus, Save, Mouse, Menu, X, Settings, Monitor, Sun, Moon, Check, ChevronDown, Undo2 } from 'lucide-react';
import { ProfileWorkspace } from './components/ProfileWorkspace';
import { WorkspaceToolbar } from './components/WorkspaceToolbar';
import { DevicesView } from './components/DevicesView';
import { MappingsView } from './components/MappingsView';
import { SettingsView } from './components/SettingsView';
import { SettingsTransfer } from './components/SettingsTransfer';
import { DriverStatusIndicator } from './components/DriverStatus';
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
      <div className="brand"><Mouse size={25} /><div><strong>Raw Accel</strong><span>Make motion yours.</span></div>
        <button className="btn icon-button" aria-label="Close navigation" onClick={navigation.toggle}><X size={17} /></button>
      </div>
      <nav className="navigation" aria-label="Main navigation">
        <span className="eyebrow">Workspace</span>
        <div className="profile-nav-heading">
          <button className={`nav-link ${page === 'profiles' ? 'active' : ''}`} aria-expanded={profilesExpanded}
            onClick={() => { setPage('profiles'); setProfilesExpanded(value => !value); }}>
            <Activity size={18} /> Profiles <ChevronDown size={14} /></button>
          <button className="btn icon-button" title="Add New Profile" onClick={() => {
            if (editor.addProfile()) { setProfilesExpanded(true); navigate('profiles'); }
          }}><Plus size={16} /></button>
        </div>
        {profilesExpanded && <div className="profile-list">
          {editor.profiles.map((profile, index) => <div key={profile.id} className={`profile-item ${editor.activeProfileIndex === index ? 'selected' : ''}`}>
            <button onClick={() => { editor.selectProfile(index); navigate('profiles'); }}
              aria-current={page === 'profiles' && editor.activeProfileIndex === index ? 'page' : undefined} title={profile.name}>
              <span className="profile-dot" />{profile.name}</button>
            {editor.profiles.length > 1 && <button className="delete-profile" title={`Delete ${profile.name}`} onClick={() => setConfirmation({
              title: `Delete ${profile.name}?`, label: 'Delete profile', message: 'Remove this profile and clear its device mappings. Apply & save when you are ready.',
              action: () => editor.deleteProfile(index),
            })}><X size={13} /></button>}
          </div>)}
          <span className="profile-count">{editor.profiles.length} of 5 profiles</span>
        </div>}
        {([{ id: 'mappings', title: 'Mappings', Icon: Layers }, { id: 'devices', title: 'Devices', Icon: Monitor }] as const).map(({id,title,Icon}) =>
          <button key={id} className={`nav-link ${page === id ? 'active' : ''}`} aria-current={page === id ? 'page' : undefined} onClick={() => navigate(id)}><Icon size={18} />{title}</button>)}
        <div className="nav-bottom">
          <button className={`nav-link ${page === 'settings' ? 'active' : ''}`} aria-current={page === 'settings' ? 'page' : undefined} onClick={() => navigate('settings')}><Settings size={18} />Settings</button>
        </div>
      </nav>
      <div className="transfer-area"><span className="eyebrow">Your configuration</span><SettingsTransfer {...transfer} /></div>
      <div className="sidebar-footer"><span>Raw Accel · Userspace</span><button className="btn icon-button" title="Toggle Theme"
        onClick={() => preferences.setTheme(document.body.dataset.theme === 'dark' ? 'light' : 'dark')}>
        {preferences.theme === 'light' ? <Moon size={16} /> : <Sun size={16} />}</button></div>
    </aside>}
    <main className="main-content" inert={!navigation.wide && navigation.open ? true : undefined}>
      <header className="workspace-header">
        <div className="page-heading">
          <button ref={menu} className="btn icon-button" aria-label={navigation.open ? 'Collapse navigation' : 'Open navigation'} aria-expanded={navigation.open} onClick={navigation.toggle}><Menu size={20} /></button>
          <div><span className="eyebrow">{page === 'profiles' ? 'Curve workspace' : 'Configuration'}</span>
            <h1>{page === 'profiles' ? <input className="profile-name" aria-label="Profile name" title="Rename profile" value={editor.activeProfile.name} onChange={event => editor.renameProfile(event.target.value)} />
              : {devices: 'Devices', mappings: 'Device mappings', settings: 'Application settings'}[page]}</h1></div>
        </div>
        <div className="header-actions"><DriverStatusIndicator snapshot={driverStatus} onRefresh={driverStatus.refresh} />
          {page !== 'settings' && <AnimatedButton disabled={apply.isApplying || disabledReason !== null} title={disabledReason ?? 'Apply all profiles and device settings, then save to disk'}
            onClick={apply.applySettings} defaultText={<><Save size={16} />{apply.isApplying ? 'Applying…' : 'Apply & save'}</>} successText={<><Check size={16} />Applied & saved</>} />}
        </div>
      </header>
      {page !== 'settings' && <div className="document-status">
        <span className={`state-dot ${status.edited ? 'edited' : ''}`} /><span>{status.disk}</span>
        <span className="status-divider">/</span><span>{status.driver}</span>
        {page === 'profiles' && <span className="mapping-status">{editor.deviceConfig.filter(device => !device.disable && device.profile_id === editor.activeProfile.name).length} mapped devices</span>}
        {status.edited && editor.savedSnapshot && <button className="btn quiet" onClick={revert}><Undo2 size={14} />Revert edits</button>}
      </div>}
      {page !== 'settings' && disabledReason && <div className="inline-notice" role={validationError ? 'alert' : 'note'}>{disabledReason}</div>}
      {page === 'profiles' && <><WorkspaceToolbar editor={editor} visibleGraphs={graphs} toggleGraph={graph => setGraphs(previous => previous.includes(graph) ? previous.length > 1 ? previous.filter(value => value !== graph) : previous : [...previous, graph])}
        expanded={expanded} toggleExpanded={() => setExpanded(value => !value)} onReset={reset} />
        <ProfileWorkspace editor={editor} tieredApplyEnabled={driverStatus.report?.tiered_apply_enabled ?? false} visibleGraphs={graphs} isExpanded={expanded} notify={notify} /></>}
      {page === 'devices' && <DevicesView devices={devices} deviceConfig={editor.deviceConfig} setDeviceConfig={editor.setDeviceConfig} loading={loading} error={error} onRefresh={refresh} />}
      {page === 'mappings' && <MappingsView devices={devices} deviceConfig={editor.deviceConfig} setDeviceConfig={editor.setDeviceConfig} profiles={editor.profiles.map(profile => profile.name)} />}
      {page === 'settings' && <SettingsView driverStatus={driverStatus} onRefreshDriver={driverStatus.refresh} {...preferences} />}
    </main>
    <CustomModal isOpen={confirmation !== null || notification.isOpen} title={confirmation?.title ?? notification.title} message={confirmation?.message ?? notification.message}
      onClose={() => { if (confirmation) setConfirmation(null); else dismiss(); }} onConfirm={confirmation?.action} confirmLabel={confirmation?.label} />
  </div>;
}
export default App;
