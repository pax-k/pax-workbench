# 007: Implement Deterministic Helper Execution

Status: complete
Type: integration
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/raw/product-discussion.md; docs/execution-rules.md
Reversibility: easy
Learning objective: prove one contract-declared helper can run with bounded effects and produce typed workflow evidence
Source under test: repo-local path

## Goal

Execute allowlisted Build Right helpers declared by validated skill contracts and
normalize their results into typed decisions, evidence, and run-inspector events.

## Non-Goals

- Execute arbitrary repository scripts.
- Run Codex or mutate product files.
- Infer commands from `SKILL.md` prose.
- Treat helper output as authority without refreshing repository state.

## Required Reading

- docs/execution-rules.md
- tasks/issues/003-validate-skill-ui-contracts.md
- tasks/issues/005-complete-safe-repository-session.md
- tasks/issues/006-add-explicit-skill-setup-adapter.md

## Acceptance Criteria

- [x] Helper registry accepts only validated contract IDs and exact argv templates.
- [x] Invocation uses argument arrays with no shell interpolation.
- [x] Structured result includes decision, confidence, next action, evidence,
      warnings, exit status, and bounded stdout/stderr.
- [x] Timeout, cancellation, output-size limits, malformed output, and missing
      runtime are explicit terminal results.
- [x] Run-inspector events distinguish command start, output, decision, failure,
      cancellation, and repository refresh.
- [x] Preflight and continue/execution resolvers have fixture-backed parsers.
- [x] Real helper output is labeled real and never confused with demo events.

## Baseline Evidence

`run_preflight` is a hardcoded blocking Rust command returning one untyped
Markdown string. It has no generalized registry, timeout, cancellation, or
bounded event stream.

Read-only reconciliation against the installed helper parsers and validated
`skill-ui` contracts established a closed registry. The native adapter never
executes the repository path directly: it opens the declared helper through
no-follow descriptors, verifies the exact length and release-pinned SHA-256,
then passes the authenticated byte snapshot to `bun -` over managed stdin.
The effective fixed argv contracts are:

- `preflight-check`: `bun -
  --cwd <canonical-root> --mode all --format json`
- `continue-check`: `bun -
  --cwd <canonical-root> --format json --strict`
- `execution-check`: `bun -
  --cwd <canonical-root> --mode <closed-mode> --task <validated-task-path>
  --format json`

Supported release anchors are `preflight-check.ts` length 11,949 and SHA-256
`e01e10f2...cbb8e`, `continue-check.ts` length 24,041 and SHA-256
`153c2212...bb40`, and `execution-check.ts` length 11,693 and SHA-256
`4025c61d...b596`.

`<canonical-root>` comes only from the selected native repository session;
`<closed-mode>` is a native enum; and `<validated-task-path>` must be a currently
inventoried repo-relative Markdown task. No executable, script path, flag, mode,
or free-form argument may cross from the frontend.

## Solution-Fit Rationale

- Requirement served: expose real readiness and resolver decisions in the UI.
- Constraints honored: contract-declared helpers and explicit user actions only.
- Guarantees preserved: exact argv, bounded output, structured failures, repo refresh.
- Cost accepted: parsers and compatibility fixtures for current helpers.
- Deferred capability: arbitrary plugin or repository command execution.

## Verification

- Helper registry/parser fixture tests.
- Timeout, cancellation, malformed-output, and output-bound tests.
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run check`
- Real preflight and resolver smoke against a disposable repository.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | Validated `skill-ui/*.json` helper declarations | pass | First-party contracts declare `preflight-check`, `continue-check`, and `execution-check` as explicit-user-action helpers. |
| 2026-07-21 | Installed helper `--help` and parser inspection | pass | Verified supported modes, JSON output, strict resolver behavior, cwd handling, and task-path input; fixed argv templates above are ready for implementation. |
| 2026-07-21 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 59 Rust tests passed, including 19 focused helper tests for the closed registry, release digests/lengths, no-follow opening, verified snapshots, parser binding, malformed/nonzero/missing-runtime outcomes, bounded output, timeout/cancellation, FIFO/sparse inputs, stdin backpressure, descendants, and repository refresh. |
| 2026-07-21 | `bun run check` | pass | Typecheck, 26 frontend tests, and the production Vite bundle passed. |
| 2026-07-21 | `bun run tauri build --debug --bundles app` | pass | Latest implementation compiled into `Build Right Studio.app` for the native smoke. |
| 2026-07-21 | Independent Sol security/lifecycle review and remediation | pass | Authenticated script bytes, task/result binding, Unix support truth, bounded source reads, and managed stdin findings were repaired; final review reported no critical or medium findings. |
| 2026-07-21 | Disposable CLI baselines in `/private/tmp/pax-workbench-task007.UHVVMS` | pass | Preflight returned `ready-for-execution`; continue returned `execute-task` for Task 007; task-contract returned `proceed`; copied helper hashes matched the native release registry. |
| 2026-07-21 | Native compiled-app helper smoke | pass | The fresh app instance ran authenticated `preflight-check`, `continue-check`, and task-bound `execution-check` through `bun -`; results matched CLI baselines, displayed bounded raw JSON and typed evidence, distinguished manual requests from real effects, and refreshed repository truth. Screenshot: `output/native/task-007-native-helper-smoke.jpeg`. |
| 2026-07-21 | Post-smoke fixture comparison | pass | `diff -qr` and `cmp` proved copied docs, tasks, skills, UI contracts, and `skills-lock.json` remained byte-identical; all 84 fixture files remained the original untracked baseline. |
| 2026-07-21 | Whitespace validation | pass | `git diff --check` passed; explicit `git diff --no-index --check` on all untracked Task 007 source files produced no diagnostics. |

## Files Changed

- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/App.tsx`
- `src/App.test.tsx`
- `output/native/task-007-native-helper-smoke.jpeg`
- Ignored build outputs under `dist/` and `src-tauri/target/`, including the debug `.app` bundle
- External disposable fixture `/private/tmp/pax-workbench-task007.UHVVMS`

## Verification Summary

- Focused and broader checks passed: helper Rust 19/19, full Rust 59/59,
  frontend 26/26, typecheck, production bundle, debug native `.app`, and
  explicit whitespace checks.
- Real boundary: the compiled app authenticated exact helper bytes and ran
  preflight, continue, and task-contract JSON helpers through managed `bun -`
  stdin, producing typed decisions and refreshed repository snapshots.
- Manual boundary: the native project picker, Task 007 selection, and each
  helper-start button were explicit desktop actions.
- Simulated boundary: no demo event or checkpoint was used as acceptance
  evidence; all smoke events were labeled manual request or real local effect.
- Platform boundary: helper execution is supported on Unix for this MVP. On
  non-Unix it returns typed `unsupportedPlatform`, `executed=false`, refreshes
  repository truth, and makes no timeout/cancellation claim.
- Unavailable optional diagnostic: `cargo fmt --check` was not run because the
  authorized minimal Rust toolchain does not include rustfmt.
- Residual risk: supported helper byte changes require a deliberate native
  registry/version update and a rebuilt application before execution resumes.

## Learning Notes

- Proved: contract-declared helpers can execute only from authenticated release
  bytes, with closed argv, bounded resources, typed decisions, explicit terminal
  failures, truthful event provenance, and repository refresh.
- Manual: project/task selection and helper start remain explicit user actions.
- Simulated: none used for native proof; fixture events remain separately labeled.
- Test next: normalize Codex JSONL through the same event boundary.

## Skill Trial Notes

- Source comparison: all three disposable helper hashes and lengths matched the
  release-pinned native registry before execution.
- Contract markers checked: validated helper declaration, exact source path,
  digest/length, canonical root, fixed argv, closed mode, selected task, output
  context, typed result, timeout/cancellation, bounded evidence, and refresh.
- Trial status: pass in the compiled native application.

## Blockers

- None.

## Follow-Ups

- Task 008 adds the Codex runtime adapter.
