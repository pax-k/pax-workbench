# Founder-Facing Workflow Contract

Status: accepted target for active Sprint 3
Owner: founder + AI
Last updated: 2026-07-23
Requirement basis: `docs/evidence/founder-workflow-ui-ux-audit.md`

## Product Thesis

Build Right Studio is a local engineering workbench for a founder or engineer
who wants an evidence-backed AI coding loop without operating the lifecycle
from a terminal. The app should reveal repository truth, present one safe next
action, preview effects, supervise one bounded operation, and make the outcome
reviewable.

The repository remains the source of product, planning, task, and completion
truth. App preferences may remember presentation and recent project choices;
they may not become planning authority.

## Target Journey

```text
open or create repository
  -> inspect project and skill provenance
  -> answer only missing founder questions
  -> preview and create authority artifacts
  -> rerun preflight until ready
  -> describe one feature or objective
  -> preview sprint/task planning changes
  -> confirm planning write
  -> resolver selects one exact task
  -> review task, gates, and expected effects
  -> confirm one bounded implementation
  -> review changes, verification, evidence, risks, and tracker state
  -> hand off or create an explicit local commit
  -> continue with a fresh confirmation, stop at a gate, or finish
```

## Workflow Phases

| Phase | User Question | Primary Action | Completion Signal |
| --- | --- | --- | --- |
| Project | Which repository am I working in? | Open or create project | Repository identity and inventory verified |
| Discover | What product truth or readiness evidence is missing? | Complete project setup | Preflight returns ready or an explicit gate |
| Plan | What bounded work should happen next? | Plan feature | One dependency-safe task is resolver-visible |
| Review task | What exactly will the agent do? | Review and confirm task | One-use confirmation bound to current source |
| Build | What is happening now? | Run one task | Runtime ends and repository refresh completes |
| Review result | What changed and is it proved? | Accept, revise, hand off, or commit locally | Human decision plus durable evidence receipt |
| Continue/finish | What is the next safe state? | Review next iteration or finish | Fresh confirmation, explicit gate, or goal complete |

`Principles` is a cross-cutting reference, not a separate destination the user
must complete.

## Workspace States

The primary canvas must render one dominant state:

- `noProject`
- `projectNeedsSetup`
- `preflightRequired`
- `preflightNeedsInput`
- `planningReady`
- `taskReadyForReview`
- `awaitingConfirmation`
- `operationRunning`
- `resultNeedsReview`
- `continueAvailable`
- `resumable`
- `repairRequired`
- `blocked`
- `goalComplete`

Each state must define:

- one primary user action;
- permitted secondary actions;
- whether mutation is possible;
- required confirmation;
- evidence displayed;
- typed failure and repair transitions;
- whether repository, goal receipt, or app preference supplies the projection.

Invalid combinations such as `goalComplete` with an unresolved primary ribbon
step, or `resumable` with automatic execution, must be prevented or explicitly
rendered as invalid state.

## Authority And Storage

### Repository authority

- `AGENTS.md` and nested instructions.
- `docs/source-index.md` and authority documents.
- sprint and task Markdown.
- Git state and content hashes.
- deterministic helper results.
- task evidence and verification.

### Durable orchestration evidence

The existing goal receipt may identify the verified checkpoint and
reconstruction inputs. It cannot select, plan, complete, or alter repository
truth.

### UI preferences

Recent project paths, pane sizes, collapsed sections, evidence filters, and the
last non-authoritative view may use bounded local app preferences. They must not
store task status, provider authority, secrets, capability URLs, or an
alternative backlog.

## Effect Classes

Every action must be visibly classified:

| Class | Examples | Required UX |
| --- | --- | --- |
| Inspect | inventory, preflight helper, resolver, diff read | Runs explicitly; no mutation confirmation needed |
| Plan mutation | create authority docs, sprint, task, decision entry | Preview exact paths and operation; explicit confirmation; post-write validation |
| Build mutation | bounded Codex execution | Review selected task/effects; one-use confirmation; repository verification |
| Git mutation | stage selected paths, local commit | Preview selected paths/message; explicit confirmation; never push automatically |
| External/shared mutation | Sprint 2 HA2HA/MDSync effects | Sanitized remote preview and separate confirmation contract |
| Developer diagnostic | fixtures, generic runtime probe, raw payload | Hidden behind Developer Tools; never presented as fulfillment |

Destructive rollback, reset, checkout-overwrite, deletion, remote push,
production publishing, and unattended execution are outside this contract.

## Guided Project Bootstrap

A new or incomplete repository must support:

1. Open an existing directory or create/select an empty directory.
2. Inspect project type, Build Right skill state, Git state, and missing
   authority artifacts.
3. Ask the smallest founder-question batch that changes product truth.
4. Label founder claims, repository evidence, and prototype assumptions.
5. Generate a preview of exact Markdown artifacts and paths.
6. Allow the user to inspect/edit the proposed content before mutation.
7. Apply only the confirmed plan through a safe create boundary.
8. Rerun inventory/preflight and show the next unresolved gate.
9. Stop when founder or external evidence remains missing.
10. Reach a state with Sprint 0 and one bounded AI-owned task when evidence
    supports it.

Existing files must never be overwritten without an exact version-bound update
preview. Partial application must report committed paths and a repair action;
it must not claim atomic success.

## Functional Planning

The Plan workflow must:

- accept one feature or objective;
- run the planning helper and show its typed decision;
- ask only founder questions that change scope or acceptance;
- preview backlog, sprint, task, decision, conflict, or evidence changes;
- explicitly exclude product implementation files;
- write only after confirmation;
- rerun planning and execution resolvers;
- finish with one exact ready-task handoff or an explicit gate.

Planning may use a controlled agent adapter, but provider output is not
authority. Repository diffs and helper readback establish the result.

## Task Execution And Result Review

The existing bounded controller remains the implementation authority. After a
run, the default review receipt must show:

1. Outcome and stop reason.
2. Changed files and repository diff.
3. Commands and checks with exit results.
4. Acceptance criteria linked to evidence.
5. Task and sprint status changes.
6. Review notes, unresolved risks, and follow-ups.
7. Next resolver decision.
8. Expandable provider/raw diagnostic detail.

The review surface may offer:

- accept the result as reviewed;
- create or prepare a revision task;
- copy/export a handoff;
- preview and confirm a local Git commit over selected task-related paths;
- review the next iteration.

It must not silently revert changes, stage unrelated files, create a commit,
push, or start another agent.

## Failure And Repair Contract

Repair copy and actions must derive from typed evidence. Minimum failure classes:

- project/path/source mismatch;
- stale Markdown or Git fingerprint;
- missing/invalid skill provenance;
- helper runtime unavailable, timeout, cancellation, malformed output, or
  unsupported platform;
- agent authentication, connectivity, permission, timeout, provider failure,
  malformed output, cancellation, or cleanup failure;
- repository verification failure;
- open founder/conflict/external gate;
- shared collaboration denial/conflict/repair debt;
- unknown failure.

Local Network guidance may appear only when the typed evidence supports it or
when presented as one explicitly labeled diagnostic hypothesis. Unknown
failures must preserve raw diagnostics without inventing a cause.

## Information Architecture

```text
┌ Project / goal / state / primary action ─────────────────────────────┐
├ Project navigator ┬ Current phase and one dominant work surface ┬ Run receipt ┤
│ authority         │ setup / plan / task / review / complete     │ summary     │
│ sprints + tasks   │ document detail opens on demand              │ filters/raw │
│ search + filters  │                                              │             │
├───────────────────┴──────────────────────────────────────────────┴─────────────┤
│ Goal evidence spine: Project → Discover → Plan → Build → Review → Continue     │
└────────────────────────────────────────────────────────────────────────────────┘
```

The evidence spine is the signature element. It should encode verified workflow
state, not decorate the shell or mirror only the selected document.

Developer Tools contains generic runtime probes, deterministic fixtures,
simulations, adapter metadata, and raw JSONL. These remain accessible but do not
compete with the product journey.

## Visual Direction

Preserve the current industrial blueprint character with disciplined
progressive disclosure.

### Color tokens

- Graphite shell: `#17191C`
- Instrument panel: `#202328`
- Paper canvas: `#F5F6F6`
- Blueprint action: `#356AE6`
- Verified state: `#2E9F8D`
- Gate/warning: `#C78A28`
- Fault: `#C95858`

### Type roles

- Display/state: Avenir Next Demi Bold or platform-equivalent humanist sans.
- Body/control: Avenir Next or platform-equivalent UI sans.
- Evidence/path/data: SF Mono or platform-equivalent mono.

Metadata must normally render at 11px or larger; body content at 13px or
larger. Smaller type requires a documented exceptional purpose and must not
carry essential state.

### Layout and motion

- Desktop-first, useful at 900x700 and common split-screen widths.
- Resizable or collapsible side panes.
- No essential action may depend on hover.
- Motion is limited to one purposeful state transition or progress signal,
  respects reduced motion, and never substitutes for status text.

## Accessibility Contract

- Complete primary workflows with keyboard only.
- Logical focus order and visible focus for every control.
- Semantic headings, landmarks, tabs, dialogs, status, and alert behavior.
- State, provenance, and severity cannot rely on color alone.
- Useful zoom/reflow and high-contrast behavior.
- Screen-reader names describe user actions, not implementation mechanics.
- Automated accessibility checks plus manual keyboard/screen-reader smoke.
- Responsive and visual-regression scenarios for critical states.

## Acceptance For Sprint 3

Sprint 3 succeeds only when a founder can, in the signed native app:

1. Start from an empty disposable repository.
2. Complete guided project setup without a terminal.
3. Produce and review canonical authority files and a first ready task.
4. Plan one additional bounded feature/task through an explicit planning write.
5. Execute one task with the existing safety guarantees.
6. Understand changed files, checks, criteria, evidence, risks, and next state.
7. Restart and resume without automatic execution or manual state reconstruction.
8. Distinguish product actions from developer diagnostics.
9. Complete the critical path with keyboard access and useful text sizing.
10. Record friction, real/manual/simulated/unproved boundaries, and follow-up
    work without claiming customer validation from one founder session.

## Sequencing

This contract is queued for Sprint 3. Sprint 2 is terminal and remains part of
the authoritative regression baseline. Post-HA2HA reconciliation is complete:
Task 031 runs first, then Task 022 composes existing local/shared contracts
before focused extraction and product feature work. Only the strict
resolver-selected task may execute.
