import { useLayoutEffect, useRef, useState, type Ref } from 'react';
import type { WorkspacesController } from '../features/workspaces/useWorkspaces';
import { ContentHeader } from '../components/content/ContentHeader';
import { Section } from '../components/content/Section';
import { DataList } from '../components/content/DataList';
import { EmptyState } from '../components/content/EmptyState';
import { Button } from '../components/ui/Button';
import {
  FeedbackMessage,
  type FeedbackNotice,
} from '../components/ui/FeedbackMessage';
import { WorkspaceRow } from '../features/workspaces/WorkspaceRow';
import { RemoveWorkspaceDialog } from '../features/workspaces/RemoveWorkspaceDialog';
import {
  toWorkspaceCommandError,
  type WorkspaceCommandError,
  type WorkspaceRecord,
} from '../lib/workspaces';
import './WorkspacesView.css';

interface Removal {
  workspace: WorkspaceRecord;
  invoker: HTMLElement | null;
  neighbors: string[];
}

export function WorkspacesView({
  workspaces,
  headingRef,
  initialFeedback = null,
}: {
  workspaces: WorkspacesController;
  headingRef?: Ref<HTMLHeadingElement>;
  initialFeedback?: FeedbackNotice | null;
}) {
  const [announcement, setAnnouncement] = useState<FeedbackNotice | null>(
    initialFeedback,
  );
  const [removal, setRemoval] = useState<Removal | null>(null);
  const [removing, setRemoving] = useState(false);
  const [dialogError, setDialogError] = useState<WorkspaceCommandError | null>(
    null,
  );
  const [skipping, setSkipping] = useState(false);
  const addButton = useRef<HTMLButtonElement>(null);
  const rowElements = useRef(new Map<string, HTMLLIElement>());
  const removalPending = useRef(false);
  const addPending = useRef(false);
  const skipPending = useRef(false);
  const pendingFocus = useRef<string[] | null>(null);
  const pendingInvoker = useRef<HTMLElement | null>(null);
  const focusedMissing = useRef<Removal | null>(null);
  const listing =
    workspaces.state.status === 'ready' ? workspaces.state.listing : null;
  const activeRemoval =
    removal &&
    listing?.workspaces.some((row) => row.id === removal.workspace.id)
      ? removal
      : null;
  const requiresReload = !!workspaces.error?.requiresReload;

  function focusFallback(ids: string[]) {
    for (const id of ids) {
      const action = rowElements.current
        .get(id)
        ?.querySelector<HTMLInputElement>('input');
      if (action && !action.disabled) {
        action.focus();
        return;
      }
    }
    addButton.current?.focus();
  }
  useLayoutEffect(() => {
    if (pendingInvoker.current && !activeRemoval && !workspaces.adding) {
      if (pendingInvoker.current.isConnected) pendingInvoker.current.focus();
      pendingInvoker.current = null;
    } else if (pendingFocus.current) {
      focusFallback(pendingFocus.current);
      pendingFocus.current = null;
    } else if (
      removal &&
      !activeRemoval &&
      !removing &&
      focusedMissing.current !== removal
    ) {
      // A lost response can still have committed. Reconciliation owns the list;
      // recover focus even when the original invoker has disappeared.
      focusFallback(removal.neighbors);
      focusedMissing.current = removal;
    }
  });

  async function reload() {
    setAnnouncement(null);
    try {
      await workspaces.reload();
      setDialogError(null);
    } catch (failure) {
      if (activeRemoval) setDialogError(toWorkspaceCommandError(failure));
    }
  }
  async function add() {
    if (addPending.current || workspaces.adding || requiresReload) return;
    addPending.current = true;
    const invoker = document.activeElement as HTMLElement | null;
    setAnnouncement(null);
    try {
      const result = await workspaces.add();
      if (result.status === 'duplicate') {
        setAnnouncement({
          tone: 'info',
          message: 'This folder is already configured.',
        });
        rowElements.current.get(result.existingId)?.focus();
      } else if (result.status === 'added') {
        setAnnouncement({
          tone: 'success',
          message: 'Workspace added. No files were scanned.',
        });
        pendingInvoker.current = addButton.current;
      } else pendingInvoker.current = invoker;
    } catch {
      pendingInvoker.current = invoker;
    } finally {
      addPending.current = false;
    }
  }
  async function skip() {
    if (skipPending.current || requiresReload) return;
    skipPending.current = true;
    setAnnouncement(null);
    setSkipping(true);
    try {
      await workspaces.skip();
    } catch {
      /* Error comes from the controller. */
    } finally {
      skipPending.current = false;
      setSkipping(false);
    }
  }
  async function check(id: string) {
    setAnnouncement(null);
    try {
      await workspaces.check(id);
    } catch {
      /* Error comes from the controller. */
    }
  }
  async function setEnabled(id: string, enabled: boolean) {
    setAnnouncement(null);
    try {
      const saved = await workspaces.setEnabled(id, enabled);
      setAnnouncement({
        tone: 'success',
        message: saved.enabled ? 'Workspace enabled.' : 'Workspace disabled.',
      });
    } catch {
      /* Preserve committed state and the controller's error. */
    }
  }
  function requestRemove(id: string, trigger: HTMLButtonElement) {
    const index = listing?.workspaces.findIndex((row) => row.id === id) ?? -1;
    const workspace = listing?.workspaces[index];
    if (!workspace || !listing) return;
    setAnnouncement(null);
    setDialogError(null);
    setRemoval({
      workspace,
      invoker: trigger,
      neighbors: [
        ...listing.workspaces.slice(index + 1),
        ...listing.workspaces.slice(0, index).reverse(),
      ].map((row) => row.id),
    });
  }
  function cancelRemove() {
    if (removalPending.current || !removal) return;
    if (removal.invoker?.isConnected) pendingInvoker.current = removal.invoker;
    else pendingFocus.current = removal.neighbors;
    setRemoval(null);
  }
  async function confirmRemove(id: string) {
    if (removalPending.current || requiresReload || !removal) return;
    removalPending.current = true;
    setRemoving(true);
    setDialogError(null);
    try {
      await workspaces.remove(id);
      pendingFocus.current = removal.neighbors;
      setRemoval(null);
      setAnnouncement({
        tone: 'success',
        message: 'Workspace removed from the app.',
      });
    } catch (failure) {
      setDialogError(toWorkspaceCommandError(failure));
    } finally {
      removalPending.current = false;
      setRemoving(false);
    }
  }

  const addControl = (
    <Button
      ref={addButton}
      disabled={workspaces.adding || requiresReload || skipping}
      onClick={() => {
        void add();
      }}
    >
      Add workspace
    </Button>
  );
  return (
    <div className="workspaces-view">
      <ContentHeader
        title="Workspaces"
        description="Folders configured for future analysis. No files are scanned."
        headingRef={headingRef}
      />
      {workspaces.state.status === 'loading' && (
        <FeedbackMessage tone="info">Loading workspaces…</FeedbackMessage>
      )}
      {workspaces.state.status === 'error' && (
        <FeedbackMessage
          tone="error"
          title="Couldn't load workspaces."
          action={
            <Button
              onClick={() => {
                void reload();
              }}
            >
              Retry
            </Button>
          }
        >
          Try loading your workspace list again.
        </FeedbackMessage>
      )}
      {listing && (
        <>
          {!activeRemoval && workspaces.error && (
            <div className="workspace-feedback-error">
              <FeedbackMessage
                tone="error"
                title="Workspace needs attention"
                action={
                  requiresReload && (
                    <Button
                      onClick={() => {
                        void reload();
                      }}
                    >
                      Reload workspaces
                    </Button>
                  )
                }
              >
                {workspaces.error.message}
              </FeedbackMessage>
            </div>
          )}
          <div
            className="workspace-announcement"
            role="status"
            aria-live="polite"
            aria-atomic="true"
          >
            {!workspaces.error &&
              (workspaces.adding || removing ? (
                <FeedbackMessage tone="info" announce={false}>
                  {workspaces.adding
                    ? 'Adding workspace…'
                    : 'Saving workspace changes…'}
                </FeedbackMessage>
              ) : (
                announcement && (
                  <FeedbackMessage tone={announcement.tone} announce={false}>
                    {announcement.message}
                  </FeedbackMessage>
                )
              ))}
          </div>
          <Section title="Workspace folders">
            {listing.workspaces.length === 0 ? (
              <>
                <EmptyState
                  title="No workspaces added"
                  description="You can add folders now or keep using the app without them."
                />
                <div className="workspace-actions workspace-empty-actions">
                  {addControl}
                  {listing.setupState === 'undecided' && (
                    <Button
                      disabled={skipping || workspaces.adding || requiresReload}
                      onClick={() => {
                        void skip();
                      }}
                    >
                      Skip for now
                    </Button>
                  )}
                </div>
              </>
            ) : (
              <>
                <div className="workspace-list-actions">{addControl}</div>
                <DataList>
                  {listing.workspaces.map((workspace) => (
                    <WorkspaceRow
                      key={workspace.id}
                      workspace={workspace}
                      observation={workspaces.availability[workspace.id]}
                      overlap={listing.overlaps.some(
                        (overlap) =>
                          overlap.parentId === workspace.id ||
                          overlap.childId === workspace.id,
                      )}
                      busy={
                        workspaces.busyIds.has(workspace.id) || requiresReload
                      }
                      checking={
                        workspaces.availability[workspace.id]?.state ===
                          'pending' && workspaces.busyIds.has(workspace.id)
                      }
                      rowRef={(element) => {
                        if (element)
                          rowElements.current.set(workspace.id, element);
                        else rowElements.current.delete(workspace.id);
                      }}
                      onEnabled={(id, enabled) => {
                        void setEnabled(id, enabled);
                      }}
                      onCheck={(id) => {
                        void check(id);
                      }}
                      onRemove={requestRemove}
                    />
                  ))}
                </DataList>
              </>
            )}
          </Section>
        </>
      )}
      {activeRemoval && (
        <RemoveWorkspaceDialog
          workspace={activeRemoval.workspace}
          busy={removing}
          error={dialogError}
          requiresReload={requiresReload}
          onReload={() => {
            void reload();
          }}
          onCancel={cancelRemove}
          onConfirm={(id) => {
            void confirmRemove(id);
          }}
        />
      )}
    </div>
  );
}
