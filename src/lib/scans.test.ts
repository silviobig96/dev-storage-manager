import { expect, it, vi } from 'vitest';
import {
  startWorkspaceScan,
  getWorkspaceScan,
  getWorkspaceScanSession,
  cancelWorkspaceScan,
  ScanCommandError,
  type ScanSnapshot,
} from './scans';

it('sends only the exact native command arguments and preserves decimal quantities', async () => {
  const request = {
    requestId: '01234567-89ab-cdef-0123-456789abcdef',
    expectedEpoch: 'p1-t1-g0',
    workspaceIds: ['7'],
    mode: 'quick' as const,
  };
  const snapshot: ScanSnapshot = {
    sessionId: 's:p1-t1-g1:1',
    requestId: request.requestId,
    epoch: 'p1-t1-g1',
    mode: 'quick',
    state: 'running',
    revision: '9007199254740993',
    startedAt: '1',
    finishedAt: null,
    workerActive: true,
    stopReason: null,
    failure: null,
    entryAttempts: '9007199254740993',
    workspaces: [
      {
        workspaceId: '7',
        displayName: 'fixture',
        selectedPath: '/fixture',
        state: 'pending',
        metrics: null,
        coverage: 'unavailable',
        coveredByWorkspaceIds: [],
      },
    ],
    metrics: {
      observedLogicalBytes: '9007199254740993',
      observedAllocatedBytes: null,
      uniqueFiles: '1',
      uniqueDirectories: '0',
      referencesDeduplicated: '0',
      allocationMeasuredFiles: '0',
      allocationUnknownFiles: '1',
      policyExcludedEntries: '0',
      unmeasuredEntriesLowerBound: '0',
      unmeasuredBytes: null,
      logicalCoverage: 'partial',
      allocatedCoverage: 'unavailable',
    },
    coverage: 'partial',
    protection: {
      safetyLevel: 'protected',
      policyVersion: 'scanner-measurement-v1',
      allowedActions: [],
      reclaimability: 'unknown',
    },
    diagnostics: [
      {
        workspaceId: '7',
        code: 'allocation_unknown',
        severity: 'warning',
        count: '1',
        message: 'Allocated space is unavailable for an observed file.',
      },
    ],
  };
  const invoke = vi
    .fn()
    .mockResolvedValueOnce(snapshot)
    .mockResolvedValueOnce({ epoch: snapshot.epoch, session: snapshot })
    .mockResolvedValueOnce(snapshot)
    .mockResolvedValueOnce({ ...snapshot, state: 'cancelling' });
  const started = await startWorkspaceScan(request, invoke);
  expect(started.metrics.observedLogicalBytes).toBe('9007199254740993');
  expect(started.metrics.observedAllocatedBytes).toBeNull();
  expect(started.revision).toBe('9007199254740993');
  expect(started.entryAttempts).toBe('9007199254740993');
  expect(started.workspaces[0]?.metrics).toBeNull();
  expect(await getWorkspaceScan(invoke)).toEqual({
    epoch: 'p1-t1-g1',
    session: snapshot,
  });
  expect(await getWorkspaceScanSession('s:p1-t1-g1:1', invoke)).toEqual(
    snapshot,
  );
  expect((await cancelWorkspaceScan('s:p1-t1-g1:1', invoke)).state).toBe(
    'cancelling',
  );
  expect(invoke.mock.calls).toEqual([
    ['start_workspace_scan', { request }],
    ['get_workspace_scan'],
    ['get_workspace_scan_session', { sessionId: 's:p1-t1-g1:1' }],
    ['cancel_workspace_scan', { sessionId: 's:p1-t1-g1:1' }],
  ]);
});

it('distinguishes backend rejection from unknown transport outcome without exposing raw errors', async () => {
  const invoke = vi
    .fn()
    .mockRejectedValueOnce({ code: 'scan_busy', message: '/private/error' })
    .mockRejectedValueOnce('/private/error');
  await expect(getWorkspaceScan(invoke)).rejects.toMatchObject({
    code: 'scan_busy',
    source: 'backend',
  });
  await expect(getWorkspaceScan(invoke)).rejects.toMatchObject({
    source: 'transport',
    message: 'Scan status is unavailable. Reconnect to check progress.',
  });
  expect(new ScanCommandError('scan_busy').message).not.toContain('/private');
});
