# Task 030 Accessibility And Visual Behavior Evidence

Date: 2026-07-23
Status: complete
Task: `tasks/issues/030-enforce-accessibility-and-visual-behavior.md`

## Outcome

The critical local and optional-shared surfaces now have an enforced
accessibility baseline without claiming certification. The collaboration
surface is a labelled modal dialog with initial focus, Escape close, focus
return, and forward/reverse focus containment. Native controls, labelled
fields, tabs, headings, landmarks, status/live regions, and explicit state text
remain the semantic authority; capability material is still absent from DOM,
logs, errors, and screenshots.

Body and control copy computes to at least 13px and metadata to at least 11px.
Status meaning retains text and iconography rather than relying on color.
Visible focus, increased-contrast, forced-color, reduced-motion, 900x700, and
200%-zoom-equivalent behavior are explicit CSS and regression contracts.

## Automated Evidence

- `axe-core` scans the full local-solo shell and open collaboration dialog in
  the normal Vitest suite. Pixel contrast is intentionally excluded from jsdom
  and covered by browser preference emulation plus the accepted palette.
- Collaboration tests cover initial focus, Escape/focus return, forward and
  reverse focus containment, secure-field clearing, Viewer denial, explicit
  shared confirmation, conflict, disconnect, restart, and repair without
  capability leakage.
- `src/lib/accessibility-contract.test.ts` pins type floors, visible focus,
  compact zoom layout, navigation/evidence overlays, increased contrast,
  forced colors, and reduced motion.
- Existing Discover, Plan, review, Git handoff, recovery, execution, and App
  suites continue to exercise native keyboard-operable controls and named
  status/confirmation surfaces.
- The authority checker now permits a planned founder-gated active pointer only
  when no ready/active task exists and the task explicitly requires founder
  participation. Ordinary planned pointers still fail closed.

## Deterministic Browser Evidence

| State | Artifact | Result |
| --- | --- | --- |
| 900x700 normal | `output/playwright/task-030-900x700-normal.png` | exact viewport fit; essential copy minimum 13px and metadata minimum 11px |
| 200% zoom equivalent | `output/playwright/task-030-200pct-equivalent.png` | 450x350 CSS viewport; primary canvas stays full width and navigation becomes its own scrollable overlay |
| Increased contrast | `output/playwright/task-030-increased-contrast.png` | `prefers-contrast: more` applied higher-contrast tokens and a 4px focused outline |
| Forced colors + reduced motion | `output/playwright/task-030-forced-colors-reduced-motion.png` | forced-color selection/focus boundaries active; pulse animation reduced to 0.001ms |

## Signed-Native And VoiceOver Evidence

Exact bundle:
`src-tauri/target/debug/bundle/macos/Build Right Studio.app`

- Apple Development signature verified on disk and satisfied its designated
  requirement. Binary SHA-256:
  `fde5c8e4333014f864a201b672e18dc4e21f008f0f71ade6bf88d0712985ba6f`.
- At the exact 900x700 native window, navigation and evidence collapsed
  independently and the primary recovery canvas remained usable:
  `output/native/task-030-signed-900x700.jpeg`.
- VoiceOver was launched from a previously-off state. Standard keyboard
  traversal announced named project/navigation controls while the native
  accessibility tree exposed navigation, headings, tabs, statuses, fields,
  workflow controls, selected state, and help text. The focused control had a
  visible non-color-only outline:
  `output/native/task-030-voiceover-keyboard-focus.jpeg`.
- VoiceOver and its Quick Start helper were fully quit after the pass.
- The existing durable receipt belongs to another repository. The signed app
  truthfully rendered `missingRepository`, required fresh confirmation, and
  started no helper, Codex, Git, or collaboration effect.

## Verification

| Command | Result |
| --- | --- |
| `bun run check` | pass: authority, typecheck, 157 tests, production build |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 236 tests |
| `cargo check --manifest-path src-tauri/Cargo.toml` | pass; three pre-existing dead-code warnings |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass |
| `git diff --check` | pass |
| `bun run tauri:build:signed` | pass: fresh Apple-development-signed app |

## Evidence Boundaries

- Proved: automated semantics, keyboard modal behavior, type floors, responsive
  zoom layout, preference media behavior, signed-native accessibility tree,
  real VoiceOver process use, and full regressions.
- Manual: visual legibility and VoiceOver keyboard observation.
- Simulated: adverse collaboration states remain deterministic fixtures.
- Not claimed: WCAG certification, customer usability, or the founder-owned
  cohesive product trial.

Task 032 remains the founder-observed usability gate and cannot be substituted
by this engineering acceptance.
