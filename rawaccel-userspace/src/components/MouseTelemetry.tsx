import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { MouseSpeed } from '../tracker';

interface MouseTelemetry { mouseSpeed: MouseSpeed; trackerUnavailable: boolean; }
const Context = createContext<MouseTelemetry | null>(null);

// The header and curve dots share one subscription. Only telemetry consumers
// rerender on input; the profile editor and settings pages remain independent.
export function MouseTelemetryProvider({ children }: { children: ReactNode }) {
  const [mouseSpeed, setMouseSpeed] = useState<MouseSpeed>({ speed_x: 0, speed_y: 0, speed_whole: 0 });
  const [trackerUnavailable, setTrackerUnavailable] = useState(false);
  useEffect(() => {
    let active = true;
    const unlisten = listen<MouseSpeed>('mouse-speed', event => {
      if (active) setMouseSpeed(event.payload);
    }).catch(() => {
      if (active) setTrackerUnavailable(true);
    });
    return () => {
      active = false;
      void unlisten.then(stop => stop?.()).catch(() => { /* Native event bridge may already be closed. */ });
    };
  }, []);
  return <Context.Provider value={{ mouseSpeed, trackerUnavailable }}>{children}</Context.Provider>;
}

export function useMouseTelemetry() {
  const telemetry = useContext(Context);
  if (!telemetry) throw new Error('Mouse telemetry must be used inside its provider.');
  return telemetry;
}

export function LiveInput({ whole }: { whole: boolean }) {
  const { mouseSpeed, trackerUnavailable } = useMouseTelemetry();
  return <div className="live-input" title="Aggregate relative Raw Input estimate across all mice, over roughly 16 ms windows. Unscaled counts; DPI, profile weighting, geometry, clamps and smoothing are excluded. Dots are theoretical curve projections, not measured kernel output.">
    <span>Live input</span>
    <strong>{trackerUnavailable ? '—' : whole ? mouseSpeed.speed_whole.toFixed(1) : `X ${mouseSpeed.speed_x.toFixed(1)} / Y ${mouseSpeed.speed_y.toFixed(1)}`}</strong>
    <span className="live-unit">counts/ms</span>
    <span className="live-caption">{trackerUnavailable ? 'Tracker unavailable' : 'Aggregate Raw Input · theoretical preview'}</span>
  </div>;
}
