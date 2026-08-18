# Blueprint Status

Status: sprint-3-active
Current phase: Sprint 3 - founder-facing product loop
Project state: existing
Source mode: founder-fed
Prototype confidence: medium
Active task: tasks/issues/032-run-founder-usability-trial-and-close-product-loop.md
Current gate: founder-observed cohesive signed-native product trial
Last evidence: Task 030 completed automated semantics, keyboard containment,
type floors, zoom/contrast/motion behavior, deterministic visual captures, full
regressions, signed-native 900x700, and VoiceOver keyboard acceptance
Last updated: 2026-07-23

Planned next phase: Sprint 3 continues with outcome review.

## Readiness

| Gate | Status | Evidence | Notes |
| --- | --- | --- | --- |
| Founder intent captured | ready | docs/raw/product-discussion.md | Source thread and repository choice recorded |
| Claims tagged | ready | docs/raw/product-discussion.md | Founder-owned claims remain founder-claimed |
| Prototype assumptions labeled | ready | docs/mvp-scope.md | Runtime and parser risks remain prototype assumptions |
| Evidence recorded | ready | docs/evidence/preflight.md | Repository and environment evidence recorded |
| Canonical docs exist | ready | docs/source-index.md | Minimum authority surface created |
| Conflicts resolved | ready | docs/conflicts.md | No product conflicts; native toolchain is a release gate |
| MVP extracted | ready | docs/mvp-scope.md | One user, workflow, value moment, and exclusions |
| Manual ops understood | ready | docs/mvp-scope.md | Local inspection and explicit task start remain user-controlled |
| Operating rules exist | ready | docs/execution-rules.md | Filesystem and command boundaries explicit |
| First task is bounded and verifiable | ready | tasks/issues/001-build-local-workbench-mvp.md | Completed with frontend evidence and recorded native gate |
| Sprint 2 authority reconciled | ready | docs/ha2ha-mdsync-reconciliation.md | Build Right stays authoritative; HA2HA/MDSync is optional collaboration |
| Sprint 2 collaboration contracts | ready | tasks/issues/013-define-collaboration-contracts-and-native-seams.md | Complete: typed provider-neutral seams, closed output boundaries, full regressions, native smoke, and independent approval |
| Sprint 2 native transport | ready | tasks/issues/014-implement-secure-native-mdsync-session-transport.md | Complete after adversarial repairs, full regression/native proof, and independent Sol/high approval |
| Sprint 2 execution envelope task is bounded | ready | tasks/issues/015-publish-and-join-build-right-ha2ha-execution-envelopes.md | Complete after fail-closed security/protocol repairs and independent Sol/high approval |
| Sprint 2 remote claim task is bounded | ready | tasks/issues/016-bind-remote-ha2ha-claims-to-confirmed-execution.md | Complete after post-claim lifecycle repairs and independent Sol/high approval |
| Sprint 2 post-run reconciliation task is bounded | ready | tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md | Complete after HIGH capability-alias repair and independent Sol/high approval |
| Sprint 2 collaboration UX task is bounded | ready | tasks/issues/018-add-shared-collaboration-and-repair-ui.md | Complete after capability-containment repair, automated/native proof, and independent Sol/high approval |
| Sprint 2 cohesive hosted proof is bounded | ready | tasks/issues/019-prove-local-and-hosted-collaboration-end-to-end.md | Complete: signed local and hosted packet, independent readback, revocation, redaction, and full verification passed |
| Sprint 3 post-HA2HA reconciliation | ready | docs/evidence/sprint-3-post-ha2ha-reconciliation.md; tasks/sprint-3.md | Tasks 031, 022, and 020-030 are complete; Task 032 remains founder-gated |
| Authority drift enforcement | ready | tasks/issues/031-add-docs-and-authority-drift-enforcement.md; `bun run authority:check` | Read-only structural checks run first in `bun run check` and cover indexed docs, links, task/sprint state, dependencies, active pointers, gates, commands, and claims |
| Unified product workflow contracts | ready | tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md | Stateless TypeScript/Rust projections compose existing local/shared contracts; plan, receipt, failure, repair, Viewer, and secret boundaries are focused-test enforced |
| Frontend projection ownership | ready | tasks/issues/020-extract-frontend-project-session-and-workflow-projections.md | Root components now consume focused project/session, workflow, collaboration projection, and effect modules; focused/full tests and live browser smoke pass |
| Native module ownership | ready | tasks/issues/021-extract-native-repository-and-workflow-controller-modules.md; docs/native-module-boundaries.md | Git reads, controller policy/ports, and exact Tauri command compatibility have focused ownership and full native/debug proof |
| Safe artifact creation | ready | tasks/issues/023-implement-safe-new-project-artifact-plan-and-apply-boundary.md | Two local-only commands provide exact create-only preview/confirm/apply with baseline, one-use token, operation lock, idempotency, and partial receipts |
| Guided project bootstrap | ready | tasks/issues/024-build-guided-discover-and-project-bootstrap-experience.md; docs/evidence/task-024-guided-bootstrap.md | Empty/partial inventory, founder inputs, exact preview/confirmation, trusted setup, truthful preflight, restart/resume, and local-first collaboration gating pass |
| Functional feature planning | ready | tasks/issues/025-build-functional-feature-planning-experience.md; docs/evidence/task-025-feature-planning.md | One feature reaches an editable create/update preview, confirmed local apply, helper readback, and exact strict resolver-selected ready task |
| Goal-centered continuity | ready | tasks/issues/026-make-shell-goal-centered-and-recovery-aware.md; docs/evidence/task-026-goal-centered-shell.md | Strict recent preferences, native reinspection, authoritative goal/task projection, shared context, and signed restart recovery pass |
| Product action clarity | ready | tasks/issues/027-separate-product-workflows-from-developer-diagnostics.md; docs/evidence/task-027-product-diagnostic-hierarchy.md | One classified product action per state, progressive diagnostics, typed repair routing, contextual principles, and signed-native hierarchy pass |
| Outcome review receipt | ready | tasks/issues/028-add-post-run-diff-and-evidence-review-receipt.md; docs/evidence/task-028-post-run-review-receipt.md | Local/shared result, bounded changes, criteria/checks, tracker, risks, repair debt, raw events, and explicit review choices pass full and signed-native proof |
| Safe local Git handoff | ready | tasks/issues/028a-add-safe-local-git-handoff-and-commit-boundary.md; docs/evidence/task-028a-safe-local-git-handoff.md | Exact reviewed paths/message, clean-index stop, one-use baseline, filter/hook isolation, commit readback, repair, and signed-native unrelated-dirt preservation pass |
| Responsive information architecture | ready | tasks/issues/029-rework-navigation-information-architecture-and-responsive-layout.md; docs/evidence/task-029-information-architecture.md | Grouped/searchable sprint navigation, breadcrumbs/history, progressive panes, 900x700/1440x900 captures, and signed-native minimum-window navigation pass |
| Accessibility and visual behavior | ready | tasks/issues/030-enforce-accessibility-and-visual-behavior.md; docs/evidence/task-030-accessibility-visual-behavior.md | Axe and keyboard contracts, readable type floors, zoom/contrast/motion media behavior, deterministic captures, signed-native 900x700, and VoiceOver pass |

## Current File Plan

### Update

- No AI-owned implementation task is ready. Preserve Task 030 evidence while
  preparing the founder-observed Task 032 trial.

### Leave Untouched

- `.agents/skills/**` because these are installed upstream skill packages.
- `skills-lock.json` unless skill dependencies themselves change.
- Later product surfaces outside Task 030.

### Needs User Input

- Founder participation is required for Task 032; scripted AI replay cannot
  satisfy the acceptance contract.
- Production distribution, persistent capability storage, multi-agent
  orchestration, or a public protocol-profile commitment requires separate
  founder authorization.

## Next Action

Run `tasks/issues/032-run-founder-usability-trial-and-close-product-loop.md`
with the founder present. Stop rather than substituting AI inference.
