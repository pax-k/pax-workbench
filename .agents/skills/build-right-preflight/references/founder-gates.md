# Founder Gates

Use this reference before asking founder questions, validating founder-owned
claims, or deciding whether preflight is ready to hand off to execution.

## Interaction Gate

In interactive runs, ask a focused founder-question batch before treating these
as captured:

- founder intent
- primary customer or user
- product promise
- positioning
- MVP boundary
- pricing or willingness-to-pay claims

If the prompt or repository already contains explicit answers, record the
evidence path instead of asking again.

If the user does not answer, continue only with repository inventory,
scaffolding, and blockers. Mark affected claims as `founder-claimed`,
`needs-founder`, `prototype-assumption`, or `unknown`; do not promote them into
validated product truth.

## Question Batches

Ask small batches. Prefer the smallest batch that unblocks the next gate:

- Who feels the pain most sharply?
- What do they do today without this product?
- What event triggers urgency or buying intent?
- What promise can we make without lying?
- What is the smallest sellable or manually deliverable version?
- Which assumptions have no direct evidence yet?

For existing projects, ask only unresolved founder-owned questions discovered
by inventory, such as primary user, product promise, MVP boundary, or public
positioning.

## Founder Validation

Ask the founder to mark important claims:

- `correct`
- `wrong`
- `unclear`
- `important-now`
- `post-mvp`
- `ignore`

Founder owns product promise, customer definition, positioning, and MVP
boundary. AI can recommend; it cannot decide these.

Record validation in `docs/source-validation.md`, `docs/decision-log.md`, or
the project-local equivalent.

## Stop/Ask Gates

Stop or ask when:

- founder-owned product promise, customer, positioning, or MVP boundary is
  unresolved
- public research is required for the selected source mode and has not run
- a required delegation trigger was skipped without a substitute review
- the next step requires publishing, paid services, secrets, production access,
  directory-indexing claims, or other external state changes
- only a blocker remains and advancing would avoid the founder question

Close with one explicit state:

```text
Go for prototype
Go for Sprint 0
No-go for product features
Needs founder/customer validation before product commitment
First blocker: <task path>
First executable AI task: <task path>
```

Do not claim product-feature readiness until product truth, MVP scope, evidence,
operating rules, and at least one bounded executable task exist.
