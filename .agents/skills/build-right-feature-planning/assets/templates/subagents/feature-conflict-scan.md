# Feature Conflict Scan Prompt

Objective: identify conflicts between the requested feature and current product
truth, MVP scope, sprint state, backlog, and evidence.

Feature request:

<feature request>

Inspect:

- docs/mvp-scope.md
- docs/decision-log.md
- docs/conflicts.md
- docs/evidence/
- tasks/sprint-*.md
- tasks/post-release-backlog.md

Non-goals:

- Do not implement the feature.
- Do not create or update tasks.

Return:

- conflicts found
- source paths
- severity
- owner recommendation
- whether the feature should be sprint, backlog, research-first, or blocked
- confidence
