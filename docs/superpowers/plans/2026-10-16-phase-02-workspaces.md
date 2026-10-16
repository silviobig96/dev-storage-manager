# Phase 02 — Workspace System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans` to
> implement this plan task-by-task after human review. Steps use checkbox
> (`- [ ]`) syntax for tracking. Never create a worktree for this plan.

**Goal:** Persist zero or more explicitly selected folders without scanning or
modifying their contents.

**Architecture:** Extend the existing core, platform adapter, SQLite Database,
and ApplicationService. A narrow Rust Tauri bridge owns the native selector;
React reuses the approved shell and primitives for Overview and Workspaces.
Configuration is durable; availability is a bounded, temporary observation.

**Tech Stack:** Existing Tauri 2.11.5, React 19, TypeScript 6, Vite 8, Vitest,
Rust stable, rusqlite 0.37 and rusqlite_migration 2.3. Proposed additions are
limited to the reviewed spec's Rust dialog and existing-lockfile bindings.

**Spec:** [Phase 02 design](../specs/2026-10-16-phase-02-workspaces-design.md).

**Status:** Human-reviewed; implementation authorized by the current task.
Tasks 1–8 are implemented and reviewed; Task 9 is in progress. Only verified
steps may be checked. Consolidated functional gates pass;
`VISUAL QA: AWAITING HUMAN REVIEW`, so Phase 02 is not complete. Evidence and
outstanding checks are recorded in the [verification record](../reviews/2026-10-16-phase-02-workspaces-verification.md).

## Global Constraints

- Use the normal checkout and `feat/phase-02-workspaces`, based on merged
  `8c9fc88`. No worktrees, other branches, reset, clean, stash, rebase, or force.
- The current implementation task authorizes publishing this feature branch
  after functional gates pass and the checkout is clean. Never
  push main/tags, change origin, open a PR, merge, or release under this plan.
- No automatic deletion. Remove workspace removes only app-owned configuration.
- Zero workspaces is valid. Never assume `~/Repositories`, HOME, or another root.
- English UI; native titlebar; system appearance; existing semantic tokens.
- Preserve `get_app_health` fields/error contract and stale-result guards;
  schema version must reflect schema 2, not stay pinned to 1.
- Preserve `io.github.silviobig96.devstoragemanager` and the current DB location.
- Do not update the existing stack. Add only the spec's pinned Rust dependencies
  and controlled fixture dev-dependencies; no new JS packages or UI framework.
- No project discovery, manifests, recursion, sizes, providers, cleanup,
  monitoring, notifications, networking, telemetry, FDA, privileged helper,
  Swift, Windows implementation, or visual redesign.
- Functional, browser-rendered, native Tauri, and human QA are separate evidence.
  Phase 02 requires human visual approval before being called complete.
- Fixtures live in temporary directories, never personal repositories. No real
  database resets, personal paths, screenshots, or inventories committed.
- When a task changes a Cargo manifest, synchronize its dependency edges in
  Cargo.lock before using `--locked`. For already-cached additions in Tasks 2–4,
  run `cargo check --workspace --all-targets --offline` and inspect the lock diff;
  retain all existing package versions. Task 5 additionally resolves the new
  pinned plugin. A stale-lock failure is setup failure, not a valid TDD RED.

## Review Focus

- A slow/disconnected mount must not freeze health/navigation or accumulate
  workers: deadline and permit-release tests in Tasks 4–5.
- An alias retargeted during add must not register a different folder silently:
  injected two-observation race in Task 2 and no-write assertion in Task 4.
- Lost IPC after a successful commit must not show an incorrect rollback or
  blindly retry: reload reconciliation in Task 6.
- A schema-1 database with unrelated metadata must upgrade intact, and a failed
  migration must leave version/data unchanged: Task 3.
- Duplicate names and very long paths must remain distinguishable at 200% zoom:
  accessible-name tests in Task 8 and rendered/native matrix in Task 9.

## Files and dependency order

Task 1 defines contracts; Tasks 2 and 3 implement their platform and persistence
sides; Task 4 composes them. Task 5 exposes narrow commands, Task 6 consumes them,
Tasks 7–8 present them, Task 9 verifies the complete implementation. Execute in
that order; do not edit shared files concurrently.

New domain modules are `crates/core/src/workspaces.rs`,
`crates/platform/src/macos/workspaces.rs`,
`crates/persistence/src/workspaces.rs`, and
`crates/application/src/workspaces.rs`; keep existing health code recognizable.
Tauri gains `src-tauri/src/workspaces.rs` and `workspace_runtime.rs` (dialog and
bounded blocking dispatch respectively). Frontend feature state lives in
`src/features/workspaces`, transport in `src/lib/workspaces.ts`, presentation in
`src/views/WorkspacesView.tsx` and focused workspace components.

## Task 1: Define shared configuration and observation contracts

**Files:** Create `crates/core/src/workspaces.rs`; modify
`crates/core/src/lib.rs`. Tests live in the new module.

**Interfaces:**

- Consumes: existing serde and `std::path::Path`; no Tauri/SQLite/OS bindings.
- Produces: `WorkspaceId(i64)` with positive-decimal-string serialization;
  `DirectoryIdentity(String)` opaque internally; `ValidatedWorkspace` with
  `selected_path: String`, `canonical_path: String`, `identity: DirectoryIdentity`.
- `StoredWorkspace { id: WorkspaceId, root: ValidatedWorkspace, enabled: bool,
created_at: i64, updated_at: i64 }` is internal; `WorkspaceRecord` has the
  camelCase fields specified in the spec and derives its display name.
- `SetupState::{Undecided, Skipped, Configured}`; `WorkspaceConfiguration {
setup_state: SetupState, records: Vec<StoredWorkspace> }`;
  `WorkspaceListing { setup_state: SetupState, workspaces: Vec<WorkspaceRecord>,
overlaps: Vec<WorkspaceOverlap> }`.
- `AvailabilityState::{Pending, Present, Missing, Denied, Unknown}` and
  `AvailabilityObservation { state, reason: Option<AvailabilityReason>,
checked_at: Option<i64> }`, with reason values exactly from the spec.
- `WorkspaceFailure { code: WorkspaceErrorCode, existing_id: Option<WorkspaceId> }`
  carries only the spec's error codes; fixed English message mapping, no raw
  source error. `WorkspaceOverlap { parent_id, child_id }`;
  `known_overlaps(records: &[StoredWorkspace]) -> Vec<WorkspaceOverlap>`.
- Internal `WorkspaceInsertOutcome::{Added(WorkspaceRecord), Duplicate(WorkspaceId)}`
  distinguishes an ordinary duplicate from a failure. A same-path/different-
  identity conflict is instead path_already_configured with existing_id.

- [x] Write failing tests named `workspace_contract`, `workspace_id_validation`,
      and `workspace_overlap`. Pin assertions such as:

  ```rust
  assert_eq!(serde_json::to_value(WorkspaceId::try_from(7).unwrap()).unwrap(), "7");
  assert!(serde_json::from_str::<WorkspaceId>("\"0\"").is_err());
  assert!(serde_json::from_str::<WorkspaceId>("\"../folder\"").is_err());
  assert_eq!(serde_json::to_value(SetupState::Skipped).unwrap(), "skipped");
  assert_eq!(known_overlaps(&[parent.clone(), child.clone()]), vec![relation]);
  assert!(known_overlaps(&[parent, sibling]).is_empty());
  ```

  Build parent `/fixture/dev`, child `/fixture/dev/nested`, sibling
  `/fixture/developer` as synthetic records with different identities; these
  paths are data only, never opened. Add round-trip assertions for every DTO,
  unknown reasons and all error codes. Test IDs beyond JS safe integer range
  still serialize as strings, same-name roots remain distinct, and paths retain
  Unicode/case exactly. Reject negative/noncanonical IDs and numbers, not just 0.

- [x] Run `cargo test -p dev-storage-core workspace --locked`; require expected
      missing-contract failures before implementation.
- [x] Implement the contracts, deriving camelCase structs and snake_case enum
      values. Use component-based canonical path comparison for advisory overlap;
      equal identities are not overlaps. Do not add safety-policy or scanner types.
- [x] Rerun the focused command; require PASS, preserving health serialization.
- [x] Commit only core files: `feat: define workspace configuration contracts`.

## Task 2: Implement read-only macOS root identity and availability

**Files:** Create `crates/platform/src/macos/workspaces.rs`; modify
`crates/platform/src/macos.rs`, `crates/platform/src/lib.rs`,
`crates/platform/Cargo.toml`. Tests in the new module; add tempfile dev dependency.
Also update `TestPlatform` implementations in `crates/application/src/lib.rs`
and `src-tauri/src/commands.rs` for the two newly required trait methods.
Include Cargo.lock dependency-edge updates at this checkpoint.

**Interfaces:**

- Consumes: Task 1 root/observation/error types.
- Extend `PlatformAdapter` with
  `validate_workspace_root(&self, selected: &Path) -> Result<ValidatedWorkspace, WorkspaceFailure>`
  and `check_workspace_root(&self, root: &ValidatedWorkspace, now: i64) -> AvailabilityObservation`.
  Update fake adapters in existing health tests with explicit fixture-only stubs;
  do not introduce permissive production default implementations.
- Produces: MacOsAdapter implementations; private metadata-reader seam returning
  directory type, supported volume UUID, persistent-ID capability, file ID,
  birth time and canonical path for deterministic race/error tests.

- [x] Write `workspace_root_accepts_unicode_spaces_and_long_paths` with tempfile
      nested directories exceeding 260 total path bytes (each component valid),
      basename `Café 工具 space`; assert exact selected-path round trip and stable
      identity on repeat validation. Real fixtures do not depend on a personal path.
- [x] Write `workspace_root_aliases_and_case`: a controlled symlink and a path
      with `.`/`..` resolve to the same identity; two distinct same-name directories
      differ. On a case-insensitive fixture volume a resolvable case variant is the
      same identity. A fake metadata reader representing a case-sensitive volume
      must preserve distinct IDs for `Code` and `code`; label this simulated-volume
      coverage, not proof that the current disk is case-sensitive.
- [x] Write `workspace_root_rejects_changed_or_invalid_targets`: file,
      missing target, dangling link, symlink loop, non-UTF-8 Unix path, injected
      EACCES/EPERM, unsupported/all-zero UUID, and first/second identity mismatch.
      Assert exact error codes and no lossy identity/path. Real permission denial
      may vary with the test runner; deterministic injected errors are mandatory.
      Test truncated attribute buffers and missing returned-attribute bits:
      decoding returns identity_unavailable without reading beyond supplied bytes.
- [x] Run `cargo test -p dev-storage-platform workspace --locked`; record RED.
- [x] Implement the two metadata observations and versioned identity in the
      macOS module using target-specific libc `=0.2.189`, already in Cargo.lock.
      Validate returned attribute masks/lengths before decoding, no unaligned typed
      pointer dereferences. Root metadata only, never `read_dir`, contents or writes.
      Ensure symlink changes between observations are rejected. Do not request
      permanent-ID allocation. Map missing/denied/replaced roots to spec states.
- [x] Add `workspace_root_disconnect_and_return`: validate fixture, rename it
      inside its tempfile, assert missing, restore it, assert present with unchanged
      identity. Add replacement test expecting unknown/identity_changed. Snapshot a
      sentinel's bytes before/after every validation/check and assert unchanged.
      Add a root containing inaccessible descendants: root validation/check still
      succeeds without visiting them. The injected metadata reader must record
      requests only for selected/canonical root references and necessary volume
      metadata, with zero content/enumeration/mutation requests.
- [x] Rerun focused tests; then `cargo check --workspace --all-targets --locked`
      for adapter implementer compatibility. Commit:
      `feat: validate workspace roots through macOS metadata`.

## Task 3: Add transactional configuration persistence and schema 2

**Files:** Create `crates/persistence/migrations/0002_workspaces.sql` and
`crates/persistence/src/workspaces.rs`; modify `crates/persistence/src/lib.rs`
and `Cargo.toml` (path dependency on core). Update existing schema-version
assertions in application/Tauri health tests to 2, not serialized field names.
Include the corresponding Cargo.lock dependency-edge update.

**Interfaces (Database methods):**

- Consumes: `ValidatedWorkspace`, `WorkspaceId`, configuration types from Task 1.
- `workspace_configuration(&self) -> Result<WorkspaceConfiguration, WorkspaceFailure>`.
- `insert_workspace(&mut self, root: &ValidatedWorkspace, now: i64) -> Result<WorkspaceInsertOutcome, WorkspaceFailure>`.
- `remove_workspace(&mut self, id: WorkspaceId) -> Result<(), WorkspaceFailure>`.
- `set_workspace_enabled(&mut self, id: WorkspaceId, enabled: bool, now: i64) -> Result<WorkspaceRecord, WorkspaceFailure>`.
- `skip_workspace_setup(&mut self) -> Result<SetupState, WorkspaceFailure>`.
- Preserve existing `Database::open`, `open_in_memory`, metadata and health APIs.

- [x] Write `workspace_migrates_schema_one_without_data_loss`: manually construct
      a temporary SQLite DB using the real unchanged 0001 SQL and `user_version=1`,
      store sentinel metadata, close, open through Database, assert schema 2,
      sentinel unchanged, zero records, undecided. Reopen/reapply: still schema 2.
      A private test migration sequence with invalid second SQL must leave original
      version/metadata and no partially created table after rollback.
- [x] Write `workspace_round_trip`: insert two roots with distinct identity,
      reopen database, assert same IDs, exact paths, enabled=true, timestamps and
      configured setup. Test toggle persistence and idempotence; skip persists on
      reopen and never changes configured to skipped.
- [x] Write `workspace_unique_across_connections` with two independent
      connections, a barrier and busy timeout. Assert one successful insert and one
      duplicate result referencing that ID, total length 1. If initial contention
      returns persistence_unavailable, retry the losing operation after the first
      commits and require duplicate; never accept two successes. Also test exact
      selected/canonical-path conflicts with a different identity return
      path_already_configured. Do not `ON CONFLICT UPDATE` a workspace.
- [x] Write `workspace_failure_rolls_back_setup`: inject an aborting SQLite
      trigger in a temp DB on the setup metadata write; assert failed add leaves
      zero rows and undecided. Test unknown IDs, invalid stored setup metadata,
      sanitized query/constraint failures and unchanged sentinel metadata.
      `workspace_remove_last_keeps_setup` asserts zero rows/configured after remove
      and after reopen; a new insert gets a different ID.
- [x] Run `cargo test -p dev-storage-persistence workspace --locked`; record RED.
- [x] Append the migration to `Migrations::new` in order; implement STRICT
      schema/unique keys exactly as spec, bound SQL, immediate add transaction,
      atomic setup update, one-second busy timeout and committed-result mapping.
      Reuse the existing sanitized PersistenceError behavior; typed duplicate
      resolution must not expose driver errors. No real DB reset or new database.
- [x] Rerun focused tests and existing persistence tests. Commit scoped files:
      `feat: persist workspace configuration with schema migration`.

## Task 4: Coordinate workspace use cases in ApplicationService

**Files:** Create `crates/application/src/workspaces.rs`; modify
`crates/application/src/lib.rs`, application Cargo.toml (tempfile dev only),
`src-tauri/src/lib.rs` and existing health command state access as necessary.
Include Cargo.lock if adding the test-only dependency changes its graph.

**Interfaces:**

- Consumes: existing `ApplicationService<A>`, Task 2 adapter, Task 3 Database.
- Keep `new(adapter: A, database: Database) -> Self` and
  `health(&self) -> Result<AppHealth, ApplicationError>`. Move the Database mutex
  inside the service; Tauri stores an `Arc<ApplicationService<MacOsAdapter>>`
  instead of a mutex wrapping the entire service. Preserve health behavior.
- `list_workspaces(&self) -> Result<WorkspaceListing, WorkspaceFailure>`.
- `prepare_workspace(&self, selected: &Path) -> Result<ValidatedWorkspace, WorkspaceFailure>`.
- `add_workspace(&self, root: ValidatedWorkspace, now: i64) -> Result<WorkspaceInsertOutcome, WorkspaceFailure>`.
- `remove_workspace(&self, id: WorkspaceId) -> Result<(), WorkspaceFailure>`.
- `set_workspace_enabled(&self, id: WorkspaceId, enabled: bool, now: i64) -> Result<WorkspaceRecord, WorkspaceFailure>`.
- `check_workspace(&self, id: WorkspaceId, now: i64) -> Result<AvailabilityObservation, WorkspaceFailure>`.
- `skip_workspace_setup(&self) -> Result<SetupState, WorkspaceFailure>`.
  prepare/add are Rust-only stages; no deserializable validated-root IPC input.

- [x] Write `workspace_use_cases` with counted fake adapter and temporary DB:
      list/skip/enable/remove make zero FS calls; prepare validates once; add persists
      once; nested roots return overlap and remain two records. Known-ID check
      loads stored root, calls metadata adapter once, never mutates configuration.
- [x] Write `workspace_validation_failure_never_saves`: all validation errors,
      changed identity and duplicate result leave setup and records unchanged.
      Keep count/assertions on adapter and repository state, not source strings.
- [x] Write `workspace_remove_preserves_files`: temp root with `sentinel.txt`
      containing `keep me`, register, remove by ID, assert root still a directory,
      bytes still `b"keep me"`, and list empty/configured after reopen. Add disabled
      and missing-root removal cases; no platform call needed for config removal.
- [x] Write `workspace_slow_check_does_not_lock_database`: fake adapter blocks on
      a barrier; while check waits on another thread, health and list complete and
      reflect schema 2. Release/join the fixture thread without timing-only sleeps.
- [x] Run `cargo test -p dev-storage-application workspace --locked`; record RED.
- [x] Implement short DB lock regions, platform work outside them, error mapping
      and conversion to public DTOs. No file traversal, provider/service registry,
      generalized repository framework or cleanup dependency.
- [x] Rerun focused tests plus existing health tests in application and Tauri.
      Commit: `feat: coordinate workspace configuration operations`.

## Task 5: Expose narrow commands and the native picker securely

**Files:** Create `src-tauri/src/workspaces.rs`, `workspace_runtime.rs`; modify
`src-tauri/src/lib.rs`, `commands.rs`, `Cargo.toml`, `build.rs`,
`capabilities/default.json`, `Cargo.lock` only for approved resolution.

**Interfaces:**

- Consumes: Task 4 service in Arc; injected Tauri WebviewWindow for caller checks.
- Commands and their exact payloads/results are the spec's IPC table. Define
  Rust `AddWorkspaceOutcome::{Cancelled, Added { workspace, availability },
Duplicate { existing_id }}` with internally tagged snake_case `status` and
  camelCase fields. Remove returns `RemoveWorkspaceResult { removed_id }`;
  skip returns `SetupWorkspaceResult { setup_state }`; check returns
  `CheckWorkspaceResult { id, availability }`.
- `WorkspaceRuntime` owns one picker gate and two worker permits. Expose
  `try_begin_selection(&self) -> Result<SelectionGuard, WorkspaceFailure>`
  and an async bounded validation dispatcher accepting only Rust closures.
  `SelectionGuard`/worker permits release via RAII, never by JS instruction.
- Private picker seam:
  `type PickResult = Result<Option<PathBuf>, WorkspaceFailure>`;
  `type PickCallback = Box<dyn FnOnce(PickResult) + Send>`;
  `trait WorkspacePicker { fn pick(&self, callback: PickCallback); }`.
  Production implementation owns a clone of the injected main window and
  passes no user-provided options. Test implementation completes callbacks
  explicitly without displaying a dialog.

- [x] Write `workspace_transport_cancel_and_busy`: hold a fake picker callback,
      second add returns operation_in_progress; complete with None, assert cancelled,
      unchanged DB/setup, gate reusable. Complete with error, assert gate reusable.
      An unexpected JS `path` field must never reach root validation: only the
      fake picker's selected result can do so; extra fields are not an add API.
      No path argument exists on the registered command. On dropped receiver, callback
      must not panic or save after cancellation.
- [x] Write `workspace_transport_deadline`: inject a 10ms test deadline (production
      3 seconds) and a barrier-blocked validation. Assert validation_timeout, zero
      DB writes, two held permits prevent a third worker, and late completion cannot
      insert. Manual check timeout returns unknown/timeout; permits are released
      only after underlying jobs finish. Do not timeout the separate DB commit
      and then falsely claim it failed; return its actual committed outcome.
- [x] Write `workspace_transport_contract`: fake-picker add creates a row,
      duplicate returns existingId, errors expose only code/message/optional ID.
      Check field casing, malformed IDs and nonboolean enabled rejected before work.
      Preserve existing health contract tests, with real schema version 2.
- [x] Write `workspace_acl` using Tauri's existing test/mock-runtime facilities
      (enable the Tauri `test` feature for dev only if needed). Exercise a fully
      generated context/authority and invocation, not only a label helper:
      local main commands allowed; ungranted window/webview and remote origin denied;
      `plugin:dialog|open`, `plugin:dialog|save`, `plugin:fs|read_file` and shell
      invocation denied/not registered. Picker/service call counters must remain
      zero for denied callers. Include get_app_health in the main allowlist.
- [x] Run `cargo test -p dev-storage-manager workspace --locked`; record RED.
- [x] Implement Rust callback picker and async command dispatch. Add dialog
      `=2.4.0`, Tokio `=1.53.1` with `sync,time`. Resolve the transitive FS plugin
      to 2.4.2 in Cargo.lock without initializing it. The dialog package's compatible
      manifests are linked in the spec; never run the broad Tauri add wizard that
      installs a JS package and permissions. Resolve once during implementation;
      subsequent checks use `--locked`. Reject resolution that upgrades existing
      Tauri/frontend/toolchain versions. Audit optional watcher features remain off.
      Use `spawn_blocking` for DB work as well as metadata; an async command must
      not run synchronous SQLite contention on the event-loop thread.
- [x] Declare all seven app commands in `AppManifest::commands` in build.rs;
      add their generated `allow-*` entries to default capability with `local: true`,
      `webviews: ["main"]` only and no `windows` grant (OR semantics), no remote.
      Retain core:default. Add injected main webview and real containing-window
      guards; ACL enforces origins. Test a second webview in the main window
      cannot inherit commands. No dialog/fs/shell grants or generic
      scopes. Callback uses oneshot; blocking metadata uses two retained permits
      and timeout; DB jobs are short and separate. No mutex is held across await.
- [x] Run focused tests, `cargo check --workspace --all-targets --locked` and
      `cargo tree -e features -p dev-storage-manager`; inspect generated ACL manifest
      and dependency diff. Build validates real permission names. Commit:
      `feat: add constrained native workspace commands`.

## Task 6: Add typed frontend transport and race-safe feature state

**Files:** Create `src/lib/workspaces.ts`, `workspaces.test.ts`,
`src/features/workspaces/useWorkspaces.ts`, `useWorkspaces.test.tsx`.
Do not replace `src/lib/desktop.ts` or its tests.

**Interfaces:**

- Consumes: Task 5 IPC DTOs; mirror their exact names, string IDs, enums and
  optional error ID in TypeScript. Keep internal identity out of TS.
- Define `WorkspaceInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>`.
  Bridge signatures are `listWorkspaces(invokeFn?: WorkspaceInvoke): Promise<WorkspaceListing>`,
  `selectAndAddWorkspace(invokeFn?: WorkspaceInvoke): Promise<AddWorkspaceOutcome>`,
  `removeWorkspace(id: string, invokeFn?: WorkspaceInvoke): Promise<RemoveWorkspaceResult>`,
  `setWorkspaceEnabled(id: string, enabled: boolean, invokeFn?: WorkspaceInvoke): Promise<WorkspaceRecord>`,
  `checkWorkspace(id: string, invokeFn?: WorkspaceInvoke): Promise<CheckWorkspaceResult>`,
  `skipWorkspaceSetup(invokeFn?: WorkspaceInvoke): Promise<SetupWorkspaceResult>`.
- `useWorkspaces(): WorkspacesController` returns `state` (loading/error/ready with listing),
  `availability: Record<string, AvailabilityObservation>`, `adding: boolean`,
  `busyIds: Set<string>`, `error: WorkspaceCommandError | null`, and actions
  `reload(): Promise<void>`, `add(): Promise<AddWorkspaceOutcome>`,
  `remove(id: string): Promise<RemoveWorkspaceResult>`,
  `setEnabled(id: string, enabled: boolean): Promise<WorkspaceRecord>`,
  `check(id: string): Promise<CheckWorkspaceResult>`,
  `skip(): Promise<SetupWorkspaceResult>`. Actions reject with typed
  errors on failure. Hook lives at App level so view navigation doesn't discard
  configuration or create automatic availability requests.

- [x] Write `workspace bridge sends only narrow arguments`: injected invoke spy
      requires exact command names and payloads, no path/options. For example:

  ```ts
  await setWorkspaceEnabled('7', false, invoke);
  expect(invoke).toHaveBeenCalledWith('set_workspace_enabled', {
    id: '7',
    enabled: false,
  });
  ```

  Pin all other wrappers analogously to the IPC table, including cancelled as
  a resolved outcome, errors as rejected values.

- [x] Write hook tests with controlled promises and renderHook/act: initial
      zero+skipped remains ready/empty; no automatic add/check; cancel doesn't
      mutate; add and duplicate reconcile authoritative listing; failed SQL leaves
      rows/setup unchanged; enabled/remove change only after success. Double-click
      attempts call selection once. Last removal never changes configured setup.
- [x] Write `workspace ignores stale results`: StrictMode load restart,
      out-of-order reload and checks, removal during a check, late resolve/reject
      after unmount. Assert no revived row/overwritten newer observation. On transport
      failure after a committed remove, mock reload without that ID; require
      reconciliation and no automatic second remove or false rollback announcement.
- [x] Run `npm run test:run -- src/lib/workspaces.test.ts src/features/workspaces/useWorkspaces.test.tsx`;
      record RED. Implement minimal wrappers and request generations/in-flight refs,
      pessimistic mutations, authoritative reload and observable retry errors.
- [x] Rerun focused tests, lint and build (includes `tsc -b`). Commit:
      `feat: manage workspace configuration state in the frontend`.

## Task 7: Activate navigation and optional setup without blocking Overview

**Files:** Modify `src/App.tsx`, `App.test.tsx`,
`src/components/shell/AppShell.tsx`, `Sidebar.tsx`, `SidebarItem.tsx`,
`Sidebar.css`, `shell.test.tsx`; create
`src/features/workspaces/WorkspaceSetup.tsx`, `WorkspaceSetup.test.tsx`.
Extend `ContentHeader.tsx` with optional `action: ReactNode` and a focusable
heading ref only if the view needs them; preserve existing rendering defaults.

**Interfaces:**

- Consumes: Task 6 hook/actions, existing health state and components.
- `type AppView = 'overview' | 'workspaces'` exported from AppShell;
  AppShell/Sidebar accept `currentView: AppView`,
  `onNavigate: (view: AppView) => void`. SidebarItem uses an explicit available
  branch with an actual button and noninteractive unavailable branch.
- `WorkspaceSetup({ adding, error, onAdd, onSkip })` receives controlled state
  and callbacks; no direct IPC. It appears only for undecided setup; Workspaces
  empty state uses the same copy/actions, not a second stacked prompt.

- [x] Update shell tests first: available Overview/Workspaces buttons operable
      by keyboard; selected button has aria-current page; other destinations have
      no button/link/tabindex. After explicit navigation current main name/header
      and focused h1 change; skip link still reaches main. Update old
      Phase-01.5-only no-buttons assertions narrowly, not health protections.
- [x] Write setup tests: neither app render nor navigation opens picker;
      Skip calls action once, prompt disappears only on saved skipped/configured;
      failure remains actionable. Overview stays usable when workspace loading
      fails. Health loading/connected/unavailable and StrictMode stale-result tests
      remain intact with independent workspace mocks.
- [x] Run `npm run test:run -- src/components/shell/shell.test.tsx src/features/workspaces/WorkspaceSetup.test.tsx src/App.test.tsx`;
      record RED. Implement controlled two-view navigation, optional inline setup
      and focus transfer only for explicit view changes. Keep current health effect,
      Overview width and native titlebar unchanged. Task 8 supplies Workspaces view;
      do not publish a fake route as a completed feature before it is connected.
- [x] Rerun focused tests and commit this navigation checkpoint together with
      Task 8's connected view if the intermediate app would not compile. Do not
      weaken the gate to commit a broken placeholder. Checkpoint message:
      `feat: enable workspace navigation and optional setup`.

## Task 8: Present workspace rows and configuration actions accessibly

**Files:** Create `src/views/WorkspacesView.tsx`, `WorkspacesView.css`,
`WorkspacesView.test.tsx`, `src/features/workspaces/WorkspaceRow.tsx`,
`RemoveWorkspaceDialog.tsx`, `RemoveWorkspaceDialog.test.tsx`;
connect in App. Reuse DataList/ListRow, Section, EmptyState, Button, tokens.
Keep feature styles scoped in WorkspacesView.css; no global palette edits.

**Interfaces:**

- Consumes: return value of `useWorkspaces`, view heading ref from Task 7.
- `WorkspacesView({ workspaces, headingRef })` consumes that controller and ref;
  `WorkspaceRow` takes `WorkspaceRecord`, observation, overlap flag, busy flag
  and callbacks by ID. It does not own backend rules or invent status data.
- `RemoveWorkspaceDialog({ workspace, busy, error, onCancel, onConfirm })`
  uses native HTML dialog semantics/showModal, not simulated window chrome;
  confirmation callback receives workspace ID only. Parent owns focus fallback.

- [x] Write concrete tests for loading/error/retry/empty and one/multiple rows.
      For two `Code` names with different fixture paths, query actions by full
      accessible name and assert each targets the correct ID. Full long path must
      be present in text, not exclusively title. Pending is Not checked; absent,
      denied, unknown and found use exact meanings from spec. Enabled and Check
      call only their own callbacks, no analysis action. Overlap message is visible.
- [x] Write removal tests using a controlled save promise: initial Cancel focus,
      Escape/cancel sends no remove, exact reassurance copy visible, confirm sends
      ID once, row persists while saving/failure, busy prevents duplicate/dismissal,
      success removes row and restores next/previous/Add focus. Removing last leaves
      configured empty, no Skip prompt. Add/cancel focus restores to its invoker;
      duplicate focuses existing row, not a success row.
- [x] Run `npm run test:run -- src/views/WorkspacesView.test.tsx src/features/workspaces/RemoveWorkspaceDialog.test.tsx`;
      record RED. Use a test-only dialog polyfill if jsdom lacks showModal; this
      cannot prove native focus trapping or actual macOS dialog behavior.
- [x] Implement compact rows, exact copy, full selectable wrapping path,
      checkbox labels, polite asynchronous announcements and restrained error text.
      Fill available structural width, wrap actions under metadata as needed; no
      overflow hiding, cards or new visual theme. Reuse system contrast/motion rules.
      Every visible control must work. Preserve markerless-list role=list.
- [x] Rerun focused tests, full frontend suite and build. Commit coherent view
      checkpoint: `feat: add accessible workspace configuration view`. If Task 7 was
      deferred, include its files and record both checkpoints as one coherent commit.

## Task 9: Verify the integrated boundary and hand off human review

**Files:** Create `docs/superpowers/reviews/2026-10-16-phase-02-workspaces-verification.md`;
update this plan checkboxes and architecture/roadmap implementation status only
to match actual evidence. Fix only defects in this plan's owning files.

**Interfaces:** Consumes the implemented Tasks 1–8. Produces an evidence record,
scoped commits and a human-review handoff, not a scanner or automatic phase PASS.

- [x] Run the consolidated functional gates once after focused tests:

  ```bash
  npm run format:check
  npm run lint
  npm run test:run
  npm run build
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo test --workspace --locked
  npm run tauri build -- --debug
  git diff --check
  ```

  Require exit 0 for every command; build includes TypeScript via `tsc -b`.
  If a defect changes code after the gate, rerun relevant focused tests and
  impacted broad gates. Do not rerun each Rust crate and then workspace without
  a new reason. Record command output totals, not expected results as actual.

- [x] Audit `git diff 8c9fc88 --stat` and the complete diff from that base:
      commands only as specified, no backend scope creep, no FS write in selected
      roots, no recursive reads, no scans/hooks/watchers, no remote grants or wide
      frontend FS API, no stack upgrades, no personal data. Inspect actual code and
      Cargo feature tree, not only keyword searches. Verify migration 0001 unchanged,
      metadata preserved, duplicate constraint active, schema health=2, and exact
      bundle identifier unchanged. Audit UI inputs cannot bypass root validation.
- [ ] Launch `npm run tauri dev` in the normal checkout. Use only synthetic
      temporary fixture folders through the picker. Verify native folder-only
      selection, cancellation, no duplicate panel on rapid activation, add, restart
      persistence, enable, manual check, remove confirmation and unchanged sentinel
      contents. Native smoke test the actual ACL main allow/other-origin denial;
      mock runtime coverage alone cannot be called native IPC QA.
- [ ] Render both views at 760 × 520, 1100 × 720, 1600 × 900 and maximized in
      light/dark; record CSS viewport dimensions, not assumed Retina dimensions.
      Measure Overview and Workspaces against main client width minus padding;
      require full width within 1 CSS px, full-width separators, centered empty
      states and no shell horizontal overflow. Long path and all row actions remain
      reachable at 200% zoom. Browser measurements prove browser layout only.
- [ ] Check keyboard order, current selection vs focus, skip link, labels,
      native picker focus return, HTML dialog Cancel/Escape/trap/return, actual screen
      reader list/path reading, increased contrast and reduced motion where possible.
      Do not change global macOS preferences without permission. Record unsupported
      or unavailable conditions as pending, separately from automated coverage.
- [x] Record loading/empty/error, SQLite failure/no false success, missing/denied/
      reappeared observations and health regression. A simulated native picker or
      forced fixture state is labelled as such. No production mock mode is added.
      Captures stay outside Git, sanitized and labelled browser vs native.
- [x] Fix any discovered defect with focused regression first. Commit
      `test: verify workspace configuration boundaries` or a precise defect message;
      commit evidence with `docs: record workspace system verification`.
- [x] Require clean checkout and exact feature branch. Push only if the current
      implementation task authorizes it, using
      `git push -u origin feat/phase-02-workspaces`; verify
      `git ls-remote --heads origin feat/phase-02-workspaces` matches
      `git rev-parse HEAD`. No force, PR, merge or release.
- [ ] Report functional result, actual browser/native coverage and pending checks.
      Request human review; only explicit approval of the running Phase 02 UI may
      establish human visual PASS. Otherwise leave roadmap implementation status
      incomplete. Phase 03 is not executed under this plan.

## Implementation verification status

Tasks 1–8 are implemented and reviewed, including fixes `8a3aa63` and `5656296`.
Task 9's consolidated functional commands pass: frontend 105 tests in 11 files,
Rust 48 unit tests, strict lint/Clippy, frontend/debug app build and formatting.
The formatting-only test-file correction has a focused 6-test pass. The
[verification record](../reviews/2026-10-16-phase-02-workspaces-verification.md)
details scope, dependency/ACL audit, measured browser coverage and outstanding
native/accessibility checks. Partially exercised checklist items remain
unchecked. `VISUAL QA: AWAITING HUMAN REVIEW`; the phase is not complete and
Phase 03 has not been executed.

Final whole-branch review at `765631d` reported no findings. The controller
published that checkpoint to `feat/phase-02-workspaces` with a normal push and
verified the remote head exactly matched local HEAD; the verification record
contains the full receipt. This receipt does not assert publication of later
documentation commits. Native/accessibility checks and human visual acceptance
remain pending.

## Documentation-run verification record

The design/planning run checks repository base, document consistency, local
links, format and diff scope. It does not execute Tasks 1–9, install packages,
create SQL files, or claim native/functional QA for unimplemented behavior.
Historical Phase 01.5 evidence remains unchanged. The human must review the
proposed enabled/manual-check/nesting/identity/native-dependency decisions in
the spec before this plan is executed.

Preparation evidence (2026-10-16):

- Clean normal checkout and unchanged expected origin confirmed before edits.
- `git fetch origin` and `git pull --ff-only origin main`: succeeded; base is
  `8c9fc88`, the Phase 01.5 merge, containing wide-window fix `8173c67`.
- `git merge-base --is-ancestor 8173c6797ecd2b7624fad87ac975e27fc5819831 origin/main`:
  exit 0. Created `feat/phase-02-workspaces`; no dependent feature-branch base.
- Read the required product/architecture/safety/provider/design documents,
  Phase 01.5 artifacts and verification record, current frontend/Rust code,
  migration, tests, manifest versions and capabilities. Reviewed official Tauri
  documentation and tagged plugin source, without installation.
- Spec/plan self-review: nine ordered tasks cover the flows, identity,
  persistence, IPC/permissions, error races and separate QA gates. All
  implementation checkboxes remain unchecked. No placeholder test bodies.
- Local-link validation: eight relative Markdown links resolve. No unresolved
  placeholder markers in the new spec/plan. This supplements manual review;
  searches are not a substitute for checking behavior and scope.
- `npm run format:check`: PASS after formatting the two new documents.
- `git diff --check`: PASS. Reviewed scope is documentation only; product code,
  SQL, manifests, lockfiles, toolchain and Phase 01.5 QA record are unchanged.
- Rust/frontend functional tests and native/browser UI QA were not run for this
  documentation-only task; their future commands/checklists are not results.
