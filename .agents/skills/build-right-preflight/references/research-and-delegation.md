# Research And Delegation

Use this reference only when public research, public-evidence claims,
existing-project inventory, conflict review, readiness review, or subagent
delegation triggers apply.

## Research Lane

Use web research when public evidence can materially improve confidence, expose
contradictions, or fill founder-context gaps for fast prototyping. Do not run
research blindly at every step.

Default loop:

```text
Founder input when available
-> AI extracts claims
-> AI identifies missing or research-worthy claims
-> Web research pass when useful or needed for fast prototyping
-> Evidence notes with sources
-> Contradictions and confidence updates
-> Prototype assumptions labeled when evidence is public-only or inferred
-> Founder clarification only for important unresolved points
-> Canonical doc update
```

Use web research for:

- competitors and alternatives
- pricing benchmarks
- market/category language
- public customer pain signals
- regulatory or platform constraints
- distribution/channel assumptions
- public reviews, forums, docs, and buying guides
- buyer vocabulary

Do not treat web research as enough to prove:

- whether this founder can sell the product
- whether a specific customer will buy
- true urgency of a pain
- willingness to pay
- feasibility inside a private workflow
- claims that require private customer conversations or internal data

Public research can upgrade a claim to `public-evidence-backed`, not
`customer-evidence-backed`. If research only makes a direction plausible enough
to prototype, use `prototype-assumption`.

Before web research, announce the narrow research pass, its purpose, and what it
will not prove. Cite sources when web research is used.

Record public research in:

- `docs/evidence/competitor-research.md`
- `docs/evidence/pricing-research.md`
- `docs/evidence/market-notes.md`

Use `docs/evidence/customer-notes.md` only for direct customer conversations,
messages, sales calls, interviews, support requests, or direct user feedback.

Ask follow-up questions only when research reveals important conflicts,
ambiguity, or unsupported critical claims.

## Delegation Lane

Subagents may gather, draft, critique, and audit. The main agent decides,
writes, updates trackers, and closes gates.

Required delegation triggers, when subagent tools are available:

- existing project inventory with multiple docs, task trackers, release gates,
  or code surfaces
- public research lane for competitors, pricing, market language, or public
  pain signals
- material conflict scan before resolving contradictions across docs
- readiness audit before closing a preflight as ready when more than simple
  scaffolding changed
- evidence completeness review when release/manual-trial claims are touched

Use subagents for:

- web research lanes
- existing project inventory
- competitor, pricing, market language, or public pain research
- conflict detection
- readiness review
- evidence completeness review
- large draft generation that the main agent will merge and edit

Do not use subagents for:

- founder interview flow
- final product promise decisions
- MVP boundary decisions
- authoritative final-state writes without main-agent review
- tracker updates without main-agent review
- committing, publishing, or external irreversible actions
- work requiring private context not explicitly passed to the subagent

Every subagent prompt must include:

- objective
- exact files or context to read
- scope boundaries
- allowed sources
- output format
- stop condition
- instruction not to edit files unless explicitly delegated

Use prompt templates in `assets/templates/subagents/`.

If a required trigger applies but subagent tools are unavailable, disabled, or
forbidden by the user, record:

- trigger that would have used a subagent
- why it was skipped
- confidence reduction or verification substituted
- whether the readiness gate remains blocked
