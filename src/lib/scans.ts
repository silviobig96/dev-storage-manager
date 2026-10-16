import { invoke } from '@tauri-apps/api/core';

export type ScanInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;
/** Native quantities remain canonical decimal strings throughout transport. */
export type DecimalQuantity = string;
export type ScanMode = 'quick' | 'deep';
export type ScanState =
  | 'validating'
  | 'running'
  | 'stopping'
  | 'cancelling'
  | 'completed'
  | 'partial'
  | 'cancelled'
  | 'error';
export type Coverage = 'complete_within_policy' | 'partial' | 'unavailable';
export type StopReason =
  | 'time_budget'
  | 'entry_limit'
  | 'identity_limit'
  | 'descriptor_limit'
  | 'diagnostic_limit'
  | 'result_limit'
  | 'quantity_overflow'
  | 'cancelled';
export type ScanDiagnosticCode =
  | StopReason
  | 'root_missing'
  | 'root_denied'
  | 'root_changed'
  | 'root_too_broad'
  | 'unsupported_filesystem'
  | 'changed_during_scan'
  | 'entry_missing'
  | 'entry_denied'
  | 'symlink_excluded'
  | 'mount_excluded'
  | 'special_excluded'
  | 'cloud_not_materialized'
  | 'cloud_scope_unsupported'
  | 'outside_read_scope'
  | 'unrepresentable_name'
  | 'allocation_unknown'
  | 'depth_limit';
export interface ScanRequest {
  requestId: string;
  expectedEpoch: string;
  workspaceIds: string[];
  mode: ScanMode;
}
export interface ScanStatus {
  epoch: string;
  session: ScanSnapshot | null;
}
export interface ScanMetrics {
  observedLogicalBytes: DecimalQuantity;
  observedAllocatedBytes: DecimalQuantity | null;
  uniqueFiles: DecimalQuantity;
  uniqueDirectories: DecimalQuantity;
  referencesDeduplicated: DecimalQuantity;
  allocationMeasuredFiles: DecimalQuantity;
  allocationUnknownFiles: DecimalQuantity;
  policyExcludedEntries: DecimalQuantity;
  unmeasuredEntriesLowerBound: DecimalQuantity;
  unmeasuredBytes: DecimalQuantity | null;
  logicalCoverage: Coverage;
  allocatedCoverage: Coverage;
}
export interface WorkspaceScanResult {
  workspaceId: string;
  displayName: string;
  selectedPath: string;
  state:
    | 'pending'
    | 'validating'
    | 'running'
    | 'completed'
    | 'partial'
    | 'unavailable'
    | 'cancelled';
  metrics: ScanMetrics | null;
  coverage: Coverage;
  coveredByWorkspaceIds: string[];
}
export interface ScanDiagnostic {
  workspaceId: string | null;
  code: ScanDiagnosticCode;
  severity: 'info' | 'warning' | 'error';
  count: DecimalQuantity;
  message: string;
}
export interface ScanSnapshot {
  sessionId: string;
  requestId: string;
  epoch: string;
  mode: ScanMode;
  state: ScanState;
  revision: DecimalQuantity;
  startedAt: DecimalQuantity;
  finishedAt: DecimalQuantity | null;
  workerActive: boolean;
  stopReason: StopReason | null;
  failure: ScanFailure | null;
  entryAttempts: DecimalQuantity;
  workspaces: WorkspaceScanResult[];
  metrics: ScanMetrics;
  coverage: Coverage;
  protection: {
    safetyLevel: 'protected';
    policyVersion: 'scanner-measurement-v1';
    allowedActions: [];
    reclaimability: 'unknown';
  };
  diagnostics: ScanDiagnostic[];
}
const messages = {
  invalid_request: 'The scan request is invalid.',
  scope_empty: 'Select at least one enabled workspace to scan.',
  scope_too_large: 'The requested scan scope exceeds supported limits.',
  workspace_not_found: 'A requested workspace is no longer configured.',
  workspace_disabled: 'A requested workspace is disabled.',
  scan_busy: 'A scan is already in progress.',
  workspace_operation_in_progress: 'A workspace operation is in progress.',
  stale_epoch: 'The scan state changed. Reconnect and try again.',
  request_conflict:
    'Reconnect to resolve the previous scan request before starting a different scan.',
  session_not_found: 'This scan session is no longer available.',
  persistence_unavailable: 'Workspace configuration is unavailable. Try again.',
  resource_unavailable: 'Scan resources are unavailable. Try again.',
  application_unavailable: 'Scan services are unavailable. Try again.',
} as const;
export type ScanErrorCode = keyof typeof messages;
export interface ScanFailure {
  code: ScanErrorCode;
  message: string;
}
export class ScanCommandError extends Error implements ScanFailure {
  constructor(
    public readonly code: ScanErrorCode,
    public readonly source: 'backend' | 'transport' = 'backend',
  ) {
    super(
      source === 'transport'
        ? 'Scan status is unavailable. Reconnect to check progress.'
        : messages[code],
    );
    this.name = 'ScanCommandError';
  }
}
export function toScanCommandError(value: unknown): ScanCommandError {
  if (value instanceof ScanCommandError) return value;
  if (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    typeof value.code === 'string' &&
    Object.hasOwn(messages, value.code)
  )
    return new ScanCommandError(value.code as ScanErrorCode);
  return new ScanCommandError('application_unavailable', 'transport');
}
async function command<T>(
  name: string,
  invokeFn: ScanInvoke,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await (args === undefined
      ? invokeFn<T>(name)
      : invokeFn<T>(name, args));
  } catch (error) {
    throw toScanCommandError(error);
  }
}
export function startWorkspaceScan(
  request: ScanRequest,
  invokeFn: ScanInvoke = invoke,
): Promise<ScanSnapshot> {
  return command('start_workspace_scan', invokeFn, { request });
}
export function getWorkspaceScan(
  invokeFn: ScanInvoke = invoke,
): Promise<ScanStatus> {
  return command('get_workspace_scan', invokeFn);
}
export function getWorkspaceScanSession(
  sessionId: string,
  invokeFn: ScanInvoke = invoke,
): Promise<ScanSnapshot> {
  return command('get_workspace_scan_session', invokeFn, { sessionId });
}
export function cancelWorkspaceScan(
  sessionId: string,
  invokeFn: ScanInvoke = invoke,
): Promise<ScanSnapshot> {
  return command('cancel_workspace_scan', invokeFn, { sessionId });
}
