# Execution Rules

## Authority Order

1. AGENTS.md and nested local instructions
2. docs/source-index.md
3. docs/mvp-scope.md
4. docs/release-gates.md
5. selected task file

## AI May Decide

- Implementation details inside an approved task boundary.
- Test structure that matches existing project patterns.
- Follow-up issue wording for unrelated discoveries.

## AI Must Ask

- Product promise changes.
- Primary customer or MVP boundary changes.
- Risky overwrites of substantial existing content.
- Changes requiring secrets, paid services, production access, or irreversible external actions.

## Stop/Ask Gates

- Founder-owned product, positioning, customer, or MVP decision required.
- Ready or active task ownership is not AI.
- Open conflict in docs/conflicts.md.
- External discovery, publishing, secrets, paid services, or production access required.
- Failed verification, stale task state, source mismatch, or ambiguous evidence.
- Required subagent review skipped without an equivalent substitute.

Ask the user when a user answer is required. Create a follow-up only when the
blocker is AI-owned, bounded, and evidence-backed.

## Subagent Review Triggers

Use subagent review when tooling is available and a selected task changes
release gates, manual-trial evidence, verifier behavior, workflows, contracts,
templates, helper scripts, diagrams, or multiple trackers. If a trigger applies
but subagent tooling is unavailable or forbidden, record the skipped review,
substitute verification, and residual risk before closing.

## Deterministic Helper Scripts

Use bundled helper scripts as read-only checks when available:

- Continue state resolver: `bun <skill-path>/scripts/continue-check.ts --cwd . --format markdown --strict`
- Preflight inventory/readiness: `bun <skill-path>/scripts/preflight-check.ts --cwd . --mode all --format markdown`
- Execution next task: `bun <skill-path>/scripts/execution-check.ts --cwd . --mode next-task --format markdown`
- Execution task contract: `bun <skill-path>/scripts/execution-check.ts --cwd . --task <path> --mode task-contract --format markdown`
- Execution stop gates: `bun <skill-path>/scripts/execution-check.ts --cwd . --task <path> --mode stop-gates --format markdown`

Helper output is evidence input, not authority. The preflight helper reports a
decision, next action, project type, missing artifacts, readiness warnings, and
founder input gaps. The main agent still reconciles helper findings with
founder input, repo evidence, web research, subagent findings, and the selected
task boundary.

Before continuing through a task queue, run the continue state resolver, report
its decision, confidence, next action, next task, blocking gates, and external
follow-ups, then follow its decision. Do not manually skim Markdown and skip a
resolver-reported founder, external-state, invalid-state, stale,
failed-verification, or source-mismatch gate.

## Workflow Customization

Project-local workflow customization belongs in this file or the project's
established execution-rules equivalent. Customization is additive policy around
the stable workflow backbone; it must not bypass stop/ask gates, ownership
checks, source-under-test checks, evidence capture, or verification.

Supported hooks:

| Hook | Policy |
| --- | --- |
| `before-task-select` | <policy before choosing or continuing a task> |
| `after-task-intake` | <policy after task goal, non-goals, owner, and evidence destination are known> |
| `before-edit` | <policy after baseline evidence and before file changes> |
| `after-verify` | <policy after verification commands or proof have run> |
| `after-evidence-recorded` | <policy after task evidence exists and before status changes> |
| `after-task-complete` | <policy after a task is marked complete, such as atomic commits> |
| `before-next-task` | <policy before selecting or continuing more work> |

Example additive policies:

- `after-task-complete`: create an atomic commit after each completed task.
- `after-verify`: require screenshot proof for UI work.
- `before-task-select`: no web research unless explicitly approved.
- `after-verify`: require stronger verification for release tasks.
- `after-evidence-recorded`: add a decision-log entry for architecture changes.

Forbidden customizations:

- skip resolver
- skip founder gate
- ignore external-state gate
- skip evidence
- skip verification
- auto-publish
- widen task scope

## Evidence Destinations

- Task evidence: tasks/issues/*.md
- Blueprint status: docs/blueprint-status.md
- Durable decisions: docs/decision-log.md
- New risks: docs/risk-register.md

## Required Verification

| Change Type | Required Checks | Evidence |
| --- | --- | --- |
| docs/task update | Inspect changed Markdown and linked paths | task Evidence Log |
| code behavior | Focused test plus relevant package/app check | task Evidence Log |
| UI behavior | Browser proof or screenshot when useful | task Evidence Log |
| helper script update | Script help and representative Bun smoke command | task Evidence Log |
| release readiness | Gate report and explicit go/no-go | release evidence path |
