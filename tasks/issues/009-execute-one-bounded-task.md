# 009: Execute One Bounded Task

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/mvp-scope.md; docs/execution-rules.md
Reversibility: moderate
Learning objective: prove the app can supervise one resolver-selected AI-owned task without bypassing repository gates
Source under test: repo-local path

## Goal

Run the deterministic resolver, execute exactly its selected ready task through
the Codex adapter, refresh repository truth, and stop with durable evidence.

## Non-Goals

- Continue automatically into a second task.
- Execute founder-owned or external-wait work.
- Mark work complete from agent output alone.
- Publish, push, deploy, or use production credentials.

## Required Reading

- docs/execution-rules.md
- docs/release-gates.md
- tasks/issues/008-implement-codex-runtime-adapter.md
- `.agents/skills/build-right-execution/SKILL.md`

## Acceptance Criteria

- [x] The full deterministic resolver runs before selection and its decision,
      confidence, next action, gates, and selected task are visible.
- [x] Only one `ready`, AI-owned, contract-complete task may start.
- [x] The user sees task goal, non-goals, source, expected effects, and live-host
      warning before confirming execution.
- [x] One Codex invocation receives the exact selected task and Build Right
      execution instruction.
- [x] Files, Git state, task evidence, and resolver state refresh after exit.
- [x] Completion requires repository evidence and verification, not provider claims.
- [x] Founder, external, stale, conflict, failure, and cancellation gates stop
      without selecting another task.
- [x] An end-to-end fixture trial covers success, verification failure, and wait-external.

## Baseline Evidence

The current UI simulates checkpoint progression and has no controller connecting
the resolver, runtime adapter, repository refresh, and evidence gate.

Reconciliation on 2026-07-21 confirms Task 008 now provides the compiled,
reviewed provider-neutral runtime port, exact closed Codex argv, native run
identity, streaming channel, cancellation/reaping, typed terminals, and
authority isolation required by this controller. The deterministic helper and
resolver surfaces from Task 007 remain current. No controller yet binds them,
so the implementation gap stated above remains accurate.

## Solution-Fit Rationale

- Requirement served: use Build Right to execute one controlled unit of work.
- Constraints honored: one task, explicit confirmation, real stop gates.
- Guarantees preserved: Markdown authority and verification-backed completion.
- Cost accepted: orchestration state machine for one provider and one task.
- Deferred capability: unattended continuation and production effects.

## Verification

- Controller state-machine tests for success and every stop gate.
- `bun run check`
- Native disposable-repository trial with a reversible ready task.
- Inspect resulting task evidence and resolver output.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | Task 008 receipt, native live evidence, current resolver/helper contracts | pass | Dependency is complete and this contract matches the current repository; promoted to the sole ready AI-owned task. |
| 2026-07-22 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 92/92 Rust tests passed, including real installed-helper success, verification-failure, wait-external, cancellation, cleanup, refresh-failure, path-swap, and production timeout-wiring coverage. |
| 2026-07-22 | `bun run check` | pass | Typecheck passed; Vitest passed 35/35; production Vite bundle succeeded. |
| 2026-07-22 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | pass | Rust formatting is clean. |
| 2026-07-22 | `bun run tauri build --debug --bundles app` | pass | Fresh compiled debug app and macOS bundle succeeded after the reviewed timeout remediation. |
| 2026-07-22 | Independent Sol/high implementation and timeout re-review | pass | No critical or medium findings remain; production command paths prove generic 120-second and controller 1,800-second bounded policies. |
| 2026-07-22 | Compiled-app disposable-repository trial at `/tmp/pax-workbench-task009-live2.HZMl99` | pass | Resolver selected only ready AI-owned Task 900; one exact `workspace-write` Codex invocation exited cleanly; UI terminal was `verified`; repository authority true/provider authority false; all five refresh surfaces obtained; no second task selected or process remained. |
| 2026-07-22 | Disposable task evidence and stop gates | pass | `bounded-proof.txt` matched exactly; Task 900 recorded all four criteria and command evidence; post-exit resolver was `no-ready-task` with no gates; stop-gates returned only `selected task status is complete`. |
| 2026-07-22 | `output/native/task-009-controller-preview-reviewed.jpeg` | pass | 1229x768, SHA-256 `3aa1f27f4f9ff4789dd32dedab1e93a37249f58e7e115bbe77035feeafce6d32`; exact decision, task, gates, goal, non-goals, effects, and live-host warning are visible. |
| 2026-07-22 | `output/native/task-009-controller-verified-reviewed.jpeg` | pass | 1229x768, SHA-256 `31318e5dee7edf0cee5b6139c9f6fbbd6ff0469213f42ff4f471737c55e96323`; verified terminal and refresh authority are visible. |

## Files Changed

- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/App.tsx`
- `src/App.test.tsx`
- `output/native/task-009-controller-preview-reviewed.jpeg`
- `output/native/task-009-controller-verified-reviewed.jpeg`
- `tasks/issues/009-execute-one-bounded-task.md`
- `tasks/sprint-1.md`
- `docs/release-gates.md`

## Verification Summary

- Full Rust suite: pass, 92/92.
- Frontend check: pass, 35/35 tests plus typecheck and production build.
- Rust formatting: pass.
- Debug Tauri app bundle: pass.
- Deterministic controller outcomes: pass for verified, verification-failed,
  wait-external, stale, conflict, cancellation, timeout, cleanup, and refresh failures.
- Real compiled-app reversible trial: pass with exactly one provider invocation,
  repository-verified completion, full post-exit refresh, and no second selection.
- Independent review: pass with no open critical or medium findings.

## Learning Notes

- Proved: the compiled app can resolve, preview, confirm, execute, refresh, and
  repository-verify exactly one bounded task while refusing provider authority.
- Real: installed resolver/helper execution, one authenticated Codex JSONL
  process, reversible repository writes, Git/task refresh, compiled WebView UI,
  timeout/cancellation process ownership, and final process cleanup.
- Manual: selecting the disposable repository and pressing the explicit prepare
  and confirm controls; screenshot inspection.
- Simulated: deterministic fixture branches for verification failure,
  wait-external, stale/conflict, cancellation, helper failure, and cleanup failure.
- First live attempt learning: an authority-incomplete fixture correctly failed
  stop gates; the corrected fixture then exposed the too-short generic 120-second
  limit. A controller-only 1,800-second bound was added without changing the
  generic runtime, and production-path regression coverage prevents rewiring.
- Residual risk: non-Unix task reads retain the canonicalize/open fallback and
  live bounded execution remains unsupported there; the Unix path is verified.
- Test next: persist and resume the checkpoint safely in Task 010.

## Skill Trial Notes

- Source comparison: project-scoped installed execution skill
- Contract markers checked: resolver, task contract, baseline, verification, evidence, stop gates
- Trial status: pass; real compiled-app trial plus deterministic adverse branches

## Blockers

- None.

## Follow-Ups

- Task 010 adds durable goal/checkpoint recovery.
