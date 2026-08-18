# 032: Run Founder Usability Trial And Close Product Loop

Status: planned
Type: validation
Owner: founder + AI

Assumption basis: founder acceptance requires observed founder use, not implementation inference
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 020-031 and 028A
Reversibility: easy
Learning objective: determine whether the implemented workbench is genuinely usable as a coherent product loop
Source under test: repo-local path

## Goal

Run one cohesive founder-led signed-native trial from an empty repository through
bootstrap, feature planning, one bounded implementation, outcome review,
optional local Git handoff, restart/resume, an optional-shared usability
checkpoint, and the exact final stop decision.

## Non-Goals

- Substitute scripted replay or developer narration for founder use.
- Fix material problems during the observation and still call the original run passed.
- Claim customer validation, production distribution, or provider portability.
- Make the primary founder journey depend on network/hosted state or treat
  multi-context coordination as multi-agent orchestration.

## Required Reading

- docs/founder-facing-workflow.md
- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/evidence/manual-trials.md
- docs/release-gates.md
- tasks/issues/020-extract-frontend-project-session-and-workflow-projections.md
- tasks/issues/031-add-docs-and-authority-drift-enforcement.md
- tasks/issues/028a-add-safe-local-git-handoff-and-commit-boundary.md
- tasks/issues/019-prove-local-and-hosted-collaboration-end-to-end.md

## Acceptance Criteria

- [ ] A founder starts from a deliberately empty local repository and completes
      Discover/bootstrap without terminal assistance or fabricated answers.
- [ ] The founder plans one real feature, understands the preview/effects, and
      reaches the truthful resolver-selected task or stop gate.
- [ ] One ready AI-owned task is reviewed, explicitly confirmed, executed once,
      and judged through the outcome receipt.
- [ ] The founder can optionally preview and confirm a path-scoped local commit
      or explicitly decline it without changing task completion truth.
- [ ] The app is quit/restarted mid-goal or after a checkpoint and resumes from
      repository/goal truth without hidden authority or automatic execution.
- [ ] The final continue/stop decision matches the strict resolver and all
      friction, confusion, assistance, elapsed checkpoints, and failures are logged.
- [ ] A separate optional-shared checkpoint proves the founder can understand
      access, local/remote authority, confirmation, and reconciled or
      repair-required state; local solo remains fully usable without it.
- [ ] Evidence labels every boundary as real, founder-manual, AI-assisted,
      simulated, or unproved and includes signed-app visual evidence.
- [ ] Full frontend, Rust, debug-build, docs-drift, accessibility, local-solo,
      optional-shared, and independent release reviews pass.
- [ ] Any material usability failure creates a narrow follow-up and prevents
      Sprint 3 completion until resolved and retried.

## Baseline Evidence

Sprint 1 proves the technical bounded loop and Sprint 2 proves optional shared
collaboration. Neither proves a founder can use the combined product workflow,
understand when shared state matters, or complete the local journey without
being distracted by integration mechanics.

## Solution-Fit Rationale

Only observed founder use can validate whether the assembled safety kernel,
workflow, information architecture, and evidence model form a usable product.

## Verification

- Founder-observed signed-native scenario with timestamped notes/screenshots.
- Full `bun run check` and repository-defined Rust/debug-build suite.
- Strict resolver and release-gate reconciliation.
- Independent implementation, UX/accessibility, and release evidence reviews.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `docs/evidence/task-032-automated-rehearsal.md` | preparation complete; founder gate open | Signed automation reached real `ask-founder`, found two material transition failures, and Task 033 repaired them. No founder acceptance is claimed. |

## Files Changed

- None yet.

## Verification Summary

- Not run yet.

## Learning Notes

- Real: signed automation reached the guided founder gate; founder trial remains pending.
- Manual: founder interaction and assistive-technology observations.
- Simulated: automation-prefixed bootstrap inputs and explicitly recorded adverse fixtures.
- Unproved: pending trial and independent review.

## Skill Trial Notes

- Source comparison: all repo-local Build Right workflows plus frontend design
- Contract markers checked: cohesive loop, founder observation, evidence classes
- Trial status: n/a

## Blockers

- Tasks 020-031 and 028A must be complete.
- Founder participation is required. This task must never be promoted as an
  ordinary AI-only ready task.

## Follow-Ups

- Create only evidence-backed, narrowly scoped follow-ups from observed friction.
