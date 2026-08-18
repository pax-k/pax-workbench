# 012: Prove the MVP End to End

Status: complete
Type: validation
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/mvp-scope.md; docs/release-gates.md
Reversibility: easy
Learning objective: determine whether the controlled workbench loop is usable and evidence-backed against a real project
Source under test: repo-local path

## Goal

Dogfood Build Right Studio from repository open through skill inspection/setup,
helper decisions, real bounded task execution, checkpoint persistence/resume,
confirmed iteration, and final resolver stop, recording durable manual-trial and
release evidence.

## Non-Goals

- Production-sign, notarize, publish, or distribute the application.
- Execute production or irreversible work.
- Validate multi-agent, cloud, marketplace, or issue-tracker features.
- Convert one founder trial into customer validation.

## Required Reading

- docs/mvp-scope.md
- docs/release-gates.md
- docs/evidence/preflight.md
- tasks/sprint-1.md
- tasks/issues/004-verify-native-tauri-boundary.md
- tasks/issues/011-run-confirmed-goal-loop.md

## Acceptance Criteria

- [x] Tasks 003-011 are all `complete` with passing evidence before this trial
      starts; deferred, canceled, split, or superseded dependencies do not qualify.
- [x] The trial opens a real disposable or this repository, inspects exact skill
      provenance, and runs real deterministic helpers.
- [x] At least one reversible AI-owned task is executed through Codex and
      verified from repository files.
- [x] The application is restarted at a checkpoint and reconstructs current
      truth before asking whether to continue.
- [x] One confirmed next iteration or a real resolver stop is observed, with
      every confirmation and stop reason visible.
- [x] `docs/evidence/manual-trials.md` records the required agent-agnostic packet.
- [x] Release gates distinguish proved, simulated, unproven, and post-MVP scope.
- [x] `bun run check`, Rust tests, and debug Tauri build pass on the trial source.

## Baseline Evidence

Reconciled 2026-07-22: Tasks 003-011 are complete with signed native evidence,
real bounded execution, checkpoint recovery, and two separately confirmed goal
iterations. This task owns the remaining whole-product dogfood packet, final
release-gate readback, and explicit real/manual/simulated boundary.

## Solution-Fit Rationale

- Requirement served: validate the complete founder-facing MVP workflow.
- Constraints honored: reversible local trial with no production effects.
- Guarantees preserved: durable evidence and explicit real/simulated boundary.
- Cost accepted: one structured dogfood packet and full-gate rerun.
- Deferred capability: public release and customer validation.

## Verification

- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run tauri build --debug`
- Manual Build Right Studio trial with recorded screenshots, commands, and artifacts.
- Final preflight, execution resolver, and release-gate readback.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-22 | Tasks 003-011 status and evidence audit | pass | Every required dependency is `complete`; broad Task 002 remains truthfully `superseded` and is not counted as a completed dependency. |
| 2026-07-22 | Fresh signed source binding | pass | `bun scripts/build-signed-macos.ts` produced the exact installed bundle at `/Users/pax/Applications/Build Right Studio.app`; `codesign --verify --deep --strict --verbose=2` passed. Binary SHA-256: `d351f45bb138dd987da3d6e0dc2dae7a49f9e1bb2976413769c5aa4fc529b6aa`; deterministic `src`/`src-tauri/src`/`skill-ui` input digest: `697ce28857b36fd6291f4d86e25c4143a3da5252af8b8a6eace682329b01e013`. |
| 2026-07-22 | Fresh disposable baseline `/tmp/pax-workbench-task012-final.gPZIjo/repo` | pass | Preflight returned `ready-for-execution` high; strict resolver selected only Task 920; its task contract had no missing fields or gates. The fixture was committed before app execution. |
| 2026-07-22 | Cohesive signed-app provenance and helpers | pass | The exact app opened the fresh fixture, exposed `pax-k/build-right`, CLI `skills@1.5.19`, fixed setup argv, lock hashes, and a read-only cancellation boundary; real preflight, continue, and execution helpers returned the expected typed decisions. Evidence: `output/native/task-012-final-provenance.jpeg`, `task-012-final-preflight.jpeg`, `task-012-final-continue.jpeg`, and `task-012-final-execution-check.jpeg`. |
| 2026-07-22 | First explicit confirmation and verified checkpoint | pass | Task 920 alone was confirmed, created exact `task012-first-proof.txt`, kept the second proof absent, completed its task/tracker evidence, promoted Task 921 to ready without execution, and ended at `continueAvailable`. Evidence: `output/native/task-012-final-first-confirmation.jpeg` and `task-012-final-first-checkpoint.jpeg`. |
| 2026-07-22 | Real restart and fresh confirmation | pass | The exact app was terminated and relaunched; no Codex child remained. Reopening the same fixture reconstructed `resumable`, repository/task/Git match, `Automatic Codex execution started: false`, and `Fresh confirmation required`. Evidence: `output/native/task-012-final-restart-resumable.jpeg`. |
| 2026-07-22 | Second explicit confirmation and final stop | pass | Task 921 preserved the first proof, created exact `task012-second-proof.txt`, completed itself and the disposable sprint, and ended at repository-affirmed `goalComplete`. The strict resolver returned `no-ready-task`; stop gates, `git diff --check`, and zero-child checks passed. Evidence: `output/native/task-012-final-second-confirmation.jpeg` and `task-012-final-goal-complete.jpeg`. |
| 2026-07-22 | `docs/evidence/manual-trials.md` | pass | Records the exact cohesive packet fields: run label, tool surface, source, target, full commands, hashed artifacts, result, proved, simulated, `Unproven:`, and follow-ups. Earlier Tasks 003-011 are labeled supporting component evidence, not substituted for the cohesive Task 012 trial. |
| 2026-07-22 | `bun run check` | pass | Typecheck, 40 frontend tests, and production build passed. |
| 2026-07-22 | `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | 120 Rust tests, native compile, and formatting check passed. |
| 2026-07-22 | Final `bun run check`; Rust test/check/fmt; `bun run tauri build --debug` | pass | Final closeout rerun passed 40 frontend tests, 120 Rust tests, compile/format checks, and a fresh debug build at `src-tauri/target/debug/pax-workbench`, SHA-256 `637f54e5af5c0da65143ad54a083462dd4ed2de05fb29a170753c980fdfbe3c6`. |
| 2026-07-22 | Development-signature and process readback | pass | Installed app verifies through Apple Development, WWDR, and Apple Root with Team ID `6DNPZ54Z8L`; no Codex wrapper/native child remained after either confirmed iteration. Production notarization/distribution is not claimed. |
| 2026-07-22 | Pre-closure Task 012 preflight, resolver, task contract, stop gates, and `git diff --check` | pass | Preflight was ready/high; strict resolver selected only active Task 012 with no gates; contract fields were complete; informational boundary labels no longer created a false-positive stop; diff check passed. |
| 2026-07-22 | Independent Sol/high release closure review | pass | Approved with no blocking findings after independently reproducing source/bundle hashes, checking all nine artifact hashes, inspecting the cohesive trial and durable receipt, reconciling resolver/gates, and confirming verification evidence. Residuals remain explicitly non-blocking and inside the recorded unproven boundary. |
| 2026-07-22 | Terminal resolver, stop gate, authority, signature, and process readback | pass | Strict resolver returned `no-ready-task` with no blocking gates, ready tasks, active tasks, or external follow-ups; Task 012 stop gate truthfully reported the selected task complete; Tasks 003-012 all read `complete`; Sprint 1, blueprint, release gates, and conflicts are terminal/reconciled; signature, diff, and zero-Codex-child checks passed. |

## Files Changed

- `docs/evidence/manual-trials.md` - exact cohesive signed whole-MVP trial packet.
- `docs/evidence/preflight.md` - reconciles the original Rust blocker with current native proof.
- `docs/mvp-scope.md` - reconciles the current toolchain/native-readiness boundary without widening scope.
- `docs/release-gates.md` - explicit proved, simulated, unproven, and post-MVP boundary.
- `output/native/task-012-final-*.jpeg` - hashed cohesive provenance, helper,
  confirmation, checkpoint, restart, and goal-complete evidence.
- `tasks/issues/012-prove-mvp-end-to-end.md` - dogfood evidence and terminal verification.
- `tasks/sprint-1.md`, `docs/blueprint-status.md`, and `docs/release-gates.md` - final sprint and release reconciliation.

## Verification Summary

- All eight acceptance criteria have durable cohesive repository, command, or native UI evidence.
- Full frontend, Rust, format, debug-build, signature, process-cleanup, preflight,
  resolver, task-contract, and diff checks pass.
- The single post-dependency trial distinguishes real/manual evidence from test-simulated stop-family
  breadth and from unproven/post-MVP claims.
- Independent release review approved Task 012 and Sprint 1 closure with no
  blocking findings.

## Learning Notes

- Proved: the development-signed local MVP executes the complete controlled
  repository/helper/Codex/checkpoint/confirmation/stop loop while keeping
  Markdown and Git authoritative.
- Real/manual: one freshly signed build opened one fresh disposable repository,
  previewed provenance/setup, ran helpers, executed Task 920, verified a
  checkpoint, restarted, reconstructed truth, required and received a fresh
  Task 921 confirmation, stopped at `goalComplete`/`no-ready-task`, and cleaned
  up its child process.
- Simulated: exhaustive terminal stop-family and concurrency/race breadth remain
  Rust production-seam tests; they are not presented as separate native trials.
- Unproven: customer usability/value, provider portability, production release,
  non-Unix containment, unattended/parallel/multi-agent loops, and the cause of
  the earlier signed pre-event stalls.
- Test next: founder usability feedback after technical proof.

## Skill Trial Notes

- Source under test: repo-local app plus project-scoped `pax-k/build-right` skills.
- Source comparison: pass against `skills-lock.json` paths/hashes and signed-app preview.
- Contract markers checked: exact source, fixed effect boundary, typed helpers,
  repository authority, real/manual/simulated labels, complete trial packet,
  verification ladder, stop gates, and release classifications.
- Trial status: pass.

## Blockers

- None.

## Follow-Ups

- Reassess post-release backlog only after this task completes.
