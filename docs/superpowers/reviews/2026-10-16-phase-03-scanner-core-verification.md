# Phase 03 scanner core verification

Date: 2026-10-16. Normal checkout: `feat/phase-03-scanner-core`.
Implementation starts from reviewed design commit
`7ad8431880ff8be4a635523b4ca8d65bf5280f57`, with merged product base
`04018df8d152268cebd2f62aa554ddfa49c30d7a`.

## Current state

Implementation in progress. Checkpoint 1 code and actual native receipts passed
independent task review. This is not a whole-phase functional PASS.
HUMAN VISUAL QA: AWAITING HUMAN REVIEW for the new Phase 03 UI.
Phase 02 visual approval remains granted and is not reopened.

The implementation assignment accepts the design decisions and explicitly
authorizes the isolated Cargo test target, pinned dev dependency edges and
controlled 127.0.0.1:0 origin fixture. No broader production capability, real
workspace scan, release or Phase 04 feature is authorized.

## Checkpoints

| Checkpoint                           | Tasks | Status                                |
| ------------------------------------ | ----- | ------------------------------------- |
| Isolated native verification         | 1     | Implemented; reviewed receipts below  |
| Contracts, policy and fixture engine | 2–5   | Tasks 2–5 implemented and reviewed    |
| Gated application/IPC/minimal UI     | 6–10  | Tasks 6–8 reviewed; 9–10 not executed |
| Functional/native/human acceptance   | 11    | Not executed                          |

## Inherited native obligations

- [P02-NATIVE-ACL](2026-10-16-phase-02-workspaces-verification.md#p02-native-acl):
  RESOLVED for the seven existing commands by Checkpoint 1 native evidence.
  The Task 7 eleven-command native denial matrix also passed before
  MetadataScanRunner integration; its actual receipt is recorded below.
- [P02-NATIVE-LAYOUT](2026-10-16-phase-02-workspaces-verification.md#p02-native-layout):
  OPEN with new exact populated-view matrix evidence below. Empty Workspaces
  and current Phase 03 controls must still be checked before UI acceptance.
- [P02-NATIVE-ZOOM](2026-10-16-phase-02-workspaces-verification.md#p02-native-zoom):
  RESOLVED for the inherited UI by actual pageZoom 2.0 and the specific human
  interaction confirmation below. New Phase 03 controls still require their own QA.

Historical browser, MockRuntime and normal-bundle evidence remains identified
in the Phase 02 record. It is not relabelled as new Phase 03 evidence.

## Review clarifications applied

- Differentiate traversal limits, inaccessible/changed folders, allocation-only
  uncertainty, cancellation and declared policy exclusions in result copy.
- Keep configuration 0..N; the 32-root cap applies only to a scan request. More
  than 32 enabled prevents UI start without selecting a silent subset.
- Account identity/ownership/metrics atomically on overflow and propagate partial
  covered-by coverage between overlapping roots.
- Resolve retained request replay before current busy/gate/epoch rejection.
- Deep remains bounded; elapsed budget does not guarantee syscall duration.
  Fixture timing cannot justify promises about personal repositories.

Only the original spec/plan were clarified; no parallel plan or roadmap rewrite.
Future evidence entries must identify exact commands, tested commits/artifact,
actual results and inconclusive/omitted checks. No personal paths, databases,
captures or inventories are recorded here.

## Checkpoint 1 — native acceptance target

Tested source: `de0e55b9a32a5a771d40bdad9e878a02884a2877`.
Independent task review: spec compliant, quality approved, no Critical/Important
findings. Existing npm user-config warning and hdiutil deprecation messages are
environmental noise; no global configuration was altered to suppress them.

The opt-in Cargo test executable uses production registration/context/commands,
temporary SQLite and twelve controlled long-path roots. No personal database
or configured workspace was opened. Same capabilities as production; their
SHA256 remains `46a60c52aa6eda33cb2c05c65c958c15451377094745b4d7b2efd6899805cafa`.
Only the two approved dev dependency edges were added; no locked version changed.
The loopback origin server binds 127.0.0.1:0, serves test HTML and stops/joins.

### Actual native authorization

```sh
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case inherited-acl
```

Exit 0. Actual Wry callers, not MockRuntime requests. Child in main, other window
and external origin in actual main each rejected all seven commands before the
application handler (zero case-scoped entries); fixture listing and SQLite bytes
stayed identical after every rejected call. Main in the wrong containing window
entered each handler and failed its actual-parent guard before service/picker.

| Command                  | Child      | Other window | External main | Wrong parent | Authorized main control   |
| ------------------------ | ---------- | ------------ | ------------- | ------------ | ------------------------- |
| get_app_health           | ACL denied | ACL denied   | ACL denied    | Guard denied | Schema 2                  |
| list_workspaces          | ACL denied | ACL denied   | ACL denied    | Guard denied | 12 fixture rows           |
| select_and_add_workspace | ACL denied | ACL denied   | ACL denied    | Guard denied | Picker not exercised here |
| remove_workspace         | ACL denied | ACL denied   | ACL denied    | Guard denied | Fixture row removed       |
| set_workspace_enabled    | ACL denied | ACL denied   | ACL denied    | Guard denied | Fixture off/on persisted  |
| check_workspace          | ACL denied | ACL denied   | ACL denied    | Guard denied | Fixture root observed     |
| skip_workspace_setup     | ACL denied | ACL denied   | ACL denied    | Guard denied | Setup persisted           |

Missing-bridge and unsettled-promise controls produced their distinct failure
types, never ACL PASS. Sandboxed macOS-services failure and pre-load eval callback
loss were diagnosed separately; the final run used native page-load completion
and authorized execution outside that sandbox. No production grants were changed.
There are no scanner commands/worker yet; the new four-command gate is not passed.

### Actual native geometry and zoom

The same Cargo command with `--case inherited-layout --hold` completed all
automatic assertions, then deliberately stayed open for interaction. Its optional
hold was interrupted after the human finished (exit 130, not a successful hold).
The exact already-built executable then ran:

```sh
target/native-acceptance/debug/deps/native_acceptance-229da906c690916f --case inherited-layout
```

Exit 0, complete final PASS JSON on unchanged source. Per-window native Light and
Dark each passed Overview and populated Workspaces at exact CSS 760 × 520,
1100 × 720 and 1600 × 900, with one measured titlebar correction per case.
Interior/view/section/separator widths were 544/828/1328 px respectively, deltas
0; maximum Overview empty-title/paragraph centering delta 0.0078125 px. No
main/document horizontal overflow. These are WK CSS measurements, not raster sizes.

Native WK pageZoom getter returned 2.0 and restored 1.0 in every matrix row.
At unchanged window size the zoomed CSS viewports were 380 × 260, 550 × 360 and
800 × 450, with zero horizontal overflow. No global appearance/zoom preference,
CSS scaling or extra JS permission was used. Automatic navigation was DOM
activation for geometry, not proof of native keyboard navigation.

For the held 800 × 450 CSS / native pageZoom 2.0 fixture, controller native AXPress
Check changed a long-path row to Available. A sent Tab exposed only WebArea to AX,
so the agent did not infer button focus. Asked specifically to verify Tab/Shift+Tab,
Enabled/Check/Remove, full paths, scrolling and dialog Cancel/Escape/focus return,
the human replied **“esta perfecto”** while that fixture was alive. This is
human-reported native zoom interaction evidence, not automated keyboard proof,
screen-reader testing or approval of the new Phase 03 UI. No screenshot was
captured from a non-frontmost window. Empty Workspaces/current new UI states
remain part of Task 11; P02-NATIVE-LAYOUT is conservatively kept open until then.

### Functional and packaging checks at this checkpoint

- `npm run build`: exit 0, includes `tsc -b`; Vite transformed 48 modules.
- `npm run test:run`: exit 0, 11 files / 113 tests.
- `cargo test --workspace --locked`: exit 0, 48 Rust tests; opt-in target omitted,
  not counted as native evidence. `npm run lint`, `cargo fmt --all -- --check`
  and `git diff --check` passed.
- `cargo metadata --no-deps --format-version 1 --locked` confirmed a test target,
  required feature and no new bin. `cargo tree -e normal,build -p dev-storage-manager --locked`
  confirmed dev-only edges; already-transitive WebKit is not harness inclusion.
- `cargo build -p dev-storage-manager --bin dev-storage-manager --all-features --locked`
  and `npm run tauri build -- --debug` passed. Normal app/DMG inspected, not launched.
  Read-only DMG mount/list/detach found only normal app/resources/installer metadata.
- Source inclusion review and app/DMG/binary inspection found no QA executable,
  HTML/report resource, QA marker or helper symbol in either all-feature main or
  bundled main. Positive control found all four QA strings in the native test
  executable, so absence was not the only exclusion check. No debug_assertions
  backdoor or additional application command was added.

SHA256 native test executable:
`047ae730623f61a575b09a7bda4121ec28cd38929961526fbe8ea844b9c1a3fe`.
SHA256 normal bundled executable:
`12a75ce66faa406e099b038dff9ac054aeeea848302b73f527f0eb6e75a8f99c`.
Normal bundle fixture-only launch remains Task 11, distinct from test-target QA.

Task 1 formatting follow-up `c17508e` changes only origin.html formatting;
`npm run format:check` and `git diff --check` then exited 0. No new native run
is inferred from whitespace formatting; the receipts/hashes above stay tied to
the explicitly tested source/artifacts.

## Task 2 — core contracts and protection policy

Commit `75ddb6b1361cf89eee83ca1bc9e53341b5d1f24c`. Independent task review approved
spec compliance and quality with no findings. Core-only implementation; no
scanner engine, filesystem adapter, service start, UI or dependency change.

Five missing-contract RED cycles precede GREEN for quantities/limits, states and
failures, read policy/protection, opaque IDs/snapshots and skip-code conversion.
Missing definitions are identified as contract/compiler RED, not falsely
reported as native/runtime behavioral failures. Final commands:

- `cargo test -p dev-storage-core scan_ --locked`: exit 0, 10 passed.
- `cargo test -p dev-storage-core --locked`: exit 0, 14 passed, doc-tests passed.
- Scoped `rustfmt --check --edition 2021` for the four core files and
  `git diff --check`: exit 0.

Tests pin every mode budget and canonical u64 quantity, reject Number/negative/
noncanonical/overflow input, distinguish null from observed zero, verify closed
fixed messages and sealed Protected/no-actions/Unknown serialization, reject
forged policy claims and deny unknown read/mount/cloud scope. Snapshot tests pin
opaque token/time/state/coverage shape and absence of descendant paths/identities.

These types do not establish future count/eligibility validation, dedup/overflow
transactions, traversal/cancellation, session replay, native scanning or UI
formatting; the owning later tasks must prove those behaviors. Whole frontend,
workspace Rust and native suites were not redundantly repeated for this pure
model/policy task. Checkpoint 2 remains in progress until Tasks 3–5 are verified.

## Task 3 — anchored macOS metadata cursor

Implementation commit `d125750`. Task-scoped tests and independent source review
passed. Checkpoint 2 remains open until accounting and engine Tasks 4–5 pass.
The cursor still has no application/IPC caller. Existing Phase 02 test adapters
were updated explicitly; saved macos-v1 identity semantics remain unchanged.

Actual commands reported against this source:

- `cargo test -p dev-storage-platform scan_ --locked`: 19 passed.
- `cargo test -p dev-storage-platform workspace_ --locked`: 6 passed.
- `cargo clippy -p dev-storage-platform --all-targets --locked -- -D warnings`:
  exit 0, no warnings.
- `cargo fmt --all --check` and `git diff --check`: exit 0.
- `cargo test -p dev-storage-application --no-run --locked` and
  `cargo test -p dev-storage-manager --no-run --locked`: exit 0; these compile
  affected test adapters, not an execution of those suites.

Controlled native fixtures cover saved-root identity/replacement, selected-root
symlink retargeting, descendant symlinks/loops/FIFO, Unicode and ordinary hidden/
.git/node_modules directories, sparse length/allocation, replacement before
openat and rename after acquisition. Syscall instrumentation records four
descendant directory opens in the mixed fixture, all no-follow, with no regular
contents, special-file or external-symlink-target opens. It does not claim that
third-party filesystems perform no internal I/O. Cancellation at every before/
after boundary closes acquired handles; the depth-32 fixture retains 33 cursors
and closes all 33 directory streams. Cleanup itself is not cancellable.

Home/volume-root, denied errno, cloud flags, missing/invalid attributes, mount
facts, allocation edge cases and non-UTF-8 names were injected. APFS rejected
creation of an invalid-UTF-8 fixture name; byte preservation was tested through
native-dirent injection instead. No real account-home guard, cloud provider,
mounted-volume crossing, permission-denied root or personal workspace traversal
was probed. These limits are retained, not relabelled as live-volume evidence.

Initial missing-contract/stub failures and four corrective RED→GREEN regressions
preceded final success (invalid native type, missing mount fact, late root
replacement and regular-file replacement). Other regression tests protect
already-present behavior; they are not all claimed as individually observed RED.

Independent review found no blocking correctness/scope defect. It noted a minor
maintainability opportunity to move the large fixture module out of scanner.rs;
this is recorded for whole-branch review, not silently dropped. The 36-descriptor
session cap still needs Task 5 enforcement; the adapter proves one descriptor per
frame, not concurrency control. Production home discovery remains unexercised
at this checkpoint and will be distinguished from the later native runner tests.

## Task 4 — bounded unique-object accounting

Implementation commit `eea6a8c`, independently reviewed with no findings. Pure core
accounting and diagnostics; no new production dependency or filesystem call.
Actual 64-byte identity slots retain first owner and bounded dependent-root bits;
directory pending/partial coverage cannot be upgraded merely by deduplicating.
All candidate identity, ownership and metric changes are checked before commit.
Unknown allocation retains usable logical measurements with overall partial
coverage; all-unknown allocation is null/unavailable, while an observed empty
set is known zero/complete. This resolves the spec's ambiguous overall wording.

Observed TDD: compiling accounting stubs failed four behavioral tests; pending
directory/unsupported-scope regressions then failed two before correction.
Diagnostics stubs failed two; grouping and terminal-count overflow each had a
subsequent observed failure before passing. Final task commands:

- `cargo test -p dev-storage-core scan_accounting --locked`: 6 passed.
- `cargo test -p dev-storage-core scan_diagnostics --locked`: 3 passed.
- `cargo test --workspace --locked`: 86 passed (application 6, core 23, Tauri 12,
  persistence 19, platform 26), doc-tests passed. This is a checkpoint run,
  not final verification of unimplemented Tasks 5–11.
- `cargo clippy -p dev-storage-core --all-targets --locked -- -D warnings`:
  exit 0, no warnings.
- `rustfmt --check --edition 2021 crates/core/src/scanner/accounting.rs crates/core/src/scanner/diagnostics.rs`
  and `git diff --check`: exit 0.

Synthetic cases cover hard-link exclusive 12/0 versus global 12, distinct-volume
identity, capacity exhaustion without forgetting duplicates, overflow rollback
and retry, pending/partial directory inheritance and allocation uncertainty.
Diagnostics retain at most 127 ordinary groups plus one reserved terminal group,
checked decimal counts and no child paths. Actual encoded DTO tests exercise the
128 KiB boundary with a 4 KiB terminal reserve. The size guard requires its future
publisher to measure and check the full JSON-encoded DTO; that integration is
not established by this helper. Root-order normalization remains Task 5;
checked native blocks × 512 was already verified in Task 3.

## Task 5 — streaming engine

Implementation commit `6294d8a` plus fix `4c6da52`; independent review and scoped
re-review passed after the allocation-coverage correction below. No application
service, IPC, native-runner hookup, UI or dependency change. New tests are purely
synthetic metadata cursors; existing platform unit fixtures running in the Rust
suite are not executions of the opt-in native-acceptance target.

Actual final commands reported:

- `cargo test --workspace --locked`: 100 passed (application 6, core 37,
  Tauri 12, persistence 19, platform 26); doc-tests passed.
- `cargo clippy -p dev-storage-core --all-targets --locked -- -D warnings`:
  exit 0, no warnings.
- `rustfmt --check --edition 2021 crates/core/src/scanner/engine.rs crates/core/src/scanner/tests.rs`
  and `git diff --check`: exit 0.

Initial compiling stubs produced 1 then 6 failed engine tests before GREEN.
Further observed RED→GREEN regressions covered the wire progress field,
verification of the entire active ancestor chain, cancel from the final callback
and exact known-omission lower bounds. Scope validation extended already-present
behavior, without claiming a separate preceding failure. Focused commands were
`cargo test -p dev-storage-core scan_ --locked`, plus filters
`scan_cancel_at_final`, `scan_cancel`, `scan_limits_and_partial` and
`scan_scope_validation` during those cycles.

Synthetic observations include known empty zero, 3 + 5 = 8 bytes, nested exclusive
8/0, stable complete totals across enumeration permutations, later duplicates
inheriting depth-partial coverage, 32 roots with peak 3 descriptors and a Deep
chain with peak 33. A blocked call retains its live cursor until released after
cancel; only then may the engine return. There is no service gate yet.
1,000 files plus EOF produce 1,001 cursor requests and at most 3 stationary-clock
publications. These are not real filesystem performance measurements.

`entryAttempts` counts issued cursor requests, including EOF/failure/stop, under
unchanged numeric budgets. The UI label will be “Traversal checks attempted”,
not an exact measured-object count. A diagnostic conservatively ends that cursor
branch partial; ancestor siblings/other roots remain eligible. Unknown stream
omissions have lower bound 0; a known skipped subtree/entry can contribute 1.
Entry, identity, descriptor, time, diagnostic, quantity, result and cancel stops
retain honest prefixes; depth remains a branch skip. Counter publications are
throttled; no queue of per-entry events or descendant-path catalogue is retained.

The report is a terminal candidate after cursor cleanup, not an IPC terminal
publication. Service Tasks 6–7 still must linearize cancellation/finalization,
retain the work reservation and enforce full actual encoded root/snapshot sizes.

Independent review identified a required correction: directory finalization
recorded logical omissions but lost allocation-only uncertainty. A later
workspace deduplicating that directory could therefore claim complete coverage
and known zero allocation even though its subtree's allocation was unknown.
The original global/first-root metrics remained partial, but the later root was
incorrect. The fix must retain separate logical/allocated directory coverage and
add nested all-unknown/mixed-allocation regressions; marking logical traversal
partial solely for unknown allocation is not an acceptable fix. Task 5 remains
was kept open until the correction and scoped independent re-review passed.

Correction `4c6da52` stores an allocation-uncertainty bit within existing slot
padding and propagates it separately through active ancestors, direct-file
duplicates and later directory duplicates. Logical coverage stays complete when
only allocation is unknown. A nested all-unknown/mixed matrix first reproduced
Completed versus required Partial, then passed after the fix; a sticky directory
finalization test prevents a later finish from upgrading unknown allocation.
Actual 64-byte slots and all capacities remain unchanged. Scoped re-review
confirmed the finding addressed with no new breakage.

Current amended-source commands: `cargo test -p dev-storage-core scan_ --locked`
passed 36 tests; core Clippy with all targets/locked/deny warnings, scoped rustfmt
and diff checks passed. The earlier 100-test workspace result belongs to
`6294d8a`, not the fix; it was not redundantly rerun here. Final consolidated
verification remains Task 11. Checkpoint 2 is implemented/reviewed with these
precisely scoped receipts, not a whole-phase functional or native UI PASS.

## Task 6 — owned sessions and configuration gate

Implementation commit `fa4c9c2` plus fix `6ac72fe`; independent review and scoped
re-review passed after the active-cancel reserve correction below. No Tauri,
platform runner, frontend, migration or native-target change. The normal service
still has no runner; a start cannot traverse until Tasks 7–8 pass their gates.
Synthetic runners/failure injectors and the coordinator's test clock are cfg(test).

Final task commands against this source:

- `cargo test -p dev-storage-application scan_ --locked`: 17 passed.
- `cargo test -p dev-storage-application workspace_ --locked`: 5 passed.
- `cargo test -p dev-storage-persistence scan_ --locked`: 1 passed.
- `cargo test --workspace --locked`: 121 passed (application 23, core 40,
  Tauri 12, persistence 20, platform 26); doc-tests passed, no warnings.
- `cargo clippy -p dev-storage-application -p dev-storage-persistence --all-targets --locked -- -D warnings`:
  exit 0.
- Scoped rustfmt checks for owned application/persistence files and
  `git diff --check`: exit 0.
- `cargo tree -p dev-storage-application -e normal --locked`: confirms the one
  approved direct runtime edge to existing serde_json 1.0.151. Cargo.lock changes
  only that application edge, not a package version/checksum/toolchain. The first
  offline resolution refreshed the edge; subsequent checks use locked resolution.

Observed RED→GREEN cycles cover compiling gate/query/service stubs, injected
thread-spawn and clock failures, missing terminal diagnostics, an oversized
callback incorrectly overwritten by a later small callback, empty-progress
initialization failure misclassified as result_limit, and the explicit exit hook.
Further already-green cases extend boundaries/read responsiveness without
claiming every assertion had a prior failure.

An accepted original-payload replay returns its retained session before gate or
current-epoch rejection; conflicting payload is request_conflict. Unseen/evicted
stale epochs reject before current-epoch busy requests. Blocked-runner tests keep
workerActive true and mutations rejected after cancel, then release the barrier
before terminal state and gate reuse. Health/list/manual check/replay respond
without waiting on runner I/O. Finalization-before-cancel preserves terminal;
cancel-before-finalization cannot become complete. Failed spawn retains the old
epoch/result and releases its unused lease. No retained session resumes on restart.

Actual serde_json encoding writes to a bounded counting sink. Internal root
descriptors are tuples of ID, display name, selected/canonical paths and identity;
the exact 32 KiB escaped boundary and +1 rejection occur before worker start.
Every accepted initial/callback snapshot and terminal/fallback is encoded and
checked; oversize freezes the last approved prefix, requests result_limit and
cannot be overwritten by a later smaller raw report. No JSON is truncated or
accepted diagnostic group dropped. Canonical entryAttempts travels with the
retained prefix. Startup clock failure yields typed application_unavailable,
never a successful null/fallback epoch, without disabling health/workspaces.

Explicit Rust-only shutdown_scans and Drop request cancellation and suppress
publication without joining blocked work; the worker retains its lease until
return. Task 8 must attach the explicit operation to normal Tauri ExitRequested.
The new-command real WK ACL matrix is still Task 7; these synthetic/core tests
do not replace it. No whole-phase functional or UI acceptance is inferred here.

Independent review found that active-stop reservation checked the
quantity_overflow/Partial shape but not the larger Cancelled per-row shape.
A 32-row boundary construction could admit a 126,941-byte snapshot with a
126,976-byte overflow reserve, then reject its 127,015-byte cancellation
acknowledgement. The control recorded cancellation but the returned snapshot
could remain Running until actual worker completion. Terminal cancellation and
lease ownership were not the defect. Fix `6ac72fe` checks both maximum-revision
QuantityOverflow and larger Cancelled envelopes, including terminal reserves.
The unsafe callback now stops result_limit before acceptance and preserves the
last approved prefix. A separate largest-safe 126,883-byte callback immediately
acknowledges Cancelling with 32 Cancelled rows while blocked; its maximum-revision
active envelope is exactly 126,976 bytes and +1 is rejected. All eight stop
reasons' active/terminal forms were encoded independently. No cap was enlarged.

Two focused boundary tests first failed, then passed. Current-source
`cargo test -p dev-storage-application scan_ --locked` passes 19 tests, app Clippy
with all targets/locked/deny warnings passes, and scoped formatting/diff checks
pass. The 121-test workspace run belongs to `fa4c9c2`, not this fix; final
consolidated verification remains Task 11. Scoped re-review found the issue
addressed with no new breakage. Task 6 is complete in its service-only scope.

## Task 7 — constrained IPC and eleven-command native authorization

Source `81e8c550db2297e53f22f9aa35c1bc3077c78df8`; independent task review
approved spec compliance and quality, no Critical/Important findings. Controller
also checked the actual native executable hash, unchanged bundle identifier and
no-runner initialization. Production still uses the no-runner constructor. No real traversal,
frontend change, dependency/schema/bundle-ID change or generic capability was
introduced. Four narrow local-main command grants extend the shared handler and
AppManifest. Actual webview/containing-window guards precede service work.
The picker now holds the owned configuration reservation through its callback,
blocking validation and commit, including receiver loss and timeout.

### Actual Wry authorization receipt

```sh
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case scanner-acl
```

Exit 0 against the final committed source, opt-in native test executable SHA256
`f48dfc60c376275bb492c2655418b57803c69e84149345b2f56c91a47ec1cbdd`.
Its runner is counted and synthetic, confined to tests: this proves transport
and authorization, not metadata-engine integration or normal-bundle operation.

| Commands                     | Child in main | Other window | External origin in actual main | Main in wrong parent |
| ---------------------------- | ------------- | ------------ | ------------------------------ | -------------------- |
| All seven inherited commands | ACL denied    | ACL denied   | ACL denied                     | Guard denied         |
| start_workspace_scan         | ACL denied    | ACL denied   | ACL denied                     | Guard denied         |
| get_workspace_scan           | ACL denied    | ACL denied   | ACL denied                     | Guard denied         |
| get_workspace_scan_session   | ACL denied    | ACL denied   | ACL denied                     | Guard denied         |
| cancel_workspace_scan        | ACL denied    | ACL denied   | ACL denied                     | Guard denied         |

Each of the three unauthorized contexts made eleven valid-shaped actual bridge
calls: zero tagged shared-handler entries, zero runner delta, identical full
workspace listing, SQLite bytes and scan status after every call. Wrong-parent
calls entered eleven guards but performed no service/picker/runner operation.
Counters identify the actual caller and test case; ordinary React calls do not
contaminate them. Missing bridge and unsettled-promise controls remained distinct
failures, never permission PASS. The controlled loopback origin listener stopped
and joined; no production remote grant changed.

Authorized main controls passed health/schema 2, list and five fixture mutations
or checks. Six scan invocations exercised all four new commands, one accepted
session/replay and one runner only. The live cancellation acknowledgment retained
workerActive=true and finishedAt=null; explicit runner release yielded cancelled
and inactive. Logical bytes `9007199254740993` and entryAttempts `3` survived IPC.

Allowed-main native picker cancellation was **not verified**. Two bounded
interactive attempts timed out despite a targeted AX read and one Escape; no
callback was observed. An intermediate outer PASS with a nested inconclusive
field is not acceptance evidence and was removed. Final default scanner-acl
explicitly reports nativePickerPositive=not_exercised. A separate test-only
`--picker-cancel` opt-in now requires a real cancelled result, unchanged database
and released reservation; error/timeout exits 1. It adds no product mode or grant.
Future reproduction: append that flag to the command above and cancel the native
panel once. This remaining positive-control gap does not replace or contradict
the completed actual eleven-command denial matrix.

### Focused and build receipts

- `cargo test -p dev-storage-manager scan_ --locked`: RED 0/3, then 1/5;
  final GREEN 6/0. Tests cover strict request nesting/fields/tokens, replay,
  stale/conflicting requests, lossless quantities, cancellation, picker ownership
  and deliberately unavailable production runner.
- `cargo test -p dev-storage-manager workspace_ --locked`: 10 passed.
- `cargo test --workspace --locked`: 127 passed (application 25, core 40,
  Tauri 16, persistence 20, platform 26), doc-tests passed. Later test-harness-only
  selector ordering was compiled and exercised by the final native run.
- `cargo check --workspace --all-targets --locked`: exit 0 on final source.
- `cargo clippy -p dev-storage-manager --all-targets --features native-acceptance,tauri/custom-protocol --locked -- -D warnings`:
  exit 0, including opt-in target.
- Scoped rustfmt and `git diff --check`: exit 0.
- `cargo build -p dev-storage-manager --bin dev-storage-manager --all-features --locked`:
  exit 0. Normal binary SHA256
  `da19eebc6bf285fa9f16bd9d73dcf515fa37305c800efc36437631b98c11f15c`;
  QA marker/report endpoints/case tag/CountedRunner absent. This is not a new
  app/DMG inspection; final Task 11 must repeat the complete exclusion audit.

No new frontend assets were needed or rebuilt here. No global preference, OS
permission or personal workspace was changed. New Phase 03 visual QA remains
awaiting human review. Inherited and new-command native ACL gates plus source
review now permit Task 8 integration; they do not establish its engine evidence.

## Task 8 — real metadata runner integration

Source `b0dc9d258941ec73290ebb7f5312a99d58388564`; independent task review
approved spec compliance and quality, no findings. Controller matched the native
and normal binary hashes below; prior core/platform/ACL receipts remain distinct.
Production and the isolated test executable now share native_runtime's
service factory: MacOsAdapter.scan_filesystem → MetadataScanRunner → core scan.
Both reviewed ACL gates preceded this wiring. No new permission, command, DTO,
dependency, schema or bundle identity. Construction does not start traversal.
Normal ExitRequested calls shutdown_scans without prevent_exit, joining or waiting
on blocked I/O. The test executable uses that same lifecycle callback for smoke.

Focused TDD: missing-runner API RED then 2 application integration tests GREEN;
registered-factory start returned application_unavailable before wiring, then
1 Tauri integration test GREEN. Native smoke first failed its post-event-loop
shutdown assertion, then passed after the shared ExitRequested hook. Retaining
the service outside app teardown distinguishes the hook from service Drop.

- `cargo test -p dev-storage-application scan_native_integration --locked`: 2 passed.
- `cargo test -p dev-storage-manager scan_native_integration --locked`: 1 passed.
- `cargo test -p dev-storage-application scan_ --locked`: 21 passed.
- `cargo test -p dev-storage-manager scan_ --locked`: 7 passed.
- `cargo build -p dev-storage-manager --locked`: exit 0, normal development
  binary built but not launched.
- Scoped rustfmt and `git diff --check`: exit 0. No consolidated suite repeated.

### Actual Wry real-engine fixture smoke

```sh
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case scan-smoke
```

Final-source run exit 0. Native executable SHA256
`2f245e20fe802edaafd215c483fe203daf54258c3d7c1d50f3dad2656c53358e`.
Each start explicitly selected only controlled fixture ID 1 from twelve temporary
rows. Fixture: root/sentinel 9 bytes and root/nested/data 5 bytes, three directory
entries total. Counts below include root and nested directory; traversal checks
include two EOF requests, not five measured objects.

| Mode  | Logical bytes | Observed allocated bytes | Files | Directories | Traversal checks | Elapsed IPC-to-terminal ms | End / stop reason          |
| ----- | ------------- | ------------------------ | ----- | ----------- | ---------------- | -------------------------- | -------------------------- |
| Quick | 14            | 8192                     | 2     | 2           | 5                | 69                         | completed, inactive / null |
| Deep  | 14            | 8192                     | 2     | 2           | 5                | 65                         | completed, inactive / null |

Durations include native transport/polling, not engine-only benchmarks. Allocation
is this volume's observation, not an asserted filesystem constant. Earlier 67/71 ms
runs are superseded by these final-artifact measurements. No performance promise
for personal repositories follows from this small fixture.

Listing, root check, enable changes and initial construction left session null.
Controlled Add occurred during fixture setup, not through a native picker.
SQLite bytes/listing and schema 2 stayed identical during scans. Sentinel/data
contents, exact entry counts, inode/size/nanosecond mtime remained unchanged.
A new service reopened on that temporary database had no retained session;
this is service reconstruction, not process-relaunch evidence. After actual
ExitRequested, the still-retained service returned application_unavailable.

The application test also runs the real engine/MetadataScanRunner against a
deliberately blocked synthetic filesystem: exact `9007199254740993` survives
serialization, cancel acknowledges while workerActive and gate remain held,
and only releasing I/O produces terminal cancelled and releases ownership.
This is not a claim of native OS-blocked-I/O cancellation. Real native smoke's
small scans finished normally. No personal workspace or application DB was used.

Normal binary SHA256
`00de68ffcca412a73fc93eb8a3c86912f8928769804b7c4cc47bc34546e2e080`;
no normal-bundle launch/DMG/current UI QA is claimed here. Those remain Task 11.
Human visual QA of Phase 03 remains awaiting review.
