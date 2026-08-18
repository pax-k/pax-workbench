# Product Discussion: Build Right Studio

Status: founder-claimed source material
Source thread: `codex://threads/019f849d-c700-7551-ab90-e5a952e023c2`
Captured: 2026-07-21
Repository decision: implement as the separate `pax-workbench` application

## Founder Prompt

Build a desktop application on top of the Build Right skills that can:

- open an engineering project;
- set up the project-scoped skills;
- use the skills to plan work;
- execute bounded units of work;
- continue through goal-driven loops;
- edit Markdown directly;
- expose skill domains and their operating rules visually; and
- show what the agent does sprint by sprint and task by task.

The visual reference supplied for the desktop shell was Tauri UI:
<https://tauriui.vercel.app/>.

## Product Direction

The product is a local-first engineering workbench, not a chat application with
a task board attached. The repository remains the source of truth. The desktop
application makes repository Markdown, skills, execution state, evidence, Git
state, and agent activity visible and editable.

Working product name: **Build Right Studio**.

The application should expose the existing workflow as a visible state machine:

```text
Open repository
-> inspect project
-> set up skills
-> preflight
-> plan feature
-> select one ready task
-> execute one task
-> verify and record evidence
-> resolve the next action
-> continue, ask, wait, block, or close the goal
```

The stable operating loop is:

```text
observe -> classify -> choose one action -> gates -> act -> verify -> record -> continue/stop
```

## Primary Workspace

The default window has four persistent surfaces:

1. **Project and sprint navigator** — canonical product documents, sprint and
   backlog tasks, decisions, conflicts, evidence, release gates, Git branch,
   and dirty-state indicators.
2. **Markdown workbench** — raw Markdown, rendered documents, structured task
   fields, sprint board, Git diff, and evidence comparison.
3. **Agent run inspector** — semantic execution events such as reading rules,
   resolving readiness, selecting a task, capturing a baseline, editing files,
   verifying behavior, and recording evidence. Raw command output stays
   available as an expandable detail.
4. **Execution ribbon** — a persistent, interactive representation of
   preflight, planning, task execution, verification, and next-action
   resolution. Each checkpoint should expose the decision and evidence behind
   it.

Sprint and task projections must come from repository Markdown. The application
must not create a proprietary shadow task database.

## Skill Domains

Build Right skills appear as operating modes:

| Domain | UI purpose | Main result |
| --- | --- | --- |
| Discover | Preflight, product truth, assumptions, MVP | Project ready for planning |
| Plan | Feature exploration, sprint placement, ready tasks | Bounded executable task |
| Build | One-task execution and verification | Completed task with evidence |
| Principles | Architecture, contracts, tests, security | Cross-cutting review lens |

Each skill operating card should show its purpose, inputs, allowed writes,
possible decisions, required evidence, and stop conditions. Full `SKILL.md`
content remains inspectable.

First-party skills need a validated machine-readable UI companion contract.
The application must not infer permissions or control decisions heuristically
from prose. A future `skill-ui.json`-style contract should describe lifecycle
phase, helper commands, decisions, read/write surfaces, evidence requirements,
stop states, and renderers. Unknown third-party skills initially receive a
generic Markdown viewer and explicit run action.

## Project Onboarding

1. Pick a local repository.
2. Inspect Git state, agent instructions, package manager, docs, tasks,
   installed skills, and Build Right readiness.
3. Show a setup preview.
4. Install or update project-scoped skills through the supported skill tooling.
5. Run the read-only preflight helper.
6. Present missing artifacts and the recommended next action.
7. Enter Discover, Plan, or Build mode.

The interface must show the exact installed location and source/version of each
skill. Repository-local, installed, GitHub, and release-tag sources must remain
visibly distinct.

## Goal Loop

A goal is durable application state with an objective, repository, current
sprint, current task, and explicit stop conditions. Each iteration:

1. runs the deterministic resolver;
2. starts one agent invocation using the execution skill;
3. streams structured events into the run inspector;
4. refreshes repository and Git state;
5. verifies evidence and tracker updates from files rather than agent claims;
6. runs the resolver again; and
7. starts the next iteration or stops at the exact gate.

The application owns orchestration, the agent runtime owns adaptive work,
helpers classify repository state, and repository Markdown remains planning
authority.

## Technical Direction

```text
React interface
  -> Markdown editor and renderer
  -> sprint/task/evidence projections
  -> structured agent event stream
  -> Tauri command boundary
      -> bounded filesystem and file watching
      -> Git inspection
      -> Bun helper execution
      -> skill installation
      -> pluggable agent runtime adapter
  -> repository Markdown + Git + Build Right skills
```

The first agent adapter may invoke `codex exec --json -C <project>` and parse
its event stream. Later runtimes should implement the same local contract.

SQLite may be used as a disposable index for event search and UI restoration.
It must not become authoritative for tasks, planning, evidence, or repository
state.

## Visual Direction

The product should feel like a precise engineering instrument rather than a
generic SaaS dashboard.

- Graphite `#17191C` — application chrome
- Alloy `#E6E8E9` — document surface
- Blueprint blue `#356AE6` — active work
- Verification teal `#2E9F8D` — proven state
- Gate amber `#C78A28` — input or waiting
- Fault red `#C95858` — failed or blocked

Use a compact humanist sans for navigation and prose and a technical monospace
face for commands, task IDs, paths, evidence, and state transitions. The
execution ribbon is the one expressive visual element; the rest should be
quiet, dense, keyboard-friendly, and native-feeling.

## MVP Boundary

Include:

1. Open and inspect one local repository.
2. Inspect installed Build Right skills.
3. Edit and render repository Markdown.
4. Show Discover, Plan, Build, and Principles domains.
5. Parse sprint and task Markdown into navigable views.
6. Run deterministic helpers through an explicit command boundary.
7. Execute exactly one task through a runtime adapter.
8. Stream actions, commands, diffs, verification, and evidence.
9. Run a checkpointed goal loop until a real stop condition.
10. Resume prior run state after restarting the application.

Defer:

- multi-agent orchestration;
- third-party skill marketplaces;
- cloud synchronization;
- issue-tracker integrations;
- visual workflow builders;
- production publishing, signing, and distribution.

## Product Promise And Validation Target

Founder-claimed promise: a founder or engineer can understand and control an
evidence-backed coding loop without living in a terminal, while retaining
ordinary repository files as the inspectable source of truth.

The first validation target is whether the MVP can open this repository, show
its real Build Right artifacts, safely edit Markdown, expose current workflow
state, and simulate or execute one bounded runtime-adapter task without creating
a competing task database.

## Implementation Risks

- The machine-readable skill UI/runtime contract is not yet established.
- Agent runtime output and permission boundaries require a stable adapter.
- Markdown parsing must preserve unknown content and avoid destructive rewrites.
- Filesystem and command access must be explicitly scoped to the opened project.
- Tauri UI is a visual/scaffolding input, not the workflow engine.

All claims in this document are `founder-claimed` unless a later evidence or
decision record upgrades their status.
