export const WINDOW_STATE_KEY = 'rawaccel.window';
export interface WindowState { x: number; y: number; width: number; height: number; maximized: boolean; }
export interface WorkArea { x: number; y: number; width: number; height: number; scale: number; }

export function parseWindowState(raw: string | null): WindowState | null {
  try {
    const state = JSON.parse(raw ?? 'null');
    if (!state || typeof state.maximized !== 'boolean') return null;
    if (!['x', 'y', 'width', 'height'].every(key => Number.isFinite(state[key]) && Math.abs(state[key]) < 100000)) return null;
    if (state.width <= 0 || state.height <= 0) return null;
    return { x: state.x, y: state.y, width: state.width, height: state.height, maximized: state.maximized };
  } catch { return null; }
}

// Keep the title bar reachable after unplugging a display or changing DPI.
export function restoreWindowState(state: WindowState, areas: WorkArea[]): WindowState | null {
  if (!areas.length) return null;
  const area = areas.find(area => state.x >= area.x && state.x < area.x + area.width && state.y >= area.y && state.y < area.y + area.height) ?? areas[0];
  const width = Math.max(1024 * area.scale, Math.min(state.width, area.width));
  const height = Math.max(768 * area.scale, Math.min(state.height, area.height));
  const x = Math.max(area.x, Math.min(state.x, area.x + Math.max(0, area.width - width)));
  const y = Math.max(area.y, Math.min(state.y, area.y + Math.max(0, area.height - height)));
  return { ...state, x, y, width, height };
}
