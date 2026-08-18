# Task 029 Information Architecture Evidence

Date: 2026-07-23
Status: complete
Task: `tasks/issues/029-rework-navigation-information-architecture-and-responsive-layout.md`

## Outcome

The shell now keeps one primary workflow canvas and treats repository
navigation and run evidence as independently collapsible secondary panes.
Project truth is grouped by canonical authority, real sprint membership,
unassigned tasks, and evidence. Search, status filtering, active/selected task
markers, document history, and project/phase/path breadcrumbs are explicit.

Local solo remains the default hierarchy. Collaboration retains its existing
contextual trigger and typed shared-state panel; it does not receive a
permanent dominant workspace column. The same goal/action/evidence hierarchy
continues to render empty, blocked, recovery, review, complete, Viewer,
conflict, sync-pending, and repair-required projections.

## Responsive Contract

- The Tauri minimum changed from 1080x760 to 900x700.
- The page has no width floor and owns viewport overflow; inner regions scroll.
- Below 1100px the primary canvas and navigation remain in the grid while the
  evidence inspector becomes an optional overlay.
- Navigation and evidence can each be collapsed from persistent labeled
  controls; the operating contract is also progressively disclosed.
- No required action depends on hover.

`src/lib/layout-contract.test.ts` pins the native minimum, resizability,
responsive breakpoint, pane-state selectors, and lack of the old width floor.
Navigation model/component tests pin sprint membership, search/filter, active
selection, and history controls. App tests pin the primary-canvas pane state
and breadcrumbs while the existing workflow/shared-state suites preserve all
critical states.

## Deterministic Viewport Evidence

| Viewport | Artifact | Result |
| --- | --- | --- |
| 900x700 | `output/playwright/task-029-900x700.png` | primary canvas, active task, search/filter, pane controls, and actions remain legible; document width/height exactly match viewport |
| 1440x900 | `output/playwright/task-029-1440x900.png` | full three-pane hierarchy scales without horizontal page overflow |

Both Playwright captures reported `scrollWidth === innerWidth`,
`scrollHeight === innerHeight`, and zero browser warnings.

## Automated Verification

| Command | Result |
| --- | --- |
| `bun run check` | pass: authority, typecheck, 151 tests, production build |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 236 tests |
| `cargo check --manifest-path src-tauri/Cargo.toml` | pass |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass |

## Signed-Native Acceptance

Exact artifact:
`src-tauri/target/debug/bundle/macos/Build Right Studio.app`

- Apple development signature verified on disk and satisfied its designated
  requirement.
- Exact binary SHA-256:
  `6d40b64d09aac8a231174feabef5cd883850254960eec0e8d393dfd94e2c9c65`.
- The fresh app was resized to the 900x700 native minimum. It retained the
  primary canvas, project/sprint navigation, active selection, and labeled
  pane controls without horizontal page scrolling.
- Sprint 3 expanded from native repository truth; Task 029 opened, Task 030
  opened, and Previous document returned to Task 029.
- Collapsing the evidence inspector restored the full primary canvas.
- The pre-existing persisted goal referenced another repository. The UI
  truthfully kept `missingRepository`, the fresh-confirmation requirement, and
  the correct review action; no helper, Codex, Git, or collaboration effect
  was started.

## Residual Boundaries

- Task 030 owns formal keyboard, focus, contrast, zoom, high-contrast,
  reduced-motion, and visual-regression enforcement.
- Task 032 remains the founder-owned cohesive usability trial.
- Browser screenshots exercise deterministic projection/layout behavior;
  signed-native evidence establishes the packaged macOS boundary.
