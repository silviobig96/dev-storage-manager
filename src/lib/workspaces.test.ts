import { expect, it, vi } from 'vitest';
import {
  checkWorkspace,
  listWorkspaces,
  removeWorkspace,
  selectAndAddWorkspace,
  setWorkspaceEnabled,
  skipWorkspaceSetup,
  WorkspaceCommandError,
} from './workspaces';

it('workspace bridge sends only narrow arguments', async () => {
  const invoke = vi
    .fn()
    .mockResolvedValue({ setupState: 'skipped', workspaces: [], overlaps: [] });
  await listWorkspaces(invoke);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('list_workspaces');
  invoke.mockClear();
  await selectAndAddWorkspace(invoke);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('select_and_add_workspace');
  invoke.mockClear();
  await removeWorkspace('9007199254740993', invoke);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('remove_workspace', {
    id: '9007199254740993',
  });
  invoke.mockClear();
  await setWorkspaceEnabled('7', false, invoke);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('set_workspace_enabled', {
    id: '7',
    enabled: false,
  });
  invoke.mockClear();
  await checkWorkspace('7', invoke);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('check_workspace', {
    id: '7',
  });
  invoke.mockClear();
  await skipWorkspaceSetup(invoke);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('skip_workspace_setup');
});

it('resolves cancellation and sanitizes known backend rejections including optional string ID', async () => {
  await expect(
    selectAndAddWorkspace(vi.fn().mockResolvedValue({ status: 'cancelled' })),
  ).resolves.toEqual({ status: 'cancelled' });
  const promise = selectAndAddWorkspace(
    vi.fn().mockRejectedValue({
      code: 'path_already_configured',
      message: '/private/raw.sql',
      existingId: '9',
    }),
  );
  await expect(promise).rejects.toBeInstanceOf(WorkspaceCommandError);
  await expect(promise).rejects.toMatchObject({
    code: 'path_already_configured',
    existingId: '9',
    source: 'backend',
    message:
      'This location has changed. Remove its old entry before adding it again.',
  });
});

it.each([
  new Error('/private/raw.sql'),
  'invalid args /private/path',
  { code: 'unknown', message: 'SQL' },
  { code: 'toString' },
])('sanitizes unrecognized transport failures', async (failure) => {
  await expect(
    listWorkspaces(vi.fn().mockRejectedValue(failure)),
  ).rejects.toMatchObject({
    code: 'application_unavailable',
    source: 'transport',
    message: 'Workspace services are unavailable. Try again.',
  });
});
