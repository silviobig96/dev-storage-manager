# Phase 02 — Workspace System Design

**Status:** Proposed specification; human review required before implementation.

**Date:** 2026-10-16. **Base:** `8c9fc88` on `origin/main`, including Phase
01.5 and wide-window correction `8173c67`. **Branch:** `feat/phase-02-workspaces`.

## Intent and authority

Remember where a developer chooses to keep projects, without inspecting those
projects. A workspace is configuration for future analysis, not evidence that a
scan occurred. This document and its [implementation plan](../plans/2026-10-16-phase-02-workspaces.md)
are documentation only; neither authorizes implementation until human review.

Already approved requirements:

- Zero, one, or multiple explicitly selected folders; no assumed location,
  including `~/Repositories`, HOME, or the current working directory.
- First-run **Add workspace** and **Skip for now**, without a mandatory wizard.
- The existing macOS visual direction, system appearance, and native titlebar.
- No automatic deletion. Adding a folder neither starts analysis nor authorizes
  cleanup. Removing configuration never removes or changes files on disk.

Proposed decisions for review, not historical user approvals:

1. Include an **Enabled** checkbox and manual **Check availability** per row.
   Enabled means eligible for a future explicitly requested analysis, not a timer
   or scan trigger. Disabled folders remain visible, checkable, and removable.
2. Allow nested folders with an informational overlap warning. Never merge or
   remove their records automatically.
3. Keep initial setup as `undecided`, `skipped`, or `configured`; record skip
   persistently and never reset setup when the final folder is removed.
4. Use persistent macOS volume/directory identity, rejecting an add when reliable
   identity cannot be obtained. Reject non-UTF-8 paths with a clear error rather
   than saving a lossy path. Ordinary Unicode is supported without normalization.
5. Use a Rust-only official dialog plugin pinned to a compatible version, plus
   small direct uses of already-locked libc/Tokio described below. No JS plugin.

## Existing foundation and alternatives

Reuse `ApplicationService<A>`, `Database`, `PlatformAdapter`, and the shared core.
The current database has only `application_metadata`, schema version 1. The
current frontend has `AppShell`, `Sidebar`, `ContentHeader`, `Section`, `DataList`
(explicit WebKit list semantics), `ListRow`, `EmptyState`, `Button`, local SVGs,
and a guarded `getAppHealth()` effect. Do not replace this structure or install a
router for two views.

A JS dialog plus arbitrary-path add command is simpler to wire but broadens the
frontend's authority unnecessarily. A handwritten AppKit/Swift picker duplicates
the official integration. Choose the Rust dialog bridge: JS requests selection
and addition as one operation; the selected path never becomes a JS input to a
filesystem command. Platform validation remains independent of Tauri.

## User journeys

### Startup and navigation

Start on Overview as today. Load workspace configuration independently of health;
neither request blocks the shell. If setup is `undecided`, show a compact inline
setup section: **Choose workspace folders**, “Workspaces are optional. Adding a
folder does not scan it.”, **Add workspace**, **Skip for now**. Do not open a
dialog automatically or select a default folder. The same choice is available
in the Workspaces empty state, without duplicating Add buttons within that view.

Skip writes `skipped` only after SQLite succeeds. Failure leaves the prompt and
an inline retryable error. Successful addition writes `configured` atomically
with the folder record; adding after skip also becomes `configured`. Cancel or
failed addition changes neither list nor setup. Remove never changes setup.
An upgraded schema-1 installation has absent setup metadata and zero workspaces,
so receives this nonblocking experience without losing existing metadata.

Overview and Workspaces are the only functional sidebar destinations: semantic
buttons with `aria-current="page"` on the current button. All other destinations
remain noninteractive, visibly unavailable. Keep current selection in memory,
not another preference. On explicit navigation, move focus to the new view's
heading (`tabIndex={-1}`); do not steal focus on initial load or async updates.
Keep the skip link and one named main landmark with the current view's name.

### Empty and loading

Workspaces title: **Workspaces**. Context: “Folders configured for future
analysis. No files are scanned.” Empty title: **No workspaces added**. Explain
“You can add folders now or keep using the app without them.” Offer Add workspace;
offer Skip only while setup is undecided. Zero is valid after skip or removal.

Loading announces “Loading workspaces…” politely inside the stable shell.
Read failure shows “Couldn't load workspaces.” and **Retry**; it must not masquerade
as an empty configuration. Overview and its health states continue to work.

### Add

One native folder per operation, with title **Add workspace**, parented to the
main window. No starting-directory override, filename, filters, multiple
selection, or creation of new folders (`set_can_create_directories(false)`).
macOS may choose its own navigation location; that is not an app-selected root.
Cancel/Escape is an ordinary `cancelled` outcome, without an alert or DB write.

Frontend disables Add while selecting, validating, or saving. A backend global
selection gate rejects a second request with `operation_in_progress`; release it
on cancellation, error, and completion. A lost frontend response does not release
the gate while its native dialog is still open. No DB lock spans the dialog.

After success show the committed row and announce “Workspace added. No files
were scanned.” Duplicate selection instead focuses the existing row and says
“This folder is already configured.” A path already registered for a different
identity says “This location has changed. Remove its old entry before adding
it again.” No implicit replacement, enabled change, or setup mutation.

### List and actions

Use `DataList`/`ListRow`: derived folder name, complete selected path, availability
text, Enabled checkbox, Check availability and Remove workspace text buttons.
Use row-specific accessible names incorporating name and path, since folder names
can repeat. Order by creation time then numeric ID; do not sort by availability.
Display names are derived from the chosen basename, not custom editable labels;
use **Filesystem root** when no basename exists.

Paths wrap with `overflow-wrap: anywhere`, remain selectable and available to
assistive technology in full, and are not shortened with ellipsis or a mouse-only
tooltip. Name/path/status precede actions in DOM order. At narrow widths or zoom,
actions wrap below metadata. Structural width fills the main pane's interior;
only explanatory paragraphs may have a reading-width cap. Preserve Overview's
`inline-size: 100%` correction and full-width separators/empty-state centering.

Checkbox and remove operations are pessimistic: retain committed state until
SQLite succeeds. Disable the affected row's conflicting actions while saving;
keep navigation and unrelated content usable. Show “Couldn't save workspace
changes. Try again.” on persistence failure, with no false success message.
On ambiguous transport failure reload configuration before offering a mutation
retry; do not assume the database rolled back merely because IPC was lost.

### Remove confirmation

Show an accessible HTML `dialog` titled **Remove workspace?**, identifying name
and full path, with exact copy:

> This only removes the workspace from the app.
> Files on disk will not be deleted.

Buttons: **Cancel** and **Remove workspace**. Initial focus is Cancel; Escape
cancels before submission. Confirmation sends only the ID. While saving, keep
the dialog busy and prevent duplicate submission or dismissal that implies
cancellation of a committed operation. Failure retains the row and error in the
dialog. Success removes the row and returns focus to the next row's first action,
the previous row if last, or Add workspace if empty. Cancel restores the invoker.
This is configuration removal, not a destructive-filesystem button or new safety
classification. No trash, unlink, move, shell, or cleanup API is involved.

## Root validation and identity

The native result must be a local absolute `PathBuf`, not a URL or JS-supplied
path. A symlink to an existing directory is accepted; a file, dangling link,
symlink loop, inaccessible destination, unsupported identity, or disappearance
before validation is rejected without mutation. No Git, manifest, contents,
language, or descendant check is permitted. Spaces, long paths, and Unicode
are handled with `Path`/OS APIs, not split strings, lowercase conversion, shell
commands, or invented path-length limits. UI rejects no path by folder name.

Retain both selected path and canonical resolved path, losslessly as UTF-8 TEXT
for this phase. No Unicode normalization or case folding is performed. Internal
`DirectoryIdentity` is an opaque, versioned key, never sent to the frontend:
`macos-v1:<volume-uuid-hex>:<file-id-hex>:<birth-seconds>:<birth-nanoseconds>`.
UUID/file ID encode native bytes/integers; they are not path transformations.

The macOS adapter obtains volume UUID, 64-bit file ID, and creation timestamp
through read-only root metadata (`getattrlist`/`fgetattrlist`, supported-attribute
validation, and directory metadata). Require nonzero supported volume UUID and
persistent object-ID capability. Do not use transient device numbers alone as
persistent identity or request attributes that allocate permanent IDs on old
filesystems. Encapsulate and document any unsafe FFI buffer parsing. Birth time
reduces inode-reuse collisions; it is not proof of immutable identity, and this
key is never future deletion authority. Unsupported volumes produce
`identity_unavailable`, not a lexical fallback that claims reliable deduplication.

The read-only attribute contract is grounded in Apple's
[getattrlist manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getattrlist.2.html)
and [current attribute definitions](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/attr.h).
Request only identity/type/creation and volume capability attributes, never
directory entry counts, allocation sizes, or other storage measurements.

Validate the selected and canonical reference twice around identification,
checking directory type and matching identity. This catches replacement,
disappearance, and symlink retargeting observed before save. Persist immediately
after successful validation, not after another user confirmation or FS traversal.
Filesystem state and SQLite cannot be made one atomic transaction: a change
after the final observation is possible. A subsequent check must detect it and
never silently bind the configuration to a replacement.

Two paths resolving to the same identity are duplicates, regardless of symlink,
spelling, case equivalence, or Unicode equivalence on that volume. Different
directories with the same name, including case variants on case-sensitive
volumes, are distinct. SQLite enforces unique identity inside an immediate
transaction; concurrent adds cannot both succeed. Exact selected-path and
canonical-path uniqueness (binary collation) also prevent stale records from
silently being rebound. Conflicts return the existing ID; they never upsert.

### Nested folders and future scanner handoff

Canonical `Path::starts_with` component ancestry (not string prefix) identifies
known nesting among saved roots. Equal identities are handled as duplicates,
not nesting. Show “Overlaps another workspace. Future analysis must avoid double
counting.” on both involved rows. This is an advisory relation computed from
configuration, not a security/scope proof; stale paths or equivalent spellings
may leave overlap uncertain. Never enumerate ancestors' children to improve it.

Future scanner input must include enabled IDs, chosen/canonical paths, expected
identities and known overlaps; re-identify roots before scanning and union
visited filesystem identities across all roots. Nested roots must not be summed
independently. Disabled roots are excluded. Symlinks inside roots, mount changes,
and file hard links need scanner-owned policy and tests in Phase 03, not a scanner
implementation here. Selection alone does not start that future operation.

## Availability is an observation

Listing reads configuration only. After restart every row is **Not checked**;
there is no startup scan, polling, watcher, timer, or automatic recheck on view
navigation. Add supplies its genuine validation observation; manual Check
availability refreshes a saved root by ID. Disabled roots may still be checked.

| State     | Visible meaning                                                   |
| --------- | ----------------------------------------------------------------- |
| `pending` | Not checked, or Checking folder… while an explicit request runs   |
| `present` | Folder found. Access to its contents has not been checked.        |
| `missing` | Folder not found. Reconnect the disk or check the saved location. |
| `denied`  | Access to this folder was denied.                                 |
| `unknown` | Couldn't confirm availability. Try checking again.                |

Unknown reasons are `timeout`, `identity_changed`, `not_directory`,
`unsupported_identity`, or `io`. Identity change adds “The location now refers
to a different folder. Remove and add it again to change configuration.”
Permission errors (`EACCES`/`EPERM`) map to denied; not-found to missing. Never
infer full descendant access from successful root metadata. Preserve records
and enabled values on every availability outcome. Moved folders are not searched
for; a disk returning at the same path with the same identity can become present
again. A different mount path requires explicit removal and reselection.

Run metadata work off the main/async event-loop thread, without a DB lock. Limit
to two in-flight root-validation workers, reject excess work as busy, and return
an unknown timeout observation after 3 seconds for a manual check (an add returns
`validation_timeout`, no insert). An OS metadata syscall may outlive the timeout:
retain its permit until it actually finishes, ignore its late result, and do not
queue unbounded retries. A timed-out validation worker cannot commit anything.
The short DB commit stage follows successful validation separately and is not
cancelled in a way that reports failure while still committing. Set SQLite busy
timeout to 1 second and report persistence contention, without retry loops.

## Layer contracts and IPC

Core `workspaces.rs` defines typed IDs (positive decimal strings at IPC), records,
setup state, identity, observations, outcomes, and sanitized errors. Add the core
dependency to persistence for these shared records; no Tauri or SQL enters core.
Platform extends its existing adapter with root validation/checking. Application
extends its existing service; put the DB behind an internal mutex so platform
work never holds the old outer service mutex. Health keeps the same signature
and serialized fields, but reads the real schema version (2 after migration).

Proposed transport, all camelCase fields and snake_case tagged status values:

| Command                    | JS arguments      | Result                                                                                                              |
| -------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------- |
| `get_app_health`           | none              | Existing `AppHealth`, unchanged fields                                                                              |
| `list_workspaces`          | none              | `{ setupState, workspaces: WorkspaceRecord[], overlaps: WorkspaceOverlap[] }`                                       |
| `select_and_add_workspace` | none              | `{ status: 'cancelled' }`, `{ status: 'added', workspace, availability }`, or `{ status: 'duplicate', existingId }` |
| `remove_workspace`         | `{ id: string }`  | `{ removedId: string }`; absent ID is `workspace_not_found`                                                         |
| `set_workspace_enabled`    | `{ id, enabled }` | Committed `WorkspaceRecord`; setting same value is idempotent                                                       |
| `check_workspace`          | `{ id: string }`  | `{ id, availability: AvailabilityObservation }`                                                                     |
| `skip_workspace_setup`     | none              | `{ setupState }`; idempotent, cannot downgrade configured                                                           |

`WorkspaceRecord`: `id`, `selectedPath`, `displayName`, `enabled`, `createdAt`,
`updatedAt`. Timestamps are Unix milliseconds; ID is a string to avoid JS integer
precision loss. `WorkspaceOverlap`: `parentId`, `childId`. `AvailabilityObservation`:
`state`, `reason` (nullable unknown reason), `checkedAt` (nullable Unix ms).
No raw identity/canonical path, SQL, OS error, or arbitrary path argument crosses
IPC. Mutation success is only returned after transaction commit.

All workspace failures have `{ code, message, existingId? }`; codes are
`operation_in_progress`, `invalid_selection`, `unsupported_path_encoding`,
`not_directory`, `folder_missing`, `access_denied`, `identity_unavailable`,
`location_changed`, `path_already_configured`, `validation_timeout`,
`workspace_not_found`, `persistence_unavailable`, and `application_unavailable`.
Messages are fixed English copy, not serialized underlying errors or SQL.
Availability failures for a known ID are observations, not command exceptions.
Health error shape/code remains as it is. The frontend bridge lives alongside,
not inside, the unchanged `src/lib/desktop.ts` contract.

## SQLite schema 2 (future implementation only)

Append `0002_workspaces.sql` to the current migration sequence; never edit 0001,
drop metadata, recreate a real DB, or change the app-data location/identifier.
Use a STRICT `workspaces` table:

- `id INTEGER PRIMARY KEY AUTOINCREMENT` (no reuse after remove).
- `selected_path TEXT NOT NULL UNIQUE`, `canonical_path TEXT NOT NULL UNIQUE`,
  both BINARY; `identity_key TEXT NOT NULL UNIQUE`.
- `enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1))`.
- `created_at INTEGER NOT NULL`, `updated_at INTEGER NOT NULL` (Unix ms).

Derive display name; do not duplicate it in SQL. Reuse `application_metadata`
key `workspace_setup_state_v1` with only `skipped` or `configured`; absence is
undecided. Invalid stored values are persistence errors, not a forced reset.
Insert + setup update is one transaction. Removal touches one workspace row
only; skip never clears rows. Migrations and failed mutations roll back fully.
Repository methods use bound parameters and typed identifiers, not SQL strings
from IPC. Read ordering is deterministic. Concurrency uniqueness is enforced
by SQLite even with two independent DB connections, not merely a UI lock.

Persist configuration paths/identity, enabled, timestamps, and setup decision
only. Availability, check timestamps, busy state, errors, current view, and
overlap relations are session observations/derived data, not saved history.
No source, file contents, project evidence, sizes, or inventories are stored.
App-owned DB writes are not user-folder writes. Existing sanitized error tests
must remain; never log folder paths automatically or publish native QA paths.

## Native integration and permission boundary

Official [dialog documentation](https://v2.tauri.app/plugin/dialog/) supports Rust
integration. Propose `tauri-plugin-dialog = "=2.4.0"` only in `src-tauri` for this
implementation, with its transitive `tauri-plugin-fs` locked at 2.4.2. The
[tagged plugin manifest](https://github.com/tauri-apps/plugins-workspace/blob/dialog-v2.4.0/plugins/dialog/Cargo.toml)
and [workspace manifest](https://github.com/tauri-apps/plugins-workspace/blob/dialog-v2.4.0/Cargo.toml)
declare Tauri `^2.8.2`, Rust 1.77.2, compatible with locked Tauri 2.11.5 and
observed Rust 1.98.1. This is manifest compatibility, not a tested build.
Latest dialog 2.8.3 requires Tauri 2.12: do not accept that stack upgrade.
Resolve/pin compatible transitives during implementation and inspect lock diff;
stop on an unavoidable existing-stack upgrade rather than hiding it.

Use callback `pick_folder`, `set_parent`, `set_title`, and
`set_can_create_directories(false)` from the
[2.4.0 Rust source](https://github.com/tauri-apps/plugins-workspace/blob/dialog-v2.4.0/plugins/dialog/src/lib.rs).
Bridge callback completion with a oneshot channel, keeping the event loop free.
Add direct Tokio `=1.53.1` with `sync,time` in the Tauri crate (already locked
transitively), and macOS-only libc `=0.2.189` in platform (also already locked)
for metadata bindings. No JS dialog/fs package, UI library, UUID package, Swift,
shell plugin, watcher, or new production framework is needed. Tempfile is reused
as a test-only dependency for platform/application fixtures.

Do not initialize the transitive FS plugin or grant any `fs:*`, `shell:*`,
`dialog:default`, `dialog:allow-open`, save/message, or persisted-scope permission
to JavaScript. The Rust dialog call needs no frontend dialog permission. A
compiled transitive crate is not an authorized IPC surface. Verify this with
actual invocation-denial tests, not just a dependency search.

Tauri's [capabilities documentation](https://v2.tauri.app/security/capabilities/)
states that registered application commands are allowed by default unless
included in `AppManifest::commands`. In `src-tauri/build.rs`, declare all seven
commands above in that manifest. Grant only their generated `allow-*` permissions
plus existing `core:default` to the existing main capability, with `local: true`,
`windows: ["main"]`, `webviews: ["main"]`, and no remote URL grants. Retain the
explicit capability selection in `tauri.conf.json`. Verify generated permission
names against the installed tauri-build 2.6.3, not a guessed prefix.

Every command also checks the injected caller's main window/webview label (not
a caller-supplied label). ACL/runtime authority must reject remote-origin and
ungranted callers before dialog or service work. Tests must exercise the ACL,
not only a mocked guard. Keep existing `get_app_health` successful response and
error contract; changing its caller authorization does not add product data.

This is a non-App-Sandbox application using the user's current OS permissions.
Selection grants no Full Disk Access, permission bypass, descendant access
guarantee, cleanup consent, or persistent security-scoped bookmark. Access may
fail on a later run. No remote IPC, generic read/write filesystem commands,
scope persistence, privilege helper, entitlements expansion, or OS preference
changes. macOS metadata bindings remain behind platform; Windows remains future.

## Accessibility, verification and acceptance

Reuse semantic tokens, compact controls, system typography, reduced-motion and
contrast handling. No redesign, cards, fabricated storage values, theme selector,
or Settings screen. Follow [design-system.md](../../design-system.md). Native
titlebar, identifier `io.github.silviobig96.devstoragemanager`, and Overview's
health/stale-effect protections remain. A workspace is not labelled Safe/Review/
Protected because configuration is not a storage finding.

The plan must test empty/cancel/persistence/reopen/multiple/concurrent duplicates,
path spelling/symlinks/Unicode/long paths/case-sensitive identity, nested roots,
missing/denied/reappearing/replaced roots, slow checks, SQLite rollback and errors,
enabled, skip persistence, remove-last behavior, unchanged folder contents,
navigation/focus/loading/errors and late-result guards. Use controlled temporary
fixtures, injected platform failures, and barriers; never contributor folders.

Future implementation must record automated, rendered-browser, native-Tauri,
and human checks separately. Mock picker success is not native-dialog QA. Check
both views in light/dark at 760 × 520, 1100 × 720, 1600 × 900 and maximized, plus
200% zoom, full long paths, keyboard navigation, dialog focus/Escape, actual
native picker cancellation/add, restart persistence, contrast/reduced motion and
screen-reader reading order. Preserve the measured full-width Overview behavior.
Missing native evidence stays pending; do not retrofit Phase 01.5 QA results.
Both functional QA and explicit human visual approval are required to complete
the future UI phase. This documentation run claims neither gate for Phase 02.

## Exclusions

No project/Git detection, manifests, filesystem traversal, storage sizes,
node_modules analysis, providers, cleanup classification/planning/execution,
user-file deletion/move/write, storage history, monitor, notifications, networking,
telemetry, Full Disk Access, privileged helpers, Windows implementation, bundle
identifier change, or Phase 01.5 redesign. Root metadata and app-owned
configuration are the entire new data boundary. Historical Phase 01.5's sole
health-command restriction ends at its phase boundary, not at all future phases.
