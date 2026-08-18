# 016: Bind Remote HA2HA Claims To Confirmed Execution

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed plus repo-evidence-backed
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; tasks/issues/015-publish-and-join-build-right-ha2ha-execution-envelopes.md
Reversibility: moderate
Learning objective: prove a stale or conflicting remote task prevents Codex while a valid claim preserves the existing local confirmation and resolver guarantees
Source under test: repo-local path plus deterministic HA2HA transport

## Goal

Extend the bounded controller so shared-mode confirmation binds both local
Build Right source truth and a remote HA2HA task version, then claims the
remote task before starting exactly one Codex process.

## Non-Goals

- Change local solo execution behavior.
- Retry remote conflicts indefinitely.
- Complete the remote task or add post-run evidence.
- Let Viewer/public sessions execute.
- Persist capability material or start work after restart.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- tasks/issues/009-execute-one-bounded-task.md
- tasks/issues/011-run-confirmed-goal-loop.md
- tasks/issues/015-publish-and-join-build-right-ha2ha-execution-envelopes.md
- `/Users/pax/Documents/robosync/docs/v1/task-claim-idempotency-and-races.md`

## Acceptance Criteria

- [x] Shared preview shows sanitized workspace/task/actor/access, remote version,
      local task hash, expected remote mutation, and stop conditions.
- [x] One-use confirmation is bound to both the local preview token and remote
      task coordinate/version without binding any secret.
- [x] Execution reruns local resolver/task gates before any remote mutation.
- [x] A Collaborator claim uses the exact `baseVersion`, actor, owner, and
      allowed next state; Viewer/public access stops before mutation.
- [x] `version_conflict`, source drift, manifest mismatch, access denial,
      timeout, cancellation, and remote unavailability all prevent Codex start.
- [x] A second conflict stops with a human-visible repair path and no hidden
      retry or task selection.
- [x] If claim succeeds but pre-spawn finalization fails, the result preserves a
      sanitized claimed/reconciliation state for explicit repair.
- [x] Local solo controller regression tests remain byte/decision compatible
      where the public contract is unchanged.

## Baseline Evidence

The Sprint 1 controller binds one-use confirmation to resolver/task source and
proves no stale local task starts. It has no remote read/claim hook.

## Solution-Fit Rationale

- Requirement served: prevent two independent agents from silently executing the same shared task.
- Constraints honored: one confirmed task and HA2HA optimistic concurrency.
- Guarantees preserved: local resolver precedence, no provider authority, and bounded retries.
- Cost accepted: one additional pre-run remote gate in shared mode.
- Deferred capability: leases, parallel orchestration, and automatic claim recovery.

## Verification

- Controller tests for clean claim and every pre-start stop family.
- Exact assertion that no runtime process starts on remote conflict/denial/failure.
- Cancellation/result linearization tests around the remote claim boundary.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | GPT-5.6 router task profile | pass | Distributed-state/security implementation routed to Sol/max; independent Sol/high review remains required before closure. |
| 2026-07-23 | Shared preview and confirmation binding | pass | Preview exposes only sanitized session metadata, exact local task/Git hashes, remote task coordinate/version, the `ready -> claimed` actor/owner mutation, and seven closed stop conditions. The one-use confirmation is exact local/shared-scope bound and cannot execute through the solo command. |
| 2026-07-23 | Exact claim projection and deterministic MDSync transport | pass | Claim projection changes only `state`, `owner`, and `updated_by`; loopback transport tests observed exactly one PUT at the confirmed base version. The concrete claim port revalidated all seven envelope files, performed one exact claim PUT, required exact actor/version/content readback, and retained no capability material. |
| 2026-07-23 | Adversarial pre-run controller tests | pass | Clean claim invokes Codex exactly once. Version conflict, source drift, manifest/protocol mismatch, access denial, timeout, cancellation, and unavailability each run the remote gate once and invoke Codex zero times. Viewer and public previews issue no confirmation. |
| 2026-07-23 | Conflict and cancellation linearization | pass | The first conflict consumes its confirmation; replay stops before the remote port. A fresh version-bound confirmation may attempt once; a second conflict exposes human inspection and never retries/selects another task. Concurrent native cancellation at the blocked claim boundary wins before any Codex probe/spawn. |
| 2026-07-23 | Claimed-state repair projection and capability scan | pass | Claim-port post-commit cancellation, source, and readback failures retain a sanitized `claimedRepairRequired` state. Shared bindings/results and transport errors exclude bearer/query capability material and provider payloads. |
| 2026-07-23 | Focused test-harness repairs | repaired and rerun | Initial focused runs exposed only fixture expectations: macOS tempfile `/var` versus canonical `/private/var`, the existing manifest validator's typed `workspace_id_mismatch` code, and quoted YAML actor serialization in the captured claim body. The fixtures/assertion were corrected and focused tests passed. No product defect was accepted around these failures. |
| 2026-07-23 | Initial `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 175 Rust tests passed before the independent post-claim lifecycle findings and repair regressions. |
| 2026-07-23 | `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | Native compilation and formatting passed. Only the existing/forward-contract dead-code warnings remain. |
| 2026-07-23 | `bun run check` | pass | Typecheck, 54 frontend tests, and the production Vite build passed. No frontend contract change was required. |
| 2026-07-23 | `bun run tauri build --debug`; fresh native Task 016 resolver preview and deterministic controller fixture | pass | The fresh executable selected only Task 016, reported no blocking gate, refreshed snapshot/Git/task-evidence/resolver/stop-gates, stopped at the expected pre-closeout `verificationFailed`/`failureStop`, and started no second task. |
| 2026-07-23 | `output/native/task-016-claim-boundary-native-smoke.jpeg` | pass | 1229x768 JPEG, SHA-256 `d7ca5e3481b1b12b665ddcec717eb26cb5cf5f98be89ed68275b1c2b14c7f39d`. |
| 2026-07-23 | Independent Sol/high findings F-016-01 and F-016-02 | repaired; rereview pending | F-016-01 found that later post-claim/pre-Codex exits could leave the remote claim projected only as `claimed`; the production shared wrapper now centrally records actual main-process start and converts every still-claimed pre-start exit to a sanitized, cause-typed, actionable `claimedRepairRequired` state. F-016-02 corrected the stale Sprint 1 go-next authority from Task 015 to Task 016 review/repair, with Task 017 promotion explicitly prohibited before approval. |
| 2026-07-23 | Production shared-wrapper repair regressions | pass | Four real wrapper/claim-port seam tests commit a claim, prove zero main Codex calls for post-claim cancellation, injected goal-storage failure, and version-probe cleanup failure, and prove a main Codex cleanup failure does not falsely create pre-spawn repair debt. |
| 2026-07-23 | Repaired-source full verification | pass | 179 Rust tests, native compile/check, Rust format check, 54 frontend tests, TypeScript typecheck, production Vite build, and strict Build Right resolver all passed. The resolver still selects only ready Task 016 with no blockers. |
| 2026-07-23 | Repaired-source `bun run tauri build --debug`; fresh native Task 016 resolver preview and deterministic controller fixture | pass | The repaired executable selected only Task 016, reported no blocking gate, refreshed all five authority surfaces, stopped at the expected pre-closeout `verificationFailed`/`failureStop`, and started no second task. |
| 2026-07-23 | `output/native/task-016-post-claim-repair-native-smoke.jpeg` | pass | Repaired-source 1229x768 JPEG, SHA-256 `5acd72d160b633a274a6b243a7e9f8b8ea7ecb54f70bcef2bc0cb71bbbe9dcad`. |
| 2026-07-23 | Independent GPT-5.6 Sol/high closure rereview | approved | F-016-01 and F-016-02 are closed; no material critical, high, or medium finding remains. Reviewer independently verified the repaired process-start boundary, four production-wrapper regressions, 179 Rust/full gates, native artifact, authority state, and absence of hosted requests. |

## Files Changed

- `src-tauri/src/collaboration.rs` - exact sanitized shared execution binding,
  typed conflict repair, cancellation-aware pre-run port boundary, and contract
  tests.
- `src-tauri/src/ha2ha_envelope.rs` - strict exact-version task-claim
  projection and manifest/source/state adversarial fixtures.
- `src-tauri/src/mdsync_transport.rs` - sanitized session accessors, exact
  committed-write matching, typed conflict coordinates, and one-PUT claim
  transport coverage.
- `src-tauri/src/lib.rs` - shared preview/execute commands, scoped one-use
  confirmation, conflict memory, concrete pre-run claim gate, claimed repair
  state, cancellation linearization, and controller integration tests.
- `docs/release-gates.md` - current Task 016 review/repair authority and explicit
  Task 017 promotion hold.
- `output/native/task-016-claim-boundary-native-smoke.jpeg`
- `output/native/task-016-post-claim-repair-native-smoke.jpeg`
- `tasks/issues/016-bind-remote-ha2ha-claims-to-confirmed-execution.md`

## Verification Summary

- Focused shared preview/claim/conflict/cancellation/no-spawn tests: pass,
  including four production-wrapper post-claim lifecycle regressions.
- Deterministic loopback MDSync exact claim write: pass, one PUT at the
  confirmed `baseVersion`.
- Full Rust regression: pass, 179 tests; compile and format pass.
- Full frontend regression: pass, 54 tests plus typecheck and production build.
- Fresh repaired-source native executable and exact Task 016 resolver/one-task
  smoke: pass.
- Hosted capability-bearing MDSync execution: intentionally not run; the task's
  source under test is deterministic transport and Task 019 owns live
  acceptance.
- Mandatory independent Sol/high closure review: approved.

## Learning Notes

- Proved: the same one-use confirmation binds local Build Right truth and one
  exact remote task version; successful claim is the only path to one Codex
  invocation.
- Proved: conflict replay, read-only access, remote/source invalidation, and
  cancellation fail closed before provider start, while post-claim uncertainty
  remains visible for explicit repair.
- Simulated: remote races and transport failures through deterministic ports
  and loopback HTTP; no hosted capability-bearing request was sent.
- Test next: Task 017 synchronizes local completion and repairs partial remote
  evidence writes.

## Skill Trial Notes

- Source comparison: pinned HA2HA task-claim idempotency/race contract
- Contract markers checked: actor, baseVersion, conflict, retries, confirmation, no-spawn
- Router result: Sol/max implementation; Sol/high independent review required.
- Trial status: implementation, self-audit, initial independent findings,
  targeted repairs, adversarial tests, full regressions, repaired native smoke,
  and independent closure rereview approved.

## Blockers

- None.

## Follow-Ups

- Task 017 owns post-run evidence and repair.
