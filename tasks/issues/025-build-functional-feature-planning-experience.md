# 025: Build Functional Feature Planning Experience

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-approved workflow design
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 022 and 024
Reversibility: moderate
Learning objective: prove one desired feature can become a dependency-valid ready task through the UI without source-code mutation
Source under test: repo-local path

## Goal

Make Plan a complete guided feature-planning workflow that resolves questions,
previews repository changes, writes only confirmed planning artifacts, and
returns the exact resolver decision.

## Non-Goals

- Implement the feature or modify application source code.
- Auto-answer founder/product questions.
- Promote dependency-blocked or stale tasks to ready.
- Publish planned/backlog/task changes into HA2HA or start shared execution.

## Required Reading

- docs/founder-facing-workflow.md
- .agents/skills/build-right-feature-planning/SKILL.md
- docs/execution-rules.md
- tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md
- tasks/issues/024-build-guided-discover-and-project-bootstrap-experience.md

## Acceptance Criteria

- [x] A founder can describe one feature and run the repo-local planning helper
      from the selected repository.
- [x] Questions, conflicts, gates, research triggers, and resolver constraints
      are displayed as typed next actions rather than raw helper output.
- [x] Proposed docs/task/tracker changes are editable and shown as exact paths
      and diffs before confirmation.
- [x] A bounded planning-proposal adapter may suggest the change set, but its
      output is untrusted until converted to the allowlisted artifact plan,
      confirmed, written, and read back by helpers.
- [x] Apply is restricted to allowlisted planning artifacts and cannot modify
      source code, execute a task, commit, push, or publish.
- [x] After apply, planning and strict resolver checks rerun from repository
      truth and show one ready task or the exact stop gate.
- [x] Reaching one ready local task does not automatically publish an HA2HA
      envelope; shared publication remains a separate explicit Build action.
- [x] Cancel, stale, invalid, restart/resume, and successful paths are tested.

## Baseline Evidence

Plan can invoke and display helper output, but it cannot generate, edit, apply,
and verify a bounded planning change set. Sprint 2 can publish a selected ready
task, which makes the separation between local planning and later shared
publication an explicit regression requirement.

## Solution-Fit Rationale

The flow composes existing deterministic helpers with the bounded planning
mutation boundary, keeping planning and implementation as separate effects.

## Verification

- Focused Plan state and integration tests.
- Helper decision fixtures for ready, questions, blocked, stale, and invalid.
- `bun run check`
- Native tests for effects touched by the flow.
- Signed-native manual planning trial recorded in task evidence.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bun run check` | pass | Authority, typecheck, 107 frontend tests, and production build passed |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 227 native tests passed |
| 2026-07-23 | signed native planning trial | pass after repair | Exact create/update diff, confirmed apply, helper readback, and strict `execute-task` resolver |

## Files Changed

- `src/App.tsx`
- `src/components/FeaturePlanning.tsx`
- `src/components/FeaturePlanning.test.tsx`
- `src/lib/feature-planning.ts`
- `src/lib/feature-planning.test.ts`
- `src/types.ts`
- `src/styles.css`
- `src-tauri/src/lib.rs`
- `src-tauri/src/artifact_plan.rs`
- `docs/evidence/task-025-feature-planning.md`
- Authority and tracker files updated at closeout.

## Verification Summary

- `bun run check`: pass; 107 frontend tests.
- `cargo test --manifest-path src-tauri/Cargo.toml`: pass; 227 native tests.
- Apple-development-signed native app: valid signature and successful manual
  planning flow ending in one strict resolver-selected ready task.

## Learning Notes

- Proved: authenticated helper, typed decision, editable proposal, exact
  version-bound update, confirmation, readback, and strict resolver.
- Manual: signed-native feature planning operated by an AI tester.
- Simulated: questions, invalid, cancel, stale, partial, and restart/resume
  variants.

## Skill Trial Notes

- Source comparison: project-scoped feature-planning skill
- Contract markers checked: one feature, questions, preview, resolver, evidence
- Trial status: passed after the first real resolver finding was repaired.

## Blockers

- None.

## Follow-Ups

- Task 026 carries the resulting goal through shell navigation and recovery.
