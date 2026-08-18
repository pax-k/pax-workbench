# Feature Planning Workflow

Use this loop when a user brings a new feature, asks to update sprint/backlog
work, or wants to turn discovery into execution-ready tasks.

## Loop

1. Read local instructions and Build Right artifacts:
   - `docs/blueprint-status.md`
   - `docs/mvp-scope.md`
   - `docs/execution-rules.md`
   - `docs/decision-log.md`
   - `docs/conflicts.md`
   - `docs/evidence/**`
   - `tasks/sprint-*.md`
   - `tasks/post-release-backlog.md`
   - `tasks/issues/*.md`
2. Run `scripts/feature-planning-check.ts`, report its decision, then reconcile
   it against repo evidence and founder intent.
3. Classify the feature:
   - product intent or priority decision
   - current-sprint implementation candidate
   - backlog or roadmap candidate
   - research-first candidate
   - review/delegation candidate
   - blocked or contradictory candidate
4. Ask only the founder questions that change priority, scope, user promise,
   product truth, requirements, constraints, required guarantees, or acceptance
   criteria.
5. Use research or subagents only when the claim cannot be safely decided from
   founder input and repo evidence.
6. Write planning artifacts:
   - update backlog or sprint rows
   - create or split task files
   - update decision/evidence/conflict docs when the feature changes those
     surfaces
7. Before closing a sprint or advancing the project phase, confirm every row in
   the closing sprint is `complete`, `deferred`, `moved`, `canceled`, `split`,
   or `superseded`.
8. Re-run the planning helper. If a task should be executable, run the execution
   resolver and confirm the task is visible to `build-right-execution`.
9. Stop before implementation.

## Planning Decisions

- `route-preflight`: required product truth, operating rules, or task surface is
  missing. Do not create feature tasks until preflight establishes the surface.
- `ask-founder`: a decision is founder-owned. Ask a small focused batch.
- `run-research`: public evidence is needed before making or prioritizing a
  planning claim.
- `delegate-review`: broad technical feasibility, conflict review, or evidence
  audit is useful before writing task files.
- `update-roadmap`: feature belongs in a roadmap or post-release backlog.
- `update-sprint`: feature belongs in the current sprint but is not yet split
  into execution-ready tasks.
- `create-ready-tasks`: create or confirm bounded AI-owned task files that the
  execution resolver can select.
- `blocked`: open conflict, external gate, stale state, or invalid task surface
  prevents safe planning.

## Feature Decomposition

Prefer a thin sequence:

1. validation or baseline task
2. smallest product behavior task
3. integration or persistence task
4. UI/UX proof task
5. release or monitoring task when relevant

Each task needs one goal, explicit non-goals, assumption basis, reversibility,
requirement basis, learning objective, baseline evidence, and verification. If
those fields cannot be written truthfully, do not mark the task ready. Missing
or contradictory requirement basis routes back to preflight or remains in
planning.

## Solution-Fit Rationale

When a task selects or changes architecture, persistence, integration,
deployment, framework, service boundaries, or a public contract, record:

```md
## Solution-Fit Rationale

- Requirement served: <current requirement>
- Constraints honored: <hard constraints>
- Guarantees preserved: <integrity, simplicity, isolation, or other guarantee>
- Cost accepted: <real tradeoff introduced>
- Deferred capability: <future flexibility intentionally not implemented>
```

Do not invent this rationale. If product truth, evidence, or an explicit
reversible assumption cannot support it, keep the task out of `ready`.

## Handoff

The ideal closeout is:

```text
Ready for execution: tasks/issues/<id>-<slug>.md
```

If no task is ready, close with the next planning gate instead of implying that
execution can start.
