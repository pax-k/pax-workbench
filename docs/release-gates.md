# Release Gates

Status: sprint-3-active
Last updated: 2026-07-23

## Gates

| Gate | Required Evidence | Command or Proof | Status |
| --- | --- | --- | --- |
| Frontend validation | typecheck, tests, and production bundle succeed | `bun run check`; tasks/issues/001-build-local-workbench-mvp.md | ready |
| Product scope | MVP and exclusions are explicit | docs/mvp-scope.md | ready |
| Task evidence | completed task contains command results and residual risk | tasks/issues/001-build-local-workbench-mvp.md | ready |
| Skill UI contract | first-party contracts validate and generic fallback is non-executable | tasks/issues/003-validate-skill-ui-contracts.md; `bun run check` | ready |
| Native boundary | Rust unit tests and Tauri compile succeed | tasks/issues/004-verify-native-tauri-boundary.md; `cargo test`; `bun run tauri build --debug` | ready |
| Repository session | one real repository round-trips Markdown with stale-write protection | tasks/issues/005-complete-safe-repository-session.md; `output/native/task-005-native-round-trip.jpeg` | ready |
| Skill setup | project-scoped setup preview and allowlisted install/update are proved | tasks/issues/006-add-explicit-skill-setup-adapter.md; `output/native/task-006-native-setup-success.png` | ready |
| Helper execution | a contract-declared helper returns bounded structured evidence | tasks/issues/007-implement-deterministic-helper-execution.md; `output/native/task-007-native-helper-smoke.jpeg` | ready |
| Runtime adapter | one Codex JSONL invocation normalizes events and cancellation | tasks/issues/008-implement-codex-runtime-adapter.md; `output/native/task-008-native-runtime-live.jpeg` | ready |
| Bounded execution | one ready AI-owned task is executed and verified from repository evidence | tasks/issues/009-execute-one-bounded-task.md; `output/native/task-009-controller-verified-reviewed.jpeg` | ready |
| Goal persistence | checkpointed state resumes without shadow planning authority | tasks/issues/010-persist-checkpointed-goal-state.md; `output/native/task-010-resumable-after-restart.jpeg`; `output/native/task-010-git-changed-after-restart.jpeg` | ready |
| Goal loop control | resolver-driven confirmed iterations stop at the exact gate | tasks/issues/011-run-confirmed-goal-loop.md; `output/native/task-011-signed-runtime-restart.jpeg`; `output/native/task-011-native-goal-complete.jpeg` | ready |
| Real workflow trial | one cohesive post-dependency signed-app loop is dogfooded with durable manual-trial evidence | tasks/issues/012-prove-mvp-end-to-end.md; docs/evidence/manual-trials.md; `output/native/task-012-final-*.jpeg` | ready |

## Go/No-Go Format

```text
Go: signed local Sprint 1 technical dogfood; Task 012 passed independent release review
Go next: execute resolver-selected Task 031 authority-drift enforcement while retaining every Sprint 1 and Sprint 2 gate
No-go: production distribution, unattended autonomous loops, or multi-agent orchestration
```

## MVP Evidence Boundary

| Classification | Current Boundary |
| --- | --- |
| Proved | Local repository authority, validated skill provenance, safe repository session, real helpers, real Codex JSONL, bounded task verification, checkpoint recovery, two confirmed iterations, signed development app, full frontend/Rust/debug-build suite |
| Simulated | Exhaustive terminal stop-family breadth and concurrency/race variants covered by production-seam tests |
| Not proved | Customer usability/value, provider portability, non-Unix process containment, and the cause of earlier signed pre-event stalls |
| Post-MVP | Production signing, notarization, publishing/distribution, cloud or issue-tracker integration, marketplaces, unattended/parallel/multi-agent execution |

The durable agent-agnostic trial packet is `docs/evidence/manual-trials.md`.

## Sprint 2 Collaboration Gates

Sprint 1 gates above remain the local solo regression baseline.

| Gate | Required Evidence | Command or Proof | Status |
| --- | --- | --- | --- |
| Collaboration contracts | Provider-neutral types, non-extractable native-only capability material, nominal session/evidence/handoff references, closed typed failures/repair/effect-debt variants, separate local/remote versions, injected policy/port seams, explicit local-only default | tasks/issues/013-define-collaboration-contracts-and-native-seams.md; `output/native/task-013-all-output-boundaries-native-smoke.jpeg` | ready |
| Native MDSync session | Strict trust-pinned URL/discovery/access parsing, bounded HTTP/input, zeroized owned capability temporaries, generation-safe lifecycle, in-memory capability, complete redaction | tasks/issues/014-implement-secure-native-mdsync-session-transport.md; `output/native/task-014-security-repair-native-smoke.jpeg` | ready |
| HA2HA execution envelope | One resolver-selected task projects onto the strictly validated real MDSync scaffold, passes pinned v1 validation, binds exact local/remote baselines, publishes task last, and joins without backlog mirroring, capability leakage, ambiguous-write duplication, or invented Git proof | tasks/issues/015-publish-and-join-build-right-ha2ha-execution-envelopes.md; `output/native/task-015-security-repair-native-smoke.jpeg` | ready |
| Pre-run remote claim | Viewer denial and stale `baseVersion` prevent Codex; valid claim binds one-use confirmation | tasks/issues/016-bind-remote-ha2ha-claims-to-confirmed-execution.md | ready |
| Post-run reconciliation | Local completion remains authoritative; partial remote sync becomes idempotent repair debt | tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md | ready |
| Collaboration UX | Access, source binding, conflict, sync, restart, and repair are legible without secret rendering | tasks/issues/018-add-shared-collaboration-and-repair-ui.md | ready |
| Cohesive hosted proof | Local solo regression, hosted publish/join/claim/evidence/repair, independent context, redaction, revocation, review | tasks/issues/019-prove-local-and-hosted-collaboration-end-to-end.md; `output/native/task-019-*` | ready |

## Sprint 2 Go/No-Go Format

```text
Go: optional shared HA2HA mode passes local, deterministic, signed-native, and hosted acceptance
No-go: whole-backlog sync, persistent capabilities, multi-agent orchestration, or production distribution
Next gate: continue the reconciled Sprint 3 dependency chain
```

## Sprint 3 Productization Gates

Sprint 3 was reconciled against the completed HA2HA/MDSync implementation.
Tasks 031, 022, 020, 021, 023, 024, 025, 026, 027, 028, 028A, 029, and 030 are
complete. Task 032 remains planned until the founder participates; AI replay
cannot satisfy that evidence gate.

| Gate | Required Evidence | Command or Proof | Status |
| --- | --- | --- | --- |
| Authority drift | README, source, blueprint, sprint/task, release, command, dependency, and predecessor-state mismatches fail normal checks | tasks/issues/031-add-docs-and-authority-drift-enforcement.md; `bun run authority:check` | ready |
| Unified workflow and repair contracts | Existing local goal/controller and optional shared states compose into one typed product projection without duplicate authority | tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md | ready |
| Frontend modular boundary | Behavior-preserving project/session/workflow/collaboration projections outside both root components | tasks/issues/020-extract-frontend-project-session-and-workflow-projections.md | ready |
| Native modular boundary | Focused repository/Git and local/shared controller ownership preserving existing collaboration modules | tasks/issues/021-extract-native-repository-and-workflow-controller-modules.md | ready |
| Safe artifact creation | Allowlisted plan/preview/confirm/apply with containment, stale-state, and partial-result proof | tasks/issues/023-implement-safe-new-project-artifact-plan-and-apply-boundary.md | ready |
| Guided bootstrap | Empty repository reaches truthful canonical authority without terminal assistance | tasks/issues/024-build-guided-discover-and-project-bootstrap-experience.md | ready |
| Functional planning | One feature reaches exact resolver state through previewed planning-only effects | tasks/issues/025-build-functional-feature-planning-experience.md; docs/evidence/task-025-feature-planning.md | ready |
| Goal continuity | Project/goal-centered local/shared open, resume, block, review, continue, repair, and complete states | tasks/issues/026-make-shell-goal-centered-and-recovery-aware.md; docs/evidence/task-026-goal-centered-shell.md | ready |
| Product action clarity | Outcome workflows are distinct from developer diagnostics and typed repair | tasks/issues/027-separate-product-workflows-from-developer-diagnostics.md; docs/evidence/task-027-product-diagnostic-hierarchy.md | ready |
| Outcome review | Local plus optional shared diffs, checks, criteria, evidence/handoff/repair, risks, tracker truth, and next action form one receipt | tasks/issues/028-add-post-run-diff-and-evidence-review-receipt.md; docs/evidence/task-028-post-run-review-receipt.md | ready |
| Safe local Git handoff | Only reviewed paths can be explicitly staged and locally committed; no push or destructive action | tasks/issues/028a-add-safe-local-git-handoff-and-commit-boundary.md; docs/evidence/task-028a-safe-local-git-handoff.md | ready |
| Information architecture | Searchable goal-centered navigation, contextual collaboration, progressive evidence, and useful 900x700 layout | tasks/issues/029-rework-navigation-information-architecture-and-responsive-layout.md; docs/evidence/task-029-information-architecture.md | ready |
| Accessibility and visual behavior | Local/shared keyboard, semantics, contrast, zoom, high contrast, reduced motion, and visual regression proof | tasks/issues/030-enforce-accessibility-and-visual-behavior.md; docs/evidence/task-030-accessibility-visual-behavior.md | ready |
| Founder product-loop proof | Founder-led signed-native bootstrap, plan, execute, review, optional commit/shared checkpoint, restart/resume, and truthful stop | tasks/issues/032-run-founder-usability-trial-and-close-product-loop.md | planned |

## Sprint 3 Go/No-Go Format

```text
Go: founder-led signed-native product loop and all regression/release reviews pass
No-go: material usability failure, stale authority, inaccessible critical path, or weakened Sprint 1/Sprint 2 guarantee
Next gate: run Task 032 with the founder present; do not substitute scripted AI replay
```
