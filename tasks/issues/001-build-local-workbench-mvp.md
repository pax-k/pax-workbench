# 001: Build Local Workbench MVP

Status: complete
Type: foundation
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/mvp-scope.md
Reversibility: easy
Learning objective: prove the repo-native workbench shape with a testable frontend and an explicit native adapter boundary
Source under test: repo-local path

## Goal

Create a runnable React/TypeScript workbench and Tauri 2 shell that make one
repository's Build Right artifacts, workflow state, and simulated run evidence
legible without introducing a shadow task database.

## Non-Goals

- Production signing or distribution.
- Unattended live agent loops.
- Cloud state or third-party integrations.
- Editing installed `.agents/skills/**` sources.

## Required Reading

- docs/raw/product-discussion.md
- docs/mvp-scope.md
- docs/execution-rules.md
- docs/release-gates.md

## Acceptance Criteria

- [x] A production-buildable React/TypeScript interface implements the project
      navigator, Markdown workbench, skill modes, run inspector, and execution
      ribbon.
- [x] Demo state is explicitly labeled and can be replaced by a typed Tauri
      bridge.
- [x] Markdown task/sprint parsing has representative unit tests.
- [x] Tauri source defines project-scoped inspection, file read/write, and
      preflight-helper commands with traversal protection.
- [x] `bun run check` succeeds.
- [x] Native build status is recorded truthfully.

## Baseline Evidence

The repository initially contained only installed skills and `skills-lock.json`.
No package manifest, application source, tests, or native toolchain existed.

## Verification

- `bun run typecheck`
- `bun run test`
- `bun run build`
- `bun run tauri build --debug` when Rust/Cargo are available
- Browser screenshot inspection of the built interface

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | Initial preflight helper | ask-founder | Resolved by imported founder discussion |
| 2026-07-21 | `bun run check` | pass | Strict TypeScript, 5 Vitest tests, and Vite production build passed |
| 2026-07-21 | `bun run tauri info` | partial | Tauri config detected; Xcode present; `rustc`, Cargo, and rustup absent |
| 2026-07-21 | Playwright structured-view inspection | pass | Current UI rendered without browser errors; screenshot at `output/playwright/workbench-structured.png` |
| 2026-07-21 | Independent implementation review | pass-after-fixes | Fixed root/nested symlink inventory, atomic project switching, truthful subprocess boundary, source-backed navigator/progress, and missing-skills empty state |

## Files Changed

- `README.md`, `package.json`, `bun.lock`, and TypeScript/Vite config - runnable project and command surface.
- `src/**` - workbench UI, typed bridge, repo-derived projections, parsers, and tests.
- `src-tauri/**` - Tauri shell plus project-scoped file and helper commands.
- `docs/**` - founder source, product truth, execution rules, decisions, gates, and evidence.
- `tasks/**` - Sprint 0 execution state and durable closeout.
- `output/playwright/workbench-structured.png` - current visual QA artifact.

## Verification Summary

- `bun run typecheck` - pass.
- `bun run test` - pass, 2 files and 5 tests.
- `bun run build` - pass, production assets emitted to ignored `dist/`.
- `bun run check` - pass after review fixes.
- `bun run tauri info` - native configuration detected; compile unavailable because Rust/Cargo are not installed.
- Rust traversal tests are authored but not executed in this environment.

## Learning Notes

- Proved: the React workbench, raw/structured Markdown projection, repo-derived
  task navigation, explicit simulation labels, typed bridge contract, and
  production frontend build.
- Simulated: agent run events and goal-loop progression.
- Test next: compile and exercise the Tauri boundary against a real repository,
  then run one bounded Codex adapter task.

## Skill Trial Notes

- Source comparison: not applicable
- Contract markers checked: task fields, preflight readiness, source authority
- Trial status: pass for frontend prototype; native trial pending

## Blockers

- None.

## Follow-Ups

- `tasks/issues/002-verify-native-tauri-boundary.md`.
