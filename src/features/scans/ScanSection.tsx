import { useRef, useState } from 'react';
import { Section } from '../../components/content/Section';
import { DataList } from '../../components/content/DataList';
import { ListRow } from '../../components/content/ListRow';
import { Button } from '../../components/ui/Button';
import { FeedbackMessage } from '../../components/ui/FeedbackMessage';
import { exactScanBytes, formatScanBytes } from '../../lib/scanFormatting';
import type {
  Coverage,
  ScanDiagnosticCode,
  ScanMetrics,
  ScanMode,
  ScanSnapshot,
  WorkspaceScanResult,
} from '../../lib/scans';
import type { WorkspaceScanController } from './useWorkspaceScan';
import './ScanSection.css';

const coverageLabels: Record<Coverage, string> = {
  complete_within_policy: 'Complete within declared scope',
  partial: 'Partial',
  unavailable: 'Not available',
};
const limits: Partial<Record<ScanDiagnosticCode, string>> = {
  depth_limit: 'Depth limit',
  entry_limit: 'Entry limit',
  identity_limit: 'Identity limit',
  descriptor_limit: 'Descriptor limit',
  diagnostic_limit: 'Diagnostic limit',
  result_limit: 'Payload limit',
  time_budget: 'Time budget',
  quantity_overflow: 'Quantity limit',
};
const exclusions: Partial<Record<ScanDiagnosticCode, string>> = {
  symlink_excluded: 'symbolic links',
  mount_excluded: 'mount crossings',
  special_excluded: 'special files',
  cloud_not_materialized: 'cloud files not materialized locally',
  cloud_scope_unsupported: 'unsupported cloud scopes',
  outside_read_scope: 'entries outside the permitted read scope',
};
const inaccessible: ScanDiagnosticCode[] = [
  'root_missing',
  'root_denied',
  'root_changed',
  'root_too_broad',
  'unsupported_filesystem',
  'changed_during_scan',
  'entry_missing',
  'entry_denied',
];
const number = (value: string) => BigInt(value).toLocaleString('en-US');

function stateMessage(scan: WorkspaceScanController): string {
  if (scan.status === 'reconciling') return 'Checking scan status…';
  if (scan.status === 'unavailable')
    return 'Scan status is unavailable. Reconnect to check progress.';
  const snapshot = scan.snapshot;
  if (!snapshot) return 'No workspace scan yet.';
  // Worker liveness, not a terminal-looking state, governs stop wording/actions.
  if (snapshot.workerActive) {
    if (snapshot.state === 'stopping')
      return 'Waiting for a filesystem operation to finish. No new scan can start yet.';
    if (snapshot.state === 'cancelling')
      return 'Cancelling scan… Waiting for the scan worker to stop.';
    if (snapshot.state === 'validating') return 'Validating workspace folders…';
    return 'Scanning…';
  }
  if (snapshot.state === 'cancelled')
    return 'Scan cancelled. Measurements shown are incomplete.';
  if (snapshot.state === 'completed')
    return 'Scan completed within the declared scope.';
  if (snapshot.state === 'error') return 'Scan could not complete.';
  return 'Partial measurement.';
}

function ByteValue({ value }: { value: string | null }) {
  return (
    <span className="scan-byte-value numeric">
      <span>{formatScanBytes(value)}</span>
      {value !== null && (
        <span className="scan-exact-bytes">{exactScanBytes(value)}</span>
      )}
    </span>
  );
}
function Measurements({ metrics }: { metrics: ScanMetrics | null }) {
  const logical =
    metrics?.logicalCoverage !== 'unavailable'
      ? (metrics?.observedLogicalBytes ?? null)
      : null;
  const allocated =
    metrics?.allocatedCoverage !== 'unavailable'
      ? (metrics?.observedAllocatedBytes ?? null)
      : null;
  return (
    <>
      <DataList>
        <ListRow
          label="Observed file sizes"
          description={`Logical coverage: ${coverageLabels[metrics?.logicalCoverage ?? 'unavailable']}`}
          trailing={<ByteValue value={logical} />}
        />
        <ListRow
          label="Observed allocated space"
          description={`Allocated coverage: ${coverageLabels[metrics?.allocatedCoverage ?? 'unavailable']}`}
          trailing={<ByteValue value={allocated} />}
        />
      </DataList>
      {metrics && (
        <p className="scan-caption">
          Allocated space measured for {number(metrics.allocationMeasuredFiles)}{' '}
          files; unknown for {number(metrics.allocationUnknownFiles)} files.
          This subtotal may include shared blocks.
        </p>
      )}
    </>
  );
}

export function WorkspaceMeasurement({
  result,
  workspaces,
}: {
  result: WorkspaceScanResult;
  workspaces: WorkspaceScanResult[];
}) {
  const covered = result.coveredByWorkspaceIds.map(
    (id) =>
      workspaces.find((row) => row.workspaceId === id)?.displayName ??
      `Workspace ${id}`,
  );
  return (
    <div className="workspace-measurement">
      <p>Latest scan contribution · {coverageLabels[result.coverage]}</p>
      {covered.length > 0 && (
        <p>
          {result.metrics?.observedLogicalBytes === '0' &&
          result.metrics.observedAllocatedBytes === '0'
            ? 'Accounted for in another workspace: '
            : 'Some entries are accounted for in: '}
          {covered.join(', ')}. Contributions are not counted twice.
        </p>
      )}
      <Measurements metrics={result.metrics} />
    </div>
  );
}

function CoverageDetails({ snapshot }: { snapshot: ScanSnapshot }) {
  const counts = new Map<ScanDiagnosticCode, bigint>();
  for (const diagnostic of snapshot.diagnostics)
    counts.set(
      diagnostic.code,
      (counts.get(diagnostic.code) ?? 0n) + BigInt(diagnostic.count),
    );
  const reasons = new Set(
    [...counts.keys()]
      .filter((code) => limits[code])
      .map((code) => limits[code]!),
  );
  if (snapshot.stopReason && limits[snapshot.stopReason])
    reasons.add(limits[snapshot.stopReason]!);
  const excluded = [...counts]
    .filter(([code, count]) => exclusions[code] && count > 0n)
    .map(
      ([code, count]) =>
        `${exclusions[code]} (${count.toLocaleString('en-US')})`,
    );
  return (
    <div className="scan-coverage">
      {reasons.size > 0 && (
        <>
          <p>Partial measurement — traversal stopped at a scan limit.</p>
          <p>{[...reasons].join(', ')}.</p>
        </>
      )}
      {inaccessible.some((code) => counts.has(code)) && (
        <p>
          Partial measurement — some folders could not be checked or changed
          during the scan.
        </p>
      )}
      {snapshot.metrics.logicalCoverage === 'complete_within_policy' &&
        snapshot.metrics.allocatedCoverage !== 'complete_within_policy' && (
          <p>File sizes measured; allocated space is only partially known.</p>
        )}
      {excluded.length > 0 && (
        <p>Declared exclusions: {excluded.join(', ')} are not included.</p>
      )}
      {snapshot.diagnostics.length > 0 && (
        <ul className="scan-diagnostics">
          {snapshot.diagnostics.map((diagnostic, index) => (
            <li key={`${diagnostic.workspaceId}:${diagnostic.code}:${index}`}>
              {diagnostic.workspaceId &&
                `${snapshot.workspaces.find((row) => row.workspaceId === diagnostic.workspaceId)?.displayName ?? `Workspace ${diagnostic.workspaceId}`}: `}
              {diagnostic.message} ({number(diagnostic.count)})
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Counter/measurement content stays outside the state-only live region. */
export function ScanSummary({ scan }: { scan: WorkspaceScanController }) {
  return (
    <div className="scan-summary">
      <div role="status" aria-live="polite" aria-atomic="true">
        {stateMessage(scan)}
      </div>
      {scan.error && scan.error.source !== 'transport' && (
        <FeedbackMessage tone="error">{scan.error.message}</FeedbackMessage>
      )}
      {scan.snapshot && (
        <>
          {scan.status !== 'ready' && (
            <p>
              Last known measurements. Current progress and worker status have
              not been confirmed.
            </p>
          )}
          <p className="scan-caption">
            {scan.snapshot.mode === 'quick' ? 'Quick' : 'Deep'} scan ·{' '}
            <span>
              Traversal checks attempted: {number(scan.snapshot.entryAttempts)}
            </span>
          </p>
          <Measurements metrics={scan.snapshot.metrics} />
          <CoverageDetails snapshot={scan.snapshot} />
          {scan.snapshot.failure && (
            <FeedbackMessage tone="error">
              {scan.snapshot.failure.message}
            </FeedbackMessage>
          )}
        </>
      )}
      <p className="scan-caption">
        Observed file sizes; not reclaimable space.
      </p>
    </div>
  );
}

export function ScanSection({
  scan,
  enabledIds,
  configurationBusy,
}: {
  scan: WorkspaceScanController;
  enabledIds: string[];
  configurationBusy: boolean;
}) {
  const pending = useRef(false);
  const [busy, setBusy] = useState(false);
  const live = !!scan.snapshot?.workerActive;
  const tooMany = enabledIds.length > 32;
  const canStart = (mode: ScanMode) => {
    if (busy || configurationBusy || tooMany || scan.status === 'reconciling')
      return false;
    if (scan.pendingStart) return scan.pendingStart.mode === mode;
    return (
      scan.status === 'ready' && !live && enabledIds.length > 0 && !tooMany
    );
  };
  async function action(operation: () => Promise<void>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    try {
      await operation();
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  function start(mode: ScanMode) {
    if (!canStart(mode)) return;
    const ids = [...(scan.pendingStart?.workspaceIds ?? enabledIds)];
    void action(() => scan.start(ids, mode));
  }
  return (
    <Section title="Workspace analysis">
      <div className="scan-controls">
        <Button disabled={!canStart('quick')} onClick={() => start('quick')}>
          Quick scan
        </Button>
        <Button disabled={!canStart('deep')} onClick={() => start('deep')}>
          Deep scan
        </Button>
        {live && (
          <Button
            disabled={busy || scan.status !== 'ready'}
            onClick={() => {
              void action(() => scan.cancel());
            }}
          >
            Cancel scan
          </Button>
        )}
        {scan.status === 'unavailable' && (
          <Button
            disabled={busy}
            onClick={() => {
              void action(() => scan.reconnect());
            }}
          >
            Reconnect
          </Button>
        )}
      </div>
      <p className="scan-caption">Deep scan also has limits.</p>
      {!scan.pendingStart && enabledIds.length === 0 && (
        <p>No enabled workspace folders. Add or enable a workspace to scan.</p>
      )}
      {tooMany && (
        <p>
          A scan supports up to 32 enabled workspaces. Disable some workspaces
          to scan.
        </p>
      )}
      {configurationBusy && (
        <p>
          Finish the workspace operation or close the folder picker before
          starting a scan.
        </p>
      )}
      {scan.pendingStart && (
        <p>
          {scan.pendingStart.mode === 'quick' ? 'Quick' : 'Deep'} scan retries
          the original {scan.pendingStart.mode === 'quick' ? 'Quick' : 'Deep'}{' '}
          scan request with its original workspace folders. Reconnect only
          checks status.
        </p>
      )}
      <ScanSummary scan={scan} />
    </Section>
  );
}
