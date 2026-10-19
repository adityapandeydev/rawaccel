import { useEffect } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { availableMonitors, getCurrentWindow, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/window';
import { parseWindowState, restoreWindowState, WINDOW_STATE_KEY, type WindowState } from '../windowState';

export function useWindowState() {
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const stops: (() => void)[] = [];
    const win = getCurrentWindow();
    let last: WindowState | null = null;
    let sequence = 0;
    const capture = async () => {
      const revision = ++sequence;
      try {
        if (await win.isMinimized()) return;
        const maximized = await win.isMaximized();
        let state = last;
        if (!maximized) {
          const [position, size] = await Promise.all([win.outerPosition(), win.innerSize()]);
          state = { x: position.x, y: position.y, width: size.width, height: size.height, maximized };
        } else if (state) state = { ...state, maximized };
        if (disposed || revision !== sequence || !state) return;
        last = state;
        localStorage.setItem(WINDOW_STATE_KEY, JSON.stringify(state));
      } catch (error) { console.warn('Could not remember window bounds:', error); }
    };
    const schedule = () => { clearTimeout(timer); timer = setTimeout(() => void capture(), 200); };
    const initialize = async () => {
      try {
        let stored: WindowState | null = null;
        try { stored = parseWindowState(localStorage.getItem(WINDOW_STATE_KEY)); } catch { /* Storage is optional. */ }
        const monitors = await availableMonitors();
        if (disposed) return;
        const areas = monitors.map(monitor => ({ ...monitor.workArea.position, ...monitor.workArea.size, scale: monitor.scaleFactor }));
        const restored = stored && restoreWindowState(stored, areas);
        if (restored) {
          last = restored;
          await win.unmaximize();
          if (disposed) return;
          await win.setSize(new PhysicalSize(restored.width, restored.height));
          await win.setPosition(new PhysicalPosition(restored.x, restored.y));
          if (restored.maximized) await win.maximize();
        } else {
          // The first launch stays maximized. Seed a normal restore rectangle
          // for sessions in which the window never leaves maximized state.
          last = restoreWindowState({ x: 0, y: 0, width: 1280, height: 860, maximized: true }, areas);
        }
        if (disposed) return;
        for (const subscribe of [() => win.onMoved(schedule), () => win.onResized(schedule)]) {
          const stop = await subscribe();
          if (disposed) stop(); else stops.push(stop);
        }
        await capture();
      } catch (error) { console.warn('Could not restore window bounds:', error); }
    };
    void initialize();
    return () => { disposed = true; clearTimeout(timer); stops.forEach(stop => stop()); };
  }, []);
}
