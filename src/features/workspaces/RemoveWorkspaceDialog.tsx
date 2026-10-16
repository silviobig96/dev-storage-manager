import { useId, useLayoutEffect, useRef } from 'react';
import { Button } from '../../components/ui/Button';
import type { WorkspaceRecord } from '../../lib/workspaces';
export function RemoveWorkspaceDialog({
  workspace,
  busy,
  error,
  onCancel,
  onConfirm,
  requiresReload = false,
  onReload,
}: {
  workspace: WorkspaceRecord;
  busy: boolean;
  error: Error | null;
  onCancel: () => void;
  onConfirm: (id: string) => void;
  requiresReload?: boolean;
  onReload?: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  // Close before the parent's layout focus restoration: native close() can
  // restore the pre-dialog element, which may differ from the actual trigger.
  useLayoutEffect(() => {
    const element = dialog.current;
    if (!element) return;
    element.showModal();
    cancel.current?.focus();
    return () => {
      if (element.open) element.close();
    };
  }, []);
  return (
    <dialog
      ref={dialog}
      className="workspace-remove-dialog"
      aria-labelledby={titleId}
      aria-describedby={descriptionId}
      aria-busy={busy}
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onCancel();
      }}
    >
      <h2 id={titleId}>Remove workspace?</h2>
      <p>{workspace.displayName}</p>
      <p className="workspace-path">{workspace.selectedPath}</p>
      <div id={descriptionId}>
        <p>This only removes the workspace from the app.</p>
        <p>Files on disk will not be deleted.</p>
      </div>
      {error && (
        <p className="workspace-error" role="alert">
          {error.message}
        </p>
      )}
      <div className="workspace-actions">
        <Button ref={cancel} disabled={busy} onClick={onCancel}>
          Cancel
        </Button>
        {requiresReload && (
          <Button disabled={busy} onClick={onReload}>
            Reload workspaces
          </Button>
        )}
        <Button
          disabled={busy || requiresReload}
          onClick={() => onConfirm(workspace.id)}
        >
          Remove workspace
        </Button>
      </div>
    </dialog>
  );
}
