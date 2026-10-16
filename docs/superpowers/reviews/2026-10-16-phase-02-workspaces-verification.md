# Phase 02 workspace verification

Date: 2026-10-16. Normal checkout: `feat/phase-02-workspaces`.
Historical implementation review through `23e4f4e`; comparison base: merged
Phase 01.5 `8c9fc88`, including wide-window correction `8173c67`. The acceptance
closure pass below tests product commit `2bb85b8` at checkout `6ccc839`.

**Implementation:** Tasks 1–8 implemented and reviewed, including the reviewed
frontend race/recovery and native dialog-focus fixes. Task 9's human acceptance
is recorded; native checks were open when that acceptance was granted.
**FUNCTIONAL QA: PASS** according to the documented historical product evidence;
no product suites or native checks were rerun for this acceptance decision.
**HUMAN VISUAL QA: PASS** — explicit human approval in this conversation.
**PHASE 02 ACCEPTANCE: ACCEPTED FOR DEVELOPMENT WITH OPEN NATIVE VERIFICATION.**
**RELEASE ACCEPTANCE: NOT GRANTED.** This is not full native verification or
readiness for distribution. Historical Phase 01.5 evidence is unchanged;
Phase 03 had not started at that acceptance decision.

Current follow-up, 2026-10-16: the three tracked native obligations below now
have new Phase 03 resolution evidence. This does not relabel historical Phase 02
test results or grant Phase 03 human visual/release acceptance.

## Human acceptance decision — 2026-10-16

The human explicitly approved both the running Workspaces presentation/experience
and proceeding with Phase 02 as a development base while retaining the native
verification gaps: “okey dale hagamos eso aprobado”. The current documentary task
explicitly identifies that response as approval of both decisions, not just of
VoiceOver or of a proposed design.

This is a limited, explicit amendment of Phase 02's original acceptance criterion,
not permission to skip verification in other phases. A PR may be prepared and
Phase 02 may be integrated for continued development **only after CI approval
and a merge decided/performed by the human**. This task publishes documentation
on the same feature branch; it does not open a PR, merge, release, change
production permissions or implement Phase 03. Phase 03 design/planning may
advance in a separate task; real scanner integration must satisfy the ACL gate.

The acceptance carries three open obligations, not three new PASS results:
[P02-NATIVE-LAYOUT](#p02-native-layout), [P02-NATIVE-ZOOM](#p02-native-zoom) and
[P02-NATIVE-ACL](#p02-native-acl). Their scope, historical evidence, procedure,
responsible follow-up and hard integration limits are recorded below. Earlier
AWAITING HUMAN REVIEW entries describe their dated checkpoints and do not
override the current human visual PASS. Existing measurements and evidence
labels remain unchanged. No universal compatibility or release acceptance is
implied; all safety and no-automatic-deletion rules continue to apply.

This documentary acceptance update starts from clean branch HEAD `9f1da86`.
Only the verification record, Phase 02 plan/spec and roadmap change; AGENTS.md
needs no exception because explicit human visual approval now exists. Document
review, local-link/heading checks, `npm run format:check` and `git diff --check`
pass. Product test/build results above remain identified with their historical
commits; no tests, native app, database or OS preference is changed in this task.

## Historical functional evidence

The initial consolidated gates ran against `5656296`. After native QA exposed a
dialog-focus defect, the affected frontend gates and native bundle were verified
again through `23e4f4e`, as recorded below. No Rust implementation changed after
the 48-test workspace run, so that suite was not redundantly repeated. Full
formatting passed at `765631d`; later focus fixes and this evidence update have
scoped formatting/diff checks. Earlier task totals are historical evidence,
not added to the final suite totals.

| Command                                                          | Current result                                    |
| ---------------------------------------------------------------- | ------------------------------------------------- |
| `npm run format:check`                                           | Exit 0; all matched files use Prettier style      |
| `npm run lint`                                                   | Exit 0; no ESLint diagnostics                     |
| `npm run test:run`                                               | Exit 0; 11 files, 107 tests passed                |
| `npm run build`                                                  | Exit 0; `tsc -b` and Vite, 46 modules transformed |
| `cargo fmt --all --check`                                        | Exit 0                                            |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0; no warnings                               |
| `cargo test --workspace --locked`                                | Exit 0; 48 unit tests passed                      |
| `npm run tauri build -- --debug`                                 | Exit 0; fresh macOS app and DMG built             |
| `git diff --check`                                               | Exit 0                                            |

Rust totals are application 6, core 4, Tauri library 12, persistence 19 and
platform 7. Binary and doctest targets contain zero tests. npm emits the
pre-existing user configuration warning about `python`; no host configuration
was changed. The final Tauri debug build at `23e4f4e` exited 0 outside the
sandbox, produced the app and DMG, reran the frontend before-build command
successfully (46 modules), and completed its Rust build in 4.41 seconds. The
initial `5656296` debug build had also passed (16.83 seconds). During the
intermediate `cfcdbb4` build, both the sandboxed build and verbose diagnostic
attempt exited 1 with `hdiutil: create failed - Device not configured`; retrying
the same build outside the sandbox succeeded. This recovered packaging
environment restriction is separate from the native focus defect below.

The initial full formatting gate exited 1 for one existing test-file chain in
`src/lib/workspaces.test.ts`. Prettier changed only that chain's line wrapping;
the focused `npm run test:run -- src/lib/workspaces.test.ts` then passed 1 file,
6 tests, and the full `npm run format:check` and `git diff --check` exited 0.
This was a formatting correction, not a behavioral regression or TDD RED. The
then-unchanged product implementation was covered by the consolidated historical
105/48 frontend/Rust results; no broad test rerun was needed for test whitespace.
The controller's final full formatting and diff checks at `765631d` also exited 0.

## Scope and boundary audit

The audit examined the changes from `8c9fc88` through `5656296`: shared domain
contracts, platform metadata implementation, SQLite migration/repositories,
application coordination, command/runtime registration, generated permissions,
frontend bridge/controller/components and their covering tests, manifests,
lockfile, specification, plan and branch-naming instruction refinement. The implementation comparison comprises
56 files, 9,193 insertions and 132 deletions, including the Phase 02 design/plan.
The verification/status documents are a separate documentation checkpoint.
Later commits `cfcdbb4` and `23e4f4e` contain only the focused removal-trigger and
dialog-lifecycle corrections with covering frontend tests; their scoped reviews
reported no findings. These changes do not expand the Rust, permission or
filesystem boundary audited below.

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

## Native application evidence

The controller launched `npm run tauri dev` from the normal checkout. Initial
attachment to the unbundled executable was unavailable and a locked macOS
session delayed fresh-bundle inspection. Those access limitations were later
resolved. The following observations use the actual freshly built application
and only explicitly chosen temporary fixture roots; they are separate from the
synthetic browser records above.

- Overview showed connected health/schema 2 and optional first-run setup.
  Opening and cancelling the native folder panel left setup/configuration
  unchanged. Selecting a Unicode fixture directory added one Present row and
  the no-scan announcement. Selecting it again kept one row and announced the
  duplicate.
- Disabling the row, explicitly quitting with Command-Q and reopening retained
  the row disabled with Not checked; Overview no longer showed setup. Enabling
  and manually checking produced Present. Explicitly renaming the controlled
  fixture away then checking produced Missing without changing enabled. After
  restoring it, another explicit check produced Present. This is a temporary
  rename fixture, not a physical-volume disconnect or automatic search.
- The HTML removal confirmation showed Cancel initially focused and both exact
  reassurance sentences. Escape preserved the row. Confirmed removal produced
  a valid empty view, focused Add, and no Skip/setup reset. A separate long
  Unicode fixture added through the native panel retained its full path in the
  accessibility tree and persisted as Not checked after explicit quit/reopen.
- Both views were inspected in the maximized dark native window. Native raster
  captures do not establish CSS viewport dimensions or the exact size matrix;
  all such captures remain outside Git. No OS appearance preference was changed.
- After the final focus retest at `23e4f4e`, confirmed removal of the remaining
  long-path configuration again produced the valid empty view with Add focused
  and no Skip. Only the two QA configuration rows were removed through the app.
  Both fixture roots and the child directory remained intact; the sentinel
  digest matched its initial value. Fixture files were left intact.

These flows exercise actual local main-window health and workspace IPC. They do
not substitute for adversarial native origin/webview-denial checks or actual
screen-reader use merely because accessibility-tree inspection was available.

### Native focus regression and verified correction

Native WKWebView QA exposed Cancel/Escape returning focus to a previously
focused row rather than the actual Remove button. Commit `cfcdbb4` captured
the triggering button directly instead of reading `document.activeElement`.
Its 107-test frontend suite and scoped independent review passed, but the fresh
native retest still reproduced the defect; that first correction alone did
not fix native focus return.

The regression fixture was then strengthened to model `dialog.close()` restoring
its pre-dialog focus. It failed with 2 failed/21 passed and exposed passive
effect cleanup closing the dialog after the parent's layout focus restoration.
Commit `23e4f4e` moved dialog lifecycle cleanup to a layout effect, closing it
before the parent restores the actual trigger. Focused view/dialog tests passed
25/25, the full frontend suite passed 107 tests in 11 files, and lint, build,
scoped formatting/diff checks and independent scoped review passed. After a
fresh final bundle build and explicit quit/reopen, native Escape and Cancel
each returned accessibility focus to the actual Remove button. The visible
focus ring was captured privately. No product change followed that native pass.

An earlier native Tab sequence reached Cancel, Remove, HTML content, dialog
container and Cancel again. This observation does not establish a complete
focus-trap acceptance pass.

## Acceptance closure pass — 2026-10-16

Historical native evidence from before the human acceptance decision above.
Its human-approval row records that earlier checkpoint, not the current status.

Started with a clean normal checkout at `6ccc839b0dd3770132a48e1dfe53a2d95eef88a9`,
on the expected feature branch and unchanged origin. This includes the three
local feedback-refinement commits after `8827ab9`; none was discarded. Product
files are identical to `2bb85b88e3d06a20859be314cf4a15f87927b8fa`.
Its [feedback verification](2026-10-16-workspace-feedback-verification.md) records
113 frontend / 48 Rust tests and the successful final native bundle build.
Those results are historical, not suites rerun by this documentation-only pass.

The tested artifact is this checkout's existing
`target/debug/bundle/macos/dev-storage-manager.app`, executable SHA-256
`5a5fb4f49f3bfd2d0fbc962958406dd0dc0738698391a16bdd2eb19856000758`.
Its bundle identifier was checked with `plutil` and remains
`io.github.silviobig96.devstoragemanager`. No rebuild, product change,
dependency, permission, test mode or diagnostic command was introduced.
All new application evidence below is actual macOS/WKWebView, not browser
emulation or MockRuntime. No new browser checks were required.

| Check                             | Result                                          | Actual environment / evidence                                                                                                                                                                                                                                                                                                                                   |
| --------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rapid picker activation           | PASS                                            | Triple-click Add showed one native sheet. Cancel returned to the unchanged empty list; double-click Add reopened one sheet normally. Only the controlled fixture was subsequently selected.                                                                                                                                                                     |
| Complete modal keyboard cycle     | PASS for exercised states                       | Six forward and six reverse steps in the initial window, then a complete forward/reverse cycle at 760 × 640 CSS. Cancel → Remove → WK document → dialog container → Cancel; reverse traversed the same set. No background action received keyboard focus.                                                                                                       |
| Modal background and cancellation | PASS                                            | Clicking the dimmed sidebar did not navigate; the modal remained. Escape and explicit Cancel both preserved the row and restored its actual Remove invoker. Narrow-window Enter opened the same dialog from the keyboard.                                                                                                                                       |
| Long-path actions, ordinary zoom  | PASS at measured size                           | At 760 × 640 CSS, full Unicode path wrapped; Check was invoked, Tab/Enter reached Remove, Shift+Tab reached Check and Enabled, and Space toggled Enabled. The value was restored. This is not 200% evidence.                                                                                                                                                    |
| Maximized appearance/layout       | PASS                                            | Both native OS light and dark; both views at measured 1728 × 969 CSS. Expected/actual structural width 1456 px, section widths 1456 px, no main/document horizontal overflow. Overview empty centering delta 0 px.                                                                                                                                              |
| Smaller native appearance/layout  | Partial                                         | Dark, both views at human-resized 760 × 640 CSS: expected/actual structural/section width 544 px, no main/document horizontal overflow, Overview centering delta 0 px. Initial dark viewport was 1100 × 688 CSS, not 1100 × 720.                                                                                                                                |
| Actual screen reader              | PASS, human-reported scope                      | VoiceOver was enabled with explicit permission. Asked to check list → name → full path → state → Enabled → Check → Remove, the human replied that it works and requested VoiceOver off. The agent cannot hear/transcribe speech; AX inspection alone is not the evidence for this result. This does not approve the whole UI or every announcement/fault state. |
| Increased contrast                | PASS for inspected dark view                    | OS setting changed from off to on with permission; WK `prefers-contrast: more` became true. Actual controls, separators, text and focus were inspected/captured. Restored off.                                                                                                                                                                                  |
| Reduced motion                    | PASS for exercised navigation                   | OS setting changed from off to on with permission; WK `prefers-reduced-motion: reduce` became true, active animation count was 0, and navigation to Overview worked. Restored off.                                                                                                                                                                              |
| Unauthorized caller/origin        | Still pending; partial native boundary evidence | Actual main `get_app_health` returned schema 2. `plugin:webview\|create_webview` was denied for lack of `core:webview:allow-create-webview`. No second WKWebView was created, so this is not an invocation from an unauthorized caller/origin.                                                                                                                  |
| Actual 200% zoom                  | Pending                                         | Native `plugin:webview\|set_webview_zoom` with value 2 was denied for lack of `core:webview:allow-set-webview-zoom`. Existing View menu has no page zoom control. No CSS zoom, transform, browser substitute or new grant was used.                                                                                                                             |
| Human visual approval             | AWAITING HUMAN REVIEW                           | VoiceOver confirmation and authorization to change preferences do not grant general visual approval.                                                                                                                                                                                                                                                            |

Native Web Inspector was operated through its existing UI, evaluating read-only
DOM geometry/media queries and the specific IPC smoke calls above. Dimensions
come from `innerWidth`/`innerHeight`, not capture pixels. Main interior is
`clientWidth - paddingLeft - paddingRight`; view/section rectangles were compared
against it. The measured maximized viewport is not relabelled 1600 × 900.
Attempts to drag the window edge did not change its geometry; `window.resizeTo`
also left it unchanged. Rather than continue retrying, the human reduced it;
that produced 760 × 640, not the exact planned small viewport.

The human separately authorized temporary native light appearance. Original
appearance was Dark, not Automatic; it was restored immediately when requested.
Final WK queries returned `dark: true`, `contrast: false`, `motion: false`;
System Settings also confirmed VoiceOver off. No other preference was changed.

The only new configured root was a controlled temporary Unicode/long-path
fixture. Only its configuration record was removed through the native dialog;
the initial zero-workspace state returned with Add focused. Its directory and
sentinel remain intact and the sentinel SHA-256 before/after matches. No personal
workspace was selected or modified, and no real database was reset.
Private native captures remain outside Git; no capture or local inventory is
included in this commit. No product defect was established or corrected in this
pass. Earlier focus defects/fixes above remain historical evidence.

Closure-pass command evidence: `git rev-parse --show-toplevel`,
`git status --short --branch`, `git remote -v`, `git log -5`, and
`git diff --quiet 2bb85b8..HEAD -- src crates src-tauri Cargo.toml Cargo.lock package.json package-lock.json`
confirmed the expected checkout and unchanged tested product. `plutil -extract
CFBundleIdentifier raw` checked the bundle's Info.plist; `shasum -a 256` identified
the executable and matched the fixture sentinel before/after configuration
removal. Scoped `npx prettier --write` formatted only this record and the plan;
`npm run format:check` and `git diff --check` exited 0. The new feedback-record
link resolves. Final diff inspection includes only these two documentation
files; no product suites were rerun solely for updating evidence.

## Remaining acceptance checks

At the Phase 02 acceptance decision all three checks were **OPEN — DEFERRED WITH
HUMAN APPROVAL**; that decision itself executed or passed none. The current
statuses below additionally reference new Phase 03 native evidence, without
relabeling any historical observation. General Phase 02 visual approval remains PASS.

### P02-NATIVE-LAYOUT

- **Status:** RESOLVED by new Phase 03 Task 11 native renderer evidence, 2026-10-16.
- **New follow-up:** Actual WK measurements now cover both views, empty and
  populated Workspaces and current result controls at all six exact size/theme
  pairs, including separators, empty centering and no horizontal overflow.
  Native maximize and pageZoom getters are separately recorded in the
  [new receipt](2026-10-16-phase-03-scanner-core-verification.md#task-11--consolidated-and-current-native-verification).
  This closes the geometric obligation, not new Phase 03 keyboard or visual QA.
- **Exact scope / missing check:** Overview and Workspaces in native macOS
  light/dark at each exact CSS viewport: 760 × 520, 1100 × 720 and 1600 × 900.
  Verify structural/section/separator width, centered empty states and no shell
  horizontal overflow using renderer measurements, not raster capture sizes.
- **Existing evidence:** Browser matrix at those exact sizes is browser-only.
  Native both-view maximized light/dark is 1728 × 969 CSS with 1456 px interior;
  native smaller dark is 760 × 640 CSS with 544 px interior. Both measured cases
  have no horizontal overflow and Overview centering delta 0 px. Initial native
  dark 1100 × 688 is not the planned 1100 × 720. No case is relabelled.
- **Procedure:** [Existing follow-up, step 1](#reproducible-follow-up-for-the-unresolved-checks),
  including human resizing and authorized appearance changes/restoration.
- **Responsible follow-up:** The next UI-modifying phase/assignment must include
  this Phase 02 verification debt in its acceptance work and record real native
  evidence here. This documentation task does not perform it.
- **Acceptance limit:** Resolve before accepting the next phase that modifies
  user-facing UI. Development acceptance of Phase 02 does not waive that gate.

### P02-NATIVE-ZOOM

- **Status:** RESOLVED by new Phase 03 Task 1 native evidence, 2026-10-16.
- **New evidence:** Actual WK pageZoom 2.0/restored 1.0 in the exact-size matrix;
  human confirmed the long-path action/keyboard/scroll/dialog check with
  “esta perfecto” in the held 200% fixture. This is not new Phase 03 visual
  approval; its added controls still need their own checks. See the
  [native receipt](2026-10-16-phase-03-scanner-core-verification.md#checkpoint-1--native-acceptance-target).
- **Exact scope / missing check:** Real 200% WKWebView page zoom, all long-path
  actions reachable and operable, scrolling, and dialog content/actions.
- **Historical evidence at Phase 02 acceptance:** Native long-path keyboard actions and modal cycles
  worked at ordinary zoom, including 760 × 640 CSS. Native page-zoom invocation
  with value 2 was denied by the intact production policy; no real 200% result
  exists. Browser results, CSS scaling and OS magnification are not substitutes.
- **Procedure:** [Existing follow-up, step 2](#reproducible-follow-up-for-the-unresolved-checks),
  requiring bounded authorized native debugger/control access, verified pageZoom
  2.0 and restoration to 1.0 without adding production capabilities.
- **Responsible follow-up:** The next UI-modifying phase/assignment must arrange
  the bounded native verification and record it here; not this documentary task.
- **Acceptance limit:** Resolve before accepting the next phase that modifies
  user-facing UI; no inference of PASS from the new human visual approval.

### P02-NATIVE-ACL

- **Status:** RESOLVED for the seven existing commands by new Phase 03 Task 1
  native evidence, 2026-10-16. New scanner commands retain a separate native
  denial gate before real engine integration.
- **New evidence:** Real child/other-window/external-main calls rejected all
  seven before handler entry with unchanged fixture DB; wrong-parent calls
  failed the guard; authorized-main controls passed, capabilities unchanged.
  Missing bridge/timeout are distinct failures. See the
  [native receipt](2026-10-16-phase-03-scanner-core-verification.md#checkpoint-1--native-acceptance-target).
- **Exact scope / missing check:** In real WKWebView, reject unauthorized
  webview/window callers and remote origins for the seven registered commands,
  before handler/picker/database work, with production policy unchanged.
- **Historical evidence at Phase 02 acceptance:** Generated-context MockRuntime ACL/dispatch tests cover
  denied labels/origins, containing-window guards and unavailable generic APIs.
  They remain automated evidence, not native evidence. Actual main health works;
  native creation of a second webview was denied. Denial of its creation does
  not demonstrate invocation rejection from that caller/origin.
- **Procedure:** [Existing follow-up, step 3](#reproducible-follow-up-for-the-unresolved-checks),
  a bounded native test/debug environment with isolated temporary data and
  independently created native callers. Do not grant generic capabilities,
  remote grants or a distributed diagnostic/test mode to production.
- **Responsible follow-up:** Phase 03's scanner integration assignment must
  include and resolve this dependency before integrating the real scanner.
  Phase 03 design/planning may advance separately; neither is execution here.
- **Integration limit:** Resolve before real Phase 03 scanner integration. That
  integration cannot assume PASS or postpone this obligation until cleanup.

### Reproducible follow-up for the unresolved checks

1. In this same debug bundle, use a controlled temporary long-path fixture,
   never a personal repository. Manually resize the window; open existing Web
   Inspector with Option-Command-I and evaluate
   `JSON.stringify({width:innerWidth,height:innerHeight})`. Adjust until each
   exact CSS size above is reached; native titlebar height is outside those
   measurements (observed 32 px). Check both views, full-width sections,
   separators, centered empty states and horizontal overflow. Repeat native
   light only with permission, restoring Dark afterward. Do not infer any
   size from a Retina capture. The current UI automation cannot reliably drag
   the border; human resizing is required for this remaining matrix.
2. Real 200% requires an existing native page-zoom/debugger control that can set
   WKWebView `pageZoom` to 2 without changing production grants. None is exposed
   by the available app/tool controls. A developer with an authorized local
   native debugger can set `pageZoom = 2.0` on the main WKWebView on its main
   thread, verify that value, then Tab/Shift+Tab and activate every long-path
   action, scroll all content/dialog copy, and restore `pageZoom = 1.0`. Do not
   use CSS scaling, an OS screen magnifier or inspector-UI zoom as a substitute.
   Debugger attachment and this procedure were not performed here; without
   that controlled access this acceptance item stays blocked, not passed.
3. Adversarial native IPC needs an independently labelled WKWebView in `main`,
   another window, and a remote-origin WK context using the unchanged generated
   authority. The current app cannot create them through JS, as the native
   denial proves. A developer-controlled local native test/debug session must
   create those callers from Rust/native context, without adding JS permissions
   or distributing a test mode. From each, invoke the seven registered commands
   with valid-shaped fixture-only arguments; require ACL rejection before any
   handler/picker/database work. Also check a main-labelled webview in a wrong
   containing window fails its explicit guard. Use an isolated temporary app
   database, verify zero mutation/no picker, and restore/close test contexts.
   The present tools expose no such native caller-creation/debugger interface;
   no harness was built to bypass this limitation. MockRuntime results remain
   separate. This procedure needs that bounded native integration environment,
   not new production capabilities or a broad automation infrastructure.
4. The original follow-up also requested general human visual approval, because
   the earlier VoiceOver response approved only reading order. That request is
   now satisfied by the separate [human acceptance decision](#human-acceptance-decision--2026-10-16).
   The three native checks above stay open under its specific development-only
   exception; Task 9's unperformed verification boxes stay unchecked.

Automated tests cover loading, empty, retryable error, failed SQLite writes with
no false success, missing/denied/reappeared root mappings and health regression.
They are not claims that every fault was reproduced in the running native app.
Phase 02 is accepted for development, not fully natively verified or accepted
for release. Publication alone is not approval; the explicit human decision
above permits a PR handoff and integration subject to CI and human merge.
It does not execute Phase 03 or relax the three future acceptance/integration gates.

## Historical review and publication receipt

Final independent whole-branch review at `765631d` reported no findings. The
implementation was ready for its authorized feature-branch publication while
native and human acceptance remained pending. Later native-focus corrections
received clean scoped reviews; remaining native and human gates stay open.

The controller published `feat/phase-02-workspaces` with the normal
`git push -u origin feat/phase-02-workspaces` command, exit 0. At that checkpoint,
`git ls-remote --heads origin feat/phase-02-workspaces` exactly matched local HEAD:
`765631dbebdbab8786adbdb8d7a54a9802363af9`. This historical receipt establishes
publication of that commit only; subsequent documentation commits require their
own push and remote verification. No force push, PR, merge or release occurred.
