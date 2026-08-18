# Pre-Execution Artifact Contract

Use strict structure where downstream execution depends on it. Use lighter
claim-tagged structure for founder discovery and product docs. Prefer templates
in `assets/templates/` instead of recreating long shapes from memory.

## Claim Statuses

Use one of:

- `founder-claimed`: founder stated it, not independently validated
- `ai-inferred`: AI inferred it from available material
- `prototype-assumption`: plausible enough for reversible prototype work, but
  not validated enough for product truth, sales claims, or durable commitments
- `repo-evidence-backed`: linked to local repository evidence such as manifests,
  source files, task files, release files, or command output
- `public-evidence-backed`: linked to public web evidence such as competitor
  pages, pricing pages, reviews, forums, public docs, or market notes
- `customer-evidence-backed`: linked to direct customer conversations, sales
  calls, user messages, support requests, interviews, or manual delivery results
- `unknown`: known gap

Do not present public web research as customer validation. Do not present local
repository evidence as market demand.

## Source Modes

Use one source mode for each preflight pass:

- `founder-fed`: founder context is primary; web research supports or challenges
  it
- `web-assisted`: founder context exists but bounded public research fills
  prototype-grade gaps
- `public-first-prototype`: founder context is missing or intentionally thin;
  public research may produce a reversible prototype scope, not product truth

Record source mode, prototype confidence, and validation required before any
prototype assumption is treated as product truth.

## Validation Marks

Use one of:

- `correct`
- `wrong`
- `unclear`
- `important-now`
- `post-mvp`
- `ignore`

## Template Map

Use these templates when creating new artifacts:

- `assets/templates/docs/blueprint-status.md`
- `assets/templates/docs/source-index.md`
- `assets/templates/docs/decision-log.md`
- `assets/templates/docs/conflicts.md`
- `assets/templates/docs/mvp-scope.md`
- `assets/templates/docs/execution-rules.md`
- `assets/templates/docs/release-gates.md`
- `assets/templates/docs/evidence/evidence-notes.md`
- `assets/templates/docs/raw/founder-dump.md`
- `assets/templates/docs/raw/founder-interview.md`
- `assets/templates/tasks/sprint-0.md`
- `assets/templates/tasks/issue-template.md`
- `assets/templates/tasks/foundation/architecture-boundaries.md`
- `assets/templates/tasks/foundation/tech-stack-runtime.md`
- `assets/templates/tasks/foundation/directory-structure.md`
- `assets/templates/tasks/foundation/deployment-env-contract.md`

If a target project has an established equivalent, adapt to it instead of
forcing these exact paths.

Foundation templates are optional. Use them as Sprint 0 tasks when inventory
shows architecture, stack, directory, deployment, environment, or boundary work
is needed before feature execution.

## Required Blueprint Status Fields

`docs/blueprint-status.md` or the project-local equivalent must include:

- status
- current phase
- project state
- source mode
- prototype confidence
- active task
- current gate
- last evidence
- readiness gates with evidence
- current file plan
- next action

Gate statuses: `missing`, `draft`, `needs-validation`, `blocked`, `ready`.

Use `docs/blueprint-status.md` as the lean state and resume file. Do not create
a second mandatory master state file unless the target project already has an
established equivalent that should be adapted.

## Decision Log Contract

Use `docs/decision-log.md` for durable decisions that future agents or
collaborators must respect:

- MVP boundary
- source mode
- architecture choice
- deployment choice
- workflow customization
- stop-gate decisions

Durable architecture, deployment, storage, integration, framework, service
boundary, and public-contract decisions must record the requirement basis and
the tradeoff or guarantee impact.

Do not use the decision log for routine command results, transient
implementation notes, every file edit, or evidence that belongs in a task
evidence log.

## Required Evidence Sections

Evidence docs should separate:

- public evidence
- customer evidence
- repository evidence
- unsupported important claims
- prototype assumptions
- evidence to gather next

Use `docs/evidence/customer-notes.md` only for direct customer evidence. Store
public research in competitor, pricing, or market evidence docs. Store local
repository proof in task evidence logs, release gates, source index entries, or
repository evidence sections.

For Build Right manual trials, use the agent-agnostic packet shape in
`docs/evidence/manual-trials.md` with these exact field labels:

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

Trial evidence must stand on durable artifacts, not an agent-specific
conversation handle.

## Required Task Fields

Every first executable task should include:

- status
- type
- owner
- assumption basis
- requirement basis
- reversibility
- learning objective
- `Source under test: <repo-local path | installed path | GitHub source | release tag | n/a>`
- goal
- non-goals
- required reading
- acceptance criteria
- baseline evidence
- verification
- evidence log
- blockers
- follow-ups

Use `assets/templates/tasks/issue-template.md` as the canonical task shape.
