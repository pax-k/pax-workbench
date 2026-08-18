# 005: Complete Safe Repository Session

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/mvp-scope.md; docs/execution-rules.md
Reversibility: easy
Learning objective: prove one real repository can be inspected and edited without stale or outside-root writes
Source under test: repo-local path

## Goal

Complete a native repository session that inventories authority surfaces, lets
the user select and round-trip one Markdown file, and refreshes repository/Git
state after changes.

## Non-Goals

- Install skills.
- Execute helpers or agents.
- Watch arbitrary paths outside the selected root.
- Introduce a task or planning database.

## Required Reading

- docs/mvp-scope.md
- docs/execution-rules.md
- tasks/issues/004-verify-native-tauri-boundary.md
- src/lib/bridge.ts
- src-tauri/src/lib.rs

## Acceptance Criteria

- [x] Native inspection reports agent instructions, docs, sprint/task trackers,
      skill provenance, Git branch, and dirty state with structured errors.
- [x] The user explicitly selects the Markdown file to edit.
- [x] Reads return a content version/hash and writes require the expected version.
- [x] Writes use an atomic replacement path and reject stale, lexical, and
      symlink escapes.
- [x] Save refreshes file, Git, and projected task state from repository truth.
- [x] File watching or explicit refresh is bounded to the selected repository.
- [x] Tests cover stale-write rejection, outside-root paths, empty repositories,
      and post-save refresh.

## Baseline Evidence

Task 004 proves the native shell compiles and rejects lexical, inventory-symlink,
regular symlink, and dangling symlink escapes. The current session still writes
directly after validation, with no stale-content precondition or atomic
replacement, and live post-save refresh is not verified in the compiled app.

## Solution-Fit Rationale

- Requirement served: open and safely work with one local engineering repository.
- Constraints honored: Markdown/Git remain authoritative; user chooses the file.
- Guarantees preserved: outside-root rejection and no shadow task state.
- Cost accepted: content-version checks and bounded refresh lifecycle.
- Deferred capability: multiple simultaneously open repositories.

## Verification

- Focused Rust and TypeScript session tests.
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run check`
- Disposable-repository Markdown round-trip proof.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 25 Rust tests cover root containment, structured inventory errors, versioned writes, stale/concurrent writers, path swaps, symlinks, empty repositories, and post-save refresh. |
| 2026-07-21 | `bun run check` | pass | Typecheck, 15 frontend tests, and production Vite build passed. |
| 2026-07-21 | `bun run tauri build --debug --bundles app` | pass | Latest code compiled and produced `Build Right Studio.app` for the real native trial. |
| 2026-07-21 | Independent Sol review and re-review | pass | Git-root, provenance-symlink, concurrency, stale-recovery, project-switch, and inventory-error findings were repaired; final review reported no remaining code findings. |
| 2026-07-21 | Native disposable-repository round trip | pass | Compiled app opened `/private/tmp/pax-workbench-task005.AFULTf`, required explicit selection, saved task 900 through Tauri IPC, and refreshed branch, dirty state, file status, and projected criteria. Screenshot: `output/native/task-005-native-round-trip.jpeg`. |
| 2026-07-21 | Disposable Git readback | pass | `git status --short` showed only the selected task modified; diff changed `Status: ready` to `active` and the criterion to checked; saved SHA-256 was `00d9c87e4d68a86975d0adcd1e06d3daf623d6b18aabc14ab7fba4fec1d2a56f`. |
| 2026-07-21 | `git diff --check` | pass | No whitespace errors. |

## Files Changed

- `src-tauri/src/lib.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/lib/demo.ts`
- `src/App.tsx`
- `src/App.test.tsx`
- `output/native/task-005-native-round-trip.jpeg`
- Ignored build outputs under `dist/` and `src-tauri/target/`, including the debug `.app` bundle
- External disposable fixture `/private/tmp/pax-workbench-task005.AFULTf`

## Verification Summary

- Focused checks: Rust 25/25 and frontend 15/15 passed.
- Broader checks: typecheck, production bundle, debug native `.app` bundle, and
  `git diff --check` passed.
- Real boundary: repository selection, explicit file selection, versioned save,
  Tauri IPC, atomic native write, Git dirty refresh, and task projection refresh
  were exercised in the compiled desktop application.
- Manual boundary: macOS folder selection and the Save action were driven through
  the native UI against a disposable repository.
- Simulated boundary: the run inspector remains explicitly labeled `SIMULATED`;
  no helper or agent runtime claim is made by this task.
- Unavailable optional diagnostic: `cargo fmt --check` was not run because the
  authorized minimal Rust toolchain does not include rustfmt.
- Residual risk: a non-cooperating external process may still win the final
  descriptor-check-to-filesystem-operation race; in-app writes and cooperative
  file-lock users are serialized and checked.

## Learning Notes

- Proved: one compiled native session can inventory repository authority, require
  explicit Markdown selection, reject stale/outside-root writes, atomically save,
  and refresh file, Git, and projected task truth.
- Manual: the real desktop folder picker and Save action remain explicit user
  operations.
- Simulated: only the separate run-inspector event stream remains simulated.
- Test next: project-scoped skill setup.

## Skill Trial Notes

- Source comparison: not applicable
- Contract markers checked: repository root, versioned write, refresh evidence
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 006 uses this repository session for skill setup.
