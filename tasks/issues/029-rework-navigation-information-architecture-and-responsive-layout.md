# 029: Rework Navigation Information Architecture And Responsive Layout

Status: complete
Type: design
Owner: AI

Assumption basis: founder-approved workflow and UI/UX audit
Requirement basis: docs/evidence/founder-workflow-ui-ux-audit.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 026-028A
Reversibility: easy
Learning objective: prove the full workflow remains understandable when project complexity or window constraints increase
Source under test: repo-local path

## Goal

Reshape navigation and layout around project, workflow phase, active task, and
review state with usable search, grouping, hierarchy, responsive panes, and
progressive optional collaboration context.

## Non-Goals

- Change native effect semantics or repository authority.
- Add mobile support or a separate design system.
- Hide necessary evidence behind hover-only interactions.

## Required Reading

- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/founder-facing-workflow.md
- tasks/issues/026-make-shell-goal-centered-and-recovery-aware.md
- tasks/issues/027-separate-product-workflows-from-developer-diagnostics.md
- tasks/issues/028-add-post-run-diff-and-evidence-review-receipt.md
- tasks/issues/028a-add-safe-local-git-handoff-and-commit-boundary.md

## Acceptance Criteria

- [x] Navigation supports project context, phase/task breadcrumbs, recent
      history, sprint grouping, search/filter, and explicit active selection.
- [x] One primary workflow canvas dominates each phase; context and evidence use
      progressive disclosure in resizable/collapsible secondary panes.
- [x] Local solo mode does not reserve dominant space for collaboration;
      shared access/binding/repair status appears contextually and expands only
      when relevant or requested.
- [x] The shell remains useful at 900x700 and scales to larger desktop windows
      without a fixed 1080px minimum-width dependency.
- [x] Empty, loading, blocked, resumable, review, and complete layouts are
      deliberate and preserve the correct primary action.
- [x] Viewer, Collaborator, conflict, disconnected-recovery, sync-pending, and
      repair-required layouts preserve the same hierarchy and never expose
      capability-bearing values.
- [x] Essential labels/actions are not hover-only, clipped, or dependent on
      horizontal page scrolling.
- [x] Responsive/layout behavior has deterministic component or screenshot tests.

## Baseline Evidence

The shell enforces a 1080px minimum width, has one principal breakpoint, and
uses a dense file/editor/inspector hierarchy. Sprint 2 added a large
collaboration overlay and many 7-9px remote-state labels, increasing the need
for contextual progressive disclosure.

## Solution-Fit Rationale

The layout preserves the existing graphite/paper/blueprint identity while
making workflow state, actions, and evidence legible at realistic window sizes.

## Verification

- Component/layout tests at defined desktop viewport matrix.
- Deterministic screenshots for critical workflow states.
- `bun run check`
- Signed-native resize and navigation walkthrough.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `docs/evidence/task-029-information-architecture.md` | pass | Grouped/searchable navigation, 900x700/1440x900 browser captures, full regressions, and fresh signed-native resize/history walkthrough passed |

## Files Changed

- `src/App.tsx`
- `src/App.test.tsx`
- `src/components/ProjectFileNavigation.tsx`
- `src/components/ProjectFileNavigation.test.tsx`
- `src/lib/navigation.ts`
- `src/lib/navigation.test.ts`
- `src/lib/layout-contract.test.ts`
- `src/styles.css`
- `src-tauri/tauri.conf.json`
- `output/playwright/task-029-900x700.png`
- `output/playwright/task-029-1440x900.png`
- `docs/evidence/task-029-information-architecture.md`
- Authority documents and Sprint 3 tracker

## Verification Summary

- `bun run check`: pass; authority, typecheck, 151 tests, production build.
- `cargo test`: pass; 236 native tests.
- `cargo check` and `cargo fmt --check`: pass.
- Exact Playwright viewports: 900x700 and 1440x900, zero horizontal page
  overflow and zero browser warnings.
- Fresh Apple-development-signed bundle verified and exercised at its 900x700
  native minimum; grouped Sprint 3 navigation, Task 029/030 selection,
  Back history, pane collapse, and truthful recovery hierarchy passed.

## Learning Notes

- Proved: navigation model, component behavior, responsive contracts, full
  frontend/native regressions, exact browser viewports, and signed-native
  resize/navigation.
- Manual: visual review of deterministic browser captures and signed-native
  minimum-window hierarchy.
- Simulated: fixture-backed workflow states.

## Skill Trial Notes

- Source comparison: frontend-design guidance
- Contract markers checked: hierarchy, density, navigation, responsive behavior
- Trial status: retained the graphite/paper/blueprint system while moving
  repository truth into a grouped evidence spine and secondary-pane controls.

## Blockers

- None.

## Follow-Ups

- Task 030 is ready and enforces accessibility and visual behavior across the
  completed shell.
