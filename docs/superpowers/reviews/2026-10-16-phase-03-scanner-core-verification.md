# Phase 03 scanner core verification

Date: 2026-10-16. Normal checkout: `feat/phase-03-scanner-core`.
Implementation starts from reviewed design commit
`7ad8431880ff8be4a635523b4ca8d65bf5280f57`, with merged product base
`04018df8d152268cebd2f62aa554ddfa49c30d7a`.

## Current state

Implementation in progress. Checkpoint 1 code and actual native receipts passed
independent task review. This is not a whole-phase functional PASS.
HUMAN VISUAL QA: AWAITING HUMAN REVIEW for the new Phase 03 UI.
Phase 02 visual approval remains granted and is not reopened.

The implementation assignment accepts the design decisions and explicitly
authorizes the isolated Cargo test target, pinned dev dependency edges and
controlled 127.0.0.1:0 origin fixture. No broader production capability, real
workspace scan, release or Phase 04 feature is authorized.

## Checkpoints

| Checkpoint                           | Tasks | Status                               |
| ------------------------------------ | ----- | ------------------------------------ |
| Isolated native verification         | 1     | Implemented; reviewed receipts below |
| Contracts, policy and fixture engine | 2–5   | Task 2 in progress                   |
| Gated application/IPC/minimal UI     | 6–10  | Not executed                         |
| Functional/native/human acceptance   | 11    | Not executed                         |

## Inherited native obligations

- [P02-NATIVE-ACL](2026-10-16-phase-02-workspaces-verification.md#p02-native-acl):
  RESOLVED for the seven existing commands by Checkpoint 1 native evidence.
  New commands additionally
  need their own native denial matrix before MetadataScanRunner is wired.
- [P02-NATIVE-LAYOUT](2026-10-16-phase-02-workspaces-verification.md#p02-native-layout):
  OPEN with new exact populated-view matrix evidence below. Empty Workspaces
  and current Phase 03 controls must still be checked before UI acceptance.
- [P02-NATIVE-ZOOM](2026-10-16-phase-02-workspaces-verification.md#p02-native-zoom):
  RESOLVED for the inherited UI by actual pageZoom 2.0 and the specific human
  interaction confirmation below. New Phase 03 controls still require their own QA.

Historical browser, MockRuntime and normal-bundle evidence remains identified
in the Phase 02 record. It is not relabelled as new Phase 03 evidence.

## Review clarifications applied

- Differentiate traversal limits, inaccessible/changed folders, allocation-only
  uncertainty, cancellation and declared policy exclusions in result copy.
- Keep configuration 0..N; the 32-root cap applies only to a scan request. More
  than 32 enabled prevents UI start without selecting a silent subset.
- Account identity/ownership/metrics atomically on overflow and propagate partial
  covered-by coverage between overlapping roots.
- Resolve retained request replay before current busy/gate/epoch rejection.
- Deep remains bounded; elapsed budget does not guarantee syscall duration.
  Fixture timing cannot justify promises about personal repositories.

Only the original spec/plan were clarified; no parallel plan or roadmap rewrite.
Future evidence entries must identify exact commands, tested commits/artifact,
actual results and inconclusive/omitted checks. No personal paths, databases,
captures or inventories are recorded here.

## Checkpoint 1 — native acceptance target

Tested source: `de0e55b9a32a5a771d40bdad9e878a02884a2877`.
Independent task review: spec compliant, quality approved, no Critical/Important
findings. Existing npm user-config warning and hdiutil deprecation messages are
environmental noise; no global configuration was altered to suppress them.

The opt-in Cargo test executable uses production registration/context/commands,
temporary SQLite and twelve controlled long-path roots. No personal database
or configured workspace was opened. Same capabilities as production; their
SHA256 remains `46a60c52aa6eda33cb2c05c65c958c15451377094745b4d7b2efd6899805cafa`.
Only the two approved dev dependency edges were added; no locked version changed.
The loopback origin server binds 127.0.0.1:0, serves test HTML and stops/joins.

### Actual native authorization

```sh
CARGO_TARGET_DIR=target/native-acceptance cargo test -p dev-storage-manager --test native_acceptance --features native-acceptance,tauri/custom-protocol --locked -- --case inherited-acl
```

Exit 0. Actual Wry callers, not MockRuntime requests. Child in main, other window
and external origin in actual main each rejected all seven commands before the
application handler (zero case-scoped entries); fixture listing and SQLite bytes
stayed identical after every rejected call. Main in the wrong containing window
entered each handler and failed its actual-parent guard before service/picker.

| Command                  | Child      | Other window | External main | Wrong parent | Authorized main control   |
| ------------------------ | ---------- | ------------ | ------------- | ------------ | ------------------------- |
| get_app_health           | ACL denied | ACL denied   | ACL denied    | Guard denied | Schema 2                  |
| list_workspaces          | ACL denied | ACL denied   | ACL denied    | Guard denied | 12 fixture rows           |
| select_and_add_workspace | ACL denied | ACL denied   | ACL denied    | Guard denied | Picker not exercised here |
| remove_workspace         | ACL denied | ACL denied   | ACL denied    | Guard denied | Fixture row removed       |
| set_workspace_enabled    | ACL denied | ACL denied   | ACL denied    | Guard denied | Fixture off/on persisted  |
| check_workspace          | ACL denied | ACL denied   | ACL denied    | Guard denied | Fixture root observed     |
| skip_workspace_setup     | ACL denied | ACL denied   | ACL denied    | Guard denied | Setup persisted           |

Missing-bridge and unsettled-promise controls produced their distinct failure
types, never ACL PASS. Sandboxed macOS-services failure and pre-load eval callback
loss were diagnosed separately; the final run used native page-load completion
and authorized execution outside that sandbox. No production grants were changed.
There are no scanner commands/worker yet; the new four-command gate is not passed.

### Actual native geometry and zoom

The same Cargo command with `--case inherited-layout --hold` completed all
automatic assertions, then deliberately stayed open for interaction. Its optional
hold was interrupted after the human finished (exit 130, not a successful hold).
The exact already-built executable then ran:

```sh
target/native-acceptance/debug/deps/native_acceptance-229da906c690916f --case inherited-layout
```

Exit 0, complete final PASS JSON on unchanged source. Per-window native Light and
Dark each passed Overview and populated Workspaces at exact CSS 760 × 520,
1100 × 720 and 1600 × 900, with one measured titlebar correction per case.
Interior/view/section/separator widths were 544/828/1328 px respectively, deltas
0; maximum Overview empty-title/paragraph centering delta 0.0078125 px. No
main/document horizontal overflow. These are WK CSS measurements, not raster sizes.

Native WK pageZoom getter returned 2.0 and restored 1.0 in every matrix row.
At unchanged window size the zoomed CSS viewports were 380 × 260, 550 × 360 and
800 × 450, with zero horizontal overflow. No global appearance/zoom preference,
CSS scaling or extra JS permission was used. Automatic navigation was DOM
activation for geometry, not proof of native keyboard navigation.

For the held 800 × 450 CSS / native pageZoom 2.0 fixture, controller native AXPress
Check changed a long-path row to Available. A sent Tab exposed only WebArea to AX,
so the agent did not infer button focus. Asked specifically to verify Tab/Shift+Tab,
Enabled/Check/Remove, full paths, scrolling and dialog Cancel/Escape/focus return,
the human replied **“esta perfecto”** while that fixture was alive. This is
human-reported native zoom interaction evidence, not automated keyboard proof,
screen-reader testing or approval of the new Phase 03 UI. No screenshot was
captured from a non-frontmost window. Empty Workspaces/current new UI states
remain part of Task 11; P02-NATIVE-LAYOUT is conservatively kept open until then.

### Functional and packaging checks at this checkpoint

- `npm run build`: exit 0, includes `tsc -b`; Vite transformed 48 modules.
- `npm run test:run`: exit 0, 11 files / 113 tests.
- `cargo test --workspace --locked`: exit 0, 48 Rust tests; opt-in target omitted,
  not counted as native evidence. `npm run lint`, `cargo fmt --all -- --check`
  and `git diff --check` passed.
- `cargo metadata --no-deps --format-version 1 --locked` confirmed a test target,
  required feature and no new bin. `cargo tree -e normal,build -p dev-storage-manager --locked`
  confirmed dev-only edges; already-transitive WebKit is not harness inclusion.
- `cargo build -p dev-storage-manager --bin dev-storage-manager --all-features --locked`
  and `npm run tauri build -- --debug` passed. Normal app/DMG inspected, not launched.
  Read-only DMG mount/list/detach found only normal app/resources/installer metadata.
- Source inclusion review and app/DMG/binary inspection found no QA executable,
  HTML/report resource, QA marker or helper symbol in either all-feature main or
  bundled main. Positive control found all four QA strings in the native test
  executable, so absence was not the only exclusion check. No debug_assertions
  backdoor or additional application command was added.

SHA256 native test executable:
`047ae730623f61a575b09a7bda4121ec28cd38929961526fbe8ea844b9c1a3fe`.
SHA256 normal bundled executable:
`12a75ce66faa406e099b038dff9ac054aeeea848302b73f527f0eb6e75a8f99c`.
Normal bundle fixture-only launch remains Task 11, distinct from test-target QA.
