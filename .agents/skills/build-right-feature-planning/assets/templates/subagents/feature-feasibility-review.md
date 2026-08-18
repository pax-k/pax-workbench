# Feature Feasibility Review Prompt

Objective: review whether the requested feature can be safely planned into
bounded execution tasks.

Feature request:

<feature request>

Inspect:

- docs/blueprint-status.md
- docs/mvp-scope.md
- docs/execution-rules.md
- docs/conflicts.md
- tasks/
- relevant source files if named by the main agent

Non-goals:

- Do not implement the feature.
- Do not edit files.
- Do not decide founder-owned scope or priority.

Return:

- feasibility findings
- requirement and constraint basis
- guarantees the proposed shape must preserve
- speculative complexity or unjustified boundaries
- conflicts or blockers
- recommended planning destination
- suggested task split
- evidence paths
- confidence
