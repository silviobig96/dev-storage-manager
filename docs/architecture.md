# Architecture

## Goals

The architecture places classification and destructive-operation safety in a shared Rust boundary, keeps the interface replaceable, isolates operating-system behavior, and allows read-only scanning and local history to evolve independently. Phase 00 defined the target boundaries. Phase 01 implements only the application shell, layer skeleton, persistence foundation, and read-only health path described below; the remaining product architecture is planned.

## System context

The following diagram is the planned product architecture. Phase 01 implements only the React-to-Tauri health path and the Rust application, platform, and persistence foundations; it does not implement the scanner, providers, project analyzer, safety policy, history features, or cleanup planning.

```text
React + TypeScript UI
          |
     typed Tauri IPC
          |
Rust application services
          |
+---------+----------+-----------+--------------+----------+
| scanner | provider | project   | safety/risk  | history  |
|         | registry | analyzer  | policy       |          |
+---------+----------+-----------+--------------+----------+
          |                  |
  cleanup planning      platform abstraction
  (separate from        |
   execution)           +-- macOS adapter (Rust, optional Swift helper)
                       +-- Windows adapter (future)
```

Tauri commands are a thin transport boundary. They parse and validate request shapes, call application services, and map typed results to IPC responses. They do not contain storage policy, provider discovery rules, SQL, or platform-specific logic.

## Layers and responsibilities

### User interface

The React interface renders data, gathers workspace choices, explains consequences, and collects explicit intent. UI state can narrow what is displayed but cannot authorize an action or override a core classification.

Frontend domain types mirror serialized IPC contracts. They are not independent definitions of safety policy.

### Tauri interface

Tauri is the desktop shell and IPC transport. Commands expose narrow application use cases. Command payloads use explicit identifiers and typed fields rather than raw shell fragments.

Phase 01 exposes only a read-only health/capabilities query sufficient to verify boundaries. Later commands will be introduced with their owning phase.

### Rust application services

Application services coordinate use cases and own transaction boundaries. Planned services include scan coordination, workspace management, history queries, and cleanup planning. Cleanup execution is a distinct service with stricter authorization and is not introduced before its roadmap phase.

### Shared core

The core owns platform-neutral models and policies:

- provider and finding contracts;
- `SAFE`, `REVIEW`, and `PROTECTED` classification;
- action eligibility and policy decisions;
- project evidence and probabilistic activity labels;
- scan summaries, warnings, and errors;
- cleanup-plan validation; and
- history-facing domain records.

The core depends on traits for filesystem metadata, clocks, persistence, and platform capabilities. It must not depend on React or Tauri.

### Providers and registry

Each provider detects one tool or artifact family and reports structured observations. Providers do not execute cleanup, make arbitrary policy exceptions, or directly persist history. A registry supplies compatible providers to the scan coordinator based on platform and scan mode.

Provider failures are isolated. The coordinator returns partial results with warnings instead of treating one inaccessible path as a failed global scan.

### Project analyzer

The analyzer operates only within user-selected workspaces. It recognizes project boundaries and generated artifacts from explicit evidence. It may calculate an activity label, but activity is informational and never a deletion predicate.

### Safety and risk policy

The policy evaluates typed finding facts, protected rules, platform facts, and proposed actions. Provider classifications are claims to validate, not authority. The policy may preserve or raise risk; a lower-risk classification requires an explicit, versioned core rule and tests. `PROTECTED` is absorbing for normal cleanup: downstream code cannot lower it.

### Persistence

SQLite stores application-owned configuration and aggregated history behind a repository interface. Schema changes are versioned migrations. UI and providers do not issue SQL directly.

Persistence records paths only when locally required for workspace identity, project attribution, or finding history. It never stores file contents or source code. Retention policy and path-minimization rules are defined before history is implemented.

### Platform abstraction

The platform boundary supplies capabilities such as known developer locations, filesystem identity, permission status, notifications, and future native integrations. The macOS implementation is first. Swift is permitted only as a narrow helper behind this boundary when safer or substantially clearer than Rust/macOS bindings.

The future Windows adapter implements the same contracts. Windows path conventions or behavior must not leak into shared domain logic.

## Read-only scan data flow

1. UI requests a quick or explicitly chosen deep scan.
2. The command boundary validates the request and resolves selected workspace identifiers.
3. The coordinator asks the platform adapter for allowed roots and capabilities.
4. Compatible providers and the project analyzer inspect only their declared scope.
5. Providers emit structured findings and diagnostics.
6. Core policy validates or raises each classification and filters unsupported actions.
7. The coordinator aggregates sizes without double counting stable filesystem identities.
8. Persistence records local scan metadata and snapshots when history is enabled.
9. The command returns results and partial-failure diagnostics to the UI.

Cancellation and progress are explicit concerns for later scanner phases. A cancelled scan records no completed snapshot unless the persistence design explicitly supports a separately marked partial snapshot.

## Monitoring flow

The monitor schedules lightweight quick scans at `Off`, `Daily`, or `Weekly` cadence. It reads configuration, runs the same read-only coordinator, compares locally persisted snapshots, and emits a notification when a configured threshold is crossed. It has no dependency on cleanup execution and no API capable of invoking it.

Future incremental scanning or filesystem-event hints may reduce work, but events are hints that require reconciliation; they do not change safety policy.

## Cleanup lifecycle boundary

Future cleanup follows `Analyze -> Plan -> Explain -> Select -> Confirm -> Revalidate -> Execute -> Verify -> Record`. The plan contains immutable target identifiers, expected filesystem identity, classification, action, estimated bytes, and consequences. Execution accepts a confirmed plan rather than a free-form path.

Immediately before mutation, the executor canonicalizes and re-identifies the target, reruns protected rules, verifies that the action remains supported, and rejects stale or broadened scope. Result recording distinguishes estimated from measured reclaimed bytes and preserves per-target failures.

Planning and execution are intentionally absent from Phase 01.

## Conceptual data model

- **Workspace:** stable ID, user-selected path, display name, enabled state, created/updated timestamps.
- **Scan:** stable ID, mode, platform, start/end timestamps, status, versioned scanner metadata.
- **Finding:** scan ID, provider/category IDs, local target identity/path, measured bytes, classification, reclaimability, explanation, supported actions, diagnostics.
- **CategorySnapshot:** scan ID, category ID, total bytes, potentially reclaimable bytes.
- **WorkspaceSnapshot:** scan/workspace IDs, total bytes, project count, diagnostics.
- **Project:** stable ID, workspace ID, root identity/path, detected ecosystems.
- **ProjectSnapshot:** scan/project IDs, total and generated bytes, activity evidence and label.

Normalization and retention are finalized in the history phase. These concepts prevent scan execution, persistent identity, and time-series aggregation from collapsing into one record.

## Error model and observability

Expected outcomes are typed: inaccessible path, unsupported platform capability, changed target, cancelled scan, provider failure, and persistence failure. User messages name the affected provider or scope and say whether results are partial. Logs remain local, avoid file contents and secrets, and redact paths when full paths are not necessary.

No telemetry is enabled by default. Networking is absent unless a future approved requirement explicitly introduces it.

## Resource constraints

Quick scans use bounded concurrency and known roots, avoid following unsafe symlink escapes, and support cancellation. Periodic work is coarse-grained and never a constantly recursive disk scan. Expensive metadata, including Git history, is queried only when it materially improves a result and can be obtained cheaply.

## Phase 01 boundary

Phase 01 implements a Tauri 2 shell and React/TypeScript foundation screen, Rust core/application/platform/persistence boundaries, a SQLite connection and migration foundation, and one read-only `get_app_health` contract that crosses those layers. Tests, formatting, linting, production frontend builds, local debug application builds, and macOS CI verify this foundation.

The command registry contains only `get_app_health`. Phase 01 contains no provider, workspace behavior, filesystem scan, cleanup planning or execution, background monitor, notification, Windows implementation, privileged helper, or Full Disk Access flow. These exclusions are architectural boundaries, not merely absent UI controls.
