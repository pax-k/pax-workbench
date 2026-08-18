# 028: Add Post-Run Diff And Evidence Review Receipt

Status: complete
Type: feature
Owner: AI

Assumption basis: audit evidence plus proved bounded-controller outputs
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 021 and 027
Reversibility: moderate
Learning objective: prove a founder can judge a completed run without reconstructing its meaning from raw events
Source under test: repo-local path

## Goal

Present one outcome-first review receipt for a bounded run, combining repository
changes, verification, acceptance evidence, tracker state, optional shared
claim/reconciliation evidence, risks, and next action.

## Non-Goals

- Add destructive revert/reset/delete controls.
- Implicitly accept, commit, push, publish, or rerun Codex.
- Treat remote claim/evidence success as local completion authority or hide
  unresolved collaboration repair debt.
- Hide raw evidence or claim unobserved verification.

## Required Reading

- docs/founder-facing-workflow.md
- docs/evidence/manual-trials.md
- docs/execution-rules.md
- tasks/issues/021-extract-native-repository-and-workflow-controller-modules.md
- tasks/issues/027-separate-product-workflows-from-developer-diagnostics.md

## Acceptance Criteria

- [x] The receipt leads with outcome/status and shows changed files plus bounded,
      escaped textual diffs or explicit unavailable reasons.
- [x] Commands/checks, results, acceptance-criterion evidence, tracker changes,
      risks/follow-ups, and the fresh resolver decision are linked coherently.
- [x] Shared runs add sanitized source binding, access, claim version/result,
      evidence/handoff/status effects, reconciliation/repair state, and no-Codex
      conflict proof without capability material or raw remote contents.
- [x] Raw normalized events and logs remain expandable secondary evidence.
- [x] Accept/handoff, request-revision, and continue/stop actions state their
      effects; none silently reverts, commits, pushes, publishes, or reruns.
- [x] Secrets, capability URLs, unsafe control text, oversized/binary diffs, and
      untrusted ANSI/HTML are redacted or safely represented.
- [x] Partial, cancelled, failed, blocked, and completed outcomes have truthful
      distinct receipts and focused tests.
- [x] Local completion with remote repair debt is shown as locally complete but
      shared continuation blocked; repair updates the same receipt without
      rerunning Codex.

## Baseline Evidence

The current app exposes local runtime/checkpoint information and a separate
collaboration timeline with claim/evidence/handoff/repair state, but no single
receipt connects either path to diffs, criteria, risks, and tracker truth.
Task 027 now provides the one-action review phase and keeps raw events, helpers,
payloads, and probes under Developer Tools; this task must populate that review
action rather than reintroduce a parallel diagnostic result card.

## Solution-Fit Rationale

The receipt turns technical execution into a founder-verifiable decision while
preserving detailed evidence and explicit effect boundaries.

## Verification

- Focused receipt normalization, redaction, diff-bound, and outcome tests.
- UI integration tests for each terminal outcome.
- `bun run check`
- Native tests for any added evidence/diff command.
- Signed-native completed and failed receipt evidence.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bun run check` | pass | Authority check, typecheck, 142 frontend tests, and production build passed. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 229 native tests passed, including bounded diff/redaction and command-contract coverage. |
| 2026-07-23 | `cargo fmt --manifest-path src-tauri/Cargo.toml --check`; `git diff --check` | pass | Formatting and whitespace gates passed. |
| 2026-07-23 | Apple-development-signed `Build Right Studio.app` current-repository fixture | pass | Exact rebuilt bundle rendered a truthful failed receipt with bounded current-worktree evidence and explicit no-effect review choices. |
| 2026-07-23 | Apple-development-signed disposable live Task 991 | pass | One confirmed live Codex invocation completed the disposable task; the receipt showed repository verification passed, criteria/checks, `goalComplete`, changed paths, raw events, and UI-only handoff intent. |

## Files Changed

- `src-tauri/src/review_receipt.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/command_contract.rs`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/lib/review-receipt.ts`
- `src/lib/review-receipt.test.ts`
- `src/components/ReviewReceipt.tsx`
- `src/components/CollaborationPanel.tsx`
- `src/components/CollaborationPanel.test.tsx`
- `src/App.tsx`
- `src/App.test.tsx`
- `src/styles.css`
- `docs/evidence/task-028-post-run-review-receipt.md`
- `docs/native-module-boundaries.md`
- `docs/blueprint-status.md`
- `docs/release-gates.md`
- `docs/decision-log.md`
- `docs/source-index.md`
- `tasks/sprint-3.md`
- `README.md`

## Verification Summary

- Outcome-first local and optional shared receipt projections are implemented.
- Native Git evidence is read-only, path-contained, redacted, binary-aware, and
  bounded to 200 paths, 64 KiB per file, and 256 KiB aggregate text.
- Completed, failed, blocked, cancelled, and partial projections are focused
  test covered; signed-native failed and completed paths were exercised.
- Review choices record intent only; no stage, commit, push, publish, revert, or
  rerun surface was added.

## Learning Notes

- Proved: repository/controller/shared evidence can form one founder-facing
  receipt without becoming a completion or collaboration authority.
- Manual: the exact signed bundle rendered both failed and completed receipts;
  a live accepted-for-handoff choice changed only UI intent.
- Simulated: hostile content, all terminal tones, partial evidence, and shared
  repair debt remain deterministic focused-test evidence.

## Skill Trial Notes

- Source comparison: Build Right execution and engineering guidance
- Contract markers checked: evidence, redaction, bounded output, explicit effects
- Trial status: passed; see `docs/evidence/task-028-post-run-review-receipt.md`

## Blockers

- None.

## Follow-Ups

- Task 028A adds the optional explicit local Git handoff after this receipt.
