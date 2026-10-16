# Workspace feedback refinement verification

Date: 2026-10-16. Branch: `feat/phase-02-workspaces`.
Comparison base: `8827ab9`. Product commits: `f3bccc1`, `2bb85b8`.

The human approved the bounded in-chat design before implementation: compact
success/information/warning/error feedback, contextual availability, restrained
surfaces and borders, icons plus text, and useful recovery actions. This is not
a new phase, notification system or application redesign.

**FUNCTIONAL QA: PASS.**
**VISUAL QA: AWAITING HUMAN REVIEW.** Design approval is not approval of the
running implementation. Historical Phase 01.5/02 verification records remain
unchanged, and this refinement does not complete the open Phase 02 acceptance
checks or start Phase 03.

## Implementation and scope

- `FeedbackMessage` owns notice/inline presentation and alert/status semantics.
  New light/dark feedback tokens are separate from storage safety categories.
- Action notices appear before the workspace section. Confirmations clear on the
  next action; no automatic dismissal, floating toast or animation was added.
- Availability is a polite atomic row announcement, including hidden workspace
  identity for ambiguous same-name/concurrent cases. Visible full paths still
  wrap. `Available` explicitly says contents have not been checked.
- Missing, denied, unknown and overlapping locations have text and icons rather
  than color-only meaning. Manual checks no longer show detached generic success.
- Setup, list and removal errors retain their existing sanitized messages.
  Retry/Reload lives inside the associated notice. Uncertain outcomes retain
  their explanation and mutation gate; a new presentation cannot claim rollback.
- Existing navigation, command/reconciliation logic and native dialog focus
  lifecycle are unchanged. The extra setup recovery slot is presentation-only.
- No diff in Rust, SQLite/migrations, IPC, capabilities, bundle identifier,
  manifests, lockfiles or toolchain. No dependency, scan, user-file operation,
  monitoring, networking, telemetry or future-phase feature was introduced.

The [design system](../../design-system.md#workspace-feedback) records the rules.

## Automated checks

| Command                                                          | Result                                     |
| ---------------------------------------------------------------- | ------------------------------------------ |
| `npm run format:check`                                           | Exit 0                                     |
| `npm run lint`                                                   | Exit 0; no diagnostics                     |
| `npm run test:run`                                               | Exit 0; 11 files, 113 tests                |
| `npm run build`                                                  | Exit 0; includes `tsc -b`, Vite 48 modules |
| `cargo fmt --all --check`                                        | Exit 0                                     |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0                                     |
| `cargo test --workspace --locked`                                | Exit 0; 48 unit tests                      |
| `npm run tauri build -- --debug`                                 | Exit 0; fresh app and DMG                  |
| `git diff --check`                                               | Exit 0; also checked from comparison base  |

The final native build used approved execution outside the sandbox so the
packager could create its disk image. It included the current frontend and
completed the Rust build in 19.54 seconds, including a build-directory lock wait.
`plutil` confirmed `io.github.silviobig96.devstoragemanager` in the bundle.
The pre-existing npm `python` configuration warning remains; host configuration
was not changed. No product test/compiler failure remains.

TDD evidence:

- The initial three-file run failed six behavioral checks with 28 passing:
  missing contextual availability, misleading generic check success, missing
  error hierarchy/recovery association and generic saved-state feedback.
- Affected App/view/setup/dialog tests passed 51 tests after implementation.
  Copy assertions were updated to the approved labels without removing the
  existing stale-result, persistence, focus or uncertainty assertions.
- The identity suggestion from independent review was verified with a focused
  failing assertion before adding the workspace identity to the live region.
- Two focused recovery tests then failed because setup/dialog Reload remained
  outside the error. Moving the existing controls into the presentation slot
  produced 52 passing affected tests. The final complete frontend suite has 113.
- Independent code review and its scoped follow-up found no remaining findings.
  Review did not substitute for native inspection or human visual acceptance.

## Rendered browser checks

Production React/CSS was exercised through an ignored, separate synthetic IPC
fixture, not a product mock mode. Light/dark was emulated with frame color-scheme;
no OS preference changed. Actual CSS measurements after the accessible-identity
correction showed:

| CSS viewport | Appearances    | View/main interior width | Horizontal overflow      |
| ------------ | -------------- | ------------------------ | ------------------------ |
| 760 × 520    | Light and dark | 544 px                   | None in document or main |
| 1100 × 720   | Light and dark | 828 px                   | None in document or main |
| 1600 × 900   | Light and dark | 1328 px                  | None in document or main |

Success, available/missing/denied states, list failure with Retry, and failed
removal were visually inspected. Narrow dialogs retained their controls.
A synthetic lost-response/removal case showed Reload inside the error while
Remove remained disabled. These are browser simulations, not native SQLite
failure or native permission-denial evidence.

The intentionally hidden identity initially made a long nonwrapping label
contribute to document overflow. A scoped clip on only that screen-reader label
resolved the measured issue. Visible paths, structural panes and actions were
not clipped to conceal overflow. Their wrapping/scrolling remains available.

Calculated token-pair contrast for inline colored labels is at least 6.06:1;
secondary text against the notice surfaces is at least 5.48:1 across both
appearances. This is arithmetic on the defined colors, not a claim of measured
OS accessibility preferences or a screen-reader audit.

## Actual Tauri inspection

The exact freshly built app was opened from this checkout. Dark appearance was
inspected without changing global preferences, using only a controlled temporary
root and a sentinel created for verification:

- Native selection/add produced the new success notice and Available caption.
- A deliberate rename of that fixture, followed by manual Check, produced the
  contextual Folder unavailable warning with no generic success notice.
- Restore plus another explicit Check returned Available. This does not prove
  physical-volume disconnect/remount behavior.
- The unchanged removal confirmation retained exact safety copy and initial
  Cancel focus. Escape restored the actual Remove button in WKWebView.
- Confirming removal removed only that fixture's app configuration. Add received
  focus in the valid empty state. The directory and sentinel remained, with its
  SHA-256 unchanged. No personal workspace was selected, checked or modified.

Native screenshots and browser captures/measurements remain outside Git.
Native raster dimensions are not asserted as CSS viewport dimensions. Native
error/reload states were not artificially induced against the real database.

## Remaining human and environmental checks

Human inspection of this running refinement is pending. Native light/exact-size
matrix, actual 200% zoom, VoiceOver delivery/concurrent announcements, full dialog
focus-cycle acceptance, OS increased contrast and reduced motion remain unproven
in this run. Styles add no animation and provide an increased-contrast border
rule, but source inspection is not a visual OS-preference test.

This follow-up did not include a new push authorization; changes remain local.
No push, PR, merge, release or Phase 03 work was performed. Phase 02's pre-existing outstanding
acceptance checks remain outstanding.
