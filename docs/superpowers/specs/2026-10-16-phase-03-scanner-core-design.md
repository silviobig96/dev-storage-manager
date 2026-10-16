# Phase 03 — Read-only Scanner Core Design

**Status:** Approved for implementation on 2026-10-16 with the review
clarifications below. Approval is authority to implement, not functional/native
evidence or acceptance of the new UI. The implementation record tracks actual
progress and remaining gates.
**Date:** 2026-10-16. **Base:** `04018df8d152268cebd2f62aa554ddfa49c30d7a`
(merged Phase 02). **Branch:** `feat/phase-03-scanner-core`.
**Plan:** [Implementation and verification plan](../plans/2026-10-16-phase-03-scanner-core.md).

## Intent, inherited requirements and proposals

Measure explicitly requested workspace scope using directory enumeration and
metadata, with bounded work, cancellation and honest coverage. This is the
scanner foundation for Phase 04, not project discovery or the Phase 05 dashboard.

Already approved: optional 0..N chosen workspaces, no default root, local-first
operation, no automatic deletion, unchanged macOS visual language, and Phase 02
[development acceptance](../reviews/2026-10-16-phase-02-workspaces-verification.md#human-acceptance-decision--2026-10-16).
Workspaces visual approval is not reopened. Release acceptance was not granted.

Inherited implementation: `WorkspaceId`, `StoredWorkspace`, opaque `macos-v1`
directory identity, enabled configuration, `ApplicationService<A>`, schema-2
`Database`, macOS adapter, seven manifest-declared commands with main-webview
capability and actual-parent guard, and race-safe React feature controllers.
Availability is root metadata only, not a scan. Its existing contract stays.

**Accepted for this implementation:** one session/one traversal worker; fixed Quick
and Deep budgets; frozen configuration during live work; descriptor-relative
macOS traversal; deterministic exclusive attribution; polling bounded snapshots;
memory-only results; the isolated native test target; and minimal scan controls
in existing views. The implementation assignment accepted these decisions;
this is not evidence that implementation or verification has passed.

## Alternatives and recommendation

| Concern     | Alternatives                                                                       | Recommendation                                                                                                                                          |
| ----------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Traversal   | Recursive `std::fs` paths; parallel walker dependency; descriptor-relative adapter | Descriptor-relative streaming DFS. More adapter code, but explicit no-follow/race/mount rules; no new walker package.                                   |
| Parallelism | Worker pool and a frontier queue; one worker                                       | One dedicated traversal thread. Predictable descriptor/memory accounting and ownership; no throughput promise.                                          |
| Progress    | Broadcast events; channels; status queries                                         | Main-authorized snapshot queries every 250 ms only during the explicitly started session. No event fanout, subscriptions, extra plugin or replay queue. |
| Native QA   | Repeated JS window/zoom attempts; product debug mode; separate test executable     | Explicit Cargo integration-test executable using real Wry. Native Rust controls, intact production ACL, no distributed testing mode.                    |

Rust documents filesystem enumeration as unordered and fallible during iteration;
do not infer a stable traversal order or total entry count from it.
[Rust directory iteration](https://doc.rust-lang.org/std/fs/fn.read_dir.html).
The chosen adapter does not use recursive path-based `read_dir` as its boundary.

## Inherited native verification gates

At design approval all were **OPEN — DEFERRED WITH HUMAN APPROVAL**. New
[Task 1 evidence](../reviews/2026-10-16-phase-03-scanner-core-verification.md#checkpoint-1--native-acceptance-target)
resolves inherited ACL and zoom; layout remains open for empty Workspaces/current
UI before acceptance. Historical measurements below remain unchanged; no native
PASS was granted by the documentary run itself. The binding limits are:

- [P02-NATIVE-ACL](../reviews/2026-10-16-phase-02-workspaces-verification.md#p02-native-acl):
  resolve before integrating the real scanner. The first implementation checkpoint
  creates and exercises the narrow native environment; synthetic core work may
  proceed independently, but no native scanner command is wired until this passes.
- [P02-NATIVE-LAYOUT](../reviews/2026-10-16-phase-02-workspaces-verification.md#p02-native-layout)
  and [P02-NATIVE-ZOOM](../reviews/2026-10-16-phase-02-workspaces-verification.md#p02-native-zoom):
  resolve before accepting the next UI-modifying phase, here proposed Phase 03.
  Keep the actual 1728 × 969 light/dark and 760 × 640 dark evidence distinct
  from the missing exact 760 × 520 / 1100 × 720 / 1600 × 900 native matrix.

Neither this plan nor future scanner tests silently close those obligations.
New scanner commands additionally require their own real native denial matrix
before their native engine is enabled at the integration checkpoint.

## Isolated native acceptance target

### Structure and exclusion from application bundles

Propose `src-tauri/tests/native_acceptance.rs` with `harness = false` and
`required-features = ["native-acceptance"]` in `[[test]]`. Add an empty opt-in
feature, not a product runtime flag. Test code exists only under `tests/`; normal
`src/main.rs`/library must never include it, even with all features enabled.
Cargo test targets have a separate executable and support a custom main loop.
[Cargo target rules](https://doc.rust-lang.org/cargo/reference/cargo-targets.html).

Extract only the existing generic context/handler constructors to
`src-tauri/src/registration.rs`. Production `lib.rs` and the test target compile
that same file and the same command modules (test target path modules), rather
than maintain a second registry. No QA handler or extra command is registered.
Existing test call sites retain small crate-local reexports. Context includes the
same `build.rs` generated AppManifest, default capability and explicit capability
selection. Harness setup alone supplies a temporary Database and controlled roots;
it never calls normal `run()` or resolves the real app-data database.

Keep native helpers, counters, fixture HTML and report collection in
`src-tauri/tests/native_acceptance/`. Use a separate target output directory for
the acceptance executable. No extra `[[bin]]`, bundle resource, entitlement,
production initialization script or `debug_assertions`-guarded backdoor.

Proposed test-only dependency edges: `tempfile = "=3.27.0"` and macOS
`objc2-web-kit = "=0.3.2"`, `default-features = false`, features `std,WKWebView`.
Both versions already exist in Cargo.lock; preserve all versions and inspect
feature closure/lock edges. Existing Tauri dev `test,unstable` supports child
webviews. The WebKit binding is only for reading actual `pageZoom` from the native
object. No new production dependency, JS package, UI automation framework, Swift,
OS permission, or generic JS grant is proposed. The implementation assignment
explicitly approved these dev edges and the test-only loopback fixture below.
It binds only 127.0.0.1:0, serves controlled HTML only, makes no Internet/proxy
requests and closes when its case ends. This authority never extends to the app.

Verify exclusion using Cargo metadata (only a `test` target), normal/build feature
tree, normal app/DMG builds without the opt-in feature, and a normal main-bin
build with all features. Inspect app/DMG contents and normal executable for the
distinct `DSM_NATIVE_ACCEPTANCE_ONLY_V1` marker/report endpoints; require none.
Do not confuse the already-transitive WebKit crate with QA code inclusion.

### Real caller/origin rejection, not creation denial

Run the Wry event loop on the process main thread. Rust creates test callers:
local `main` in `main`; ungranted child in `main`; another window; and a `main`
webview in the wrong containing window in an isolated sequential case. Native
creation needs no JS create-webview grant.

For origin isolation, navigate the actual `main` caller in its `main` window to
a controlled HTML document served by a test-only `std::net::TcpListener` bound to
`127.0.0.1:0`. Its actual URL is external and ungranted. No Internet access,
production server or remote capability is added. This **test-only loopback
network exception** is proposed for review; app networking remains absent.

From each actual loaded caller, evaluate a JS invocation using the normal Tauri
transport, catching rejection into a page-local result. Read that result via
`eval_with_callback`, with a five-second deadline; a missing bridge, navigation
failure or timeout is inconclusive/FAIL, never ACL PASS. Do not forge an origin
field in a Rust InvokeRequest or test only a label guard. Unauthorized cases
must reject with ACL denial before the wrapper around the real application
handler is entered. Require handler count 0, no picker, identical fixture DB
state and zero scanner work. The wrong-parent case may enter its explicit guard
but must leave service/picker/scanner untouched. Test allowed main health/list
and valid fixture mutations separately as positive controls.

At the first checkpoint test the existing seven commands. At integration test
all eleven (the same four new commands below); use only seeded fixture IDs and
valid-shaped requests. Close all test contexts and restore per-window controls.
The inability to create a caller is not successful rejection of its invocation.

### Exact native geometry, appearance and zoom

Use Rust window `set_size`, `set_theme(Some(Light/Dark))` and webview
`set_zoom(1.0/2.0)`, not their denied JS commands. Locked Tauri 2.11.5 provides
these APIs; the installed Wry 0.55.1 implementation calls WK `setPageZoom`.
Read the getter through `with_webview` and the dev-only WebKit binding on the
main thread. The locked source also provides `eval_with_callback` for JSON
geometry without adding a reporting IPC command.
[Pinned Tauri source](https://github.com/tauri-apps/tauri/blob/tauri-v2.11.5/crates/tauri/src/webview/mod.rs),
[Apple pageZoom](https://developer.apple.com/documentation/webkit/wkwebview/pagezoom).

At zoom 1, request each CSS size; measure `innerWidth/innerHeight` inside actual
WKWebView. Compensate for the measured titlebar delta using at most two size
corrections, failing explicitly if exact dimensions remain unavailable. Do not
assume a 32 px titlebar, a particular display scale, or enough screen area.
At zoom 2 verify native getter equals 2.0 and record the resulting CSS viewport
separately; do not enlarge the window just to hide pressure. Exercise all long
paths/actions, scrolling and modal states with native keyboard/pointer use.
Use real production frontend assets built by Vite; all rows come from temporary
SQLite fixtures, not a shipped mock mode. Per-window native theme must propagate
to `prefers-color-scheme`; no global preference change is necessary. This is
native test-executable evidence, not inspection of the normally bundled binary;
record a short normal-bundle smoke separately before handoff.
Run Cargo acceptance tests with both `native-acceptance` and
`tauri/custom-protocol` features. The locked context macro otherwise chooses the
configured development URL and does not embed frontendDist; a successful Vite
build alone is insufficient. The custom-protocol feature selects the normal
bundled asset path, not a QA protocol or an extra capability.

## Request scope and concurrent workspace configuration

`start_workspace_scan` takes explicit workspace IDs and mode, never paths,
arbitrary exclusions, worker counts or caller-chosen budgets. Requests contain
1–32 distinct IDs; zero, duplicates, unknown or disabled IDs reject the entire
request before filesystem work. With zero enabled roots UI offers configuration,
not a fake successful zero-byte scan. The UI's default is all enabled roots,
captured at click time; an individual workspace can be requested by its ID.
The 32-root cap belongs only to a scan request, never to saved configuration.
If more than 32 are enabled, disable both scan-start controls and explain:
“A scan supports up to 32 enabled workspaces. Disable some workspaces to scan.”
Do not pick a prefix/subset, change enabled or add a subset selector. The backend
also rejects an oversized request before filesystem work.

Resolve only those IDs through a bounded Database query using the existing
repository. Store an owned session snapshot sorted by numeric WorkspaceId;
retain selected/canonical paths and expected identities internally. Bound the
JSON-encoded root descriptors (IDs, display names, selected/canonical paths and
identity strings, including JSON escaping) to 32 KiB before accepting start;
reject `scope_too_large`, never silently truncate. This leaves space within the
128 KiB snapshot ceiling for metrics and terminal diagnostics.
No schema change. Missing/denied/changed roots discovered during validation become
per-root unavailable results; valid roots continue. If none validates, finish
`error` with root diagnostics, no zero-total success. Reject volume roots and the
current account home root as `root_too_broad` (volume-root identity and current
account home obtained through `getpwuid_r`, not an environment variable or
project-location convention); preserve configuration and ask for a narrower root.
That platform guard may obtain one metadata-only reference to the OS account
home to compare descriptor identity; it never enumerates home, reads contents,
uses a JS path or publishes that guard path. Filesystem tests inject controlled
temporary home facts. F_GETPATH/string equality alone cannot prove the broad-root
identity check for case/firmlink/symlink aliases.
Do not traverse a whole Mac or broaden to global providers.

Propose one application-owned `ActivityGate`: idle, workspace change reserved,
or scan reserved. All workspace writes (Add including its picker, Remove,
Enabled, Skip) reserve a short-lived change guard, or reject `scan_in_progress`
while a scan is validating/running/stopping/cancelling. Start rejects while a
picker/commit/change is active. Reserve before reading the root snapshot, then
release the gate mutex; never hold that mutex or SQLite lock across native I/O.
An add guard survives callback/receiver loss through commit as the selector gate
already does. Add's validation workers still cannot commit after their deadline.

Keep health, list and manual root-availability checks available. Preserve the
existing two root-check permits/three-second timeout separately; one scanner
thread adds at most one filesystem worker. Block writes also at service entry,
not just by disabling UI. Scope changes require cancel and **actual worker stop**
first; cancellation acknowledgement alone never unfreezes configuration. This
simple freeze prevents removed roots, changed enabled states or newly added
folders from silently changing an active session. No DB lock spans traversal.
Add only `ScanInProgress` / `scan_in_progress` to the existing workspace failure
contract; preserve every existing DTO/error and map this new outcome explicitly.

No scan starts on app load, navigation, Add, Enabled, availability check, result
query, restart or cancellation recovery. There is no queued automatic rerun.

## Read-only traversal and race boundary

Core defines `ScanFilesystem: Send + Sync` / `ScanDirectory: Send` traits for a streaming cursor,
root anchoring, metadata entries and verified child-directory opens. Their API
has no read-content, write, unlink, chmod, shell or tool-execution method.
`OsString` names stay internal; no path is split on separators or lowercased.
I/O methods receive the small `ScanIoControl::checkpoint` interface so the
adapter checks cancellation/deadline before and after each native syscall,
including calls within one cursor method. Task 5 ScanControl implements that
interface; stopped work remains distinct from a recoverable metadata diagnostic.
Cleanup close/closedir remains unconditional after cancellation; checkpoints must
never prevent releasing descriptors or publish terminal before that cleanup ends.

The macOS implementation reuses the descriptor identity decoding from Phase 02,
factored into `macos/identity.rs` without changing its wire identity. Validate the
chosen root against its saved identity; open its canonical directory with
no-follow semantics and compare that descriptor's identity before enumeration.
Root symlink selection remains accepted only through this saved/revalidated
canonical target, not by following it repeatedly during walking.
Validate/anchor each root immediately before its turn, not all roots up front;
close its complete cursor stack before anchoring the next root. This keeps both
revalidation and the descriptor budget meaningful across up to 32 selected roots.

Stream directory names with `fdopendir`/`readdir` on an owned descriptor, ignoring
only `.` and `..`. For each single component use `fstatat(...,AT_SYMLINK_NOFOLLOW)`;
never concatenate a descendant absolute path for I/O. Child directories open
with `openat(parent_fd,name,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC)`, then compare
opened type/identity with the observed entry and reject observed replacement.
Enumerate only directories; never open regular contents or special files.
Darwin no-follow flags and descriptor ownership belong only in platform code.
[Apple open flags](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html),
[Darwin definitions](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/fcntl.h).

This prevents symlink substitution redirecting opens into an external tree.
An open directory descriptor is not a frozen pathname: a directory can be moved
after being anchored. Authorization here is the root object and descendant
objects acquired through it, not an atomic snapshot of current path membership.
Check active directory identity and its observed parent link before continuing
a cursor and after child open; observed move/replacement stops that branch with
`changed_during_scan`. A rename between checks cannot be made impossible by these
APIs. Already anchored descriptors may outlive it briefly. State this limitation;
do not claim hostile-filesystem isolation, transactional containment, or that
`canonicalize + starts_with` proves race safety. No fallback follows new paths
to find moved objects, and this is never authority for future cleanup.

| Entry/condition                                           | Policy and accounting                                                                                                                                                                                                                                   |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Descendant symbolic link, including cycles/outside target | Never follow/readlink; count policy exclusion, no target I/O.                                                                                                                                                                                           |
| Hard link / repeated directory identity                   | Deduplicate across the session; count repeated references separately, do not re-enumerate a known directory.                                                                                                                                            |
| Selected nested roots                                     | Traverse in numeric ID order, share identity set; later owners report covered-by IDs rather than independent additive totals.                                                                                                                           |
| Mount under a root                                        | Do not cross device/mount boundary, including same-volume mount-root facts when reported. Mark exclusion; if boundary identity is inconclusive, skip partial. An explicitly selected mounted root runs separately, never inherits a parent's traversal. |
| Separate selected volumes                                 | Independently anchored; UUID is part of object identity. No cross-volume inode collision. Unsupported identity skips that root.                                                                                                                         |
| Changed/disappearing entry                                | Keep observations already made, mark partial; do not retry a replacement or rescan automatically. Check directory change metadata before/after its cursor; unchanged timestamps are not a snapshot guarantee.                                           |
| Permission denied                                         | Skip branch, retain other results, group diagnostic; no permission changes or FDA prompt.                                                                                                                                                               |
| Device/socket/FIFO/other special                          | Metadata only; exclude, never open it.                                                                                                                                                                                                                  |
| Cloud dataless file                                       | Metadata only; logical length may be observed, allocated observation unavailable; no content request.                                                                                                                                                   |
| Cloud dataless directory / sync root                      | Do not enumerate; partial `cloud_not_materialized` / `cloud_scope_unsupported`. No hydration or provider APIs.                                                                                                                                          |
| Non-UTF-8 descendant name                                 | Native OsString/descriptor traversal is valid; no lossy path crosses IPC. Count `unrepresentable_name` without a sample path; not alone a size failure. Root encoding rules from Phase 02 remain.                                                       |

macOS inspects `st_flags & SF_DATALESS` and extended `EF_IS_SYNC_ROOT` from
`ATTR_CMNEXT_EXT_FLAGS`, checking returned-attribute masks. Query child metadata
with `getattrlistat(parent_fd, single_name, ..., FSOPT_NOFOLLOW | FSOPT_ATTR_CMN_EXTENDED)`;
no regular-file content handle or xattr read is needed. Use descriptor `fstatfs`
facts (`MNT_LOCAL`, device and mount identity) before directory enumeration;
unknown cloud/mount capability fails closed for that branch. These declarations
were checked in the installed Apple SDK and locked libc 0.2.189 binding; the
future adapter must test actual support, not assume SDK presence means volume
support. Metadata calls are not a promise that
third-party filesystems never perform their own I/O. Support initially covers
local volumes with usable identity/mount/cloud metadata; unsupported/network
filesystems produce explicit partial/unavailable scope rather than a hidden
generic fallback. `.git`, `node_modules`, manifests and hidden folders are **not**
excluded by name; their metadata counts without reading their contents or
identifying projects. [Apple metadata and supported attributes](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getattrlist.2.html),
[Darwin dataless/sync flags](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/stat.h).

## Minimal protected policy, distinct from read scope

Reading metadata in an allowed scope and offering cleanup are different grants.
Core adds `SafetyLevel` and `ScanReadPolicy::evaluate(facts: &ReadFacts) -> ReadDecision`;
`ReadDecision` is MeasureMetadata, EnumerateDirectory or Skip(reason), unrelated
to a deletion action. The scan's `MeasurementProtection` is always Protected,
version `scanner-measurement-v1`, allowed actions empty, reclaimability Unknown.
It explicitly protects all measured data in this phase, including roots, source,
`.git`, manifests/lockfiles and unclassified entries, without detecting providers
or inventing Safe/Review categories. No target-level finding or cleanup plan is
created. Test that neither size, mode, enabled nor a forged provider/UI claim
can lower this protection. Future provider policy must refine facts under the
existing safety model, not reinterpret this measurement as deletion consent.

## Size meaning, attribution and coverage

Count regular-file logical length from metadata; directories, symlinks and
special objects have entry counters, not regular-file logical bytes. Allocated
bytes are the sum of available regular-file `st_blocks × 512` observations,
not exclusive physical usage. No allocation fallback to logical size. Missing
allocation is null when no observation exists, not zero; report measured/missing
file counts to describe partial allocation coverage. Zero is valid for an
actually observed empty set or an observed empty file.
For partially measured allocation, the non-null number is only the subtotal of
measured objects, accompanied by partial coverage; it is never the missing total.

Sparse files can have large logical length and few blocks. Compression and APFS
clones/shared blocks mean per-object allocation may be shared or filesystem-
dependent. No sum here means reclaimable space, exclusive APFS blocks, Finder
parity or filesystem capacity; no clone/content hashing. Counts include only
observations made before terminal stop and are not transactional disk snapshots.

Identity key includes volume UUID, file ID and birth time. Attribute each unique
object's contribution exclusively to the first requesting root in sorted numeric
WorkspaceId order. One worker processes roots in that order, regardless of UI
selection order. A nested or hard-linked object already covered by another root
adds no bytes twice; report `coveredByWorkspaceIds` and reference counts. Global
total equals checked sum of exclusive workspace contributions, not sum of
independently measured trees. A disabled nested workspace is not an exclusion
from an enabled parent; an unrequested child likewise has no exclusion effect.
Partial prefixes can vary with filesystem enumeration order; stable complete
fixture totals/ownership must not vary with that order or worker timing.
If an already-visited directory was only partially measured, a later root
deduplicating that directory inherits that incomplete coverage; covered-by never
turns an unfinished subtree into a complete independent workspace measurement.

Use checked u64 arithmetic; overflow terminates partial with `quantity_overflow`
and retains representable observations. Do not wrap, saturate or stringify an
already imprecise JS Number. Every byte/entry/revision/time quantity crossing
IPC is a canonical nonnegative decimal string; TS parses BigInt for formatting.
UI uses **GiB = 2^30 bytes**, or MiB/KiB for small values, explicitly labelled.
Never call a GiB value GB. The exact integer byte value is accessible text.
An observation commits dedup identity, ownership and all metric increments
atomically after checked calculations succeed. Overflow must not leave an
identity marked accounted with inconsistent totals or claim complete coverage.

`ScanMetrics` has `observedLogicalBytes`, nullable `observedAllocatedBytes`,
`uniqueFiles`, `uniqueDirectories`, `referencesDeduplicated`,
`allocationMeasuredFiles`, `allocationUnknownFiles`, `policyExcludedEntries`,
`unmeasuredEntriesLowerBound`, and `unmeasuredBytes: null` whenever omissions
have unknown extent (otherwise "0"). No estimate of the number/bytes beneath an
unreadable directory is invented. Each ScanMetrics carries `logicalCoverage` and
`allocatedCoverage`; each root/session also has overall `coverage`. Their values
are complete_within_policy, partial or unavailable. Overall is complete only
when both applicable metrics are complete; it is unavailable when no usable
root measurement exists. If logical observations are usable but every file's
allocation is unknown, logical coverage can be complete, allocated coverage is
unavailable with a null allocated subtotal, and overall coverage is partial.
An observed empty set has known zero allocation, not unavailable allocation.
Fixed policy exclusions (links, mount crossings, special files)
are counted and declared even in complete_within_policy. Resource skips,
unreadable/changed/cloud directories, missing metrics or cancellation mark the
affected metric/root partial; results must never be titled exact workspace total.

## Budgets, session lifecycle and retention

| Limit                                           | Quick                           | Deep                             |
| ----------------------------------------------- | ------------------------------- | -------------------------------- |
| Requested roots                                 | 1–32 enabled, same explicit IDs | Same; never broader roots        |
| Entry metadata attempts, session-wide           | 25,000                          | 250,000                          |
| Maximum child depth (root = 0)                  | 8                               | 32                               |
| Cooperative elapsed budget including validation | 5 seconds                       | 120 seconds                      |
| Traversal threads / queued sessions             | 1 / 0                           | 1 / 0                            |
| Scanner-owned descriptors                       | At most 36                      | At most 36                       |
| Identity records                                | 25,000                          | 250,000                          |
| Fixed identity-table storage                    | 65,536 × 64-byte slots (4 MiB)  | 524,288 × 64-byte slots (32 MiB) |
| Diagnostic groups / retained child-path samples | 128 / 0                         | 128 / 0                          |
| Snapshot IPC size / query interval              | At most 128 KiB / 250 ms        | Same                             |

Implementation counting clarification: the entry-attempt budget conservatively
charges every issued `next_entry` cursor request, including EOF or a failed/
stopped request. It upper-bounds descendant metadata-entry attempts; it is not
an exact count of objects measured or native syscalls. Root anchoring is bounded
separately by requested-root, descriptor and elapsed limits. The current cursor
does not promise resumability after a metadata/stream diagnostic: abandon that
directory branch partial, retain ancestor siblings and other roots, and do not
retry a replacement. These conservative choices may reduce coverage; they do
not widen either mode's numeric budget or conceal omitted entries.

These are conservative initial proposals, not benchmarked performance claims.
DFS streams entries with no frontier vector or complete directory listing; at
most 33 directory cursor frames plus transient descriptors fit the 36 limit.
Names/pending state are bounded by stack, platform component limits and the root
snapshot cap. Fixed open-addressed identity slots avoid unbounded HashMap growth;
check slot size and load/record limits, never drop deduplication on exhaustion.
Allocation failure is typed `resource_unavailable`. Diagnostic groups are keyed
by workspace ID, code, severity with counts; on the 128-group bound stop partial
with a reserved `diagnostic_limit` group. No child path/catalog leaves Rust.
No overall process-RSS guarantee is implied by the table's payload-storage cap.

Quick can complete a small scope; Deep offers a larger explicit traversal budget
but may still be partial. Hitting a depth limit skips that subtree and continues
siblings; entry/identity/time/descriptor limits stop the session cooperatively
with an explicit reason. No hidden-directory heuristic makes Quick appear total.

States: validating → running → completed or partial; validation with no usable
root → error. Cancel can move validating/running to cancelling; budget/resource
stop moves to stopping. Both retain the session/activity reservation until all
native work and descriptors finish. Only then publish cancelled/partial/error
with `workerActive: false`. Blocking calls cannot be force-aborted: an elapsed
budget may be exceeded while stopping, with honest `workerActive: true`. No new
session or workspace write is accepted while that work lives.

Use a dedicated `std::thread`, atomics and a short coordinator mutex; no new
Tokio features in application/core. Existing Tauri async dispatch remains for
bounded command/DB work. Tokio documents that a running blocking task is not
abortable; abandoning its response must not release its work permit.
[Tokio 1.53.1 blocking semantics](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html).

Check cancellation/budgets before and after each filesystem call and before
adding another observation. First stop/finalization decision under the mutex
wins: cancel accepted before finalization yields cancelled; cancel after terminal
returns that terminal snapshot without relabelling it. Successful observations
remain visibly partial when cancelled. Unexpected panic is sanitized error;
RAII closes handles and never leaves an idle gate with live I/O.
Status queries may mark an elapsed deadline as stopping without any filesystem
work; they cannot turn a blocked worker into a terminal result. Failed thread
creation releases an unused reservation and returns resource_unavailable.

Retain one active session, or the most recent terminal session, in memory. Accept
a new start only after prior worker stop; it replaces that terminal result.
No SQLite scan/history writes, migration, file log or persistent result export.
App exit requests cancellation and stops publication, without blocking the UI
on an uninterruptible syscall or leaving a daemon. Process exit ends its workers;
next launch has no session and never resumes automatically.

## IPC, recovery and minimum UI

Core defines opaque `ScanSessionId`, `ScanEpoch`, `ScanRequestId`, mode, snapshot,
metrics, protection, grouped diagnostics and sanitized `ScanFailure`. Epoch is
process-specific plus a monotonic generation; seed it with process ID and checked
Unix startup nanoseconds, rejecting unavailable clock/overflow rather than using
a constant fallback. This is replay separation, not a cryptographic credential;
restart invalidates old epochs. The generation is incremented with checked u64
arithmetic, not compared lexically by the frontend.
Request ID is a UUID-format token generated by `crypto.randomUUID()`; not a path
or authorization token. Session IDs use process epoch plus a checked counter;
no new random/UUID crate is required. Field casing is camelCase, enums snake_case.

Implementation wire clarification: epoch is the opaque string
`p<pid>-t<startupNanos>-g<generation>` (canonical u32/u128/u64 decimals);
session ID is `s:<epoch>:<positive-u64-counter>`. Request IDs are canonical
36-character lowercase-hex UUID-format strings; no additional UUID version
restriction. Clients compare tokens for equality, not lexical ordering.
startedAt/finishedAt are UTC Unix milliseconds as u64 decimal strings, with
finishedAt null until actual worker termination. Existing workspace timestamps
are unchanged. Workspace row states are pending, validating, running, completed,
partial, unavailable or cancelled; unobserved/unavailable rows have null metrics,
not a fabricated zero measurement. Global prefix counters may be zero while
coverage remains unavailable; display Not available rather than an empty root.
Only actual empty measured roots have available zero-byte measurements.

Use a dedicated StopReason subset for time/entry/identity/descriptor/diagnostic/
result/quantity limits and cancellation. Depth is a branch skip diagnostic, not
a session-stop request; root errors and missing allocation are not global stop
reasons. Grouped diagnostic severity is info/warning/error; fields are workspaceId,
code, severity, count and fixed message. Add outside_read_scope as an explicit
diagnostic for denied read scope; it never authorizes target I/O.

| Command                      | Arguments                                                       | Result                                                   |
| ---------------------------- | --------------------------------------------------------------- | -------------------------------------------------------- |
| `start_workspace_scan`       | `{ request: { requestId, expectedEpoch, workspaceIds, mode } }` | Authoritative ScanSnapshot, initially validating         |
| `get_workspace_scan`         | none                                                            | `{ epoch, session: ScanSnapshot or null }`               |
| `get_workspace_scan_session` | `{ sessionId }`                                                 | Retained matching snapshot, or session_not_found         |
| `cancel_workspace_scan`      | `{ sessionId }`                                                 | Snapshot acknowledging cancelling, or unchanged terminal |

Start is linearized with ActivityGate. Same request ID and identical payload
replays the retained session even if the start response was lost; different
payload is request_conflict. An unseen request with stale expectedEpoch rejects
stale_epoch; generation changes on every accepted start. Thus a late replay of
an evicted request cannot secretly create a new scan. Different request during
live work rejects scan_busy. Retain only one request mapping, with its session.
Check the retained request replay before gate/busy or current-epoch rejection.
Its original accepted payload/expectedEpoch still replays while its own gate is
occupied; only other requests and evicted stale replays use those rejections.

Snapshot includes sessionId, requestId, epoch, mode, state, decimal revision,
startedAt/finishedAt, decimal entryAttempts, workerActive, nullable stopReason, sorted workspace rows
(workspace ID, original display name/path, state, metrics, coverage and known
coveredBy IDs), global metrics/coverage/protection and grouped diagnostics.
No raw file identities, paths beneath roots, file findings or arbitrary errors.
Bound the whole serialized snapshot; reserve room for terminal diagnostics and
use `result_limit` partial rather than truncate into an apparently complete DTO.

Scan failure codes: invalid_request, scope_empty, scope_too_large,
workspace_not_found, workspace_disabled, scan_busy, workspace_operation_in_progress,
stale_epoch, request_conflict, session_not_found, persistence_unavailable,
resource_unavailable, application_unavailable. Root/traversal diagnostic codes:
root_missing, root_denied, root_changed, root_too_broad, unsupported_filesystem,
changed_during_scan, entry_missing, entry_denied, symlink_excluded, mount_excluded,
special_excluded, cloud_not_materialized, cloud_scope_unsupported,
unrepresentable_name, allocation_unknown, depth_limit, entry_limit, identity_limit,
descriptor_limit, time_budget, diagnostic_limit, result_limit, quantity_overflow,
cancelled. Counts coalesce; fixed English messages, no raw OS/SQL text.

Add all four to the shared AppManifest/handler and default capability allowlist
for local webview main only. Keep actual containing-window guards. No broadcast
events, remote grants, generic filesystem API, permission to create windows,
JS page-zoom grant, or frontend dialog permission. The native rejection suite
must cover all new commands before native engine integration.
[Tauri capability boundary](https://v2.tauri.app/security/capabilities/).

For that ordering, the existing `ApplicationService::new` initially has no scan
runner and start returns application_unavailable without traversal. A Rust-only
`new_with_scan_runner(adapter, database, Arc<dyn ScanRunner>)` constructor provides
normal dependency injection, not a JS command, runtime test flag or capability.
The native ACL target first injects a counted synthetic runner. Only after both
native denial gates pass does production setup inject `MetadataScanRunner`,
which uses the platform filesystem and the tested core engine. No product
synthetic runner or successful fake scan is distributed.

React `useWorkspaceScan` lives at App level independently of health/workspaces.
Initial get only recovers memory state; a 250 ms self-scheduling query runs only
while a known session is live and the document is visible, at most one query in
flight. Pause on hide, reconcile once on show; cancel/status remain user intents.
Unmount stops queries, not work. Navigation never cancels, resets or starts.
Reject stale session/epoch/request revisions after unmount or a newer session;
transport loss means “Scan status is unavailable. Reconnect to check progress.”,
not rollback, empty success or an automatic new start. Retry only queries state.

Proposed UI: Workspaces gets a compact “Workspace analysis” Section, explicit
Quick scan / Deep scan text buttons for all enabled roots and a Cancel scan
button while work is active. The existing rows show their latest contribution
and coverage separately from root Availability; no per-row start button or new
selection system. Overview replaces only its truthful scan EmptyState with the
same session summary when data exists, retaining full structural width/health.
No active roots: “No enabled workspace folders. Add or enable a workspace to scan.”
No progress denominator: show “Scanning…” and “Traversal checks attempted: 1,234”
from entryAttempts, never an invented object count, percent or ETA.
Stopping: “Waiting for a filesystem operation to finish. No new scan can
start yet.” Coverage copy derives from actual diagnostics and may combine:

- Limits: “Partial measurement — traversal stopped at a scan limit.” Name the
  actual depth, entry, identity, descriptor, diagnostic, payload or time reason.
- Missing/denied/changed branches: “Partial measurement — some folders could
  not be checked or changed during the scan.”
- Complete logical traversal, incomplete allocation: “File sizes measured;
  allocated space is only partially known.” Never claim folders were omitted
  solely because allocation is unknown.
- Cancellation: “Scan cancelled. Measurements shown are incomplete.”
- Policy exclusions: “Declared exclusions: symbolic links, mount crossings and
  special files are not included.” Show only categories actually excluded and
  their counts; exclusions alone do not mean a resource-limited traversal.

Deep copy states “Deep scan also has limits.” Time is a cooperative work budget,
not a duration guarantee; a blocked call can outlive it. A null value is “Not
available”; an observed zero is “0 B”; zero exclusive contribution covered by
another root is “Accounted for in another workspace” with its coveredBy names,
not an empty-folder claim. Announce start/state/error/terminal changes, not each
250 ms counter update; progress never moves focus.

Terminal complete: “Scan completed within the declared scope.” Always show
“Observed file sizes; not reclaimable space.” with the policy exclusions.

Reuse Section/DataList/ListRow/Button/FeedbackMessage, semantic tokens, full paths,
focus/announcements, system appearances and existing unavailable destinations.
Disable workspace writes consistently with backend scan ownership; explain why.
Do not steal navigation focus when snapshots arrive. Availability remains
Available/Contents have not been checked, independent of session completion.
No scan badges labelled Safe, cleanup buttons, charts, new theme, routes or cards.

## Verification and acceptance

The [plan](../plans/2026-10-16-phase-03-scanner-core.md) maps every behavior to named
synthetic/temporary-fixture, service, IPC, UI and native checks. Count I/O through
the narrow adapter seam; assert zero content reads, writes, shell and target
operations. Native fixtures may create/read sentinel contents only in test setup
and verification; production scanner has no such method. Permission/race/mount/
cloud failure injection is labelled simulated, not native proof on every volume.

Checkpoint order: (1) isolated inherited native verification; (2) contracts,
policy and engine with fixtures; (3) gated application/IPC/minimal UI integration;
(4) functional/native verification and **new Phase 03** human visual approval.
Keep the already-granted Phase 02 approval. No native pending item is PASS here.
Acceptance requires functional gates, the three P02 obligations at their defined
limits, native new-command authorization checks, truthful measurements and human
inspection of the new UI. Distribution is a separate release decision.

## Exclusions

No provider registry/tool detection, manifests/content reads, project intelligence,
global Mac scan, cleanup classification proposals, cleanup plans/execution,
user-file mutation, notifications, monitoring, telemetry, product networking,
FDA/permission bypass, privileged helpers, Swift styling, Windows, schema/history
changes, identifier change or Phase 05 dashboard. The initial design-only run
performed no implementation or analysis. The current assignment authorizes
implementation and controlled fixtures, not analysis of personal workspaces.
