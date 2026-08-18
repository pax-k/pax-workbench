# 027: Separate Product Workflows From Developer Diagnostics

Status: complete
Type: feature
Owner: AI

Assumption basis: audit evidence plus founder-approved workflow design
Requirement basis: docs/evidence/founder-workflow-ui-ux-audit.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 022 and 026
Reversibility: easy
Learning objective: prove founders see outcome actions while diagnostic power remains available and correctly classified
Source under test: repo-local path

## Goal

Rebuild action hierarchy around Start, Plan, Review, Execute, and Continue while
moving probes and low-level controls behind an explicit Developer Tools surface.

## Non-Goals

- Remove evidence, raw logs, or diagnostic capabilities.
- Change controller safety or authorize new effects.
- Add generic advice unsupported by typed failures.
- Hide collaboration access, source binding, conflict, or repair actions needed
  to understand and safely continue shared work.

## Required Reading

- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/founder-facing-workflow.md
- tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md
- tasks/issues/026-make-shell-goal-centered-and-recovery-aware.md

## Acceptance Criteria

- [x] Each workflow phase presents at most one visually dominant product action
      with consequence, scope, and confirmation status visible.
- [x] Runtime probes, simulated checkpoints, raw payloads, and troubleshooting
      controls live under a labeled Developer Tools/diagnostics surface.
- [x] Collaboration connect/disconnect, Viewer inspection, shared confirmation,
      and explicit repair remain product actions; raw workspace coordinates,
      adapter/provider events, fixture controls, and protocol diagnostics move
      to progressive or Developer Tools detail.
- [x] Every control is classified and rendered as inspection, mutation, or
      diagnostic; simulation is never styled as the primary product path.
- [x] Typed failure evidence selects the repair action and explanation.
- [x] Local Network guidance appears only for a matching typed cause or is
      explicitly labeled as an unproved hypothesis.
- [x] `Principles` becomes contextual guidance/reference rather than a required
      primary workflow destination.
- [x] Action hierarchy, destructive-affordance absence, and failure routing have
      focused tests.

## Baseline Evidence

Generic runtime probing competes with bounded execution, “Simulate next
checkpoint” is a primary inspector control, blanket Local Network guidance can
follow unrelated failures, and the 1,084-line collaboration panel mixes product
actions with low-level session/version evidence.

## Solution-Fit Rationale

The change improves comprehension without discarding the evidence-oriented
diagnostics needed for development and support.

## Verification

- Focused action-model and typed-repair tests.
- UI integration tests for each workflow phase and failure family.
- `bun run check`
- Signed-native visual/manual evidence for primary and diagnostic surfaces.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bun run check` | pass | Authority, typecheck, 134 frontend tests, and production build passed |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 227 native tests passed |
| 2026-07-23 | signed product/diagnostic trial | pass after repair | One dominant action, progressive diagnostics, terminal recovery, and zero automatic execution verified |

## Files Changed

- `src/App.tsx`
- `src/App.test.tsx`
- `src/components/CollaborationPanel.tsx`
- `src/lib/action-hierarchy.ts`
- `src/lib/action-hierarchy.test.ts`
- `src/lib/product-workflow.ts`
- `src/styles.css`
- `docs/evidence/task-027-product-diagnostic-hierarchy.md`
- Authority and tracker files updated at closeout.

## Verification Summary

- `bun run check`: pass; 134 frontend tests.
- Native suite: pass; 227 tests.
- Signed-native manual trial: pass after state-precedence repairs.
- Apple Development signature verified; notarization remains out of scope.

## Learning Notes

- Proved: product-action mapping, state precedence, diagnostic containment,
  typed repair routing, and full regressions.
- Manual: signed product-versus-diagnostic hierarchy across default, expanded,
  collapsed, and completed-recovery states.
- Simulated: typed failure, Viewer, repair, controller, and runtime fixtures.

## Skill Trial Notes

- Source comparison: frontend-design and Build Right engineering guidance
- Contract markers checked: hierarchy, effect labels, failure evidence
- Trial status: passed; the industrial evidence-spine design was retained while
  hierarchy was concentrated into one product action and one diagnostics drawer.

## Blockers

- None.

## Follow-Ups

- Task 028 is ready to provide the product-facing execution outcome.
