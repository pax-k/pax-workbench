# 024: Build Guided Discover And Project Bootstrap Experience

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-approved workflow design
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Task 023
Reversibility: moderate
Learning objective: prove a founder can turn an empty repository into truthful Build Right authority without using a terminal
Source under test: repo-local path

## Goal

Make Discover a guided, resumable path from repository inventory through
founder inputs, artifact preview, confirmed creation, and preflight rerun.

## Non-Goals

- Infer or fabricate founder decisions.
- Implement feature planning or code execution.
- Hide gates, uncertainty, or invalid repository state.
- Connect, publish, or require HA2HA/MDSync before local authority and one
  resolver-selected task exist.

## Required Reading

- docs/founder-facing-workflow.md
- docs/evidence/founder-workflow-ui-ux-audit.md
- tasks/issues/023-implement-safe-new-project-artifact-plan-and-apply-boundary.md
- .agents/skills/build-right-preflight/SKILL.md

## Acceptance Criteria

- [x] An empty or partially initialized repository produces an evidence-backed
      inventory and the exact missing authority artifacts.
- [x] Founder questions are minimal, editable, and clearly distinguish supplied
      facts, assumptions, decisions, and unresolved gates.
- [x] The user can preview every proposed path and content change before one
      explicit confirmed apply.
- [x] Successful apply reruns preflight from repository state and presents the
      exact next workflow action.
- [x] Shared mode remains unavailable/non-primary throughout bootstrap and
      becomes eligible only after local preflight and resolver truth support one
      selected execution envelope.
- [x] Cancel, stale plan, invalid input, partial apply, restart, and resume have
      deliberate states with no terminal requirement.
- [x] Tests prove no fabricated product truth and no write before confirmation.

## Baseline Evidence

Discover currently shows preflight/helper projections but cannot create the
missing artifacts required to move a blank repository forward. The signed app
also exposes collaboration at project open, so the guided state must prevent a
remote-first path from bypassing local product truth.

## Solution-Fit Rationale

This is the first complete founder outcome: transform evidence and explicit
answers into inspectable repository authority through a bounded effect.

## Verification

- Focused Discover state and integration tests.
- Empty, partial, stale, cancel, failure, restart, and success fixtures.
- `bun run check`
- Native tests for effects touched by the flow.
- Signed-native manual bootstrap trial recorded in task evidence.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | Focused Discover model, component, and App integration tests | pass | Empty/partial inventory, missing founder input, preview-before-write, cancel, stale token, partial receipt, restart/resume, successful apply/preflight, and collaboration eligibility are covered. |
| 2026-07-23 | `bun run check` before signed trial | pass | Authority check, typecheck, 101 frontend tests, and production build passed. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` before signed trial | pass | 226 native tests passed, including seven artifact-plan cases. |
| 2026-07-23 | Signed-native trial `/private/tmp/build-right-task024-HRVdFL` | pass after two fail-closed repairs | The app created 12 exact canonical files only after separate preview/confirmation. The trial exposed and repaired premature collaboration eligibility plus Bun/Bunx alias loss and absent first-party UI contracts. |
| 2026-07-23 | Corrected setup command and contract receipt | pass | Signed app ran `bun x skills@1.5.19 add ...`, exited 0, created four trusted `skill-ui` contracts bound to actual lock hashes, and rendered all operating cards. |
| 2026-07-23 | Native post-bootstrap preflight | truthful stop | Real helper returned `ask-founder`/medium with exact unresolved gates; collaboration remained disabled at `Available after ready preflight`. No HA2HA/MDSync/session/workspace material existed in bootstrap artifacts. |
| 2026-07-23 | Apple development signature verification | pass | `codesign --verify --deep --strict --verbose=2` accepted the rebuilt local `.app`; notarization remains out of scope. |

## Files Changed

- `src/lib/discover-bootstrap.ts`
- `src/lib/discover-bootstrap.test.ts`
- `src/components/DiscoverBootstrap.tsx`
- `src/components/DiscoverBootstrap.test.tsx`
- `src/components/CollaborationPanel.tsx`
- `src/App.tsx`
- `src/App.test.tsx`
- `src/styles.css`
- `src/types.ts`
- `src-tauri/src/artifact_plan.rs`
- `src-tauri/src/lib.rs`
- `docs/evidence/task-024-guided-bootstrap.md`
- Sprint/task/blueprint/release/source/decision authority updates.

## Verification Summary

- Focused frontend and native regressions pass.
- Full frontend/native/build/authority results are recorded above and in the
  linked evidence packet.
- The signed app trial used a disposable empty Git repository and real local
  effects; no browser fixture or collaboration effect was accepted as proof.
- The required independent subagent review was skipped because active
  collaboration policy prohibited spawning; full automated, signed-native,
  filesystem readback, and fail-closed repair evidence substituted without
  claiming independence.

## Learning Notes

- Proved: exact inventory/drafts, create-only confirmation, restart/resume,
  trusted setup contracts, real helper execution, and collaboration gating.
- Manual: an AI operator exercised the signed-native founder path; this is not
  founder usability or customer validation.
- Simulated: deterministic stale, cancel, invalid-input, and partial-apply
  fixtures.
- Unproved: a founder completing the flow unaided; Task 032 owns that evidence.

## Skill Trial Notes

- Source comparison: project-scoped preflight and frontend design guidance
- Contract markers checked: claims, gates, preview, confirmation, continuity
- Trial status: pass for the bounded AI-operator trial; founder acceptance remains Task 032.

## Blockers

- None.

## Follow-Ups

- Task 025 turns Plan into a complete next phase.
