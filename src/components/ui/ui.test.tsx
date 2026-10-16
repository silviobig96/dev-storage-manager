import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { AppIcon } from './AppIcon';
import { Button } from './Button';
import { IconButton } from './IconButton';
import { StatusIndicator } from './StatusIndicator';

describe('interface primitives', () => {
  it('names an icon action and exposes a tooltip', async () => {
    const action = vi.fn();
    render(<IconButton label="Show sidebar" icon="sidebar" onClick={action} />);
    const button = screen.getByRole('button', { name: 'Show sidebar' });
    expect(button).toHaveAttribute('title', 'Show sidebar');
    await userEvent.tab();
    expect(button).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    expect(action).toHaveBeenCalledOnce();
  });

  it('prevents activation of disabled controls', async () => {
    const action = vi.fn();
    render(
      <Button disabled onClick={action}>
        Continue
      </Button>,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Continue' }));
    expect(action).not.toHaveBeenCalled();
  });

  it('does not submit an enclosing form by default', async () => {
    const submit = vi.fn((event: React.FormEvent) => event.preventDefault());
    render(
      <form onSubmit={submit}>
        <Button>Continue</Button>
      </form>,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Continue' }));
    expect(submit).not.toHaveBeenCalled();
  });

  it.each(['safe', 'review', 'protected', 'error', 'neutral'] as const)(
    'retains visible text for %s status',
    (tone) => {
      render(<StatusIndicator tone={tone}>Status explanation</StatusIndicator>);
      expect(screen.getByText('Status explanation')).toBeVisible();
    },
  );

  it('hides decorative icons from assistive technology', () => {
    const { container } = render(<AppIcon name="overview" />);
    expect(container.querySelector('svg')).toHaveAttribute(
      'aria-hidden',
      'true',
    );
    expect(container.querySelector('svg')).toHaveAttribute(
      'focusable',
      'false',
    );
  });
});
