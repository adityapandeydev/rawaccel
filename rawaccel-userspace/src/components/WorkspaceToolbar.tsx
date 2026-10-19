import { Maximize, Minimize, RotateCcw } from 'lucide-react';
import type { ProfileEditor } from '../hooks/useProfileEditor';

export function WorkspaceToolbar({ editor, visibleGraphs, toggleGraph, expanded, toggleExpanded, onReset }: {
  editor: ProfileEditor; visibleGraphs: string[]; toggleGraph: (graph: string) => void;
  expanded: boolean; toggleExpanded: () => void; onReset: () => void;
}) {
  return <div className="workspace-toolbar">
    <div className="segmented graph-toggles" aria-label="Visible graphs">
      {['sensitivity', 'velocity', 'gain'].map(graph => <button key={graph} className="segment"
        aria-pressed={visibleGraphs.includes(graph)} onClick={() => toggleGraph(graph)}>
        <span className="graph-key" style={{ background: `var(--color-${graph})` }} />{graph[0].toUpperCase() + graph.slice(1)}
      </button>)}
    </div>
    <div className="workspace-tools">
      <div className="segmented" aria-label="Curve editing axes">
        <button className="segment" aria-pressed={editor.linkXY} onClick={editor.handleLinkXY}>Link X/Y</button>
        <button className="segment" aria-pressed={!editor.linkXY && editor.activeCurveAxis === 'x'} onClick={editor.handleSelectXAxis}>X Axis</button>
        <button className="segment" aria-pressed={!editor.linkXY && editor.activeCurveAxis === 'y'} onClick={editor.handleSelectYAxis}>Y Axis</button>
      </div>
      <button className="btn icon-button" aria-pressed={expanded} title={expanded ? 'Minimize Graph Domain' : 'Expand Graph Domain'} onClick={toggleExpanded}>
        {expanded ? <Minimize size={16} /> : <Maximize size={16} />}
      </button>
      <button className="btn quiet" onClick={onReset} title="Reset active profile parameters"><RotateCcw size={15} /> Reset profile</button>
    </div>
  </div>;
}
