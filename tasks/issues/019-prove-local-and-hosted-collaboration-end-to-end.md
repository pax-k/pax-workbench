# 019: Prove Local And Hosted Collaboration End To End

Status: complete
Type: validation
Owner: AI

Assumption basis: founder-claimed plus repo-evidence-backed
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; docs/release-gates.md; tasks/sprint-2.md
Reversibility: easy
Learning objective: determine whether the signed workbench can coordinate one Build Right task through hosted HA2HA/MDSync without weakening local solo execution or leaking capabilities
Source under test: exact repo-local app plus exact pinned/deployed HA2HA/MDSync source

## Goal

Run one cohesive Sprint 2 acceptance packet covering unchanged local solo mode,
hosted publish/join, Viewer denial, Collaborator claim, remote conflict,
repository-verified execution, evidence/handoff synchronization, partial-sync
repair, restart, revocation, and two independent agent contexts.

## Non-Goals

- Claim customer validation or production-distribution readiness.
- Orchestrate multiple agents from the workbench.
- Run irreversible or production repository work.
- Validate provider-specific GitHub/Jira/CI/deployment adapters.
- Retain a live edit capability after the trial.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- docs/evidence/sprint-2-current-implementation-review.md
- docs/evidence/manual-trials.md
- tasks/sprint-2.md
- tasks/issues/013-018
- `/Users/pax/Documents/robosync/docs/v2/tasks/V2-012-url-based-ha2ha-agent-handoff.md`

## Acceptance Criteria

- [x] Tasks 013-018 are complete with passing evidence before the cohesive trial.
- [x] The exact signed app repeats the Sprint 1 local solo loop with no network dependency.
- [x] A fresh reversible Build Right task is published as one valid HA2HA
      execution envelope and joined through Viewer and Collaborator URLs.
- [x] Viewer inspection succeeds while Viewer claim/execution is denied before Codex.
- [x] A stale remote `baseVersion` conflict is observed and prevents Codex start.
- [x] A valid Collaborator claim starts exactly one separately confirmed Codex
      invocation and local completion is established only from repository evidence.
- [x] Remote evidence, handoff, status, event/history, and source binding are
      independently read back without capability leakage.
- [x] A deterministic or safely induced post-commit sync failure stops at
      repair-required; reconnect/repair completes missing remote state without rerunning Codex.
- [x] Restart reconstructs sanitized collaboration debt/state, retains no
      capability, starts nothing automatically, and requires reconnect plus explicit action.
- [x] A second independent agent context reads the handoff/evidence and can
      safely continue or review without private chat history.
- [x] Edit capability is revoked at closeout; retained Viewer access is optional
      and explicitly recorded.
- [x] Full frontend/Rust/build/signature checks and an independent release review pass.

## Baseline Evidence

Sprint 1 proves the signed local loop. The HA2HA/MDSync repository records live
multi-agent handoff, Viewer denial, version conflicts, evidence, capability
redaction, and revocation, but the workbench has not consumed those contracts.

## Solution-Fit Rationale

- Requirement served: prove optional shared execution around the local workbench.
- Constraints honored: reversible fixture, explicit actions, and no orchestration claim.
- Guarantees preserved: local authority, conflict stops, secret redaction, and revocation.
- Cost accepted: one structured hosted dogfood packet plus full local regression.
- Deferred capability: customer validation, production distribution, and background sync.

## Verification

- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- Signed development build and `codesign --verify --deep --strict`.
- Deterministic MDSync mock/conformance suite.
- Live hosted trial against the exact configured MDSync discovery origin.
- Capability scans over repository, goal state, logs, UI events, screenshots,
  runtime prompt/output, and retained workspace records.
- Strict Build Right resolver and HA2HA/MDSync readback after closeout.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | Tasks 013-018 tracker/evidence readback plus strict resolver | pass | Every dependency was complete and the strict resolver selected only Task 019 before the trial. |
| 2026-07-23 | Signed local fixture Task 989 | pass | The exact installed app ran one separately confirmed local Codex invocation, created the exact 19-byte local proof, verified repository truth, stopped with Task 990 ready, and did not start it automatically. |
| 2026-07-23 | Hosted discovery and empty-scaffold normalization | pass | Both configured discovery origins agreed; one workspace was created. The initial provider-generated status/participant content was normalized in place to the exact four-file HA2HA 1.0.0 scaffold; no second workspace was created. |
| 2026-07-23 | Viewer and Collaborator signed-app trial | pass | Viewer connected read-only and exposed no mutation action. Collaborator published exactly the decision, evidence, and task files and joined one `tasks/BR-990.md` envelope. |
| 2026-07-23 | Same-content remote version advance before confirmation | pass | Remote Task 990 advanced from version 1 to 2 after preview; confirmation ended with `Runtime started: false`, created no shared proof, left no Codex child, and required a fresh join/preview. The UI deliberately exposed the sanitized no-start result rather than raw provider conflict text. |
| 2026-07-23 | Fresh version-2 shared confirmation | pass | Exactly one subsequent shared Codex invocation started: one runtime-session/usage sequence followed the fresh confirmation. Repository evidence alone established Task 990 and fixture sprint completion; both exact proof assertions passed. Remote reconciliation completed at task version 4. |
| 2026-07-23 | Deterministic post-commit partial-sync and repair coverage | pass | Rust production-seam tests cover partial effect debt, restart-safe sanitized persistence, reconnect, idempotent repair, and no Codex rerun. The live hosted success path synchronized normally, so no live failure was invented. |
| 2026-07-23 | Restart of the exact signed app | pass | Reopened fixture showed terminal repository truth, a sanitized disconnected remote coordinate, no native session/capability, `Automatic Codex execution started: false`, and fresh action requirements. |
| 2026-07-23 | Independent Viewer readback | pass | A separate agent context read 9 hosted files, one remote task, 17 events, versions 1 ready / 2 ready / 3 claimed / 4 done, and matching completion evidence without private chat history. |
| 2026-07-23 | Edit revocation and retained Viewer probe | pass | Revocation succeeded; the old edit URL no longer authorized even a read, while the explicitly retained Viewer URL still listed 9 files. Signed-app inspection with the revoked edit session stopped before trusted state. |
| 2026-07-23 | Capability-retention scan | pass | Exact full URLs and extracted edit/read token values produced 0 matches across 1,293 files and more than 21 MB in the main repo, fixture, app support, and both WebKit stores. |
| 2026-07-23 | `bun run check` | pass | Typecheck, 69 frontend tests, and production bundle passed. |
| 2026-07-23 | Rust/check/format/signature ladder | pass | 203 Rust tests, `cargo check`, `cargo fmt --check`, strict deep code-sign verification, and installed-binary SHA-256 verification passed. |

## Files Changed

- `src-tauri/src/lib.rs` - resolve trusted absolute Bun/Bunx and pinned Codex
  Node launchers under the minimal LaunchServices environment; no shell fallback.
- `tasks/issues/019-prove-local-and-hosted-collaboration-end-to-end.md` - durable
  signed-local, live-hosted, restart, readback, revocation, and review evidence.
- `tasks/sprint-2.md`, `docs/blueprint-status.md`, and `docs/release-gates.md` -
  terminal Sprint 2 authority and next-gate boundary.
- `docs/evidence/manual-trials.md` - cohesive Sprint 2 acceptance packet.
- `output/native/task-019-*` - six sanitized 1229x768 signed-app artifacts.

## Verification Summary

- Signed artifact:
  `/Users/pax/Applications/Build Right Studio.app`; installed binary SHA-256
  `756500ab00fdffe9bb96f61e834cb209f03184fc05fc208833d3b463e1519d58`.
- Frontend: 69/69 tests plus typecheck and production bundle.
- Native: 203/203 Rust tests, `cargo check`, `cargo fmt --check`, and strict
  deep code-sign verification.
- Fixture: exact 19-byte local proof, exact 15-byte shared proof, two complete
  tasks, complete sprint, and strict `no-ready-task`.
- Hosted: one workspace, one Build Right envelope, one valid shared invocation,
  terminal version history, independent Viewer readback, revoked edit access,
  and retained read-only Viewer access.
- Secret boundary: 0 exact capability matches across repository, fixture, goal
  stores, app support, and WebKit surfaces included in the scan.

## Learning Notes

- Proved: signed local solo remained independent of hosted state; hosted
  Viewer/Collaborator access, one-envelope publish/join, stale-version no-start,
  one confirmed shared invocation, repository-authoritative completion, remote
  reconciliation, restart safety, independent readback, redaction, and edit
  revocation.
- Simulated/deterministic: post-commit partial-sync effect permutations,
  restart with incomplete effect debt, repeated conflict, timeout, and repair
  retry breadth remain production-seam test evidence. The live service completed
  the valid effect sequence normally.
- Unproved: customer usability/value, production signing/notarization/
  distribution, provider portability, multi-agent orchestration, and unattended
  execution.
- Test next: reconcile Sprint 3 against terminal Sprint 2, then run founder-led
  product-loop work only through its resolver-selected task.

## Skill Trial Notes

- Source comparison: exact workbench source, signed artifact, and pinned/deployed HA2HA/MDSync source
- Contract markers checked: local authority, access, conflict, evidence, repair, restart, redaction, revocation
- Trial status: pass; exact signed source plus pinned/deployed MDSync contracts
  passed local, deterministic, live-hosted, restart, revocation, and independent
  readback gates.

## Review Notes

- Independent read-only release review: PASS with no material or actionable
  findings after terminal authority drift was repaired.
- The reviewer independently verified the 9-file hosted packet, 17-event
  history, ready-v1 / stale-ready-v2 / claimed-v3 / done-v4 lineage, final
  evidence hash, exact fixture proofs, strict `no-ready-task`, full frontend and
  Rust gates, installed signature/hash, redaction, and terminal documentation.
- Residual boundaries remain the explicitly unproved customer, distribution,
  portability, orchestration, and unattended-execution claims above.

## Blockers

- None.

## Follow-Ups

- Reconcile Task 020 and Sprint 3 planning against this terminal packet before
  promoting any implementation task.
- Reassess customer validation only through the founder-facing Sprint 3 loop.
