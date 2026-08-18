# Task 026 Goal-Centered Shell Evidence

Date: 2026-07-23
Status: complete
Source under test: repo-local path and Apple-development-signed native app

## Run Label

`task-026-signed-restart-reinspect`

## Agent / Tool Surface

- Codex desktop implementation and verification
- Build Right execution and engineering-principles guidance
- Native macOS app operated through accessibility-backed computer control
- No model router, subagent delegation, runtime provider, or collaboration
  provider

## Targets

- Main repository: `/Users/pax/Documents/Repos/pax-workbench`
- Completed-checkpoint fixture:
  `/private/tmp/pax-workbench-task019-live-20260723-a/repo`
- Missing-repository mismatch fixture:
  `/private/tmp/build-right-task025-final-xWrJ1h`
- Signed bundle:
  `src-tauri/target/debug/bundle/macos/Build Right Studio.app`

## Commands And Checks

- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `git diff --check`
- `bun run tauri:build:signed`
- `codesign --verify --deep --strict --verbose=2 <app>`
- Signed-app open, recovery, quit/restart, recent-project reopen, and
  repository/recovery reinspection

## Artifacts And Result

- Recent-project persistence retained only absolute root, last-opened time,
  selected operating-skill preference, and editor view.
- Reopen invoked native repository inspection and durable recovery again; the
  signed UI reported: `Reopened and re-inspected ... No helper or Codex process
  started.`
- The completed fixture restored the repository-affirmed `completed` checkpoint,
  Task 990, event cursor 34, the goal objective, and a `Complete` evidence spine.
- Opening the non-matching fixture rendered `Project path missing` rather than
  projecting the persisted task as current repository authority.
- The resolver-selected task, status card, primary action, run label, and footer
  evidence spine came from repository/goal/controller projections. Raw Markdown
  selection remained an advanced editor action and could not replace them.
- Local solo, Viewer, Collaborator, disconnected, reconciled, conflict,
  sync-pending, and repair-required context is composed into the same shell
  projection without persisting remote state.

## Live Finding Repaired

The first signed restart correctly restored `Complete` and started no process,
but the lower controller diagnostic still left `Prepare one bounded task`
enabled. That contradicted the closed `goalComplete` transition. The control is
now disabled for empty, complete, missing-path, and stale-repository shell
states, with a regression test.

## Security And Authority Boundary

- The preference parser rejects unknown keys, relative/control-character roots,
  malformed view/skill values, oversized input, and entries containing task,
  goal, capability, remote-content, or provider-payload fields.
- A preference can request reinspection only. It cannot supply task status,
  selected task, goal state, confirmation, collaboration session, or repair
  authority.
- Resolver/controller/recovery evidence has explicit precedence over app
  preference and editor state.
- Restart and reopen do not invoke a helper, preview controller execution,
  consume confirmation, run Codex, publish, commit, push, or deploy.

## Evidence Classification

- Proved: strict preference allowlist, reinspection path, state precedence,
  editor non-authority, named local/shared projections, frontend/native
  regressions, signed bundle and signature.
- Manual: signed native open/restart/recent-reopen trial operated by an AI
  tester against existing truthful fixtures.
- Simulated: exhaustive missing, stale, blocked, resumable, review-required,
  Viewer, conflict, sync-pending, and repair-required projection variants.
- Unproved: unaided founder usability, notarized distribution, and production
  release.

## Follow-Up

Task 027 is ready to move generic runtime probes and fixtures behind Developer
Tools while keeping product actions dominant.
