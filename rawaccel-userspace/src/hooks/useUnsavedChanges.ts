import { useCallback, useEffect, useRef } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Notify } from './useNotifications';

export function useUnsavedChanges(dirty: boolean, busy: boolean,
  ask: (discard: () => void, reason: 'close' | 'replace') => void, notify: Notify) {
  const current = useRef({ dirty, busy, ask });
  current.current = { dirty, busy, ask };
  const closing = useRef(false);
  const reportBusy = useCallback(() => notify({ isOpen: true, title: 'Settings are being applied',
    message: 'Wait for Apply & Save to finish before closing or replacing this document.' }), [notify]);
  const confirmReplace = useCallback((replace: () => void) => {
    const state = current.current;
    if (state.busy) reportBusy();
    else if (state.dirty) state.ask(replace, 'replace');
    else replace();
  }, [reportBusy]);

  useEffect(() => {
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (!closing.current && (current.current.dirty || current.current.busy)) {
        event.preventDefault();
        event.returnValue = '';
      }
    };
    window.addEventListener('beforeunload', beforeUnload);
    let disposed = false;
    let stop: (() => void) | undefined;
    if (isTauri()) {
      const win = getCurrentWindow();
      void win.onCloseRequested(event => {
        if (closing.current) return;
        const state = current.current;
        if (!state.dirty && !state.busy) return;
        event.preventDefault();
        if (state.busy) { reportBusy(); return; }
        state.ask(() => {
          closing.current = true;
          void win.destroy().catch(error => {
            closing.current = false;
            notify({ isOpen: true, title: 'Could not close RawAccel', message: String(error) });
          });
        }, 'close');
      }).then(unlisten => { if (disposed) unlisten(); else stop = unlisten; })
        .catch(error => notify({ isOpen: true, title: 'Close protection unavailable', message: String(error) }));
    }
    return () => { disposed = true; stop?.(); window.removeEventListener('beforeunload', beforeUnload); };
  }, [notify, reportBusy]);
  return { confirmReplace };
}
