import { StrictMode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  WorkspaceCommandError,
  type AvailabilityObservation,
  type WorkspaceListing,
  type WorkspaceRecord,
} from '../../lib/workspaces';
import { useWorkspaces } from './useWorkspaces';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const empty: WorkspaceListing = {
  setupState: 'skipped',
  workspaces: [],
  overlaps: [],
};
const row: WorkspaceRecord = {
  id: '7',
  selectedPath: '/fixture/root',
  displayName: 'root',
  enabled: true,
  createdAt: 1,
  updatedAt: 1,
};
const present: AvailabilityObservation = {
  state: 'present',
  reason: null,
  checkedAt: 2,
};
const configured: WorkspaceListing = {
  setupState: 'configured',
  workspaces: [row],
  overlaps: [],
};
const ipc = vi.mocked(invoke);
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
beforeEach(() => {
  ipc.mockReset().mockResolvedValue(empty);
});
async function ready(listing = configured) {
  ipc.mockResolvedValueOnce(listing);
  const hook = renderHook(() => useWorkspaces());
  await waitFor(() => expect(hook.result.current.state.status).toBe('ready'));
  return hook;
}

it('loads a skipped empty configuration as ready without selection or checks', async () => {
  const { result } = renderHook(() => useWorkspaces());
  await waitFor(() =>
    expect(result.current.state).toEqual({ status: 'ready', listing: empty }),
  );
  expect(ipc).toHaveBeenCalledExactlyOnceWith('list_workspaces');
  expect(result.current.availability).toEqual({});
});

it('cancel does not mutate or reload and rapid add attempts open selection once', async () => {
  const { result } = await ready();
  const selection = deferred<{ status: 'cancelled' }>();
  ipc.mockReturnValueOnce(selection.promise);
  let add!: ReturnType<typeof result.current.add>;
  await act(async () => {
    add = result.current.add();
    await expect(result.current.add()).rejects.toMatchObject({
      code: 'operation_in_progress',
    });
  });
  expect(result.current.adding).toBe(true);
  expect(ipc.mock.calls.map(([name]) => name)).toEqual([
    'list_workspaces',
    'select_and_add_workspace',
  ]);
  await act(async () => {
    selection.resolve({ status: 'cancelled' });
    await add;
  });
  expect(result.current.state).toEqual({
    status: 'ready',
    listing: configured,
  });
  expect(result.current.adding).toBe(false);
});

it.each(['added', 'duplicate'] as const)(
  'reconciles %s with the authoritative listing',
  async (status) => {
    const { result } = await ready(empty);
    ipc
      .mockResolvedValueOnce(
        status === 'added'
          ? { status, workspace: row, availability: present }
          : { status, existingId: '7' },
      )
      .mockResolvedValueOnce(configured);
    await act(async () => {
      await result.current.add();
    });
    expect(result.current.state).toEqual({
      status: 'ready',
      listing: configured,
    });
    expect(result.current.availability).toEqual(
      status === 'added' ? { '7': present } : {},
    );
    expect(ipc.mock.calls.map(([name]) => name)).toEqual([
      'list_workspaces',
      'select_and_add_workspace',
      'list_workspaces',
    ]);
  },
);

it.each(['add', 'remove', 'setEnabled', 'skip'] as const)(
  'known SQL rejection in %s preserves configuration and setup',
  async (action) => {
    const { result } = await ready();
    ipc.mockRejectedValueOnce({
      code: 'persistence_unavailable',
      message: '/private/SQL',
    });
    await act(async () => {
      const promise =
        action === 'remove'
          ? result.current.remove('7')
          : action === 'setEnabled'
            ? result.current.setEnabled('7', false)
            : result.current[action]();
      await expect(promise).rejects.toBeInstanceOf(WorkspaceCommandError);
    });
    expect(result.current.state).toEqual({
      status: 'ready',
      listing: configured,
    });
    expect(result.current.error?.message).toBe(
      "Couldn't save workspace changes. Try again.",
    );
    expect(result.current.busyIds.size).toBe(0);
    expect(ipc).toHaveBeenCalledTimes(2);
  },
);

it('enabled and last removal change only after success and retain configured setup', async () => {
  const { result } = await ready();
  const enable = deferred<WorkspaceRecord>();
  ipc.mockReturnValueOnce(enable.promise).mockResolvedValueOnce({
    ...configured,
    workspaces: [{ ...row, enabled: false }],
  });
  let pending!: ReturnType<typeof result.current.setEnabled>;
  act(() => {
    pending = result.current.setEnabled('7', false);
  });
  expect(result.current.state).toEqual({
    status: 'ready',
    listing: configured,
  });
  expect(result.current.busyIds.has('7')).toBe(true);
  await act(async () => {
    enable.resolve({ ...row, enabled: false });
    await pending;
  });
  expect(result.current.state).toMatchObject({
    listing: { workspaces: [{ enabled: false }] },
  });
  const removal = deferred<{ removedId: string }>();
  ipc
    .mockReturnValueOnce(removal.promise)
    .mockResolvedValueOnce({ ...empty, setupState: 'configured' });
  let removing!: ReturnType<typeof result.current.remove>;
  act(() => {
    removing = result.current.remove('7');
  });
  expect(result.current.state).toMatchObject({
    listing: { workspaces: [{ id: '7' }] },
  });
  await act(async () => {
    removal.resolve({ removedId: '7' });
    await removing;
  });
  expect(result.current.state).toEqual({
    status: 'ready',
    listing: { ...empty, setupState: 'configured' },
  });
});

it('skip uses the committed setup response', async () => {
  const { result } = await ready({ ...empty, setupState: 'undecided' });
  ipc
    .mockResolvedValueOnce({ setupState: 'skipped' })
    .mockResolvedValueOnce(empty);
  await act(async () => {
    await result.current.skip();
  });
  expect(result.current.state).toEqual({ status: 'ready', listing: empty });
});

it.each(['resolve', 'reject'] as const)(
  'ignores stale StrictMode load %s',
  async (outcome) => {
    const stale = deferred<WorkspaceListing>();
    ipc.mockReturnValueOnce(stale.promise).mockResolvedValueOnce(configured);
    const { result } = renderHook(() => useWorkspaces(), {
      wrapper: StrictMode,
    });
    await waitFor(() =>
      expect(result.current.state).toEqual({
        status: 'ready',
        listing: configured,
      }),
    );
    await act(async () => {
      if (outcome === 'resolve') stale.resolve(empty);
      else stale.reject(new Error('late'));
      await stale.promise.catch(() => undefined);
    });
    expect(result.current.state).toEqual({
      status: 'ready',
      listing: configured,
    });
    expect(result.current.error).toBeNull();
  },
);

it('latest reload wins even when older reload resolves last', async () => {
  const { result } = await ready();
  const stale = deferred<WorkspaceListing>();
  ipc.mockReturnValueOnce(stale.promise).mockResolvedValueOnce(empty);
  let old!: Promise<void>;
  await act(async () => {
    old = result.current.reload();
    await result.current.reload();
  });
  await act(async () => {
    stale.resolve(configured);
    await old;
  });
  expect(result.current.state).toEqual({ status: 'ready', listing: empty });
});

it('latest explicit check wins and observations reset on a fresh hook session', async () => {
  const { result, unmount } = await ready();
  const old = deferred<{ id: string; availability: AvailabilityObservation }>();
  ipc
    .mockReturnValueOnce(old.promise)
    .mockResolvedValueOnce({ id: '7', availability: present });
  let stale!: ReturnType<typeof result.current.check>;
  await act(async () => {
    stale = result.current.check('7');
    await result.current.check('7');
  });
  await act(async () => {
    old.resolve({
      id: '7',
      availability: { state: 'missing', checkedAt: 1, reason: null },
    });
    await stale;
  });
  expect(result.current.availability).toEqual({ '7': present });
  unmount();
  const fresh = await ready();
  expect(fresh.result.current.availability).toEqual({});
});

it('a check resolving after removal cannot revive its observation or row', async () => {
  const { result } = await ready();
  const checked = deferred<{
    id: string;
    availability: AvailabilityObservation;
  }>();
  ipc
    .mockReturnValueOnce(checked.promise)
    .mockResolvedValueOnce({ removedId: '7' })
    .mockResolvedValueOnce({ ...empty, setupState: 'configured' });
  let pending!: ReturnType<typeof result.current.check>;
  await act(async () => {
    pending = result.current.check('7');
    await result.current.remove('7');
  });
  await act(async () => {
    checked.resolve({ id: '7', availability: present });
    await pending;
  });
  expect(result.current.availability).toEqual({});
  expect(result.current.state).toMatchObject({ listing: { workspaces: [] } });
});

it('a listing started before a newer mutation cannot roll that commit back', async () => {
  const second = { ...row, id: '8', displayName: 'second' };
  const two = { ...configured, workspaces: [row, second] };
  const { result } = await ready(two);
  const oldListing = deferred<WorkspaceListing>();
  ipc
    .mockResolvedValueOnce({ ...row, enabled: false })
    .mockReturnValueOnce(oldListing.promise);
  let disabling!: ReturnType<typeof result.current.setEnabled>;
  await act(async () => {
    disabling = result.current.setEnabled('7', false);
    await Promise.resolve();
    await Promise.resolve();
  });
  ipc.mockResolvedValueOnce({ removedId: '8' }).mockResolvedValueOnce({
    ...configured,
    workspaces: [{ ...row, enabled: false }],
  });
  await act(async () => {
    await result.current.remove('8');
  });
  await act(async () => {
    oldListing.resolve(two);
    await disabling;
  });
  expect(result.current.state).toMatchObject({
    listing: { workspaces: [{ id: '7', enabled: false }] },
  });
});

it('lost remove response reconciles a committed removal without retry or rollback claim', async () => {
  const { result } = await ready();
  ipc
    .mockRejectedValueOnce(new Error('/private/transport'))
    .mockResolvedValueOnce({ ...empty, setupState: 'configured' });
  await act(async () => {
    await expect(result.current.remove('7')).rejects.toMatchObject({
      outcomeUnknown: true,
      requiresReload: false,
    });
  });
  expect(result.current.state).toMatchObject({
    listing: { workspaces: [], setupState: 'configured' },
  });
  expect(result.current.error?.message).toMatch(/list was refreshed/i);
  expect(ipc.mock.calls.map(([name]) => name)).toEqual([
    'list_workspaces',
    'remove_workspace',
    'list_workspaces',
  ]);
});

it('failed reconciliation preserves committed rows and requires reload before retrying a mutation', async () => {
  const { result } = await ready();
  ipc
    .mockRejectedValueOnce(new Error('lost'))
    .mockRejectedValueOnce(new Error('offline'));
  await act(async () => {
    await expect(result.current.remove('7')).rejects.toMatchObject({
      outcomeUnknown: true,
      requiresReload: true,
    });
  });
  expect(result.current.state).toEqual({
    status: 'ready',
    listing: configured,
  });
  ipc.mockRejectedValueOnce(new Error('still offline'));
  await act(async () => {
    await expect(result.current.remove('7')).rejects.toMatchObject({
      requiresReload: true,
    });
  });
  expect(
    ipc.mock.calls.filter(([name]) => name === 'remove_workspace'),
  ).toHaveLength(1);
});

it('a successful commit survives a failed follow-up list', async () => {
  const { result } = await ready();
  ipc
    .mockResolvedValueOnce({ ...row, enabled: false })
    .mockRejectedValueOnce(new Error('offline'));
  await act(async () => {
    await expect(result.current.setEnabled('7', false)).resolves.toMatchObject({
      enabled: false,
    });
  });
  expect(result.current.state).toMatchObject({
    listing: { workspaces: [{ enabled: false }] },
  });
  expect(result.current.error).toBeInstanceOf(WorkspaceCommandError);
});

it.each(['resolve', 'reject'] as const)(
  'ignores late action %s after unmount',
  async (outcome) => {
    const { result, unmount } = await ready();
    const request = deferred<{ status: 'cancelled' }>();
    ipc.mockReturnValueOnce(request.promise);
    let pending!: ReturnType<typeof result.current.add>;
    act(() => {
      pending = result.current.add();
    });
    unmount();
    const snapshot = result.current;
    await act(async () => {
      if (outcome === 'resolve') request.resolve({ status: 'cancelled' });
      else request.reject(new Error('late'));
      await pending.catch(() => undefined);
    });
    expect(result.current).toBe(snapshot);
    expect(ipc).toHaveBeenCalledTimes(2);
  },
);

it('a failed latest check restores the last settled observation, not an older pending check', async () => {
  const { result } = await ready();
  ipc.mockResolvedValueOnce({ id: '7', availability: present });
  await act(async () => {
    await result.current.check('7');
  });
  const older = deferred<{
    id: string;
    availability: AvailabilityObservation;
  }>();
  ipc
    .mockReturnValueOnce(older.promise)
    .mockRejectedValueOnce({ code: 'application_unavailable' });
  let old!: ReturnType<typeof result.current.check>;
  await act(async () => {
    old = result.current.check('7');
    await expect(result.current.check('7')).rejects.toBeInstanceOf(
      WorkspaceCommandError,
    );
  });
  expect(result.current.availability).toEqual({ '7': present });
  await act(async () => {
    older.reject(new Error('stale failure'));
    await old.catch(() => undefined);
  });
  expect(result.current.availability).toEqual({ '7': present });
  expect(result.current.error?.source).toBe('backend');
});

it('a check for a stale backend ID reconciles removal without retrying metadata', async () => {
  const { result } = await ready();
  ipc
    .mockRejectedValueOnce({ code: 'workspace_not_found' })
    .mockResolvedValueOnce({ ...empty, setupState: 'configured' });
  await act(async () => {
    await expect(result.current.check('7')).rejects.toMatchObject({
      code: 'workspace_not_found',
    });
  });
  expect(result.current.state).toMatchObject({ listing: { workspaces: [] } });
  expect(result.current.availability).toEqual({});
});

it('initial load errors are typed and reload recovers without selecting folders', async () => {
  ipc.mockRejectedValueOnce(new Error('/private/SQL'));
  const { result } = renderHook(() => useWorkspaces());
  await waitFor(() => expect(result.current.state.status).toBe('error'));
  expect(result.current.error).toMatchObject({
    code: 'application_unavailable',
  });
  await act(async () => {
    await result.current.reload();
  });
  expect(result.current.state).toEqual({ status: 'ready', listing: empty });
  expect(result.current.error).toBeNull();
  expect(ipc.mock.calls.map(([name]) => name)).toEqual([
    'list_workspaces',
    'list_workspaces',
  ]);
});

it('a delayed skip response cannot downgrade a newer configured setup when refresh fails', async () => {
  const { result } = await ready({ ...empty, setupState: 'undecided' });
  const skipping = deferred<{ setupState: 'skipped' }>();
  ipc.mockReturnValueOnce(skipping.promise);
  let pending!: ReturnType<typeof result.current.skip>;
  act(() => {
    pending = result.current.skip();
  });
  ipc
    .mockResolvedValueOnce({
      status: 'added',
      workspace: row,
      availability: present,
    })
    .mockResolvedValueOnce(configured);
  await act(async () => {
    await result.current.add();
  });
  ipc.mockRejectedValueOnce(new Error('offline'));
  await act(async () => {
    skipping.resolve({ setupState: 'skipped' });
    await pending;
  });
  expect(result.current.state).toEqual({
    status: 'ready',
    listing: configured,
  });
});
