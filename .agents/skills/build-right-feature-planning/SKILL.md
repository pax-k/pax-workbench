---
name: build-right-feature-planning
description: Maintain Build Right feature planning after preflight and before execution. Use when the user wants to explore a desired feature, groom or reprioritize backlog/sprint work, run bounded product or technical research, use subagents for planning review, create or split execution-ready tasks, update planning evidence, or hand off a ready task to build-right-execution without implementing product code.
---

# Build Right Feature Planning

Use this skill after preflight has created product truth, operating rules, and a
task surface, and before execution starts implementation.

Core rule:

```text
Explore the feature.
Clarify only what changes the plan.
Research or delegate when evidence is needed.
Update backlog, sprint, decisions, evidence, conflicts, and task files.
Stop before implementation.
```

## Required Reading

- Always read `references/workflow.md` before acting.
- Read `references/gates.md` before asking founder questions, running
  research, changing sprint/backlog state, or declaring a task ready.
- Read `references/planning-contract.md` before creating or updating planning
  artifacts, sprint trackers, backlog rows, or task files.
- Read `references/research-and-delegation.md` only when web research,
  public-evidence claims, technical feasibility review, conflict review, or
  subagent delegation triggers apply.
- Read `../build-right-engineering-principles/references/principles.md` when
  splitting features into implementation tasks, selecting technologies,
  planning architecture, storage, integration, deployment, service-boundary,
  or package changes, designing contracts, changing provider boundaries, or
  planning security, reliability, observability, or test strategy.
- Use `assets/templates/tasks/feature-task.md` when creating a new execution
  task.
- Use `assets/templates/tasks/post-release-backlog.md` when creating a missing
  backlog tracker.
- Use bundled `scripts/feature-planning-check.ts` for deterministic planning
  surface, gate, destination, and task-candidate signals. Treat script output as
  input to judgment, not authority.

## Operating Mode

1. Inspect the project first. Read local agent instructions, Build Right docs,
   current sprint/backlog trackers, task files, conflicts, and evidence.
2. Run the read-only planning helper when available:

   ```sh
   bun <skill-path>/scripts/feature-planning-check.ts --cwd <project> --feature "<feature request>" --format markdown
   ```

3. Report the helper findings before writing:

   ```text
   Planning decision: <decision>
   Confidence: <confidence>
   Feature request: <feature or none>
   Recommended destination: <path or none>
   Blocking gates: <gates or none>
   Founder questions: <questions or none>
   Research triggers: <triggers or none>
   Ready task candidates: <tasks or none>
   Next action: <next action>
   ```

4. Reconcile the helper decision before updating artifacts:

   - `route-preflight`: stop and route back to `build-right-preflight`.
   - `ask-founder`: ask the smallest useful founder-question batch.
   - `run-research`: run bounded research and record public evidence.
   - `delegate-review`: run narrow subagent review when tooling is available.
   - `update-roadmap`: update roadmap or post-release backlog only.
   - `update-sprint`: update the active sprint with planned work.
   - `create-ready-tasks`: create or confirm execution-ready task files.
   - `blocked`: record or report the blocking gate and stop.

5. Announce a concise file plan:

   ```text
   Create:
   - <path> - <purpose>

   Update:
   - <path> - <purpose>

   Leave untouched:
   - <path> - <reason>

   Needs user input:
   - <question or blocker>
   ```

6. Update planning artifacts by default after the file plan unless the user
   requested planning-only chat, the write would overwrite ambiguous content, or
   a gate requires founder or external input first.
7. Do not modify product implementation files. Create task files and planning
   docs only.
8. Keep scope bounded. Split large features into sequential tasks with explicit
   assumptions, requirement basis, reversibility, learning objective, baseline
   evidence, and verification. Do not mark a task ready when its requirement
   basis is missing or contradicts current product truth; keep it in planning or
   route it to preflight.
9. Record research and claim basis in the right evidence file. Public research
   may support public-evidence-backed planning, not customer validation.
10. Record durable scope, priority, architecture, or workflow choices in the
    decision log with their requirement basis and tradeoff or guarantee impact.
    Record unresolved contradictions in conflicts.
11. Run the planning helper again after updates when available, then run the
    execution resolver if a task is expected to be ready. Use the full Bun
    command; do not rely on a PATH alias or short helper name:

    ```sh
    bun <build-right-execution-path>/scripts/continue-check.ts --cwd <project> --format markdown --strict
    ```

12. Before marking a sprint complete or advancing to the next sprint, verify
    that every row in the closing sprint is `complete`, `deferred`, `moved`,
    `canceled`, `split`, or `superseded`. If not, finish the work or make an
    explicit deferral/move/cancel/split/supersede decision with destination and
    gate evidence.
13. End with one explicit result:

    ```text
    Ready for execution: <task path>
    Updated backlog: <path>
    Updated sprint: <path>
    Needs founder decision: <question>
    Needs research before planning: <topic>
    Blocked: <gate and path>
    Route to preflight: <missing surface>
    ```

## Scope Rule

This skill may update product-planning files, evidence notes, sprint/backlog
trackers, and task files. It must not implement the feature. If the user asks
to continue into implementation, hand off to `build-right-execution` with the
ready task path.

## Stop States

Use one of:

```text
Ready for execution: <task path>
Updated sprint: <tracker path>
Updated backlog: <backlog path>
Needs founder decision: <question>
Needs research before planning: <topic>
Blocked: <reason>
Route to preflight: <missing artifact or gate>
```

## User-Visible Status Badge

End every final response with exactly one status badge block:

```text
🟢 [GREEN] Status: ALL GREEN
Decision: <decision/result>
Next action: <next action or none>
Needs user input: <none | concise ask>
Blocked by: <none | blocker>
```

Use this status map:

- `🟢 [GREEN] Status: ALL GREEN` for `Ready for execution`.
- `🟡 [YELLOW] Status: NEEDS INPUT` for `ask-founder` or
  `Needs founder decision`.
- `🟠 [ORANGE] Status: NEEDS WORK` for `Needs research before planning`,
  `route-preflight`, `delegate-review`, `run-research`, `update-roadmap`,
  `update-sprint`, `create-ready-tasks`, `Updated sprint`, `Updated backlog`,
  or `Route to preflight`.
- `🔵 [BLUE] Status: WAITING EXTERNAL` for external proof, publishing,
  indexing, credentials, paid services, production access, or third-party state.
- `🔴 [RED] Status: BLOCKED` for `Blocked`, open conflicts, route
  failures, failed verification, stale/source mismatch, or invalid state.
