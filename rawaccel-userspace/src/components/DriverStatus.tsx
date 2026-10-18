import { RefreshCw } from "lucide-react";
import { driverStatusView, versionText } from "../driverStatus";
import type { DriverStatusSnapshot } from "../driverStatus";

interface StatusProps {
  snapshot: DriverStatusSnapshot;
  onRefresh: () => Promise<void>;
  announce?: boolean;
}

export function DriverStatusIndicator({ snapshot, onRefresh, announce = true }: StatusProps) {
  const view = driverStatusView(snapshot);
  return (
    <div className={`driver-status-indicator driver-status-${view.tone}`} title={view.description}>
      <span className="driver-status-dot" aria-hidden="true" />
      <span role={announce ? "status" : undefined} aria-live={announce ? "polite" : "off"}>{view.label}</span>
      <button className="driver-status-refresh" type="button" aria-label="Refresh driver status"
        title="Refresh driver status" disabled={snapshot.checking} onClick={() => { void onRefresh(); }}>
        <RefreshCw size={14} className={snapshot.checking ? "driver-status-spinner" : undefined} />
      </button>
    </div>
  );
}

export function DriverStatusCard({ snapshot, onRefresh }: StatusProps) {
  const view = driverStatusView(snapshot);
  const report = snapshot.report;
  const diagnostic = report?.diagnostic;
  return (
    <div className="driver-status-card">
      <div className="driver-status-card-heading">
        <h4>Installed driver</h4>
        <DriverStatusIndicator snapshot={snapshot} onRefresh={onRefresh} announce={false} />
      </div>
      <p>{view.description}</p>
      <dl className="driver-status-facts">
        <div><dt>Reported version</dt><dd>{report?.version ? versionText(report.version) : "Not available"}</dd></div>
        <div><dt>Apply target</dt><dd>{report
          ? `${report.tiered_apply_enabled ? "Custom Tiered" : "Stock RawAccel"} ${versionText(report.supported_version)}`
          : "Waiting for status"}</dd></div>
        <div><dt>Last checked</dt><dd>{snapshot.checkedAt === null ? "Not yet checked" : new Date(snapshot.checkedAt).toLocaleTimeString()}</dd></div>
      </dl>
      {!report?.tiered_apply_enabled && <p className="driver-status-preview-note">Tiered editing, both submodes, and all graph views remain available. Applying Tiered requires a future signed driver that supports it.</p>}
      {(diagnostic || snapshot.ipcError) && (
        <details className="driver-status-diagnostics">
          <summary>Diagnostic details</summary>
          {diagnostic && <p>Stage: {diagnostic.stage.replace(/_/g, " ")}
            {diagnostic.win32_code !== null && ` · Windows error ${diagnostic.win32_code}`}
            {diagnostic.hresult && ` · ${diagnostic.hresult}`}</p>}
          <pre className="selectable">{snapshot.ipcError || diagnostic?.message}</pre>
        </details>
      )}
    </div>
  );
}
