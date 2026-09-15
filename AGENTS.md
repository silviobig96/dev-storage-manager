# Repository Instructions

These rules apply to every contributor and automated agent working in this repository.

## Safety invariants

- **No automatic deletion.** Scans, history, estimates, and notifications must never trigger cleanup.
- Never execute or implement a destructive filesystem operation unless the current task explicitly authorizes that exact operation.
- Phase 00 contains documentation only and no destructive implementation.
- `SAFE` still requires explicit user selection and confirmation before any future destructive action.
- Safety classifications are enforced by the Rust core, not only by provider text or UI state.
- `PROTECTED` items are never offered for normal deletion, and no category may silently move from `PROTECTED` to `REVIEW` or `SAFE`.
- Keep cleanup planning separate from cleanup execution. Revalidate every future target immediately before execution.

## Product and architecture constraints

- Never hardcode `~/Repositories` or any other project-root convention. Workspaces are zero or more user-selected paths.
- Keep platform-specific behavior behind adapters.
- Put shared logic in Rust when practical.
- Swift is allowed only for macOS-native integration behind the macOS platform boundary; it is not the primary UI or application architecture.
- Do not introduce Electron. The approved shell is Tauri 2 with React, TypeScript, and Vite.
- Windows is a future platform; do not introduce its implementation during macOS-first phases.
- Optimize for local-first operation and minimal background resource usage.
- Do not add telemetry by default.
- Do not add networking unless a future approved requirement explicitly needs it.

## Engineering principles

- Use test-driven development in implementation phases: focused failing test, minimal implementation, focused passing test.
- Prefer small, focused modules and explicit interfaces.
- Apply YAGNI; do not build later-phase features early.
- Use focused tests before full suites.
- Make frequent, scoped commits at independently reviewable checkpoints.
- Verify with current command output before claiming completion.
- Never modify unrelated files.
- Never commit secrets, credentials, local filesystem inventories, or personal data.
- Keep documentation synchronized with product, security, and architecture decisions.

## Phase ordering

Do not implement destructive cleanup until the read-only scanner, safety model, and dashboard are stable and tested. Phase 01 is application foundation only: no providers, full-disk scanning, cleanup, monitor daemon, privileged helpers, Full Disk Access flow, or Windows implementation.
