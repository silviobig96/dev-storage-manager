# Security Policy

`dev-storage-manager` examines developer files and may eventually perform explicitly requested cleanup. A defect at that boundary could cause data loss, so safety and security reports receive priority even during pre-alpha development.

## Reporting a vulnerability

Do not open a public issue for a vulnerability that could expose private local data, execute unintended commands, escape an allowed path, bypass confirmation, misclassify protected data, or delete the wrong target.

After the repository is published, use GitHub's private vulnerability reporting for this repository when it is available. If that channel is unavailable, contact the maintainers through a private channel listed on the repository owner's GitHub profile. Include the affected revision, platform, reproduction steps, impact, and any safe proof of concept. Do not include unrelated filesystem contents, credentials, or personal paths.

The project will acknowledge reports through the same private channel and coordinate disclosure after the risk has been understood and a fix is available. This pre-alpha project does not yet promise a fixed response-time service level.

## Security boundaries

The following rules apply to all future implementation:

- The monitor may scan, compare, store local history, estimate reclaimable bytes, and notify. It may never delete, prune, uninstall, clear caches, or schedule cleanup.
- A `SAFE` classification does not authorize deletion. Every destructive action requires explicit selection and confirmation.
- `PROTECTED` targets are rejected by core policy and are not normally executable, regardless of provider or UI input.
- Cleanup analysis, planning, presentation, execution, verification, and audit recording are separate stages.
- A target must be revalidated immediately before execution to reduce stale-path, symlink, mount, and time-of-check/time-of-use risk.
- Shell command construction must not interpolate untrusted paths. Typed process arguments or direct filesystem APIs are required.
- Workspace roots and allowed provider roots must be explicit. A configured workspace does not make every descendant deletable.
- Platform privileges must be narrow and optional. Full Disk Access and privileged helpers are outside the application-foundation scope.
- Local data remains local by default. Telemetry and networking are disabled unless a future approved requirement defines the data and consent model.

## High-risk review areas

Changes involving canonicalization, symlinks, mount boundaries, package-manager commands, Docker operations, permissions, protected-path matching, SQLite migrations, or native platform bridges require dedicated tests and a reviewer to consider data-loss scenarios.
