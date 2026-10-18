import type { Profile } from "./types";

export interface DriverVersion { major: number; minor: number; patch: number }
export type ConnectionState = "connected" | "unavailable" | "access_denied" | "busy" | "open_failed" | "query_failed" | "malformed_response";
export interface DriverDiagnostic {
  stage: "open" | "version_query" | "version_response";
  win32_code: number | null;
  hresult: string | null;
  message: string;
}
export interface DriverStatus {
  state: ConnectionState;
  version: DriverVersion | null;
  supported_version: DriverVersion;
  driver_contract: string;
  tiered_apply_enabled: boolean;
  compatibility: "supported" | "unverified" | "unsupported" | "unknown";
  diagnostic: DriverDiagnostic | null;
}
export interface DriverStatusSnapshot {
  report: DriverStatus | null;
  checking: boolean;
  ipcError: string | null;
  checkedAt: number | null;
}
export const versionText = (version: DriverVersion) => `${version.major}.${version.minor}.${version.patch}`;

function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === "object" && !Array.isArray(value); }
function integer(value: unknown, max = 2147483647): value is number { return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= max; }
function isVersion(value: unknown): value is DriverVersion { return object(value) && [value.major, value.minor, value.patch].every(n => integer(n)); }

export function parseDriverStatus(value: unknown): DriverStatus {
  const invalid = () => { throw new Error("The app received an invalid driver-status response. Refresh to try again."); };
  if (!object(value) || !["connected", "unavailable", "access_denied", "busy", "open_failed", "query_failed", "malformed_response"].includes(value.state as string)
    || !isVersion(value.supported_version) || typeof value.driver_contract !== "string" || typeof value.tiered_apply_enabled !== "boolean"
    || !["supported", "unverified", "unsupported", "unknown"].includes(value.compatibility as string)) return invalid();
  if (value.state === "connected") {
    if (!isVersion(value.version) || value.diagnostic !== null || value.compatibility === "unknown") return invalid();
    const matches = versionText(value.version) === versionText(value.supported_version);
    if (value.compatibility !== (matches ? value.tiered_apply_enabled ? "unverified" : "supported" : "unsupported")) return invalid();
  } else {
    if (value.version !== null || value.compatibility !== "unknown" || !object(value.diagnostic)) return invalid();
    const d = value.diagnostic;
    if (!["open", "version_query", "version_response"].includes(d.stage as string) || typeof d.message !== "string"
      || !(d.win32_code === null || integer(d.win32_code, 4294967295)) || !(d.hresult === null || (typeof d.hresult === "string" && /^0x[0-9a-f]{8}$/i.test(d.hresult)))) return invalid();
  }
  return value as unknown as DriverStatus;
}

export interface StatusEnvironment {
  now(): number;
  visible(): boolean;
  onFocus(callback: () => void): () => void;
  onVisibility(callback: () => void): () => void;
  every(callback: () => void, milliseconds: number): () => void;
}
const REFRESH_INTERVAL = 30000;
const FOCUS_COOLDOWN = 5000;

/** One owner per app: requests are shared, including across React StrictMode restarts. */
export function createDriverStatusMonitor(query: () => Promise<unknown>, environment: StatusEnvironment) {
  let snapshot: DriverStatusSnapshot = { report: null, checking: true, ipcError: null, checkedAt: null };
  const listeners = new Set<(value: DriverStatusSnapshot) => void>();
  let active = false, generation = 0, queued = false;
  let inFlight: Promise<void> | null = null;
  let lastStarted = -Infinity;
  let cleanup: (() => void)[] = [];
  const publish = (value: DriverStatusSnapshot) => { snapshot = value; for (const listener of listeners) listener(value); };

  const refresh = (force = true): Promise<void> => {
    if (!active) return Promise.resolve();
    if (inFlight) return inFlight;
    if (!force && (!environment.visible() || environment.now() - lastStarted < FOCUS_COOLDOWN)) return Promise.resolve();
    const startedGeneration = generation;
    lastStarted = environment.now();
    inFlight = Promise.resolve().then(query).then(parseDriverStatus).then(report => {
      if (active && startedGeneration === generation) publish({ report, checking: false, ipcError: null, checkedAt: environment.now() });
    }).catch(error => {
      // IPC/worker failures must not leave a stale Connected status on screen.
      if (active && startedGeneration === generation) publish({ report: null, checking: false, ipcError: error instanceof Error ? error.message : String(error), checkedAt: environment.now() });
    }).finally(() => {
      inFlight = null;
      if (active && queued) { queued = false; void refresh(true); }
    });
    publish({ ...snapshot, checking: true });
    return inFlight;
  };
  return {
    getSnapshot: () => snapshot,
    subscribe(listener: (value: DriverStatusSnapshot) => void) { listeners.add(listener); return () => { listeners.delete(listener); }; },
    refresh,
    // Apply may finish while a previous status query is still pending: queue a fresh check.
    afterApply() { if (inFlight) queued = true; return refresh(true); },
    start() {
      if (active) return;
      active = true; generation += 1;
      cleanup = [environment.onFocus(() => { void refresh(false); }), environment.onVisibility(() => { void refresh(false); }), environment.every(() => { void refresh(false); }, REFRESH_INTERVAL)];
      if (inFlight) queued = true; else void refresh(true);
    },
    stop() {
      active = false; generation += 1; queued = false;
      cleanup.forEach(remove => remove()); cleanup = [];
    },
  };
}

export function driverStatusView(snapshot: DriverStatusSnapshot): { label: string; description: string; tone: "neutral" | "success" | "warning" | "error" } {
  if (snapshot.checking) return { label: "Checking driver", description: "Reading the installed driver's version and connection status.", tone: "neutral" };
  if (snapshot.ipcError || !snapshot.report) return { label: "Status unavailable", description: "The app could not retrieve driver status. Refresh to try again.", tone: "error" };
  const report = snapshot.report;
  if (report.state === "connected") {
    if (report.compatibility === "unsupported") return { label: "Unsupported driver", description: `Driver ${versionText(report.version!)} does not match the supported ${versionText(report.supported_version)} protocol. Apply is blocked; editing and graphs remain available.`, tone: "error" };
    return { label: `Connected · ${versionText(report.version!)}`, description: report.tiered_apply_enabled
      ? "This build targets the custom Tiered driver. A matching version does not verify the installed custom layout."
      : "Stock driver connected. Standard modes can be applied; Tiered is available for graph previews only.", tone: report.tiered_apply_enabled ? "warning" : "success" };
  }
  const states = {
    unavailable: { label: "Driver unavailable", description: "The RawAccel device could not be found. Check that the stock driver is installed and running.", tone: "warning" },
    access_denied: { label: "Access denied", description: "Windows denied access to the driver. Check this app's permissions and refresh.", tone: "error" },
    busy: { label: "Driver in use", description: "Another application is using the driver. Close it and refresh.", tone: "warning" },
    open_failed: { label: "Connection failed", description: "Windows could not open the RawAccel device. See the diagnostic details and refresh.", tone: "error" },
    query_failed: { label: "Version query failed", description: "The device opened, but its version could not be read. Apply is blocked until a complete version response is available.", tone: "error" },
    malformed_response: { label: "Invalid driver response", description: "The driver returned a malformed version response. Apply is blocked; see the diagnostic details.", tone: "error" },
  } as const;
  return states[report.state];
}

export function applyBlockReason(snapshot: DriverStatusSnapshot, profiles: Profile[]): string | null {
  if (!snapshot.report?.tiered_apply_enabled) {
    const preview = profiles.find(p => p.accel_x.mode === "tiered" || p.accel_y.mode === "tiered");
    if (preview) return `"${preview.name}" contains Tiered, which is preview only. Switch its X and Y modes before applying. You can still save or export the preview.`;
  }
  if (snapshot.checking) return "Wait for the driver status check to finish.";
  if (!snapshot.report || snapshot.ipcError) return "Refresh driver status before applying settings.";
  if (snapshot.report.state !== "connected" || snapshot.report.compatibility === "unsupported") return driverStatusView(snapshot).description;
  return null;
}
