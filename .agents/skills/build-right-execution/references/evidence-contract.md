# Execution Evidence Contract

Every completed task must record enough evidence that another person or agent
can understand why the task is complete without reading the whole conversation.

Use `workflow.md` for the execution loop, `gates.md` for stop/ask decisions,
and `review-and-delegation.md` for subagent review triggers. This file only
defines evidence and closeout shape.

## Task Intake Record

Use this at the start:

```text
Active task: <task id or path>
Done means: <observable completion criteria>
Non-goals: <explicit exclusions>
Assumption basis: <founder-claimed | ai-inferred | prototype-assumption | repo-evidence-backed | public-evidence-backed | customer-evidence-backed>
Requirement basis: <authority path, decision ID, evidence reference, or explicit assumption>
Reversibility: <easy | moderate | hard>
Learning hook: <how this task will produce evidence>
Source under test: <repo-local path | installed path | GitHub source | release tag | n/a>
Baseline evidence: <artifact or command>
Verification ladder: <focused -> broader checks>
Evidence destination: <task file or evidence file>
```

## Evidence Log Fields

Capture:

- baseline result
- source under test for skill trials: repo-local path, installed path, GitHub
  source, or release tag
- source comparison result for skill trials
- assumption basis
- reversibility and learning hook for prototype work
- files changed
- commands run
- exit codes or key output
- test names
- report paths
- screenshots or URLs when relevant
- generated artifacts kept or discarded
- what was proved
- what was only simulated
- assumption to test next
- known blockers
- follow-up issues

## Manual Trial Evidence Packet

For skill release/manual trials, record an agent-agnostic packet either in
`docs/evidence/manual-trials.md` or the selected task file. The packet must not
depend on a chat transcript or agent-specific conversation handle.

Required fields:

```text
Run label:
Agent/tool surface:
Skill source:
Target:
Commands:
Artifacts:
Result:
Proved:
Simulated:
Unproven:
Follow-ups:
```

Existing task evidence may hold command output and detailed notes; the manual
trial evidence document should keep the durable summary and links to task
artifacts.

## Task Evidence Section

Use this shape when updating task files:

```md
## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| YYYY-MM-DD | `<command>` | pass/fail/skipped | <key output or reason> |

## Files Changed

- `<path>` - <reason>

## Verification Summary

- `<command or proof>` - <result>

## Learning Notes

- Proved: <what evidence supports>
- Simulated: <what remains unproven>
- Test next: <assumption or follow-up>

## Skill Trial Notes

- Source under test: <repo-local path | installed path | GitHub source | release tag>
- Source comparison: <pass | mismatch | not applicable>
- Contract markers checked: <markers>
- Trial status: <pass | partial-needs-rerun | fail>

## Blockers

- <blocker or none>

## Follow-Ups

- <issue path/title or none>
```

## Completion Rules

A task is complete only when:

- acceptance criteria are satisfied
- required verification has been run or explicitly recorded as skipped with a
  reason
- evidence is recorded in the task or evidence file
- discovered unrelated work is captured as follow-up
- trackers/docs reflect the current state

Do not use vague completion statements. Say what was proved.

## Skipped Verification

Skipped verification is allowed only when it is impossible or inappropriate in
the current environment. Record:

- command or proof skipped
- reason
- risk left behind
- what would prove it later

## Not-Ready Blocker Shape

When execution is requested but the project lacks required operating artifacts,
create or propose the smallest blocker task:

```md
# 001: Establish Execution Baseline

Status: ready
Type: validation
Owner: AI

## Goal

Create the minimum authority docs, task tracker, and verification surface needed
to run bounded execution tasks with evidence.

## Non-Goals

- Implement product features.
- Resolve product positioning.
- Create production deployment unless required for validation.

## Required Reading

- AGENTS.md
- docs/blueprint-status.md
- docs/execution-rules.md

## Acceptance Criteria

- [ ] Execution authority docs exist or missing docs are explicitly tracked.
- [ ] One task tracker exists.
- [ ] Required verification commands are documented.
- [ ] Evidence destination is documented.

## Baseline Evidence

Record missing files and current validation commands before edits.

## Verification

- Inspect created docs and task tracker.
- Run documented validation command if available.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |

## Blockers

- None yet.

## Follow-Ups

- None yet.
```
