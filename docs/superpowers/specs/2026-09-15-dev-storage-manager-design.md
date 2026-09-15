# dev-storage-manager Product Foundation Design

**Date:** 2026-09-15

**Status:** Approved foundation for Phase 00

**Working product name:** `dev-storage-manager`

## Executive summary

`dev-storage-manager` is a free and open-source desktop application for understanding, monitoring, and safely reclaiming storage created by developer tools and projects. Its working position is: **Understand what your development environment is doing to your disk.**

The product is not a generic Mac cleaner. It attributes bytes to recognizable tools and projects, explains why the data exists and what removal would cost, tracks change locally, and preserves human control. Phase 00 creates documentation only.

## Approved product decisions

- Local-first; no account or cloud service is required.
- Filesystem analysis, configuration, and history remain on the device.
- No automatic deletion or scheduled cleanup exists.
- Every destructive operation requires exact user selection and confirmation.
- Safety decisions are core-enforced and cannot rely on UI state.
- The only conceptual safety levels are `SAFE`, `REVIEW`, and `PROTECTED`.
- A `SAFE` item is regenerable but still requires explicit user action.
- A `PROTECTED` item is never offered for normal deletion.
- Workspaces are zero or more paths selected by the user, never a hardcoded convention.
- macOS is first; Windows is a future adapter that reuses shared logic.
- Background work must be lightweight and must not repeatedly traverse the entire disk.
- No telemetry or networking is present by default.
- MIT is the working license, provisional until initial publication.

## Scope and user experience

The application organizes developer storage by tool category, workspace, and project. It shows measured size, potential reclaimability, safety level, explanation, and scan uncertainty. It can eventually compare current usage against one-, seven-, and thirty-day history.

A user configures any number of workspace roots such as `~/Developer`, `~/Code`, `~/Projects`, or a custom volume. Future Windows users may choose roots on any drive. Project discovery happens inside those roots using evidence including Git metadata, manifests, lockfiles, and ecosystem project files. Choosing a root grants read-only scan scope, not blanket cleanup authority.

Quick scans cover known developer storage and configured workspaces. Deep scans are explicit, more expensive requests. Periodic monitoring uses bounded quick scans or future incremental hints; it is never a constant recursive crawler.

## Safety design

### SAFE

Narrowly recognized regenerable data, potentially including package caches, downloaded test binaries, DerivedData, and selected temporary caches. Re-creation cost is explained. Nothing is deleted without explicit selection and confirmation.

### REVIEW

Potentially reclaimable data requiring judgment because removal may break an offline workflow, trigger a large download or rebuild, remove local state, or affect a configured runtime. Examples include project dependency directories, older runtimes, simulator and emulator data, Docker data, Pods, and broad build caches.

### PROTECTED

Source, Git data, manifests, lockfiles, user documents, active runtime requirements, critical SDK data, and system files. Normal cleanup actions are unavailable.

Providers propose facts, classification, and action identifiers. Versioned Rust core policy validates the proposal and can preserve or raise risk. A lower-risk result exists only when an explicit policy rule proves it. Ambiguity fails closed, and protected status cannot be downgraded downstream.

Classification, reclaimability, supported action, and user authorization are separate fields. Age, size, or an inferred activity label never grants deletion permission.

## Architecture decision

The approved stack is:

- Tauri 2 desktop shell;
- React, TypeScript, and Vite frontend;
- Rust shared core and application services;
- SQLite local persistence;
- macOS adapter implemented in Rust with an optional narrow Swift helper; and
- a future Windows adapter implementing the same platform contracts.

Electron is excluded. Swift is not the primary UI or application stack.

```text
UI (React + TypeScript)
          |
typed Tauri commands
          |
application services (Rust)
          |
+----------------+----------------+----------------+
| scan/project   | safety/cleanup | local history  |
| coordination   | planning       | repositories   |
+----------------+----------------+----------------+
          |
provider contracts + platform abstraction
          |
macOS adapter now | Windows adapter later
```

### Boundary rules

- React presents state and intent but owns no safety policy.
- Tauri commands validate transport shapes and call one application use case.
- The application layer coordinates workflows and persistence transactions.
- The core owns domain types, classification, policy, provider contracts, and cleanup-plan validation.
- Providers are read-only detectors; they do not persist, notify, or execute cleanup.
- Platform adapters expose operating-system capabilities without leaking platform rules into shared logic.
- SQLite is accessed through Rust repositories, not directly from the UI or providers.
- Cleanup planning and execution are separate services and roadmap phases.

## Provider model

A provider descriptor declares stable provider/category IDs, display name, compatible platforms, and scan modes. A scan receives the mode, platform capabilities, configured workspaces, and cancellation. It returns proposed findings and diagnostics.

Each proposed finding includes provider/category IDs, filesystem target and available identity, measured bytes, a structured proposed safety level, reclaimability, explanation, typed supported actions, evidence, and warnings. The policy produces the validated finding consumed by IPC and persistence.

Provider failures are isolated. Unreadable locations create scoped diagnostics and partial results; they do not discard successful work. Duplicate filesystem identities are not counted twice.

Initial provider candidates are npm, Yarn, pnpm, NVM, Node project artifacts, Xcode, CoreSimulator, CocoaPods, Android SDK and emulators, Gradle, Cypress, Playwright, Homebrew, and Docker. They are not implemented in Phase 00 or Phase 01.

## Project intelligence

Inside selected workspaces, the future analyzer identifies project boundaries and generated items such as `node_modules`, `.next`, `dist`, `build`, `out`, `coverage`, `.turbo`, `Pods`, Gradle build output, and Expo-generated data.

Evidence may include manifests, lockfiles, Git presence, modification timestamps, and cheaply available recent Git activity. The UI describes activity probabilistically as `Active`, `Recently used`, or `Likely inactive`. This label is explanatory only.

## Persistence model

SQLite holds configuration and aggregated local history behind Rust interfaces. The conceptual model separates:

- a scan run and its completion state;
- category totals for a scan;
- configured workspaces and their per-scan totals;
- stable project identities and per-scan project observations; and
- individual validated findings and diagnostics.

Schema migrations are ordered, atomic, versioned, and tested against a fresh database and the prior supported schema. Source contents are never stored. Paths are retained only when locally needed for identity or attribution, and retention/path-minimization policy is finalized before history ships.

## Monitoring

Monitoring preferences are `Off`, `Daily`, and `Weekly`. Thresholds are configurable and may cover total growth, reclaimable estimates, category size, or unusual growth. Monitoring reuses the read-only scanner and history comparison. Its dependency graph contains no cleanup executor.

## Future cleanup protocol

Destructive work does not begin before the read-only scanner, safety rules, and dashboard are stable and tested. The required flow is:

`Analyze -> Plan -> Explain -> User selects -> Explicit confirmation -> Revalidate -> Execute -> Verify -> Record`

A confirmed plan binds exact target identity, action, classification, policy version, expected scope, and explanation. Execution canonicalizes and re-identifies the target immediately before mutation, reruns protected rules, and rejects stale or broadened plans. Results record per-target status and measured reclaimed bytes.

No scheduled path can create confirmation or call execution.

## Failure and privacy behavior

Expected errors are typed and scoped: permission denied, inaccessible path, unsupported capability, provider failure, target changed, cancellation, and persistence failure. Partial results remain visible with warnings. Logs avoid file contents, secrets, and unnecessary full paths.

No account, upload, analytics, or networking is required. A later network feature needs explicit approval plus a documented data and consent model.

## Delivery sequence

1. Product foundation.
2. Application foundation.
3. Workspace system.
4. Read-only scanner core.
5. Developer storage providers.
6. Dashboard.
7. Explicit safe cleanup.
8. Project intelligence.
9. Local storage history.
10. Background monitor and notifications.
11. Signed and notarized macOS release.
12. Windows platform support.

The numbered delivery order corresponds to roadmap Phases 00 through 11. Cleanup cannot move earlier than Phase 06.

## Phase 01 acceptance boundary

Phase 01 creates a buildable Tauri/React shell, Rust crate boundaries, a SQLite migration foundation, a read-only health command, tests, lint/format tooling, and CI. It deliberately excludes providers, storage scanning, workspace behavior, cleanup planning or execution, monitoring, notifications, Windows code, privileged helpers, and Full Disk Access behavior.

The foundation is accepted when frontend and Rust focused tests pass, formatting and linting pass, the production frontend and Tauri application build, the SQLite migration works on an empty temporary database, the health command crosses the intended layers, and CI runs the same checks on macOS.

## Open-source posture

External contributors should be able to audit boundaries and understand why an action is or is not offered. Safety or destructive-operation changes require explicit documentation, focused tests, and security review. The MIT license is provisional until the repository's first public release.
