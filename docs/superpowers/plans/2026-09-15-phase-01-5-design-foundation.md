# Phase 01.5 macOS Design Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the temporary Phase 01 presentation with an accessible,
macOS-first application shell and focused component foundation without adding
Phase 02 or later behavior.

**Architecture:** Keep `App` responsible for the existing read-only health
query, move presentation into a two-pane shell and an Overview view, and make
components consume semantic CSS tokens. Shared components own structure and
interaction semantics; macOS presentation lives in token and style layers that
a future Windows implementation can replace without changing product meaning.

**Tech Stack:** Tauri 2, React 19, TypeScript 6, Vite 8, CSS custom properties,
Vitest, Testing Library, Rust workspace health IPC. No UI framework and no icon
package are added.

**Spec:**
[`docs/superpowers/specs/2026-09-15-phase-01-5-design-foundation-design.md`](../specs/2026-09-15-phase-01-5-design-foundation-design.md)

## Global constraints

- Work on `codex/phase-01-5-design-foundation` in the ordinary repository
  checkout. Do not create or use a worktree.
- Preserve the existing remote. This implementation task authorizes pushing only
  `codex/phase-01-5-design-foundation` after functional gates and scope review.
  Human visual QA may remain pending. Do not create a PR, merge, or release.
- Keep `get_app_health` as the only Tauri command and the only runtime data
  request.
- Do not modify Rust application behavior, SQLite schema, Tauri capabilities,
  or the macOS bundle identifier.
- Do not add workspaces, routes for later features, project discovery, scanning,
  providers, real storage categories, cleanup, destructive operations,
  monitoring, Full Disk Access, privileged helpers, telemetry, networking,
  Windows behavior, Electron, or Swift helpers.
- Do not assume `~/Repositories` or any project-root convention.
- Do not present representative content as measured storage data. Prefer a
  truthful empty state.
- Use focused red-green-refactor cycles for component behavior. Pure token and
  layout declarations that jsdom cannot validate are verified by production
  build plus visual QA rather than brittle source-text tests.
- Run focused tests after each behavioral change and broad gates only at the
  checkpoints below.
- A functional pass does not complete the phase. Stop at
  `VISUAL QA: AWAITING HUMAN REVIEW` until the human inspects the running app.

---

## Task 1: Establish semantic visual tokens and global desktop defaults

**Files:**

- Create: `src/styles/tokens.css`
- Create: `src/styles/base.css`
- Modify: `src/main.tsx`
- Modify: `src/index.css`

- [x] **Step 1: Record the existing baseline**

Run:

```bash
npm run test:run -- src/App.test.tsx
npm run build
```

Expected: both commands exit zero before the style foundation changes.

- [x] **Step 2: Create the semantic token layer**

Create `src/styles/tokens.css` with the exact token families defined in
`docs/design-system.md`:

Use the concrete light/dark palette in the implementation values table in
`docs/design-system.md`. Keep `color-scheme: light dark`, the documented
`2–32px` spacing scale, `5px` control radius, and `8px` surface radius. The
production source of truth is `src/styles/tokens.css`; do not duplicate the
whole stylesheet here.

Define separately tuned dark values under
`@media (prefers-color-scheme: dark)` and increased-contrast overrides under
`@media (prefers-contrast: more)`. Literal palette values remain in this file
only.

- [x] **Step 3: Create global desktop defaults**

Create `src/styles/base.css` with:

- the system font stack;
- border-box sizing;
- full-height `html`, `body`, and `#root`;
- the primary semantic surface and text colors;
- inherited form-control typography;
- a visible `:focus-visible` rule using `--focus-ring`;
- tabular-numeral utility behavior for measurements and versions; and
- a `prefers-reduced-motion: reduce` rule that removes nonessential motion.

Do not add gradients, decorative backgrounds, or a mobile page layout.

- [x] **Step 4: Wire styles through the entry point**

Update `src/main.tsx` to import `styles/tokens.css` and `styles/base.css` before
rendering. Reduce `src/index.css` to a compatibility import or remove its old
palette and gradient declarations, leaving no duplicated token source.

- [x] **Step 5: Verify compilation and formatting**

Run:

```bash
npm run build
npm run format:check
```

Expected: both commands exit zero. Visual values are not considered accepted
until Task 7 human review.

- [x] **Step 6: Commit the token foundation**

```bash
git add src/main.tsx src/index.css src/styles/tokens.css src/styles/base.css
git commit -m "style: add macOS semantic design tokens"
```

---

## Task 2: Add the focused icon, control, and status primitives

**Files:**

- Create: `src/components/ui/AppIcon.tsx`
- Create: `src/components/ui/Button.tsx`
- Create: `src/components/ui/IconButton.tsx`
- Create: `src/components/ui/StatusIndicator.tsx`
- Create: `src/components/ui/controls.css`
- Create: `src/components/ui/ui.test.tsx`

- [x] **Step 1: Write focused failing accessibility tests**

Create `src/components/ui/ui.test.tsx` with tests that require:

```tsx
it('gives an icon-only button an accessible name and tooltip', () => {
  render(<IconButton label="Show sidebar" icon="sidebar" />);
  expect(screen.getByRole('button', { name: 'Show sidebar' })).toHaveAttribute(
    'title',
    'Show sidebar',
  );
});

it('renders status meaning as visible text', () => {
  render(<StatusIndicator tone="protected">Protected</StatusIndicator>);
  expect(screen.getByText('Protected')).toBeVisible();
});

it('keeps decorative icons out of the accessibility tree', () => {
  const { container } = render(<AppIcon name="overview" />);
  expect(container.querySelector('svg')).toHaveAttribute('aria-hidden', 'true');
});
```

Run:

```bash
npm run test:run -- src/components/ui/ui.test.tsx
```

Expected: FAIL because the components do not exist.

- [x] **Step 2: Implement one local line-icon language**

Create `AppIcon` with a closed TypeScript union for only the symbols needed by
Phase 01.5:

```ts
type AppIconName =
  | 'overview'
  | 'workspaces'
  | 'projects'
  | 'categories'
  | 'history'
  | 'settings'
  | 'sidebar'
  | 'info'
  | 'check'
  | 'warning'
  | 'lock';
```

Render consistent `currentColor` SVGs with an `18 × 18` view, round line caps,
and `aria-hidden="true"`. Do not add another icon source or embed emoji.

- [x] **Step 3: Implement controls and status**

Implement:

- `Button` with `primary` and `secondary` visual variants;
- `IconButton` requiring `label` and `icon` props and applying both
  `aria-label` and `title`;
- `StatusIndicator` with `neutral`, `safe`, `review`, `protected`, and `error`
  tones; and
- compact rest, hover, pressed, focus-visible, and disabled styles in
  `controls.css`.

Every status renders children as visible text. Color and iconography supplement
that text. Do not add a destructive button variant.

- [x] **Step 4: Make the focused tests pass**

Run:

```bash
npm run test:run -- src/components/ui/ui.test.tsx
```

Expected: PASS.

- [x] **Step 5: Verify the focused component checkpoint**

Run:

```bash
npm run lint
npm run build
```

Expected: both commands exit zero.

- [x] **Step 6: Commit the UI primitives**

```bash
git add src/components/ui
git commit -m "feat: add accessible interface primitives"
```

---

## Task 3: Build the macOS application shell and sidebar navigation

**Files:**

- Create: `src/components/shell/AppShell.tsx`
- Create: `src/components/shell/AppShell.css`
- Create: `src/components/shell/Sidebar.tsx`
- Create: `src/components/shell/SidebarItem.tsx`
- Create: `src/components/shell/Sidebar.css`
- Create: `src/components/shell/Toolbar.tsx`
- Create: `src/components/shell/Toolbar.css`
- Create: `src/components/shell/shell.test.tsx`

- [x] **Step 1: Write the shell contract tests**

Add tests that render the shell and require:

- a navigation landmark named `Main`;
- Overview to expose `aria-current="page"`;
- Workspaces, Projects, Categories, History, and Settings to be visible with an
  unavailable description but absent from the button/link role query;
- one `main` landmark for view content;
- a toolbar/header with the short title `Overview`; and
- sidebar content to precede main content in DOM order.

Run:

```bash
npm run test:run -- src/components/shell/shell.test.tsx
```

Expected: FAIL because the shell components do not exist.

- [x] **Step 2: Implement `SidebarItem` semantics**

`SidebarItem` supports two deliberate states:

- `current`: a non-routing element with `aria-current="page"` for the only
  implemented view; and
- `unavailable`: a noninteractive row with visible or screen-reader-accessible
  `Unavailable` text.

Do not create placeholder click handlers, empty anchors, fake route state, or
disabled buttons for later phases.

- [x] **Step 3: Implement `Sidebar`**

Render the locked information architecture in order. Keep Overview selected,
group Settings separately, and use the local line icons. Apply compact macOS
row spacing and a persistent selection surface using text, shape, and icon
treatment rather than color alone.

- [x] **Step 4: Implement `Toolbar` and `AppShell`**

`AppShell` owns the two-pane landmark structure and the scrollable primary
content region. `Toolbar` displays the current view title. Do not render a
nonfunctioning collapse button; the sidebar stays visible at the Phase 01.5
minimum width.

Use a semantic separator between panes. Avoid card wrappers, broad shadows, or
custom titlebar positioning.

- [x] **Step 5: Make the shell tests pass**

Run:

```bash
npm run test:run -- src/components/shell/shell.test.tsx
```

Expected: PASS.

- [x] **Step 6: Verify and commit the shell**

Run:

```bash
npm run lint
npm run build
```

Then commit:

```bash
git add src/components/shell
git commit -m "feat: establish macOS application shell"
```

---

## Task 4: Build content primitives and the truthful Overview view

**Files:**

- Create: `src/components/content/ContentHeader.tsx`
- Create: `src/components/content/Section.tsx`
- Create: `src/components/content/DataList.tsx`
- Create: `src/components/content/ListRow.tsx`
- Create: `src/components/content/EmptyState.tsx`
- Create: `src/components/content/content.css`
- Create: `src/components/content/content.test.tsx`
- Create: `src/views/OverviewView.tsx`
- Create: `src/views/OverviewView.css`
- Create: `src/views/OverviewView.test.tsx`

- [x] **Step 1: Write failing content-primitive tests**

Require that:

- `ContentHeader` provides the visible `Overview` level-one heading;
- `Section` connects its heading to its region;
- `DataList` and `ListRow` preserve primary label, secondary explanation, and
  trailing metadata in DOM reading order; and
- `EmptyState` exposes a heading and explanation without requiring an action.

Run:

```bash
npm run test:run -- src/components/content/content.test.tsx
```

Expected: FAIL because the components do not exist.

- [x] **Step 2: Implement the content primitives**

Use semantic headings, sections, and lists. Style grouping with alignment,
spacing, and separators. Do not wrap every component in a rounded filled
surface.

`ListRow` accepts a primary label, optional secondary text, and optional
trailing content. Its DOM order must remain meaningful when CSS stacks
secondary content at narrow widths.

- [x] **Step 3: Make the content tests pass**

Run:

```bash
npm run test:run -- src/components/content/content.test.tsx
```

Expected: PASS.

- [x] **Step 4: Write failing Overview behavior tests**

`OverviewView.test.tsx` supplies explicit health-state props and requires:

- the title `Overview` without the Phase 01 marketing headline;
- loading text in a polite status region;
- connected text plus schema version;
- unavailable text with a recovery instruction;
- a storage empty state that says analysis is unavailable in the foundation
  release;
- local-first and no-automatic-deletion commitments as compact rows; and
- no scan, add-workspace, cleanup, or measured-storage action.

Run:

```bash
npm run test:run -- src/views/OverviewView.test.tsx
```

Expected: FAIL because `OverviewView` does not exist.

- [x] **Step 5: Implement `OverviewView`**

Accept the existing health union as a prop and compose:

1. `ContentHeader` with the short title and one restrained description.
2. A Foundation status section using a compact `ListRow` and
   `StatusIndicator`.
3. A Storage overview section using `EmptyState` and no fake measurement or
   action.
4. A Safety posture section containing concise local-first and no-automatic-
   deletion rows.

Do not include the previous eyebrow, large slogan, gradient, commitments card
grid, or representative disk totals.

- [x] **Step 6: Make the Overview tests pass and commit**

Run:

```bash
npm run test:run -- src/views/OverviewView.test.tsx src/components/content/content.test.tsx
```

Expected: PASS.

Then commit:

```bash
git add src/components/content src/views
git commit -m "feat: add truthful overview foundation view"
```

---

## Task 5: Integrate the shell while preserving the read-only health path

**Files:**

- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Delete: `src/App.css`

- [x] **Step 1: Rewrite the app-level tests first**

Update `src/App.test.tsx` to require the integrated shell while retaining all
three existing health outcomes:

Write real callbacks for loading, connected schema version, unavailable core,
and late resolution after unmount. Use controlled promises and React `act` to
settle asynchronous updates. Assert visible shell/status output and verify that
an old request cannot replace a newly mounted instance's state. Include late
rejection coverage as well as resolution.

Also assert:

- Overview is current;
- the marketing headline is absent;
- `getAppHealth` is called once;
- no action named Scan, Clean, Delete, Add workspace, or Grant Access exists;
  and
- no mock byte measurement is rendered.

Run:

```bash
npm run test:run -- src/App.test.tsx
```

Expected: FAIL because `App` still renders the Phase 01 page.

- [x] **Step 2: Integrate `AppShell` and `OverviewView`**

Keep the current `HealthState` union and cancellation guard in `App.tsx`.
Replace the old presentation with:

```tsx
<AppShell currentView="overview">
  <OverviewView healthState={healthState} />
</AppShell>
```

Do not change `src/lib/desktop.ts`, the Tauri command, Rust crates, or the
database schema.

- [x] **Step 3: Remove obsolete page styling**

Delete `src/App.css` after every needed style has moved to the focused shell,
content, view, or global style file. Remove its import from `App.tsx`. Confirm
that no gradient, hero, commitment-grid, oversized headline, or old literal
palette declaration remains in frontend source.

- [x] **Step 4: Make app integration tests pass**

Run:

```bash
npm run test:run -- src/App.test.tsx src/lib/desktop.test.ts
```

Expected: PASS. The existing desktop bridge contract remains unchanged.

- [x] **Step 5: Run the frontend checkpoint**

Run:

```bash
npm run format:check
npm run lint
npm run test:run
npm run build
```

Expected: every command exits zero.

- [x] **Step 6: Commit the integrated redesign**

```bash
git add src/App.tsx src/App.test.tsx src/App.css
git commit -m "feat: integrate macOS design foundation"
```

---

## Task 6: Harden accessibility, appearance, and minimum-window behavior

**Files:**

- Modify: `src/styles/tokens.css`
- Modify: `src/styles/base.css`
- Modify: `src/components/ui/controls.css`
- Modify: `src/components/shell/AppShell.css`
- Modify: `src/components/shell/Sidebar.css`
- Modify: `src/components/shell/Toolbar.css`
- Modify: `src/components/content/content.css`
- Modify: `src/views/OverviewView.css`
- Modify: `src/components/shell/shell.test.tsx`
- Modify: `src/views/OverviewView.test.tsx`

- [x] **Step 1: Add focused semantic regressions before visual polish**

Extend tests to require:

- one level-one view heading;
- one main landmark;
- a named main navigation landmark;
- current navigation state distinct from unavailable items;
- a polite health live region;
- no icon-only control without an accessible name; and
- no interactive unavailable destination.

Run:

```bash
npm run test:run -- src/components/shell/shell.test.tsx src/views/OverviewView.test.tsx
```

Expected: tests fail for any missing semantics; if they already pass, preserve
them as regression coverage before CSS changes.

- [x] **Step 2: Complete the default and minimum-width layouts**

Implement the documented behavior:

- about `224px` sidebar at default width and `184px` near `760px`;
- `20–24px` default primary-pane insets and `16px` minimum-width insets;
- stable toolbar with scrolling content below it;
- compact `36–44px` rows;
- responsive row stacking without changing DOM order; and
- no shell-level horizontal overflow at `760px` or 200% zoom.

- [x] **Step 3: Complete appearance and preference states**

Check and tune:

- light tokens;
- separately tuned dark tokens;
- increased-contrast separators, selection, and focus;
- reduced-motion behavior;
- hover, pressed, focus-visible, current, unavailable, loading, connected, and
  error states; and
- tabular alignment for schema version and future numeric content.

Keep shadows limited to actual elevation. Do not introduce colored glass,
gradients, or broad translucency during polish.

- [x] **Step 4: Run the focused and broad frontend checks**

Run:

```bash
npm run test:run -- src/components/ui/ui.test.tsx src/components/shell/shell.test.tsx src/components/content/content.test.tsx src/views/OverviewView.test.tsx src/App.test.tsx
npm run format:check
npm run lint
npm run test:run
npm run build
```

Expected: every command exits zero.

- [x] **Step 5: Commit the responsive and accessible polish**

```bash
git add src/components/shell/shell.test.tsx docs/superpowers/plans/2026-09-15-phase-01-5-design-foundation.md
git commit -m "test: verify macOS accessibility foundation"
```

---

## Task 7: Verify scope, run the application, and hand off visual QA

**Files:**

- Modify only if verification exposes a defect in the files already listed.
- Create: `docs/superpowers/reviews/2026-10-16-phase-01-5-verification.md`
  to record executed gates and native/browser/human evidence separately.

- [x] **Step 1: Run the complete functional verification gate**

From the repository root, run:

```bash
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

Expected: every command exits zero and the unsigned local debug application
bundle is produced.

- [x] **Step 2: Audit the Phase 01.5 boundary**

Run:

```bash
rg -n -i "delete|remove_dir|remove_file|trash|prune|uninstall|full disk access|windows|telemetry|fetch\(|axios|reqwest|~/Repositories" --glob '!docs/**' --glob '!package-lock.json' --glob '!Cargo.lock' .
rg -n "invoke_handler" src-tauri/src
rg -n -i "electron|gradient|hero|commitment-grid" src package.json package-lock.json
git diff main...HEAD -- src-tauri crates Cargo.toml Cargo.lock
git status --short
```

Expected:

- no executable destructive, workspace, scan, network, telemetry, Windows,
  permission, or assumed-root implementation;
- explanatory UI copy may include `no automatic deletion`;
- the command registry still contains only `get_app_health`;
- no Electron dependency or rejected visual pattern remains;
- no Rust, SQLite, Tauri capability, or bundle-identifier change; and
- the worktree contains only intentional Phase 01.5 changes.

- [ ] **Step 3: Run agent visual preflight in the native application**

Launch:

```bash
npm run tauri dev
```

Inspect and record results for:

- light appearance at `1100 × 720`;
- dark appearance at `1100 × 720`;
- light and dark appearance at `760 × 520`;
- keyboard order and visible focus;
- loading, connected, and unavailable health states;
- 200% zoom/text pressure;
- increased contrast where available; and
- reduced motion.

Confirm the screen contains no hero composition, card grid, decorative
gradient, giant safety blocks, fake storage totals, dead controls, clipped
content, or color-only status.

- [x] **Step 4: Correct defects with focused regression tests**

For each functional or semantic defect, add a focused failing test before the
fix and rerun the smallest relevant check. For a purely visual defect, change
the smallest owning style module, repeat the affected appearance/size check,
then rerun:

```bash
npm run test:run
npm run build
git diff --check
```

Commit any correction with a scoped message describing the actual defect. Do
not combine unrelated visual experiments.

- [ ] **Step 5: Confirm repository state and push the authorized feature branch**

Run:

```bash
git status --short
git log --oneline --decorate main..HEAD
git remote -v
git branch --show-current
git push -u origin codex/phase-01-5-design-foundation
git rev-parse HEAD
git ls-remote --heads origin codex/phase-01-5-design-foundation
```

Expected: the checkout is clean, the branch is exactly
`codex/phase-01-5-design-foundation`, the pre-existing remote is unchanged, and
the remote branch hash equals local HEAD. Push only after functional gates and
scope review pass. Do not force a rejected push, open a PR, merge, or release.
The push is authorized even while human visual QA remains pending.

- [x] **Step 6: Report the two QA gates separately**

When automated gates pass, report the functional result separately from native
and browser inspection. List uninspected visual conditions explicitly:

```text
FUNCTIONAL QA: PASS
VISUAL QA: AWAITING HUMAN REVIEW
```

Ask the human to inspect the running application. Do not call Phase 01.5
complete and do not begin Phase 02.

- [ ] **Step 7: Process human visual review**

If the human approves, record `VISUAL QA: PASS` in the task outcome. If the
human requests changes, apply only those within this specification, repeat the
relevant focused tests and full functional gate, and return to Step 6. A change
that expands product behavior or contradicts the locked design requires a new
approved scope before implementation.

---

## Completion criteria

Phase 01.5 is complete only when all of the following are true:

- The current presentation is replaced by the macOS split-view foundation.
- Semantic design tokens drive light, dark, contrast, focus, and status color.
- The small component set has explicit responsibilities and focused tests.
- Overview truthfully presents health, an unavailable storage-analysis empty
  state, and safety posture without fake measurements.
- Default and minimum window sizes, keyboard focus, 200% zoom, reduced motion,
  and health states are verified.
- All functional verification commands exit zero.
- The scope audit confirms no Phase 02+ behavior or prohibited dependency.
- `FUNCTIONAL QA: PASS` is recorded.
- The human has inspected the running application and explicitly recorded
  `VISUAL QA: PASS`.
- The authorized feature branch is pushed and its remote HEAD matches local
  HEAD. This does not imply visual acceptance or permission to merge.

Phase 02 must not begin as part of this plan.

## Execution record — 2026-10-16

- Preflight: normal checkout, expected branch and remote, clean tree, design
  commit contained in HEAD. Baseline: 3 frontend tests and production build pass.
- Ruling: execute inline in the existing checkout; no worktree. Track progress
  here rather than creating additional execution infrastructure.
- Interface review: Tasks 1–6 share semantic tokens; Tasks 3–5 share shell and
  Overview composition; Tasks 4–5 share the unchanged health union. No conflict.
- Ruling: the Task 2 control primitives remain tested building blocks; no
  inactive control is inserted into Overview merely to demonstrate them.
- Ruling: Foundation status follows Storage overview and Safety posture so
  technical health/schema remains secondary, per the current task refinement.
- Ruling: consolidate final Rust checks to the workspace commands. Individual
  crate tests would repeat the same regression coverage without a new concern.
- Ruling: explicitly authorized feature-branch push supersedes the documentation
  run's local-only restriction. Human visual acceptance remains independent.
- Task 1: complete. Baseline tests 3/3; production build and format check pass.
  Concrete neutral palettes documented alongside semantic token implementation.
- Task 2: complete. Missing-module RED, then 9/9 focused tests pass, including
  keyboard activation, disabled behavior, form safety, and status text. Lint and
  production build pass. Local SVGs only; dependencies unchanged.
- Task 3: complete. Missing-module RED, then 2/2 shell tests pass. Navigation
  landmark/current state/reading order verified. Lint and build pass. A working
  skip link and focusable content region provide keyboard access without dead
  navigation controls.
- Task 4: complete. Content and Overview missing-module RED runs, then 7/7
  focused tests and build pass. The existing health union is exported as a
  type from App without changing runtime behavior. Foundation uses an inline
  status footer rather than a prominent section, as refined by the task.
- Task 5: complete. Integration RED: 3 expected shell failures; GREEN: 8/8
  App/bridge tests. Full frontend suite 26/26, lint, format, and build pass.
  StrictMode stale resolve/reject tests protect the existing cancellation guard.
  Removed the obsolete App.css; health contract and backend remain unchanged.
- Task 6: implementation complete. Semantic regression baseline and added
  keyboard-order test pass (7/7 shell/Overview). Browser: both appearances at
  both approved sizes; no horizontal overflow. Real Chrome 200% zoom at the
  minimum window yields a 380 × 260 CSS viewport with no horizontal overflow;
  content can scroll. Emulated increased contrast resolves to 3px focus and
  stronger separators; reduced motion resolves to zero-duration transitions.
  Native: dark default-size debug bundle, connected state, visible skip-link
  focus and successful keyboard activation to main. Native light/minimum-size
  and native accessibility preferences remain unverified, not assumed passing.
- Ruling: Task 6 needed verification and regression coverage, not extra visual
  decoration. Keep the inspected layout and record remaining native checks in
  Task 7 rather than claiming full visual coverage.
- Task 7: functional verification and source/scope review pass. Final reviewer
  found one Important WebKit accessibility issue; explicit list semantics fixed
  with a focused failing regression, then 4/4 focused and 28/28 full frontend
  tests pass. No critical or deferred minor findings.
- Final review boundaries: later product functionality and unchanged backend
  are excluded as requested; human visual acceptance, missing native visual
  conditions, actual VoiceOver announcements, CI, and remote publication are
  not inferred from code review. Native pending conditions are documented in
  `docs/superpowers/reviews/2026-10-16-phase-01-5-verification.md`.
- Human visual QA remains pending. Task 7 native visual Step 3 and human
  acceptance Step 7 stay unchecked; Phase 01.5 is not marked complete.
- Final native debug bundle rebuilt after the accessibility fix. Its native
  accessibility tree now exposes Safety posture as a content list. VoiceOver
  speech output is still pending. Production styles/composition are unchanged.
