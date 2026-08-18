# 020: Extract Frontend Project Session Workflow And Collaboration Projections

Status: complete
Type: architecture
Owner: AI

Assumption basis: repo-evidence-backed
Requirement basis: docs/evidence/founder-workflow-ui-ux-audit.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Task 022
Reversibility: easy
Learning objective: prove the frontend can evolve by workflow phase without `App.tsx` or `CollaborationPanel.tsx` remaining a product state machine
Source under test: repo-local path

## Goal

Extract behavior-preserving project/session, unified workflow, and optional
collaboration projection modules from the frontend before adding new product
behavior.

## Non-Goals

- Change visual behavior or native command contracts.
- Add bootstrap, planning, new collaboration, or execution effects.
- Reimplement native collaboration policy or create frontend authority.
- Replace repository Markdown with frontend-owned authority.

## Required Reading

- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/founder-facing-workflow.md
- docs/execution-rules.md
- tasks/issues/018-add-shared-collaboration-and-repair-ui.md
- tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md

## Acceptance Criteria

- [x] Project/session lifecycle, repository projections, unified workflow phase,
      and optional shared access/binding/repair projections have explicit types
      and focused modules outside both root components.
- [x] Repository files, helper results, and native controller results remain the
      only authoritative inputs; extracted state is projection or UI preference.
- [x] Existing open, refresh, select, save, helper, runtime, controller, goal,
      and collaboration behavior remains unchanged.
- [x] `CollaborationPanel` is split by responsibility into projection/state,
      effect orchestration, and presentation without exposing capabilities or
      duplicating native policy.
- [x] Shared and local UI state cannot disagree on active operation, goal,
      repair debt, or automatic-execution status.
- [x] Focused tests cover projection transitions and stale or absent selection.
- [x] Existing frontend checks and production build pass without snapshot or
      assertion weakening.

## Baseline Evidence

`src/App.tsx` is 1,290 lines and mixes project lifecycle, workflow projection,
effect orchestration, diagnostics, and rendering.
`src/components/CollaborationPanel.tsx` is 1,084 lines and separately owns
session, envelope, publish, shared execution, repair, notice, and presentation
state. Extracting only `App.tsx` would preserve a second state monolith.

## Solution-Fit Rationale

This creates a replaceable presentation boundary while retaining repository
authority and reducing conflict with the completed Sprint 2 UI work.

## Verification

- Focused frontend projection tests.
- `bun run check`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | Focused Vitest projection/component suites | pass | 59 tests cover project/session, unified workflow, collaboration projection, root wiring, and stale/absent selection |
| 2026-07-23 | Full frontend and native regression | pass | `bun run check` passed 91 tests plus typecheck/build; Rust passed 212 tests, check, and format |
| 2026-07-23 | Live Chromium smoke | pass | Real rendered Vite UI opened the collaboration authority panel with focus and expanded semantics; demo adapter remained explicitly labeled and no app console error occurred |
| 2026-07-23 | Independent review | skipped | Subagent delegation was unavailable for this run; focused compatibility suites, full regressions, live UI smoke, and diff review were used as the equivalent verification |

## Files Changed

- src/App.tsx
- src/App.test.tsx
- src/components/CollaborationPanel.tsx
- src/components/CollaborationPanel.test.tsx
- src/lib/project-session.ts
- src/lib/project-session.test.ts
- src/lib/project-effects.ts
- src/lib/collaboration-panel-model.ts
- src/lib/collaboration-panel-model.test.ts
- src/lib/collaboration-effects.ts
- src/lib/product-workflow.ts
- src/lib/product-workflow.test.ts
- src-tauri/src/product_workflow.rs

## Verification Summary

- Passed focused Vitest suites: 59 tests.
- Passed `bun run check`: authority drift, typecheck, 91 tests, and production build.
- Passed `cargo test`: 212 tests; `cargo check` and `cargo fmt --check`.
- Passed a real Chromium render/interaction smoke against Vite. The browser
  path used the explicit simulated adapter and did not claim native effects.

## Learning Notes

- Proved: root components now consume focused pure projections and effect
  adapters while repository/native results remain the only authoritative inputs.
- Simulated: live browser product data and effects remain explicitly demo-only.
- Test next: preserve these contracts while extracting native controller seams.

## Skill Trial Notes

- Source comparison: project-scoped installed skills
- Contract markers checked: authority, state ownership, seams, tests
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 021 extracts the corresponding native seams.
