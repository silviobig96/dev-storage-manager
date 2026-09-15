# Contributing

Thank you for helping build `dev-storage-manager`. The project is in a pre-alpha foundation stage, and safety is part of the product contract rather than a UI preference.

## Before contributing

Read these documents in order:

1. [docs/product-spec.md](docs/product-spec.md)
2. [docs/architecture.md](docs/architecture.md)
3. [docs/safety-model.md](docs/safety-model.md)
4. [docs/provider-contract.md](docs/provider-contract.md)
5. [AGENTS.md](AGENTS.md)

Phase 01 implements only the macOS application-foundation shell and read-only health check. Do not describe or implement later-phase workspace selection, providers, scanning, cleanup, monitoring, notifications, privileged helpers, Full Disk Access behavior, or Windows support as part of a Phase 01 change.

## Prerequisites and setup

Development currently targets macOS. Install:

- the Xcode Command Line Tools;
- [nvm](https://github.com/nvm-sh/nvm); and
- the stable Rust toolchain with the `rustfmt` and `clippy` components.

From the repository root, select the pinned Node.js version, install locked dependencies, and launch the Tauri development application:

```bash
nvm use
npm ci
npm run tauri dev
```

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

## Verification

Run focused tests first. Before submitting a Phase 01 foundation change, run the complete local verification sequence from the repository root:

```bash
npm run test:run -- src/lib/desktop.test.ts src/App.test.tsx
cargo test -p dev-storage-core
cargo test -p dev-storage-platform
cargo test -p dev-storage-persistence
cargo test -p dev-storage-application
cargo test -p dev-storage-manager
npm run format:check
npm run lint
npm run test:run
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run tauri build -- --debug
git diff --check
```

`npm run tauri build -- --debug` builds an unsigned, unnotarized local debug application bundle. It does not perform a release or distribution step.

## Reporting problems

Use normal issue tracking for reproducible non-sensitive bugs and proposals. Follow [SECURITY.md](SECURITY.md) for vulnerabilities or reports that could lead to unintended data loss, privacy exposure, or command execution.
