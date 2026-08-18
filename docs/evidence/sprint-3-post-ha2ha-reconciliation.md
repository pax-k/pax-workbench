# Sprint 3 Post-HA2HA Reconciliation

Status: complete planning evidence
Owner: AI
Date: 2026-07-23
Confidence: high for repository facts and task-contract changes; medium for
founder usability outcomes pending Task 032

## Question

How must Sprint 3 change now that HA2HA/MDSync integration was implemented
before the founder-facing productization work that was originally intended to
shape the same frontend and native seams?

## Evidence Basis

- Terminal Sprint 2 tasks and signed/live evidence in `tasks/issues/013-019`.
- Current source and tests under `src/**` and `src-tauri/src/**`.
- `docs/founder-facing-workflow.md` and the original founder workflow/UI audit.
- Current strict resolver and release-gate state.

No public research is required. This is a repository/current-implementation
reconciliation, not a market or protocol-feasibility question.

## Current Implementation Facts

| Surface | Post-Sprint-2 fact | Planning consequence |
| --- | --- | --- |
| Frontend ownership | `src/App.tsx` is 1,290 lines and `src/components/CollaborationPanel.tsx` is 1,084 lines | Task 020 must extract both core and collaboration projections; extracting only `App.tsx` would preserve a second product-state monolith |
| Native ownership | `src-tauri/src/lib.rs` is 17,250 lines | Task 021 remains necessary but must not re-extract collaboration code already separated into dedicated modules |
| Existing collaboration modules | `collaboration.rs`, `mdsync_transport.rs`, and `ha2ha_envelope.rs` already own typed policy, transport, and envelope behavior | Reuse these as stable boundaries and remove any Sprint 3 plan that invents parallel shared state, failures, versions, or repair semantics |
| Workflow contracts | Local goal/controller and shared collaboration types exist, but the primary shell still derives workflow from selected Markdown and display-oriented component state | Task 022 must move before extraction and define one product projection that composes existing local/shared contracts |
| Artifact creation | Native save supports stale-safe updates to existing Markdown only | Tasks 023-025 remain required |
| Goal recovery | Durable local/shared recovery exists and never auto-starts, but repository reselection and file selection still dominate the shell | Task 026 should productize existing recovery rather than create another persistence model |
| Action hierarchy | Generic runtime, simulated checkpoint, helper controls, and collaboration mechanics remain prominent | Task 027 remains required; collaboration access/claim/repair are product effects, while raw session/provider diagnostics are secondary |
| Result evidence | Local controller and remote reconciliation return rich evidence, but no unified outcome receipt exists | Task 028 must combine local repository and optional shared evidence |
| Git handoff | The target journey promises an optional explicit local commit, but no Sprint 3 task owns a stage/commit preview boundary | Add Task 028A; keep push, reset, checkout-overwrite, and automatic staging excluded |
| Layout | The shell still has a 1080px minimum width; shared collaboration adds another dense overlay | Task 029 must integrate optional shared state without making it dominate local solo mode |
| Accessibility | Existing semantics and reduced-motion support are partial; many essential labels remain 7-9px and collaboration states lack complete keyboard/VoiceOver proof | Task 030 must include local and shared critical paths |
| Authority drift | Task 019 closeout found and repaired stale blueprint, release, and Sprint 3 sequencing copy; `bun run check` still has no authority-drift command | Task 031 should execute first, not last |
| Product proof | Sprint 2 proves technical collaboration, not founder usability | Task 032 remains founder-owned and must include a bounded optional-shared usability checkpoint without making the local journey network-dependent |

## Revised Task Decisions

| Task | Decision | Required adaptation |
| --- | --- | --- |
| 031 | move first, ready | Add structural authority drift enforcement before Sprint 3 reshapes more code/docs |
| 022 | move before extraction, modify | Compose existing goal, controller, and collaboration contracts into one typed product workflow/effect/repair projection |
| 020 | retain, broaden | Extract project/goal/workflow/collaboration projections from both frontend monoliths |
| 021 | retain, narrow | Extract repository and controller ownership needed by upcoming features; preserve existing collaboration modules and stable helper/runtime mechanics |
| 023 | retain, adapt | Reuse operation/preview patterns; block overlap with active runtime/collaboration effects; never publish planning artifacts remotely |
| 024 | retain, adapt | Keep shared mode unavailable until local authority exists; bootstrap remains local and founder-supplied |
| 025 | retain, adapt | Add a bounded planning-proposal path; applying plans never mirrors backlog/tasks into HA2HA |
| 026 | retain, broaden | Make local/shared goal and repair state first-class while persisting only non-authoritative preferences |
| 027 | retain, adapt | Keep collaboration connect/claim/repair as product actions; move raw coordinates, provider events, fixtures, and probes to diagnostics |
| 028 | retain, broaden | Produce one receipt for local results plus optional remote claim/evidence/handoff/reconciliation |
| 028A | add | Implement explicit allowlisted stage/local-commit preview and confirmation; no push or destructive Git action |
| 029 | retain, broaden | Integrate collaboration status through progressive disclosure and preserve local-solo simplicity |
| 030 | retain, broaden | Cover collaboration connection, access, conflict, repair, and secure-input states in accessibility/visual gates |
| 032 | retain, adapt | Founder trial covers the local product loop plus a separate optional-shared checkpoint and never treats technical dogfood as customer validation |

No existing Sprint 3 task should be deleted. The obsolete material is the old
dependency order, pre-HA2HA baselines, duplicate-contract risk, broad native
rewrite scope, blanket failure guidance, and the unowned local-commit promise.

## Revised Execution Order

```text
031 authority drift guard
  -> 022 unified local/shared product contracts
  -> 020 frontend projection extraction
  -> 021 focused native extraction
  -> 023 safe artifact plan/apply
  -> 024 guided Discover/bootstrap
  -> 025 functional Plan
  -> 026 goal-centered local/shared shell
  -> 027 product actions vs diagnostics
  -> 028 unified result receipt
  -> 028A safe local Git handoff/commit
  -> 029 information architecture/responsive layout
  -> 030 accessibility/visual enforcement
  -> 032 founder-led cohesive trial
```

## Guarantees

- HA2HA/MDSync remains optional and is not removed or rewritten.
- Repository Markdown, Git, helpers, and verified evidence remain authority.
- Local and remote versions remain separate.
- Planning/bootstrapping never mirrors a backlog into HA2HA.
- Capability material remains native-memory-only and is excluded from new
  preferences, projections, receipts, diagnostics, and Git messages.
- No extraction is justified by file size alone; each boundary must serve an
  upcoming workflow responsibility and preserve compatibility.
- Every UI task carries keyboard, semantics, readable-text, and local/shared
  regression criteria before Task 030 completes the enforcement suite.

## Result

Sprint 3 is safe to begin with Task 031. The plan is larger by one bounded task
(028A), but narrower in native refactoring and stricter about reusing the
collaboration contracts already proved in Sprint 2.
