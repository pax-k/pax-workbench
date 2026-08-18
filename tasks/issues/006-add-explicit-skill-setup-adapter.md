# 006: Add Explicit Skill Setup Adapter

Status: complete
Type: integration
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/raw/product-discussion.md; docs/mvp-scope.md
Reversibility: easy
Learning objective: prove first-party skills can be previewed and installed or updated without shell interpolation or hidden provenance changes
Source under test: repo-local path

## Goal

Add a project-scoped skill setup flow that previews the exact supported action,
requires confirmation, executes an allowlisted adapter, and refreshes installed
source/version/hash evidence.

## Non-Goals

- General-purpose shell execution.
- Third-party marketplace discovery.
- Automatic updates.
- User-scoped or global skill mutation.

## Required Reading

- docs/execution-rules.md
- skills-lock.json
- tasks/issues/003-validate-skill-ui-contracts.md
- tasks/issues/005-complete-safe-repository-session.md

## Acceptance Criteria

- [x] The exact supported first-party install/update CLI and argv contract are
      verified from available tooling before the task becomes ready.
- [x] Setup preview shows target project, source, skill IDs, version/hash change,
      exact argv, and files expected to change.
- [x] Only allowlisted first-party operations can execute; no shell string or
      arbitrary arguments cross the boundary.
- [x] The user explicitly confirms mutation.
- [x] Result captures exit status, bounded output, changed lock/provenance state,
      and structured repair guidance.
- [x] Repository inspection refreshes after success or failure.
- [x] Tests cover cancellation, unsupported source, argument injection, failure,
      and successful provenance refresh.

## Baseline Evidence

Installed skills and hashes exist in `skills-lock.json`, but the application has
no setup adapter. Read-only reconciliation against the upstream Build Right
README and cached `skills` CLI v1.5.19 help/parser establishes this allowlisted
project-scoped contract:

- executable: `bun`
- install argv: `x skills@1.5.19 add pax-k/build-right --skill
  build-right-preflight --skill build-right-feature-planning --skill
  build-right-execution --skill build-right-engineering-principles --agent
  codex --yes --copy`
- update argv: `x skills@1.5.19 update build-right-preflight
  build-right-feature-planning build-right-execution
  build-right-engineering-principles --project --yes`

The adapter must construct these tokens from a closed operation enum and fixed
skill registry; no user-provided executable, source, flag, or argument token may
cross the native boundary.

## Solution-Fit Rationale

- Requirement served: set up Build Right skills from the desktop workbench.
- Constraints honored: project-scoped, explicit, allowlisted mutation only.
- Guarantees preserved: visible provenance and no ambient shell authority.
- Cost accepted: narrow adapter tied to supported first-party tooling.
- Deferred capability: arbitrary sources and marketplace installation.

## Verification

- Adapter contract and injection tests.
- `bun run check`
- Native disposable-project setup trial.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | `/Users/pax/Documents/Repos/build-right/README.md` Quickstart | pass | Canonical upstream command is `bunx skills add pax-k/build-right`. |
| 2026-07-21 | Cached `skills` v1.5.19 README and `dist/cli.mjs` parser inspection | pass | Verified project default, Codex target path `.agents/skills/`, repeated `--skill`, `--copy`, `--yes`, named update arguments, `--project`, and shell-free positional parsing. |
| 2026-07-21 | `skills-lock.json` comparison before/after approved refresh | pass | All four `pax-k/build-right` sources and computed hashes remained identical; user explicitly approved retaining refreshed files. |
| 2026-07-21 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 41 Rust tests cover the closed registry, injection rejection, bounded output, timeout/cancellation, process-tree cleanup, duplicate invocation, stale-preview rejection, structured repair, and successful provenance refresh. |
| 2026-07-21 | `bun run check` | pass | Typecheck, 21 frontend tests, and the production Vite bundle passed. |
| 2026-07-21 | `bun run tauri build --debug --bundles app` | pass | Latest implementation compiled into `Build Right Studio.app` for the native trials. |
| 2026-07-21 | Independent Sol review and re-review | pass | Process lifecycle, UI races, provenance truthfulness, refreshed-error classification, and preview-baseline binding findings were repaired; final review reported no findings. |
| 2026-07-21 | Native cancel trial in `/private/tmp/pax-workbench-task006.po0yHd` | pass | The compiled app previewed the exact install argv, Cancel recorded a manual event, and `git status --short` remained empty. |
| 2026-07-21 | Native empty-project install in `/private/tmp/pax-workbench-task006.po0yHd` | expected repair | The real command exited `0` and changed 54 paths, but verification truthfully returned `verificationFailed` because no `skill-ui` contracts existed; refreshed hashes, bounded output, changed paths, and repair guidance were preserved. |
| 2026-07-21 | Native validated install in `/private/tmp/pax-workbench-task006-success.0TWHfK` | pass | With byte-identical validated `skill-ui` contracts present, the compiled app displayed `Setup completed`, exit status `0`, four validated operating cards, 54 changed paths, bounded output, and refreshed repository/provenance truth. Screenshot: `output/native/task-006-native-setup-success.png`. |
| 2026-07-21 | Disposable fixture readback | pass | `skills-lock.json` recorded source `pax-k/build-right` and hashes `826ee890…`, `2fe95e0c…`, `f85c42a2…`, and `48699c59…`; Git showed only the seeded contracts plus the expected project-scoped install outputs. |
| 2026-07-21 | `git diff --check` | pass | No whitespace errors before evidence closeout. |
| 2026-07-23 | Task 024 signed blank-project regression | repaired / pass | LaunchServices canonicalized the `bunx` symlink to `bun`, losing alias dispatch. The closed command is now `bun x skills@1.5.19 ...`; confirmed setup also creates or safely rebinds the four trusted provenance-bound `skill-ui` contracts so a blank project no longer requires terminal seeding. |

## Files Changed

- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/lib/demo.ts`
- `src/App.tsx`
- `src/App.test.tsx`
- `output/native/task-006-native-setup-success.png`
- Ignored build outputs under `dist/` and `src-tauri/target/`, including the debug `.app` bundle
- External disposable fixtures `/private/tmp/pax-workbench-task006.po0yHd` and `/private/tmp/pax-workbench-task006-success.0TWHfK`

## Verification Summary

- Focused and broader checks passed: Rust 41/41, frontend 21/21, typecheck,
  production build, debug native `.app` bundle, and `git diff --check`.
- Real boundary: the original compiled-app trial executed `bunx`; Task 024
  superseded native dispatch with semantically stable
  `bun x skills@1.5.19 add pax-k/build-right ...` argv. The two original
  disposable trials remain historical evidence. One intentionally
  demonstrated structured verification repair; the validated fixture completed
  and refreshed exact lock hashes and operating cards.
- Manual boundary: native folder selection, setup preview, Cancel, and Confirm
  were driven explicitly through the desktop UI.
- Simulated boundary: no simulated event was used as Task 006 proof; the demo
  checkpoint remains separately labeled and disabled while a real run is active.
- Unavailable optional diagnostic: `cargo fmt --check` was not run because the
  authorized minimal Rust toolchain does not include rustfmt.
- Residual risk: process-group termination is proved on the current Unix target;
  equivalent non-Unix descendant termination remains unproved.

## Learning Notes

- Proved: a closed, shell-free, confirmation-bound adapter can install all four
  project-scoped skills, reject stale previews, preserve failure repair evidence,
  and refresh validated provenance in the compiled app.
- Manual: preview, cancellation, and confirmation remain explicit user actions.
- Simulated: none used for the native acceptance proof.
- Test next: run a contract-declared deterministic helper.

## Skill Trial Notes

- Source comparison: installed source `pax-k/build-right` matched the validated
  UI contracts and the pinned `skills@1.5.19` preview.
- Contract markers checked: exact source, target, argv, preview token, lock hash,
  installed path, changed files, bounded output, and refreshed repository truth.
- Trial status: pass, with both the expected repair branch and successful install
  branch exercised in the compiled native application.

## Blockers

- None.

## Follow-Ups

- Task 007 consumes installed contract-declared helpers.
