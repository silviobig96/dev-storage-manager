import type { Ref } from 'react';
import { ListRow } from '../../components/content/ListRow';
import { Button } from '../../components/ui/Button';
import {
  FeedbackMessage,
  type FeedbackTone,
} from '../../components/ui/FeedbackMessage';
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
  const status: Record<
    typeof availability,
    { tone: FeedbackTone; title: string; description?: string }
  > = {
    pending: {
      tone: checking ? 'info' : 'neutral',
      title: checking ? 'Checking folder…' : 'Not checked',
    },
    present: {
      tone: 'success',
      title: 'Available',
      description: 'Contents have not been checked.',
    },
    missing: {
      tone: 'warning',
      title: 'Folder unavailable',
      description:
        'Folder not found. Reconnect the disk or check the saved location.',
    },
    denied: {
      tone: 'warning',
      title: 'Access denied',
      description: 'Access to this folder was denied.',
    },
    unknown: {
      tone: 'warning',
      title:
        observation?.reason === 'identity_changed'
          ? 'Location changed'
          : 'Availability unknown',
      description:
        observation?.reason === 'identity_changed'
          ? 'The location now refers to a different folder. Remove and add it again to change configuration.'
          : "Couldn't confirm availability. Try checking again.",
    },
  };
  const meaning = status[availability];
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
          <div
            className="workspace-availability"
            aria-live="polite"
            aria-atomic="true"
          >
            <span className="visually-hidden">{identity}: </span>
            <FeedbackMessage
              tone={meaning.tone}
              title={meaning.title}
              variant="inline"
              announce={false}
            >
              {meaning.description}
            </FeedbackMessage>
          </div>
          {overlap && (
            <div className="workspace-overlap">
              <FeedbackMessage
                tone="warning"
                title="Overlapping folders"
                variant="inline"
                announce={false}
              >
                Overlaps another workspace. Future analysis must avoid double
                counting.
              </FeedbackMessage>
            </div>
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
