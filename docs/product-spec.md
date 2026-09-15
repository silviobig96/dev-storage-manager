# Product Specification

## Product summary

`dev-storage-manager` is a free, open-source desktop application that helps developers understand, monitor, and safely reclaim disk space consumed by development environments.

Its working promise is: **Understand what your development environment is doing to your disk.** It is developer storage intelligence, not a generic system cleaner.

The product should answer:

- Why is the development environment using so much space?
- Which tools, categories, workspaces, and projects are growing?
- Which data is reproducible, which needs judgment, and which must not be touched?
- How did developer storage change today, over seven days, and over thirty days?

## Audience and outcomes

The initial audience is macOS developers using combinations of JavaScript, Rust, Apple, Android, container, and browser-testing tools. The user should be able to discover storage use without knowing each tool's cache layout, understand the consequence of a possible action, and retain control of every destructive choice.

Success means the product makes storage attributable, explainable, and safely actionable while remaining lightweight and private. It does not promise that all disk use can be classified or reclaimed.

## Product principles

1. Free and open source.
2. No account or cloud requirement.
3. Filesystem analysis and saved history remain local.
4. No automatic deletion and no scheduled cleanup.
5. Every destructive action requires explicit user intent.
6. Explain an item before offering an action.
7. Use specific category names; never hide behavior behind labels such as "junk."
8. Enforce safety in the shared core, independent of the UI.
9. Support macOS first and Windows later without rewriting the shared core.
10. Minimize background CPU, I/O, memory, and wakeups.

## Functional scope

### Workspaces

A user may configure zero, one, or many workspace directories. A workspace is an explicit local path chosen by the user; the application must not assume `~/Repositories` or any other convention. Project discovery may examine descendants of those roots for indicators such as Git metadata, package manifests, lockfiles, native project files, and build-system files.

Selecting a workspace authorizes read-only analysis within that scope. It does not authorize deletion, imply that all descendants are generated, or weaken protected-path rules.

### Scans

A **quick scan** examines known developer-storage locations and configured workspaces. It is designed for interactive and periodic use.

A **deep scan** is explicitly requested, more expensive, and may broaden analysis where current permissions allow. It is never a default periodic full-disk traversal.

Both scan modes produce structured findings, warnings, and partial results. Unreadable paths or provider failures must not erase successful findings from other providers.

### Providers

Independent providers describe storage owned by developer tools or project artifact families. Initial concepts include npm, Yarn, pnpm, NVM, Node project artifacts, Xcode, CoreSimulator, CocoaPods, Android SDK and emulators, Gradle, Cypress, Playwright, Homebrew, and Docker. These are roadmap candidates, not implemented features or a promise that every item is reclaimable.

Each finding identifies its provider/category, path, measured bytes, classification, reclaimability, explanation, supported actions, platform compatibility, and any warnings. The shared core validates classifications and actions against policy.

### Project intelligence

Project scanning may recognize generated or reproducible directories such as `node_modules`, `.next`, `dist`, `build`, `out`, `coverage`, `.turbo`, `Pods`, Gradle build output, and Expo-generated data. Evidence can include manifests, lockfiles, Git presence, modification times, and cheaply available Git activity.

Activity labels use probabilistic wording: `Active`, `Recently used`, or `Likely inactive`. Inferred inactivity is never sufficient authorization for deletion or a safety downgrade.

### Local history

SQLite stores configuration and aggregated local history. The model separates scan runs, category snapshots, configured workspaces, workspace snapshots, projects, project snapshots, and findings, while allowing names and normalization to evolve through reviewed migrations.

History supports comparisons such as current size and change over one, seven, and thirty days. The application does not upload source, file contents, repository data, inventories, or telemetry.

### Monitoring and notifications

Monitoring preferences are `Off`, `Daily`, or `Weekly`. A monitor performs bounded quick scans or reuses future incremental signals; it does not continuously recurse through the full disk.

Configurable notification rules may cover total developer-storage growth, reclaimable storage thresholds, category thresholds, or unusually fast growth. Monitoring may scan, record, compare, estimate, and notify. It must never invoke a destructive action.

## Safety classifications

The product has exactly three conceptual safety levels:

- **SAFE:** Regenerable data, such as selected package caches, downloaded test binaries, Xcode DerivedData, and narrowly defined temporary caches. Explicit user action is still required.
- **REVIEW:** Potentially reclaimable data whose removal can affect workflows, require downloads or rebuilds, or discard configuration. Examples include project dependencies, old runtimes, Docker data, Android images, simulators, Pods, build output, and broad build caches.
- **PROTECTED:** Data not offered for normal deletion, including source code, `.git`, manifests, lockfiles, user documents, active runtime dependencies, critical SDK/runtime data, and system files.

The complete invariants are in [safety-model.md](safety-model.md).

## Destructive-operation lifecycle

Future cleanup must keep these stages distinct:

1. Analyze without mutation.
2. Build a cleanup plan.
3. Explain effects and recovery cost.
4. Let the user select exact targets.
5. Obtain explicit confirmation.
6. Revalidate each target and its classification.
7. Execute only authorized actions.
8. Verify the result.
9. Record outcome and measured reclaimed bytes.

This lifecycle is documentation only in Phase 00 and is not part of Phase 01.

## Technology direction

- Desktop shell: Tauri 2.
- UI: React, TypeScript, and Vite.
- Shared application and policy core: Rust.
- Local persistence: SQLite.
- macOS integration: Rust/macOS APIs where appropriate, with optional Swift helpers behind a macOS adapter when Apple APIs are significantly cleaner or safer from Swift.
- Windows: future adapter reusing the Rust core, UI, persistence model, and provider contracts.

Electron is excluded because the product prioritizes a small runtime footprint, native integration, and a strong Rust safety boundary.

## Privacy and non-goals

The product has no default telemetry and no network dependency. Networking requires a later, explicitly approved requirement and a defined data and consent model.

Current non-goals include generic consumer cleanup, automatic optimization, scheduled cleanup, a constantly running full-disk crawler, cloud sync, remote inventory, Windows implementation, privileged helpers, and bypassing operating-system permissions.

## Release state

The project is pre-alpha. Phase 00 produces only documentation. Planned capability must not be described as already implemented. MIT is the provisional working license until initial publication.
