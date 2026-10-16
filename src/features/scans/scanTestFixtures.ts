import { vi } from 'vitest';
import type { ScanMetrics, ScanMode, ScanSnapshot } from '../../lib/scans';
import type { WorkspaceScanController } from './useWorkspaceScan';

export function scanMetrics(overrides: Partial<ScanMetrics> = {}): ScanMetrics {
  return {
    observedLogicalBytes: '1073741824',
    observedAllocatedBytes: '4096',
    uniqueFiles: '1',
    uniqueDirectories: '1',
    referencesDeduplicated: '0',
    allocationMeasuredFiles: '1',
    allocationUnknownFiles: '0',
    policyExcludedEntries: '0',
    unmeasuredEntriesLowerBound: '0',
    unmeasuredBytes: null,
    logicalCoverage: 'complete_within_policy',
    allocatedCoverage: 'complete_within_policy',
    ...overrides,
  };
}
export function scanSnapshot(
  overrides: Partial<ScanSnapshot> = {},
): ScanSnapshot {
  return {
    sessionId: 's:p1-t1-g1:1',
    requestId: '01234567-89ab-cdef-0123-456789abcdef',
    epoch: 'p1-t1-g1',
    mode: 'quick',
    state: 'completed',
    revision: '1',
    startedAt: '1',
    finishedAt: '2',
    workerActive: false,
    stopReason: null,
    failure: null,
    entryAttempts: '1234',
    metrics: scanMetrics(),
    coverage: 'complete_within_policy',
    workspaces: [
      {
        workspaceId: '1',
        displayName: 'Code',
        selectedPath: '/fixture/one/Code',
        state: 'completed',
        metrics: scanMetrics(),
        coverage: 'complete_within_policy',
        coveredByWorkspaceIds: [],
      },
    ],
    protection: {
      safetyLevel: 'protected',
      policyVersion: 'scanner-measurement-v1',
      allowedActions: [],
      reclaimability: 'unknown',
    },
    diagnostics: [],
    ...overrides,
  };
}
type TestScanController = Omit<
  WorkspaceScanController,
  'start' | 'cancel' | 'reconnect'
> & {
  start: (ids: string[], mode: ScanMode) => Promise<void>;
  cancel: () => Promise<void>;
  reconnect: () => Promise<void>;
};
export function scanController(
  overrides: Partial<WorkspaceScanController> = {},
): TestScanController {
  return {
    status: 'ready',
    snapshot: null,
    error: null,
    pendingStart: null,
    start: vi.fn().mockResolvedValue(undefined),
    cancel: vi.fn().mockResolvedValue(undefined),
    reconnect: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}
