# Phase 01.5 verification — 2026-10-16

Implementation is available for review. This record distinguishes functional
verification, agent visual inspection, and human acceptance. The initial
Phase 01.5 evidence below is retained; the wide-window follow-up records the
subsequent human approval and the narrowly scoped layout correction.

```text
FUNCTIONAL QA: PASS
DIRECCIÓN VISUAL: APROBADA POR EL HUMANO.
AJUSTE DE VENTANA ANCHA: PENDIENTE DE CONFIRMACIÓN HUMANA.
```

## Automated checks

| Command                                                          | Result                                      |
| ---------------------------------------------------------------- | ------------------------------------------- |
| `npm run format:check`                                           | PASS                                        |
| `npm run lint`                                                   | PASS                                        |
| `npm run test:run`                                               | PASS, 28 tests in 6 files                   |
| `npm run build`                                                  | PASS; runs `tsc -b` before Vite             |
| `cargo fmt --all --check`                                        | PASS                                        |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS                                        |
| `cargo test --workspace --locked`                                | PASS, 10 Rust tests plus doc-test targets   |
| `npm run tauri build -- --debug`                                 | PASS; unsigned app and DMG produced locally |
| `git diff --check`                                               | PASS                                        |

Focused tests were run during each component task. Initial component runs
failed because the modules were absent; integration tests then demonstrated
three missing-shell failures before App integration. Stale resolve and reject
tests under StrictMode verify that an obsolete effect cannot replace the
current connection state. The original loading and error coverage remains.

An independent whole-branch review found one Important accessibility issue:
WebKit can suppress semantics for a markerless list outside navigation. The
native accessibility tree initially showed the Safety posture group as a
container. `DataList` now explicitly declares `role="list"`; its targeted
regression failed before the change and passes afterward. The final frontend
suite passes 28/28. After rebuilding and reopening the debug bundle, the native
accessibility tree exposes Safety posture as a content list. Actual VoiceOver
speech output remains unverified. No
critical or deferred minor issues were found by that review.

The first dev-server attempt was blocked by sandbox socket permissions, and
the first Clippy attempt could not unpack a cached Cargo dependency. Both were
rerun successfully with the required environment permissions. No manifest,
lockfile, dependency version, or toolchain was changed.

## Visual evidence

| Condition                | Native Tauri                                   | Browser                                                                                            | Automated tests       |
| ------------------------ | ---------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------- |
| Dark, default window     | Inspected debug bundle, screenshot             | Inspected at 1100 × 720                                                                            | Semantics covered     |
| Light, default window    | Pending                                        | Inspected at 1100 × 720                                                                            | Semantics covered     |
| Dark, minimum window     | Pending; resize attempts did not change window | Inspected at 760 × 520                                                                             | Semantics covered     |
| Light, minimum window    | Pending                                        | Inspected at 760 × 520                                                                             | Semantics covered     |
| Connected health/schema  | Inspected real IPC result                      | Not available outside Tauri                                                                        | Covered               |
| Loading health           | Pending                                        | Not visually reproduced                                                                            | Covered               |
| Unavailable health       | Pending                                        | Inspected real browser fallback                                                                    | Covered               |
| Keyboard focus/skip link | Visible focus, Return moved focus to main      | Keyboard/scroll structure inspected                                                                | Covered               |
| 200% zoom                | Pending                                        | Chrome showed 200%; minimum physical viewport yielded 380 × 260 CSS pixels, no horizontal overflow | Reading order covered |
| Increased contrast       | Pending                                        | DevTools emulation; stronger separator and 3px focus resolved                                      | Not a visual test     |
| Reduced motion           | Pending                                        | DevTools emulation; media query active and transition duration 0s                                  | Not a visual test     |

The native screenshot is the locally built debug bundle, with native titlebar
and `tauri://localhost` content. `npm run tauri dev` was also launched from the
normal checkout. A separately installed old application was initially resolved
by name; its screen was excluded from this evidence.

Browser appearance variants were emulated through DevTools, not by changing
macOS global preferences. Browser health is unavailable because browser pages
do not have Tauri IPC. No production mock, preview route, or test switch was
added. Browser viewport overrides and zoom were restored after inspection.

Screenshots are kept outside Git and supplied with the task handoff:

- `tauri-dark-default.png`
- `tauri-keyboard-focus.png`
- `browser-light-1100x720.jpg`
- `browser-light-760x520.jpg`
- `browser-dark-1100x720.jpg`
- `browser-dark-760x520.jpg`
- `browser-200-percent-minimum.jpg`

Some screenshot calls immediately after viewport changes returned stale scaled
frames. The four main browser captures were repeated after settled viewport
inspection; only those captures are offered as review artifacts.

## Scope audit

The complete feature diff is reviewed from branch base `ef52eb5`, including
design commit `aff133f` and implementation checkpoints. Backend, SQLite,
Tauri configuration/capabilities, bundle identifier, health bridge, manifests,
and lockfiles are unchanged. `get_app_health` remains the sole command and
`io.github.silviobig96.devstoragemanager` remains the identifier.

No workspace behavior, future route, filesystem scan, storage measurement,
provider, destructive operation, monitoring, networking, telemetry, permission
flow, Swift helper, or Windows implementation was added. Source-file removal
is limited to the obsolete frontend stylesheet. No screenshots, secrets,
personal paths, or local filesystem inventories are included in Git.

## Human review instructions

Run `npm run tauri dev` from the normal repository checkout. Inspect the native
application in light and dark appearances at its default and minimum sizes.
Check keyboard focus, Tab order, Return on the skip link, content scrolling,
200% text pressure, increased contrast, and reduced motion using your chosen
system accessibility settings. Do not infer native visual success from the
browser evidence or the build result.

Loading and unavailable native visual states were not reproduced. Their
functional behavior is covered by tests; visual acceptance of those states
remains pending. Do not introduce runtime mock modes solely to mark them done.

Human visual approval and CI must pass before merge. Publishing the authorized
feature branch does not complete Phase 01.5. Phase 02 has not started.

## Publication checkpoint

`git push -u origin codex/phase-01-5-design-foundation` succeeded. A subsequent
`git ls-remote --heads origin codex/phase-01-5-design-foundation` matched local
HEAD `f04f953` before this evidence-only record was added. The source and QA
changes were committed, the normal checkout was clean, and origin was unchanged.
No PR was opened and no merge or release was performed. The task handoff records
the final hash after publishing this documentation update as well.

## Wide-window follow-up — 2026-10-16

The human inspected the captures and running application and stated:
“me gusta el diseño, sí creo que está bien”. This approves the visual direction.
It does not approve unperformed native accessibility checks. Confirmation of
this width adjustment is still pending.

### Reproduction and cause

The normal checkout was clean on `codex/phase-01-5-design-foundation`, with the
expected unchanged origin. `npm run tauri dev` was invoked; it could not start
a second Vite instance because port 1420 was already occupied. Read-only
process checks confirmed an existing Tauri dev session and Vite server from
this same checkout, which were reused without terminating unrelated processes.

The rendered container chain was measured before editing:
`AppShell → primary-pane → content-pane → overview-view → sections/rows/empty-state`.
The grid gives the sidebar 224px at ordinary widths, with the primary pane
receiving the remainder. Main content uses 24px padding on each side. Width,
inline size, maximum size, margins, padding, grid columns, flex alignment, and
bounding rectangles were inspected.

The responsible rule was `src/views/OverviewView.css`, selector
`.overview-view`, declaration `max-width: 880px`. At a 1600 × 900 CSS viewport,
the primary pane was 1376px wide and had 1328px available after padding.
Overview stopped at 880px, leaving 448px unused on the right. Its sections,
separators, rows, and empty-state container inherited that cap. The empty
state's explanation was centered in the capped section rather than the whole
content pane. At 1100 × 720, the available 828px did not reach the cap, so the
problem was absent.

No 880px structural limit is specified in the design-system or design spec.
The separate `.empty-state p { max-width: 42ch; }` limits reading width and is
appropriate. It was preserved.

### Correction and renderer checks

Replaced the structural maximum with `.overview-view { inline-size: 100%; }`.
No other component or style changed. Typography, sidebar, icons, colors,
vertical spacing, and the health contract remain unchanged.

A one-off renderer assertion compared the Overview rectangle with main
content's available width (client width minus horizontal padding). It failed
before the fix with 448px unused, then passed afterward. Section, heading
separator, list, row, and empty-state rectangles also match the full available
width. The paragraph center differs from its section center by less than
0.01 CSS px, while its reading width stays approximately 317.46px.

| Browser CSS viewport | Available width | Overview before | Overview after, dark | Overview after, light |
| -------------------- | --------------- | --------------- | -------------------- | --------------------- |
| 760 × 520            | 544px           | Not measured    | 544px                | 544px                 |
| 1100 × 720           | 828px           | 828px           | 828px                | 828px                 |
| 1600 × 900           | 1328px          | 880px           | 1328px               | 1328px                |

Both appearances were inspected in Chrome, with light emulated through DevTools
only. No horizontal overflow occurred in the six cases. Overview's content
height remained approximately 522.96px across these sizes: no vertical stretch
was introduced. Existing child flex alignment and paragraph wrapping remain.

Chrome's visible zoom control confirmed 200%. At a minimum physical viewport
of 760 × 520, the effective CSS viewport was 380 × 260. The available content
width was approximately 218.80px (fractional rounding below 1px); Overview
filled it without horizontal overflow. Scrolling main to its 446px maximum
made the health status fully reachable, with its bottom at 243.90px inside
the 260px viewport. Browser zoom and viewport overrides were restored.

### Native application evidence

`npm run tauri build -- --debug` succeeded to produce the corrected local
bundle. It was reopened, moved using the native Window menu to the built-in
Retina display, and maximized with Window → Fill. The native application shows
the full-width separators, centered empty state, and actual connected health
result. The final screenshot contains only the application window.

Read-only evaluation in the detached native Web Inspector confirmed:

| Native measurement                            | Result       |
| --------------------------------------------- | ------------ |
| Actual CSS viewport                           | 1728 × 971   |
| `devicePixelRatio`                            | 2            |
| Main client width                             | 1504px       |
| Main horizontal padding                       | 24px + 24px  |
| Available interior width                      | 1456px       |
| Overview / section / row / empty-state widths | 1456px each  |
| Overview computed maximum width               | `none`       |
| Empty-state paragraph width                   | 317.453125px |

These dimensions came from the running WebView, not from dividing a screenshot
size by an assumed Retina factor. The inspector was closed after reading the
values. Native maximized inspection covered the system's dark appearance.
Native light, exact 760 × 520 and 1100 × 720 sizes, native 200% zoom, and the
previously pending accessibility conditions remain unverified in this follow-up.
Browser evidence is not presented as native inspection.

Captures and measurement JSON are outside Git in the task's temporary artifact
directory. The handoff supplies their actual paths. Main capture filenames:

- `browser-before-wide.jpg`
- `browser-after-dark-wide.jpg`
- `browser-after-light-wide.jpg`
- `tauri-wide-corrected.png`

### Follow-up functional gate and scope

- `npm run format:check`: PASS.
- `npm run lint`: PASS.
- `npm run test:run`: PASS, all 28 existing tests, including health states,
  stale effect results, keyboard access, and list semantics.
- `npm run build`: PASS, including `tsc -b`.
- `git diff --check`: PASS.
- Additional local native bundle build: PASS as recorded above.

This follow-up changes only `src/views/OverviewView.css` and this verification
record. No CSS-string or jsdom-only test is offered as layout evidence. No
backend, IPC, SQLite, capability, identifier, dependency, system preference,
or Phase 02 functionality changed. No overflow hiding, scale transform, zoom
workaround, viewport-relative typography, or artificial data was introduced.
Publication is limited to the already authorized feature branch; the final
commit and remote equality are reported in the task handoff.
