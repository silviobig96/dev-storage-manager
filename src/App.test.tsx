import { StrictMode } from 'react';
import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { getAppHealth, type AppHealth } from './lib/desktop';

vi.mock('./lib/desktop', () => ({ getAppHealth: vi.fn() }));

const health: AppHealth = {
  appName: 'dev-storage-manager',
  appVersion: '0.1.0',
  platform: 'macos',
  databaseSchemaVersion: 1,
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
  beforeEach(() => {
    vi.mocked(getAppHealth).mockReset().mockResolvedValue(health);
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
    expect(screen.getByRole('status')).toHaveTextContent(/schema 1/i);
    expect(getAppHealth).toHaveBeenCalledOnce();
    expect(screen.getByText('No automatic deletion')).toBeVisible();
    expect(
      screen.getByText('Overview', { selector: '.sidebar-item span' })
        .parentElement,
    ).toHaveAttribute('aria-current', 'page');
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(
      screen.queryByText(/\d+(\.\d+)?\s*[KMGTP]B/),
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
