# Founder Workflow And UI/UX Audit

Status: complete planning evidence
Owner: AI
Date: 2026-07-22
Confidence: high for repository and implementation facts; medium for usability
outcomes pending founder observation

## Audit Question

What works, what does not work as a coherent product workflow, what is genuinely
missing, and what must improve before Build Right Studio can serve as a daily
founder-facing workbench rather than only a technically proved execution shell?

## Evidence Basis

- `docs/evidence/manual-trials.md` and the signed Sprint 1 artifacts.
- `tasks/issues/003-*.md` through `tasks/issues/012-*.md`.
- `src/App.tsx`, `src/styles.css`, `src/lib/**`, and `src/types.ts`.
- `src-tauri/src/lib.rs` and its native test suite.
- Current Build Right preflight, feature-planning, execution, and engineering
  contracts under `.agents/skills/`.
- Terminal Sprint 2 collaboration authority, Tasks 013-019, and signed/live
  Task 019 evidence.
- `docs/evidence/sprint-3-post-ha2ha-reconciliation.md`.
- Read-only independent planning review on 2026-07-22.

No customer or founder usability session was observed for this audit. Product
quality conclusions are therefore prototype findings, not customer evidence.

## Executive Assessment

The safety-critical local execution kernel works. Repository authority, safe
Markdown round-trip, skill provenance, deterministic helpers, Codex JSONL,
bounded one-task execution, repository verification, checkpoint recovery,
fresh confirmation, and terminal resolver behavior have real evidence.
Optional HA2HA/MDSync publish/join, Viewer denial, Collaborator claim, remote
conflict, evidence/handoff repair, restart redaction, and revocation also have
technical evidence.

The surrounding product workflow remains incomplete. A user cannot yet move
from an empty repository through product discovery, artifact creation, feature
planning, task authoring, execution, change review, and iteration entirely in
the app. Discover and Plan are primarily contract/helper viewers. The shell is
more file-centered than goal-centered. Engineering diagnostics compete with
product actions, and outcome evidence is reconstructable but not easy to
consume.

The product should preserve its industrial blueprint identity and proved safety
model while reorganizing the experience around one current phase, one clear
next action, and one reviewable evidence receipt.

## What Works

| Surface | Proved Behavior | Evidence Strength |
| --- | --- | --- |
| Repository authority | Markdown, Git, helpers, and task evidence determine state; provider output cannot self-promote authority | strong |
| Project session | Native directory selection, inventory, Markdown read/write, refresh, stale-version detection, and unsaved-draft guards | strong |
| Skill setup | Exact source, version, hashes, argv, and changed-path boundary are previewed before explicit confirmation | strong |
| Helpers | Allowlisted preflight, planning, continuation, and execution helpers return bounded typed decisions | strong |
| Runtime | Real Codex JSONL events, cancellation, timeout/output bounds, and descendant cleanup | strong |
| Bounded controller | One resolver-selected AI task, one-use confirmation, one invocation, repository verification, and stop gates | strong |
| Persistence | Verified checkpoints recover without automatic execution and detect repository/Git/task drift | strong |
| Optional shared collaboration | Native-only capability sessions, one-envelope publish/join, Viewer denial, version-bound claim, evidence/handoff repair, restart, and revocation | strong technical evidence |
| Native boundary | Path traversal and symlink rejection, atomic existing-file writes, typed failures, concurrency control, and extensive tests | strong |
| Visual identity | Graphite/paper/blueprint palette and evidence-instrument language are distinctive and appropriate | prototype evidence |

## What Does Not Yet Work As A Product Workflow

### New-project bootstrap

- A blank repository can be inventoried but cannot be completed from inside the
  app.
- Save requires an already selected, already existing Markdown file.
- There is no create-file, create-artifact-plan, founder-question, Sprint 0, or
  first-task workflow.
- A founder still needs Codex/CLI outside the app to run the mutating Build
  Right preflight workflow.

### Discover and Plan

- Discover can report missing artifacts but cannot resolve the sequence in-app.
- Plan can expose its contract/helper but cannot turn feature intent into
  previewed repository planning changes.
- The UI presents all operating contracts at once instead of taking the user
  through the active phase.

### Goal continuity

- Restart recovery can require manual repository reselection.
- The central surface may show `Select a Markdown file` while a verified goal
  receipt already knows the current task and continuation state.
- The execution ribbon derives too much from the selected document, so it can
  show unknown or unresolved steps after repository-affirmed completion.
- There is no recent-project landing page or explicit `Resume verified goal`
  entry point.

### Product action hierarchy

- Generic live runtime probing appears beside the real bounded-task action.
- Dry fixtures, simulated checkpoints, and raw adapter surfaces remain prominent
  during real native operation.
- Inspect-only, mutating, diagnostic, and destructive-risk actions do not have a
  sufficiently clear hierarchy.

### Failure repair

- Any failed real bounded runtime may receive Local Network repair guidance even
  when the typed failure does not prove that cause.
- Failure copy is not consistently derived from boundary-specific error classes.
- Repair actions are not gathered into a single evidence-backed next-action model.

### Post-run review

- The app reports outcome, resolver, and refresh facts but lacks one coherent
  review receipt.
- Changed files, Git diff, verification commands, acceptance criteria, task
  evidence, tracker changes, risks, and raw provider detail are not presented as
  a linked review sequence.
- There is no safe, explicit Git handoff/commit preview; destructive discard and
  remote push are intentionally absent.

## UI/UX Findings

### Information architecture

The three-pane workbench is the right high-level metaphor, but too many layers
are permanently visible: project truth, task list, operating modes, editor,
skill contracts, runtime adapter, bounded controller, event stream, notice bar,
and execution ribbon. Internal architecture is exposed before the user knows
the next action.

The primary surface should answer, in order:

1. What state is this project in?
2. What needs attention?
3. What is the next safe action?
4. What will change?
5. What changed and why did the workflow continue or stop?

### Readability and responsive behavior

- Metadata commonly renders at 7-9px.
- The application enforces a 1080px minimum body width.
- Only one constrained desktop breakpoint exists.
- Panes are neither resizable nor collapsible through implemented controls.
- Increased text size, zoom, split-screen, and small-window behavior lack proof.

### Navigation

Missing navigation affordances include:

- recent projects and resume-last-project;
- search or command palette;
- sprint grouping and collapse;
- status filters;
- resolver-selected task auto-navigation;
- document back/forward history;
- useful breadcrumbs and full-name inspection for truncated rows.

### Empty, blocked, resumable, and complete states

The current generic editor empty state does not guide the user toward project
setup, preflight repair, planning, or task execution. Resumable and terminal
goal states appear inside the document canvas instead of becoming deliberate
workspace states. `goalComplete` needs a final summary and next-choice surface.

### Evidence inspector

The timeline is technically rich but operationally noisy. Raw command lifecycle
events, provider messages, duplicated start/completion detail, usage events, and
file changes compete equally. The default view needs outcome-first summaries,
filters, collapsible raw detail, and direct links from criteria to evidence.

### Accessibility

ARIA labels, tab roles, focus outlines, and reduced-motion CSS exist. Missing
proof includes keyboard-only completion paths, screen-reader semantics,
high-contrast behavior, zoom/reflow, responsive window scenarios, automated
accessibility checks, and visual-regression coverage.

## Engineering Findings

- `src/App.tsx` is 1,290 lines and owns project session, document editing,
  setup, helpers, runtime, controller, recovery, evidence projection, and most
  rendering. `CollaborationPanel.tsx` adds a second 1,084-line state/effect/UI
  root.
- `src-tauri/src/lib.rs` is 17,250 lines and owns project, filesystem, Git,
  helper, runtime, persistence, bounded-control, and most native tests, while
  collaboration policy, MDSync transport, and HA2HA envelopes now have separate
  modules.
- The code is strongly tested, but adding product workflows directly to these
  roots would create more unrelated reasons to change.
- Modularization must be behavior-preserving and responsibility-led; a broad
  rewrite is not justified.
- README and authority state have drifted before. Repeatable status/command
  checks should replace reviewer memory.
- The repository currently has no commit. Planning must not invent branch or
  Git-history proof.

## Product And Scope Boundaries

This improvement program must preserve:

- repository Markdown and Git as authority;
- explicit user actions for every helper, agent, Git, or external effect;
- current path, symlink, stale-write, timeout, cancellation, and cleanup gates;
- raw Markdown access and preservation of unknown content;
- provider output as evidence only;
- local solo operation with no network dependency;
- current Sprint 2 HA2HA/MDSync work and its separate authority model.

It must not add production distribution, whole-backlog cloud sync, marketplace,
issue-tracker integration, multi-agent orchestration, provider portability
claims, background execution, persistent capabilities, destructive rollback,
or remote Git push.

## Improvement Priority

### P0: complete the founder-facing lifecycle

1. Safe artifact creation and guided project bootstrap.
2. Functional Discover and Plan workflows.
3. Goal-centered navigation, project continuity, and terminal states.
4. Clear separation between product actions and developer diagnostics.
5. Typed failure repair based on real evidence.
6. Outcome-first post-run diff and evidence review.

### P1: make the workbench understandable and comfortable

1. Goal/phase-driven information architecture.
2. Search, sprint grouping, status filtering, breadcrumbs, and history.
3. Resizable/collapsible panes and progressive disclosure.
4. Readable typography and useful small-window/zoom behavior.
5. Keyboard, semantic, contrast, reduced-motion, and visual-regression proof.

### P2: preserve maintainability and truthful authority

1. Docs/source-index/blueprint/sprint/release-gate drift enforcement.
2. Unified local/shared workflow contracts followed by responsibility-led
   frontend and native module extraction.
3. A cohesive signed-native founder usability trial with explicit real,
   manual, simulated, and unproved boundaries.

## Sequencing Decision

Sprint 2 owned Tasks 013-019 for optional HA2HA/MDSync collaboration and is
terminal. The post-HA2HA reconciliation now governs Sprint 3: Task 031 installs
authority-drift enforcement first, Task 022 defines the unified local/shared
product contract, focused extraction follows, and new Task 028A owns the
previously unassigned explicit local Git handoff.

## Residual Risks

- Completed HA2HA work added a second large frontend state surface before the
  local founder journey was validated; Sprint 3 must integrate it contextually
  without making local solo mode collaboration-centered.
- A technically complete workflow may still be confusing in real use.
- Bootstrap and planning introduce new workspace-write effects and need the same
  preview, confirmation, verification, and repair discipline as execution.
- Accessibility added only as polish would arrive too late; every UI task must
  preserve semantic and keyboard behavior before the final enforcement task.

## Conclusion

Proceed with a planned founder-facing Sprint 3. The safe kernel should remain
intact; the work is to make the proven machinery form one understandable,
complete, evidence-backed product loop.
