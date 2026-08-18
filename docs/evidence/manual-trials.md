# Manual Trials

Run label: Sprint 1 cohesive signed native MVP dogfood, 2026-07-22

Agent/tool surface: Codex root orchestration; Apple-development-signed Build
Right Studio launched by macOS LaunchServices and operated through Computer
Use; repo-local Bun helpers; Codex CLI 0.144.4; Rust/Cargo native verification.

Skill source: Project-scoped `pax-k/build-right` installations at
`.agents/skills/build-right-{preflight,feature-planning,execution,engineering-principles}`,
validated against `skills-lock.json`. Build Right execution hash:
`f85c42a213f8e75c5df6ff493887c64f846d62642613b3e4fb60c11d9acf6fb0`.

Target: Trial source `/Users/pax/Documents/Repos/pax-workbench`; exact signed
bundle `/Users/pax/Applications/Build Right Studio.app`; disposable cohesive
repository `/tmp/pax-workbench-task012-final.gPZIjo/repo`. The installed binary
SHA-256 was `d351f45bb138dd987da3d6e0dc2dae7a49f9e1bb2976413769c5aa4fc529b6aa`.
The deterministic digest of the `src`, `src-tauri/src`, and `skill-ui` build
inputs was `697ce28857b36fd6291f4d86e25c4143a3da5252af8b8a6eace682329b01e013`.

Commands: Built and signed the current source with `bun
scripts/build-signed-macos.ts`; installed that exact bundle at the canonical
path; verified it with `codesign --verify --deep --strict --verbose=2
/Users/pax/Applications/Build Right Studio.app`. Before UI execution, the fresh
fixture passed `bun .agents/skills/build-right-preflight/scripts/preflight-check.ts
--cwd /tmp/pax-workbench-task012-final.gPZIjo/repo --format markdown`, `bun
.agents/skills/build-right-execution/scripts/continue-check.ts --cwd
/tmp/pax-workbench-task012-final.gPZIjo/repo --format markdown --strict`, and
`bun .agents/skills/build-right-execution/scripts/execution-check.ts --cwd
/tmp/pax-workbench-task012-final.gPZIjo/repo --task
tasks/issues/920-create-first-cohesive-proof.md --mode task-contract --format
markdown`. In the signed app: opened the fixture; previewed and cancelled the
fixed `bunx skills@1.5.19 update build-right-preflight
build-right-feature-planning build-right-execution
build-right-engineering-principles --project --yes` setup mutation; ran real
preflight, continuation, and task-contract helpers; explicitly confirmed Task
920; restarted the exact app after repository verification; reopened the same
repository; observed `resumable`, `Automatic Codex execution started: false`,
and `Fresh confirmation required`; then explicitly confirmed Task 921. Final
shell checks asserted both exact proof files, reran the strict resolver and stop
gates, ran `git diff --check`, and verified zero Codex child processes. The
broader source verification was `bun run check`; `cargo test --manifest-path
src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml`;
`cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`; and `bun run
tauri build --debug`.

Artifacts: `output/native/task-012-final-provenance.jpeg` (SHA-256
`a479df17dadd2f11391c50aec44a5060a72b3062563af1d7ef1ca87f38544f2d`);
`output/native/task-012-final-preflight.jpeg` (`b600d8d91d4d7dd59c2651b4862c0b5d8ff88c2d1d008c28f1731ee28ea79161`);
`output/native/task-012-final-continue.jpeg` (`ef90550186a2ba476037657ec9609f3e682f28f39c07d42cec7d86628cc94ffd`);
`output/native/task-012-final-execution-check.jpeg` (`07b5d9d3f173c23292680163277d02ec86cc7b585f3a73aa1c9c765f17adeb40`);
`output/native/task-012-final-first-confirmation.jpeg` (`bf9346ff31441df9181ae28c017d526d8a7f49408975909d549ed5b260087e75`);
`output/native/task-012-final-first-checkpoint.jpeg` (`0424033a10fffaa17075f4d7618d8a2dfbe163e09121bdd422313cff8e2e85cf`);
`output/native/task-012-final-restart-resumable.jpeg` (`abee24f4f69cb3c7ec570456fd23aca24301dade156e1f591cfd80019c0edf36`);
`output/native/task-012-final-second-confirmation.jpeg` (`b988b74e2900abc12ac52caf3d8e774a091fdc2add4fce6ff7c1156da9e59235`);
`output/native/task-012-final-goal-complete.jpeg` (`673c732a05a2c54ce5b6de92d59d535667ec4cdbac7ebfca816851626eef1095`).
Tasks 003-011 remain supporting component evidence; this packet's whole-loop
claim rests on the one cohesive post-dependency Task 012 trial above.

Result: Pass for the local signed MVP technical loop. Task 920 produced only
`task012-first-proof.txt`, passed its exact checks, and ended at
`continueAvailable` with Task 921 ready but unexecuted. After a real restart,
Task 921 required a new confirmation, preserved the first proof, produced
`task012-second-proof.txt`, passed both exact checks, and ended at
repository-affirmed `goalComplete`. The strict final resolver returned
`no-ready-task`; no Codex child remained.

Proved: Exact installed skill provenance and fixed setup boundary; real native
repository selection; real allowlisted helpers; real Codex JSONL execution; one
cohesive two-task repository-verified loop; checkpoint restart and receipt
recovery; two distinct confirmations and invocations; `continueAvailable` then
`goalComplete`; final `no-ready-task`; no hidden third invocation or remaining
Codex child; frontend, Rust, debug-build, and development-signature gates.

Simulated: Exhaustive founder/external/conflict/failure/cancellation/stale/
invalid terminal-family breadth and concurrency/race variants are
production-seam Rust test coverage. Those tests complement, but are not
represented as, separate native manual failures. Demo-only activity remains
labeled simulated in the product.

Unproven: Customer usability or value; production signing, notarization,
publishing, and distribution; non-Unix process containment; unattended,
parallel, or multi-agent goal loops; provider portability; the environmental
cause of the earlier signed pre-event stalls.

Follow-ups: Conduct founder usability feedback after technical proof; assess
only the explicitly recorded post-release backlog after Sprint 1; preserve the
unknown earlier-stall cause as residual evidence rather than claiming a repair.

---

Run label: Sprint 2 cohesive signed local and hosted HA2HA/MDSync acceptance,
2026-07-23

Target: Exact source `/Users/pax/Documents/Repos/pax-workbench`; signed bundle
`/Users/pax/Applications/Build Right Studio.app`; disposable two-task fixture
`/tmp/pax-workbench-task019-live-20260723-a/repo`; one hosted MDSync workspace
created through the pinned clean source at `/Users/pax/Documents/robosync`
revision `ebd5c8d483a26096f95fdcc8e4f5242270481e9b`.

Commands and actions: Built and promoted the exact repaired source, verified
strict deep code signing, and launched it through macOS LaunchServices. The
signed app completed Task 989 locally with one confirmation and stopped before
Task 990. One hosted workspace was normalized to the deterministic four-file
HA2HA 1.0.0 scaffold, then the app published exactly three envelope files.
Viewer access remained read-only. A same-content remote version advance after
preview caused the first shared confirmation to stop with `Runtime started:
false`. A fresh version-2 join and preview started exactly one shared Codex
invocation; repository checks alone completed Task 990 and its sprint, and the
remote task progressed through claimed to done. The app was restarted; the
receipt recovered terminal repository and sanitized remote coordinates without
a native capability/session and reported `Automatic Codex execution started:
false`. A separate Viewer-only agent context independently read the hosted
task, evidence, handoff/status, event history, and version history. The edit
capability was then revoked and denied; retained Viewer access remains
read-only by explicit closeout choice.

Artifacts: `output/native/task-019-signed-local-solo.png` (SHA-256
`6fbf045730868eadd936976aa5cee7e49627d7e65442c4d3ae132b01e09ea156`);
`output/native/task-019-hosted-viewer-denial.jpeg`
(`b0f8938323289c2a018e99d9e57424f006adf265346ad845d79a344ecfc08e1b`);
`output/native/task-019-hosted-conflict-no-codex.jpeg`
(`26e7273d5a1a57a14e39e7e1fddb39bbc68f817a703debc82bc5392cd1d1ea57`);
`output/native/task-019-hosted-reconciled.jpeg`
(`2ef325e24d67c4b66f2800d486f9f6329e8a7784bbf15fdfdbee58f123a2309f`);
`output/native/task-019-hosted-restart-safe.jpeg`
(`a44d167dd10e0e65d4d13900c7390d7188c15b7f0fd0936be22045319e45b76d`);
`output/native/task-019-hosted-revoked-edit-denied.jpeg`
(`36c063db9458bc4e00afac6eca0a2aaf8378fa02ee9a30ddd3d2c9379a0e3dfb`).
All six artifacts are 1229x768 and expose no capability material.

Verification: `bun run check` passed typecheck, 69 tests, and the production
bundle. `cargo test --manifest-path src-tauri/Cargo.toml` passed 203 tests;
`cargo check` and `cargo fmt --check` passed. `codesign --verify --deep
--strict` passed. The installed binary SHA-256 is
`756500ab00fdffe9bb96f61e834cb209f03184fc05fc208833d3b463e1519d58`.
Both fixture proof assertions passed, the strict fixture resolver returned
`no-ready-task`, and exact capability values produced zero matches across 1,293
files and more than 21 MB in the main repo, fixture, app support, and WebKit
stores.

Result: Pass for Sprint 2 technical acceptance. Proved: unchanged signed local
solo execution; optional hosted publish/join; Viewer denial; stale-version
no-start; one explicit Collaborator claim/invocation; repository-authoritative
completion; hosted evidence/history readback; restart without retained
capability or automatic execution; second independent context; revocation; and
full regression/signature gates. Deterministic production-seam tests, not an
invented live outage, prove post-commit partial-sync effect debt and idempotent
repair breadth.

Unproved: customer usability/value, production signing/notarization/
distribution, provider portability, whole-backlog synchronization, unattended
execution, and multi-agent orchestration.
