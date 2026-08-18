# Review And Delegation

Use this reference after implementation or evidence updates when independent
review would improve confidence or a required trigger applies.

Subagents may gather, critique, and audit. The main agent decides, writes,
updates trackers, and closes gates.

## Required Review Triggers

Run subagent review when tooling is available and any trigger applies:

- release gates, release checklist, manual-trial evidence, or verifier behavior
  changed
- skill workflows, artifact contracts, evidence contracts, templates, helper
  scripts, or diagrams changed
- more than three durable docs/task files changed in one task
- a verification command failed and was fixed inside the same task
- task status changes across multiple trackers
- findings imply a founder-owned, external-state, stale-task, source-mismatch,
  or release-claim gate

If a required trigger applies but subagent tools are unavailable, disabled, or
forbidden by the user, record the skipped review, substitute verification, and
residual risk before closing. If no adequate substitute exists, stop at the gate
instead of advancing.

## Useful Review Lanes

- evidence completeness review
- scope creep review
- focused risk review for high-blast-radius changes

Use templates in `assets/templates/subagents/` when present.

## Prompt Contract

Every review prompt must include:

- objective
- exact task file, changed files, or evidence to inspect
- scope boundaries
- output format
- stop condition
- instruction not to edit files

Do not use subagents for final tracker updates, commits, publishing, or
authoritative task closeout unless the user explicitly asks.

## Handling Findings

If findings are in scope, fix them and rerun verification. If findings are out
of scope, create a follow-up issue and keep the current task bounded.

Do not let review expand the task into a broader refactor.
