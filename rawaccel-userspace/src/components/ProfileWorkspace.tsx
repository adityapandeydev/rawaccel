import { CurveGraph } from './CurveGraph';
import { CurveParameters } from './CurveParameters';
import { AdvancedParameters } from './AdvancedParameters';
import { SensitivityParameters } from './SensitivityParameters';
import { WorkspaceToolbar } from './WorkspaceToolbar';
import { Mouse, ChevronRight } from 'lucide-react';
import type { Device } from '../types';
import type { Notify } from '../hooks/useNotifications';
import type { ProfileEditor } from '../hooks/useProfileEditor';
const copyLookupTable = (table: { x: number; y: number }[]) =>
  table.map((point) => ({ ...point }));


export function ProfileWorkspace({ editor, visibleGraphs, isExpanded, notify, toggleGraph, toggleExpanded, devices, devicesLoading, onMappings }: {
  editor: ProfileEditor; visibleGraphs: string[];
  isExpanded: boolean; notify: Notify;
  toggleGraph: (graph: string) => void; toggleExpanded: () => void; devices: Device[]; devicesLoading: boolean; onMappings: () => void;
}) {
  const { activeProfile, linkXY, updateActiveProfile, updateProfile } = editor;
  const assigned = editor.deviceConfig.filter(device => !device.disable && device.profile_id === activeProfile.name);
  return (
    <div className="profile-workspace">

      {/* Left Column: Mode Selection, Fields & Advanced Config */}
      <div className="profile-inspector custom-scrollbar">

        {/* Mode Settings Pane */}
        <CurveParameters editor={editor} notify={notify} />
        <SensitivityParameters editor={editor} />
        <AdvancedParameters editor={editor} />

      </div>

      <div className="preview-column">
      <section className="panel curve-preview">
        <WorkspaceToolbar editor={editor} visibleGraphs={visibleGraphs} toggleGraph={toggleGraph} expanded={isExpanded} toggleExpanded={toggleExpanded} />
      <div className="workspace-graphs">
        <CurveGraph
          whole={activeProfile.speed_processor_args.whole}
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
      </section>
      <section className="panel assigned-devices"><h2>Assigned devices</h2><div className="assigned-device-content">
        <Mouse size={32} strokeWidth={1.4} /><div className="assigned-device-list">
          {assigned.length ? assigned.map(device => <div key={device.id} className="assigned-device"><strong>{devices.find(item => item.id === device.id)?.name ?? (devicesLoading ? 'Discovering devices…' : 'Disconnected device')}</strong>
            <span>{device.polling_rate ? `${device.polling_rate} Hz` : 'Default polling rate'} · {device.dpi ? `${device.dpi} DPI` : 'Default DPI'}</span></div>)
            : <div className="assigned-device"><strong>No devices assigned</strong><span>Choose which mice use this profile.</span></div>}
        </div><button className="btn quiet" onClick={onMappings}>Manage mappings <ChevronRight size={18} /></button>
      </div></section>
      </div>

    </div>
  );
}
