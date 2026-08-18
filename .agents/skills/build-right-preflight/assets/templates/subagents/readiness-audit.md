Objective:
Audit whether the project is ready for Sprint 0 or bounded execution.

Context to read:
- `docs/blueprint-status.md`
- `docs/source-index.md`
- `docs/mvp-scope.md`
- `docs/execution-rules.md`
- `docs/release-gates.md`
- `tasks/`

Scope:
- Include: go/no-go, blockers, missing evidence, and first executable task
  quality.
- Exclude: edits, final readiness declaration, tracker updates, commits.

Sources:
- Local project files only unless the main agent explicitly allows web research.

Output format:
- Go/no-go recommendation
- Blockers
- Evidence paths
- Confidence
- Recommended next task

Stop condition:
Return when readiness can be judged by the main agent. Do not edit files.
