# Safety Model

## Purpose

Safety classification communicates the consequence of a finding and constrains which actions the core may expose. It is not a promise that deletion is harmless, and it never replaces user intent.

There are exactly three conceptual levels: `SAFE`, `REVIEW`, and `PROTECTED`.

## Classifications

### SAFE

The data is regenerable under a narrowly defined, tested rule. Examples can include npm or Yarn caches, Cypress downloaded binaries, Xcode DerivedData, and selected temporary or update caches.

`SAFE` does not mean automatic deletion. A user must select the exact target, see the explanation and expected consequence, explicitly confirm, and pass revalidation. Regeneration may still cost time, bandwidth, or compute and must be disclosed.

### REVIEW

The item may be reclaimable, but removal can disrupt work, require downloads or rebuilds, remove local state, or demand human judgment. Candidate examples include `node_modules`, older runtime installations, Docker images and volumes, Android system images, Xcode simulators, Pods, generated build output, and Gradle caches.

Review items are opt-in at target granularity. They are never preselected on the basis of age, size, or inferred inactivity.

### PROTECTED

The application does not offer normal deletion for the item. Examples include source code, `.git`, manifests, lockfiles, user documents, active runtime dependencies, critical SDK/runtime data, and macOS system files.

Core policy rejects normal cleanup actions for protected items even if a provider, database row, IPC payload, or stale UI state claims a lower classification.

## Classification authority

Providers report facts and a proposed classification. The shared Rust policy is authoritative. It applies versioned protected rules and action rules before findings cross the application boundary.

Policy can preserve a classification or raise its risk. Lowering risk requires an explicit core rule change, focused tests, security review, and corresponding documentation. No provider or UI path may silently translate `PROTECTED` to `REVIEW` or `SAFE`.

Unknown, ambiguous, unsupported, inaccessible, or conflicting evidence defaults to non-actionable `REVIEW` or `PROTECTED`, depending on whether the core can prove that the candidate lies within a recognized generated-data boundary. Uncertainty never defaults to `SAFE`.

## Protected-rule precedence

Protected rules run before reclaimability and action eligibility. At minimum, future rules must guard:

- workspace and project source roots as containers whose descendants require independent classification;
- `.git` directories and contents;
- recognized manifests and lockfiles;
- user document locations and operating-system paths;
- filesystem roots, home roots, mount roots, and configured workspace roots as direct cleanup targets;
- active runtime and required SDK data identified by platform or tool state;
- symlinks or resolved targets that escape the provider's declared allowed roots; and
- targets whose identity or type changed after analysis.

A lexical descendant check alone is insufficient. Canonical paths, filesystem identity, symlink behavior, mount boundaries, and race conditions must be considered at execution time.

## Action eligibility

Findings distinguish classification from possible actions and from estimated reclaimable bytes.

- `PROTECTED`: no normal destructive action.
- `REVIEW`: only provider-declared actions allowed by core policy; explicit selection and confirmation required.
- `SAFE`: only provider-declared actions allowed by core policy; explicit selection and confirmation still required.

The core returns a reason when it removes or rejects a proposed action. A UI must not construct an unapproved action locally.

## No automatic deletion invariant

No trigger, schedule, threshold, notification, scan result, activity label, or classification can constitute consent. The monitor has no dependency or callable route to cleanup execution. There is no scheduled cleanup feature.

Bulk selection, if later approved, must still enumerate targets and consequences. Default selection behavior is defined and tested in the cleanup phase; no item may be silently included.

## Future destructive-operation protocol

No destructive operation is implemented in Phase 00 or Phase 01. A later implementation must enforce:

1. **Analyze:** gather current facts without mutation.
2. **Plan:** produce explicit immutable candidates and expected identities.
3. **Explain:** show classification, evidence, consequence, recovery path, and estimated bytes.
4. **Select:** accept explicit per-target intent.
5. **Confirm:** bind confirmation to the exact plan and target set.
6. **Revalidate:** resolve paths and identity again, rerun protected and eligibility rules, and reject drift.
7. **Execute:** perform only the approved typed action without shell-string interpolation.
8. **Verify:** inspect outcome and calculate measured rather than assumed reclamation.
9. **Record:** store per-target result, error, and reclaimed-byte measurement locally.

Plans expire when relevant filesystem identity, metadata, provider version, policy version, or user selection changes.

## Safety test obligations

Future tests cover policy precedence, every protected rule, unknown classifications, unsupported actions, path traversal, symlink escape, mount changes, stale plans, target replacement, partial failures, cancellation, integer overflow in byte totals, and the inability of monitor code to reach execution code.

Tests must assert fail-closed behavior. UI tests supplement but never replace core policy tests.

## Examples

| Candidate | Default conceptual level | Reason |
| --- | --- | --- |
| Xcode DerivedData at a recognized location | SAFE | Regenerated by builds, with rebuild cost explained |
| Project `node_modules` | REVIEW | Reinstallable but may disrupt offline or unusual workflows |
| Docker volume | REVIEW | May contain unique databases or development state |
| Project lockfile | PROTECTED | Defines reproducible dependency resolution |
| `.git` directory | PROTECTED | Contains history, branches, and repository state |
| Unrecognized large directory | PROTECTED | Size alone is not evidence of safe regeneration |

These are policy examples, not implemented detectors.
