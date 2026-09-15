# dev-storage-manager

> Understand what your development environment is doing to your disk.

`dev-storage-manager` is a free and open-source desktop application for developer storage intelligence. It is intended to explain which tools and projects consume space, how that usage changes, and what may be reclaimable without presenting developer data as anonymous "junk."

The project is currently **pre-alpha and in its product-foundation stage**. No application or cleanup functionality has been implemented yet.

## Product commitments

- Local-first filesystem analysis with no account or cloud requirement.
- macOS first, with Windows support planned behind a platform boundary.
- User-selected workspace directories; no fixed project-root convention.
- Quick scans for known developer locations and configured workspaces, plus explicitly requested deep scans.
- Local history for understanding growth over a day, week, or month.
- No automatic deletion or scheduled cleanup.
- Explanations and explicit user intent before every destructive action.
- Core-enforced `SAFE`, `REVIEW`, and `PROTECTED` classifications.

`SAFE` means data is considered regenerable; it never means automatic deletion. `REVIEW` means removal may disrupt a workflow or require restoration. `PROTECTED` items are not offered for normal deletion.

## Direction

The approved application direction is Tauri 2 with a React and TypeScript interface, a shared Rust core, SQLite for local configuration and history, and narrow platform adapters. Swift may be used behind the macOS adapter when an Apple API is materially cleaner or safer from Swift. Electron is not part of the architecture.

The intended boundaries are described in [docs/architecture.md](docs/architecture.md), and the complete product requirements are in [docs/product-spec.md](docs/product-spec.md).

## Current scope

Phase 00 establishes documentation only:

- product requirements and non-negotiable invariants;
- system boundaries and data flow;
- safety policy and provider contract;
- phased delivery order; and
- a detailed, unexecuted Phase 01 application-foundation plan.

See [docs/roadmap.md](docs/roadmap.md) for the release sequence. In particular, destructive cleanup does not begin until the read-only scanner, safety model, and dashboard are stable and tested.

## Contributing and security

Contributions are welcome once they follow the safety and architecture constraints in [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md). Please report security issues using the process in [SECURITY.md](SECURITY.md), especially issues involving path validation or destructive operations.

## License

The project currently uses the [MIT License](LICENSE). This choice is provisional until the first public release and may be reconsidered before initial publication.
