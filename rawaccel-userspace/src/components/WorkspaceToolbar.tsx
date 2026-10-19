import { Link, Unlink, Maximize, Minimize } from 'lucide-react';
import type { ProfileEditor } from '../hooks/useProfileEditor';

export function WorkspaceToolbar({ editor, visibleGraphs, toggleGraph, expanded, toggleExpanded }: {
  editor: ProfileEditor; visibleGraphs: string[]; toggleGraph: (graph: string) => void;
  expanded: boolean; toggleExpanded: () => void;
}) {
  return <div className="preview-heading"><h2>Curve preview</h2><div className="preview-controls">
    <div className="segmented graph-toggles" aria-label="Visible graphs">
      {['sensitivity', 'velocity', 'gain'].map(graph => <button key={graph} className="segment"
        aria-pressed={visibleGraphs.includes(graph)} onClick={() => toggleGraph(graph)}>
        {graph[0].toUpperCase() + graph.slice(1)}
      </button>)}
    </div>
    <div className="axis-control">
      <button className="btn quiet" aria-pressed={editor.linkXY} title="Link X/Y curves" onClick={editor.handleLinkXY}>
        {editor.linkXY ? <Link size={18} /> : <Unlink size={18} />}<span>{editor.linkXY ? 'X / Y linked' : 'Link X / Y'}</span>
      </button>
      {!editor.linkXY && <div className="segmented" aria-label="Curve editing axes">
        <button className="segment" aria-pressed={editor.activeCurveAxis === 'x'} onClick={editor.handleSelectXAxis}>X Axis</button>
        <button className="segment" aria-pressed={editor.activeCurveAxis === 'y'} onClick={editor.handleSelectYAxis}>Y Axis</button>
      </div>}
    </div>
    <button className="btn icon-button" aria-pressed={expanded} title={expanded ? 'Minimize Graph Domain' : 'Expand Graph Domain'} onClick={toggleExpanded}>
      {expanded ? <Minimize size={16} /> : <Maximize size={16} />}
    </button>
  </div></div>;
}
