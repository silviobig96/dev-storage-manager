import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WorkspacesView } from './WorkspacesView';
import { useWorkspaces } from '../features/workspaces/useWorkspaces';
import * as api from '../lib/workspaces';
import type { WorkspaceScanController } from '../features/scans/useWorkspaceScan';
import {
  scanController,
  scanSnapshot,
} from '../features/scans/scanTestFixtures';

vi.mock('../lib/workspaces', async (original) => ({
  ...(await original<typeof import('../lib/workspaces')>()),
  listWorkspaces: vi.fn(),
  selectAndAddWorkspace: vi.fn(),
  removeWorkspace: vi.fn(),
  setWorkspaceEnabled: vi.fn(),
  checkWorkspace: vi.fn(),
  skipWorkspaceSetup: vi.fn(),
}));
const first: api.WorkspaceRecord = {
  id: '1',
  displayName: 'Code',
  selectedPath: '/fixture/one/Code',
  enabled: true,
  createdAt: 1,
  updatedAt: 1,
};
const second: api.WorkspaceRecord = {
  ...first,
  id: '2',
  selectedPath:
    '/fixture/a-very-long-directory-name/another-long-directory/コード/Code',
};
const listing = (rows = [first, second]): api.WorkspaceListing => ({
  setupState: 'configured',
  workspaces: rows,
  overlaps: [],
});
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (value: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
function Harness({
  scan,
  busyIds,
}: {
  scan?: WorkspaceScanController;
  busyIds?: Set<string>;
}) {
  const workspaces = useWorkspaces();
  return (
    <WorkspacesView
      workspaces={busyIds ? { ...workspaces, busyIds } : workspaces}
      scan={scan}
    />
  );
}
const action = (verb: string, row = first) =>
  `${verb} ${row.displayName} — ${row.selectedPath}`;

const originalDialogMethods = Object.getOwnPropertyDescriptors(
  HTMLDialogElement.prototype,
);
let dialogFocusEvents: string[] = [];
// Model close() restoring the element focused before showModal(). This does
// not implement the native top layer, focus trap, or actual Escape delivery.
beforeEach(() => {
  vi.resetAllMocks();
  dialogFocusEvents = [];
  const previousFocus = new WeakMap<HTMLDialogElement, HTMLElement | null>();
  HTMLDialogElement.prototype.showModal = function () {
    previousFocus.set(this, document.activeElement as HTMLElement | null);
    dialogFocusEvents.push('showModal');
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    dialogFocusEvents.push('close');
    this.open = false;
    previousFocus.get(this)?.focus();
  };
  vi.mocked(api.listWorkspaces).mockResolvedValue(listing());
});
afterEach(() => {
  // Unmount while the polyfill is installed, then restore the test environment.
  cleanup();
  for (const name of ['showModal', 'close']) {
    const original = originalDialogMethods[name];
    if (original)
      Object.defineProperty(HTMLDialogElement.prototype, name, original);
    else Reflect.deleteProperty(HTMLDialogElement.prototype, name);
  }
});

describe('Workspaces view', () => {
  it.each(['Cancel', 'Escape'] as const)(
    'restores usable row focus after %s closes a dialog frozen by late scan status',
    async (dismissal) => {
      const scan = scanController();
      const { rerender } = render(<Harness scan={scan} />);
      await userEvent.click(
        await screen.findByRole('button', { name: action('Remove workspace') }),
      );
      const dialog = screen.getByRole('dialog', { name: 'Remove workspace?' });
      const cancel = within(dialog).getByRole('button', { name: 'Cancel' });
      expect(cancel).toHaveFocus();
      expect(
        within(dialog).getByText(
          'This only removes the workspace from the app.',
        ),
      ).toBeVisible();
      expect(
        within(dialog).getByText('Files on disk will not be deleted.'),
      ).toBeVisible();
      expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
      rerender(
        <Harness
          scan={{
            ...scan,
            snapshot: scanSnapshot({ state: 'running', workerActive: true }),
          }}
        />,
      );
      expect(cancel).toHaveFocus();
      expect(
        within(dialog).getByRole('button', { name: 'Remove workspace' }),
      ).toBeDisabled();
      expect(api.removeWorkspace).not.toHaveBeenCalled();
      if (dismissal === 'Cancel') await userEvent.click(cancel);
      else fireEvent(dialog, new Event('cancel', { cancelable: true }));
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      expect(
        screen.getByRole('button', { name: action('Check availability for') }),
      ).toHaveFocus();
    },
  );
  it.each([
    { busyIds: new Set(['1']), focus: 'neighbor' },
    { busyIds: new Set(['1', '2']), focus: 'heading' },
  ])(
    'falls back to $focus when scan freezing leaves the original row without usable controls',
    async ({ busyIds, focus }) => {
      const scan = scanController();
      const { rerender } = render(<Harness scan={scan} />);
      await userEvent.click(
        await screen.findByRole('button', { name: action('Remove workspace') }),
      );
      const dialog = screen.getByRole('dialog', { name: 'Remove workspace?' });
      rerender(
        <Harness scan={{ ...scan, status: 'unavailable' }} busyIds={busyIds} />,
      );
      fireEvent(dialog, new Event('cancel', { cancelable: true }));
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      if (focus === 'neighbor')
        expect(
          screen.getByRole('button', {
            name: action('Check availability for', second),
          }),
        ).toHaveFocus();
      else
        expect(
          screen.getByRole('heading', { level: 1, name: 'Workspaces' }),
        ).toHaveFocus();
      expect(api.removeWorkspace).not.toHaveBeenCalled();
    },
  );
  it.each(['ready', 'unavailable', 'reconciling'] as const)(
    'freezes writes with %s scan state while keeping manual checks usable',
    async (status) => {
      const scan = scanController({
        status,
        snapshot: scanSnapshot({
          workerActive: status === 'ready',
          state: status === 'ready' ? 'stopping' : 'completed',
        }),
      });
      render(<Harness scan={scan} />);
      const add = await screen.findByRole('button', { name: 'Add workspace' });
      expect(add).toBeDisabled();
      expect(
        screen.getByRole('checkbox', { name: action('Enabled for') }),
      ).toBeDisabled();
      expect(
        screen.getByRole('button', { name: action('Remove workspace') }),
      ).toBeDisabled();
      expect(
        screen.getByRole('button', { name: action('Check availability for') }),
      ).toBeEnabled();
      expect(screen.getByText(/Workspace changes are paused/)).toBeVisible();
      expect(
        within(document.getElementById('workspace-1')!).getByText(
          'Observed file sizes',
        ),
      ).toBeVisible();
    },
  );
  it('preserves all 33 enabled rows and blocks starting without changing configuration', async () => {
    vi.mocked(api.listWorkspaces).mockResolvedValue(
      listing(
        Array.from({ length: 33 }, (_, i) => ({
          ...first,
          id: String(i + 1),
          selectedPath: `/fixture/${i + 1}`,
        })),
      ),
    );
    const scan = scanController();
    render(<Harness scan={scan} />);
    await screen.findByText(
      'A scan supports up to 32 enabled workspaces. Disable some workspaces to scan.',
    );
    expect(screen.getAllByRole('checkbox')).toHaveLength(33);
    for (const checkbox of screen.getAllByRole('checkbox')) {
      expect(checkbox).toBeChecked();
      expect(checkbox).toBeEnabled();
    }
    expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
    expect(scan.start).not.toHaveBeenCalled();
    expect(api.setWorkspaceEnabled).not.toHaveBeenCalled();
  });
  it('blocks scan start while the native picker is already open', async () => {
    vi.mocked(api.selectAndAddWorkspace).mockReturnValue(
      new Promise(() => undefined),
    );
    render(<Harness scan={scanController()} />);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Add workspace' }),
    );
    expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Deep scan' })).toBeDisabled();
  });
  it('announces availability in its row without a redundant generic success message', async () => {
    vi.mocked(api.listWorkspaces).mockResolvedValue(listing([first]));
    vi.mocked(api.checkWorkspace).mockResolvedValue({
      id: '1',
      availability: { state: 'present', reason: null, checkedAt: 1 },
    });
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', {
        name: action('Check availability for'),
      }),
    );
    const available = await screen.findByText('Available');
    expect(available.closest('[aria-live]')).toHaveAttribute(
      'aria-live',
      'polite',
    );
    expect(available.closest('[aria-live]')).toHaveAttribute(
      'aria-atomic',
      'true',
    );
    expect(available.closest('[aria-live]')).toHaveTextContent(
      first.displayName,
    );
    expect(available.closest('[aria-live]')).toHaveTextContent(
      first.selectedPath,
    );
    expect(screen.getByText('Contents have not been checked.')).toBeVisible();
    expect(
      screen.queryByText('Folder availability checked.'),
    ).not.toBeInTheDocument();
  });

  it('presents a missing root as a warning rather than a successful availability result', async () => {
    vi.mocked(api.listWorkspaces).mockResolvedValue(listing([first]));
    vi.mocked(api.checkWorkspace).mockResolvedValue({
      id: '1',
      availability: { state: 'missing', reason: null, checkedAt: 1 },
    });
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', {
        name: action('Check availability for'),
      }),
    );
    const warning = await screen.findByText('Folder unavailable');
    expect(warning.closest('[aria-live]')).toHaveTextContent(
      'Folder not found. Reconnect the disk or check the saved location.',
    );
    expect(screen.queryByText('Available')).not.toBeInTheDocument();
    expect(screen.getByRole('status')).toBeEmptyDOMElement();
    expect(
      screen.getByRole('checkbox', { name: action('Enabled for') }),
    ).toBeChecked();
  });

  it('keeps reload recovery inside the persistent error feedback', async () => {
    vi.mocked(api.setWorkspaceEnabled).mockRejectedValue(
      new Error('lost response'),
    );
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing([first]))
      .mockRejectedValue(new Error('offline'));
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('checkbox', { name: action('Enabled for') }),
    );
    const error = screen.getByRole('alert');
    expect(within(error).getByText('Workspace needs attention')).toBeVisible();
    expect(error).toHaveTextContent('Reload workspaces before trying again.');
    expect(
      within(error).getByRole('button', { name: 'Reload workspaces' }),
    ).toBeEnabled();
    expect(
      screen.getByRole('checkbox', { name: action('Enabled for') }),
    ).toBeChecked();
  });

  it('confirms the committed enabled choice with an icon and specific visible text', async () => {
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing([first]))
      .mockResolvedValue(listing([{ ...first, enabled: false }]));
    vi.mocked(api.setWorkspaceEnabled).mockResolvedValue({
      ...first,
      enabled: false,
    });
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('checkbox', { name: action('Enabled for') }),
    );
    const feedback = screen.getByRole('status');
    expect(feedback).toHaveTextContent('Workspace disabled.');
    expect(feedback.querySelector('svg')).toHaveAttribute(
      'aria-hidden',
      'true',
    );
    expect(
      screen.getByRole('checkbox', { name: action('Enabled for') }),
    ).not.toBeChecked();
  });

  it('distinguishes loading, failed loading, retry, and configured empty', async () => {
    const load = deferred<api.WorkspaceListing>();
    vi.mocked(api.listWorkspaces)
      .mockReturnValueOnce(load.promise)
      .mockResolvedValue(listing([]));
    render(<Harness />);
    expect(screen.getByRole('status')).toHaveTextContent('Loading workspaces…');
    expect(
      screen.queryByRole('button', { name: 'Add workspace' }),
    ).not.toBeInTheDocument();
    await act(async () => {
      load.reject(new Error('offline'));
      await load.promise.catch(() => undefined);
    });
    expect(screen.getByText("Couldn't load workspaces.")).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByText('No workspaces added')).toBeVisible();
    expect(
      screen.queryByRole('button', { name: 'Skip for now' }),
    ).not.toBeInTheDocument();
    expect(api.selectAndAddWorkspace).not.toHaveBeenCalled();
    expect(api.checkWorkspace).not.toHaveBeenCalled();
  });

  it('keeps complete paths and unique action names and sends only the selected ID', async () => {
    vi.mocked(api.listWorkspaces).mockResolvedValue({
      ...listing(),
      overlaps: [{ parentId: '1', childId: '2' }],
    });
    const check = deferred<api.CheckWorkspaceResult>();
    vi.mocked(api.checkWorkspace).mockReturnValue(check.promise);
    render(<Harness />);
    expect(await screen.findByText(second.selectedPath)).toBeVisible();
    expect(screen.getByRole('list')).toHaveAttribute('role', 'list');
    expect(screen.getAllByText('Not checked')).toHaveLength(2);
    expect(
      screen.getAllByText(
        'Overlaps another workspace. Scan contributions avoid double counting.',
      ),
    ).toHaveLength(2);
    await userEvent.click(
      screen.getByRole('button', {
        name: action('Check availability for', second),
      }),
    );
    expect(api.checkWorkspace).toHaveBeenCalledWith('2');
    expect(screen.getByText('Checking folder…')).toBeVisible();
    expect(
      screen.getByRole('button', { name: action('Remove workspace', second) }),
    ).toBeDisabled();
    await act(async () => {
      check.resolve({
        id: '2',
        availability: { state: 'present', reason: null, checkedAt: 1 },
      });
      await check.promise;
    });
    expect(screen.getByText('Available')).toBeVisible();
    expect(screen.getByText('Contents have not been checked.')).toBeVisible();
    expect(api.setWorkspaceEnabled).not.toHaveBeenCalled();
  });

  it.each([
    [
      'missing',
      null,
      'Folder not found. Reconnect the disk or check the saved location.',
    ],
    ['denied', null, 'Access to this folder was denied.'],
    [
      'unknown',
      'timeout',
      "Couldn't confirm availability. Try checking again.",
    ],
    [
      'unknown',
      'identity_changed',
      'The location now refers to a different folder. Remove and add it again to change configuration.',
    ],
  ] as const)(
    'reports %s without implying content access',
    async (state, reason, text) => {
      vi.mocked(api.listWorkspaces).mockResolvedValue(listing([first]));
      vi.mocked(api.checkWorkspace).mockResolvedValue({
        id: '1',
        availability: { state, reason, checkedAt: 1 },
      });
      render(<Harness />);
      await userEvent.click(
        await screen.findByRole('button', {
          name: action('Check availability for'),
        }),
      );
      expect(await screen.findByText(text)).toBeVisible();
      expect(
        screen.getByRole('checkbox', { name: action('Enabled for') }),
      ).toBeChecked();
    },
  );

  it('retains committed Enabled while saving and catches persistence failure', async () => {
    const save = deferred<api.WorkspaceRecord>();
    vi.mocked(api.setWorkspaceEnabled).mockReturnValue(save.promise);
    render(<Harness />);
    const enabled = await screen.findByRole('checkbox', {
      name: action('Enabled for', second),
    });
    await userEvent.click(enabled);
    expect(api.setWorkspaceEnabled).toHaveBeenCalledWith('2', false);
    expect(enabled).toBeChecked();
    expect(enabled).toBeDisabled();
    expect(
      screen.getByRole('checkbox', { name: action('Enabled for') }),
    ).toBeEnabled();
    await act(async () => {
      save.reject(new api.WorkspaceCommandError('persistence_unavailable'));
      await save.promise.catch(() => undefined);
    });
    expect(screen.getByRole('alert')).toHaveTextContent(
      "Couldn't save workspace changes. Try again.",
    );
    expect(enabled).toBeChecked();
    expect(enabled).toBeEnabled();
  });

  it('clears an earlier save announcement before a later rejected save', async () => {
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing([first]))
      .mockResolvedValue(listing([{ ...first, enabled: false }]));
    vi.mocked(api.setWorkspaceEnabled)
      .mockResolvedValueOnce({ ...first, enabled: false })
      .mockRejectedValueOnce(
        new api.WorkspaceCommandError('persistence_unavailable'),
      );
    render(<Harness />);
    const enabled = await screen.findByRole('checkbox', {
      name: action('Enabled for'),
    });
    await userEvent.click(enabled);
    expect(screen.getByRole('status')).toHaveTextContent('Workspace disabled.');
    await userEvent.click(enabled);
    expect(screen.getByRole('alert')).toHaveTextContent(
      "Couldn't save workspace changes. Try again.",
    );
    expect(screen.getByRole('status')).not.toHaveTextContent(
      'Workspace disabled.',
    );
    expect(enabled).not.toBeChecked();
  });

  it('restores Add focus on cancellation and focuses the existing row on duplicate', async () => {
    vi.mocked(api.selectAndAddWorkspace)
      .mockResolvedValueOnce({ status: 'cancelled' })
      .mockResolvedValueOnce({ status: 'duplicate', existingId: '2' });
    render(<Harness />);
    const add = await screen.findByRole('button', { name: 'Add workspace' });
    await userEvent.click(add);
    expect(add).toHaveFocus();
    await userEvent.click(add);
    expect(
      await screen.findByText('This folder is already configured.'),
    ).toBeVisible();
    expect(screen.getByText(second.selectedPath).closest('li')).toHaveFocus();
    expect(
      screen.queryByText('Workspace added. No files were scanned.'),
    ).not.toBeInTheDocument();
  });

  it('shows added rows only after commit and announces no scan', async () => {
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing([]))
      .mockResolvedValue(listing([first]));
    const add = deferred<api.AddWorkspaceOutcome>();
    vi.mocked(api.selectAndAddWorkspace).mockReturnValue(add.promise);
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Add workspace' }),
    );
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
    expect(screen.queryByText(first.selectedPath)).not.toBeInTheDocument();
    await act(async () => {
      add.resolve({
        status: 'added',
        workspace: first,
        availability: { state: 'present', reason: null, checkedAt: 1 },
      });
      await add.promise;
    });
    expect(screen.getByText(first.selectedPath)).toBeVisible();
    expect(screen.getByRole('status')).toHaveTextContent(
      'Workspace added. No files were scanned.',
    );
  });

  it.each(['next', 'previous', 'empty'] as const)(
    'removes pessimistically then focuses %s action',
    async (target) => {
      const rows = target === 'empty' ? [first] : [first, second];
      const removed = target === 'previous' ? second : first;
      vi.mocked(api.listWorkspaces)
        .mockResolvedValueOnce(listing(rows))
        .mockResolvedValue(
          listing(rows.filter((row) => row.id !== removed.id)),
        );
      const save = deferred<api.RemoveWorkspaceResult>();
      vi.mocked(api.removeWorkspace).mockReturnValue(save.promise);
      render(<Harness />);
      await userEvent.click(
        await screen.findByRole('button', {
          name: action('Remove workspace', removed),
        }),
      );
      const dialog = screen.getByRole('dialog', { name: 'Remove workspace?' });
      expect(
        within(dialog).getByRole('button', { name: 'Cancel' }),
      ).toHaveFocus();
      await userEvent.click(
        within(dialog).getByRole('button', { name: 'Remove workspace' }),
      );
      expect(api.removeWorkspace).toHaveBeenCalledExactlyOnceWith(removed.id);
      expect(screen.getAllByText(removed.selectedPath)).toHaveLength(2);
      expect(dialog).toHaveAttribute('aria-busy', 'true');
      await act(async () => {
        save.resolve({ removedId: removed.id });
        await save.promise;
      });
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      expect(screen.queryByText(removed.selectedPath)).not.toBeInTheDocument();
      expect(
        target === 'empty'
          ? screen.getByRole('button', { name: 'Add workspace' })
          : screen.getByRole('checkbox', {
              name: action('Enabled for', target === 'next' ? second : first),
            }),
      ).toHaveFocus();
      expect(
        screen.queryByRole('button', { name: 'Skip for now' }),
      ).not.toBeInTheDocument();
    },
  );

  it('retains failed removal in the dialog and restores its invoker on cancellation', async () => {
    vi.mocked(api.removeWorkspace).mockRejectedValue(
      new api.WorkspaceCommandError('persistence_unavailable'),
    );
    render(<Harness />);
    const invoker = await screen.findByRole('button', {
      name: action('Remove workspace'),
    });
    await userEvent.click(invoker);
    const dialog = screen.getByRole('dialog');
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'Remove workspace' }),
    );
    expect(within(dialog).getByRole('alert')).toHaveTextContent(
      "Couldn't save workspace changes. Try again.",
    );
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'Cancel' }),
    );
    expect(invoker).toHaveFocus();
    expect(screen.getByText(first.selectedPath)).toBeVisible();
  });

  it('restores Cancel focus only after the modal has left the document', async () => {
    render(<Harness />);
    const invoker = await screen.findByRole('button', {
      name: action('Remove workspace'),
    });
    await userEvent.click(invoker);
    let dialogPresentAtFocus: boolean | undefined;
    invoker.addEventListener('focus', () => {
      dialogPresentAtFocus = screen.queryByRole('dialog') !== null;
    });
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Cancel',
      }),
    );
    expect(invoker).toHaveFocus();
    expect(dialogPresentAtFocus).toBe(false);
  });

  it.each(['Cancel', 'Escape'] as const)(
    'restores the actual Remove trigger on %s when activation leaves focus on the row',
    async (dismissal) => {
      render(<Harness />);
      const trigger = await screen.findByRole('button', {
        name: action('Remove workspace'),
      });
      const row = screen.getByText(first.selectedPath).closest('li');
      row?.focus();
      expect(row).toHaveFocus();
      row?.addEventListener('focus', () =>
        dialogFocusEvents.push('previous focus'),
      );
      trigger.addEventListener('focus', () =>
        dialogFocusEvents.push('trigger focus'),
      );
      // WebKit mouse/AX activation need not focus the clicked button.
      // fireEvent preserves that pre-existing focus, unlike userEvent.click.
      fireEvent.click(trigger);
      const dialog = screen.getByRole('dialog');
      expect(
        within(dialog).getByRole('button', { name: 'Cancel' }),
      ).toHaveFocus();
      if (dismissal === 'Cancel') {
        fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
      } else {
        // jsdom does not deliver native Escape; exercise its cancel event.
        fireEvent(dialog, new Event('cancel', { cancelable: true }));
      }
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      expect(dialogFocusEvents).toEqual([
        'showModal',
        'close',
        'previous focus',
        'trigger focus',
      ]);
      expect(trigger).toHaveFocus();
      expect(api.removeWorkspace).not.toHaveBeenCalled();
    },
  );

  it('gates uncertain mutation retries until a successful reload and recovers focus when removal committed', async () => {
    vi.mocked(api.removeWorkspace).mockRejectedValue(
      new Error('lost response'),
    );
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing([first]))
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue(listing([]));
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', { name: action('Remove workspace') }),
    );
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Remove workspace',
      }),
    );
    expect(
      within(screen.getByRole('dialog')).getByRole('alert'),
    ).toHaveTextContent('Reload workspaces before trying again.');
    expect(
      within(within(screen.getByRole('dialog')).getByRole('alert')).getByRole(
        'button',
        { name: 'Reload workspaces' },
      ),
    ).toBeEnabled();
    expect(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Remove workspace',
      }),
    ).toBeDisabled();
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Reload workspaces',
      }),
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add workspace' })).toHaveFocus();
  });

  it('keeps the reload gate after a second failed reload', async () => {
    vi.mocked(api.removeWorkspace).mockRejectedValue(
      new Error('lost response'),
    );
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing([first]))
      .mockRejectedValue(new Error('offline'));
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', { name: action('Remove workspace') }),
    );
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Remove workspace',
      }),
    );
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Reload workspaces',
      }),
    );
    expect(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Remove workspace',
      }),
    ).toBeDisabled();
    expect(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Reload workspaces',
      }),
    ).toBeEnabled();
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Cancel',
      }),
    );
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
  });

  it('keeps mutation controls gated when another row Check rejects after uncertain removal', async () => {
    const check = deferred<api.CheckWorkspaceResult>();
    vi.mocked(api.checkWorkspace).mockReturnValue(check.promise);
    vi.mocked(api.listWorkspaces)
      .mockResolvedValueOnce(listing())
      .mockRejectedValue(new Error('offline'));
    vi.mocked(api.removeWorkspace).mockRejectedValue(
      new Error('lost response'),
    );
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', {
        name: action('Check availability for'),
      }),
    );
    await userEvent.click(
      screen.getByRole('button', { name: action('Remove workspace', second) }),
    );
    await userEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', {
        name: 'Remove workspace',
      }),
    );
    await act(async () => {
      check.reject(new api.WorkspaceCommandError('application_unavailable'));
      await check.promise.catch(() => undefined);
    });
    const dialog = screen.getByRole('dialog');
    expect(
      within(dialog).getByRole('button', { name: 'Remove workspace' }),
    ).toBeDisabled();
    expect(
      within(dialog).getByRole('button', { name: 'Reload workspaces' }),
    ).toBeEnabled();
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
    expect(
      screen.getByRole('checkbox', { name: action('Enabled for') }),
    ).toBeDisabled();
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'Remove workspace' }),
    );
    expect(api.removeWorkspace).toHaveBeenCalledExactlyOnceWith('2');
  });

  it('catches a rejected manual Check and keeps the prior observation actionable', async () => {
    vi.mocked(api.checkWorkspace).mockRejectedValue(
      new api.WorkspaceCommandError('application_unavailable'),
    );
    render(<Harness />);
    const check = await screen.findByRole('button', {
      name: action('Check availability for'),
    });
    await userEvent.click(check);
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Workspace services are unavailable. Try again.',
    );
    expect(check).toBeEnabled();
    expect(screen.getAllByText('Not checked')).toHaveLength(2);
  });

  it('keeps Retry available after a rejected retry without presenting empty configuration', async () => {
    vi.mocked(api.listWorkspaces).mockRejectedValue(new Error('offline'));
    render(<Harness />);
    await userEvent.click(await screen.findByRole('button', { name: 'Retry' }));
    expect(screen.getByText("Couldn't load workspaces.")).toBeVisible();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeEnabled();
    expect(screen.queryByText('No workspaces added')).not.toBeInTheDocument();
  });

  it('catches failed Add and Skip handlers without unhandled rejections', async () => {
    vi.mocked(api.listWorkspaces).mockResolvedValue({
      setupState: 'undecided',
      workspaces: [],
      overlaps: [],
    });
    vi.mocked(api.selectAndAddWorkspace).mockRejectedValue(
      new api.WorkspaceCommandError('access_denied'),
    );
    vi.mocked(api.skipWorkspaceSetup).mockRejectedValue(
      new api.WorkspaceCommandError('persistence_unavailable'),
    );
    render(<Harness />);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Add workspace' }),
    );
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Access to this folder was denied.',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Skip for now' }));
    expect(screen.getByRole('alert')).toHaveTextContent(
      "Couldn't save workspace changes. Try again.",
    );
    expect(
      screen.getAllByRole('button', { name: 'Add workspace' }),
    ).toHaveLength(1);
    expect(
      screen.queryByText('Choose workspace folders'),
    ).not.toBeInTheDocument();
  });
});
