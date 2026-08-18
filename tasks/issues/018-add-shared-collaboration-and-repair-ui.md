# 018: Add Shared Collaboration And Repair UI

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed plus prototype-assumption
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; tasks/issues/014-017
Reversibility: easy
Learning objective: determine whether users can distinguish local authority, remote collaboration, access, conflict, and repair without exposing capability material
Source under test: repo-local path

## Goal

Add a focused collaboration surface for publishing/joining/disconnecting,
inspecting the current HA2HA envelope, previewing shared execution effects, and
repairing partial sync while keeping the existing workbench workflow legible.

## Non-Goals

- Rebuild the application as an MDSync dashboard.
- Render the entire remote workspace or comment/history product.
- Show or copy capability tokens after connection.
- Add autonomous multi-agent controls.
- Redesign unrelated Sprint 1 editing, skill, helper, or runtime surfaces.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- tasks/issues/014-017
- docs/evidence/manual-trials.md

## Acceptance Criteria

- [x] Local solo versus Shared HA2HA mode is explicit and local remains default.
- [x] Publish/join accepts a URL only in the native boundary and returns a
      sanitized workspace/access/actor summary.
- [x] Viewer, Collaborator, disconnected, reconciled, conflict, stale,
      sync-pending, and repair-required states are visually and semantically distinct.
- [x] Shared execution preview shows local task binding, remote task/version,
      expected mutation, and no secret/provider payload.
- [x] Disconnect/project switch clears the native session and updates UI state.
- [x] Repair UI explains that local work may already be complete, previews only
      missing remote effects, requires explicit action, and never suggests rerunning Codex.
- [x] Network/session activity appears in the run inspector with sanitized
      adapter provenance and exact real/simulated labeling.
- [x] Component tests cover read-only denial, conflict before execution,
      post-commit repair, restart without capability, and unchanged solo flow.
- [x] Native visual inspection covers narrow and standard window widths and
      confirms no capability-bearing text appears in the DOM or screenshots.

## Baseline Evidence

`App.tsx` owns the complete current experience and has no collaboration state.
The run inspector already distinguishes real, adapter, manual, and simulated
events, providing a compatible evidence vocabulary.

## Solution-Fit Rationale

- Requirement served: make shared execution understandable and controllable.
- Constraints honored: no dashboard duplication and no secret rendering.
- Guarantees preserved: explicit confirmation, local authority, raw Markdown fallback.
- Cost accepted: one focused collaboration panel/state projection.
- Deferred capability: full MDSync activity/comments/admin UX.

## Verification

- Focused component/state tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Native visual inspection with Viewer, Collaborator, conflict, and repair fixtures.
- DOM/screenshot capability scan.

## Frontend Design Plan

- Subject / audience / job: a founder-engineer coordinating one locally
  authoritative task through an optional remote envelope; the panel's one job
  is to show whether shared work is safe to inspect, execute, or repair.
- Tokens: reuse Graphite `#17191c`, Slate `#202328`, Blueprint `#356ae6`,
  Verified `#2e9f8d`, Gate `#c78a28`, and Fault `#c95858`. Reuse Avenir Next
  for display/body roles and SFMono for authority labels, versions, and paths.
- Layout: one focused overlay opened by an always-visible Local solo / Shared
  control. Its compact authority rail is
  `Local truth -> Access -> Binding -> Sync/repair`; at narrow width the same
  causal sequence stacks without hiding labels or controls.
- Signature: Local truth is a fixed visual anchor and remote coordination
  branches from it only after access is established. Boldness is spent on that
  rail/state transition; forms and actions remain quiet.
- Generic-dashboard critique and revision: independent KPI/status cards would
  make authority look like unrelated metrics. The chosen continuous rail
  encodes dependency and fail-closed progression instead, with no decorative
  statistics, new gradient language, or unrelated redesign.
- Accessibility floor: semantic region/status labels, visible keyboard focus,
  Escape and explicit close behavior, readable contrast, native-minimum-width
  layout, and reduced-motion compliance.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `bun .agents/skills/build-right-execution/scripts/continue-check.ts --cwd /Users/pax/Documents/Repos/pax-workbench --format markdown --strict` | pass | Resolver selected only ready Task 018 with high confidence and no blocking gate or external follow-up. |
| 2026-07-23 | Frontend-design two-pass plan and live UI/native-boundary baseline | pass | Reused the existing Studio system, replaced a generic status-card tendency with one authority rail, and bounded implementation to sanitized native commands plus a focused component. |
| 2026-07-23 | `bunx vitest run src/components/CollaborationPanel.test.tsx --reporter=dot` | pass | 11 component cases cover unchanged solo flow, focus return, opaque URL handoff/DOM redaction, Viewer denial, typed publish/disconnect, pre-execution conflict, stale binding, post-commit repair without Codex, restart without capability, sync-pending, and project-switch clearing. |
| 2026-07-23 | `bun run check` | pass | TypeScript, 69 Vitest cases across 6 files, and the production Vite build passed. The App integration case records sanitized collaboration activity as real adapter evidence and rejects the opaque alias marker. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 197 Rust tests passed, including the Task 014-017 native authority, session, binding, execution, and repair contracts reused by this UI. Three existing dead-code warnings remain. |
| 2026-07-23 | `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | Native compilation and formatting checks passed with the same three existing dead-code warnings. |
| 2026-07-23 | `bun run tauri build --debug` | pass | Root-owned fresh packaged-path rebuild produced `src-tauri/target/debug/pax-workbench` (SHA-256 `1e938409d2684bcdb4aef2b85a697eb5de3c869733924d1c272377c3510bd86c`). |
| 2026-07-23 | Production `dist` scan for Task 018 opaque URL/token markers | pass | No `opaque-alias-018`, error alias, query capability, Authorization, or Bearer fixture marker was present in the built frontend. Component tests separately scan the rendered DOM. |
| 2026-07-23 | Packaged-native role and responsive visual inspection | pass | The real debug app exercised Local solo, Viewer, and Collaborator through a loopback discovery endpoint. macOS accessibility reported the standard app window as 1440x900 and the native-minimum window as 1080x760. The app-control service normalizes standard captures to 1229x768, so `output/native/task-018-collaborator-standard.jpeg` is a scaled capture of the verified 1440x900 window rather than a 1440x900 raster; `task-018-collaborator-narrow.jpeg` is an exact 1080x760 raster. Local-solo and Viewer standard artifacts are likewise normalized 1229x768 captures. `output/native/task-018-window-evidence.txt` records the exact app, binary, window/raster dimensions, artifact hashes, and live-versus-fixture scope. All captures show the sanitized authority rail; Viewer mutation controls are disabled and the one-shot handoff is absent from the accessibility tree after connection. SHA-256: `86f9ccd8a7ac266b4d9776c7ab4e0bbbfd382df28a1d59bfef871b9012a0eebe`, `b760eff99d1117c21fdeea561fe413972de1919205335a27fa81e0cdba548cc8`, `9cab96fe0ac8daee07b0763eaa8a1bd416fa0083363e586900601f61d444994e`, `defff704f93221691ef660c26ee861d3380d5cd5a869d9e0d0d9dfc3a02c0f`. |
| 2026-07-23 | Conflict/repair projection and capability scan | pass | Deterministic component fixtures render distinct conflict, stale, sync-pending, and repair-required states, including the missing-effect-only/no-Codex repair copy. Packaged-app accessibility snapshots contain sanitized workspace/access/actor data only; the pasted Viewer/Collaborator capability values disappear before the native command returns and appear in neither the DOM projection nor screenshots. No hosted request was made. |
| 2026-07-23 | Independent Sol/high review after `F018-01` repair | pass | Reviewer independently reran 8 capability-alias regressions, all 200 Rust tests, and all 69 frontend tests; verified repaired binary/artifact hashes, responsive evidence normalization, native/UI state clearing, exact provenance, accessibility, and local-versus-fixture evidence; approved closure with no open finding. |
| 2026-07-23 | Build Right `continue-check --strict` and `execution-check --mode stop-gates` after evidence capture | stop | Resolver still selects only Task 018; its recorded closure blockers correctly trigger the stop/ask gate, so Task 019 was not started. |
| 2026-07-23 | Independent Sol/high finding `F018-01` | fail then repaired | Review proved the native alias guard compared only whole-field digests, so a capability embedded inside an otherwise permitted actor could cross IPC. The repair remains inside Task 018 and does not expose the reviewed capability in errors, logs, or evidence. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml embedded_capability_alias_is_rejected_before_connect_metadata_is_exposed -- --nocapture` before repair | expected fail | The adversarial connect returned sanitized metadata containing an embedded alias, reproducing `F018-01` before implementation. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml capability_alias -- --nocapture` after repair | pass | 8 focused cases prove exact, capability-at-prefix, capability-at-suffix, and infix rejection across connect, metadata reread, caller input, remote result, and completion persistence. Failures are typed `capability_material_rejected`, secret-free, and occur before unsafe output or mutation transport. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` after `F018-01` repair | pass | 200 Rust tests passed. The shared native guard now performs a full bounded candidate scan without early match exit, preserves constant-time digest comparison for equal-length aliases, and zeroizes secret-derived scan buffers. |
| 2026-07-23 | `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` after repair | pass | Native compilation and formatting passed with the same three existing dead-code warnings. |
| 2026-07-23 | `bun run check` after repair | pass | TypeScript, 69 Vitest cases across 6 files, and the production Vite build remained green. |
| 2026-07-23 | `bun run tauri build --debug` after repair | pass | Fresh repaired native build produced `src-tauri/target/debug/pax-workbench` (SHA-256 `0a7d15a61e5eab8ecd16c3fb0b213b622eb2be915e4cd88e8e62e5350f786760`). |
| 2026-07-23 | Repaired-source packaged-native smoke | pass | The repaired binary was copied into the debug app bundle, relaunched, reopened on this repository, and inspected in Local solo mode with Task 018 selected. `output/native/task-018-security-repair-native-smoke.jpeg` is 1229x768, has SHA-256 `5c0044143d5a61d2fcdbf167519409261eb731bf5439f2a2e66cc7afe033530a`, and contains no F018-01 capability fixture marker. |
| 2026-07-23 | Repaired production `dist` embedded-secret marker scan | pass | The adversarial prefix/suffix aliases and the prior opaque URL/token fixture markers were absent from the built frontend. |

## Files Changed

- `src/components/CollaborationPanel.tsx` - added the focused local-authority,
  access, binding, sync, conflict, stale, and explicit repair surface.
- `src/components/CollaborationPanel.test.tsx` - added component, state,
  keyboard, redaction, denial, conflict, repair, restart, and project-switch
  coverage.
- `src/lib/collaboration.ts` - added the sanitized UI projection, surface-state
  classifier, repair vocabulary, and safe effect/hash labels.
- `src/lib/bridge.ts` - added typed wrappers for the existing Task 014-017
  native collaboration commands.
- `src/lib/bridge.test.ts` - verified exact native command names, argument
  shapes, and transient URL handoff.
- `src/types.ts` - added the typed sanitized collaboration command contracts.
- `src/App.tsx` - integrated the panel, mutation exclusion, project reset, and
  sanitized real/simulated run-inspector projection.
- `src/App.test.tsx` - added collaboration/run-inspector provenance and opaque
  alias regression coverage while preserving existing flows.
- `src/styles.css` - added the token-driven authority rail, semantic state,
  focus, standard/narrow layout, and quiet control treatment.
- `src-tauri/src/mdsync_transport.rs` - repaired `F018-01` with a shared
  fail-closed containment guard and adversarial connect, metadata reread,
  caller-input, result, and completion-persistence regressions.
- `tasks/issues/018-add-shared-collaboration-and-repair-ui.md` - recorded the
  design decision, implementation, validation, and residual acceptance gates.

## Verification Summary

- Automated frontend, Rust, formatting, production-bundle, native debug-build,
  and opaque-marker gates pass.
- Real evidence: typed native command integration, the local production build,
  run-inspector adapter labeling, and adversarial native rejection before
  embedded capability aliases can reach IPC. No hosted collaboration was
  exercised.
- Simulated evidence: deterministic collaboration fixtures cover role,
  conflict, stale, repair, restart, and sync-pending presentation.
- Closure passed after the mandatory independent Sol/high rereview. Packaged-
  native Local solo, Viewer, Collaborator, and responsive evidence is complete;
  deterministic UI fixtures cover conflict and repair.

## Learning Notes

- Proved: a single local-authority rail can keep solo default, remote access,
  local/remote binding, and repair debt legible without retaining or rendering
  capability material. The repaired native boundary rejects exact and embedded
  capability aliases consistently across every enumerated sanitized field.
- Simulated: Viewer/Collaborator and failure/recovery fixtures prove UI
  projection and explicit-action behavior, not hosted MDSync acceptance.
- Test next: independent critical review, then Task 019 cohesive local and
  hosted dogfood.

## Skill Trial Notes

- Source comparison: not applicable
- Contract markers checked: access, authority, confirmation, conflict, repair, redaction
- Trial status: implementation, automated gates, native visual acceptance, and
  independent review passed.

## Blockers

- None. Hosted MDSync acceptance remains owned by Task 019.

## Follow-Ups

- Task 019 owns cohesive native and hosted proof.
