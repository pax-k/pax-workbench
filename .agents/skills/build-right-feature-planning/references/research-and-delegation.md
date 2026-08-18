# Research and Delegation

Read this only when the feature needs public evidence or independent review.

## Research Lane

Keep research bounded:

1. Name the planning claim under test.
2. Search only enough to support or challenge that claim.
3. Prefer primary sources for platform, policy, regulatory, pricing, or API
   claims.
4. Record source links, dates, and confidence in `docs/evidence/`.
5. Update `docs/decision-log.md` only when the research changes a durable
   product or workflow decision.
6. Update `docs/conflicts.md` when public evidence materially contradicts
   founder claims or existing scope.

Do not let research drift into implementation.

## Delegation Lane

Use a narrow subagent prompt from `assets/templates/subagents/` when available.
Include:

- objective
- feature request
- relevant repo paths
- sources to inspect
- explicit non-goals
- output format
- stop condition

Subagents may gather, critique, and audit. The main agent synthesizes, decides,
writes, and closes gates.

## Required Subagent Output

Ask for:

- findings
- evidence paths or source links
- conflicts or blockers
- recommended planning destination
- task-splitting risks
- confidence

Do not ask subagents to implement the product feature.
