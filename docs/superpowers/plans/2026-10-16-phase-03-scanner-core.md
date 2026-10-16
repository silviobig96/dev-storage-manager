# Phase 03 — Read-only Scanner Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans` after
> human review and selection of execution method. Steps use checkbox (`- [ ]`)
> syntax. Normal checkout only; never create a worktree for this plan.

**Goal:** Measure explicitly requested workspace metadata with bounded work,
honest coverage and cancellation, without reading file contents or mutation.

**Architecture:** Add models/policy/accounting/streaming engine to core, a
descriptor-relative macOS cursor to platform, and one memory-only scan
coordinator to ApplicationService. Thin authorized Tauri commands and an
App-level React controller expose a minimal existing-view integration.

**Tech Stack:** Existing Rust stable/edition 2021, serde, libc 0.2.189, Tauri
2.11.5/tauri-build 2.6.3/Wry 0.55.1, Tokio 1.53.1, SQLite schema 2, React 19,
TypeScript 6/Vite 8/Vitest 5. Do not upgrade packages, Node 22 or toolchain.

**Spec:** [Phase 03 design](../specs/2026-10-16-phase-03-scanner-core-design.md).
**Status:** Approved for implementation with the 2026-10-16 assignment's narrow
clarifications. Checkboxes record actual execution, not approval. Base `04018df`
on `feat/phase-03-scanner-core`. Phase 02 human visual approval is already granted;
its three native checks remain open until new sufficient evidence closes them.

## Global constraints

- Zero or more workspaces remain valid; explicit start uses 1–32 enabled IDs.
  No arbitrary JS paths, automatic scans, scan resumption or general Mac scan.
  Configuration itself has no 32-workspace cap; over 32 enabled blocks UI start
  with the spec's actionable message, never a silently chosen subset.
- No content reads, hashes, project tooling, providers, cleanup, filesystem
  writes, permission changes, monitor, product networking/telemetry or Windows.
- Database/migrations stay schema 2; results in memory only, no history tables.
- Preserve existing health/Workspace DTOs, identity format, uncertainty/focus
  protections, full-width Overview and `io.github.silviobig96.devstoragemanager`.
- New Workspace `scan_in_progress` error is an explicit additive contract,
  not a generic error/false success. Existing errors remain unchanged.
- One session/traversal thread, no queued sessions; retained work owns its
  reservation through stopping/cancellation. No SQLite lock during traversal.
- Spec budgets are fixed backend policy: Quick 25,000 entries/depth 8/5 s/25,000
  identities/4 MiB slots; Deep 250,000/depth 32/120 s/250,000/32 MiB slots.
  Both: 36 scanner FDs, 128 diagnostic groups, 32 KiB JSON-encoded root snapshot,
  128 KiB result, 250 ms live polling.
- One numeric-ID-ordered traversal, exclusive attribution/global deduplication,
  decimal-string quantities, GiB labels, unavailable distinct from zero.
- P02-NATIVE-ACL must pass before real scanner integration; new commands need
  native denial coverage too. P02-NATIVE-LAYOUT/ZOOM must pass before Phase 03
  UI acceptance. No automatic inheritance of PASS from mocks/browser/builds.
- The implementation assignment approves the test-only loopback fixture and dev edges
  `tempfile =3.27.0`, `objc2-web-kit =0.3.2` (`std,WKWebView`, no defaults).
  No production permission expansion, harness resource or debug product mode.
- Functional, browser, native test-target, normal bundle and human QA remain
  separately labelled. New Phase 03 visual acceptance is required, not a repeat
  of Phase 02 approval. No release acceptance is implied.
- Before execution, require clean correct checkout and inspect current task's
  Git authority. Push only if that task authorizes the named feature branch;
  never PR/merge/tag/release, reset, stash, rebase, worktree or remote replacement.

## Review focus

- A renamed directory between metadata and open must not redirect into a
  symlink target; descriptor tests in Task 3 and an engine barrier in Task 5.
- A blocked syscall after cancellation must not permit another scan or a
  workspace removal; retained-worker/gate tests in Task 6.
- A lost start response or very late replay must not start a second traversal;
  epoch/idempotency tests in Tasks 6, 7 and 9.
- Nested roots/hard links with reversed request order must not inflate totals
  or change ownership; accounting and engine tests in Tasks 4–5.
- A cloned/sparse/cloud object or missing allocation must not become an exact
  physical/reclaimable total; platform/accounting/UI tests in Tasks 3, 4 and 10.

## File ownership and checkpoint order

Checkpoint 1 is Task 1 (isolated native verification). Checkpoint 2 is Tasks 2–5
(core/policy/platform with fixtures). Checkpoint 3 is Tasks 6–10 (application,
transport and minimal UI). Checkpoint 4 is Task 11 (integration acceptance).
These are checkpoints within Phase 03, not new roadmap phases.

Core source: `crates/core/src/scanner/{mod,model,policy,accounting,engine}.rs`.
Platform: `crates/platform/src/macos/{identity,scanner}.rs` plus a public
`crates/platform/src/scanner.rs` factory implementing core traits. Application:
`crates/application/src/{activity,scans}.rs`. Transport:
`src-tauri/src/{registration,scans}.rs`; existing workspace runtime remains.
Frontend: `src/lib/scans.ts`, `src/features/scans/{useWorkspaceScan,ScanSection}`
with covering tests/styles. No new crate, router, UI framework or persistence layer.

## Task 1: Build the isolated native verification target, not a product mode

**Files:** New `src-tauri/tests/native_acceptance.rs`,
`src-tauri/tests/native_acceptance/{mod,acl,geometry}.rs`,
`src-tauri/tests/native_acceptance/origin.html`, `src-tauri/src/registration.rs`; modify `src-tauri/src/lib.rs`,
`src-tauri/Cargo.toml` and dev dependency edges in Cargo.lock. Existing command
modules and build/capability registry are reused, not duplicated. Evidence in
`docs/superpowers/reviews/2026-10-16-phase-03-scanner-core-verification.md` (created
only during implementation); update P02 checks only on actual evidence.

**Interfaces:** Extract existing
`application_context<R: Runtime>() -> Context<R>` and
`application_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync`
unchanged into registration. Production and path-included test modules use them.
Test-only `run_case(case: NativeCase) -> Result<NativeEvidence, QaFailure>` and
`measure_css(webview: &Webview<Wry>) -> future Result<Geometry, QaFailure>` own
five-second eval deadlines. `NativeCase` is inherited-acl or inherited-layout;
future scanner-acl/scan-smoke/scan-ui extend the test enum, never application commands.

- [x] Add the `native_acceptance` Cargo test target with `harness=false`,
      `required-features=["native-acceptance"]` and an empty opt-in feature.
      Set up the main-thread Wry event loop with temporary Database, seeded
      fixture roots and unchanged production-generated context/capability.
      Add the spec's reviewed dev-only edges without upgrading any version;
      resolve edges offline once, inspect lock, then use locked commands.
- [x] Write native cases first: allowed main health/list succeeds; ungranted
      child, other window and external-origin actual main reject every existing
      command with handler-entry count 0 and identical fixture DB; wrong-parent
      main rejects its guard before service/picker. A deliberately invalid case
      must fail if its promise times out or transport is missing, not count as
      denied. Use the test-only ephemeral loopback server, not Internet origins.
- [x] Run `npm run build`, then
      `CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case inherited-acl`.
      The explicit custom-protocol feature embeds frontendDist in the pinned
      context macro, avoiding the configured dev URL without a Vite server.
      Establish failure of missing cases/assertions, implement native evaluation
      result collection with `eval_with_callback`, then require actual Wry PASS.
      Do not label a missing test target/lock setup failure as behavioral RED.
- [x] Add geometry assertions: exact CSS sizes 760 × 520, 1100 × 720, 1600 × 900
      in both native per-window themes; at most two titlebar corrections;
      view/section/separator content-width delta ≤1 px after measured padding,
      Overview empty centering ≤1 px,
      no main/document horizontal overflow. Native getter pageZoom must equal
      2.0 after Rust `set_zoom(2.0)`, and 1.0 after restoration. Exercise long-path
      actions, scroll/dialog text and focus with native input, not synthetic DOM
      events as proof of keyboard use. Run the same command with
      `--case inherited-layout`; separate automatic geometry from agent/human
      interaction and capture evidence. No global appearance changes required.
- [x] Verify separation using `cargo metadata --no-deps --format-version 1`,
      `cargo tree -e normal,build -p dev-storage-manager --locked`,
      `cargo build -p dev-storage-manager --bin dev-storage-manager --all-features --locked`
      and a normal `npm run tauri build -- --debug`. Audit manifest target kind,
      bundle/DMG file lists, normal-binary symbols/strings: QA marker and fixture
      report endpoints absent even in all-feature main build. No QA code under
      normal modules, default resources or debug_assertions. Require unchanged
      production capability bytes. Commit `test: add isolated native acceptance target`.

**Exit:** Actual native inherited ACL PASS is the later integration gate. Record
layout/zoom separately; environmental failures remain open, not blocked design.
If native ACL cannot be established, continue only isolated fixture work in
Tasks 2–5, not real application/scanner integration. Preserve all P02 historical
measurements, acceptance, and each check's original integration limit.

Task 1 completed at `de0e55b`, independently reviewed. See the
[actual native receipt](../reviews/2026-10-16-phase-03-scanner-core-verification.md#checkpoint-1--native-acceptance-target).
Inherited ACL/zoom resolved by new evidence; layout remains conservatively open
for empty Workspaces and the current UI checks at Task 11. No new scanner gate
or Phase 03 human visual PASS follows from this checkpoint.

## Task 2: Define scan contracts and conservative read/protection policy

**Files:** New core scanner `mod.rs`, `model.rs`, `policy.rs`, modify
`crates/core/src/lib.rs`; tests in the new modules. No dependency additions.

**Interfaces:** `ScanMode::{Quick,Deep}`, `ScanRequest` (spec start request),
`ScanSessionId`, `ScanEpoch`, `ScanRequestId`, `ScanState`, `ScanMetrics`,
`WorkspaceScanResult`, `ScanSnapshot`, `ScanFailure`, `ScanDiagnostic`,
`ScanLimits::for_mode(mode: ScanMode) -> ScanLimits`. Add `DecimalQuantity(u64)` serde as canonical
decimal string and `SafetyLevel::{Safe,Review,Protected}`.
`ScanReadPolicy::evaluate(facts: &ReadFacts) -> ReadDecision` and
`measurement_protection() -> MeasurementProtection` enforce spec metadata-only
decisions and always-Protected/no-actions output. Core does not inspect Tauri/SQL.

- [x] Write `scan_contract_quantities`: serialize 9,007,199,254,740,993 and
      u64::MAX exactly as strings, reject Number, negative/noncanonical strings;
      nullable allocated/unmeasured values stay null, observed zero stays "0".
      Round-trip modes/states/errors, fields/casing and decimal revision/time;
      no private object identity or descendant path appears in JSON.
- [x] Write `scan_read_policy`: metadata on ordinary/hidden/.git/manifest entries
      remains allowed inside the read scope; external symlinks, unsupported
      mount/cloud facts return Skip with exact reasons. For every mode and size,
      assert measurement protection Protected, actions empty, reclaimability
      Unknown. A proposed Safe claim cannot downgrade it. Broad roots get
      root_too_broad without deleting configuration.
- [x] Run `cargo test -p dev-storage-core scan_ --locked`; require behavioral or
      missing-contract RED, implement spec types/budgets/messages, rerun GREEN.
      Pin every numeric limit rather than testing only presence of a field.
- [x] Commit `feat: define read-only scan contracts and protection policy`.

The quantity assertion is a real test body, not an approximate Number comparison
(the existing core dev dependency already includes serde_json):

```rust
#[test]
fn scan_contract_large_quantity_is_lossless() {
    let value = DecimalQuantity(9_007_199_254_740_993);
    assert_eq!(serde_json::to_string(&value).unwrap(), "\"9007199254740993\"");
    assert!(serde_json::from_str::<DecimalQuantity>("9007199254740993").is_err());
    assert!(serde_json::from_str::<DecimalQuantity>("\"01\"").is_err());
    assert_eq!(serde_json::to_string(&DecimalQuantity(u64::MAX)).unwrap(), "\"18446744073709551615\"");
}
```

**Exit:** Stable serialized interfaces/limits used by later tasks; no provider
contracts rewritten and no target-level findings/cleanup API introduced.

## Task 3: Implement a descriptor-relative macOS metadata cursor

**Files:** New `crates/platform/src/scanner.rs`,
`crates/platform/src/macos/scanner.rs`, `macos/identity.rs`; modify platform
`lib.rs`, `macos.rs`, `macos/workspaces.rs`, and core scanner `model.rs` for the
trait signatures below. Update fake PlatformAdapter implementations explicitly
in existing application/Tauri tests; no permissive production fallback.

**Interfaces:** Add `PlatformAdapter::scan_filesystem(&self) -> Box<dyn ScanFilesystem>`.
`ScanFilesystem: Send + Sync` supplies
`anchor_root(&self, root: &StoredWorkspace, control: &dyn ScanIoControl) -> Result<Box<dyn ScanDirectory>, ScanIoError>`.
`ScanDirectory: Send` provides `identity() -> ObjectIdentity`,
`next_entry(&mut self, control: &dyn ScanIoControl) -> Result<Option<ScanEntry>, ScanIoError>`,
`open_child(&mut self, entry: &ScanEntry, control: &dyn ScanIoControl) -> Result<Box<dyn ScanDirectory>, ScanIoError>`,
`verify_link(&self, entry: &ScanEntry, child: &ObjectIdentity, control: &dyn ScanIoControl) -> Result<(), ScanIoError>`,
and `finish(&mut self, control: &dyn ScanIoControl) -> Result<(), ScanIoError>` for directory-change observation.
The small core trait `ScanIoControl: Send + Sync` exposes only
`checkpoint(&self) -> Result<(), StopReason>`. Task 5 ScanControl implements it;
Task 3 fixtures use a controlled implementation. Every actual syscall checks
before/after through this seam, not only between multi-syscall cursor methods.
RAII close/closedir still run unconditionally after stop to avoid leaking handles.
ScanIoError distinguishes a diagnostic from a cooperative Stopped(StopReason).
`ScanEntry` contains native name, type, identity/mount/cloud facts and nullable
logical/allocated metadata; no contents. ObjectIdentity is a fixed native key,
volume UUID + file ID + birth time, separate from transport strings.

- [x] Write `scan_root_anchor` on temporary roots: saved identity accepted;
      renamed/missing/replaced/denied/broad-root facts map to exact reasons,
      no enumeration on failed validation. Symlink-selected root uses the saved
      canonical object; returning a different identity never follows a new target.
- [x] Write `scan_cursor_no_follow`: controlled external symlink, loop, FIFO,
      broken link, hidden/.git/node_modules directories. Count external-target
      metadata/content opens 0; special-file opens 0; regular-content opens 0.
      A barrier replaces a child between fstatat and openat with a symlink:
      expect changed/skipped and external sentinel unvisited. Another barrier
      renames a directory after open: observed parent mismatch stops its branch;
      test the documented anchored-descriptor limitation, not atomic containment.
- [x] Write `scan_metadata_cases`: sparse fixture length from `set_len`, observed
      blocks distinct from logical size; injected missing/negative allocation
      returns unknown, never zero; a representable blocks count whose ×512
      overflows produces quantity_overflow, not a wrapped byte measurement.
      SF_DATALESS/sync-root and mount
      facts use spec getattrlistat/fstatfs and checked returned masks; unsupported
      attributes fail closed. These are injected deterministic tests; real volume/permissions evidence
      is labelled separately. Byte/Unicode names survive internally; no transport
      failure or lossy path is required for a non-UTF-8 descendant.
- [x] Run `cargo test -p dev-storage-platform scan_ --locked` RED, implement the
      narrow cursor/RAII and factor existing identity decoding without changing
      macos-v1 semantics. Validate attribute masks and FFI buffers; no unsafe
      aligned struct casts, descendants opened as files, xattr/content reads or
      fallback that traverses unsupported cloud/network scope. Rerun GREEN plus
      `cargo test -p dev-storage-platform workspace_ --locked` after extraction.
- [x] Commit `feat: add anchored macOS scan metadata traversal`.

**Exit:** Only temporary fixture calls made; root-only Phase 02 behavior remains.
No command or application path starts the native cursor yet.

Execution receipt: `d125750`, independently reviewed. The verification record
separates native fixtures from injected cloud/mount/home/denial/name facts;
those conditions are not represented as live-volume tests. Session-wide
descriptor enforcement remains Task 5, not an adapter-only claim.

## Task 4: Implement bounded unique-object accounting

**Files:** Core scanner `accounting.rs`, tests therein.

**Interfaces:** `ScanAccounting::new(limits: ScanLimits) -> Result<Self, ScanFailure>`;
`observe(&mut self, owner: WorkspaceId, metadata: &ScanEntry) -> Result<ObservationOutcome, StopReason>`;
`snapshot(&self) -> AggregateMetrics`. Fixed 64-byte slots and caps from spec; no
unbounded collections or loss of dedup at capacity. Owner order is supplied by
engine, not arbitrary worker completion. Grouped diagnostics use a reserved
overflow slot within the total limit of 128.

- [x] Write `scan_accounting_deduplicates`: two root IDs 7 and 9 share a 12-byte
      hard-linked file; exclusive contributions 12/0, global 12 and coveredBy=7,
      not 24. Same inode on different volume UUIDs contributes 24. Reversed input
      selection normalizes before observation; repeated directory has one visit.
- [x] Write `scan_accounting_bounds`: small injected table capacity 2 then third
      unique object produces identity_limit, preserves exact first two totals;
      actual slot size 64 bytes, capacities/bytes equal spec. Addition near
      u64::MAX and blocks ×512 overflow return quantity_overflow without wrapping.
      Failed observation leaves identity/ownership/all metric fields unchanged;
      retrying that identity is not treated as a previously accounted object.
      One unknown allocation among two files preserves logical observations,
      allocation sample count/unknown count and partial allocation coverage.
      Assert logicalCoverage remains complete_within_policy while
      allocatedCoverage/overall coverage are partial; all-unknown allocation is
      null/unavailable, not a zero-byte total.
- [x] Write `scan_diagnostics_bound`: 127 ordinary groups plus reserved stop
      group; the next new group stops diagnostic_limit, repeated code increments
      its decimal count, no path samples appear. A 128 KiB result boundary
      reserves terminal data, returns result_limit, never a truncated complete DTO.
- [x] Run `cargo test -p dev-storage-core scan_accounting --locked` and
      `cargo test -p dev-storage-core scan_diagnostics --locked` RED/GREEN around
      implementation. Commit `feat: account for bounded unique scan observations`.

**Exit:** Metrics are observations, not reclaimability; resource limits cannot
turn duplicate objects into a silently overcounted exact result.

Execution receipt: `eea6a8c`, independently reviewed. Request-order normalization
and ancestor propagation belong to Task 5; full actual serialized-size checks
before publication remain service/IPC integration requirements. The guard alone
does not establish end-to-end payload enforcement.

## Task 5: Drive streaming traversal, coverage and cooperative stop in core

**Files:** Core scanner `engine.rs`, reusable synthetic fixtures in
`crates/core/src/scanner/tests.rs` (cfg(test) only), module exports and the narrow
`ScanSnapshot.entryAttempts` decimal progress field in `model.rs` with its JSON test.

**Interfaces:** `ScanControl` holds cancellation/deadline state and an injected
monotonic clock. `scan(roots: Vec<StoredWorkspace>, mode: ScanMode,
fs: &dyn ScanFilesystem, control: &ScanControl,
publish: &mut dyn FnMut(ScanProgress)) -> ScanReport`. Native cursor follows
Task 3 traits; fake cursor records every operation. Publication is bounded
state replacement, not queueing every entry or exporting paths.
Each issued next_entry call conservatively consumes one attempt, including EOF
or failure; UI labels the counter “Traversal checks attempted”, not objects
measured. A diagnostic abandons that cursor partial without retry; ancestor
siblings/other roots remain eligible. Numeric budgets do not change.

- [x] Write `scan_empty_and_multiple_roots`: empty directory produces measured
      "0"/complete_within_policy; two roots' deterministic 3+5-byte regular files
      total "8". Snapshot configuration with no requested roots is rejected by
      service, not treated as an empty directory. Names .git/node_modules/hidden
      contribute like ordinary metadata. Forbidden content/mutation counters 0.
- [x] Write `scan_nesting_and_order`: parent/child and hard links with IDs
      reversed in request produce identical exclusive attribution and global
      totals; child covered by parent is explicit, not an independent total.
      Disabled/unrequested child is not a parent-tree exclusion. Directory
      enumeration permutations preserve complete stationary totals/ownership.
      A later root deduplicating an earlier partial directory inherits partial
      coverage, including when the first root exhausted its depth budget.
- [x] Write `scan_limits_and_partial`: inject depth 1, entry 2, identity 2, FD
      limit and deadline clocks. Assert exact stop/skip reason, live descriptors
      never exceed cap, no frontier/full-tree growth, unmeasured bytes null for
      unknown subtrees. Known policy exclusions stay counted in declared scope;
      inaccessible/disappeared/changed/cloud/unknown allocation is partial.
- [x] Write `scan_cancel_and_errors`: cancel before next call permits no new
      cursor work, retains observations; a barrier-blocked call cannot report
      completion until released. Mixed valid/invalid roots retain valid metrics;
      no usable root finishes error. Verify parent link before resuming a child,
      directory finish drift, stream iteration failure and panic sanitation path.
- [x] Run `cargo test -p dev-storage-core scan_ --locked` RED/GREEN; implement
      one-root-at-a-time anchoring and streaming DFS. Close each root/cursor stack
      before opening the next root so 32 roots never add 32 retained root FDs.
      Commit `feat: traverse bounded scan scopes with honest coverage`.

**Exit:** Engine is independently testable with synthetic filesystem operations;
native implementation still not exposed through IPC.

Execution receipt: `6294d8a` plus reviewed correction `4c6da52`. Nested directory
deduplication now retains allocation uncertainty independently from logical
coverage. The verification record distinguishes the initial workspace-suite run
from focused checks on the amended source. Service/IPC size and lifecycle gates
remain Tasks 6–7; no production native runner is connected by this checkpoint.

## Task 6: Coordinate owned sessions and configuration changes in application

**Files:** Application `activity.rs`, `scans.rs`, `lib.rs`, `workspaces.rs`;
add bounded `Database::scan_roots_by_ids(&[WorkspaceId]) -> Result<Vec<StoredWorkspace>, WorkspaceFailure>`
in existing persistence `workspaces.rs`, with covering tests. Core workspace
error enum/message adds ScanInProgress; no migrations. No new production crates.

**Interfaces:** `ActivityGate::try_workspace_change(&self) -> Result<WorkspaceChangeGuard, WorkspaceFailure>`;
`try_scan(&self) -> Result<ScanReservation, ScanFailure>`. `ApplicationService` owns
gate and `ScanCoordinator`; `start_scan(&self, request: ScanRequest) -> Result<ScanSnapshot, ScanFailure>`,
`current_scan(&self) -> ScanStatus`, `scan_session(&self, id: &ScanSessionId) -> Result<ScanSnapshot, ScanFailure>`,
`cancel_scan(&self, id: &ScanSessionId) -> Result<ScanSnapshot, ScanFailure>`.
Tauri calls start through short existing blocking dispatch; the service starts
one dedicated std thread which owns reservation/cursors until actual I/O stops.
`reserve_workspace_change(&self) -> Result<WorkspaceChangeGuard, WorkspaceFailure>` supports picker lifetime;
`commit_workspace_add(&self, guard: WorkspaceChangeGuard, root: ValidatedWorkspace, now: i64) -> Result<WorkspaceInsertOutcome, WorkspaceFailure>`
commits under that guard, preserving the existing insert outcome. Keep ordinary Rust `add_workspace` as a gated wrapper;
Remove/Enabled/Skip acquire the same gate internally. No lock spans picker/I/O.
`ScanRunner: Send + Sync` supplies
`run(&self, roots: Vec<StoredWorkspace>, mode: ScanMode, control: &ScanControl, publish: &mut dyn FnMut(ScanProgress)) -> ScanReport`.
`ApplicationService::new` preserves existing behavior with no runner; scan start
returns application_unavailable. Add
`new_with_scan_runner(adapter: A, database: Database, runner: Arc<dyn ScanRunner>) -> Self`
for Rust dependency injection. Counted runners stay in tests; Task 8 alone wires
the real runner into production. No frontend-injectable testing switch.

- [ ] Write `scan_scope_resolves_ids`: zero/duplicate/33 IDs, unknown, disabled,
      oversized path snapshot reject before adapter calls. Query returns only
      requested stored rows sorted by numeric ID; missing IDs do not broaden.
      SQLite read failure starts no worker. Existing schema 2/metadata unchanged.
      Include quotes/control characters in stored fixture paths: JSON-encoded
      descriptors at 32 KiB fit, one byte over rejects before work. No runner
      returns application_unavailable; failed thread spawn releases its guard.
- [ ] Write `scan_activity_serializes`: held picker/change guard blocks start;
      scan validation/running/stopping/cancelling blocks every workspace write
      with scan_in_progress; root checks/list/health still respond. Guard movement
      through callback, dropped start/picker response and commit retains ownership.
      No reentrant add-gate deadlock. Barrier-blocked worker keeps the reservation
      after cancel until released; all mutation counters stay zero meanwhile.
- [ ] Write `scan_idempotent_start`: same request/payload returns same session
      after dropped receiver; adapter starts once.
      Replay with its original accepted expectedEpoch succeeds before busy/gate
      checks while that same session is validating/running/cancelling/stopping.
      It starts no worker and never revalidates its roots. Changed payload gives
      request_conflict; different request while live gives scan_busy. After a
      replacement session, evicted old request/epoch gives stale_epoch. Restart
      invalidates old epochs, has no active/retained result and starts nothing.
- [ ] Write `scan_lifecycle`: injected barriers/clocks cover validating→running,
      no-valid-root error, complete/partial, cancel during validation/work, budget
      stopping, finalize/cancel ordering and panic. Acknowledged cancellation has
      workerActive true until actual stop; only then terminal/guard reusable.
      While slow native I/O waits, health/list query completes without DB lock.
      On shutdown request stop/no publication, no automatic persisted restart.
- [ ] Run `cargo test -p dev-storage-application scan_ --locked` and
      `cargo test -p dev-storage-persistence scan_ --locked` RED/GREEN; rerun
      application workspace regression tests for the guarded mutations. Commit
      `feat: coordinate isolated scan sessions and workspace ownership`.

**Exit:** Race-safe service and retained-worker model; schema/DB location intact.
Native app integration stays gated on Task 1 ACL evidence.

## Task 7: Register narrow scan commands and prove native authorization first

**Files:** New `src-tauri/src/scans.rs`; modify shared `registration.rs`,
`build.rs`, `capabilities/default.json`, `lib.rs`; generated permissions for the
four commands; Task 1 native ACL cases extended. Workspace command picker passes
its new application change guard through selection/commit. No generic grants.

**Interfaces:** Four exact spec commands wrapping Task 6 methods; current returns
ScanStatus (epoch + nullable session), start/query/cancel return ScanSnapshot.
Injected real Webview/parent guard precedes service work. `application_handler`
and `AppManifest::commands` now contain all eleven identically in app and target.
The test target initially uses a counted synthetic ScanRunner rather than a native
scanner; production runner is not wired until Task 8's exit gate.

- [ ] Write `scan_ipc_contract`: exact start request nesting, mode/ID/epoch
      parsing and serialized quantities. Reject JS path, options and budget
      fields with deny_unknown_fields on ScanRequest; no arbitrary roots. Same
      request replay, stale epoch, cancel acknowledgement and session_not_found
      are typed, sanitized and authoritative. Preserve health DTO/schema 2.
- [ ] Extend generated-context MockRuntime tests: all four new commands denied
      for ungranted caller/origin before handler; no file/picker/DB/scan calls.
      Actual-parent guard independently tested; local-main fixture paths succeed.
- [ ] Run `cargo test -p dev-storage-manager scan_ --locked` RED/GREEN, then
      `cargo check --workspace --all-targets --locked`. Inspect generated
      permissions: four narrow allow entries only, local main webview, no window
      OR grant/remote generic access. No product test handler or public injection.
- [ ] Rebuild frontend assets if changed, and run the native target with
      `--case scanner-acl` using all eleven commands and synthetic scanner
      counters. Require real remote-main/child/other-window denial before work;
      missing transport/timeouts are not PASS. Commit
      `feat: expose constrained scan session commands` only with this boundary
      reviewed. Record native evidence against the exact tested target/commit.

**Exit:** Both inherited and new-command native ACL evidence exists; a test stub
is never presented as a production-complete scanner or shipped fake success.

## Task 8: Wire the native engine without expanding its authority

**Files:** Application scans/native factory wiring, Tauri `lib.rs` setup and
test-target scan cases; no new commands, capabilities or persistence schema.

**Interfaces:** In `crates/application/src/scans.rs`,
`MetadataScanRunner::new(filesystem: Box<dyn ScanFilesystem>) -> Self` implements
Task 6 ScanRunner by invoking Task 5 `scan`. Production setup consumes
`MacOsAdapter.scan_filesystem()` and `ApplicationService::new_with_scan_runner`;
the native target can supply controlled runners through that same constructor.
There is no additional IPC, test-only product branch or alternate handler list.

- [ ] Write `scan_native_integration`: seed temporary schema-2 roots and a
      sentinel, explicit native start yields genuine metadata metrics, no start
      on load/list/Add/Enabled/check/restart. Cancel reaches stopping then terminal
      only after work stops; root filesystem/sentinel unchanged. Permit/gate and
      scalar snapshots tested under a deliberately slow synthetic adapter too.
- [ ] Verify Task 1 P02-NATIVE-ACL and Task 7 scanner-acl receipts before wiring
      real `MacOsAdapter.scan_filesystem` into production setup. If either is
      unresolved stop this integration step, not silently downgrade it to mock QA.
- [ ] Implement wiring, run focused application/Tauri `scan_native_integration`
      tests and native `--case scan-smoke` with temporary roots. Inspect scope:
      directory/metadata only, no read-content method or executable call; no DB
      scan rows created, schema remains 2. Commit `feat: integrate read-only native workspace scanning`.

**Exit:** Real engine accessible only by authorized explicit start; no providers
or Phase 05 UI added. Record test-target vs normal-app evidence separately.

## Task 9: Add typed frontend scan transport and resilient polling state

**Files:** `src/lib/scans.ts`, `scans.test.ts`,
`src/features/scans/useWorkspaceScan.ts`, `useWorkspaceScan.test.tsx`; App hook
ownership wiring in `src/App.tsx`. Preserve useWorkspaces and health guards.

**Interfaces:** `startWorkspaceScan(request, invokeFn?)`, `getWorkspaceScan(invokeFn?)`,
`getWorkspaceScanSession(sessionId, invokeFn?)`, `cancelWorkspaceScan(sessionId, invokeFn?)`.
`useWorkspaceScan()` exposes `{status, snapshot, error, start(ids, mode), cancel(), reconnect()}`;
status includes reconciling/ready/unavailable with no invented backend terminal.
UUID-format requestId from crypto.randomUUID; authoritative epoch fetched before
start. BigInt byte formatting lives in `src/lib/scanFormatting.ts` with tests.

- [ ] Write exact transport spies for four commands/args, no paths or Number
      coercion; parse 9,007,199,254,740,993 bytes unchanged, 2^30 displays 1 GiB,
      null displays Not available, zero displays 0 B. Preserve exact-byte label.
- [ ] Hook tests use fake timers/deferred promises: one status query in flight,
      250 ms cadence only while a visible live session exists, pause on hide,
      reconcile once on show. Navigation/unmount does not send cancel/start;
      initial status fetch never causes filesystem work. No idle infinite polling.
- [ ] Assert response loss keeps request ID for reconciliation/replay, does not
      start another request; later low-revision/old-epoch/session results ignored,
      StrictMode stale responses ignored, query failure leaves unknown status
      with Reconnect and never claims rollback/completion. Terminal stops queries.
- [ ] Run `npm run test:run -- src/lib/scans.test.ts src/lib/scanFormatting.test.ts src/features/scans/useWorkspaceScan.test.tsx`
      RED/GREEN, implement narrow transport/controller. Commit
      `feat: manage scan session recovery in the frontend`.

**Exit:** Presentation can observe state without inventing work or counts.

## Task 10: Present minimal scan intent and truthful results in existing views

**Files:** `src/features/scans/ScanSection.tsx`, `ScanSection.css`, tests;
modify `src/views/WorkspacesView.tsx`, `OverviewView.tsx`, their tests and App
integration tests; existing WorkspaceRow and useWorkspaces error mapping as
needed for additive scan_in_progress. Keep AppShell/sidebar/tokens unchanged.

**Interfaces:** `ScanSection({scan, enabledIds, configurationBusy})` consumes
the controller, not IPC. Overview gets optional scan summary; Workspaces owns
Quick/Deep/Cancel controls and frozen-write explanation. Existing row availability
and exact configuration-removal safety copy/focus lifecycle stay independent.

- [ ] Write UI tests for zero enabled roots, loading/reconnect/error, validating,
      running without fake percentage, cancelling/stopping with live worker,
      complete_within_policy/partial/cancelled and missing allocation. Assert
      exact spec copy, GiB/exact bytes, policy exclusions, covered-by attribution,
      no reclaimable-value label, cleanup action, fabricated metric or extra route.
      The explicit “not reclaimable space” disclaimer remains visible.
      Assert distinct spec copy for traversal limits, inaccessible/changed roots,
      allocation-only uncertainty, cancellation and actual policy exclusions.
      coveredBy with zero contribution shows accounted elsewhere, not empty.
      More than 32 enabled disables starts with the actionable message, leaves
      all saved/enabled rows intact and invokes no subset; Deep says it has limits.
      Counter-only updates do not trigger repeated live-region announcements.
- [ ] Assert Start fires once on double activation with captured IDs/mode; Deep
      is an explicit separate intent; nothing starts on render/navigation/config
      change. Cancel remains available until worker stop; no new Start or workspace
      mutation meanwhile. Native picker already open blocks Start consistently.
      Late results do not steal heading/Remove focus or overwrite newer sessions.
- [ ] Run `npm run test:run -- src/features/scans/ScanSection.test.tsx src/views/WorkspacesView.test.tsx src/App.test.tsx`
      RED/GREEN. Reuse existing primitives, live-region semantics and full-width
      layout; keep text wrapping/scroll access, no hidden focusable overflow.
- [ ] Run frontend suite, lint and build once for the coherent integration.
      Commit `feat: present explicit workspace scan controls and partial results`.

**Exit:** Minimal functional integration only. Phase 03 human visual acceptance
and inherited layout/zoom are required at Task 11; Phase 02 approval is retained.

## Task 11: Verify functional/native boundaries and request Phase 03 acceptance

**Files:** Implementation verification record created in Task 1, this plan,
architecture/roadmap current status; update P02 tracked checks only with actual
new evidence, preserving every historical label. Fix only demonstrated defects.

**Interfaces:** Consumes the Task 1 `run_case` executable and Tasks 7–10 contracts,
produces evidence indexed by tested commit/artifact and gate, not new product APIs.

- [ ] After focused work, run once: `npm run format:check`, `npm run lint`,
      `npm run test:run`, `npm run build` (includes tsc -b),
      `cargo fmt --all --check`,
      `cargo clippy --workspace --all-targets --locked -- -D warnings`,
      `cargo test --workspace --locked`, `npm run tauri build -- --debug`,
      `git diff --check`. Capture exact exits/counts, not expected totals.
- [ ] Rerun native inherited-acl/scanner-acl against current shared registration,
      plus scan-smoke/scan-ui on temporary data. Current native command matrix
      must reject before handlers/picker/DB/scanner. Normal app bundle smoke uses
      only a deliberately selected temporary fixture, never personal workspaces.
      Identify hashes/commits/artifact type; test target is not normal bundle QA.
      Record controlled fixture entry counts, mode, measured duration and actual
      termination reason; do not extrapolate real-repository performance.
- [ ] Run native exact-size/light-dark/zoom cases for both views and new controls,
      max window, full long paths, empty/partial/stopping/complete and dialog access.
      Include the empty Workspaces state at all three exact sizes/appearances,
      not only Overview empty and populated Workspaces from Task 1.
      Check keyboard, focus, reader announcements, contrast/reduced motion with
      explicit permission for any global change. Record native vs browser vs
      tests vs human; no screenshot pixel inference. Restore native test controls.
      Close P02 layout/zoom only if their full criteria pass, before accepting UI.
- [ ] Inspect whole diff from `04018df`: no contents/user-file writes, recursive
      paths outside descriptor scope, networking in product, providers, history,
      new generic grants, stack upgrades or local inventories. Recheck normal
      app/DMG exclusion of harness even with opt-in feature/all-features main
      compilation. Sentinel bytes remain intact. Confirm schema/identifier stay.
- [ ] Request human inspection of **new Phase 03 UI**, not another approval of
      Workspaces. Without it record HUMAN VISUAL QA: AWAITING HUMAN REVIEW.
      No inherited temporary exception automatically extends to Phase 03; do not
      accept this UI phase with P02 layout/zoom unresolved. Record blockers honestly.
- [ ] Commit scoped evidence `docs: record scanner core verification`; publish
      only if the current execution task authorizes this feature branch, verify
      remote HEAD and clean checkout. No automatic PR/merge/release.

**Exit:** Evidence-backed Phase 03 handoff, not a release; unresolved gates remain
open. No later provider/dashboard/cleanup feature is executed by this plan.

## Design-run evidence and self-review

This section describes preparation, not execution of Tasks 1–11. Main was fetched
and fast-forwarded to `04018df`, containing merge #3 and the Phase 02 acceptance;
its tree matched `5e7dff9`, independently of ancestry assumptions. Branch created
from that clean main. Required docs/code and locked upstream API source were
read; official Apple/Rust/Tokio/Tauri references are linked in the spec.

Self-review maps request scope/config concurrency to Task 6, identity/race/cloud
to Task 3, dedup/quantities to Task 4, limits/partial/cancel to Tasks 5–6,
authorization/transport to Tasks 1/7–9 and minimal UI/accessibility to Tasks 10–11.
The four checkpoints and three P02 gates remain separate. No implementation
checkbox is marked by preparing these documents. No tests/harness/native scan,
dependency resolution, SQL change, OS setting or real workspace access is part
of this run. Review both spec and plan before choosing/authorizing execution.

Documentary verification: scoped Prettier formatting; `npm run format:check`
PASS; `git diff --check` PASS; Node inspection of four documents found 24 local
Markdown links/anchors valid, 11 sequential tasks, no checked implementation
boxes and no unfinished plan markers. Inspected the complete staged scope before
commit; only the two new documents and necessary architecture/roadmap edits are
permitted. No functional or native suite was rerun as documentary evidence.
