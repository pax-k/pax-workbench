# 015: Publish And Join Build Right HA2HA Execution Envelopes

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed plus prototype-assumption
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; tasks/issues/014-implement-secure-native-mdsync-session-transport.md
Reversibility: moderate
Learning objective: prove one Build Right executable task can be represented portably without mirroring the backlog or weakening local authority
Source under test: repo-local workbench plus HA2HA v1/MDSync workspace contract

## Goal

Create and join a minimal HA2HA workspace containing one source-bound Build
Right execution envelope, validate it, and reconcile local/remote truth without
starting Codex.

## Non-Goals

- Mirror every sprint or backlog task.
- Create a public HA2HA Build Right profile.
- Claim or execute the remote task.
- Store capability URLs in repository files or goal state.
- Treat remote task status as local planning authority.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- tasks/issues/014-implement-secure-native-mdsync-session-transport.md
- `/Users/pax/Documents/robosync/docs/v1/ha2ha-protocol.md`
- `/Users/pax/Documents/robosync/docs/v1/workspace-conventions.md`
- `/Users/pax/Documents/robosync/packages/ha2ha-protocol/examples/valid/minimal-workspace/`

## Acceptance Criteria

- [x] Publish is an explicit confirmed action and sends a complete HA2HA v1
      workspace only after deterministic local validation.
- [x] The workspace contains manifest, status, participant, one task envelope,
      and initial evidence/decision references required by the chosen contract.
- [x] The envelope records local task path/hash, repository id, nullable Git
      HEAD, dirty state, requirement basis, and no capability/provider payload.
- [x] Only a resolver-selected `ready`/`active` task can be projected.
- [x] Join validates manifest/workspace id, actor, access, remote task shape, and
      local source binding before reporting reconciled.
- [x] Local source mismatch, unsupported remote state, missing task, and
      duplicate/ambiguous envelope fail with typed repair guidance.
- [x] Viewer joins are useful for inspection but remain non-executable.
- [x] Generated envelope fixtures pass the pinned HA2HA validator/conformance
      path or a byte-equivalent contract fixture.

## Baseline Evidence

Build Right task files use a richer local status/contract and are not HA2HA v1
task files. The earlier review identified direct task-format conversion as a
dual-authority and state-loss risk.

## Solution-Fit Rationale

- Requirement served: share one executable work object between independent agents.
- Constraints honored: no backlog mirror and no task-format migration.
- Guarantees preserved: local authority, portable inspection, and source drift detection.
- Cost accepted: one explicit execution-envelope projection.
- Deferred capability: standardized profile and whole-project sync.

## Verification

- Envelope generation/validation fixtures for clean, dirty, and no-HEAD repos.
- Publish/join/read-only/mismatch tests against deterministic transport.
- Capability/provider-payload scans.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Pinned HA2HA validator over generated workspace fixture.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | GPT-5.6 router v0.3 task profile | pass | Protocol/security/distributed-state implementation routed to `gpt56_router_sol_engineer`, GPT-5.6 Sol/medium, with independent `gpt56_router_sol_reviewer`, Sol/high, required before closure. |
| 2026-07-23 | Pinned HA2HA v1 source review at clean robosync commit `ebd5c8d483a26096f95fdcc8e4f5242270481e9b` | pass | Reviewed protocol/workspace conventions plus the complete valid minimal-workspace fixture before implementation. |
| 2026-07-23 | Deployed MDSync scaffold reconciliation | repaired before acceptance | Root review proved that real MDSync creation prepopulates the reserved manifest, `HA2HA.md`, `STATUS.md`, and actor participant. The initial empty-workspace design was discarded. The accepted design byte-validates and version-binds the pinned generated scaffold, never overwrites the manifest, and creates decision, evidence, then the envelope task last. |
| 2026-07-23 | Initial independent Sol/high security/protocol review | fail closed | F-015-01 found capability-bearing URL variants could reach published metadata; F-015-02 found an ambiguous committed-PUT/lost-response outcome; F-015-03 found stale release-gate wording. Task 015 remained `ready` and Task 016 remained `planned`. |
| 2026-07-23 | Security/protocol repair | pass | Shared metadata and every published file now reject URL/query/token/provider-payload variants. PUT failure performs exactly one read and zero write retries, accepting a committed create only after exact path/content/type/workspace/actor and expected post-write version `1` proof; identical version `2+` readback fails closed. Stale release-gate wording was corrected. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml ha2ha -- --nocapture` | pass | 12 matching tests cover clean/dirty/no-HEAD projection, strict real-scaffold/tamper rejection, whole-workspace restricted-content scans, one-use project/session and exact remote content/version binding, Viewer inspection-only join, mismatch/state/missing/duplicate failures, partial non-joinable sequencing, recovered task-write sequencing, and the pinned validator. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml mdsync_transport -- --nocapture` | pass | 17 matching tests include committed-write lost-response recovery by exact bounded readback, version `2+` rejection, mismatch rejection, and readback-failure behavior with no retry. |
| 2026-07-23 | Pinned `ha2ha-validate` invoked by focused Rust fixture | pass | Generated merged real-scaffold workspace validated with `"ok": true` against robosync commit `ebd5c8d483a26096f95fdcc8e4f5242270481e9b`. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | 161 Rust tests passed; native compilation and formatting passed with only the recorded forward-contract dead-code warnings. |
| 2026-07-23 | `bun run check` | pass | Typecheck, 54 frontend tests, and the production Vite build passed. |
| 2026-07-23 | `bun run tauri build --debug`; fresh post-review-repair native Task 015 resolver preview and deterministic controller fixture | pass | Fresh repaired-source executable selected only Task 015, reported no blocking gate, refreshed all five repository authority surfaces, stopped at the expected pre-closeout `verificationFailed`/`failureStop`, and started no second task. |
| 2026-07-23 | `output/native/task-015-envelope-native-smoke.jpeg` | pass | 1229x768 JPEG, SHA-256 `5210f16b46634e3f95c21276b8dac0418ce922181973d421488fde3f40528fdb`. |
| 2026-07-23 | `output/native/task-015-security-repair-native-smoke.jpeg` | pass | Post-review-repair 1229x768 JPEG, SHA-256 `9fd638ae63d78e524d64b439ae7415e24d09e89c6bb8a73947a4d6439d641a9a`. |
| 2026-07-23 | Independent GPT-5.6 Sol/high closure rereview | approved | F-015-01 through F-015-03 are closed; no material critical, high, or medium finding remains. Reviewer independently verified the exact-version repair, regression counts, artifact hash/dimensions, authority state, and absence of hosted requests. |

## Files Changed

- `src-tauri/src/ha2ha_envelope.rs` - strict HA2HA v1 projection, real MDSync
  scaffold validation, one-use publish plan, join reconciliation, typed repair,
  and pinned-validator fixtures.
- `src-tauri/src/mdsync_transport.rs` - sanitized session context and bounded
  accessors used without exposing capability material, plus exact bounded
  readback reconciliation for ambiguous committed writes.
- `src-tauri/src/collaboration.rs` - shared strict portable-metadata validation.
- `src-tauri/src/lib.rs` - resolver-bound preview/apply/join commands, exact
  local/remote revalidation, ordered partial publication, and integration tests.
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `output/native/task-015-envelope-native-smoke.jpeg`
- `output/native/task-015-security-repair-native-smoke.jpeg`
- `tasks/issues/015-publish-and-join-build-right-ha2ha-execution-envelopes.md`
- `docs/blueprint-status.md`
- `docs/release-gates.md`

## Verification Summary

- Focused HA2HA envelope/publish/join command: pass, 12 matching tests.
- Focused MDSync transport command: pass, 17 matching tests.
- Pinned real HA2HA v1 validator: pass.
- Full Rust regression: pass, 161 tests; compile and format pass.
- Full frontend regression: pass, 54 tests plus typecheck and production build.
- Fresh native executable and exact Task 015 resolver/one-task smoke: pass.
- Hosted MDSync: intentionally not run; Task 019 owns capability-bearing live
  acceptance after Tasks 016-018 complete.

## Learning Notes

- Proved: one source-bound Build Right task projects into a validator-conformant
  HA2HA v1 workspace layered onto the real pinned MDSync scaffold.
- Proved: preview/apply binds local task/Git authority plus every scaffold
  content/version, consumes confirmation before effects, and publishes the
  envelope-bearing task last so a partial write cannot be joined as complete.
- Proved: Viewer/public joins remain inspection-only; collaborator join is
  reconciled but deliberately non-executable until Task 016 adds claim binding.
- Simulated: hosted creation through deterministic transport until task 019.
- Test next: bind a remote claim to one-use local confirmation.

## Skill Trial Notes

- Source comparison: HA2HA v1 protocol and minimal-workspace fixture at clean
  robosync commit `ebd5c8d483a26096f95fdcc8e4f5242270481e9b`.
- Contract markers checked: workspace, task, actor, evidence, version, validation
- Router result: Sol/medium engineer with Sol/high independent review.
- Trial status: implementation, initial fail-closed review, repairs, full
  regressions, pinned validation, post-repair native smoke, and independent
  closure rereview approved.

## Blockers

- None.

## Follow-Ups

- Task 016 owns claim and pre-run execution gating.
