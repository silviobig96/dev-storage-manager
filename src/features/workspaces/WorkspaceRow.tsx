import type { Ref } from 'react';
import { ListRow } from '../../components/content/ListRow';
import { Button } from '../../components/ui/Button';
import type {
  AvailabilityObservation,
  WorkspaceRecord,
} from '../../lib/workspaces';

export function WorkspaceRow({
  workspace,
  observation,
  overlap,
  busy,
  checking,
  rowRef,
  onEnabled,
  onCheck,
  onRemove,
}: {
  workspace: WorkspaceRecord;
  observation?: AvailabilityObservation;
  overlap: boolean;
  busy: boolean;
  checking: boolean;
  rowRef?: Ref<HTMLLIElement>;
  onEnabled: (id: string, enabled: boolean) => void;
  onCheck: (id: string) => void;
  onRemove: (id: string, trigger: HTMLButtonElement) => void;
}) {
  const identity = `${workspace.displayName} — ${workspace.selectedPath}`;
  const availability = observation?.state ?? 'pending';
  const meaning = {
    pending: checking ? 'Checking folder…' : 'Not checked',
    present: 'Folder found. Access to its contents has not been checked.',
    missing:
      'Folder not found. Reconnect the disk or check the saved location.',
    denied: 'Access to this folder was denied.',
    unknown: "Couldn't confirm availability. Try checking again.",
  }[availability];
  return (
    <ListRow
      className="workspace-row"
      id={`workspace-${workspace.id}`}
      rowRef={rowRef}
      tabIndex={-1}
      label={
        <>
          <span>{workspace.displayName}</span>
          <p className="workspace-path">{workspace.selectedPath}</p>
          <p>{meaning}</p>
          {observation?.reason === 'identity_changed' && (
            <p>
              The location now refers to a different folder. Remove and add it
              again to change configuration.
            </p>
          )}
          {overlap && (
            <p>
              Overlaps another workspace. Future analysis must avoid double
              counting.
            </p>
          )}
        </>
      }
      trailing={
        <div className="workspace-actions">
          <label className="workspace-enabled">
            <input
              type="checkbox"
              checked={workspace.enabled}
              disabled={busy}
              aria-label={`Enabled for ${identity}`}
              onChange={(event) =>
                onEnabled(workspace.id, event.target.checked)
              }
            />
            Enabled
          </label>
          <Button
            disabled={busy}
            aria-label={`Check availability for ${identity}`}
            onClick={() => onCheck(workspace.id)}
          >
            Check availability
          </Button>
          <Button
            disabled={busy}
            aria-label={`Remove workspace ${identity}`}
            onClick={(event) => onRemove(workspace.id, event.currentTarget)}
          >
            Remove workspace
          </Button>
        </div>
      }
    />
  );
}
