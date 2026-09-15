# Provider Contract

## Purpose

A provider is a read-only detector for one developer tool or artifact family. It reports observations in a common shape so scanning, policy, history, and the UI do not depend on provider-specific prose or filesystem assumptions.

Providers are not cleanup executors, policy authorities, persistence clients, or notification schedulers.

## Conceptual interface

The Rust names below establish the intended contract and may be refined during the provider phase without weakening the semantics.

```rust
trait StorageProvider: Send + Sync {
    fn descriptor(&self) -> ProviderDescriptor;
    fn scan(&self, context: &ScanContext) -> Result<ProviderReport, ProviderFailure>;
}

struct ProviderDescriptor {
    id: ProviderId,
    display_name: String,
    category: CategoryId,
    supported_platforms: Vec<Platform>,
    supported_scan_modes: Vec<ScanMode>,
}

struct ScanContext {
    mode: ScanMode,
    platform: Platform,
    workspace_roots: Vec<WorkspaceRoot>,
    cancellation: CancellationToken,
}

struct ProviderReport {
    findings: Vec<ProposedFinding>,
    diagnostics: Vec<ScanDiagnostic>,
}

struct ProposedFinding {
    provider_id: ProviderId,
    category_id: CategoryId,
    target: ObservedTarget,
    measured_bytes: u64,
    proposed_classification: SafetyLevel,
    reclaimability: Reclaimability,
    explanation: Explanation,
    proposed_actions: Vec<ActionKind>,
    evidence: Vec<Evidence>,
}
```

The policy service converts `ProposedFinding` into a validated `Finding`. The validated form contains the authoritative classification, allowed actions, policy version, and any policy reasons. Only validated findings may cross IPC or enter cleanup planning.

## Required semantics

### Identity

Provider and category identifiers are stable, namespaced, machine-readable values. Display names are localizable labels and are never used as database or policy keys.

`ObservedTarget` includes a local path plus the available filesystem identity and type. Paths remain local. A path string alone is not stable identity and cannot support future execution without revalidation.

### Size

`measured_bytes` is an unsigned physical or logical byte count whose measurement method is declared in evidence. Providers must not label an estimate as measured. Aggregation must prevent overflow and account for overlapping findings or hard-linked content.

### Classification and reclaimability

`SafetyLevel` has exactly `Safe`, `Review`, and `Protected`. It is a structured enum, not provider text.

`Reclaimability` distinguishes `PotentiallyReclaimable`, `NotReclaimable`, and `Unknown`. It is independent of safety. For example, an item may be reproducible but currently not actionable, or protected despite having a large size.

The core policy may raise risk or remove actions. Providers cannot downgrade a protected decision.

### Explanation

An explanation states:

- what created or owns the item;
- why it consumes space;
- why the classification applies;
- the consequence and likely recovery cost of each proposed action; and
- relevant uncertainty.

Explanations use specific nouns and do not call data "junk."

### Actions

`ActionKind` is a closed, typed identifier registered in the core. It is not a shell command, executable path, or arbitrary string. Reporting an action does not execute it or authorize it.

A provider that cannot support a safe, explainable action returns an empty action list. `Protected` findings always leave policy with no normal destructive actions.

### Platforms and modes

Descriptors declare supported platforms and scan modes. The registry excludes incompatible providers before scanning. A future Windows provider can share an identifier and core contract while using a distinct adapter implementation.

Providers receive configured workspace roots; they never invent a universal project root. Deep-scan behavior must be explicitly declared and remains constrained by permissions and policy.

## Diagnostics and partial results

`ScanDiagnostic` contains a stable code, severity, provider ID, optional non-sensitive scope, and human-readable message. Expected diagnostic kinds include inaccessible path, permission denied, changed during scan, unsupported capability, size incomplete, and cancelled.

Recoverable path errors are diagnostics, not a global provider failure. `ProviderFailure` is reserved for inability to produce a meaningful report, such as invalid provider configuration or an internal invariant failure. The coordinator retains successful provider reports and labels the overall scan partial when needed.

## Provider lifecycle

1. The registry selects providers compatible with platform and scan mode.
2. The coordinator creates a read-only context with workspaces and cancellation.
3. Providers discover candidates only within documented roots.
4. Providers measure and emit observations plus evidence.
5. Core policy validates classification and action eligibility.
6. Aggregation deduplicates overlapping filesystem identities.
7. Validated findings may be displayed and recorded locally.

Providers never call cleanup execution or write history directly.

## Contract tests

Every provider implementation must demonstrate:

- stable descriptor identifiers;
- rejection on unsupported platforms or scan modes;
- no assumption about workspace path conventions;
- cancellation and partial-error behavior;
- deterministic output for a fixture filesystem;
- correct size accounting and overflow handling;
- structured explanations and evidence;
- inability to emit arbitrary action commands;
- core policy precedence over proposed classification; and
- no writes during scanning.

Fixtures use synthetic paths and contents; tests must not depend on or record a contributor's real home directory.

## Initial provider candidates

Candidates include npm, Yarn, pnpm, NVM, Node project artifacts, Xcode, CoreSimulator, CocoaPods, Android SDK, Android emulators, Gradle, Cypress, Playwright, Homebrew, and Docker. Phase 00 defines only this contract. Provider implementation begins in its dedicated roadmap phase.
