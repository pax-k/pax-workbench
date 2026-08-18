# 026: Make The Shell Goal-Centered And Recovery-Aware

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-approved workflow design plus repo evidence
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 024-025
Reversibility: moderate
Learning objective: prove navigation and recovery can follow the active goal instead of a selected Markdown file
Source under test: repo-local path

## Goal

Make the application shell project- and goal-centered across reopen, resume,
block, review, continue, complete, and optional shared repair states.

## Non-Goals

- Add unattended execution or automatic task starts.
- Store authoritative project/task state outside the repository.
- Persist MDSync capabilities, capability-bearing URLs, remote file contents,
  or a shadow collaboration/task state.
- Build the final visual polish or accessibility suite.

## Required Reading

- docs/founder-facing-workflow.md
- docs/execution-rules.md
- tasks/issues/010-persist-checkpointed-goal-state.md
- tasks/issues/024-build-guided-discover-and-project-bootstrap-experience.md
- tasks/issues/025-build-functional-feature-planning-experience.md
- tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md

## Acceptance Criteria

- [x] Recent-project entries store UI preferences only and explicitly reopen
      and re-inspect repository state before presenting actions.
- [x] Preference storage has an explicit allowlist and excludes task status,
      goal authority, actor capability material, remote contents, and provider
      payloads.
- [x] A resolver-selected task is projected automatically when unambiguous;
      manual file selection remains an advanced inspection action.
- [x] Workflow stage, status ribbon, primary action, and recovery guidance derive
      from project/goal state rather than selected-file presence.
- [x] Local solo, Viewer, Collaborator, disconnected, conflict, sync-pending,
      and repair-required projections compose into the same goal-centered shell;
      local mode remains uncluttered and network-independent.
- [x] Empty, resumable, blocked, review-required, complete, missing-path, and
      stale-repository states have distinct accessible presentations.
- [x] Restart restores the last truthful checkpoint without auto-running Codex
      or consuming confirmation.
- [x] Tests prove projections cannot override repository/helper/controller truth.

## Baseline Evidence

Recovery already stores sanitized local/shared checkpoints and never
auto-starts, but the shell is partly file-centered and can reduce the next
action to “Select a Markdown file.” Project restart also requires manual
reselection, and collaboration recovery remains a separate panel-local state.

## Solution-Fit Rationale

Goal-centered derivation connects the already-proved checkpoint/controller
kernel to the way founders understand ongoing work.

## Verification

- Focused project/goal derivation and restart tests.
- Navigation integration tests across all named states.
- `bun run check`
- Signed-native restart/resume evidence.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bun run check` | pass | Authority, typecheck, 130 frontend tests, and production build passed |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 227 native tests passed |
| 2026-07-23 | signed restart/reinspect trial | pass after repair | Complete and missing-path projections survived restart/reinspection with no automatic process |

## Files Changed

- `src/App.tsx`
- `src/App.test.tsx`
- `src/lib/goal-shell.ts`
- `src/lib/goal-shell.test.ts`
- `src/lib/recent-projects.ts`
- `src/lib/recent-projects.test.ts`
- `src/styles.css`
- `docs/evidence/task-026-goal-centered-shell.md`
- Authority and tracker files updated at closeout.

## Verification Summary

- `bun run check`: pass; 130 frontend tests.
- `cargo test --manifest-path src-tauri/Cargo.toml`: pass; 227 native tests.
- Apple-development-signed restart and recent-project reinspection restored
  repository-affirmed completion with zero automatic helper, controller,
  runtime, or Codex execution.

## Learning Notes

- Proved: strict UI-preference allowlist, native reinspection, shell precedence,
  editor non-authority, state composition, and full regressions.
- Manual: signed-native restart/reopen against completed and mismatched
  repository fixtures.
- Simulated: blocked, stale, resumable, review-required, Viewer, conflict,
  sync-pending, and repair-required variants.

## Skill Trial Notes

- Source comparison: project-scoped execution guidance
- Contract markers checked: repository authority, checkpoint, recovery, consent
- Trial status: passed after disabling controller preparation in terminal and
  invalid recovery states.

## Blockers

- None.

## Follow-Ups

- Task 027 is ready to clarify product actions within the goal-centered shell.
