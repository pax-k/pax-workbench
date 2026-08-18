---
name: build-right-execution
description: Execute one bounded task using baseline evidence, narrow implementation, verification, evidence capture, and tracker updates. Use when the user invokes /build-right-execution, wants to work the next issue, run an evidence-driven implementation loop, verify a task, update task state, or continue execution from a prepared task queue.
license: MIT
---

# Build Right Execution

Use this skill after pre-execution work has produced product truth, MVP scope,
operating rules, and an executable task queue.

Core rule:

```text
Do one task.
Prove the current state.
Change narrowly.
Verify with the right evidence.
Update the tracker.
Then move to the next task.
```

## Required Reading

- Always read `references/workflow.md` before acting.
- Read `references/gates.md` before task selection, after task intake, and
  before advancing to another task.
- Read `references/review-and-delegation.md` only when review triggers apply,
  subagent review is useful, or broad evidence/tracker changes are touched.
- Read `references/evidence-contract.md` before completing or updating a task.
- Read `../build-right-engineering-principles/references/principles.md` before
  implementation or review when a task touches architecture boundaries, public
  APIs, package ownership, provider adapters, generated code, error semantics,
  tests, observability, security, or enforceable policy.
- Use `assets/templates/task-template.md` when creating a missing task or
  splitting an overbroad task.
- Use bundled helper scripts only through the full Bun command form shown
  below. Do not rely on PATH aliases or invoke short names such as
  `continue-check` as shell commands.
- Use the state resolver before selecting a task or advancing through a queue.
  Report and reconcile its decision before continuing.
- Use the execution helper for deterministic task, contract, and gate signals.
  Treat script output as input to judgment, not authority.

## Operating Mode

1. Run the read-only state resolver when available:

   ```sh
   bun <skill-path>/scripts/continue-check.ts --cwd <project> --format markdown --strict
   ```

2. Report the resolver findings before selecting work:

   ```text
   Resolver decision: <decision>
   Confidence: <confidence>
   Next action: <next action>
   Next task: <task or none>
   Blocking gates: <gates or none>
   External follow-ups: <follow-ups or none>
   ```

3. Follow the resolver decision before selecting work:

   - `ask-founder`: ask or report the founder-owned gate; do not continue.
   - `wait-external`: report the external-state gate; do not continue.
   - `create-blocker`: create or propose the smallest AI-owned blocker.
   - `no-ready-task`: stop and report that no AI-owned task is ready.
   - `invalid-state`: stop and reconcile contradictory tracker/gate state.
   - `continue-active-task` or `execute-task`: select exactly that task.

4. Run the read-only execution helper when available:

   ```sh
   bun <skill-path>/scripts/execution-check.ts --cwd <project> --mode next-task --format markdown
   ```

5. Read task, sprint/milestone tracker, authority docs, and local agent
   instructions.
6. Print task intake:

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

7. Run the execution helper in `task-contract` or `all` mode when a task path
   exists, then reconcile any missing fields before editing.
8. Inspect current workspace state before editing.
9. Capture baseline evidence.
10. Implement the smallest change that satisfies the task and its requirement
    basis while preserving required guarantees.
11. Verify in layers.
12. Run subagent review when a required review trigger applies and subagent tools
   are available. If unavailable or forbidden, record the skipped review and
   substitute verification before closing.
13. Record evidence before marking the task complete.
14. Update only the relevant tracker/docs.
15. Run the full Bun state resolver command and the execution helper in
    `stop-gates` mode before selecting another task.
16. Report the resolver findings again before deciding whether another task is
    safe to select.
17. Stop at any founder, external-state, failed-verification, stale-task, source
    mismatch, open-conflict, non-AI-owner, or release-claim gate. Do not
    advance to the next task until the gate is resolved or explicitly converted
    into a ready AI-owned task.
18. Commit or hand off according to project workflow.

## Not-Ready Rule

If authority docs, MVP scope, execution rules, requirement basis, or task files
are missing, do not pretend execution is ready. Route the user to
`/build-right-preflight` or create the smallest Sprint 0 blocker needed to
establish the missing execution surface.

Prototype tasks may run from `prototype-assumption` only when reversibility,
learning hook, and validation required before product truth are explicit.

For skill release/manual trials, record the exact source under test. If the
installed or invoked skill is stale versus the repo-local skill source, mark the
trial `partial-needs-rerun` and do not advance release gates to ready.

## Scope Rule

Do not silently widen scope. If the task reveals unrelated work, create a
follow-up issue and continue only inside the selected task boundary.

## Stop/Ask Gate

After each task, check whether the next step is truly AI-owned and ready. Stop
and ask or report the blocker when the next step requires:

- founder-owned product, positioning, buyer/user, or MVP decisions
- ready or active task ownership that is not AI
- open conflicts in `docs/conflicts.md`
- external discovery, search indexing, publishing, secrets, paid services, or
  production access
- failed verification, stale task state, source mismatch, or ambiguous evidence
- required subagent review that was skipped without an equivalent substitute

Continuing through a prepared queue is allowed only while every next task is
ready, AI-owned, evidence-backed, and not blocked by one of these gates.

## User-Visible Status Badge

End every final response with exactly one status badge block:

```text
✅ [DONE] Status: DONE
Decision: <decision/result>
Next action: <next action or none>
Needs user input: <none | concise ask>
Blocked by: <none | blocker>
```

Use this status map:

- `✅ [DONE] Status: DONE` for a completed task with evidence,
  verification, and tracker updates.
- `🟢 [GREEN] Status: ALL GREEN` for `execute-task` or
  `continue-active-task` before implementation starts.
- `🟡 [YELLOW] Status: NEEDS INPUT` for `ask-founder`.
- `🟠 [ORANGE] Status: NEEDS WORK` for AI-owned follow-up work that is
  not yet a ready execution task.
- `🔵 [BLUE] Status: WAITING EXTERNAL` for `wait-external`, publishing,
  indexing, credentials, paid services, production access, or third-party state.
- `🔴 [RED] Status: BLOCKED` for `create-blocker`, `no-ready-task`,
  `invalid-state`, open conflicts, failed verification, stale/source mismatch,
  non-AI-owned task ownership, or skipped required review without substitute.
