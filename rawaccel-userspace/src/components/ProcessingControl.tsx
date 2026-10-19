import type { ProfileEditor } from '../hooks/useProfileEditor';

export function ProcessingControl({ editor }: { editor: ProfileEditor }) {
  return <div className="input-group processing-control">
    <span className="input-label">Apply to</span>
    <div className="segmented" aria-label="Input processing">
      <button className="segment" aria-pressed={editor.linkXY} onClick={editor.handleLinkXY}>Combined</button>
      <button className="segment" aria-pressed={!editor.linkXY} onClick={() => {
        if (editor.linkXY) editor.handleSelectXAxis();
      }}>By component</button>
    </div>
  </div>;
}
