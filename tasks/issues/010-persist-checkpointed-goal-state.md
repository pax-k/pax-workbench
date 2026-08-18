# 010: Persist Checkpointed Goal State

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/raw/product-discussion.md; docs/mvp-scope.md
Reversibility: moderate
Learning objective: prove a stopped run can resume safely without persisting a competing task or planning database
Source under test: repo-local path

## Goal

Persist the goal objective, repository identity, run/event cursor, and last
verified checkpoint so the application can restart and reconstruct authority
from repository files before continuing.

## Non-Goals

- Persist sprint/task truth outside the repository.
- Automatically resume an agent process after restart.
- Sync state across machines.
- Run more than one task per checkpoint.

## Required Reading

- docs/mvp-scope.md
- docs/execution-rules.md
- tasks/issues/009-execute-one-bounded-task.md

## Acceptance Criteria

- [x] A versioned goal record contains only objective, canonical repository
      identity, stop conditions, current run ID/event cursor, and checkpoint.
- [x] Sprint, task, status, evidence, and gate truth are reconstructed from files
      and helpers after every launch.
- [x] Resume detects moved/missing repositories, changed Git state, stale tasks,
      schema incompatibility, and incomplete prior processes.
- [x] Resume never starts Codex without a new explicit confirmation.
- [x] Event persistence is bounded and preserves raw evidence references without
      storing secrets.
- [x] Tests cover clean resume, stale repository, interrupted run, incompatible
      version, and goal completion.

## Baseline Evidence

No durable goal or run state exists. Demo checkpoints and the Task 009 native
controller result are component state and disappear when the application
reloads. Task 009 now provides native run identity, bounded event cursors,
repository reconstruction, explicit confirmation, and verified stop terminals;
this task owns only the versioned durable checkpoint and recovery boundary.

## Solution-Fit Rationale

- Requirement served: checkpointed goal loops that survive application restart.
- Constraints honored: repository files remain planning and execution authority.
- Guarantees preserved: explicit confirmation, stale-state detection, bounded records.
- Cost accepted: versioned local orchestration persistence and recovery logic.
- Deferred capability: cloud sync and cross-device coordination.

## Verification

- Persistence schema and recovery state-machine tests.
- `bun run check`
- Native restart/resume trial with a disposable repository.
- Inspect persisted state to confirm no task/planning shadow copy.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-22 | Task 009 receipt, controller implementation, and compiled-app reversible trial | pass | Dependency is complete; current code has no durable goal record or restart reconstruction, so this contract remains accurate and is promoted as the sole ready AI-owned task. |
| 2026-07-22 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 106/106 Rust tests passed, including clean/completed reconstruction, missing/moved/replaced repositories, HEAD/index/worktree drift, stale/interrupted runs, strict schema bounds, CAS, descriptor-relative storage, temp-source swaps, and production completion authority. |
| 2026-07-22 | `bun run check` | pass | Typecheck passed; Vitest passed 38/38; production Vite build succeeded. |
| 2026-07-22 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | pass | Rust formatting is clean. |
| 2026-07-22 | `bun run tauri build --debug --bundles app` | pass | Fresh compiled debug app and macOS bundle succeeded. |
| 2026-07-22 | Independent Sol/high security and final gate reviews | pass | F010-01 through F010-04 and the final checkpoint-row correlation defect are closed; no critical or medium findings remain. |
| 2026-07-22 | Live nonterminal record at `/Users/pax/Library/Application Support/com.pax.buildrightstudio/goal-state.json` | pass | Schema v3 was durable before provider completion with revision, objective, canonical repository identity, fixed stop conditions, run ID, bounded cursor, `nonterminal: true`, no checkpoint, and no evidence references. |
| 2026-07-22 | Compiled-app close/reopen trial against `/tmp/pax-workbench-task010-live.p7O6Hf` | pass | One live reversible task produced a repository-verified checkpoint; after app restart and explicit repository selection, recovery was `resumable`, cursor 39, automatic execution false, and a fresh two-step confirmation remained required. No Codex process existed after restart. |
| 2026-07-22 | Compiled-app Git-stale restart trial | pass | Adding `recovery-stale-marker.txt` in the disposable worktree changed recovery to `gitChanged` after restart; automatic execution remained false and no Codex process started. |
| 2026-07-22 | Persisted-record inspection | pass | Top-level keys were exactly version, revision, objective, repository, stopConditions, currentRun, lastCheckpoint, and evidenceReferences; no task/sprint/status/gate truth, raw provider payload, secret, or persisted completion Boolean existed. |
| 2026-07-22 | `output/native/task-010-checkpoint-resumable-before-restart.jpeg` | pass | 1229x768, SHA-256 `bdfb412f754aad205f8bfd3d5d5d262e12dcd802f245be4fcf852745a5b0d340`; verified checkpoint and resumable projection are visible. |
| 2026-07-22 | `output/native/task-010-resumable-after-restart.jpeg` | pass | 1229x768, SHA-256 `47114eea2f64819f9b3fd6e8ac0ef6aab8e7d2292729672d9421746cc895cd34`; restart recovery and fresh-confirmation boundary are visible. |
| 2026-07-22 | `output/native/task-010-git-changed-after-restart.jpeg` | pass | 1229x768, SHA-256 `8f82e64bc28376e30f917c787bcee9e90bbc4125df18ebaca3954e460d63e9b0`; Git-stale recovery and zero automatic execution are visible. |
| 2026-07-22 | Explicit receipt discard | pass | UI removed the disposable app-data record and confirmed repository authority files were unchanged. |

## Files Changed

- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/App.tsx`
- `src/App.test.tsx`
- `output/native/task-010-checkpoint-resumable-before-restart.jpeg`
- `output/native/task-010-resumable-after-restart.jpeg`
- `output/native/task-010-git-changed-after-restart.jpeg`
- `tasks/issues/010-persist-checkpointed-goal-state.md`
- `tasks/sprint-1.md`
- `docs/release-gates.md`

## Verification Summary

- Rust persistence/recovery and regression suite: pass, 106/106.
- Frontend recovery/confirmation suite: pass, 38/38 plus typecheck and build.
- Rust formatting and fresh native app bundle: pass.
- Independent security/gate review: pass with no open critical or medium findings.
- Real compiled restart: pass for clean resumable and Git-changed states, with
  zero automatic Codex execution and a fresh confirmation requirement.
- Persisted schema inspection: pass; bounded orchestration receipt only, no
  shadow repository authority or raw provider payload.

## Learning Notes

- Proved: a verified checkpoint survives app restart while repository files and
  helpers remain the only task, sprint, evidence, gate, and completion authority.
- Real: native app-data persistence, live run cursor updates, atomic checkpoint
  advancement, compiled app close/reopen, repository reconstruction, Git-stale
  detection, explicit receipt discard, and zero post-restart Codex processes.
- Manual: selecting the disposable repository before and after restart, pressing
  the explicit prepare/confirm controls, and inspecting screenshots/record JSON.
- Simulated: corrupt, incompatible, oversized, missing, moved, replaced,
  interrupted, stale-task, concurrent-writer, cursor-regression, symlink-swap,
  temp-source-swap, and repository-terminal completion branches.
- Review learning: completion cannot be persisted as a Boolean or inferred from
  `no-ready-task`; it is reconstructed from a terminal tracker whose exact
  complete row points to the exact repository-verified checkpoint task.
- Residual risk: fingerprinting intentionally fails closed above 20,000 files or
  128 MiB; live persistence is supported on Unix and typed unsupported elsewhere.
- Test next: Task 011 dogfoods resolver-driven confirmed iterations.

## Skill Trial Notes

- Source comparison: not applicable
- Contract markers checked: goal schema, checkpoint, stop conditions, reconstruction
- Trial status: pass; real compiled close/reopen and Git-stale recovery trials

## Blockers

- None.

## Follow-Ups

- Task 011 owns resolver-driven confirmed loop control.
