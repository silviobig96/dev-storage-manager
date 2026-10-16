# Phase 01.5 macOS Design Foundation Specification

**Date:** 2026-09-15

**Status:** Direction approved for implementation; human visual QA pending

**Scope:** Design foundation and existing foundation-screen redesign only

## Summary

Phase 01.5 replaces the Phase 01 proof-of-foundation presentation with the
visual and structural foundation for a professional macOS developer utility.
It establishes a sidebar-based application shell, semantic design tokens,
system-driven light and dark appearances, compact reusable components, and a
truthful Overview foundation screen.

The phase changes the React presentation layer only. The existing read-only
`get_app_health` request remains the sole application capability. No Phase 02
or later behavior is introduced.

The durable visual rules live in
[`docs/design-system.md`](../../design-system.md). This specification records
the decisions and acceptance boundary for the Phase 01.5 implementation.

## Context and problem

Phase 01 proved that the Tauri shell, React frontend, layered Rust workspace,
SQLite migration path, and read-only health IPC work together. Its screen is a
temporary product statement rather than a desktop application structure:

- a centered web-page frame;
- a large marketing headline;
- a decorative gradient canvas;
- a translucent rounded status card; and
- a three-column commitments grid.

That composition does not provide the navigation, information density, row
patterns, or platform hierarchy needed by Workspaces and later storage views.
Extending it would produce a generic dashboard of cards. Phase 01.5 corrects
the presentation foundation before product workflows are added.

## Design goals

- Feel at home on macOS through hierarchy, density, alignment, typography,
  selection, focus, and window behavior.
- Create an application shell that can receive Workspaces, Projects,
  Categories, and History without redesigning navigation.
- Present information primarily through sections, lists, rows, and future
  tables rather than card grids.
- Use semantic tokens so light, dark, and future platform adaptations are
  coherent.
- Keep the component system focused and explicit.
- Preserve truthful product state: Phase 01.5 has a health connection and no
  storage analysis.
- Make keyboard access, contrast, reduced motion, and non-color meaning part of
  the foundation rather than later remediation.
- Keep native-window integration maintainable within Tauri.

## Anti-goals

Phase 01.5 does not create:

- a SaaS dashboard, marketing landing page, or generic admin template;
- a collection of large storage statistic cards;
- fragile custom traffic lights or titlebar chrome;
- a broad animation or glass-effect system;
- a heavyweight component framework;
- a full design-system showcase inside the product;
- persisted appearance preferences;
- fake scan results or unlabeled mock measurements; or
- a platform-neutral visual style that forces Windows to imitate macOS.

## Locked product and safety boundaries

The following are unchanged:

- No automatic deletion exists.
- No destructive filesystem operation exists.
- `SAFE`, `REVIEW`, and `PROTECTED` remain the only conceptual safety levels;
  color supplements their text and never grants action eligibility.
- Rust remains the future authority for safety classification and actions.
- No filesystem, workspace, project, disk, or developer-directory scanning is
  added.
- No provider, cleanup, monitoring, Full Disk Access, privileged helper,
  telemetry, networking, Windows implementation, or styling-only Swift helper
  is added.
- Electron remains excluded.
- The command registry remains limited to `get_app_health`.

## macOS visual principles

The implementation follows Apple's current Human Interface Guidelines for
macOS, sidebars, split views, toolbars, typography, adaptive color, Dark Mode,
focus, and accessibility. It adopts the following principles without copying a
specific Apple application:

- Use the available desktop width to show top-level navigation and content at
  the same time.
- Keep navigation selection persistent so location is obvious.
- Use a concise view title rather than the application name in content chrome.
- Favor system typography, semantic colors, separators, and compact controls.
- Expose keyboard focus clearly and preserve a logical navigation order.
- Follow the operating system appearance rather than inventing an in-app theme
  setting during this phase.
- Use material-like translucency only on a neutral surface where it improves
  integration; do not make every surface glass.

## Information architecture

The target top-level architecture is:

```text
Overview
Workspaces
Projects
Categories
History

Settings
```

Only Overview has content in Phase 01.5. The shell can show later destinations
as noninteractive, visibly unavailable orientation cues. They must not use
anchor or button semantics, respond to activation, or suggest their features
are installed. Settings is treated the same way until an approved phase owns
it.

The ordering is stable enough for Phase 02 to add Workspaces without changing
the shell. Deeper future detail views use a content list or detail pane rather
than adding nested levels to the top-level sidebar.

## App shell

### Window contract

- Keep the native Tauri window and titlebar.
- Preserve the Phase 01 default size of `1100 × 720` and minimum size of
  `760 × 520`.
- Do not use private macOS APIs, custom traffic-light positioning, or a Swift
  helper for appearance.

### Layout

`AppShell` renders a two-pane split:

```text
+----------------------+---------------------------------------------+
| Sidebar              | Toolbar / current view                     |
|                      +---------------------------------------------+
| Overview             | Content header                              |
| Workspaces           |                                             |
| Projects             | Section                                     |
| Categories           |   compact rows / empty or status state       |
| History              |                                             |
|                      | Section                                     |
| Settings             |                                             |
+----------------------+---------------------------------------------+
```

The sidebar is approximately `224px` at normal width and narrows to `184px`
near the minimum. The primary pane contains a compact toolbar/title region and
owns vertical content scrolling. A semantic separator, not a shadowed card,
defines the pane boundary.

The toolbar identifies Overview and may contain a sidebar control if the shell
later supports collapsing. It contains no decorative copy and no control that
does not work. The content header uses a `24px` title and at most one concise
supporting line.

## Phase 01.5 Overview

The redesigned screen communicates only facts that exist:

1. **Foundation health:** loading, connected with database schema version, or
   unavailable. This remains an inline, polite status region.
2. **Storage overview:** a compact empty state explaining that storage analysis
   is not available in the foundation release. It does not show a scan button,
   sample total, or simulated progress.
3. **Safety posture:** a restrained list of durable facts such as local-first
   behavior and no automatic deletion. These are rows with short descriptions,
   not promotional cards.

The view avoids the working product slogan as a hero headline. The content
title is simply `Overview`. Copy is direct, short, and utility-oriented.

## Typography

The implementation uses the system font stack and the scale in
`docs/design-system.md`:

- view title: `24px`, semibold;
- important measured metric: `34px`, semibold, reserved until real data exists;
- section title: `14px`, semibold;
- body/control: `13px`;
- secondary/meta: `12px`; and
- compact caption: `11px` when essential.

The UI uses tabular numerals for versions and future measurements. It does not
use broad uppercase eyebrow labels, decorative tracking, ultra-light weights,
or website-scale responsive headlines.

## Spacing and geometry

The semantic spacing scale is `2, 4, 6, 8, 12, 16, 20, 24, 32px`. Normal rows
are `36–44px` high. Content insets are `20–24px` at default width and `16px` at
minimum width. Ordinary controls use approximately `5px` corner radii; selected
rows use a similarly restrained radius. Grouped surfaces use up to `8px` only
when a surface is actually needed.

Shadows are reserved for real elevation such as a future menu or popover.
Sections in the primary pane use headings, whitespace, and separators rather
than independent rounded cards.

## Color and token strategy

The frontend replaces `--canvas`, `--ink`, and other appearance-specific names
with semantic roles:

- surfaces: primary, secondary, elevated, and hover;
- text: primary, secondary, and tertiary;
- structure: separator, selection, and selection text;
- interaction: accent and focus ring;
- status: safe, review, protected, and error; and
- elevation: one restrained overlay shadow.

Literal color values live only in token definitions. Components consume the
semantic variables. Light values are the default and dark values are defined
under `prefers-color-scheme: dark`. `prefers-contrast: more` strengthens
separators, focus, and selection. `color-scheme: light dark` lets native web
controls follow the system.

Dark appearance is separately tuned rather than inverted. Small text must meet
WCAG AA contrast against the surface where it appears. Status colors are paired
with text and an icon or shape, and are never the only source of meaning.

## Theming

Phase 01.5 supports three appearance outcomes through one policy:

- macOS Light → light semantic tokens;
- macOS Dark → dark semantic tokens; and
- macOS Auto/System → the current system preference.

There is no application preference, persistence model, Rust command, or theme
toggle. Adding one would exceed the phase and contradict the preferred macOS
behavior without a specific product need.

## Component foundation

The implementation creates only components used by the new screen:

| Component                 | Responsibility                                                      |
| ------------------------- | ------------------------------------------------------------------- |
| `AppShell`                | Sidebar/content regions, shell landmarks, and window-width behavior |
| `Sidebar` / `SidebarItem` | Navigation grouping, current and unavailable states                 |
| `Toolbar`                 | Concise current-view title and valid view controls                  |
| `ContentHeader`           | View title, supporting text, optional action alignment              |
| `Section`                 | Semantic grouping with a compact heading                            |
| `DataList` / `ListRow`    | Aligned row presentation and responsive reading order               |
| `StatusIndicator`         | Text + shape/icon + supplemental status color                       |
| `EmptyState`              | Compact truthful absence explanation                                |
| `Button` / `IconButton`   | Shared control hierarchy, sizing, and interaction states            |
| `AppIcon`                 | One local, line-based SVG icon language                             |

If implementation shows that a listed primitive has only one trivial use and
adds indirection, keep the markup in its owning feature instead. This is not a
component-count requirement.

No UI framework or icon package is added. `AppIcon` contains only the small set
of `currentColor` SVG symbols needed by Phase 01.5 so the project does not
adopt multiple overlapping icon systems.

## Interaction states

Interactive elements implement rest, hover, pressed, visible focus, selected,
disabled, and busy states where applicable. Navigation uses `aria-current` for
the current view. Future destinations are not disabled buttons; they are
noninteractive rows whose visible `Unavailable` text or accessible description
explains their state.

Hover never replaces focus or selected styling. Focus uses a clear ring or
full-row highlight. Icon-only buttons have an accessible name and tooltip.
Pressed feedback uses color/opacity only and does not shift layout.

## Empty, loading, and error states

### Empty

The storage section says that no storage analysis is available in this
foundation release. It does not imply the user needs to configure a workspace
or that a scan ran with zero results.

### Loading

The shell renders immediately while the health query is pending. The health
row uses concise live-region text and a neutral status indicator. It does not
block navigation or display a full-window spinner.

### Connected

The health row reports that the foundation is connected and displays the
returned schema version as secondary metadata. This is application health, not
a storage-analysis success state.

### Error

The health row reports that the desktop core is unavailable and recommends
restarting the app. Other static foundation information remains visible. The
error uses text and an icon/shape in addition to semantic color.

## Safety presentation

Future safety rows use full labels and restrained secondary indicators:

```text
node_modules                       58.3 GB
67 projects                        Review
```

Classification never becomes a large colored card. `Safe` is not presented as
permission or automatic selection; `Protected` is visually and semantically
non-actionable. Phase 01.5 may demonstrate the `StatusIndicator` in component
tests, but the running Overview does not invent actual storage findings.

## Accessibility

The implementation must provide:

- semantic `aside`/`nav`, toolbar/header, `main`, sections, headings, lists,
  buttons, and status regions;
- one clear page/view heading;
- logical DOM order matching the visual layout;
- keyboard-operable active controls and no focus traps;
- visible focus distinct from selected state;
- accessible names and tooltips for icon-only controls;
- `aria-current="page"` for Overview;
- a polite live region for asynchronous health changes;
- at least WCAG AA contrast in light and dark modes;
- visible text or an accessible description for every status;
- stable behavior at 200% zoom; and
- `prefers-reduced-motion` and `prefers-contrast` handling.

## Responsive and minimum-window behavior

The desktop layout is tested at the default `1100 × 720` size and minimum
`760 × 520` size.

At narrower widths:

- the sidebar reduces from about `224px` to `184px`;
- content insets reduce to `16px`;
- secondary row metadata can wrap below the primary label;
- trailing values remain associated with their row; and
- the primary content pane scrolls vertically without causing whole-window
  horizontal overflow.

The Phase 01.5 sidebar stays present at the supported minimum. A future
show/hide-sidebar behavior requires a functioning control and keyboard/menu
path; it is not simulated in this phase.

## Future Windows adaptation boundary

Shared React code may retain information architecture, state models, safety
language, component semantics, and accessible behavior. A future Windows layer
may replace token values, chrome integration, spacing, type metrics, selection,
control geometry, focus, icons, and keyboard/menu conventions.

No macOS material, fixed sidebar icon color, or traffic-light assumption is
encoded as a universal product rule. No Windows styles or platform detection
are added in Phase 01.5.

## Phase 02 workspace decision

Workspaces are optional and remain unimplemented until Phase 02:

- configuration supports zero through many user-selected folders;
- first run offers **Add workspace** and **Skip for now**;
- global providers may operate with zero workspaces; and
- no code or copy assumes `~/Repositories` or any default project root.

The Phase 01.5 empty state must not pressure the user to add a workspace or
model the absence of a workspace as an error.

## Visual QA gate

Any phase that changes user-facing UI requires both:

```text
FUNCTIONAL QA: PASS
VISUAL QA: PASS
```

Automated tests, builds, lint, formatting, and agent inspection can satisfy only
functional QA. Until a human inspects the running app in light and dark modes,
at default and minimum window sizes, and with keyboard focus, report:

```text
FUNCTIONAL QA: PASS
VISUAL QA: AWAITING HUMAN REVIEW
```

The phase cannot be called complete, merged, or used as a visual precedent
until the human gives explicit visual approval. Visual approval does not waive
functional failures.

## Acceptance criteria

Phase 01.5 implementation is ready for human visual review when:

- the marketing-page composition is replaced by the specified desktop shell;
- Overview is the only active destination and future destinations are truthful;
- the health query retains loading, connected, and error behavior;
- no fake storage measurements or Phase 02 interactions appear;
- semantic tokens supply light, dark, increased-contrast, focus, and safety
  colors;
- the component responsibilities remain focused and accessible;
- default and minimum sizes, 200% zoom, keyboard navigation, and reduced motion
  are verified;
- frontend tests, type checks, lint, formatting, production build, Rust/Tauri
  checks, and `git diff --check` pass;
- source and dependency audits confirm no Electron, networking, telemetry,
  destructive behavior, workspace default, or Windows implementation; and
- the running result is handed to the human with
  `VISUAL QA: AWAITING HUMAN REVIEW`.

Phase 01.5 is complete only after the human returns `VISUAL QA: PASS` and any
requested visual corrections also pass functional verification.
