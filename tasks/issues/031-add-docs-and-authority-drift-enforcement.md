# 031: Add Docs And Authority Drift Enforcement

Status: complete
Type: quality
Owner: AI

Assumption basis: repo-evidence-backed
Requirement basis: docs/evidence/founder-workflow-ui-ux-audit.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; terminal Sprint 2
Reversibility: easy
Learning objective: prove product claims and execution authority remain synchronized as the workbench evolves
Source under test: repo-local path

## Goal

Add deterministic documentation/authority drift checks before Sprint 3
implementation reshapes more code and authority surfaces.

## Non-Goals

- Generate product decisions or task evidence automatically.
- Treat documentation as a substitute for runtime verification.
- Claim production distribution, customer validation, or excluded portability.

## Required Reading

- README.md
- docs/source-index.md
- docs/blueprint-status.md
- docs/release-gates.md
- tasks/sprint-2.md
- tasks/sprint-3.md
- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/evidence/sprint-3-post-ha2ha-reconciliation.md

## Acceptance Criteria

- [x] A deterministic command detects missing indexed authority documents,
      broken local references, invalid sprint/task statuses, bad dependency IDs,
      stale active-task pointers, release-gate/task mismatches, and non-terminal
      predecessor sprints.
- [x] Supported README commands and product claims are checked against package,
      Tauri, current authority, and proved evidence where mechanically possible.
- [x] The command runs as part of `bun run check` and has focused positive and
      negative fixtures that fail for representative drift.
- [x] README, source index, blueprint status, sprint trackers, decision log, and
      release gates truthfully reflect completed Sprint 2 and the revised
      Sprint 3 order, including nonnumeric task IDs such as `028A`.
- [x] Real, manual, simulated, unproved, and post-MVP boundaries remain explicit.
- [x] Checks do not silently rewrite authority or manufacture completion evidence.

## Baseline Evidence

Task 019 closeout required repeated manual repairs to stale blueprint,
release-gate, source-index, and Sprint 3 sequencing text. `bun run check` still
contains only typecheck, frontend tests, and build, so this repeated failure
class is not enforced.

## Solution-Fit Rationale

Small deterministic structural checks protect the product’s core promise that
repository state is inspectable truth, while leaving judgment to humans and AI.

## Verification

- Focused docs-drift positive/negative fixture tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Manual link and product-claim review.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bunx vitest run scripts/check-authority-drift.test.ts` | pass | 6 positive/negative fixture tests cover indexed documents, local links, statuses, dependencies, active pointers, gate mismatches, predecessor sprints, README commands/claims, and `028A`. |
| 2026-07-23 | `bun run authority:check` | pass | Current repository authority, links, trackers, gates, package scripts, Tauri-major claim, and evidence boundaries are synchronized. |
| 2026-07-23 | `bun run check` | pass | Authority check, typecheck, 75 frontend/script tests, and production Vite build passed. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 203 native tests passed; only three pre-existing dead-code warnings remained. |
| 2026-07-23 | `git diff --check` | pass | No whitespace errors. |
| 2026-07-23 | Manual README/source/blueprint/sprint/release/decision review | pass | Completed Sprint 2, active Sprint 3, revised order, command behavior, Tauri 2, and real/manual/simulated/unproved/post-MVP boundaries agree. One stale Task 013 example path was repaired. |
| 2026-07-23 | Independent subagent review | skipped with substitute | Current collaboration policy forbids spawning subagents without an explicit user request. Focused adversarial fixtures, live repository validation, full frontend/native suites, and manual claim review substitute for this quality-task review. |

## Files Changed

- `scripts/check-authority-drift.ts` - read-only structural authority checker and CLI.
- `scripts/check-authority-drift.test.ts` - synchronized and representative drift fixtures.
- `package.json` - authority check command and normal-check integration.
- `README.md` - documented the expanded validation command.
- `docs/ha2ha-mdsync-reconciliation.md` - repaired a stale local Task 013 example path.
- `docs/source-index.md` - recorded the enforcement surface and refreshed authority review.
- `docs/blueprint-status.md` - advanced the active pointer to Task 022.
- `docs/release-gates.md` - closed the drift gate and promoted the unified-contract gate.
- `docs/decision-log.md` - recorded the deterministic enforcement decision.
- `tasks/sprint-3.md` - completed Task 031 and promoted Task 022.
- `tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md` - promoted the dependency-satisfied next task.

## Verification Summary

- The standalone command and all fixture tests pass.
- Normal frontend validation now starts with the authority check and passes.
- The unchanged native boundary passes all 203 Rust tests.
- The checker is read-only: it reports typed issues and exits nonzero without
  rewriting any authority file.

## Learning Notes

- Proved: current repository authority is synchronized and representative drift
  produces deterministic failures.
- Manual: product-claim reconciliation across README and authority surfaces.
- Simulated: intentionally stale fixtures; no fixture result is presented as
  product or native-runtime evidence.
- Test next: Task 022 should extend typed product contracts without weakening
  the newly enforced authority relationships.

## Skill Trial Notes

- Source comparison: Build Right planning and execution guidance
- Contract markers checked: authority, evidence, status, commands, drift
- Trial status: pass

## Blockers

- None.

## Follow-Ups

- Task 022 defines the unified local/shared product contracts after this guard
  is installed.
