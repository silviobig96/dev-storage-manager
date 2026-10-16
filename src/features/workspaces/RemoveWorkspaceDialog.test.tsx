import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { RemoveWorkspaceDialog } from './RemoveWorkspaceDialog';
const workspace = {
  id: '5',
  displayName: 'Code',
  selectedPath: '/fixture/Code',
  enabled: true,
  createdAt: 1,
  updatedAt: 1,
};
beforeEach(() => {
  // Test-only lifecycle polyfill: native focus trapping needs real webview QA.
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});
describe('remove workspace confirmation', () => {
  it('starts on Cancel, identifies full path and cancels Escape without sending an ID', () => {
    const cancel = vi.fn();
    const confirm = vi.fn();
    render(
      <RemoveWorkspaceDialog
        workspace={workspace}
        busy={false}
        error={null}
        onCancel={cancel}
        onConfirm={confirm}
      />,
    );
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    expect(screen.getByText(workspace.selectedPath)).toBeVisible();
    expect(
      screen.getByText('This only removes the workspace from the app.'),
    ).toBeVisible();
    expect(
      screen.getByText('Files on disk will not be deleted.'),
    ).toBeVisible();
    fireEvent(
      screen.getByRole('dialog'),
      new Event('cancel', { cancelable: true }),
    );
    expect(cancel).toHaveBeenCalledOnce();
    expect(confirm).not.toHaveBeenCalled();
  });
  it('sends only the ID once and prevents dismissal while busy', async () => {
    const cancel = vi.fn();
    const confirm = vi.fn();
    const { rerender } = render(
      <RemoveWorkspaceDialog
        workspace={workspace}
        busy={false}
        error={null}
        onCancel={cancel}
        onConfirm={confirm}
      />,
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Remove workspace' }),
    );
    expect(confirm).toHaveBeenCalledExactlyOnceWith('5');
    rerender(
      <RemoveWorkspaceDialog
        workspace={workspace}
        busy={true}
        error={null}
        onCancel={cancel}
        onConfirm={confirm}
      />,
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Remove workspace' }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    fireEvent(
      screen.getByRole('dialog'),
      new Event('cancel', { cancelable: true }),
    );
    expect(confirm).toHaveBeenCalledOnce();
    expect(cancel).not.toHaveBeenCalled();
  });
});
