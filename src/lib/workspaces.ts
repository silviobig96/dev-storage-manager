import { invoke } from '@tauri-apps/api/core';

export type WorkspaceInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;
export type SetupState = 'undecided' | 'skipped' | 'configured';
export interface WorkspaceRecord {
  id: string;
  selectedPath: string;
  displayName: string;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}
export interface WorkspaceOverlap {
  parentId: string;
  childId: string;
}
export interface WorkspaceListing {
  setupState: SetupState;
  workspaces: WorkspaceRecord[];
  overlaps: WorkspaceOverlap[];
}
export type AvailabilityState =
  'pending' | 'present' | 'missing' | 'denied' | 'unknown';
export type AvailabilityReason =
  | 'timeout'
  | 'identity_changed'
  | 'not_directory'
  | 'unsupported_identity'
  | 'io';
export interface AvailabilityObservation {
  state: AvailabilityState;
  reason: AvailabilityReason | null;
  checkedAt: number | null;
}
export type AddWorkspaceOutcome =
  | { status: 'cancelled' }
  | {
      status: 'added';
      workspace: WorkspaceRecord;
      availability: AvailabilityObservation;
    }
  | { status: 'duplicate'; existingId: string };
export interface RemoveWorkspaceResult {
  removedId: string;
}
export interface CheckWorkspaceResult {
  id: string;
  availability: AvailabilityObservation;
}
export interface SetupWorkspaceResult {
  setupState: SetupState;
}

const messages = {
  operation_in_progress: 'Another workspace operation is in progress.',
  invalid_selection: 'Select a local folder.',
  unsupported_path_encoding:
    "This folder's path cannot be represented as UTF-8.",
  not_directory: 'The selected location is not a folder.',
  folder_missing: 'The folder could not be found.',
  access_denied: 'Access to this folder was denied.',
  identity_unavailable:
    'A reliable identity could not be obtained for this folder.',
  location_changed:
    'The folder changed during validation. Try selecting it again.',
  path_already_configured:
    'This location has changed. Remove its old entry before adding it again.',
  validation_timeout: 'The folder check timed out. Try again.',
  workspace_not_found: 'This workspace is no longer configured.',
  persistence_unavailable: "Couldn't save workspace changes. Try again.",
  application_unavailable: 'Workspace services are unavailable. Try again.',
} as const;
export type WorkspaceErrorCode = keyof typeof messages;
export interface WorkspaceFailure {
  code: WorkspaceErrorCode;
  message: string;
  existingId?: string;
}

/** Frontend-only outcome metadata distinguishes a rejection from a lost response. */
export class WorkspaceCommandError extends Error implements WorkspaceFailure {
  constructor(
    public readonly code: WorkspaceErrorCode,
    public readonly source: 'backend' | 'transport' = 'backend',
    public readonly existingId?: string,
    public readonly outcomeUnknown = false,
    public readonly requiresReload = false,
  ) {
    super(
      outcomeUnknown
        ? requiresReload
          ? "Couldn't confirm workspace changes. Reload workspaces before trying again."
          : "Couldn't confirm workspace changes. The list was refreshed; check it before trying again."
        : messages[code],
    );
    this.name = 'WorkspaceCommandError';
  }
}

export function toWorkspaceCommandError(value: unknown): WorkspaceCommandError {
  if (value instanceof WorkspaceCommandError) return value;
  if (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    typeof value.code === 'string' &&
    Object.hasOwn(messages, value.code)
  ) {
    const existingId =
      'existingId' in value &&
      typeof value.existingId === 'string' &&
      /^[1-9]\d*$/.test(value.existingId)
        ? value.existingId
        : undefined;
    return new WorkspaceCommandError(
      value.code as WorkspaceErrorCode,
      'backend',
      existingId,
    );
  }
  return new WorkspaceCommandError('application_unavailable', 'transport');
}

async function command<T>(
  name: string,
  invokeFn: WorkspaceInvoke,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await (args === undefined
      ? invokeFn<T>(name)
      : invokeFn<T>(name, args));
  } catch (error) {
    throw toWorkspaceCommandError(error);
  }
}
export function listWorkspaces(
  invokeFn: WorkspaceInvoke = invoke,
): Promise<WorkspaceListing> {
  return command('list_workspaces', invokeFn);
}
export function selectAndAddWorkspace(
  invokeFn: WorkspaceInvoke = invoke,
): Promise<AddWorkspaceOutcome> {
  return command('select_and_add_workspace', invokeFn);
}
export function removeWorkspace(
  id: string,
  invokeFn: WorkspaceInvoke = invoke,
): Promise<RemoveWorkspaceResult> {
  return command('remove_workspace', invokeFn, { id });
}
export function setWorkspaceEnabled(
  id: string,
  enabled: boolean,
  invokeFn: WorkspaceInvoke = invoke,
): Promise<WorkspaceRecord> {
  return command('set_workspace_enabled', invokeFn, { id, enabled });
}
export function checkWorkspace(
  id: string,
  invokeFn: WorkspaceInvoke = invoke,
): Promise<CheckWorkspaceResult> {
  return command('check_workspace', invokeFn, { id });
}
export function skipWorkspaceSetup(
  invokeFn: WorkspaceInvoke = invoke,
): Promise<SetupWorkspaceResult> {
  return command('skip_workspace_setup', invokeFn);
}
