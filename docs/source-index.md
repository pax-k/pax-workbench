# Source Index

| Document | Purpose | Status | Confidence | Owner | Last Reviewed |
| --- | --- | --- | --- | --- | --- |
| docs/raw/product-discussion.md | Founder context and product direction | raw | medium | founder | 2026-07-21 |
| docs/raw/founder-dump.md | Canonical founder-context index | captured | medium | founder | 2026-07-21 |
| docs/raw/founder-interview.md | Structured founder-gate answers from the source thread | captured | medium | founder + AI | 2026-07-21 |
| docs/mvp-scope.md | Active local plus optional shared product boundary and Sprint 3 phase | active | medium | founder | 2026-07-23 |
| docs/execution-rules.md | Rules for AI and application effects | active | high | founder + AI | 2026-07-22 |
| docs/release-gates.md | Sprint 1-2 proof and Sprint 3 productization gates | active | high | founder + AI | 2026-07-23 |
| docs/ha2ha-mdsync-reconciliation.md | HA2HA/MDSync authority, state, security, and failure contract | accepted | high | founder + AI | 2026-07-22 |
| docs/native-module-boundaries.md | Native command, repository/Git, controller-port, and collaboration module direction | active | high | AI | 2026-07-23 |
| docs/decision-log.md | Durable product and architecture choices | active | medium | founder + AI | 2026-07-23 |
| docs/conflicts.md | Contradictions and state-mismatch reconciliation | resolved | high | founder + AI | 2026-07-22 |
| docs/evidence/preflight.md | Repository and environment evidence | active | high | AI | 2026-07-22 |
| docs/evidence/manual-trials.md | Cohesive signed native Sprint 1 and Sprint 2 trial packets | complete | high | AI | 2026-07-23 |
| docs/evidence/sprint-2-current-implementation-review.md | Sprint 1 code/test baseline and collaboration integration gaps | complete | high | AI | 2026-07-22 |
| docs/evidence/founder-workflow-ui-ux-audit.md | Current product, workflow, UI/UX, accessibility, and engineering gap evidence | complete | high | AI | 2026-07-22 |
| docs/evidence/sprint-3-post-ha2ha-reconciliation.md | Post-Sprint-2 implementation comparison and revised Sprint 3 task order/scope | complete | high | AI | 2026-07-23 |
| docs/evidence/task-020-frontend-extraction.md | Focused/full and live-browser evidence for the behavior-preserving frontend extraction | complete | high | AI | 2026-07-23 |
| docs/evidence/task-021-native-extraction.md | Focused/full, debug-build, and live-launch evidence for native module seams | complete | high | AI | 2026-07-23 |
| docs/evidence/task-023-artifact-plan.md | Security, filesystem, debug-build, and startup proof for safe artifact creation | complete | high | AI | 2026-07-23 |
| docs/evidence/task-024-guided-bootstrap.md | Focused, signed-native, provenance, preflight, and local-only proof for guided bootstrap | complete | high | AI | 2026-07-23 |
| docs/evidence/task-025-feature-planning.md | Authenticated helper, editable diff, safe planning update, signed-native, and strict resolver proof | complete | high | AI | 2026-07-23 |
| docs/evidence/task-026-goal-centered-shell.md | Strict recent preferences, goal-shell projection, signed restart, and native reinspection proof | complete | high | AI | 2026-07-23 |
| docs/evidence/task-027-product-diagnostic-hierarchy.md | Product-action hierarchy, diagnostic containment, typed repair, and signed-native visual proof | complete | high | AI | 2026-07-23 |
| docs/evidence/task-028-post-run-review-receipt.md | Bounded/redacted Git evidence, unified local/shared receipt, full regressions, and signed-native failed/completed proof | complete | high | AI | 2026-07-23 |
| docs/evidence/task-028a-safe-local-git-handoff.md | Exact selected-path staging/commit security, repair, full regression, and signed-native unrelated-dirt preservation proof | complete | high | AI | 2026-07-23 |
| docs/evidence/task-029-information-architecture.md | Grouped navigation, responsive pane contracts, exact viewport captures, full regression, and signed-native minimum-window proof | complete | high | AI | 2026-07-23 |
| docs/evidence/task-030-accessibility-visual-behavior.md | Automated semantics/keyboard, readable type, preference media, deterministic visual, signed-native, and VoiceOver proof | complete | high | AI | 2026-07-23 |
| docs/founder-facing-workflow.md | Accepted target journey, states, effects, repair, IA, and visual contract | accepted | high | founder + AI | 2026-07-22 |
| tasks/sprint-0.md | Completed foundation tracker | complete | high | founder + AI | 2026-07-21 |
| tasks/sprint-1.md | Controlled MVP loop execution tracker | complete | high | founder + AI | 2026-07-22 |
| tasks/sprint-2.md | Optional HA2HA/MDSync shared collaboration tracker | complete | high | founder + AI | 2026-07-23 |
| tasks/sprint-3.md | Active founder-facing product-loop tracker reconciled after Sprint 2 | active | high | founder + AI | 2026-07-23 |
| tasks/post-release-backlog.md | Explicit post-MVP exclusions | active | high | founder | 2026-07-21 |

## Authority Notes

- Repository Markdown, Git state, helper results, and recorded evidence are
  authoritative for project state.
- `.agents/skills/**` defines installed workflow guidance but not this product's
  scope.
- Application indexes and demo data are projections, never planning authority.
- HA2HA execution envelopes are portable collaboration projections; MDSync
  hosting, comments, history, and activity are not Build Right task authority.
- `bun run authority:check` is the read-only structural enforcement surface for
  indexed documents, local Markdown links, sprint/task state, dependencies,
  active pointers, release gates, supported README commands, and mechanically
  provable product claims. It reports drift but never rewrites authority.
