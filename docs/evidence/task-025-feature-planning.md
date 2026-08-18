# Task 025 Feature Planning Evidence

Date: 2026-07-23
Status: complete
Source under test: repo-local path and Apple-development-signed native app

## Run Label

`task-025-signed-feature-planning`

## Agent / Tool Surface

- Codex desktop implementation and verification
- Build Right feature-planning, execution, and engineering-principles guidance
- Native macOS app operated through accessibility-backed computer control
- No model router, subagent delegation, runtime provider, or collaboration provider

## Targets

- Main repository: `/Users/pax/Documents/Repos/pax-workbench`
- Final successful disposable trial: `/private/tmp/build-right-task025-final-xWrJ1h`
- Earlier fail-closed trials: `/private/tmp/build-right-task025-1YqPGc` and
  `/private/tmp/build-right-task025-retry-CGbGog`

## Commands And Checks

- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run tauri:build:signed`
- `codesign --verify --deep --strict --verbose=2 <app>`
- Native founder feature input, authenticated planning helper, typed decision,
  editable proposal, exact diff, confirmed apply, planning readback, and strict
  resolver readback
- Independent helper and filesystem readback plus HA2HA/MDSync coordinate scan

## Artifacts And Result

- The authenticated helper accepted one bounded feature request and returned
  `update-sprint` with no questions, conflicts, gates, or research triggers.
- The app proposed one new task plus one exact-version tracker update; both
  contents were editable before preview.
- Preview labeled the task `CREATE` and tracker `UPDATE`, then required a
  separate confirmation.
- Confirmed apply wrote only those two planning Markdown files.
- Post-apply planning returned `create-ready-tasks` for exactly Task 002.
- Strict resolver returned `execute-task` for that same AI-owned task with no
  blocking gates.
- The receipt and repository scan showed zero shared publications and no
  HA2HA, MDSync, session, workspace, claim, or remote-write artifact.

## Fail-Closed Finding Repaired

The first signed apply created a task that the strict resolver rejected as
`invalid-state`: `Required Reading`, `Baseline Evidence`, and `Follow-Ups` were
missing. The generator now emits those task-contract sections. The clean retry
reached `execute-task`, proving the repair against the real helper rather than
only a fixture.

## Security And Authority Boundary

- Feature helper bytes are pinned by path, length, and SHA-256 and execute from
  stdin through fixed Bun argv.
- Planning updates require the exact existing content version plus an unchanged
  Git baseline; Discover drafts without an expected version remain create-only.
- Allowlisting excludes application source and all non-planning targets.
- Apply does not execute a task, run Codex, commit, push, deploy, publish, or
  invoke collaboration.
- The final updater reuses the existing descriptor-safe, identity-checked,
  competing-writer-tested atomic Markdown writer instead of defining a second
  replacement primitive.

## Evidence Classification

- Proved: helper authentication/parsing, bounded proposal generation, expected-
  version update, create-only regression, confirmation, readback, strict
  resolver, signed app operation, and local-only effects.
- Manual: UI navigation and confirmation were performed by an AI operator.
- Simulated: questions, invalid input, cancel, stale baseline, partial apply,
  and restart/resume fixtures.
- Unproved: unaided founder usability, customer value, notarized distribution,
  and production release.
- Independent subagent review was unavailable under the active collaboration
  constraint; equivalent main-agent security review found and removed the
  duplicate update primitive before the final signed rebuild and rerun.

## Follow-Up

Task 026 is ready to make the shell goal-centered and recovery-aware. Task 032
remains the founder-owned usability gate.
