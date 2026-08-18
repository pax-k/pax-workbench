# 023: Implement Safe New Project Artifact Plan And Apply Boundary

Status: complete
Type: implementation
Owner: AI

Assumption basis: accepted workflow design plus repo-evidence-backed security constraints
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 021-022
Reversibility: moderate
Learning objective: prove a blank repository can receive canonical planning artifacts without granting arbitrary filesystem writes
Source under test: repo-local path

## Goal

Implement a native plan/preview/confirm/apply boundary for new canonical
Markdown artifacts in a selected repository.

## Non-Goals

- Generate product truth without founder input.
- Overwrite existing files, edit source code, commit, push, or publish.
- Accept arbitrary paths or unbounded file content.
- Publish, join, or mutate HA2HA/MDSync state as part of planning artifact
  creation.

## Required Reading

- docs/founder-facing-workflow.md
- docs/execution-rules.md
- tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md
- tasks/issues/021-extract-native-repository-and-workflow-controller-modules.md
- tasks/issues/005-complete-safe-repository-session.md

## Acceptance Criteria

- [x] The planner accepts only bounded, repo-relative, allowlisted planning and
      task Markdown targets and returns exact paths, contents/diffs, and effects.
- [x] Absolute paths, traversal, symlink escape, disallowed extensions,
      oversized input, duplicate targets, and existing-file overwrite fail
      before any write.
- [x] Apply requires the matching current baseline and an unexpired one-use
      confirmation token bound to the exact plan.
- [x] Plan issuance and apply use the existing operation linearization model;
      active helper/runtime/controller/collaboration mutation blocks the effect
      before any repository write.
- [x] Successful apply is atomic where practical, reports every committed path,
      and refreshes repository projections.
- [x] Retry is idempotent; partial failure reports committed and unapplied paths
      without claiming full success.
- [x] Planning writes never create a collaboration session, publish an
      execution envelope, or mirror sprint/task state remotely.
- [x] Native security/contract tests, frontend checks, and full Rust checks pass.

## Baseline Evidence

The application can save only a selected existing Markdown file; it has no safe
new-artifact planning or apply command. Sprint 2 added a proved one-use publish
plan and operation registry pattern, but those remote-envelope mechanics must
not be reused as planning authority or cause remote effects.

## Solution-Fit Rationale

A narrow allowlisted mutation provides the missing bootstrap primitive while
retaining repository containment, stale-state protection, and explicit consent.

## Verification

- Native path, symlink, stale-plan, token, partial-failure, and idempotency tests.
- Frontend bridge contract tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | Focused native artifact boundary tests | pass | 7 tests cover allowlists, bounds, symlinks, stale state, one-use/expiry/root binding, operation locking, success, idempotency, and partial failure |
| 2026-07-23 | Frontend bridge contract | pass | Typed preview/apply arguments contain only local root, exact drafts, token, and confirmation; no shared coordinate |
| 2026-07-23 | Full frontend/native regression | pass | 92 frontend and 226 Rust tests, typecheck/build, Rust check/format, and debug Tauri build pass |
| 2026-07-23 | Live native startup | pass | Produced debug binary launched and stayed healthy until deliberate smoke termination |
| 2026-07-23 | Independent review | skipped | Subagent delegation was unavailable; security fixtures, full regressions, debug packaging, live startup, and local diff review substituted |

## Files Changed

- src-tauri/src/artifact_plan.rs
- src-tauri/src/lib.rs
- src-tauri/src/command_contract.rs
- src/types.ts
- src/lib/bridge.ts
- src/lib/bridge.test.ts

## Verification Summary

- Passed 7 focused native artifact tests and 3 frontend bridge tests.
- Passed `bun run check`: 92 tests, authority, typecheck, and production build.
- Passed 226 Rust tests, check, format, debug Tauri build, and native startup.

## Learning Notes

- Proved: exact local Markdown plans are bounded, create-only, baseline/token
  bound, linearized, refresh-producing, idempotent across a fresh preview, and
  never coupled to collaboration.
- Simulated: the partial-write failure is injected after one real committed
  create and produces an exact repair receipt.
- Test next: guided blank-project bootstrap through the product UI.

## Skill Trial Notes

- Source comparison: project-scoped installed skills
- Contract markers checked: allowlists, preview, confirmation, idempotency
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 024 composes this primitive into Discover.
