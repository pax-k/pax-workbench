Objective:
Inspect the project for existing product truth, authority docs, validation
commands, release gates, and task trackers.

Context to read:
- `AGENTS.md`, `CLAUDE.md`, `README.md`
- `docs/`
- `tasks/`, `issues/`, roadmap or sprint files
- package manifests and app/module layout

Scope:
- Include: existing artifacts, missing artifacts, conflicts, and suggested file
  plan entries.
- Exclude: edits, canonical doc updates, tracker updates, product decisions.

Sources:
- Local project files only unless the main agent explicitly allows web research.

Output format:
- Existing surfaces
- Missing surfaces
- Conflicts
- Suggested file plan entries
- Confidence

Stop condition:
Return when the inventory can support a file plan. Do not edit files.
