# Roadmap

The roadmap is ordered by risk. Each phase must meet its verification and safety gates before later work relies on it.

## Phase 00 — Product Foundation

Define the product, architecture, safety model, provider contract, contribution rules, security posture, roadmap, and Phase 01 implementation plan. Deliver documentation only.

## Phase 01 — Application Foundation

**Complete.** The Tauri 2, React, TypeScript, Vite, and Rust application shell, core/application/platform/persistence boundaries, SQLite migration and repository foundation, read-only health contract, tests, linting, formatting, local debug build, and macOS GitHub Actions CI are in place. This phase implements no scanning, providers, workspace behavior, cleanup, monitor, notifications, Windows support, privileged helpers, or Full Disk Access flow.

## Phase 02 — Workspace System

Implement user-selected workspace configuration for zero, one, or many paths, including validation, local persistence, and macOS selection UX. A workspace grants read-only analysis scope, not deletion authority.

## Phase 03 — Read-only Scanner Core

Implement bounded quick-scan coordination, explicit deep-scan requests, cancellation, progress, size accounting, diagnostics, and synthetic-filesystem tests. Establish protected policy enforcement before scanning real provider locations.

## Phase 04 — Developer Storage Providers

Implement and validate independent macOS providers in small groups using the common contract. Begin with narrow, well-understood cache sources before higher-risk runtimes, simulators, package artifacts, or Docker data.

## Phase 05 — Dashboard

Present category, workspace, project, size, classification, explanation, warnings, and reclaimable estimates. Clearly distinguish measured size from estimates and partial scans from complete scans.

## Phase 06 — Explicit Safe Cleanup

After the read-only scanner, safety model, and dashboard are stable and tested, implement the staged cleanup lifecycle for narrowly approved `SAFE` targets: plan, explain, select, confirm, revalidate, execute, verify, and record. No automatic or scheduled cleanup.

## Phase 07 — Project Intelligence

Detect project ecosystems and generated artifacts inside configured workspaces. Add evidence-based `Active`, `Recently used`, and `Likely inactive` labels without using inferred activity as deletion authority.

## Phase 08 — Local Storage History

Persist versioned local snapshots and show today, seven-day, and thirty-day change. Define retention and path minimization before storing longitudinal data.

## Phase 09 — Background Monitor / Notifications

Add lightweight `Off`, `Daily`, and `Weekly` quick-scan scheduling plus configurable local thresholds. The monitor can scan, compare, record, estimate, and notify, but has no cleanup capability.

## Phase 10 — Signed & Notarized macOS Release

Harden packaging, permissions messaging, privacy disclosures, update strategy, code signing, notarization, and release verification for the first supported macOS distribution.

## Phase 11 — Windows Platform Support

Implement a Windows adapter and Windows-specific providers while reusing the Rust core, React/TypeScript UI, persistence model, provider contracts, and safety policy.

## Ordering gates

- No destructive implementation precedes stable, tested read-only scanning, core safety policy, and dashboard explanations.
- No background monitor is introduced before manual scanning and local history are dependable.
- No platform-specific behavior is placed in shared core or UI code when an adapter can contain it.
- No phase adds automatic deletion, scheduled cleanup, default telemetry, or an assumed workspace path.
