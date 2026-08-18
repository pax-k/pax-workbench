# Feature Planning Gates

Use these gates before changing sprint/backlog state or declaring a feature
execution-ready.

## Founder-Owned Gates

Stop and ask when the feature depends on:

- target user, buyer, or customer promise
- MVP inclusion or exclusion
- priority relative to active work
- acceptance criteria that affect product behavior
- pricing, packaging, positioning, or launch commitment
- irreversible scope or data-retention decisions

Ask one small batch. Do not convert founder-owned questions into AI-owned tasks.

## Research Gates

Run bounded research when planning depends on:

- competitors, alternatives, public pricing, or market vocabulary
- regulatory, platform, legal, payment, app-store, or policy constraints
- public technical feasibility for an unfamiliar integration
- public evidence that may challenge founder claims

Public research can produce `public-evidence-backed` claims. It does not produce
customer validation.

## Delegation Gates

Use subagents when available for:

- existing-project inventory across many files
- technical feasibility review across broad surfaces
- conflict scan between feature request and MVP/roadmap docs
- evidence completeness audit before marking task candidates ready

The main agent still decides, writes artifacts, and owns closeout.

## Blocked Gates

Stop when:

- preflight artifacts are missing or contradictory
- `docs/conflicts.md` has an unresolved material conflict
- a sprint or milestone is being marked complete while any row remains
  non-terminal
- the only next action is external-state work
- the feature would require product implementation before planning evidence
- task files cannot satisfy the execution task contract
- a ready task would not be AI-owned

Record the gate in the relevant artifact when writing is safe. Otherwise report
the blocker and stop.

## Sprint Closure Gate

Before marking a sprint complete or moving the project state to the next sprint,
scan every row in the closing sprint tracker.

Allowed terminal row statuses are:

- `complete`
- `deferred`
- `moved`
- `canceled`
- `split`
- `superseded`

Any `planned`, `draft`, `ready`, `active`, `in_progress`, `blocked`,
`needs-founder`, or external-wait row blocks closure. Either finish the work or
make a deliberate defer/move/cancel/split/supersede decision with destination,
owner, and approval gate recorded.

## Execution-Readiness Gate

A task is ready for `build-right-execution` only when:

- it is listed in a sprint or backlog tracker
- its task file exists
- `Status:` is `ready`
- `Owner:` is `AI`, `agent`, or `Codex`
- assumption basis, requirement basis, reversibility, learning objective, and
  source under test are explicit
- acceptance criteria, baseline evidence, verification, blockers, and follow-ups
  are present
- the requirement basis does not contradict current product truth, constraints,
  or required guarantees
- no founder, external, conflict, or failed-verification gate blocks it
