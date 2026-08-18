# 021: Extract Focused Native Repository And Workflow Controller Modules

Status: complete
Type: architecture
Owner: AI

Assumption basis: repo-evidence-backed
Requirement basis: docs/evidence/founder-workflow-ui-ux-audit.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Tasks 020 and 022
Reversibility: easy
Learning objective: prove native responsibilities can be changed independently without weakening the Tauri security boundary
Source under test: repo-local path

## Goal

Extract only the repository/Git and goal/bounded/shared-controller ownership
needed by upcoming artifact planning and result review from the native root.

## Non-Goals

- Change an effect, permission, Tauri command name, or serialized response.
- Add new artifact creation or planning behavior.
- Perform a broad rewrite unrelated to the guided workflow.
- Re-extract or redesign `collaboration.rs`, `mdsync_transport.rs`, or
  `ha2ha_envelope.rs`.
- Move stable helper/runtime process mechanics unless an extracted controller
  contract requires a small compatibility-preserving seam.

## Required Reading

- docs/execution-rules.md
- docs/evidence/founder-workflow-ui-ux-audit.md
- docs/founder-facing-workflow.md
- tasks/issues/013-define-collaboration-contracts-and-native-seams.md
- tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md
- tasks/issues/020-extract-frontend-project-session-and-workflow-projections.md

## Acceptance Criteria

- [x] Repository inspection/persistence and filesystem/Git read boundaries have
      explicit module ownership suitable for artifact plans and review diffs.
- [x] Goal persistence and local/shared bounded-controller orchestration have
      explicit module ownership while depending on existing collaboration
      contracts and ports.
- [x] Tauri command names, request/response contracts, effect order, redaction,
      timeout/cancellation, and stale-write guarantees remain compatible.
- [x] Core controller code depends on ports/contracts rather than WebView state.
- [x] Existing collaboration, MDSync transport, and HA2HA envelope modules stay
      authoritative; local/remote version types and effect order are unchanged.
- [x] Command registration is thin and compatibility tests prove no command or
      serialized-contract drift.
- [x] Module direction is documented and enforced where practical by tests.
- [x] Rust tests, checks, formatting, frontend checks, and debug build pass.

## Baseline Evidence

`src-tauri/src/lib.rs` is 17,250 lines after Sprint 2 and still holds project,
setup, helper/runtime, goal, controller, command registration, and most tests.
Collaboration policy, MDSync transport, and HA2HA envelopes are already
separate, well-tested modules; a broad extraction would duplicate or destabilize
proved boundaries.

## Solution-Fit Rationale

The focused extraction lowers change risk for artifact planning, review diffs,
and local Git handoff while avoiding refactoring stable process or
collaboration mechanics without a requirement.

## Verification

- Focused native module and compatibility tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- `bun run tauri build --debug`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | Focused native module suites | pass | 7 tests cover injected Git reads, closed controller selection/stops, effect/stop declarations, and exact command registration |
| 2026-07-23 | Full frontend/native regression | pass | `bun run check` passed 91 tests/build; Rust passed 219 tests, check, and format |
| 2026-07-23 | Debug native build and launch | pass | Tauri debug bundle built; native binary launched and remained healthy until deliberate smoke termination |
| 2026-07-23 | Independent review | skipped | Subagent delegation was unavailable; exact command-contract tests, focused port tests, full regressions, debug build, live launch, and local diff review substituted |

## Files Changed

- src-tauri/src/lib.rs
- src-tauri/src/command_contract.rs
- src-tauri/src/repository_service.rs
- src-tauri/src/workflow_controller.rs
- docs/native-module-boundaries.md

## Verification Summary

- Passed 7 focused module/compatibility tests.
- Passed `bun run check`: authority drift, typecheck, 91 tests, and build.
- Passed 219 Rust tests, `cargo check`, and `cargo fmt --check`.
- Passed `bun run tauri build --debug` and a real debug-binary launch smoke.

## Learning Notes

- Proved: Git inspection and bounded-controller policy now depend on focused
  native ports/contracts; the 26-command Tauri registration is closed and
  mechanically compared with the compatibility list.
- Simulated: injected Git failures validate adapter independence.
- Test next: implement artifact planning through these seams.

## Skill Trial Notes

- Source comparison: project-scoped installed skills
- Contract markers checked: module ownership, ports, compatibility, effects
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 023 implements the first new artifact effect through these seams.
