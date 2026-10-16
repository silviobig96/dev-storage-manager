import { StrictMode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useWorkspaceScan } from './useWorkspaceScan';
import type { ScanSnapshot } from '../../lib/scans';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const ipc = vi.mocked(invoke);
let uuid: { mock: { calls: unknown[] } };
const requestId = '01234567-89ab-cdef-0123-456789abcdef';
const epoch = 'p1-t1-g1';
const snapshot: ScanSnapshot = {
  sessionId: 's:p1-t1-g1:1',
  requestId,
  epoch,
  mode: 'quick',
  state: 'running',
  revision: '9007199254740993',
  startedAt: '1',
  finishedAt: null,
  workerActive: true,
  stopReason: null,
  failure: null,
  entryAttempts: '1',
  workspaces: [],
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
  diagnostics: [],
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
async function flush() {
  await act(async () => {
    await Promise.resolve();
  });
}
async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}
function visibility(value: 'hidden' | 'visible') {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    value,
  });
  document.dispatchEvent(new Event('visibilitychange'));
}
beforeEach(() => {
  vi.useFakeTimers();
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    value: 'visible',
  });
  ipc.mockReset().mockResolvedValue({ epoch: 'p1-t1-g0', session: null });
  uuid = vi.spyOn(crypto, 'randomUUID').mockReturnValue(requestId);
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it('recovers once at mount and never starts, cancels or polls an idle session', async () => {
  const { result, rerender, unmount } = renderHook(() => useWorkspaceScan());
  await flush();
  expect(result.current.status).toBe('ready');
  expect(result.current.snapshot).toBeNull();
  rerender();
  await advance(5000);
  unmount();
  expect(ipc.mock.calls).toEqual([['get_workspace_scan']]);
});

it.each([false, true])(
  'accepts a new explicit session after completion, lost Start response: %s',
  async (lostResponse) => {
    const completed = {
      ...snapshot,
      state: 'completed' as const,
      workerActive: false,
    };
    ipc.mockResolvedValueOnce({ epoch, session: completed });
    const { result, unmount } = renderHook(() => useWorkspaceScan());
    await flush();
    const nextRequestId = '11234567-89ab-cdef-0123-456789abcdef';
    vi.spyOn(crypto, 'randomUUID').mockReturnValueOnce(nextRequestId);
    const next = {
      ...snapshot,
      // The real service increments generation/epoch for every accepted Start.
      epoch: 'p1-t1-g2',
      sessionId: 's:p1-t1-g2:2',
      requestId: nextRequestId,
      revision: '1',
    };
    if (lostResponse) ipc.mockRejectedValueOnce(new Error('lost transport'));
    else ipc.mockResolvedValueOnce(next);
    await act(async () => {
      await result.current.start(['7'], 'quick');
    });
    if (lostResponse) {
      expect(result.current.status).toBe('unavailable');
      ipc.mockResolvedValueOnce({ epoch: next.epoch, session: next });
      await act(async () => {
        await result.current.reconnect();
      });
    }
    expect(result.current.status).toBe('ready');
    expect(result.current.snapshot).toEqual(next);
    expect(result.current.pendingStart).toBeNull();
    unmount();
  },
);

it('rejects an unrelated new session when no explicit Start is pending', async () => {
  const completed = {
    ...snapshot,
    state: 'completed' as const,
    workerActive: false,
  };
  ipc.mockResolvedValueOnce({ epoch, session: completed });
  const { result, unmount } = renderHook(() => useWorkspaceScan());
  await flush();
  ipc.mockResolvedValueOnce({
    epoch,
    session: { ...snapshot, sessionId: 'unrelated', requestId: 'other' },
  });
  await act(async () => {
    await result.current.reconnect();
  });
  expect(result.current.status).toBe('unavailable');
  expect(result.current.snapshot).toEqual(completed);
  unmount();
});

it('rejects a new session without the epoch advance required by the service even during Start', async () => {
  const completed = {
    ...snapshot,
    state: 'completed' as const,
    workerActive: false,
  };
  ipc.mockResolvedValueOnce({ epoch, session: completed });
  const { result, unmount } = renderHook(() => useWorkspaceScan());
  await flush();
  const nextRequestId = '11234567-89ab-cdef-0123-456789abcdef';
  vi.spyOn(crypto, 'randomUUID').mockReturnValueOnce(nextRequestId);
  ipc.mockResolvedValueOnce({
    ...snapshot,
    sessionId: 'different-session',
    requestId: nextRequestId,
    revision: '1',
  });
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(result.current.status).toBe('unavailable');
  expect(result.current.snapshot).toEqual(completed);
  unmount();
});

it('polls at 250ms with one query in flight, pauses hidden and reconciles once on show', async () => {
  ipc.mockResolvedValue({ epoch, session: snapshot });
  const { result, unmount } = renderHook(() => useWorkspaceScan());
  await flush();
  const pending = deferred<unknown>();
  ipc.mockReturnValueOnce(pending.promise);
  await advance(249);
  expect(ipc).toHaveBeenCalledTimes(1);
  await advance(1);
  await advance(2000);
  expect(ipc).toHaveBeenCalledTimes(2);
  act(() => visibility('hidden'));
  await act(async () => {
    pending.resolve({ epoch, session: snapshot });
    await pending.promise;
  });
  await advance(1000);
  expect(ipc).toHaveBeenCalledTimes(2);
  act(() => visibility('visible'));
  await flush();
  expect(ipc).toHaveBeenCalledTimes(3);
  expect(result.current.snapshot?.state).toBe('running');
  unmount();
  await advance(1000);
  expect(
    ipc.mock.calls.every(([command]) => command === 'get_workspace_scan'),
  ).toBe(true);
  expect(ipc).toHaveBeenCalledTimes(3);
});

it('keeps a failed query unknown and preserves the live snapshot until explicit reconnect', async () => {
  ipc
    .mockResolvedValueOnce({ epoch, session: snapshot })
    .mockRejectedValueOnce('lost response');
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  await advance(250);
  expect(result.current.status).toBe('unavailable');
  expect(result.current.snapshot).toEqual(snapshot);
  expect(result.current.error?.source).toBe('transport');
  await advance(1000);
  expect(ipc).toHaveBeenCalledTimes(2);
  const terminal = {
    ...snapshot,
    revision: '9007199254740994',
    state: 'completed',
    workerActive: false,
    finishedAt: '2',
  };
  ipc.mockResolvedValueOnce({ epoch, session: terminal });
  await act(async () => {
    await result.current.reconnect();
  });
  expect(result.current.status).toBe('ready');
  expect(result.current.snapshot).toEqual(terminal);
  await advance(1000);
  expect(ipc).toHaveBeenCalledTimes(3);
});

it('reconciles lost start response by request identity and does not issue another start', async () => {
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  ipc.mockRejectedValueOnce('lost response');
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(result.current.status).toBe('unavailable');
  ipc.mockResolvedValueOnce({ epoch, session: snapshot });
  await act(async () => {
    await result.current.reconnect();
  });
  expect(result.current.snapshot).toEqual(snapshot);
  expect(result.current.status).toBe('ready');
  expect(ipc.mock.calls).toEqual([
    ['get_workspace_scan'],
    [
      'start_workspace_scan',
      {
        request: {
          requestId,
          expectedEpoch: 'p1-t1-g0',
          workspaceIds: ['7'],
          mode: 'quick',
        },
      },
    ],
    ['get_workspace_scan'],
  ]);
});

it('retains the exact lost request for an explicit replay and blocks a different request', async () => {
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  ipc.mockRejectedValueOnce('lost response');
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(result.current.pendingStart).toEqual({
    workspaceIds: ['7'],
    mode: 'quick',
  });
  await act(async () => {
    await result.current.reconnect();
  });
  expect(result.current.status).toBe('unavailable');
  await act(async () => {
    await result.current.start(['8'], 'deep');
  });
  expect(ipc).toHaveBeenCalledTimes(3);
  ipc.mockResolvedValueOnce(snapshot);
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(ipc.mock.calls[3]).toEqual(ipc.mock.calls[1]);
  expect(uuid.mock.calls).toHaveLength(1);
  expect(result.current.snapshot).toEqual(snapshot);
  expect(result.current.pendingStart).toBeNull();
});

it('ignores lower decimal revisions and contradictory session identities within an epoch', async () => {
  ipc.mockResolvedValueOnce({ epoch, session: snapshot });
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  ipc.mockResolvedValueOnce({
    epoch,
    session: {
      ...snapshot,
      revision: '9007199254740992',
      state: 'completed',
      workerActive: false,
    },
  });
  await advance(250);
  expect(result.current.snapshot).toEqual(snapshot);
  ipc.mockResolvedValueOnce({
    epoch,
    session: {
      ...snapshot,
      sessionId: 's:p1-t1-g1:2',
      revision: '9007199254740994',
    },
  });
  await advance(250);
  expect(result.current.snapshot).toEqual(snapshot);
});

it('ignores a stale query after cancellation and never treats cancel acknowledgement as completion', async () => {
  ipc.mockResolvedValueOnce({ epoch, session: snapshot });
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  const pending = deferred<unknown>();
  ipc.mockReturnValueOnce(pending.promise);
  await advance(250);
  const cancelling = {
    ...snapshot,
    revision: '9007199254740994',
    state: 'cancelling',
  };
  ipc.mockResolvedValueOnce(cancelling);
  await act(async () => {
    await result.current.cancel();
  });
  await act(async () => {
    pending.resolve({ epoch: 'p1-t1-g0', session: null });
    await pending.promise;
  });
  expect(result.current.snapshot).toEqual(cancelling);
  expect(ipc.mock.calls[2]).toEqual([
    'cancel_workspace_scan',
    { sessionId: snapshot.sessionId },
  ]);
});

it('ignores StrictMode stale recovery without overlapping native status queries', async () => {
  const pending = deferred<unknown>();
  ipc
    .mockReturnValueOnce(pending.promise)
    .mockResolvedValueOnce({ epoch, session: snapshot });
  const { result, unmount } = renderHook(() => useWorkspaceScan(), {
    wrapper: StrictMode,
  });
  expect(ipc).toHaveBeenCalledTimes(1);
  await act(async () => {
    pending.resolve({ epoch: 'p1-t1-g0', session: null });
    await pending.promise;
  });
  expect(result.current.snapshot).toEqual(snapshot);
  expect(ipc.mock.calls).toEqual([
    ['get_workspace_scan'],
    ['get_workspace_scan'],
  ]);
  unmount();
});

it('cannot start without an authoritative epoch or after a failed recovery', async () => {
  const pending = deferred<unknown>();
  ipc.mockReturnValueOnce(pending.promise);
  const { result } = renderHook(() => useWorkspaceScan());
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(ipc).toHaveBeenCalledTimes(1);
  await act(async () => {
    pending.reject({ code: 'application_unavailable' });
    await Promise.resolve();
  });
  expect(result.current.status).toBe('unavailable');
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(ipc).toHaveBeenCalledTimes(1);
  expect(uuid.mock.calls).toHaveLength(0);
});

it('does not trust a nullable epoch as a successful recovery', async () => {
  ipc.mockResolvedValueOnce({ epoch: null, session: null });
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  expect(result.current.status).toBe('unavailable');
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  expect(ipc).toHaveBeenCalledTimes(1);
});

it('prevents rapid start duplication and discards its response after unmount', async () => {
  const { result, unmount } = renderHook(() => useWorkspaceScan());
  await flush();
  const pending = deferred<ScanSnapshot>();
  ipc.mockReturnValueOnce(pending.promise);
  let starting!: Promise<void>;
  act(() => {
    starting = result.current.start(['7'], 'quick');
  });
  await act(async () => {
    await result.current.start(['7'], 'quick');
  });
  const before = result.current;
  expect(before.status).toBe('reconciling');
  unmount();
  pending.resolve(snapshot);
  await starting;
  expect(result.current).toBe(before);
  expect(ipc.mock.calls.map(([name]) => name)).toEqual([
    'get_workspace_scan',
    'start_workspace_scan',
  ]);
});

it('ignores a retired epoch after authoritative recovery adopts a different epoch', async () => {
  ipc.mockResolvedValueOnce({ epoch, session: snapshot });
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  const newer = {
    ...snapshot,
    epoch: 'p1-t1-g2',
    sessionId: 's:p1-t1-g2:2',
    requestId: '11234567-89ab-cdef-0123-456789abcdef',
    revision: '1',
  };
  ipc.mockResolvedValueOnce({ epoch: newer.epoch, session: newer });
  await act(async () => {
    await result.current.reconnect();
  });
  expect(result.current.snapshot).toEqual(newer);
  ipc.mockResolvedValueOnce({ epoch, session: snapshot });
  await advance(250);
  expect(result.current.snapshot).toEqual(newer);
});

it('preserves an unknown cancellation until a status read confirms the native worker stopped', async () => {
  ipc
    .mockResolvedValueOnce({ epoch, session: snapshot })
    .mockRejectedValueOnce('lost cancel response');
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  await act(async () => {
    await result.current.cancel();
  });
  expect(result.current.status).toBe('unavailable');
  expect(result.current.snapshot?.workerActive).toBe(true);
  await advance(1000);
  expect(ipc).toHaveBeenCalledTimes(2);
  ipc.mockResolvedValueOnce({
    epoch,
    session: {
      ...snapshot,
      revision: '9007199254740994',
      state: 'cancelled',
      workerActive: false,
      finishedAt: '2',
    },
  });
  await act(async () => {
    await result.current.reconnect();
  });
  expect(result.current.snapshot?.state).toBe('cancelled');
  expect(result.current.snapshot?.workerActive).toBe(false);
  expect(result.current.status).toBe('ready');
});

it('defers show recovery until a pending cancel and older status query settle', async () => {
  ipc.mockResolvedValueOnce({ epoch, session: snapshot });
  const { result } = renderHook(() => useWorkspaceScan());
  await flush();
  const query = deferred<unknown>();
  const cancel = deferred<ScanSnapshot>();
  ipc.mockReturnValueOnce(query.promise).mockReturnValueOnce(cancel.promise);
  await advance(250);
  let cancelling!: Promise<void>;
  act(() => {
    cancelling = result.current.cancel();
  });
  act(() => {
    visibility('hidden');
    visibility('visible');
  });
  await act(async () => {
    query.resolve({ epoch, session: snapshot });
    await query.promise;
  });
  expect(ipc).toHaveBeenCalledTimes(3);
  const stopped: ScanSnapshot = {
    ...snapshot,
    revision: '9007199254740994',
    state: 'cancelled',
    workerActive: false,
    finishedAt: '2',
  };
  ipc.mockResolvedValueOnce({ epoch, session: stopped });
  await act(async () => {
    cancel.resolve(stopped);
    await cancelling;
  });
  expect(result.current.status).toBe('ready');
  expect(result.current.snapshot).toEqual(stopped);
  expect(ipc.mock.calls.map(([name]) => name)).toEqual([
    'get_workspace_scan',
    'get_workspace_scan',
    'cancel_workspace_scan',
    'get_workspace_scan',
  ]);
});
