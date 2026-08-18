# 017: Reconcile Post-Run Evidence And Repair Partial Sync

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed plus ai-inferred
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; tasks/issues/016-bind-remote-ha2ha-claims-to-confirmed-execution.md
Reversibility: moderate
Learning objective: prove local completion remains authoritative while incomplete remote evidence becomes bounded repair debt rather than data loss or duplicate execution
Source under test: repo-local path plus deterministic HA2HA transport

## Goal

After the local controller classifies a verified checkpoint, synchronize HA2HA
evidence, handoff, and task/status state idempotently; persist only a sanitized
reconciliation cursor and repair missing remote state without rerunning Codex.

## Non-Goals

- Roll back a locally completed Build Right task.
- Mark local work complete because the remote task says `done`.
- Persist capability tokens or remote bodies.
- Automatically retry forever or continue to another shared task with debt.
- Synchronize comments, provider payloads, or the full backlog.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- tasks/issues/010-persist-checkpointed-goal-state.md
- tasks/issues/016-bind-remote-ha2ha-claims-to-confirmed-execution.md
- `/Users/pax/Documents/robosync/docs/v1/ha2ha-protocol.md`
- `/Users/pax/Documents/robosync/docs/v3/decisions/V3-DR-004-evidence-review-governance-and-audit.md`

## Acceptance Criteria

- [x] Only a repository-verified local checkpoint may produce remote completion evidence.
- [x] Remote evidence records actor, timestamp, task/target, result, command or
      source summary, local task hash, and sanitized artifacts without provider
      payloads or secrets.
- [x] Evidence write, task link/update, handoff, and status transitions have
      deterministic idempotency/replay behavior and explicit partial-write results.
- [x] A successful remote sync records a bounded cursor and permits the next
      separately confirmed shared iteration.
- [x] A post-local-commit remote failure returns
      `collaborationRepairRequired`, preserves local completion, and blocks the
      next shared iteration without blocking local solo mode.
- [x] Restart reconstructs local truth and sanitized repair debt but requires a
      reconnected Collaborator session and explicit repair action.
- [x] Repair rereads current remote versions, detects incompatible divergence,
      applies only missing records, and never starts Codex.
- [x] Goal-state schema/size/security tests prove no token, URL query,
      authorization header, remote body, or shadow task status is persisted.

## Baseline Evidence

Sprint 1 persists only local run/checkpoint/evidence hashes. HA2HA add-evidence
can itself partially succeed when the evidence file is written but linking the
task conflicts, so partial completion needs an explicit repair contract.

## Solution-Fit Rationale

- Requirement served: make shared evidence durable without sacrificing local truth.
- Constraints honored: external failure cannot rewrite verified repository state.
- Guarantees preserved: idempotency, bounded persistence, explicit repair, no rerun.
- Cost accepted: a small reconciliation state machine and repair command.
- Deferred capability: background sync and durable secret storage.

## Verification

- Post-commit success and every partial-write ordering permutation.
- Restart/reconnect/repair tests with changed remote versions.
- Proof that repair starts no runtime process.
- Goal-state size, schema, CAS, corruption, and capability-leak tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | GPT-5.6 router task profile | pass | Security/distributed-state/persistence/idempotency/partial-failure implementation routed to GPT-5.6 Sol/max; independent Sol/high review is required before closure. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml post_run -- --nocapture` | pass | Five focused tests cover deterministic artifact projection, all 16 compatible partial-state permutations, changed remote versions, exact replay, and incompatible divergence. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml goal_collaboration -- --nocapture` | pass | Four focused tests cover durable sync-pending/repair/reconciled cursors, restart recovery, local-solo isolation, CAS, bounds, corruption, and restricted persisted fields. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml concrete_post_commit -- --nocapture` | pass | Loopback Mdsync transport performed one claim followed by evidence, exact-version task update, handoff, and status writes; post-run bodies contained no capability/provider material. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml verified_shared_completion -- --nocapture` | pass | A repository-verified local checkpoint survived a simulated remote partial result, persisted repair debt, returned `collaborationRepairRequired`, and blocked shared continuation without blocking local preview. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml explicit_collaboration_repair -- --nocapture` | pass | Static command-boundary proof rejects Codex/runtime launch paths and requires explicit confirmation; the result is fixed to `codexStarted: false`. |
| 2026-07-23 | Initial independent Sol/high review F017-01 | fail | HIGH: a shape-valid opaque edit capability could equal the actor, escape through sanitized session metadata, and be copied into a durable remote-completion cursor; marker-shaped leak tests did not prove secret-aware equality. |
| 2026-07-23 | Pre-repair opaque-alias regressions | fail as expected | `opaque_capability_alias_is_rejected_before_connect_metadata_is_exposed` and `forged_native_session_capability_alias_is_rejected_on_metadata_read` both observed the alias returned before the repair. |
| 2026-07-23 | Repaired-source opaque capability-alias regressions | pass | Six focused tests cover connect/session metadata, forged native session readback, every remote-intent string/artifact coordinate, caller-controlled paths before transport, remote result actor suppression, and a forged internal intent before goal persistence. The durable-goal proof returns typed `capability_material_rejected`, leaves goal bytes unchanged, exposes no alias to the WebView result, and creates no reconciliation cursor. |
| 2026-07-23 | Focused transport/collaboration/goal security suites | pass | 25 Mdsync transport tests, 14 sanitized collaboration-boundary tests, four goal-cursor tests, and the separate forged-goal regression passed; existing marker/query/header/Bearer rejection remains covered independently. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 197 Rust unit/integration tests passed; no hosted service was contacted. |
| 2026-07-23 | `cargo check --manifest-path src-tauri/Cargo.toml` | pass | Native implementation type-check passed; three pre-existing dead-code warnings remain. |
| 2026-07-23 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | Rust formatting gate passed. |
| 2026-07-23 | `bun run check` | pass | Typecheck, 55 Vitest tests, and the production Vite build passed. |
| 2026-07-23 | Strict Build Right resolver, Task 017 contract, and stop gates after F017-01 repair | pass | Resolver remains `execute-task`/high with only Task 017 selected and no blocking repository gate; task contract says proceed; stop-gates correctly halt advancement while Task 017 retains closure blockers. |
| 2026-07-23 | `bun run tauri build --debug`; fresh native Task 017 resolver preview and deterministic controller fixture | pass | The fresh executable selected only Task 017, reported no blocking gate, refreshed all five repository authority surfaces, stopped at the expected pre-closeout `verificationFailed`/`failureStop`, and started no second task. |
| 2026-07-23 | `output/native/task-017-reconciliation-native-smoke.jpeg` | pass | 1229x768 JPEG, SHA-256 `9809b5d8978413ffc1b3b8af2079a7414550b45f439fade84aece09e10f6940e`. |
| 2026-07-23 | Repaired-source `bun run tauri build --debug`; fresh native Task 017 resolver preview and deterministic controller fixture | pass | The F017-01 repaired executable selected only Task 017, reported no blocking gate, refreshed all five authority surfaces, stopped at the expected pre-closeout `verificationFailed`/`failureStop`, and started no second task. |
| 2026-07-23 | `output/native/task-017-capability-alias-repair-native-smoke.jpeg` | pass | Repaired-source 1229x768 JPEG, SHA-256 `044444af70f3c51f28a789917c323e960a7f0c34f7e1dd744a10c27f9d3d094a`. |
| 2026-07-23 | Independent GPT-5.6 Sol/high closure rereview | approved | F017-01 is closed; no material critical, high, or medium finding remains. Reviewer independently verified secret-alias guards, forged-session/intent proofs, 197 Rust/full gates, native artifact, authority state, and absence of hosted requests. |

## Files Changed

- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/src/collaboration.rs`
- `src-tauri/src/ha2ha_envelope.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/mdsync_transport.rs`
- `src/lib/collaboration.ts`
- `src/lib/collaboration.test.ts`
- `src/types.ts`
- `src/App.test.tsx`
- `output/native/task-017-reconciliation-native-smoke.jpeg`
- `output/native/task-017-capability-alias-repair-native-smoke.jpeg`
- `tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md`

## Verification Summary

- Repository verification commits the local goal checkpoint before any remote
  completion intent is persisted or written.
- The sanitized bounded cursor records only exact bindings, hashes, local
  reference IDs, artifact paths/hashes, remote task version, and missing effect
  enum values; schema and recovery reject bodies, queries, capabilities,
  duplicate effects, stale versions, and shadow task status.
- Native-owned capability material is compared in constant time through
  fixed-length digests against session metadata, caller-controlled transport
  paths/results, and every completion-intent string/artifact coordinate before
  IPC exposure or goal-state write. Typed failures contain no secret material,
  and the long-lived native capability remains zeroized on drop.
- Remote replay uses the fixed order evidence, task link/update, v1-compatible
  `logs/` handoff, then status. Exact existing records are skipped; incompatible
  content fails closed.
- Successful sync leaves a reconciled cursor and permits a separately confirmed
  shared iteration. Partial sync preserves repository verification, persists
  repair debt, and blocks only shared continuation.
- Explicit repair requires a reconnected matching Collaborator session, rereads
  current workspace versions, writes only the projected missing effects, and has
  no Codex execution path.

## Learning Notes

- Proved: local repository authority remains final even when remote completion
  becomes repair debt; exact replay can recover any compatible subset without a
  duplicate remote write.
- Simulated: deterministic loopback transport, every partial-state permutation,
  every sequential write failure boundary, restart/corruption, changed remote
  versions, and incompatible divergence.
- Residual uncertainty: automated tests did not exercise a hosted Mdsync service;
  Task 019 owns that live acceptance.
- Test next: make shared state and repair legible in the UI.

## Skill Trial Notes

- Source comparison: pinned HA2HA evidence/client behavior
- Contract markers checked: local authority, idempotency, partial writes, repair, persistence
- Trial status: implementation, initial fail-closed review, HIGH secret-alias
  repair, automated/native proof, and independent closure rereview approved.

## Blockers

- None.

## Follow-Ups

- Task 018 owns user-facing session and repair controls.
