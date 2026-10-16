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

| Checkpoint                           | Tasks | Status                               |
| ------------------------------------ | ----- | ------------------------------------ |
| Isolated native verification         | 1     | Implemented; reviewed receipts below |
| Contracts, policy and fixture engine | 2–5   | Tasks 2–5 implemented and reviewed   |
| Gated application/IPC/minimal UI     | 6–10  | Not executed                         |
| Functional/native/human acceptance   | 11    | Not executed                         |

## Inherited native obligations

- [P02-NATIVE-ACL](2026-10-16-phase-02-workspaces-verification.md#p02-native-acl):
  RESOLVED for the seven existing commands by Checkpoint 1 native evidence.
  New commands additionally
  need their own native denial matrix before MetadataScanRunner is wired.
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
