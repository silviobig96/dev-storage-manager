# dev-storage-manager Design System

**Status:** Direction approved; implementation awaits human visual QA

**Platform:** macOS first; Windows adaptation planned

**Last reviewed:** 2026-10-16

This document is the durable visual and interaction contract for
`dev-storage-manager`. It translates the product's safety and local-first
principles into a restrained desktop interface. The Phase 01.5 design
specification records the decisions for the first implementation; this document
continues to govern later user-facing phases.

## Reference hierarchy

1. Product safety and architecture rules in `AGENTS.md`, `docs/safety-model.md`,
   and `docs/architecture.md`.
2. Apple's current Human Interface Guidelines for
   [designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos/),
   [sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars),
   [split views](https://developer.apple.com/design/human-interface-guidelines/split-views),
   [toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars),
   [typography](https://developer.apple.com/design/human-interface-guidelines/typography),
   [color](https://developer.apple.com/design/human-interface-guidelines/color),
   [Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode),
   [accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility),
   and [focus and selection](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection).
3. The rules in this document.

Apple's guidance is a source of platform conventions, not a request to copy a
specific Apple application. When a webview cannot inherit an AppKit semantic
value directly, the UI uses an equivalent semantic CSS token and validates it
in every supported appearance.

## Design character

The application is calm, precise, local, and trustworthy. It should look like
a professional utility that can remain open during development work rather than
a marketing page or administrative dashboard.

The interface favors:

- a stable sidebar and content hierarchy;
- compact rows, lists, and tables;
- system typography and tabular numerals for measurements;
- separators and alignment before cards or shadows;
- a small number of clearly prioritized controls;
- plain explanations of safety, uncertainty, and unavailable capabilities; and
- native-feeling selection, focus, hover, and disabled states.

The interface avoids:

- giant headlines, slogans, and website-style hero regions;
- dashboard card grids or cards nested inside cards;
- decorative gradients, blobs, glows, and broad glass effects;
- oversized radii, repeated pills, and heavy shadows;
- emoji as interface icons;
- color-only status communication; and
- invented measurements or UI that suggests a scan has occurred when it has
  not.

## Application structure

### Window

The native Tauri window and titlebar remain maintainable and conventional. The
default content size is `1100 × 720` CSS pixels and the minimum supported size
is `760 × 520`. Phase 01.5 does not implement custom traffic-light placement,
private titlebar APIs, or a Swift styling helper.

### Shell

The shell has two persistent regions:

1. A leading sidebar for top-level navigation.
2. A primary pane containing a compact toolbar/header and a scrollable content
   region.

The sidebar is approximately `224px` at the default window size and may narrow
to `184px` near the minimum width. A one-pixel semantic separator defines the
split. The primary pane owns content scrolling; the window must not become a
single vertically scrolling website.

The sidebar may become collapsible when navigation is implemented, but Phase
01.5 does not build fragile custom chrome to simulate a native split-view
controller. At minimum width, labels remain legible and the content pane keeps
the reading order. There is no mobile hamburger layout for the desktop window.

### Information architecture

The intended top-level order is:

1. Overview
2. Workspaces
3. Projects
4. Categories
5. History

Settings belongs in a visually separate utility group near the lower portion of
the sidebar when it becomes available. It must not be the only access point for
critical safety information or actions because a window's lower edge can be
obscured on macOS.

Phase 01.5 implements only the foundation view. Future destinations may appear
as visibly unavailable orientation cues, but they are not interactive routes
and cannot imply that Phase 02 or later features exist.

### Toolbar and content header

The toolbar or title-area region identifies the current view and holds only
frequent view-level actions. It is not a second navigation bar. Icon-only
controls require an accessible name and a tooltip; each must remain at least
`28 × 28px` in its ordinary state.

The content header carries a short view title and, only when useful, one line of
secondary context or a primary action. View titles use sentence or title case
consistently and avoid repeating the product name.

## Typography

Use the system font stack. Do not bundle or name a proprietary font file:

```css
font-family:
  -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', Arial,
  sans-serif;
```

Use regular, medium, semibold, and bold weights. Avoid thin weights. Keep the
number of levels small and preserve hierarchy in both appearances.

| Role             | Default size | Weight       | Line height | Use                                  |
| ---------------- | ------------ | ------------ | ----------- | ------------------------------------ |
| View title       | 24px         | 650 semibold | 1.2         | One per primary view                 |
| Important metric | 34px         | 600 semibold | 1.1         | Measured totals only, when available |
| Section title    | 14px         | 600 semibold | 1.35        | List/table section heading           |
| Body/control     | 13px         | 400–550      | 1.45        | Main copy and controls               |
| Secondary/meta   | 12px         | 400–500      | 1.4         | Supporting labels and timestamps     |
| Compact caption  | 11px         | 500–600      | 1.35        | Rare, nonessential metadata          |

Storage measurements use tabular numerals and align to support comparison.
Uppercase is reserved for the canonical safety labels `SAFE`, `REVIEW`, and
`PROTECTED`; headings are not letter-spaced uppercase decoration.

Text enlargement must not clip controls, hide meaning, or overlap adjacent
columns. At 200% browser zoom, the shell may reflow secondary metadata beneath
the primary label while preserving every action and status.

## Spacing and geometry

Use the following compact scale through semantic variables:

| Token     | Value | Typical use                         |
| --------- | ----- | ----------------------------------- |
| `space-1` | 2px   | Optical correction                  |
| `space-2` | 4px   | Icon/label micro-gap                |
| `space-3` | 6px   | Compact inline grouping             |
| `space-4` | 8px   | Control and row internals           |
| `space-5` | 12px  | Related groups                      |
| `space-6` | 16px  | Standard section and pane inset     |
| `space-7` | 20px  | Content header separation           |
| `space-8` | 24px  | Major section separation            |
| `space-9` | 32px  | Maximum ordinary content separation |

Default list rows are `36–44px` high according to content. Buttons and icon
buttons use compact desktop dimensions, with a `28px` minimum target. Corners
use approximately `5px` for controls and selections and `8px` for the few
elevated or grouped surfaces that require them. A larger radius needs a content
reason, not a style preference.

## Semantic color tokens

Components consume semantic tokens, never literal palette values. The initial
CSS variable contract is:

| Token                 | Purpose                                             |
| --------------------- | --------------------------------------------------- |
| `--surface-primary`   | Main content background                             |
| `--surface-secondary` | Sidebar and grouped background                      |
| `--surface-elevated`  | Popover-like or emphasized neutral surface          |
| `--surface-hover`     | Neutral hover feedback                              |
| `--text-primary`      | Main labels and values                              |
| `--text-secondary`    | Supporting explanation                              |
| `--text-tertiary`     | De-emphasized metadata                              |
| `--separator`         | Pane, row, and group separation                     |
| `--selection`         | Selected navigation/list surface                    |
| `--selection-text`    | Content on selected surface                         |
| `--accent`            | Interactive emphasis and focus relationship         |
| `--focus-ring`        | Keyboard focus indicator                            |
| `--status-safe`       | Supplemental SAFE indicator                         |
| `--status-review`     | Supplemental REVIEW indicator                       |
| `--status-protected`  | Supplemental PROTECTED indicator                    |
| `--status-error`      | Error indicator and critical feedback               |
| `--shadow-elevated`   | Restrained shadow for actual overlay/elevation only |

Each token has light and dark values. Increased-contrast media queries may
strengthen separators, selection, and focus without changing meaning. Status
colors are never reused as generic decoration.

### Implementation palette

Minor visual values concretized for Phase 01.5 (2026-10-16). These neutral
surfaces and blue selection/accent values are webview equivalents, not a claim
to inherit AppKit's dynamic accent color. Only the token stylesheet contains
production palette literals.

| Role              | Light     | Dark      |
| ----------------- | --------- | --------- |
| Primary surface   | `#ffffff` | `#202123` |
| Secondary surface | `#f3f3f5` | `#28292c` |
| Elevated surface  | `#ffffff` | `#323337` |
| Hover surface     | `#e9eaed` | `#37393e` |
| Primary text      | `#24262b` | `#f0f1f3` |
| Secondary text    | `#60636c` | `#b7bac3` |
| Tertiary text     | `#6b6f79` | `#a4a8b2` |
| Separator         | `#dcdde1` | `#45474d` |
| Selection         | `#dce8fa` | `#334865` |
| Selection text    | `#153d72` | `#e1edff` |
| Accent/focus      | `#215ca8` | `#9cc5ff` |
| Safe              | `#287044` | `#8ccea5` |
| Review            | `#875600` | `#e4be76` |
| Protected         | `#535e78` | `#b5c1dc` |
| Error             | `#b53232` | `#f1a0a0` |

Overlay shadow: `0 4px 16px rgb(0 0 0 / 12%)` in light appearance and
`0 4px 16px rgb(0 0 0 / 28%)` in dark appearance. No foundation surface needs
that shadow. Increased contrast uses stronger neutral separators (`#81858e`
light, `#9da2ad` dark), primary secondary-text roles, and a `3px` focus ring.

### Appearance behavior

The default is the operating system appearance through `color-scheme: light
dark` and `prefers-color-scheme`. Phase 01.5 does not add a persisted appearance
preference; following the system is the complete behavior for this phase.

Dark appearance is designed, not mechanically inverted. It uses distinct
surface levels, quieter shadows, legible muted text, and status colors checked
against their actual backgrounds. Neutral translucency is allowed for the
sidebar or a true elevated surface when it improves integration. Colored
status, error, and selection surfaces stay stable rather than picking up
unpredictable desktop tint.

## Icons

Use one restrained line-icon language at a consistent `16px` or `18px` visual
size. Phase 01.5 uses a small local SVG icon component for only the symbols the
foundation screen needs. This avoids a broad UI dependency and keeps the icon
set auditable. Icons use `currentColor`, inherit semantic state, and are hidden
from assistive technology when adjacent text already supplies the label.

Do not use emoji, mixed icon families, detailed illustrations, or color as the
only distinction. An unfamiliar icon-only action must include a tooltip in
addition to its accessible name.

## Component contract

The component set remains small. A component is created only when it owns a
repeatable semantic or interaction responsibility.

### `AppShell`

Owns the sidebar/content split, region landmarks, minimum-width behavior, and
content scrolling. It does not own routes, data fetching, or product policy.

### `Sidebar` and `SidebarItem`

Own navigation grouping, selected/unavailable presentation, and accessible
current-page state. Available items are keyboard-operable controls; future
items are noninteractive and explicitly labelled unavailable. Selection uses
shape, text, and icon treatment in addition to color.

### `Toolbar`

Owns the current-view title and a deliberately small set of view-level actions.
It does not duplicate the sidebar or hold marketing copy.

### `ContentHeader`

Owns the visible view title, optional concise description, and optional primary
action alignment.

### `Section`

Groups related content under a semantic heading. It normally uses whitespace
and separators rather than a filled rounded card.

### `DataList` and `ListRow`

Present comparable information with stable column alignment. Rows support a
primary label, optional secondary text, trailing value, and an optional status.
They preserve a sensible reading order when narrow or zoomed.

### `StatusIndicator`

Pairs a visible word with an icon or shape and supplemental semantic color.
Safety indicators always render the full text `Safe`, `Review`, or `Protected`
and never authorize an action.

### `EmptyState`

Explains why content is absent and what can happen next. It is compact and
embedded in the owning section, not a full-window marketing illustration.
Phase 01.5 uses it to state truthfully that storage analysis is not available
in the foundation release.

### `Button` and `IconButton`

Own shared control sizing, hierarchy, focus, hover, pressed, and disabled
states. Prefer a text button when words are clearer. Destructive styling is not
introduced in Phase 01.5.

## Interaction states

Every interactive component defines:

- **Rest:** clear label and stable target.
- **Hover:** subtle surface or text change; never the only affordance.
- **Pressed:** immediate, low-motion feedback.
- **Focus visible:** high-contrast ring or row highlight distinct from
  selection.
- **Selected/current:** persistent shape and semantic `aria-current` state.
- **Disabled/unavailable:** reduced emphasis plus explicit text or accessible
  description; no pointer-only explanation.
- **Busy:** prevents duplicate activation and announces progress if an action is
  actually in flight.

Transitions are limited to short color or opacity feedback. Under
`prefers-reduced-motion: reduce`, nonessential transitions and animations are
removed. No content state relies on animation.

## Content states

### Empty

Use one short title, one explanatory sentence, and at most one valid next
action. If the capability belongs to a later phase, do not render a fake or
disabled action; state that it is unavailable in this release.

### Loading

Keep the surrounding shell stable. Use concise live-region text or restrained
row placeholders. Avoid full-window spinners and layout shifts.

### Error

Place the message near the affected content, explain whether other information
is still valid, and offer a recovery action only when it works. Do not encode
the error only through red. The Phase 01.5 health failure is an inline status,
not a fatal full-window modal.

### Partial or uncertain

Later scan phases must state what was not measured and keep totals visibly
qualified. Uncertainty never changes a safety classification or creates
permission. This state is documented here but not implemented with fake scan
data in Phase 01.5.

### Workspace feedback

Workspace action feedback uses `FeedbackMessage`: a compact notice with a local
SVG icon, readable text, a subtly tinted surface and a restrained border.
Success, information, warning and error use separate `--feedback-*` foreground,
surface and border tokens in both appearances. These tokens describe operation
feedback, not the `SAFE`/`REVIEW`/`PROTECTED` storage safety classifications.

View-level confirmations appear before the workspace section, not as detached
footer text. They clear when the next action starts; there is no timed dismissal,
floating toast, notification infrastructure or motion. Errors remain visible
until another explicit operation changes the state; usable Retry/Reload controls
belong inside their notice. Uncertain outcomes retain their existing explanation
and reload gate rather than claiming rollback or success.

Root availability stays inside its row using the unfilled inline variant, with
a short label and supplemental explanation. `Available` means only that the root
was found; its visible caption states that contents have not been checked.
Missing/denied/unknown roots and overlaps have explicit warning text and icons.
A manual check updates the row's polite live region instead of adding a generic
success notice. Action confirmations are polite; request errors use `alert`.
Icons are decorative to assistive technology, and no meaning relies on color.
The feedback styles add no animation and strengthen borders at increased contrast.

## Safety presentation

Safety information appears in compact rows, with classification subordinate to
the item name and measurement:

```text
DerivedData                         4.5 GB
Regenerable                         Safe
```

`Safe`, `Review`, and `Protected` combine visible text with a small icon or
shape and semantic color. They do not become large traffic-light panels.
`Safe` never means automatic or preselected cleanup, and `Protected` never
offers a normal deletion action. All action eligibility remains enforced by the
Rust core in the phase that implements it.

## Accessibility requirements

- Use semantic landmarks, headings, lists, tables, buttons, and status regions.
- Preserve a logical DOM and keyboard order from sidebar to toolbar to content.
- Make every control keyboard-operable and give every icon-only control an
  accessible name.
- Use `aria-current="page"` for current navigation and live regions only for
  meaningful asynchronous status changes.
- Keep ordinary macOS control targets at least `28 × 28px` and never smaller
  than `20 × 20px` for a compact exceptional control.
- Maintain at least WCAG AA contrast in light and dark appearances, aiming for
  higher contrast for small secondary text.
- Communicate status with text and shape/icon, never color alone.
- Support reduced motion, increased contrast, keyboard focus, and 200% zoom.
- Do not disable zoom or hide overflow that contains focusable content.

## Responsive and minimum-window behavior

At `900px` and wider, the sidebar uses its default width and list rows keep
their primary and trailing columns. Between `760px` and `899px`, the sidebar
narrows, content insets tighten, and secondary row metadata may wrap beneath
the primary label. The sidebar remains visible in Phase 01.5 because the
minimum window accommodates both panes.

Vertical resizing keeps the toolbar fixed within the primary pane and lets the
content region scroll. Critical actions and status never depend on the bottom
edge remaining visible. Horizontal scrolling is not used for the shell; a
future dense table may own its own explicitly labelled scroll container.

## Platform adaptation boundary

The following can remain shared across platforms:

- information architecture and route meaning;
- component responsibilities and accessible semantics;
- product copy and safety vocabulary;
- data formatting and state models; and
- Rust-owned policy and application behavior.

The following may vary in a future Windows presentation layer:

- titlebar and toolbar integration;
- navigation chrome and selection shape;
- type metrics, spacing density, control geometry, and focus treatment;
- semantic token values, materials, shadows, and icon conventions; and
- platform-standard keyboard/menu behavior.

Windows must not imitate macOS. Phase 01.5 adds no Windows code and no
platform-conditional styling that cannot be validated yet.

## Visual QA gate

Every phase that changes user-facing UI has two independent gates:

1. **Functional QA:** automated tests, type checking, linting, formatting,
   production build, and applicable Rust/Tauri checks pass.
2. **Human visual QA:** a human inspects the running application and explicitly
   accepts the result.

Before human review, inspect at least:

- light and dark system appearances;
- default `1100 × 720` and minimum `760 × 520` window sizes;
- keyboard-only navigation and visible focus;
- loading, connected, and unavailable health states;
- 200% zoom and increased text pressure;
- reduced motion; and
- truthful labels for unavailable future functionality.

An agent may report:

```text
FUNCTIONAL QA: PASS
VISUAL QA: AWAITING HUMAN REVIEW
```

It may not report the UI phase complete until the human supplies `VISUAL QA:
PASS`. Human visual review supplements rather than replaces functional checks.

## Phase 01.5 boundary

Phase 01.5 implements only the presentation foundation described here and
retains the read-only health IPC. It does not implement workspace selection,
project discovery, filesystem or disk scanning, storage providers, real
category totals, cleanup, destructive operations, monitoring, telemetry,
networking, Full Disk Access, privileged helpers, Windows behavior, or Swift
helpers.

Representative content is permitted only when it is visibly labelled as a
design sample and cannot be mistaken for a measurement. The preferred
foundation screen uses truthful empty and status states instead of sample disk
totals.
