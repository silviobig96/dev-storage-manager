import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { OverviewView } from './OverviewView';

describe('Overview', () => {
  it('explains the absence of analysis without inventing data or actions', () => {
    render(<OverviewView healthState={{ status: 'loading' }} />);
    expect(
      screen.getByRole('heading', { level: 1, name: 'Overview' }),
    ).toBeVisible();
    const storage = screen.getByRole('region', { name: 'Storage overview' });
    expect(
      within(storage).getByText(/Start an explicit scan from Workspaces/i),
    ).toBeVisible();
    expect(screen.getByText('Local-first')).toBeVisible();
    expect(screen.getByText('No automatic deletion')).toBeVisible();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(
      screen.queryByText(/\d+(\.\d+)?\s*[KMGTP]B/),
    ).not.toBeInTheDocument();
  });

  it('announces loading politely while content remains available', () => {
    render(<OverviewView healthState={{ status: 'loading' }} />);
    expect(screen.getByRole('status')).toHaveAttribute('aria-live', 'polite');
    expect(screen.getByRole('status')).toHaveTextContent(
      /connecting to desktop foundation/i,
    );
  });

  it('reports the returned schema without presenting it as storage data', () => {
    render(
      <OverviewView
        healthState={{
          status: 'connected',
          health: {
            appName: 'dev-storage-manager',
            appVersion: '0.1.0',
            platform: 'macos',
            databaseSchemaVersion: 7,
          },
        }}
      />,
    );
    expect(screen.getByRole('status')).toHaveTextContent(
      /foundation connected.*schema 7/i,
    );
  });

  it('keeps static information available after a connection failure', () => {
    render(<OverviewView healthState={{ status: 'unavailable' }} />);
    expect(screen.getByRole('status')).toHaveTextContent(
      /desktop core is unavailable.*restart the app/i,
    );
    expect(
      screen.getByRole('region', { name: 'Safety posture' }),
    ).toBeVisible();
  });
});
