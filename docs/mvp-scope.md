# MVP Scope

Status: sprint-3-ready
Owner: founder
Confidence: medium
Source mode: founder-fed
Prototype confidence: medium
Last updated: 2026-07-23

## Primary Customer

A founder or engineer using repository-native Build Right workflows who needs to
understand and control an evidence-backed coding loop without living in a
terminal. (`founder-claimed`)

## Primary Workflow

Open one local repository, inspect its authority and execution state, edit a
Markdown artifact, run a deterministic readiness helper, and execute one real
bounded AI-owned task through an explicit runtime adapter. Simulation is only
fixture/pre-live validation, not fulfillment of the workflow. Sprint 2 adds an
optional shared mode that publishes or joins one MDSync-hosted HA2HA execution
envelope around the same local task; local solo mode remains the default.
(`founder-claimed`)

## Value Moment

The user can see exactly why the workflow is ready, waiting, or blocked and can
trace that state to ordinary repository files and evidence. (`founder-claimed`)

## Requirements And Constraints

| Requirement or Constraint | Kind | Evidence Status | Design Consequence |
| --- | --- | --- | --- |
| Repository files remain source of truth | hard | founder-claimed | No proprietary task-state database |
| Show Discover, Plan, Build, and Principles domains | user outcome | founder-claimed | Persistent skill-domain navigation |
| Make workflow checkpoints inspectable | user outcome | founder-claimed | Execution ribbon maps to real states and evidence |
| Scope app-mediated filesystem reads and writes to the opened repository | hard | ai-inferred | Tauri file commands validate project roots, relative paths, and symlink targets |
| Preserve unknown Markdown | hard | ai-inferred | Raw editor is always available; structured views are projections |
| Support future runtime adapters | soft | founder-claimed | Frontend depends on a typed bridge rather than Codex directly |
| Rust/Cargo and a debug native build are available | hard environment | repo-evidence-backed | Native verification is executable; production signing/distribution remain excluded |
| Build Right remains local planning and completion authority | hard | founder-claimed | HA2HA projects only the selected executable task; MDSync never selects or completes local work |
| Shared collaboration is optional | hard | founder-claimed | Local solo execution has no network dependency and remains a regression gate |
| MDSync URLs contain bearer capabilities | hard security | repo-evidence-backed | Parse and retain capabilities only in a native in-memory session; redact every other surface |

## Guarantees To Preserve

- No hidden task/planning authority outside the opened repository.
- No command execution without an explicit user action and visible boundary.
- No app-mediated file read or write outside the selected project root.
- User-triggered helper and agent subprocesses run with host permissions; they
  are not sandboxed by the current MVP and must remain explicit actions.
- Unknown Markdown remains editable as raw source.
- Helper and agent claims are checked against repository state.
- Local file hashes and HA2HA remote integer versions remain separate.
- A failed remote update never rolls back repository-verified local completion;
  it stops shared continuation at an explicit repair-required state.

## Included

| Capability | User Outcome | Risk Reduced | Evidence |
| --- | --- | --- | --- |
| Project workbench shell | Understand one repository at a glance | Product-shape risk | docs/raw/product-discussion.md |
| Markdown editor and preview | Inspect and change authority files directly | Shadow-state risk | docs/raw/product-discussion.md |
| Sprint/task projections | Navigate work without replacing Markdown | Workflow legibility risk | docs/raw/product-discussion.md |
| Skill operating cards | Understand skill purpose and stop gates | Unsafe-action risk | docs/raw/product-discussion.md |
| Execution ribbon and run inspector | Trace decisions and evidence | Observability risk | docs/raw/product-discussion.md |
| Tauri bridge contracts | Prepare safe local project inspection and helper execution | Integration risk | docs/execution-rules.md |
| Project-scoped skill setup | Make skill source, version, and installation effects explicit | Provenance risk | docs/raw/product-discussion.md |
| Deterministic helper execution | Show readiness and resolver decisions from real helpers | State-classification risk | docs/raw/product-discussion.md |
| One bounded Codex task execution | Prove the workbench can supervise one real unit of work | Runtime-integration risk | docs/raw/product-discussion.md |
| Checkpointed goal persistence | Resume orchestration without creating shadow planning authority | Continuity risk | docs/raw/product-discussion.md |
| Optional HA2HA execution envelope | Coordinate one selected task across independent human-agent contexts | Duplicate-work and handoff risk | docs/ha2ha-mdsync-reconciliation.md |
| Native MDSync session adapter | Publish or join hosted collaboration without exposing bearer capabilities | Secret and provider-boundary risk | docs/ha2ha-mdsync-reconciliation.md |
| Shared claim, evidence, handoff, and repair | Stop conflicts before execution and repair partial sync without rerunning work | Conflict and partial-write risk | docs/ha2ha-mdsync-reconciliation.md |

## Excluded

- Multi-agent orchestration.
- Third-party skill marketplaces.
- Whole-repository or whole-backlog cloud synchronization.
- Issue-tracker, Git provider, CI, deployment, or chat integrations.
- Visual workflow builders.
- Production signing, publishing, and distribution.
- Automatic irreversible or production actions.
- Persistent capability/keychain storage and background synchronization.
- A public HA2HA Build Right profile.

## Manual Before Automated

- The user explicitly selects a repository.
- The user chooses which Markdown file to edit.
- The user starts each helper or runtime action.
- The user explicitly publishes or joins shared mode and reconnects capability
  access after restart.
- The user explicitly repairs partial remote sync; repair never reruns Codex.
- Native packaging prerequisites are installed outside the application.

## Readiness Notes

The signed local MVP has frontend, Rust, debug-build, real helper, real Codex,
bounded-task, persistence, and confirmed-loop evidence. Production release,
provider portability, and customer-value claims remain outside that proof.
Sprint 2 is completed technical integration, not customer validation. Sprint 3
is the founder-facing productization phase; it must compose the existing local
and optional shared contracts rather than replace or duplicate them.

## Validation Plan

- A founder uses the app against a real Build Right project.
- The application round-trips a nontrivial Markdown file without content loss.
- The Tauri file boundary rejects lexical and symlink traversal outside the selected root.
- A real helper and one bounded agent task are observed end to end.
- Local solo mode remains green without network access.
- Viewer denial, Collaborator claim, conflict-before-Codex, synchronized
  evidence/handoff, repair-after-partial-write, restart without secret
  persistence, and edit-capability revocation are observed end to end.

## Learning Objective

Determine whether a repo-native control bench makes Build Right state and agent
work materially easier to understand without weakening authority or safety.
