import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { createDriverStatusMonitor } from "../driverStatus";

export function useDriverStatus() {
  const [monitor] = useState(() => createDriverStatusMonitor(() => invoke("get_driver_status"), {
    now: Date.now,
    visible: () => document.visibilityState === "visible",
    onFocus: callback => { window.addEventListener("focus", callback); return () => window.removeEventListener("focus", callback); },
    onVisibility: callback => { document.addEventListener("visibilitychange", callback); return () => document.removeEventListener("visibilitychange", callback); },
    every: (callback, ms) => { const timer = window.setInterval(callback, ms); return () => window.clearInterval(timer); },
  }));
  const [snapshot, setSnapshot] = useState(monitor.getSnapshot);
  useEffect(() => {
    const unsubscribe = monitor.subscribe(setSnapshot);
    monitor.start();
    return () => { unsubscribe(); monitor.stop(); };
  }, [monitor]);
  return { ...snapshot, refresh: monitor.refresh, afterApply: monitor.afterApply };
}
