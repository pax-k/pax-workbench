# Build Right Studio

A local-first desktop engineering workbench that makes repository Markdown,
Build Right skills, sprint/task state, workflow gates, and agent evidence
visible without replacing the repository as source of truth.

The completed Sprint 1 local MVP includes:

- a three-pane project, document, and run workspace;
- raw Markdown editing, rendered preview, and structured task projection;
- visible Discover, Plan, Build, and Principles operating modes;
- an inspectable execution ribbon;
- explicitly labeled simulated, adapter, manual, and real run evidence;
- a typed Tauri 2 command boundary for scoped project inspection, stale-safe
  Markdown writes, skill setup, deterministic helpers, and Codex JSONL;
- one-use confirmation, cancellation, repository-evidence verification,
  checkpoint persistence/recovery, and resolver-controlled goal iteration; and
- a development-signed native two-confirmation end-to-end trial.

Completed Sprint 2 adds optional MDSync-hosted HA2HA coordination around one selected
Build Right task. Repository Markdown and Git remain authoritative, local solo
mode remains the default, and MDSync is not a backlog or completion authority.
See [`docs/ha2ha-mdsync-reconciliation.md`](docs/ha2ha-mdsync-reconciliation.md)
and [`tasks/sprint-2.md`](tasks/sprint-2.md).

Sprint 3 is the active founder-facing productization phase. Its post-HA2HA
reconciliation, revised task order, and explicit local Git handoff are recorded
in [`docs/evidence/sprint-3-post-ha2ha-reconciliation.md`](docs/evidence/sprint-3-post-ha2ha-reconciliation.md)
and [`tasks/sprint-3.md`](tasks/sprint-3.md).
The current bounded-run review combines outcome, bounded current-worktree
changes, criteria/checks, tracker state, risks, optional sanitized shared
evidence, and explicit no-effect review choices in one receipt. A separate
native handoff can then inspect, preview, and explicitly commit only selected
eligible receipt paths; it never pushes or changes completion authority.
The responsive shell groups repository truth by authority and sprint, adds
search/filter plus document history, and lets navigation and evidence collapse
around one primary workflow canvas down to a signed-native 900x700 window.
Accessibility is part of the normal release contract: automated semantic and
keyboard checks, readable type floors, zoom/contrast/motion adaptations,
deterministic visual captures, and signed-native VoiceOver evidence are recorded
in [`docs/evidence/task-030-accessibility-visual-behavior.md`](docs/evidence/task-030-accessibility-visual-behavior.md).

Product direction and scope are recorded in
[`docs/raw/product-discussion.md`](docs/raw/product-discussion.md) and
[`docs/mvp-scope.md`](docs/mvp-scope.md).

## Develop the interface

```sh
bun install
bun run dev
```

Open <http://localhost:1420>. Browser mode uses a clearly labeled demo
projection and does not write files or execute commands.

## Validate

```sh
bun run check
```

This first checks repository authority/documentation consistency, then runs
TypeScript checks, Vitest, and a production Vite build. Run the deterministic
authority check by itself with:

```sh
bun run authority:check
```

## Run the desktop shell

Tauri requires Rust and the platform prerequisites described in the
[official Tauri prerequisite guide](https://v2.tauri.app/start/prerequisites/).
After installing them:

```sh
bun run tauri dev
```

The native shell uses a directory picker and exposes only project-scoped Tauri
file commands. Deterministic helper subprocesses remain explicit user actions
and run with normal host permissions; the current MVP does not sandbox them.
Native compilation, tests, a debug build, and an Apple-development-signed local
trial are proved on this machine. Production signing, notarization, publishing,
and distribution remain out of scope.

## Authority

- Repository Markdown and Git state are authoritative.
- UI cards and boards are projections of files.
- Browser demo events are simulated, not execution evidence.
- The app runs only explicitly confirmed bounded agent work and never resumes
  execution automatically after restart.
- Installed workflow skills under `.agents/skills/` are dependencies and change
  only through the explicit previewed/confirmed first-party setup action.
- HA2HA execution envelopes are optional collaboration projections; Build Right
  helpers and repository evidence still decide readiness and completion.
