# 022: Define Guided Workflow Effects And Typed Repair Contracts

Status: complete
Type: architecture
Owner: AI

Assumption basis: repo-evidence-backed plus accepted workflow design
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Task 031
Reversibility: easy
Learning objective: prove every guided action and repair path can be typed before mutation is implemented
Source under test: repo-local path

## Goal

Define one guided product projection and effect model that composes the
existing local goal/controller and Sprint 2 collaboration contracts across UI
and native boundaries.

## Non-Goals

- Create or modify repository artifacts.
- Run Codex, Git mutations, or external collaboration effects.
- Add generic catch-all recovery advice.
- Replace, rename, or duplicate the proved HA2HA/MDSync state, version,
  confirmation, failure, redaction, or repair contracts.

## Required Reading

- docs/founder-facing-workflow.md
- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/execution-rules.md
- docs/ha2ha-mdsync-reconciliation.md
- docs/evidence/sprint-3-post-ha2ha-reconciliation.md
- tasks/issues/013-define-collaboration-contracts-and-native-seams.md
- tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md
- tasks/issues/031-add-docs-and-authority-drift-enforcement.md

## Acceptance Criteria

- [x] Workflow states and legal transitions cover open/create, setup, founder
      input, preview, confirmation, running, review, continue, resume, repair,
      blocked, and complete in local solo, Viewer inspection, and Collaborator
      execution modes.
- [x] The product projection is derived from existing `GoalLoopState`,
      `GoalRecovery`, bounded-controller, and collaboration
      access/reconciliation contracts; no parallel authoritative state machine
      or string-derived version exists.
- [x] Effects are classified as inspect, planning mutation, build mutation, Git
      mutation, external/shared, or developer diagnostic.
- [x] Mutation plans carry exact targets, expected baselines, effect summaries,
      expiring one-use confirmation tokens, and truthful result receipts.
- [x] Failure classes distinguish repository, contract, helper, runtime, Git,
      network-policy, collaboration, cancellation, and stale-state failures.
- [x] Local planning/build/Git effects and optional shared effects remain
      separately confirmed; planning never publishes or mirrors tasks remotely.
- [x] Repair guidance is selected from typed evidence; Local Network guidance is
      impossible without matching evidence or an explicitly labeled hypothesis.
- [x] Contracts exclude secrets and capability-bearing values and have focused
      frontend/native compatibility tests.

## Baseline Evidence

Local goal/controller and shared collaboration contracts are individually
strong, but the shell has no unified product projection. `App.tsx` derives its
ribbon largely from the selected Markdown task; `CollaborationPanel.tsx` owns a
second component-local workflow; blanket Local Network guidance can still
appear after unrelated live-runtime failure.

## Solution-Fit Rationale

Modeling state and effects first keeps later guided UX truthful, testable, and
portable across local and optional shared execution.

## Verification

- Focused TypeScript state/contract tests.
- Focused Rust serialization, redaction, and transition tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bunx vitest run src/lib/product-workflow.test.ts` | pass | 8 focused tests cover local/recovery/shared derivation, Viewer inspection, founder input specificity, closed transitions, exact plans/receipts, separate shared effects, typed repairs, and secret rejection. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml product_workflow -- --nocapture` | pass | 8 focused native tests cover camelCase compatibility, existing local/shared enum composition, Viewer denial, plan/receipt validation, evidence-selected Local Network guidance, and secret rejection. |
| 2026-07-23 | `bun run check` | pass | Authority drift, typecheck, 83 frontend/script tests, and production build passed. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 211 native tests passed after contract integration; only three pre-existing dead-code warnings remained. |
| 2026-07-23 | `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | Native compilation and formatting passed with only three pre-existing dead-code warnings. |
| 2026-07-23 | `rg -n 'invoke\(|tauri::command|fs::|Command::|Mdsync|CapabilityMaterial|authorization|bearer|access_token|refresh_token|provider payload|https?://' src/lib/product-workflow.ts src-tauri/src/product_workflow.rs` | pass | Matches are limited to fixed rejection markers and adversarial fixtures; the contract modules contain no command, filesystem, transport, persistence, or capability effect. |
| 2026-07-23 | Independent subagent review | skipped with substitute | Current collaboration policy forbids spawning without an explicit user request. Mirrored frontend/native fixtures, adversarial boundary tests, full suites, formatting, and manual semantic review substitute for the required architecture review. |

## Files Changed

- `src/lib/product-workflow.ts` - stateless product projection, transitions,
  effect/plan/receipt contracts, failure taxonomy, repair selection, and
  secret-free validation.
- `src/lib/product-workflow.test.ts` - focused projection, plan, receipt,
  repair, Viewer, and security tests.
- `src-tauri/src/product_workflow.rs` - mirrored native serialization,
  validation, projection, and repair contracts.
- `src-tauri/src/lib.rs` - registers the new contract module without exposing a
  command or effect.
- `docs/decision-log.md` - records projection ownership and confirmation
  separation.
- `docs/blueprint-status.md`, `docs/release-gates.md`, `tasks/sprint-3.md` -
  advance authority to Task 020.
- `tasks/issues/020-extract-frontend-project-session-and-workflow-projections.md`
  - promoted after dependency completion.

## Verification Summary

- Product workflow state is a stateless projection over existing repository,
  goal/recovery, controller, and collaboration contracts.
- Viewer projections contain inspect effects only.
- Plans have one mutation class, exact targets and distinct baselines, bounded
  one-use expiry, and receipts that account for every target.
- Planning plans reject shared publish/repair operations; remote effects require
  their own plan/confirmation.
- Local Network settings guidance requires `localNetworkDenied`; weaker signals
  produce an explicitly labeled hypothesis.
- No command, persistence, filesystem, Git, runtime, or collaboration effect was
  added by this contract task.

## Learning Notes

- Proved: TypeScript and Rust serialize compatible product-contract shapes and
  derive local/shared UI state without introducing new authority.
- Simulated: typed plan, receipt, failure, and repair fixtures; no mutation or
  remote effect ran.
- Test next: Task 020 should move root-component derivation onto this pure
  projection before Task 023 implements mutation.

## Skill Trial Notes

- Source comparison: project-scoped installed skills
- Contract markers checked: state machine, confirmation, failures, redaction
- Trial status: pass

## Blockers

- None.

## Follow-Ups

- Task 020 extracts frontend ownership around this contract.
