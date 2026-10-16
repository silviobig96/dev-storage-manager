# Phase 02 workspace verification

Date: 2026-10-16. Normal checkout: `feat/phase-02-workspaces`.
Implementation reviewed through `5656296`; comparison base: merged Phase 01.5
`8c9fc88`, including wide-window correction `8173c67`.

**Implementation:** Tasks 1–8 implemented and reviewed, including the reviewed
frontend race/recovery fixes. Task 9 remains open for outstanding checks below.
**FUNCTIONAL QA: PASS.** Consolidated commands pass, including the fresh debug
application bundle and final formatting correction described below. Native and
accessibility coverage is partial and is reported separately.
**VISUAL QA: AWAITING HUMAN REVIEW.** Phase 02 is not complete. Browser rendering,
automated tests and agent inspection do not constitute human visual approval.
Historical Phase 01.5 evidence is unchanged; Phase 03 has not been executed.

## Current functional evidence

The controller ran the product gates against the integrated implementation at
`5656296`; final documentation/formatting checks and the focused test-formatting
correction are recorded below. Focused behavioral RED/GREEN cycles were recorded
during each owning task before this consolidated run; their earlier totals are
not counted as the final suite result.

| Command                                                          | Current result                                    |
| ---------------------------------------------------------------- | ------------------------------------------------- |
| `npm run format:check`                                           | Exit 0; all matched files use Prettier style      |
| `npm run lint`                                                   | Exit 0; no ESLint diagnostics                     |
| `npm run test:run`                                               | Exit 0; 11 files, 105 tests passed                |
| `npm run build`                                                  | Exit 0; `tsc -b` and Vite, 46 modules transformed |
| `cargo fmt --all --check`                                        | Exit 0                                            |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0; no warnings                               |
| `cargo test --workspace --locked`                                | Exit 0; 48 unit tests passed                      |
| `npm run tauri build -- --debug`                                 | Exit 0; fresh macOS app and DMG built             |
| `git diff --check`                                               | Exit 0                                            |

Rust totals are application 6, core 4, Tauri library 12, persistence 19 and
platform 7. Binary and doctest targets contain zero tests. npm emits the
pre-existing user configuration warning about `python`; no host configuration
was changed. The Tauri build also reran its frontend before-build command
successfully (46 modules) and completed the normal Rust application build in
16.83 seconds. No product test or compiler failure remains in the reported gates.

The initial full formatting gate exited 1 for one existing test-file chain in
`src/lib/workspaces.test.ts`. Prettier changed only that chain's line wrapping;
the focused `npm run test:run -- src/lib/workspaces.test.ts` then passed 1 file,
6 tests, and the full `npm run format:check` and `git diff --check` exited 0.
This was a formatting correction, not a behavioral regression or TDD RED. The
unchanged product implementation remains covered by the consolidated 105/48
frontend/Rust results above; no broad test rerun was needed for test whitespace.
The controller's final full formatting and diff checks at `765631d` also exited 0.

## Scope and boundary audit

The audit examined the changes from `8c9fc88` through `5656296`: shared domain
contracts, platform metadata implementation, SQLite migration/repositories,
application coordination, command/runtime registration, generated permissions,
frontend bridge/controller/components and their covering tests, manifests,
lockfile, specification, plan and branch-naming instruction refinement. The implementation comparison comprises
56 files, 9,193 insertions and 132 deletions, including the Phase 02 design/plan.
The verification/status documents are a separate documentation checkpoint.

- The native Rust picker is the only production source of a new root. Its
  no-argument JS command uses one folder, the actual containing window, the title
  Add workspace, no default directory and disabled directory creation. IDs and
  enabled booleans are typed inputs to the other dedicated commands. Extra JS
  path/label fields cannot bypass root validation or choose a caller identity.
- The seven registered commands are `get_app_health`, `list_workspaces`,
  `select_and_add_workspace`, `remove_workspace`, `set_workspace_enabled`,
  `check_workspace` and `skip_workspace_setup`. All seven are declared through
  `AppManifest::commands` and have reviewed generated allow/deny definitions.
  The selected capability grants `core:default` and their seven allow entries,
  `local: true`, `webviews: ["main"]`, no `windows` alternative and no remote
  URLs. Every handler additionally checks the injected webview's actual label
  and actual containing window label.
- The dialog plugin is initialized in Rust. Its transitive filesystem crate is
  not initialized. There are no generic JS dialog, filesystem, shell or
  persisted-scope grants. Generated-context invocation tests exercise Tauri's
  authority and registered handler with MockRuntime: all seven commands reject
  another webview inside main, another window and a remote origin before the
  handler runs. A main-labelled webview in the wrong containing window fails
  the explicit guard. Generic dialog open/save and FS/shell invocations fail.
  These are automated ACL/dispatch tests, not native WKWebView denial evidence.
- Production root access uses `open(O_EVTONLY | O_CLOEXEC)`, descriptor ownership,
  separate root and volume `fgetattrlist` calls, and canonicalization. Requested
  attributes are type, file ID, birth time, volume UUID/capabilities and returned
  masks. It requests no permanent-ID allocation, sizes, directory counts,
  children, project manifests or file contents. Buffer lengths/masks are checked
  before bounds-checked byte decoding; native bindings stay inside macOS.
- Root validation observes selected/canonical/selected/canonical references and
  rejects observed identity/path changes. The opaque `macos-v1` identity combines
  supported nonzero volume UUID, persistent file ID and birth timestamp. Paths
  preserve UTF-8 spelling and case without normalization. Non-UTF-8 or unsupported
  identity fails explicitly. Birth time reduces reuse collisions; neither this
  observation nor selection grants future deletion authority. Filesystem and
  SQLite state are not atomic; a change after the final observation remains
  possible, and later checks compare the saved identity.
- The selector gate survives a dropped receiver while its panel or commit is
  active. Root work has two nonqueuing worker permits and a three-second
  deadline. A timed-out OS worker retains its permit until it ends; it cannot
  insert. Database commit is a separate untimed stage. Platform work does not
  hold the database mutex; health/list remain responsive in barrier tests.
- Migration `0001_application_metadata.sql` is byte-for-byte unchanged. Appended
  migration 0002 creates a STRICT table with AUTOINCREMENT IDs, BINARY unique
  selected/canonical paths and identity, and an enabled check constraint. Add
  and configured setup commit atomically in an immediate transaction. Inputs
  are bound; success follows commit; the busy timeout is one second. The only
  upsert updates app-owned setup metadata, never an existing workspace record.
- Real temporary SQLite tests preserve schema-1 sentinel metadata on upgrade
  to schema 2/reopen, roll back an invalid migration and failed mutations, and
  enforce uniqueness across independent concurrent connections. Invalid setup
  metadata remains an error. Skip persists without downgrading configured;
  remove-last preserves configured setup and re-add receives a new ID.
  Health reads schema 2 with unchanged serialized fields/error contract.
- Remove deletes only the chosen app-owned SQLite row. The application sentinel
  test retains the fixture directory and exact file bytes; removal never calls
  the platform. Availability is session-only, root-only and explicitly requested:
  listing, restart and navigation do not scan or automatically check roots.
  Enabled means eligibility for future analysis. Nested roots remain separate
  and show advisory component-based overlaps; no scanning/double-counting
  implementation is introduced.
- Frontend committed state is pessimistic. Failed or uncertain mutations do not
  claim rollback or success; lost responses require authoritative reconciliation
  before another mutation. Request/session/per-ID guards cover stale lists,
  add/check results, removed rows, setup state and newer navigation intent.
  Repeated failed reloads or concurrent checks cannot clear uncertainty. Dialog
  submission, cancellation, exact reassurance copy and focus fallbacks have
  automated coverage. The unchanged health bridge/effect guards remain separate.
- Only Overview and Workspaces are interactive destinations. Other destinations
  remain visibly unavailable. Full paths wrap and remain selectable with
  row-specific accessible action names. Feature CSS reuses semantic tokens and
  existing primitives. Overview's `inline-size: 100%` CSS is unchanged; its
  optional setup slot follows the header and precedes Storage overview.
- No project discovery, traversal, sizes, provider, scanner, safety-category
  claims, cleanup, user-folder mutation, monitor, notification, networking,
  telemetry, Full Disk Access, helper, Swift or Windows implementation was added.
  No personal paths, databases, screenshots or filesystem inventories are part
  of the tracked change. Fixture creation/rename/permission changes occur only
  in tests or private controlled QA setup, not application operations.

## Dependency and configuration audit

The base lockfile contains 448 package/version pairs; the implementation contains 531. All existing pairs remain, with 83 additional pairs from the reviewed
dialog/test dependency closure. No existing package version was removed or
upgraded. Existing JS manifests/lockfile and toolchain configuration are unchanged.

Production additions are exact `tauri-plugin-dialog =2.4.0`, direct Tokio
`=1.53.1` with `sync,time`, and macOS-only libc `=0.2.189`. The transitive
`tauri-plugin-fs` remains locked at 2.4.2. Tempfile and Tauri `test,unstable` are
development-only fixture support. The production normal/build feature tree and
reverse FS feature tree were inspected with locked offline `cargo tree`: FS has
only its default feature; no watcher or Tauri test/unstable feature appears in
the production graph. Cross-target crates in Cargo.lock do not add a Windows
application adapter. Tauri 2.11.5, tauri-build 2.6.3, utils 2.9.3, runtime 2.11.3
and runtime-wry 2.11.4 are retained.

`src-tauri/tauri.conf.json`, including identifier
`io.github.silviobig96.devstoragemanager`, is unchanged; `plutil` confirmed that
exact identifier in the freshly built bundle. App-data resolution and
the `dev-storage-manager.db` filename are unchanged. No entitlement or generic
frontend filesystem authority was added.

## Filesystem evidence and limits

Real platform tests use temporary directories on the local macOS fixture
volume: Unicode/spaces and paths over 260 bytes, symlink/dot aliases, distinct
same-name directories, files/missing/dangling/loop targets, repeat identity,
root-only access despite an inaccessible child, and unchanged sentinel bytes.
The task's observed local volume was case-insensitive; the test conditionally
checks the actual fixture volume. Case-sensitive `Code`/`code` distinction is
also covered with injected metadata and is not native case-sensitive evidence.

Rename-away/restore/replacement fixtures prove missing, present and
identity-changed observations; they do not prove physical disk unmount/remount.
EACCES/EPERM, unsupported identity, symlink retargeting races and malformed
attribute buffers have deterministic injected coverage. Existing invalid-byte
directory names could not be created on the local volume; a constructed
non-UTF-8 input is rejected before I/O. Real native permission denial, network
volumes and every filesystem/mount type remain unproven. Successful metadata
never claims descendant readability or Full Disk Access.

## Rendered browser evidence

The controller rendered the production React/CSS through a separate private
fixture entrypoint with synthetic Tauri IPC records. The fixture is outside the
product entrypoint and is not committed or shipped. CSS viewport dimensions
were measured inside the renderer, independently of screenshot raster size.
Light/dark uses a frame's color-scheme emulation with the production media-query
tokens; global macOS preferences were not changed. These are browser results,
not native OS appearance, picker, SQLite persistence or WKWebView evidence.

| Actual CSS viewport | Appearances / views              | Main interior and structural/separator width | Width delta | Horizontal overflow      |
| ------------------- | -------------------------------- | -------------------------------------------- | ----------- | ------------------------ |
| 760 × 520           | Light/dark × Overview/Workspaces | 544 px                                       | 0 px        | None in document or main |
| 1100 × 720          | Light/dark × Overview/Workspaces | 828 px                                       | 0 px        | None in document or main |
| 1600 × 900          | Light/dark × Overview/Workspaces | 1,328 px                                     | 0 px        | None in document or main |

All 12 measured cases satisfy the one-CSS-pixel structural-width tolerance;
Overview empty-state centering delta is 0 px. Six additional empty Workspaces
cases across the same sizes/appearances have width and centering deltas 0 px,
the same separator widths and no horizontal overflow. Browser captures remain private.
The configured/skipped Overview layout retains its previous width behavior.
At 1100 × 720, moving optional first-run setup directly below the header places
Add/Skip at y=236.38–265.23, fully inside the viewport; their previous
y=695.70–724.55 placement clipped the controls. Browser simulated picker cancel
restores Add focus; simulated Skip removes setup and yields a valid empty
Workspaces view with Add only, full width and centered empty state.

At 760 px light, the browser HTML confirmation opened with Cancel focused;
Tab reached Remove. Escape closed it without removing the row and returned
focus to its Remove invoker. A complete focus-trap cycle was not established.
These observations supplement jsdom tests, whose test-only dialog polyfill
cannot prove browser top-layer or native picker behavior.

Additional browser checks at 1100 × 720 used synthetic IPC after `765631d`:
light-mode list failure displayed “Couldn't load workspaces.” and Retry; a
failed Retry retained that error without false success. Dark-mode loading
displayed “Loading workspaces…” without premature mutation controls. In light
mode, Remove opened the HTML dialog with Cancel focused and both reassurance
sentences. A simulated save failure displayed “Couldn't save workspace changes.
Try again.” and retained the dialog; Cancel retained both rows and restored the
Remove invoker's focus. These browser states do not prove native SQLite failure
or native picker behavior; captures remain outside Git.

## Native and remaining acceptance checks

The controller launched `npm run tauri dev` from the normal checkout. It compiled
and started the checkout's debug binary. Computer-use attachment to that
unbundled executable was unavailable; launch alone establishes no picker, health
screen or native flow coverage. Attachment to the exact freshly built bundle
awaits an unlocked macOS session. Native UI checks remain pending; this access
limitation is not a failed product test. No native flow result is claimed.

The following remain open and cannot be inferred from automated/browser results:

- Native folder-only picker selection/cancellation, rapid activation, focus
  return, successful add, restart persistence, enabled/manual-check/remove flows
  and unchanged sentinel contents through that native workflow.
- Actual WKWebView main allow/other-origin ACL denial smoke checks.
- Maximized layout, 200% zoom with all long-path row actions reachable, native
  light/dark appearance, complete dialog focus trapping, and actual screen-reader
  list/path reading order.
- Increased contrast and reduced motion inspection. No global OS preference was
  changed to perform these checks.
- Explicit human inspection/approval of the running Phase 02 UI.

Automated tests cover loading, empty, retryable error, failed SQLite writes with
no false success, missing/denied/reappeared root mappings and health regression.
They are not claims that every fault was reproduced in the running native app.
The phase remains incomplete while these checks and human visual acceptance
are outstanding. Feature-branch publication does not establish visual acceptance
or authorize Phase 03, a PR, merge or release.

## Review and publication receipt

Final independent whole-branch review at `765631d` reported no findings. The
implementation was ready for its authorized feature-branch publication while
native and human acceptance remained pending.

The controller published `feat/phase-02-workspaces` with the normal
`git push -u origin feat/phase-02-workspaces` command, exit 0. At that checkpoint,
`git ls-remote --heads origin feat/phase-02-workspaces` exactly matched local HEAD:
`765631dbebdbab8786adbdb8d7a54a9802363af9`. This historical receipt establishes
publication of that commit only; subsequent documentation commits require their
own push and remote verification. No force push, PR, merge or release occurred.
