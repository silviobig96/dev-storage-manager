import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ContentHeader } from './ContentHeader';
import { Section } from './Section';
import { DataList } from './DataList';
import { ListRow } from './ListRow';
import { EmptyState } from './EmptyState';

describe('content presentation', () => {
  it('names a view and its section without duplicate heading levels', () => {
    render(
      <>
        <ContentHeader title="Overview" description="Developer storage" />
        <Section title="Safety posture">
          <p>Explicit intent</p>
        </Section>
      </>,
    );
    expect(
      screen.getByRole('heading', { level: 1, name: 'Overview' }),
    ).toBeVisible();
    const section = screen.getByRole('region', { name: 'Safety posture' });
    expect(within(section).getByRole('heading', { level: 2 })).toBeVisible();
  });

  it('keeps row labels, explanations, and metadata in reading order', () => {
    render(
      <DataList>
        <ListRow
          label="Foundation"
          description="Read-only connection"
          trailing="Schema 1"
        />
      </DataList>,
    );
    const row = screen.getByRole('listitem');
    expect(row).toHaveTextContent(/Foundation.*Read-only connection.*Schema 1/);
  });

  it('explains missing content without requiring a fake action', () => {
    render(
      <EmptyState
        title="No storage analysis yet"
        description="Analysis is not available in this foundation release."
      />,
    );
    expect(
      screen.getByRole('heading', {
        level: 3,
        name: 'No storage analysis yet',
      }),
    ).toBeVisible();
    expect(
      screen.getByText(/not available in this foundation release/),
    ).toBeVisible();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
