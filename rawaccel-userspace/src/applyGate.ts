// Shared by all Apply controls. A second click is refused, never queued with a
// stale snapshot of profiles or assignments. Rejections and partial success unlock.
export function createApplyGate() {
  let running = false;
  return async (apply: () => Promise<boolean>): Promise<boolean> => {
    if (running) return false;
    running = true;
    try { return await apply(); }
    finally { running = false; }
  };
}
