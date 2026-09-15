import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { getAppHealth } from './lib/desktop';

vi.mock('./lib/desktop', () => ({
  getAppHealth: vi.fn(),
}));

describe('App', () => {
  beforeEach(() => {
    vi.mocked(getAppHealth).mockResolvedValue({
      appName: 'dev-storage-manager',
      appVersion: '0.1.0',
      platform: 'macos',
      databaseSchemaVersion: 1,
    });
  });

  it('announces that the desktop foundation is loading', () => {
    vi.mocked(getAppHealth).mockReturnValue(new Promise(() => undefined));

    render(<App />);

    expect(screen.getByRole('status')).toHaveTextContent(
      /connecting to desktop foundation/i,
    );
  });

  it('describes the product as foundation-stage developer storage intelligence', async () => {
    render(<App />);
    expect(
      screen.getByRole('heading', {
        name: /understand what your development environment/i,
      }),
    ).toBeVisible();
    expect(await screen.findByText(/foundation connected/i)).toBeVisible();
    expect(screen.getByText(/no automatic deletion/i)).toBeVisible();
  });

  it('shows a recoverable message when the desktop core is unavailable', async () => {
    vi.mocked(getAppHealth).mockRejectedValueOnce(new Error('offline'));
    render(<App />);
    expect(
      await screen.findByText(/desktop core is unavailable/i),
    ).toBeVisible();
  });
});
