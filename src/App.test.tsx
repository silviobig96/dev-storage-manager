import { StrictMode } from 'react';
import { act, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import userEvent from '@testing-library/user-event';
import { scanSnapshot } from './features/scans/scanTestFixtures';
import {
  listWorkspaces,
  selectAndAddWorkspace,
  skipWorkspaceSetup,
  removeWorkspace,
  checkWorkspace,
} from './lib/workspaces';
import { getAppHealth, type AppHealth } from './lib/desktop';
import {
  getWorkspaceScan,
  startWorkspaceScan,
  cancelWorkspaceScan,
} from './lib/scans';

vi.mock('./lib/scans', async (original) => ({
  ...(await original<typeof import('./lib/scans')>()),
  getWorkspaceScan: vi.fn(),
  startWorkspaceScan: vi.fn(),
  cancelWorkspaceScan: vi.fn(),
}));

vi.mock('./lib/desktop', () => ({ getAppHealth: vi.fn() }));
vi.mock('./lib/workspaces', async (original) => ({
  ...(await original<typeof import('./lib/workspaces')>()),
  listWorkspaces: vi.fn(),
  selectAndAddWorkspace: vi.fn(),
  skipWorkspaceSetup: vi.fn(),
  removeWorkspace: vi.fn(),
  checkWorkspace: vi.fn(),
}));

const health: AppHealth = {
  appName: 'dev-storage-manager',
  appVersion: '0.1.0',
  platform: 'macos',
  databaseSchemaVersion: 2,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe('App', () => {
  it('freezes empty setup on unknown scan state while health and navigation remain usable', async () => {
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'undecided',
      workspaces: [],
      overlaps: [],
    });
    vi.mocked(getWorkspaceScan).mockRejectedValue(new Error('offline'));
    render(<App />);
    expect(await screen.findByText(/foundation connected/i)).toBeVisible();
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Skip for now' })).toBeDisabled();
    expect(screen.getByText(/Workspace changes are paused/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(screen.getByRole('button', { name: 'Reconnect' })).toBeEnabled();
    expect(selectAndAddWorkspace).not.toHaveBeenCalled();
    expect(skipWorkspaceSetup).not.toHaveBeenCalled();
  });
  it('shares an explicitly started scan across views without moving navigation focus or permitting setup writes', async () => {
    const row = {
      id: '1',
      displayName: 'Code',
      selectedPath: '/fixture/one/Code',
      enabled: true,
      createdAt: 1,
      updatedAt: 1,
    };
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'configured',
      workspaces: [row],
      overlaps: [],
    });
    const response = deferred<ReturnType<typeof scanSnapshot>>();
    vi.mocked(startWorkspaceScan).mockReturnValue(response.promise);
    render(<App />);
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    await userEvent.click(
      await screen.findByRole('button', { name: 'Quick scan' }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
    const heading = screen.getByRole('heading', { level: 1, name: 'Overview' });
    expect(heading).toHaveFocus();
    const request = vi.mocked(startWorkspaceScan).mock.calls[0]![0];
    await act(async () => {
      response.resolve(scanSnapshot({ requestId: request.requestId }));
      await response.promise;
    });
    expect(
      screen.getByText('Scan completed within the declared scope.'),
    ).toBeVisible();
    expect(heading).toHaveFocus();
    expect(startWorkspaceScan).toHaveBeenCalledOnce();
    expect(
      screen.queryByRole('button', { name: 'Quick scan' }),
    ).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(
      screen.getByText('Scan completed within the declared scope.'),
    ).toBeVisible();
    expect(startWorkspaceScan).toHaveBeenCalledOnce();
  });
  it('keeps setup reload recovery inside its error without re-enabling uncertain mutations', async () => {
    vi.mocked(listWorkspaces)
      .mockResolvedValueOnce({
        setupState: 'undecided',
        workspaces: [],
        overlaps: [],
      })
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue({
        setupState: 'skipped',
        workspaces: [],
        overlaps: [],
      });
    vi.mocked(skipWorkspaceSetup).mockRejectedValue(new Error('lost response'));
    render(<App />);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Skip for now' }),
    );
    const error = screen.getByRole('alert');
    expect(error).toHaveTextContent('Setup needs attention');
    const reload = within(error).getByRole('button', {
      name: 'Reload workspaces',
    });
    expect(reload).toBeEnabled();
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Skip for now' })).toBeDisabled();
    await userEvent.click(reload);
    expect(
      screen.queryByText('Choose workspace folders'),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'Overview', level: 1 }),
    ).toBeVisible();
  });
  beforeEach(() => {
    vi.mocked(getWorkspaceScan)
      .mockReset()
      .mockResolvedValue({ epoch: 'p1-t1-g0', session: null });
    vi.mocked(startWorkspaceScan).mockReset();
    vi.mocked(cancelWorkspaceScan).mockReset();
    vi.mocked(getAppHealth).mockReset().mockResolvedValue(health);
    vi.mocked(listWorkspaces).mockReset().mockResolvedValue({
      setupState: 'skipped',
      workspaces: [],
      overlaps: [],
    });
    vi.mocked(selectAndAddWorkspace).mockReset();
    vi.mocked(skipWorkspaceSetup).mockReset();
    vi.mocked(removeWorkspace).mockReset();
    vi.mocked(checkWorkspace).mockReset();
  });

  it('owns scan recovery across navigation without starting or cancelling work', async () => {
    const { unmount } = render(<App />);
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
    expect(getWorkspaceScan).toHaveBeenCalledTimes(1);
    unmount();
    expect(startWorkspaceScan).not.toHaveBeenCalled();
    expect(cancelWorkspaceScan).not.toHaveBeenCalled();
  });

  it('renders the shell while health is loading', () => {
    vi.mocked(getAppHealth).mockReturnValue(new Promise(() => undefined));
    render(<App />);
    expect(screen.getByRole('navigation', { name: 'Main' })).toBeVisible();
    expect(
      screen.getByRole('heading', { level: 1, name: 'Overview' }),
    ).toBeVisible();
    expect(screen.getByRole('status')).toHaveTextContent(
      /connecting to desktop foundation/i,
    );
  });

  it('reports the connected schema version without adding storage actions', async () => {
    render(<App />);
    expect(await screen.findByText(/foundation connected/i)).toBeVisible();
    expect(screen.getByRole('status')).toHaveTextContent(/schema 2/i);
    expect(getAppHealth).toHaveBeenCalledOnce();
    expect(screen.getByText('No automatic deletion')).toBeVisible();
    expect(
      screen.getByText('Overview', { selector: '.sidebar-item span' })
        .parentElement,
    ).toHaveAttribute('aria-current', 'page');
    expect(screen.getAllByRole('button')).toHaveLength(2);
    expect(
      screen.queryByText(/\d+(\.\d+)?\s*[KMGTP]B/),
    ).not.toBeInTheDocument();
  });

  it('focuses only explicit navigation headings and never opens the picker on load or navigation', async () => {
    render(<App />);
    await screen.findByText(/foundation connected/i);
    expect(
      screen.getByRole('heading', { name: 'Overview', level: 1 }),
    ).not.toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(
      screen.getByRole('heading', { name: 'Workspaces', level: 1 }),
    ).toHaveFocus();
    expect(
      screen.getByRole('main', { name: 'Workspaces content' }),
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
    expect(
      screen.getByRole('heading', { name: 'Overview', level: 1 }),
    ).toHaveFocus();
    expect(selectAndAddWorkspace).not.toHaveBeenCalled();
  });

  it('keeps Overview usable when workspace loading fails and retries on Workspaces', async () => {
    vi.mocked(listWorkspaces).mockRejectedValueOnce(new Error('offline'));
    render(<App />);
    await screen.findByText(/foundation connected/i);
    expect(
      screen.getByRole('region', { name: 'Storage overview' }),
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(screen.getByText("Couldn't load workspaces.")).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByText('No workspaces added')).toBeVisible();
  });

  it.each(['added', 'duplicate'] as const)(
    'preserves later explicit navigation when setup Add returns %s',
    async (status) => {
      const row = {
        id: '1',
        displayName: 'Code',
        selectedPath: '/fixture/Code',
        enabled: true,
        createdAt: 1,
        updatedAt: 1,
      };
      vi.mocked(listWorkspaces)
        .mockResolvedValueOnce({
          setupState: 'undecided',
          workspaces: [],
          overlaps: [],
        })
        .mockResolvedValue({
          setupState: 'configured',
          workspaces: [row],
          overlaps: [],
        });
      const selection =
        deferred<Awaited<ReturnType<typeof selectAndAddWorkspace>>>();
      vi.mocked(selectAndAddWorkspace).mockReturnValue(selection.promise);
      render(<App />);
      await userEvent.click(
        await screen.findByRole('button', { name: 'Add workspace' }),
      );
      await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
      await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
      const heading = screen.getByRole('heading', {
        level: 1,
        name: 'Overview',
      });
      expect(heading).toHaveFocus();
      await act(async () => {
        selection.resolve(
          status === 'duplicate'
            ? { status, existingId: '1' }
            : {
                status,
                workspace: row,
                availability: { state: 'present', reason: null, checkedAt: 1 },
              },
        );
        await selection.promise;
      });
      expect(
        screen.getByRole('main', { name: 'Overview content' }),
      ).toBeVisible();
      expect(heading).toHaveFocus();
      expect(
        screen.queryByText('Workspace added. No files were scanned.'),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByText('This folder is already configured.'),
      ).not.toBeInTheDocument();
      await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
      expect(screen.getByText(row.selectedPath)).toBeVisible();
      expect(selectAndAddWorkspace).toHaveBeenCalledOnce();
    },
  );

  it('does not restore a cancelled setup picker over a later explicit Overview focus', async () => {
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'undecided',
      workspaces: [],
      overlaps: [],
    });
    const selection =
      deferred<Awaited<ReturnType<typeof selectAndAddWorkspace>>>();
    vi.mocked(selectAndAddWorkspace).mockReturnValue(selection.promise);
    render(<App />);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Add workspace' }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
    const heading = screen.getByRole('heading', { level: 1, name: 'Overview' });
    await act(async () => {
      selection.resolve({ status: 'cancelled' });
      await selection.promise;
    });
    expect(heading).toHaveFocus();
  });

  it('places optional setup after the Overview header and before storage content', async () => {
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'undecided',
      workspaces: [],
      overlaps: [],
    });
    render(<App />);
    const setup = await screen.findByRole('region', {
      name: 'Choose workspace folders',
    });
    const heading = screen.getByRole('heading', { level: 1, name: 'Overview' });
    const storage = screen.getByRole('region', { name: 'Storage overview' });
    expect(
      heading.compareDocumentPosition(setup) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      setup.compareDocumentPosition(storage) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      screen.getAllByRole('button', { name: 'Add workspace' }),
    ).toHaveLength(1);
    expect(
      screen.getAllByRole('button', { name: 'Skip for now' }),
    ).toHaveLength(1);
  });

  it('consumes setup success so a later removal and reload cannot revive its announcement', async () => {
    const row = {
      id: '1',
      displayName: 'Code',
      selectedPath: '/fixture/Code',
      enabled: true,
      createdAt: 1,
      updatedAt: 1,
    };
    vi.mocked(listWorkspaces)
      .mockResolvedValueOnce({
        setupState: 'undecided',
        workspaces: [],
        overlaps: [],
      })
      .mockResolvedValueOnce({
        setupState: 'configured',
        workspaces: [row],
        overlaps: [],
      })
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue({
        setupState: 'configured',
        workspaces: [],
        overlaps: [],
      });
    vi.mocked(selectAndAddWorkspace).mockResolvedValue({
      status: 'added',
      workspace: row,
      availability: { state: 'present', reason: null, checkedAt: 1 },
    });
    vi.mocked(removeWorkspace).mockRejectedValue(new Error('lost response'));
    HTMLDialogElement.prototype.showModal = function () {
      this.open = true;
    };
    HTMLDialogElement.prototype.close = function () {
      this.open = false;
    };
    render(<App />);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Add workspace' }),
    );
    expect(
      screen.getByText('Workspace added. No files were scanned.'),
    ).toBeVisible();
    await userEvent.click(
      screen.getByRole('button', {
        name: 'Remove workspace Code — /fixture/Code',
      }),
    );
    expect(
      screen.queryByText('Workspace added. No files were scanned.'),
    ).not.toBeInTheDocument();
    await userEvent.click(
      screen.getByRole('button', { name: 'Remove workspace' }),
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Reload workspaces' }),
    );
    expect(screen.getByText('No workspaces added')).toBeVisible();
    expect(
      screen.queryByText('Workspace added. No files were scanned.'),
    ).not.toBeInTheDocument();
    expect(removeWorkspace).toHaveBeenCalledOnce();
  });

  it('retains a manual availability check through navigation without moving focus on completion', async () => {
    const row = {
      id: '1',
      displayName: 'Code',
      selectedPath: '/fixture/Code',
      enabled: true,
      createdAt: 1,
      updatedAt: 1,
    };
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'configured',
      workspaces: [row],
      overlaps: [],
    });
    const request = deferred<Awaited<ReturnType<typeof checkWorkspace>>>();
    vi.mocked(checkWorkspace).mockReturnValue(request.promise);
    render(<App />);
    await screen.findByText(/foundation connected/i);
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    await userEvent.click(
      screen.getByRole('button', {
        name: 'Check availability for Code — /fixture/Code',
      }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(screen.getByText('Checking folder…')).toBeVisible();
    const add = screen.getByRole('button', { name: 'Add workspace' });
    add.focus();
    await act(async () => {
      request.resolve({
        id: '1',
        availability: { state: 'present', reason: null, checkedAt: 1 },
      });
      await request.promise;
    });
    expect(screen.getByText('Available')).toBeVisible();
    expect(screen.getByText('Contents have not been checked.')).toBeVisible();
    expect(add).toHaveFocus();
    expect(checkWorkspace).toHaveBeenCalledExactlyOnceWith('1');
  });

  it('preserves the uncertain mutation gate across failed reloads and view navigation', async () => {
    const row = {
      id: '1',
      displayName: 'Code',
      selectedPath: '/fixture/Code',
      enabled: true,
      createdAt: 1,
      updatedAt: 1,
    };
    vi.mocked(listWorkspaces)
      .mockResolvedValueOnce({
        setupState: 'configured',
        workspaces: [row],
        overlaps: [],
      })
      .mockRejectedValue(new Error('offline'));
    vi.mocked(removeWorkspace).mockRejectedValue(new Error('lost response'));
    HTMLDialogElement.prototype.showModal = function () {
      this.open = true;
    };
    HTMLDialogElement.prototype.close = function () {
      this.open = false;
    };
    render(<App />);
    await screen.findByText(/foundation connected/i);
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    await userEvent.click(
      screen.getByRole('button', {
        name: 'Remove workspace Code — /fixture/Code',
      }),
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Remove workspace' }),
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Reload workspaces' }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
    expect(
      screen.getByRole('checkbox', {
        name: 'Enabled for Code — /fixture/Code',
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole('button', { name: 'Reload workspaces' }),
    ).toBeEnabled();
    expect(removeWorkspace).toHaveBeenCalledOnce();
  });

  it('retains undecided setup until Skip commits and avoids duplicate prompts', async () => {
    const saved = deferred<{ setupState: 'skipped' }>();
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'undecided',
      workspaces: [],
      overlaps: [],
    });
    vi.mocked(skipWorkspaceSetup).mockReturnValue(saved.promise);
    render(<App />);
    await screen.findByText('Choose workspace folders');
    await userEvent.click(screen.getByRole('button', { name: 'Skip for now' }));
    expect(screen.getByText('Choose workspace folders')).toBeVisible();
    expect(skipWorkspaceSetup).toHaveBeenCalledOnce();
    vi.mocked(listWorkspaces).mockResolvedValue({
      setupState: 'skipped',
      workspaces: [],
      overlaps: [],
    });
    await act(async () => {
      saved.resolve({ setupState: 'skipped' });
      await saved.promise;
    });
    expect(
      screen.queryByText('Choose workspace folders'),
    ).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Workspaces' }));
    expect(
      screen.getAllByRole('button', { name: 'Add workspace' }),
    ).toHaveLength(1);
    expect(
      screen.queryByRole('button', { name: 'Skip for now' }),
    ).not.toBeInTheDocument();
  });

  it('keeps the shell visible when the desktop core is unavailable', async () => {
    vi.mocked(getAppHealth).mockRejectedValueOnce(new Error('offline'));
    render(<App />);
    expect(
      await screen.findByText(/desktop core is unavailable/i),
    ).toHaveTextContent(/restart the app/i);
    expect(screen.getByRole('navigation', { name: 'Main' })).toBeVisible();
    expect(
      screen.getByRole('region', { name: 'Storage overview' }),
    ).toBeVisible();
  });

  it.each(['resolve', 'reject'] as const)(
    'settles a late %s after unmount without reviving the view',
    async (outcome) => {
      const request = deferred<AppHealth>();
      vi.mocked(getAppHealth).mockReturnValueOnce(request.promise);
      const { unmount, container } = render(<App />);
      unmount();
      await act(async () => {
        if (outcome === 'resolve') request.resolve(health);
        else request.reject(new Error('late failure'));
        await request.promise.catch(() => undefined);
      });
      expect(container).toBeEmptyDOMElement();
    },
  );

  it.each(['resolve', 'reject'] as const)(
    'ignores a stale effect %s after StrictMode restarts it',
    async (outcome) => {
      const oldRequest = deferred<AppHealth>();
      const currentRequest = deferred<AppHealth>();
      vi.mocked(getAppHealth)
        .mockReturnValueOnce(oldRequest.promise)
        .mockReturnValueOnce(currentRequest.promise);
      render(
        <StrictMode>
          <App />
        </StrictMode>,
      );
      await act(async () => {
        currentRequest.resolve({ ...health, databaseSchemaVersion: 2 });
        await currentRequest.promise;
      });
      expect(screen.getByRole('status')).toHaveTextContent(/schema 2/i);
      await act(async () => {
        if (outcome === 'resolve')
          oldRequest.resolve({ ...health, databaseSchemaVersion: 99 });
        else oldRequest.reject(new Error('stale failure'));
        await oldRequest.promise.catch(() => undefined);
      });
      expect(screen.getByRole('status')).toHaveTextContent(
        /foundation connected.*schema 2/i,
      );
    },
  );
});
