import { CurveGraph } from './CurveGraph';
import { CurveParameters } from './CurveParameters';
import { AdvancedParameters } from './AdvancedParameters';
import type { Notify } from '../hooks/useNotifications';
import type { ProfileEditor } from '../hooks/useProfileEditor';
const copyLookupTable = (table: { x: number; y: number }[]) =>
  table.map((point) => ({ ...point }));


export function ProfileWorkspace({ editor, tieredApplyEnabled, visibleGraphs, isExpanded, notify }: {
  editor: ProfileEditor; tieredApplyEnabled: boolean; visibleGraphs: string[];
  isExpanded: boolean; notify: Notify;
}) {
  const { activeProfile, linkXY, updateActiveProfile, updateProfile } = editor;
  const tieredPreview = !tieredApplyEnabled && (activeProfile.accel_x.mode === 'tiered' || activeProfile.accel_y.mode === 'tiered');
  return (
    <div className="profile-workspace">

      {/* Left Column: Mode Selection, Fields & Advanced Config */}
      <div className="profile-inspector custom-scrollbar">

        {tieredPreview && <div className="tiered-preview-notice" role="note">
          <strong>Tiered · Preview only</strong>
          <p>Edit either submode and explore all graph views. Switch to a supported mode before applying to the stock driver.</p>
        </div>}

        {/* Mode Settings Pane */}
        <CurveParameters editor={editor} tieredApplyEnabled={tieredApplyEnabled} notify={notify} />
        <AdvancedParameters editor={editor} />

      </div>

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

    </div>
  );
}
