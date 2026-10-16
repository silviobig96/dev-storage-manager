# Phase 03 scanner core verification

Date: 2026-10-16. Normal checkout: `feat/phase-03-scanner-core`.
Implementation starts from reviewed design commit
`7ad8431880ff8be4a635523b4ca8d65bf5280f57`, with merged product base
`04018df8d152268cebd2f62aa554ddfa49c30d7a`.

## Current state

Tasks 1–10 and scoped fixes are implemented/reviewed. Task 11's source/helpers
also passed independent review. Consolidated automated FUNCTIONAL QA: PASS at
`9da451c` (182 frontend / 132 Rust tests). All five default native test cases
passed at `d1a0261`; current-source scan-smoke and the post-unlock scan-ui rerun
passed after the final scoped fix. The normal bundle also passed fixture-only
Quick/Deep, configuration restoration and process-restart checks below. The human
confirmed the narrowly scoped 200% keyboard/path/dialog check on the earlier
identified artifact below. Remaining real-input/accessibility and human acceptance
remain open. Final
whole-branch review found an Overview recovery presentation defect, now fixed;
the single scoped re-review confirmed both source corrections, with no new
Critical/Important breakage. This is not complete native verification or
phase acceptance.
HUMAN VISUAL QA: AWAITING HUMAN REVIEW for the new Phase 03 UI.
Phase 02 visual approval remains granted and is not reopened.

The implementation assignment accepts the design decisions and explicitly
authorizes the isolated Cargo test target, pinned dev dependency edges and
controlled 127.0.0.1:0 origin fixture. No broader production capability, real
workspace scan, release or Phase 04 feature is authorized.

## Checkpoints

| Checkpoint                           | Tasks | Status                                       |
| ------------------------------------ | ----- | -------------------------------------------- |
| Isolated native verification         | 1     | Implemented; reviewed receipts below         |
| Contracts, policy and fixture engine | 2–5   | Tasks 2–5 implemented and reviewed           |
| Gated application/IPC/minimal UI     | 6–10  | Tasks 6–10 implemented and reviewed          |
| Functional/native/human acceptance   | 11    | Automated/native cases pass; acceptance open |

## Inherited native obligations

- [P02-NATIVE-ACL](2026-10-16-phase-02-workspaces-verification.md#p02-native-acl):
  RESOLVED for the seven existing commands by Checkpoint 1 native evidence.
  The Task 7 eleven-command native denial matrix also passed before
  MetadataScanRunner integration; its actual receipt is recorded below.
- [P02-NATIVE-LAYOUT](2026-10-16-phase-02-workspaces-verification.md#p02-native-layout):
  RESOLVED by Task 11's exact empty/populated/current native geometry below.
  New Phase 03 input and human visual checks remain separate.
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

## Task 9 — frontend transport and session recovery

Source `c3ed5d5b17192f06be21b08ed167f4cbbc14903c`; independent task review
approved substance and quality with two minor follow-ups: retired-epoch tracking
grows per accepted session (final review must triage); transport recovery text
should match “Reconnect to check progress” when integrated in Task 10. No blocking
lifecycle defect found. Four typed IPC wrappers mirror current core DTOs and closed safe errors.
BigInt formatting retains exact bytes beyond Number precision; null is Not
available, known zero is 0 B, 1073741824 is 1 GiB. App mounts the controller above
both views without changing health/workspace lifecycle, focus, shell or routes.
No new scan UI, backend, dependency or persistence change.

Visible live-worker sessions schedule one query at a time, 250 ms after the last
response. Hidden/idle/unavailable/terminal states do not poll. Foregrounding queues
one recovery read after any outstanding query/action. Generations/action versions,
opaque epoch equality, session/request identity and BigInt revision checks reject
stale responses. Unmount/navigation never cancel or start native work.

Lost Start retains its original UUID, epoch and ordered IDs/mode. Reconnect only
reads. A matching retained request reconciles it; no matching session does not
falsely prove rejection. Explicit identical Start can replay that same payload;
changed intent is blocked. Read-only pendingStart IDs/mode exposes only enough
intent for presentation; UUID/epoch remain controller-owned. This state is in
memory, not a new persisted request history. Unknown cancel outcome likewise
keeps the last snapshot without inventing worker termination.

TDD: absent modules/signature stubs failed 15 focused assertions, then 15 passed.
App ownership assertion failed before mounting the hook, then passed. Additional
pending-intent and hide/show-during-cancel regressions each failed before their
fix; the latter now waits for both read and action responses before recovery.

```sh
npm run test:run -- src/lib/scans.test.ts src/lib/scanFormatting.test.ts src/features/scans/useWorkspaceScan.test.tsx src/App.test.tsx
```

Final focused run: 4 files, 40 tests passed (transport 2, formatting 5, hook 14,
App 19). `npx tsc -b`, scoped ESLint/Prettier and `git diff --check` exited 0.
Existing host npm user-config warning is not hidden by changing global settings.
Full frontend/native/current UI gates remain Tasks 10–11; this is not a new native
receipt or human visual approval.

## Task 10 — explicit scan controls and truthful presentation

Source `a0d7495` plus fix `8371051`; independent review and scoped re-review
approved after the focus regression correction below. Compact controls and shared
summary use the existing Workspaces/Overview structure and primitives; no shell,
theme, route, backend, capability, dependency or schema change. Start copies the
enabled scope, rejects duplicate activation and explains the 32-root request cap
without changing saved/enabled rows. Deep remains explicitly bounded.

The UI keeps unknown/reconciling outcomes conservative: workspace writes remain
disabled, retained metrics are labelled last known, Reconnect only reads, and
an explicit unresolved Start can reuse only its original IDs/mode. Manual root
checks, health and navigation stay available. Add/Remove/Enabled/Skip explain
their live-work restriction; the native picker/configuration guard blocks Start.
scan_in_progress is mapped as a definite backend rejection, not lost transport.

Logical and allocated coverage stay independent. Null/unavailable, observed zero
and zero exclusive contribution covered elsewhere are distinct; mixed nonzero
contributions retain their observed metrics. Real grouped diagnostics generate
different limit/access/change/allocation/cancellation/exclusion copy and counts.
Exact bytes accompany BigInt display units; no reclaimable metric, percentage,
ETA, chart or cleanup action. Meaningful state/error transitions are live, while
250 ms metric changes do not announce or move focus. Removal safety/focus and
full paths are preserved. Recovery text now matches “Reconnect to check progress.”

TDD: initial integration failed 8 behavior assertions; empty component stub then
failed 21 component behavior assertions. Implementation passed focused tests;
later regressions reproduced scan_in_progress transport misclassification and
mixed coveredBy contribution wording before their fixes.

- `npm run test:run -- src/features/scans/ScanSection.test.tsx src/views/WorkspacesView.test.tsx src/App.test.tsx`:
  82 passed.
- `npm run test:run`: 15 files, 172 passed.
- `npm run build`: exit 0, TypeScript plus Vite, 53 transformed modules.
- `npm run lint`: final exit 0 after test callback/async typing fixes; the two
  affected test files then passed 49 tests. No controller contract was weakened.
- `npm run format:check` and `git diff --check`: exit 0.

Existing npm user-config noise and Vitest's jsdom performance suggestion are not
native or product failures; no host settings were changed to silence them.
These are DOM/test and build receipts only, not browser/native visual review.
New Phase 03 visual approval and all current native matrix evidence remain
Task 11. The retired-epoch memory minor remains tracked for final review.

Review found that closing an open Remove dialog after scan status froze writes
tried to restore focus to its disabled Remove invoker; Add fallback was also
disabled. Four focused DOM regressions failed before fix `8371051`: Cancel,
simulated cancel/Escape event, busy original row with usable neighbor, and all
row/Add actions disabled with heading fallback. The view now checks invoker
usability and preserves original-row/neighbor order before enabled Add/heading.
Ordinary enabled-invoker return and safety copy remain unchanged; scan updates
alone do not move focus. Scoped re-review found the defect addressed without new
breakage.

Current amended-source receipts: focused Workspaces/App 57 passed;
`npm run test:run` 15 files/175 passed; `npm run lint`, `npm run build`,
`npm run format:check` and `git diff --check` exited 0. Earlier 172-test evidence
belongs to `a0d7495`, not the fix. Actual native keyboard/Escape/focus remains
Task 11, not proven by dispatching a DOM cancel event.

## Task 11 — consolidated and current native verification

Tested source: `d1a0261`, including native helper commits `84139a0` and `d23223b`.
Independent review approved the implemented helper/evidence scope, without
Critical/Important findings; full Task 11 acceptance remains open. Controller
matched the actual native executable hash and the empty production-hook diff.
Environmental npm/Vitest/hdiutil advisories remain qualified, not suppressed.

### Test assumption corrected, not a product defect

An intermediate constant-epoch successor mock failed and prompted relaxation
`accc674`. Controller review compared it with Rust's actual generation advance:
each accepted successor has a new epoch. The mock was invalid; no real product
defect was demonstrated. `d1a0261` removes that relaxation completely and tests
new-epoch successors, lost-response recovery, and same-epoch rejection. The
production hook is identical to `2fa6b16`; no weaker session rule survived.
Earlier 178-test/intermediate artifact receipts are not the final result.

### Consolidated automated gates

| Command                                                                                                                                                                  | Actual result                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `npm run format:check`                                                                                                                                                   | exit 0                                                                                            |
| `npm run lint`                                                                                                                                                           | exit 0                                                                                            |
| `npm run test:run`                                                                                                                                                       | exit 0; 15 files, 179 tests                                                                       |
| `npm run build`                                                                                                                                                          | exit 0; includes `tsc -b`, Vite 53 modules                                                        |
| `cargo fmt --all --check`                                                                                                                                                | exit 0; scoped scanner test-helper signature formatting                                           |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`                                                                                                         | exit 0                                                                                            |
| `cargo test --workspace --locked`                                                                                                                                        | exit 0; 130 tests: application 27, core 40, Tauri 17, persistence 20, platform 26; doc-tests pass |
| `CARGO_TARGET_DIR=target/native-acceptance cargo clippy -p dev-storage-manager --all-targets --features native-acceptance,tauri/custom-protocol --locked -- -D warnings` | exit 0; opt-in target included                                                                    |
| `npm run tauri build -- --debug`                                                                                                                                         | exit 0; normal app and DMG                                                                        |
| `cargo build -p dev-storage-manager --bin dev-storage-manager --all-features --locked`                                                                                   | exit 0; normal main, not test executable                                                          |
| `git diff --check`                                                                                                                                                       | exit 0                                                                                            |

Frontend gates/assets/bundle were refreshed after restoring the strict hook.
Rust production code did not change after its consolidated gate; helper Clippy
was repeated after the hold helper change. No redundant full Rust rerun was
claimed. Final whole-branch review is a separate remaining gate.

### Five explicit actual-native cases

Exact commands executed; normal Cargo tests do not implicitly execute this
required-feature target.

```sh
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case inherited-acl
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case inherited-layout
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case scanner-acl
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case scan-smoke
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case scan-ui
```

| CASE             | Actual result and scope                                                                                                                   |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| inherited-acl    | exit 0; seven inherited commands, real unauthorized callers and parent guards                                                             |
| inherited-layout | exit 0; both views, empty/populated configuration, exact native geometry/zoom/maximize                                                    |
| scanner-acl      | exit 0; all eleven commands; zero unauthorized handler/runner work and unchanged fixture DB/status                                        |
| scan-smoke       | exit 0; production factory/MetadataScanRunner, explicit fixture ID 1, schema/sentinels/exit hook                                          |
| scan-ui          | exit 0 after unlock; actual visible WK polling, explicit successive Quick/Deep, partial/complete/stopping and DOM dialog/focus assertions |

Earlier foreground waits failed while the Mac was locked. A supplemental
`--snapshot-rendering` case used real reload/mount recovery without a visibility
override and was explicitly narrower than the default. Only after visibility
changed and the human confirmed unlock was default scan-ui repeated and passed.
Default activation is still DOM, not real keyboard/pointer evidence.

Current eleven-command matrix: ACL means explicit rejection before tagged shared
handler entry for each real child/other-window/external-main caller. Guard means
one handler entry but actual-parent rejection before service/picker/runner work.

| Command                    | Child / other / external main | Wrong parent | Authorized-main control                                                   |
| -------------------------- | ----------------------------- | ------------ | ------------------------------------------------------------------------- |
| get_app_health             | ACL / ACL / ACL               | Guard        | schema 2                                                                  |
| list_workspaces            | ACL / ACL / ACL               | Guard        | fixture listing                                                           |
| select_and_add_workspace   | ACL / ACL / ACL               | Guard        | automated picker positive not exercised; human observation below          |
| remove_workspace           | ACL / ACL / ACL               | Guard        | fixture record only                                                       |
| set_workspace_enabled      | ACL / ACL / ACL               | Guard        | fixture off/on                                                            |
| check_workspace            | ACL / ACL / ACL               | Guard        | fixture root metadata                                                     |
| skip_workspace_setup       | ACL / ACL / ACL               | Guard        | setup persisted                                                           |
| start_workspace_scan       | ACL / ACL / ACL               | Guard        | one accepted synthetic runner/replay in ACL case; real runner in smoke/UI |
| get_workspace_scan         | ACL / ACL / ACL               | Guard        | authoritative status                                                      |
| get_workspace_scan_session | ACL / ACL / ACL               | Guard        | exact retained quantities                                                 |
| cancel_workspace_scan      | ACL / ACL / ACL               | Guard        | live acknowledgment then terminal after release                           |

All three unauthorized contexts have zero tagged handler entries/runner delta;
wrong-parent has eleven guard entries and zero runner delta. Fixture listing,
SQLite bytes and scan state stay identical after every denied call. Missing
bridge, navigation error or unsettled promise is failure, not ACL PASS. The
controlled 127.0.0.1:0 origin listener closes/joins; production grants are intact.

### Controlled fixture results

| Real case / mode | Logical bytes | Observed allocation | Files / directories / traversal checks | IPC-to-terminal ms | End and reason                                   |
| ---------------- | ------------- | ------------------- | -------------------------------------- | ------------------ | ------------------------------------------------ |
| scan-smoke Quick | 14            | 8192                | 2 / 2 / 5                              | 63                 | completed; stopReason null                       |
| scan-smoke Deep  | 14            | 8192                | 2 / 2 / 5                              | 64                 | completed; stopReason null                       |
| scan-ui Quick    | 14            | 8192                | 2 / 22 / 33                            | 286                | partial; depth_limit diagnostic, stopReason null |
| scan-ui Deep     | 28            | 12288               | 3 / 23 / 37                            | 271                | completed; stopReason null                       |

Smoke has files of 9+5 bytes, three directory entries and root+nested directory.
UI has twelve temporary roots: the smoke tree, ten nested directories with a
14-byte sentinel under root 2, and ten empty roots. Hand-counted totals are
14 directory entries, 23 directories including roots, three files and 28 bytes.
Quick's omitted branch yields one unmeasured-entry lower bound and null unknown
bytes; Deep completes within policy. Traversal checks include EOF, not just
objects. Durations include IPC/polling, not engine-only benchmarks; allocation is
this volume's observation, not a portable constant or recoverable-space estimate.

SQLite/listing/schema 2 and sentinels remained unchanged. Smoke also checks inode,
size, nanosecond mtime and exact entry counts. Reconstructing a service retains
no scan; this is not normal-process relaunch evidence. No personal root was used.

Only the third UI-runner invocation injects controlled stopping after fixture
metadata work: exact `9007199254740993`, null allocation, TimeBudget request and
held worker. It renders exact bytes/Not available, freezes writes and keeps
workerActive until release. This is not native OS-blocked I/O, a measured
five-second deadline or real allocation observation. Later interactive starts
use the real MetadataScanRunner again. No synthetic runner ships in the product.

### Native geometry, input and human boundary

Actual CSS 760×520, 1100×720 and 1600×900 in Light/Dark pass for Overview and
Workspaces, including empty centering, current result controls, separators,
long paths and no document/main/control horizontal overflow. One measured
titlebar correction per size; no raster-size inference. WK pageZoom getter
confirms 2.0 without enlarging to hide pressure and restores 1.0. Native maximize
measures 1728×965 (zoomed 864×482) in inherited/supplemental cases; the final
visible default/held scan-ui case measures 1920×1018 (zoomed 960×509). These
are per-artifact observations, not a fixed display assumption or the older P02
dimensions.
Empty centering delta is at most 0.0078125 CSS px. DOM-activated modal focus and
Cancel return pass; that is not real keyboard/Escape input.

After unlock a single held fixture was shown at native 200%, getter verified,
without personal data. Asked specifically about Add/cancel/reopening, the human
said “lo veo bien, esta muy grande las letras pero no veo que se duplique nada”.
This records no duplicated panels observed, not complete cancellation/reopen,
keyboard-cycle or new Phase 03 visual approval. The large type is the deliberate
native zoom, not a typography change.

In that same live fixture the human answered “si” to the separate question about
Tab/Shift+Tab reaching Quick scan, Deep scan, Enabled, Check availability and
Remove workspace; full-path reading/scrolling; and closing the dialog with
Escape or Cancel and returning to a usable control. This is human-reported
current native functional keyboard/200% evidence, not automated input or visual
approval. It does not establish unasked modal Tab wrapping/background inertness,
Cancel scan keyboard activation, VoiceOver or contrast/reduced-motion results.
The ten-minute held session completed naturally with exit 0 at 18:43:52 local;
the post-hold native getter verified pageZoom 1.0 before final PASS. No input or
termination was sent to close it. Pre-hold DB/sentinel assertions do not cover
any human picker interaction during the hold.

VoiceOver: NOT REPEATED AT HUMAN REQUEST; no fresh Phase 03 PASS. Prior P02
human evidence stays historical. Increase contrast/Reduce motion authorization
was not granted, so no global setting was changed. Remaining real-input scope
and human visual approval must not be inferred from DOM assertions or builds.
No private screenshot was captured or committed.

### Artifact identity and product exclusion

| Artifact at d1a0261                   | SHA256                                                           |
| ------------------------------------- | ---------------------------------------------------------------- |
| native acceptance executable          | 105b6e2bd6a983615d54a94588bc67fbbe6c9953cb809bf76e0fbd592b8d569f |
| normal debug app / DMG app executable | 04b8e446166ca2be4b65076a133d5f5db2ad0f55818f65988963a60423b51d8e |
| normal all-features main              | 600a231a35e36a6a6c6f637149784496ea9730bf03f3e49047eebe686e94b934 |
| debug DMG                             | 9883d4cd13550298feb64bff0a9a783a76aaf6538e1bcd9ae0f7f2dd95d034b4 |

Cargo metadata confirms a test-only required-feature target, no QA bin; direct
tempfile/objc2-web-kit QA edges are dev-only. Normal/build feature tree, source
inclusion, normal and all-feature main builds, app and read-only mounted DMG
contents were inspected. All locked package versions match base 04018df; only
documented edges changed. The normal app contains Info.plist, executable/icon;
the DMG contains that app, Applications link and normal presentation metadata.
Mount detached. Six QA markers positively exist in the test executable and are
absent in both normal executables and DMG app; strings alone are not the proof.
Bundle identifier is unchanged. Loopback listener and UiRunner are test-only.

Normal app/DMG were built and inspected, not launched/scanned in this d1a0261 receipt.
At that point fixture-only normal-bundle smoke remained open: never click all-enabled Start
with personal roots enabled, change unrelated flags, reset DB or add a diagnostic
backdoor. Use existing authorized-main tooling for only a deliberately selected
temporary fixture ID if available; otherwise report the gap. Final whole-branch
review, human acceptance and publication remained separate pending steps. The
later normal-bundle receipt below supplies new evidence, without changing these
historical labels.

### Remaining acceptance procedures

The current 200% human confirmation closes only the actions explicitly asked
about. Keep the following scope separate from automated and historical receipts:

| Check                       | Missing evidence / reproducible next step                                                                                                                                                                                                                   |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Full real-input modal cycle | In the fixture window, open Remove; Tab and Shift+Tab through the complete cycle, confirm focus cannot enter the background, then Escape/Cancel and usable return. DOM assertions and the human's narrower return confirmation do not prove the full cycle. |
| Cancel scan keyboard action | With controlled live work, reach and activate Cancel using the keyboard, observe cancelling/stopping, and confirm configuration remains frozen until worker completion. Synthetic DOM activation is not real input.                                         |
| Contrast / reduced motion   | Obtain authorization, record original preferences, enable each temporarily, inspect both views and new state feedback in the fixture window, and restore exact originals. No preference was changed in this run.                                            |
| Fresh screen-reader reading | Not repeated at the human's request; no new Phase 03 reader PASS. Do not substitute the historical P02 result or infer announcements from the DOM.                                                                                                          |
| Normal-bundle fixture smoke | RESOLVED by the current-source receipt below; only a newly created fixture's ID was passed through the existing authorized-main inspector. No personal enabled root was scanned or modified.                                                                |
| New human visual approval   | Inspect the actual Phase 03 controls/results at normal zoom, both appearances and small/wide sizes; request explicit approval of this new UI. The keyboard/zoom replies do not grant it.                                                                    |

No future check may expand production permissions, add a diagnostic backdoor,
reset the real database, select personal roots or silently relabel an omission
as PASS. Phase 04 has not started, and no release acceptance is granted.

## Final whole-branch review and scoped fix

The read-only independent review covered `04018df..96299bb`, including every
layer, native authority, test isolation and earlier deferred risks. It found no
Critical issue or new load-bearing traversal, replay, persistence or containment
defect, but identified one Important presentation defect: Overview selected its
empty state solely by snapshot presence, hiding uncertainty after an initial
status failure or a lost first Start response. A read-only rendering diagnostic
reproduced the false “No storage analysis yet” prompt.

One fix wave, commit `9da451c97c1d1f38a5ec190b6826064c10add901`, reuses the
existing summary for reconciling/unavailable states and directs recovery through
Workspaces. Ready with a confirmed absent session retains the empty state.
The production hook and its replay/stale guards are unchanged. Four focused
Overview/App tests failed before the correction; 74 focused frontend tests then
passed, covering initial read failure, navigation after lost first Start,
read-only reconciliation, identical explicit replay, mode protection and focus.

The same wave fixes a Minor native errno misdiagnosis: EMFILE/ENFILE now stop
with DescriptorLimit rather than UnsupportedFilesystem. A failing errno test
became green for both errors at root/descendant contexts. A core characterization
test confirms partial coverage, preservation of a measured prefix, no later-root
work and cursor release; the engine's existing stop propagation needed no change.

### Current-source verification at 9da451c

| Command                                                                                | Actual result                                                                                     |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `npm run format:check`                                                                 | exit 0                                                                                            |
| `npm run lint`                                                                         | exit 0                                                                                            |
| `npm run test:run`                                                                     | exit 0; 182 tests / 15 files                                                                      |
| `npm run build`                                                                        | exit 0; `tsc -b` and Vite 53 modules                                                              |
| `cargo fmt --all --check`                                                              | exit 0                                                                                            |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`                       | exit 0                                                                                            |
| `cargo test --workspace --locked`                                                      | exit 0; 132 tests: application 27, core 41, Tauri 17, persistence 20, platform 27; doc-tests pass |
| `npm run tauri build -- --debug --verbose`                                             | final exit 0; normal app and DMG                                                                  |
| `cargo build -p dev-storage-manager --bin dev-storage-manager --all-features --locked` | exit 0; normal main                                                                               |
| `git diff --check`                                                                     | exit 0                                                                                            |

Focused platform scanner 20/core scanner 37 tests and their Clippy also passed
before the consolidated workspace run. This was one workspace Rust regression
pass for the changed errno behavior, not a documentation-only rerun.

The first sandboxed normal package attempt built the app but failed at the DMG
script. The first sandboxed scan-smoke encountered LaunchServices/hiservices
XPC errors before readiness; only its owned fixture process was terminated.
Read-only mount inspection found no app image left mounted. One authorized
outside-sandbox retry of packaging and smoke passed on unchanged source/config,
matching the previously documented macOS-service limitation. No production
permission, host setting or toolchain was changed. These initial failures remain
identified, not silently converted into successful native evidence.

The exact opt-in scan-smoke command above passed at `9da451c`: production runner,
explicit temporary ID 1, schema 2, unchanged DB/sentinels. Quick 66 ms and Deep
78 ms each observed 14 logical / 8192 allocated bytes, two files, two directories
and five traversal checks, completed with null stopReason and inactive worker.
The earlier depth-partial UI measurements remain evidence at `d1a0261`, not a
new benchmark of the corrected artifact.

The exact default opt-in scan-ui command at `9da451c` exited 1 with
`Timeout("foreground native frontend")` after its existing 60-second wait; it
did not reach scan assertions. Controller CUA then independently reported the
Mac locked. No repeated blind attempts or visibility override was used. A single
manual unlock request was made; opening the normal bundle was also blocked at
this point. These failed attempts are distinct from the post-unlock results below.

| Current artifact at 9da451c                                | SHA256                                                           |
| ---------------------------------------------------------- | ---------------------------------------------------------------- |
| Native acceptance executable                               | fc82ca36406fb11227414631a0c5b73074cbfbf7e94fcdb13eff397576eb52eb |
| Normal app executable                                      | 3a2b78b5995ed0ee77898481ccf7e487d27f54475f68059f50615b37a36616f9 |
| Debug DMG                                                  | 9b1d341aecff630a1e4ad7ddb5b84922bab4eab036e9c86b8d6496f51358c5bc |
| All-features normal main, captured before ordinary rebuild | 1ae18528c9e43b5944de47b6119a115e8d2e8d2a7b85be3231154d4dff78b443 |

The five-file fix changes no registration, native test source, dependency edge,
capability, schema or identifier. Earlier structural/source/DMG exclusion proof
therefore remains identified at `d1a0261`; current normal/all-features compilation
also passed. No claim of a second mounted-DMG audit is made.

Explicit minor follow-ups: retiredEpochs retains a string per accepted manual
session, and the native scanner fixture module can be extracted for readability.
The small memory-growth risk is deferred rather than weakening opaque-epoch
stale-response protection during acceptance; a bounded replacement needs its own
in-flight generation and long-session regressions. Neither is claimed fixed.
Environmental npm/Vitest/hdiutil advisories remain qualified.

The one scoped re-review of `96299bb..9da451c` confirmed the Overview and errno
corrections addressed with no new breakage. It explicitly left epochSet growth
unfixed/deferred and distinguished the blocked UI attempt from historical PASS.
No second fix wave, repeated whole-branch review or acceptance claim followed.

### Post-unlock native rerun and normal-bundle smoke

The human replied “listo y continua con el trabajo”; CUA independently confirmed
the unlocked environment. One rerun of the exact default scan-ui command on
unchanged `9da451c`/native hash `fc82ca36…eb52eb` exited 0, visible Wry, no hold
or visibility workaround. Quick was partial at 278 ms: 14 logical / 8192 allocated
bytes, 2 files / 22 directories / 33 checks, depth_limit diagnostic and null
stopReason. Deep completed at 269 ms: 28 / 12288 bytes, 3 / 23 / 37, null stopReason.
DB/sentinels, current DOM focus/dialog and native geometry assertions passed.
These timings are IPC/poll observations on fixtures only. The initially blocked
current UI attempt is not relabelled; this is fresh sufficient visibility evidence.

`npm run tauri dev` also compiled and started the normal development executable
on current source (14.20-second Rust build; Vite ready). Its owned session was
stopped with Ctrl+C, exit 0, before the fixture case to avoid concurrent windows.
This startup alone did not establish visual or scan PASS.

Controller then opened the current normal `.app` (executable SHA256
`3a2b78b5…616f9`) using CUA. Actual `tauri://localhost` Overview showed Foundation
connected / Schema 2 and no scan. Workspaces navigation worked. An existing
personal enabled record was observed, so the all-enabled Quick/Deep UI controls
were deliberately not used and no unrelated flag was changed.

The existing debug Web Inspector was available through Inspect Element. Its
authorized-main console invoked only existing narrow commands, not added QA APIs:
list_workspaces, select_and_add_workspace, get_workspace_scan,
start_workspace_scan, get_workspace_scan_session, get_app_health and
remove_workspace. The native picker was directed explicitly to one newly created
controlled fixture: two files of 9+5 bytes and one nested directory. Only that
new record's exact ID was sent, one ID per scan; no JavaScript path went to Start.

| Normal-bundle mode | Result    | Logical / allocated | Files / directories / checks | IPC-to-terminal ms | Stop reason |
| ------------------ | --------- | ------------------- | ---------------------------- | ------------------ | ----------- |
| Quick              | completed | 14 / 8192 bytes     | 2 / 2 / 5                    | 58                 | null        |
| Deep               | completed | 14 / 8192 bytes     | 2 / 2 / 5                    | 56                 | null        |

Runtime assertions verified one result row for only that ID, terminal worker
inactive, the expected exact logical/file/directory counts, and Schema 2. A JSON
comparison of listing/setup/overlaps excluding the fixture equalled the original.
Only the fixture configuration record was then removed; the entire listing again
equalled the original. The real DB was not reset and its bytes are not claimed
unchanged across this legitimate add/remove. Both fixture files retained their
original SHA256 and 9/5-byte lengths; their contents were not read by the scanner.
Console-only scratch state was removed and the inspector closed; none is product
code, a capability, a distributed test mode or a committed local inventory.

A normal reload recovered the Deep result in Overview (14 bytes / 8 KiB, exact
8192 bytes, five checks). Agent inspection captured only that safe native
Overview, with no personal path; the image is inline tool evidence, not a saved
repository artifact. The app was quit and reopened normally: Overview returned
to “No storage analysis yet” with Schema 2, demonstrating process-local results
rather than persisted history. The reopened app remains available for human
review. No screenshot pixel dimensions were used as CSS measurements.

This resolves the normal-bundle smoke gap, not real-pointer Quick/Deep UI
activation, every modal keyboard criterion, contrast/reduced motion or human
visual approval. HUMAN VISUAL QA remains AWAITING HUMAN REVIEW.

### Documentation and publication receipt

After the source/native checks, `npm run format:check`, `git diff --check` and
`git diff --check 04018df HEAD` exited 0. A read-only Node check validated 51 local
Markdown links/anchors across seven phase/architecture/roadmap documents. No
product suite was repeated for documentation-only changes. No database, binary,
capture or private QA path was added to Git.

Evidence commit `14dc6c2c31aa09465dd20c92c44596bc04dd4ad3` was pushed using
`git push -u origin feat/phase-03-scanner-core`, exit 0. A separate
`git ls-remote --heads origin refs/heads/feat/phase-03-scanner-core` returned the
same full hash as local HEAD; `git status --short --branch` was clean. This
receipt identifies the first verified publication; the subsequent documentation
receipt does not change the tested product source `9da451c`.

Task 11 and phase acceptance remain open for the explicitly retained native/input
checks and human visual review. No PR, merge, tag, release or Phase 04 was created.
