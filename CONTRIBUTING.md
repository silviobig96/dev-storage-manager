# Contributing

Thank you for helping build `dev-storage-manager`. The project is in a pre-alpha foundation stage, and safety is part of the product contract rather than a UI preference.

## Before contributing

Read these documents in order:

1. [docs/product-spec.md](docs/product-spec.md)
2. [docs/architecture.md](docs/architecture.md)
3. [docs/safety-model.md](docs/safety-model.md)
4. [docs/provider-contract.md](docs/provider-contract.md)
5. [AGENTS.md](AGENTS.md)

Phase 00 contains documentation only. Do not add application scaffolding or dependencies as part of a Phase 00 change.

## Non-negotiable rules

- Never add automatic or scheduled deletion.
- Never make a destructive filesystem operation available without an explicitly authorized, reviewed flow.
- Never downgrade a core `PROTECTED` classification in provider or UI code.
- Never infer deletion permission from inactivity, file age, size, or a `SAFE` label.
- Never hardcode a workspace convention such as `~/Repositories`.
- Never add telemetry or networking without an approved product requirement.
- Keep platform-specific behavior behind adapters and shared policy in Rust where practical.
- Do not introduce Electron.

## Development approach

- Keep changes small, focused, and independently reviewable.
- Use explicit interfaces and modules with one responsibility.
- Practice test-driven development when adding behavior: first demonstrate the missing behavior with a failing focused test, then implement the minimum change, then rerun the focused test.
- Run focused checks before broad suites.
- Add regression tests for fixes and tests at trust boundaries.
- Update the relevant specification when changing a product or safety invariant.
- Do not modify unrelated files or commit credentials, private paths, filesystem contents, or generated secrets.
- Verify claims with command output before describing work as complete.

## Changes and commits

Use concise, scoped commits. Explain the user-facing or architectural reason for the change in the pull request, including safety implications. Changes to cleanup planning, execution, path validation, classification, platform adapters, persistence, or permissions require focused security review.

Documentation should describe current behavior accurately. Do not claim planned features already exist, and avoid vague cleanup labels such as "junk."

## Reporting problems

Use normal issue tracking for reproducible non-sensitive bugs and proposals. Follow [SECURITY.md](SECURITY.md) for vulnerabilities or reports that could lead to unintended data loss, privacy exposure, or command execution.
