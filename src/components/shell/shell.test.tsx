import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AppShell } from './AppShell';

describe('application shell', () => {
  it('identifies the current view and keeps future destinations noninteractive', () => {
    render(
      <AppShell currentView="overview">
        <h1>Overview</h1>
      </AppShell>,
    );
    const navigation = screen.getByRole('navigation', { name: 'Main' });
    expect(
      within(navigation).getByText('Overview').closest('[aria-current]'),
    ).toHaveAttribute('aria-current', 'page');
    for (const label of [
      'Workspaces',
      'Projects',
      'Categories',
      'History',
      'Settings',
    ]) {
      const item = within(navigation).getByText(label).closest('li');
      expect(item).toHaveTextContent(/unavailable/i);
      expect(item?.querySelector('a, button, [tabindex]')).toBeNull();
    }
    expect(within(navigation).queryByRole('button')).not.toBeInTheDocument();
    expect(within(navigation).queryByRole('link')).not.toBeInTheDocument();
  });

  it('provides a single main region after navigation and a concise view header', () => {
    render(
      <AppShell currentView="overview">
        <h1>Overview</h1>
      </AppShell>,
    );
    const main = screen.getByRole('main');
    const navigation = screen.getByRole('navigation', { name: 'Main' });
    expect(
      navigation.compareDocumentPosition(main) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      screen.getByRole('banner', { name: 'Current view' }),
    ).toHaveTextContent('Overview');
    expect(within(main).getAllByRole('heading', { level: 1 })).toHaveLength(1);
  });
});
