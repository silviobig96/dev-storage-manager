import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { WorkspaceSetup } from './WorkspaceSetup';

describe('optional workspace setup', () => {
  it('offers controlled choices without calling either on render', async () => {
    const add = vi.fn().mockResolvedValue(undefined);
    const skip = vi.fn().mockResolvedValue(undefined);
    render(
      <WorkspaceSetup adding={false} error={null} onAdd={add} onSkip={skip} />,
    );
    expect(add).not.toHaveBeenCalled();
    expect(skip).not.toHaveBeenCalled();
    expect(
      screen.getByText(
        'Workspaces are optional. Adding a folder does not scan it.',
      ),
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Skip for now' }));
    expect(skip).toHaveBeenCalledOnce();
    expect(screen.getByText('Choose workspace folders')).toBeVisible();
  });
  it('handles rejected choices and leaves retry available', async () => {
    const skip = vi.fn().mockRejectedValue(new Error('save failed'));
    const { rerender } = render(
      <WorkspaceSetup
        adding={false}
        error={null}
        onAdd={vi.fn()}
        onSkip={skip}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Skip for now' }));
    rerender(
      <WorkspaceSetup
        adding={false}
        error={new Error("Couldn't save workspace changes. Try again.")}
        onAdd={vi.fn()}
        onSkip={skip}
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent(
      "Couldn't save workspace changes. Try again.",
    );
    expect(screen.getByRole('button', { name: 'Skip for now' })).toBeEnabled();
  });
  it('blocks duplicate choices while adding and catches rejected Add', async () => {
    const add = vi.fn().mockRejectedValue(new Error('cancelled transport'));
    const { rerender } = render(
      <WorkspaceSetup
        adding={true}
        error={null}
        onAdd={add}
        onSkip={vi.fn()}
      />,
    );
    expect(
      screen.getByRole('button', { name: 'Add workspace' }),
    ).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Skip for now' })).toBeDisabled();
    rerender(
      <WorkspaceSetup
        adding={false}
        error={null}
        onAdd={add}
        onSkip={vi.fn()}
      />,
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Add workspace' }),
    );
    expect(add).toHaveBeenCalledOnce();
  });
  it('restores its Add invoker after a cancelled picker returns and controls re-enable', async () => {
    let settle!: () => void;
    const selection = new Promise<void>((resolve) => {
      settle = resolve;
    });
    const add = () => {
      (document.activeElement as HTMLElement).blur();
      return selection;
    };
    render(
      <WorkspaceSetup
        adding={false}
        error={null}
        onAdd={add}
        onSkip={vi.fn()}
      />,
    );
    const invoker = screen.getByRole('button', { name: 'Add workspace' });
    await userEvent.click(invoker);
    expect(invoker).not.toHaveFocus();
    expect(invoker).toBeDisabled();
    await act(async () => {
      settle();
      await selection;
    });
    expect(invoker).toBeEnabled();
    expect(invoker).toHaveFocus();
  });
});
