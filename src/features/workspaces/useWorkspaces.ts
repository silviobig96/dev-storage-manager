import { useCallback, useEffect, useRef, useState } from 'react';
import {
  checkWorkspace,
  listWorkspaces,
  removeWorkspace,
  selectAndAddWorkspace,
  setWorkspaceEnabled,
  skipWorkspaceSetup,
  toWorkspaceCommandError,
  WorkspaceCommandError,
  type AddWorkspaceOutcome,
  type AvailabilityObservation,
  type CheckWorkspaceResult,
  type RemoveWorkspaceResult,
  type SetupWorkspaceResult,
  type WorkspaceListing,
  type WorkspaceRecord,
} from '../../lib/workspaces';

export type WorkspacesState =
  | { status: 'loading' }
  | { status: 'error'; error: WorkspaceCommandError }
  | { status: 'ready'; listing: WorkspaceListing };
export interface WorkspacesController {
  state: WorkspacesState;
  availability: Record<string, AvailabilityObservation>;
  adding: boolean;
  busyIds: Set<string>;
  error: WorkspaceCommandError | null;
  reload(): Promise<void>;
  add(): Promise<AddWorkspaceOutcome>;
  remove(id: string): Promise<RemoveWorkspaceResult>;
  setEnabled(id: string, enabled: boolean): Promise<WorkspaceRecord>;
  check(id: string): Promise<CheckWorkspaceResult>;
  skip(): Promise<SetupWorkspaceResult>;
}

export function useWorkspaces(): WorkspacesController {
  const [state, setState] = useState<WorkspacesState>({ status: 'loading' });
  const [availability, setAvailability] = useState<
    Record<string, AvailabilityObservation>
  >({});
  const [adding, setAdding] = useState(false);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<WorkspaceCommandError | null>(null);
  const session = useRef(0);
  const active = useRef(false);
  const listing = useRef<WorkspaceListing | null>(null);
  const observations = useRef<Record<string, AvailabilityObservation>>({});
  const listRequest = useRef(0);
  const revision = useRef(0);
  const checkRequests = useRef(new Map<string, symbol>());
  const rowOperations = useRef(new Map<string, Set<symbol>>());
  const mutations = useRef(new Set<string>());
  const needsReconciliation = useRef(false);

  const isCurrent = useCallback(
    (generation: number) => active.current && session.current === generation,
    [],
  );
  const publishObservations = useCallback(() => {
    const next = { ...observations.current };
    for (const id of checkRequests.current.keys()) {
      next[id] = { state: 'pending', reason: null, checkedAt: null };
    }
    setAvailability(next);
  }, []);
  const publishListing = useCallback(
    (next: WorkspaceListing) => {
      listing.current = next;
      const ids = new Set(next.workspaces.map((workspace) => workspace.id));
      observations.current = Object.fromEntries(
        Object.entries(observations.current).filter(([id]) => ids.has(id)),
      );
      for (const id of checkRequests.current.keys()) {
        if (!ids.has(id)) checkRequests.current.delete(id);
      }
      for (const id of rowOperations.current.keys()) {
        if (!ids.has(id)) rowOperations.current.delete(id);
      }
      setBusyIds(new Set(rowOperations.current.keys()));
      publishObservations();
      setState({ status: 'ready', listing: next });
    },
    [publishObservations],
  );

  const reload = useCallback(async () => {
    const generation = session.current;
    const request = ++listRequest.current;
    const version = revision.current;
    try {
      const next = await listWorkspaces();
      if (
        isCurrent(generation) &&
        request === listRequest.current &&
        version === revision.current
      ) {
        needsReconciliation.current = false;
        publishListing(next);
        setError(null);
      }
    } catch (failure) {
      const commandError = toWorkspaceCommandError(failure);
      if (
        isCurrent(generation) &&
        request === listRequest.current &&
        version === revision.current
      ) {
        setError(commandError);
        if (!listing.current)
          setState({ status: 'error', error: commandError });
      }
      throw commandError;
    }
  }, [isCurrent, publishListing]);

  useEffect(() => {
    active.current = true;
    session.current += 1;
    void reload().catch(() => undefined);
    return () => {
      active.current = false;
      session.current += 1;
      listRequest.current += 1;
    };
  }, [reload]);

  function markBusy(id: string, token: symbol, busy: boolean) {
    const operations = rowOperations.current.get(id) ?? new Set<symbol>();
    if (busy) operations.add(token);
    else operations.delete(token);
    if (operations.size) rowOperations.current.set(id, operations);
    else rowOperations.current.delete(id);
    setBusyIds(new Set(rowOperations.current.keys()));
  }

  async function mutate<T>(
    key: string,
    command: () => Promise<T>,
    apply: (result: T) => boolean,
  ): Promise<T> {
    if (!active.current)
      throw new WorkspaceCommandError('application_unavailable');
    if (mutations.current.has(key))
      throw new WorkspaceCommandError('operation_in_progress');
    const generation = session.current;
    const token = Symbol();
    mutations.current.add(key);
    if (key === 'add') setAdding(true);
    else if (key !== 'skip') markBusy(key, token, true);
    setError(null);
    // A list requested before this operation cannot describe its committed result.
    revision.current += 1;
    listRequest.current += 1;
    try {
      if (needsReconciliation.current) {
        try {
          await reload();
        } catch {
          /* Preserve the uncertainty until a list succeeds. */
        }
        if (!isCurrent(generation) || needsReconciliation.current) {
          throw new WorkspaceCommandError(
            'application_unavailable',
            'transport',
            undefined,
            true,
            true,
          );
        }
      }
      let result: T;
      try {
        result = await command();
      } catch (failure) {
        let commandError = toWorkspaceCommandError(failure);
        if (isCurrent(generation)) {
          if (commandError.source === 'transport') {
            needsReconciliation.current = true;
            revision.current += 1;
            try {
              await reload();
            } catch {
              /* Keep the last committed listing. */
            }
            commandError = new WorkspaceCommandError(
              'application_unavailable',
              'transport',
              undefined,
              true,
              needsReconciliation.current,
            );
          } else if (commandError.code === 'workspace_not_found') {
            try {
              await reload();
            } catch {
              /* The original actionable error remains authoritative. */
            }
          }
        }
        throw commandError;
      }
      if (isCurrent(generation) && apply(result)) {
        revision.current += 1;
        listRequest.current += 1;
        // Commit responses update known rows first; refresh supplies authoritative overlaps.
        try {
          await reload();
        } catch {
          /* A list failure cannot undo a successful commit. */
        }
      }
      return result;
    } catch (failure) {
      const commandError = toWorkspaceCommandError(failure);
      if (isCurrent(generation)) setError(commandError);
      throw commandError;
    } finally {
      if (isCurrent(generation)) {
        mutations.current.delete(key);
        if (key === 'add') setAdding(false);
        else if (key !== 'skip') markBusy(key, token, false);
      }
    }
  }

  function updateListing(
    transform: (current: WorkspaceListing) => WorkspaceListing,
  ) {
    if (listing.current) publishListing(transform(listing.current));
  }

  async function add() {
    return mutate('add', selectAndAddWorkspace, (result) => {
      if (result.status === 'cancelled') return false;
      if (result.status === 'added') {
        updateListing((current) => ({
          ...current,
          setupState: 'configured',
          workspaces: [
            ...current.workspaces.filter(
              (row) => row.id !== result.workspace.id,
            ),
            result.workspace,
          ],
        }));
        observations.current = {
          ...observations.current,
          [result.workspace.id]: result.availability,
        };
        publishObservations();
      }
      return true;
    });
  }

  async function remove(id: string) {
    return mutate(
      id,
      () => removeWorkspace(id),
      (result) => {
        updateListing((current) => ({
          ...current,
          workspaces: current.workspaces.filter(
            (row) => row.id !== result.removedId,
          ),
          overlaps: current.overlaps.filter(
            (overlap) =>
              overlap.parentId !== result.removedId &&
              overlap.childId !== result.removedId,
          ),
        }));
        return true;
      },
    );
  }

  async function setEnabled(id: string, enabled: boolean) {
    return mutate(
      id,
      () => setWorkspaceEnabled(id, enabled),
      (result) => {
        updateListing((current) => ({
          ...current,
          workspaces: current.workspaces.map((row) =>
            row.id === result.id ? result : row,
          ),
        }));
        return true;
      },
    );
  }

  async function skip() {
    return mutate('skip', skipWorkspaceSetup, (result) => {
      updateListing((current) => ({
        ...current,
        setupState:
          current.setupState === 'configured'
            ? 'configured'
            : result.setupState,
      }));
      return true;
    });
  }

  async function check(id: string) {
    if (!active.current)
      throw new WorkspaceCommandError('application_unavailable');
    if (!listing.current?.workspaces.some((row) => row.id === id))
      throw new WorkspaceCommandError('workspace_not_found');
    const generation = session.current;
    const token = Symbol();
    checkRequests.current.set(id, token);
    markBusy(id, token, true);
    publishObservations();
    setError(null);
    const current = () =>
      isCurrent(generation) &&
      checkRequests.current.get(id) === token &&
      !!listing.current?.workspaces.some((row) => row.id === id);
    try {
      const result = await checkWorkspace(id);
      if (current()) {
        observations.current = {
          ...observations.current,
          [id]: result.availability,
        };
        checkRequests.current.delete(id);
        publishObservations();
      }
      return result;
    } catch (failure) {
      const commandError = toWorkspaceCommandError(failure);
      if (current()) {
        checkRequests.current.delete(id);
        publishObservations();
        if (commandError.code === 'workspace_not_found') {
          try {
            await reload();
          } catch {
            /* Keep the last committed configuration. */
          }
        }
        if (isCurrent(generation) && !checkRequests.current.has(id))
          setError(commandError);
      }
      throw commandError;
    } finally {
      if (isCurrent(generation)) markBusy(id, token, false);
    }
  }

  return {
    state,
    availability,
    adding,
    busyIds,
    error,
    reload,
    add,
    remove,
    setEnabled,
    check,
    skip,
  };
}
