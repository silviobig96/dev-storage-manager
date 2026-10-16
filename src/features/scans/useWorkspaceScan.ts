import { useEffect, useState, useSyncExternalStore } from 'react';
import {
  cancelWorkspaceScan,
  getWorkspaceScan,
  ScanCommandError,
  startWorkspaceScan,
  toScanCommandError,
  type ScanMode,
  type ScanRequest,
  type ScanSnapshot,
  type ScanStatus,
} from '../../lib/scans';
export interface WorkspaceScanController {
  status: 'reconciling' | 'ready' | 'unavailable';
  snapshot: ScanSnapshot | null;
  error: ScanCommandError | null;
  /** Read-only intent for an explicit same-payload Start; UUID stays controller-owned. */
  pendingStart: {
    readonly workspaceIds: readonly string[];
    readonly mode: ScanMode;
  } | null;
  start(ids: string[], mode: ScanMode): Promise<void>;
  cancel(): Promise<void>;
  reconnect(): Promise<void>;
}
type ScanViewState = Pick<
  WorkspaceScanController,
  'status' | 'snapshot' | 'error' | 'pendingStart'
>;

/** One App-owned controller survives view navigation and StrictMode effect replay. */
class ScanController {
  private state: ScanViewState = {
    status: 'reconciling',
    snapshot: null,
    error: null,
    pendingStart: null,
  };
  private listeners = new Set<() => void>();
  private active = false;
  private generation = 0;
  private version = 0;
  private epoch: string | null = null;
  private retiredEpochs = new Set<string>();
  private pending: ScanRequest | null = null;
  private mutation = false;
  private query: Promise<void> | null = null;
  private queuedRecovery = false;
  private timer: ReturnType<typeof setTimeout> | undefined;

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(patch: Partial<ScanViewState>) {
    this.state = {
      ...this.state,
      ...patch,
      pendingStart: this.pending
        ? Object.freeze({
            workspaceIds: Object.freeze([...this.pending.workspaceIds]),
            mode: this.pending.mode,
          })
        : null,
    };
    this.listeners.forEach((listener) => listener());
  }
  private clearTimer() {
    clearTimeout(this.timer);
    this.timer = undefined;
  }
  private schedule() {
    this.clearTimer();
    // Visibility/StrictMode recovery waits for both read and action responses.
    // In particular, showing a window must never invalidate a pending cancel.
    if (
      this.active &&
      document.visibilityState === 'visible' &&
      !this.query &&
      !this.mutation &&
      this.queuedRecovery
    ) {
      this.queuedRecovery = false;
      void this.reconcile(true);
      return;
    }
    if (
      this.active &&
      document.visibilityState === 'visible' &&
      !this.query &&
      !this.mutation &&
      this.state.status === 'ready' &&
      this.state.snapshot?.workerActive
    ) {
      this.timer = setTimeout(() => {
        void this.reconcile(false);
      }, 250);
    }
  }
  private current(generation: number, version: number) {
    return (
      this.active && generation === this.generation && version === this.version
    );
  }
  private accept(status: ScanStatus): boolean {
    // Epochs are opaque: only equality/retirement is meaningful, never ordering.
    if (
      typeof status.epoch !== 'string' ||
      !status.epoch ||
      status.session === undefined
    )
      throw new ScanCommandError('application_unavailable', 'transport');
    if (this.retiredEpochs.has(status.epoch)) return false;
    const next = status.session;
    const previous = this.state.snapshot;
    if (next && next.epoch !== status.epoch) return false;
    if (this.epoch === status.epoch && previous) {
      // A confirmed explicit Start (or its recovery read) replaces the retained
      // terminal session. Unrelated sessions and stale revisions still reject.
      const pendingReplacement =
        !previous.workerActive &&
        this.pending?.expectedEpoch === status.epoch &&
        next?.requestId === this.pending?.requestId &&
        next?.sessionId !== previous.sessionId;
      if (
        !pendingReplacement &&
        (!next ||
          next.sessionId !== previous.sessionId ||
          next.requestId !== previous.requestId ||
          BigInt(next.revision) < BigInt(previous.revision))
      )
        return false;
    }
    if (this.epoch && this.epoch !== status.epoch)
      this.retiredEpochs.add(this.epoch);
    this.epoch = status.epoch;
    if (this.pending && next?.requestId === this.pending.requestId)
      this.pending = null;
    // A read with no matching request cannot prove a lost start was rejected.
    this.publish({
      snapshot: next,
      status: this.pending ? 'unavailable' : 'ready',
      error: this.pending
        ? new ScanCommandError('application_unavailable', 'transport')
        : null,
    });
    return true;
  }
  private reconcile = (explicit: boolean): Promise<void> => {
    if (!this.active || this.mutation) return Promise.resolve();
    if (this.query) return this.query;
    this.clearTimer();
    const generation = this.generation;
    const version = this.version;
    if (explicit) this.publish({ status: 'reconciling', error: null });
    this.query = (async () => {
      try {
        const status = await getWorkspaceScan();
        if (this.current(generation, version)) {
          const accepted = this.accept(status);
          if (!accepted && explicit)
            this.publish({
              status: 'unavailable',
              error: new ScanCommandError(
                'application_unavailable',
                'transport',
              ),
            });
        }
      } catch (error) {
        if (this.current(generation, version))
          this.publish({
            status: 'unavailable',
            error: toScanCommandError(error),
          });
      } finally {
        this.query = null;
        this.schedule();
      }
    })();
    return this.query;
  };
  reconnect = () => this.reconcile(true);
  private visibilityChanged = () => {
    this.clearTimer();
    if (document.visibilityState === 'visible') {
      if (this.mutation) this.queuedRecovery = true;
      else if (this.query) {
        this.version += 1;
        this.queuedRecovery = true;
      } else void this.reconnect();
    }
  };
  connect = () => {
    this.active = true;
    this.generation += 1;
    document.addEventListener('visibilitychange', this.visibilityChanged);
    if (this.query || this.mutation) this.queuedRecovery = true;
    else void this.reconnect();
    return () => {
      this.active = false;
      this.generation += 1;
      this.clearTimer();
      document.removeEventListener('visibilitychange', this.visibilityChanged);
    };
  };
  start = async (ids: string[], mode: ScanMode): Promise<void> => {
    if (!this.active || this.mutation) return;
    if (this.pending) {
      if (
        this.pending.mode !== mode ||
        this.pending.workspaceIds.length !== ids.length ||
        this.pending.workspaceIds.some((id, index) => id !== ids[index])
      ) {
        this.publish({ error: new ScanCommandError('request_conflict') });
        return;
      }
    } else if (
      this.state.status !== 'ready' ||
      !this.epoch ||
      this.state.snapshot?.workerActive
    )
      return;
    this.clearTimer();
    this.mutation = true;
    const generation = this.generation;
    const version = ++this.version;
    this.publish({ status: 'reconciling', error: null });
    try {
      this.pending ??= {
        requestId: crypto.randomUUID(),
        expectedEpoch: this.epoch!,
        workspaceIds: [...ids],
        mode,
      };
      const snapshot = await startWorkspaceScan(this.pending);
      if (this.current(generation, version)) {
        if (
          snapshot.requestId !== this.pending.requestId ||
          !this.accept({ epoch: snapshot.epoch, session: snapshot })
        )
          throw new ScanCommandError('application_unavailable', 'transport');
      }
    } catch (error) {
      if (this.current(generation, version)) {
        const failure = toScanCommandError(error);
        if (failure.source === 'backend') this.pending = null;
        this.publish({ status: 'unavailable', error: failure });
      }
    } finally {
      this.mutation = false;
      this.schedule();
    }
  };
  cancel = async (): Promise<void> => {
    const snapshot = this.state.snapshot;
    if (
      !this.active ||
      this.mutation ||
      this.state.status !== 'ready' ||
      !snapshot?.workerActive
    )
      return;
    this.clearTimer();
    this.mutation = true;
    const generation = this.generation;
    const version = ++this.version;
    this.publish({ status: 'reconciling', error: null });
    try {
      const next = await cancelWorkspaceScan(snapshot.sessionId);
      if (this.current(generation, version)) {
        if (
          next.epoch !== snapshot.epoch ||
          next.sessionId !== snapshot.sessionId ||
          !this.accept({ epoch: next.epoch, session: next })
        )
          throw new ScanCommandError('application_unavailable', 'transport');
      }
    } catch (error) {
      if (this.current(generation, version))
        this.publish({
          status: 'unavailable',
          error: toScanCommandError(error),
        });
    } finally {
      this.mutation = false;
      this.schedule();
    }
  };
}

export function useWorkspaceScan(): WorkspaceScanController {
  const [controller] = useState(() => new ScanController());
  const state = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
  );
  useEffect(() => controller.connect(), [controller]);
  return {
    ...state,
    start: controller.start,
    cancel: controller.cancel,
    reconnect: controller.reconnect,
  };
}
