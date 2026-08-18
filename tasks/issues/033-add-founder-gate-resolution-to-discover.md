# 033: Add Founder Gate Resolution To Discover

Status: complete
Type: usability
Owner: AI

Assumption basis: signed-native Task 032 rehearsal evidence
Requirement basis: Task 032 material-usability-failure clause; docs/founder-facing-workflow.md
Reversibility: easy
Learning objective: prove an `ask-founder` preflight result remains actionable without terminal or Developer Tools assistance
Source under test: repo-local path

## Goal

Keep project-scoped skill setup, readiness preflight, and the smallest founder
gate on the dominant signed-native Discover path.

## Non-Goals

- Supply founder answers or claim founder acceptance.
- Upgrade founder input to customer validation.
- Run implementation, Git, collaboration, or remote effects.

## Acceptance Criteria

- [x] Missing validated skills produce a dominant setup preview/confirmation,
      not premature resolver review.
- [x] Validated skill installation transitions to a dominant readiness-preflight
      action without using Developer Tools.
- [x] Preflight preserves founder gaps separately from readiness warnings.
- [x] `ask-founder` renders a guided founder-input surface with substantive
      context and explicit MVP-scope confirmation requirements.
- [x] Resolution previews one create and two exact-version updates, requires a
      separate confirmation, preserves customer-evidence limits, and reruns
      preflight.
- [x] Automated frontend/native gates and a signed-native rehearsal pass without
      entering or fabricating founder-owned answers.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bun run check` | pass | Authority, typecheck, 162 tests, and production build passed. |
| 2026-07-23 | Rust test/check/fmt suite | pass | 236 tests passed; check and format gates passed with only three existing dead-code warnings. |
| 2026-07-23 | Signed-native empty-repo rehearsal | pass | Setup and preflight became dominant actions; `ask-founder` rendered the guided gate and stopped before founder input. |
| 2026-07-23 | `output/native/task-032-rehearsal-founder-gate.jpeg` | pass | Exact signed app shows typed founder gap, readiness warnings, disabled preview, and no fabricated answer. |

## Files Changed

- `src/lib/product-workflow.ts`, `src/lib/action-hierarchy.ts`,
  `src/lib/goal-shell.ts` - explicit preflight state/action ordering.
- `src/App.tsx` - validated-helper availability and founder-path integration.
- `src/components/FounderGateResolution.tsx` - guided gate, exact preview,
  confirmation, and preflight rerun.
- `src/lib/founder-gate-resolution.ts` - deterministic founder-gate drafts.
- `src-tauri/src/lib.rs` - typed founder-gap preservation.
- Focused tests, styles, workflow docs, tracker, and rehearsal evidence.

## Verification Summary

The material automation-discovered usability failure is repaired and verified.
Founder-owned content and acceptance remain intentionally unproved for Task 032.

## Blockers

- None for this follow-up.

## Follow-Ups

- Resume Task 032 with founder input in the signed gate, then continue the
  cohesive founder-led Plan, Build, review, restart, and stop trial.
