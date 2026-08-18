# Pre-Execution Workflow

Use this workflow to turn founder context and existing project material into
trusted source docs, operating rules, Sprint 0, and the first executable AI
task.

## Reference Routing

- Read `founder-gates.md` before founder question batches, validation,
  readiness, or stop/ask decisions.
- Read `research-and-delegation.md` when public research, public evidence,
  existing-project inventory, conflict review, readiness audit, or subagent
  triggers apply.
- Read `artifact-contract.md` before creating or updating docs or task files.
- Use templates in `assets/templates/` instead of recreating long shapes.

## 1. Inspect Project State

Read available project context:

- local agent instructions
- README and docs
- tasks, issues, sprint trackers, or roadmap files
- package manifests and app/module layout
- validation, deploy, or release scripts
- existing product/customer/market language

Run the read-only helper when available:

```sh
bun <skill-path>/scripts/preflight-check.ts --cwd <project> --mode all --format markdown
```

Report its decision, confidence, project type, next action, missing artifacts,
readiness warnings, and founder input gaps. Use its output as deterministic
input, not as final judgment.

Classify the project:

- `blank/new`: little or no product truth, evidence, operating docs, or tasks
- `existing`: meaningful docs, code, release process, or previous decisions

Classify source mode:

- `founder-fed`
- `web-assisted`
- `public-first-prototype`

Create or update `docs/blueprint-status.md` early. It is the resume point.

## 2. Announce File Plan

Before writing, print:

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

Proceed by default with safe repo inventory and scaffolding. Stop before writing
only for planning-only mode, risky overwrites, or ambiguous project ownership.

Interaction gate: ask a focused founder-question batch before treating founder
intent, customer, positioning, MVP, or product promise as captured unless the
prompt or repo already contains explicit evidence. See `founder-gates.md`.

## 3. Capture Founder Context

Gather messy founder context:

- product idea
- target customer
- observed pain
- real-world examples
- known competitors
- pricing instincts
- desired capabilities
- user and system requirements
- non-goals
- hard constraints that rule solutions out
- soft constraints that rank viable solutions
- guarantees the solution must preserve
- constraints around time, money, team, and market timing

Useful outputs:

- `docs/raw/founder-dump.md`
- `docs/raw/founder-interview.md`
- `docs/open-questions.md`

AI may organize this material, but it does not become product truth until
validated or evidence-backed.

## 4. Tag Claims And Evidence

Turn raw input into structured draft sources. Every important claim must use one
claim status from `artifact-contract.md`.

For every important claim, ask:

```text
Do we have evidence, or only an assumption?
```

Evidence may come from direct customer material, public web sources, or local
repository artifacts. The evidence type must match the claim:

- repository evidence proves repository/package/release state
- public evidence supports market/category/public-source claims
- customer evidence supports customer truth

Unsupported claims remain assumptions.

## 5. Research And Delegation

Use web research only when public evidence can materially improve confidence,
expose contradictions, or fill reversible prototype gaps.

Use subagents when a required delegation trigger applies and tooling is
available. Do not skip required delegation silently. The main agent owns
synthesis, canonical writes, tracker updates, and gate closure.

See `research-and-delegation.md` for triggers, prompt contract, and evidence
destinations.

## 6. Create Canonical Operating Artifacts

Create or update the minimum durable artifacts needed by the project:

- `docs/source-index.md`
- `docs/decision-log.md`
- `docs/conflicts.md`
- `docs/mvp-scope.md`
- `docs/execution-rules.md`
- `docs/release-gates.md`
- `docs/evidence/*`
- `tasks/sprint-0.md`
- `tasks/issues/*.md`

For richer product preflight, also produce canonical product docs such as
product thesis, personas, problem statement, market notes, business model,
product vision, product flows, risk register, and manual ops playbook.

Use `artifact-contract.md` and templates under `assets/templates/`.

## 7. Extract MVP Or Prototype Scope

For founder-backed product work, extract:

- one primary customer
- one primary workflow
- one clear value moment
- smallest product that can be sold, demoed, or manually delivered
- explicit exclusions
- requirements with their evidence status
- hard constraints separated from soft constraints
- guarantees that the chosen shape must preserve
- main risk each MVP capability reduces
- what can be manual before automated

For `public-first-prototype`, extract the smallest reversible prototype instead:

- one plausible primary customer
- one plausible workflow
- one demoable value moment
- explicit assumptions and validation required
- no hard-to-reverse architecture, schema, pricing, or positioning commitments

## 8. Prepare Sprint 0

Sprint 0 is foundation work, not product feature work.

Typical tasks:

- validation baseline
- build/type/lint/test command surface
- environment and configuration contract
- persistence or migration home
- deployment path or deployment blockers
- core module boundaries
- first architecture risks
- first issue tracker shape

Optional foundation task templates are available under
`assets/templates/tasks/foundation/` for:

- architecture boundaries and module ownership
- tech stack and runtime choices
- directory structure and ownership map
- deployment path, Docker/self-hosting, cloud equivalents, and environment
  contract

Use these only when project inventory shows the foundation area is relevant.
They should surface as Sprint 0 tasks, not mandatory docs for every project.

Prepare the first executable task with assumption basis, requirement basis,
reversibility, learning objective, source under test, baseline evidence, and
verification.

## 9. Readiness Gate

Before handoff, rerun the read-only helper when available:

```sh
bun <skill-path>/scripts/preflight-check.ts --cwd <project> --mode readiness --format markdown
```

Report and reconcile the helper decision before claiming readiness.

Then answer:

- What source mode is active?
- Are product truth and MVP scope documented?
- Are assumptions separated from evidence?
- Are important requirements and constraints explicit?
- Are hard constraints separated from preferences?
- Are required guarantees recorded?
- Can the MVP shape be traced to those requirements and constraints?
- Are prototype assumptions clearly labeled?
- Are major conflicts resolved?
- Is manual delivery understood?
- Are authority docs and working rules in place?
- Is there a task tracker?
- Is the first task small, ordered, and verifiable?
- Is feature work allowed, or should Sprint 0 cleanup happen first?
- Is the first task only prepared, or did the user explicitly ask to continue
  into execution?

Stop/ask gates: use `founder-gates.md`. If a gate fires, ask, record the
blocker, or stop. Do not advance as if ready.

## Preflight And Execution Boundary

By default, this skill prepares the first executable task but does not complete
it. Stop after creating a bounded task and tell the user to run
`/build-right-execution` or `$build-right-execution`.

Continue into execution only when the user explicitly asks for end-to-end
preflight plus execution in the same run. Record that boundary crossing in the
task evidence log and closeout.

## Blank Project Handling

Create the starter scaffold progressively. Prefer the templates in
`assets/templates/`.

Do not require every canonical doc to be complete before Sprint 0. It is enough
for the status file to show missing evidence, blockers, and the first small task
needed to establish the project operating system.

## Existing Project Handling

Do not overwrite structure. First index what exists, then fill gaps.

For existing files:

- preserve content unless clearly stale or contradicted
- append evidence or decisions instead of replacing useful context
- create conflict notes when claims disagree
- keep local validation and release commands as the authority until proven wrong

When a project already has task tracking, adapt to it instead of forcing the
default `tasks/issues/` layout.
