# 011: Run Confirmed Goal Loop

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/raw/product-discussion.md; docs/mvp-scope.md
Reversibility: moderate
Learning objective: prove checkpointed one-task iterations can continue toward a goal without bypassing confirmation or stop gates
Source under test: repo-local path

## Goal

After each verified checkpoint, rerun the deterministic resolver, show its exact
decision, request explicit confirmation, run at most one next task, and stop at
the first founder, external, conflict, failure, stale, cancel, no-task, or goal
completion state.

## Non-Goals

- Run tasks without confirmation.
- Run more than one task per confirmed iteration.
- Override resolver gates or repository evidence.
- Support parallel or multi-agent execution.

## Required Reading

- docs/mvp-scope.md
- docs/execution-rules.md
- tasks/issues/009-execute-one-bounded-task.md
- tasks/issues/010-persist-checkpointed-goal-state.md

## Acceptance Criteria

- [x] Goal state defines objective and the complete allowed stop-condition set.
- [x] Every iteration reconstructs repository truth and reruns the full resolver
      after the previous task evidence is recorded.
- [x] Continue is offered only for a resolver-selected ready AI-owned task.
- [x] The UI shows next task, expected effects, gates, and requires a new explicit
      confirmation for every iteration.
- [x] One confirmation starts at most one Codex invocation.
- [x] Founder, external, no-ready-task, invalid-state, conflict, failure,
      cancellation, and goal-complete results stop without hidden retries.
- [x] Restart resumes at a checkpoint, refreshes truth, and asks again rather
      than auto-starting work.
- [x] State-machine tests cover two successful iterations followed by every
      terminal stop family.

## Baseline Evidence

Task 009 owns one bounded task and task 010 owns persistence/reconstruction, but
no task currently owns the resolver-confirm-iterate-stop controller.

Reconciled 2026-07-22: Tasks 009 and 010 are complete with reviewed controller,
checkpoint persistence, compiled restart, and Git-stale evidence. The current
frontend can prepare and confirm one bounded task and recover one checkpoint,
but it does not expose a typed resolver-driven next-iteration contract or prove
two separately confirmed iterations. This task remains the narrow missing loop.

## Solution-Fit Rationale

- Requirement served: continue through checkpointed goals while keeping control visible.
- Constraints honored: one task and one explicit confirmation per iteration.
- Guarantees preserved: resolver gates, repository authority, no hidden retries.
- Cost accepted: a small explicit orchestration state machine.
- Deferred capability: unattended, parallel, and multi-agent loops.

## Authorized Diagnostic Continuation

Founder authorization on 2026-07-22 reopens this task for an AI-owned causal
diagnostic and narrow fix. The prior native failure remains baseline evidence;
it is not waived or treated as consent to weaken acceptance.

The continuation is bounded to:

- capture the signed LaunchServices parent and Codex child state before the
  first JSONL event with non-mutating process and macOS diagnostic evidence;
- distinguish application code, inherited environment, provider startup,
  network policy, and macOS responsible-code hypotheses with controlled tests;
- implement only the smallest repository-local change supported by causal
  evidence; and
- rerun the original signed native restart plus two-confirmation acceptance.

Stop again for credentials, production access, system-level changes, a new
founder/product choice, destructive operations, ambiguous causal evidence, or
another failed acceptance result.

## Verification

- Goal-loop state-machine tests for two iterations and every stop family.
- `bun run check`
- Native disposable-repository trial with two reversible ready tasks.
- Restart at a checkpoint and inspect reconstructed resolver state.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-22 | `bun .agents/skills/build-right-execution/scripts/continue-check.ts --cwd /Users/pax/Documents/Repos/pax-workbench --format markdown --strict` | pass | Resolver selected Task 011 as the sole ready AI-owned task with high confidence and no blocking gates or external follow-ups. |
| 2026-07-22 | Current Task 009/010 controller, persistence, and recovery surfaces | baseline | One confirmed task and one durable checkpoint are implemented; no typed next-iteration orchestration contract or two-confirmation loop exists. |
| 2026-07-22 | Independent Sol/high reviews of F011-01 through F011-05 and F011-R1 | pass | Gate precedence, checkpoint evidence carry-forward, single-use confirmations, two-iteration production lifecycle, cancellation provenance, atomic cancel/result finalization, and off-thread cancellation enforcement have no remaining critical or medium findings. |
| 2026-07-22 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 120 Rust tests passed, including two fresh confirmations, all terminal families, both atomic finalization race orders, real process/persistence streaming, and off-thread concurrent cancellation. |
| 2026-07-22 | `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`; `bun run check` | pass | Native compile/format passed; frontend typecheck, 39 tests, and production build passed. |
| 2026-07-22 | `bun run tauri build --debug --bundles app` | pass | Fresh compiled app binary SHA-256 `fb3a4742a2ee3be22a6d08d3271fa0b1774bb24754304e070f0ae700558b9f4b`. |
| 2026-07-22 | Fresh compiled-app disposable trial at `/tmp/pax-workbench-task011-live.MhYsZH/repo` | partial / failed acceptance | Resolver selected only Task 900 and the async remediation kept the WebView responsive with a visible cancel action. The Codex child accepted the closed argv but emitted no complete JSONL line: durable revision 3 remained at event cursor 0, no proof file was created, Task 900 stayed `ready`, and Task 901 stayed `planned`. |
| 2026-07-22 | Native `Cancel bounded task` during the stalled trial | pass | Cancellation was accepted in the live UI, the goal loop returned `cancelledStop`, wrapper/native child processes were reaped, the receipt remained nonterminal with no checkpoint, and no second task or hidden retry started. |
| 2026-07-22 | Exact direct `codex exec` control on a clean clone | diagnostic pass only | The same task prompt produced JSONL, completed Task 900, and stopped before Task 901. This isolates the remaining failure to Codex early initialization under the compiled-app parent; direct execution is not counted as native acceptance. |
| 2026-07-22 | Compiled-parent launch matrix in `/tmp/pax-workbench-parent-probe.rarhGs` | causal isolation | Terminal/exec-tool parent emitted `thread.started` in 406-957 ms and exited 0. LaunchServices parents emitted stderr but zero JSONL with the Node wrapper or native binary, process groups on/off, null or closed stdin, inherited or reconstructed environment, `setsid`, PTY, minimal bundle metadata, and `danger-full-access`. Every timed-out process group was killed and reaped. |
| 2026-07-22 | macOS responsible-code `posix_spawn` control | causal proof | With the exec-tool parent, the Codex child inherited responsible PID 87737 (`/Applications/ChatGPT.app`, signed identifier `com.openai.codex`) and succeeded. Disclaiming responsibility made Codex self-responsible and reproduced the stall. Under LaunchServices, both Pax Workbench responsibility and Codex self-responsibility stalled. Private responsibility SPI is therefore neither a supported nor effective fix. |
| 2026-07-22 | Apple TN3179 and local network policy readback | blocking platform boundary | macOS Local Network Privacy attributes helper traffic to responsible code. The current signed OpenAI app identity is allowed, while `com.pax.buildrightstudio` and disposable probes have denied policy state. The supported route requires `NSLocalNetworkUsageDescription`, stable Apple-issued signing, explicit user consent/onboarding, and denied-state handling; this exceeds Task 011's approved spawn/controller scope. A grant was not approved or tested. |
| 2026-07-22 | `security find-identity -v -p codesigning` | historical blocker, resolved | The original audit found zero valid identities. It is preserved as blocker history and superseded by the later Apple Development identity proof below. |
| 2026-07-22 | Founder selected Route A | approved | Preserve direct `codex exec`; add development signing, `NSLocalNetworkUsageDescription`, consent onboarding, and denied-state handling. Production signing, notarization, publishing, and distribution remain excluded. |
| 2026-07-22 | `security find-identity -v -p codesigning` | pass | One valid identity found: SHA-1 `2CDCC44AEF9DAFA7002069CCA320F211147B7911`, `Apple Development: ndreipoppa@gmail.com (D2TWS575C3)`. |
| 2026-07-22 | `codesign --force --sign 2CDCC44AEF9DAFA7002069CCA320F211147B7911 --timestamp=none <temporary-rust-probe>`; `codesign --verify --strict --verbose=4 <temporary-rust-probe>` | pass | A real Rust executable signed successfully and verified through `Apple Worldwide Developer Relations Certification Authority` to `Apple Root CA`, with TeamIdentifier `6DNPZ54Z8L`. The temporary probe was deleted after verification. |
| 2026-07-22 | `bun run check`; `cargo test --manifest-path src-tauri/Cargo.toml` | pass | Frontend typecheck, 40 tests, and production build passed; all 120 Rust tests passed. The added UI test covers the Local Network repair path after a failed live run. |
| 2026-07-22 | `bun scripts/build-signed-macos.ts`; `plutil -p <app>/Contents/Info.plist`; `codesign --verify --deep --strict --verbose=4 <app>` | pass | The debug app contains `NSLocalNetworkUsageDescription`, uses identifier `com.pax.buildrightstudio`, and verifies through Apple Development, WWDR, and Apple Root with TeamIdentifier `6DNPZ54Z8L`; binary SHA-256 `a5be24fa841ee50230b2e9814b3c9737f6a4829ad1b0753b15d8e3d84bf38722`. Gatekeeper rejection is expected for the explicitly excluded non-notarized development build. |
| 2026-07-22 | Signed compiled-app restart against `/tmp/pax-workbench-task011-live.MhYsZH/repo` | pass | The app reconstructed the interrupted nonterminal receipt at event cursor 0, displayed `automaticExecutionStarted: false`, reread the resolver, selected only Task 900, and required a fresh confirmation. |
| 2026-07-22 | Signed compiled-app Task 900 trials from the build artifact and `/Users/pax/Applications/Build Right Studio.app` | failed acceptance | Both separately confirmed runs started one cancellable Codex child but produced no provider event. Both were cancelled and reaped; `loop-proof-one.txt` remained absent, Task 900 stayed `ready`, Task 901 stayed `planned`, and the durable receipt remained nonterminal at event cursor 0. |
| 2026-07-22 | System Settings Local Network readback; `/usr/bin/tccutil reset LocalNetwork com.pax.buildrightstudio` | source mismatch / failed repair | Build Right Studio did not appear among the 17 Local Network applications after either signed run, including from a stable user Applications path. The targeted reset returned exit 70. Signing and purpose-string prerequisites are proved, but the expected consent registration did not occur, so Local Network denial is no longer sufficient as the confirmed cause. |
| 2026-07-22 | Founder authorization to reopen Task 011 diagnostics | approved | Reclassify the signed child-initialization diagnosis as ready AI-owned work inside Task 011. Prior failed acceptance remains authoritative and Task 012 stays dependency-blocked. |
| 2026-07-22 | Signed LaunchServices restart through `/Users/pax/Applications/Build Right Studio.app` | pass | The unchanged signed artifact completed the generic read-only Codex invocation with `executed: true`, exit 0, Codex CLI 0.144.4, normalized events, and no provider-authority advancement. Durable UI evidence: `output/native/task-011-signed-runtime-restart.jpeg` (SHA-256 `fcfd5ddfd4531c4b5e1a7f4996e34d7e19d2ff90a2c1f73435a95ff2232ff9f3`). |
| 2026-07-22 | Native disposable-repository loop at `/tmp/pax-workbench-task011-live.XKAbqT/repo` | pass | Two distinct UI confirmations started exactly one Task 900 and one Task 901 invocation. The first stopped at `continueAvailable` with only Task 901 selected; the second stopped at `goalComplete` with post-exit resolver `no-ready-task`. Both repository tasks and the disposable sprint are complete. Durable UI evidence: `output/native/task-011-native-goal-complete.jpeg` (SHA-256 `b9e8fd84da416a22081c8c599cdaaedfba3e01e8b021b08db595772b4de06817`). |
| 2026-07-22 | Root disposable-repository verification and process audit | pass | Both task-prescribed proof checks passed, task/tracker statuses were complete, `git diff --check` passed, strict resolver returned `no-ready-task`, and an exact `ps` audit found no Codex wrapper or native child remaining. Task 901's proof is a conventional LF-terminated line; no 22-byte or no-newline claim is made. |
| 2026-07-22 | `bun run check`; `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | Frontend typecheck, 40 tests, and production build passed; 120 Rust tests, native compile, and format check passed. |
| 2026-07-22 | Independent Sol/high closure review | pass | Approved Task 011 closure after verifying the signed artifact, restart screenshot, exactly two native decisions and `thread.started` events, `continueAvailable` then `goalComplete`, terminal receipt at Task 901/event cursor 58, repository proof, resolver stop, and child cleanup. The review supplies the independent status/evidence audit missing from the disposable child run. |

## Files Changed

- `src-tauri/src/lib.rs` - typed loop/persistence/finalization logic, single-use
  confirmations, closed bounded argv, async controller worker, and production
  lifecycle/concurrency regressions.
- `src/types.ts` - typed loop and terminal contracts.
- `src/App.tsx` - explicit review/confirm/continue/stop UI and durable recovery.
- `src/App.test.tsx` - confirmation, loop, terminal, and restart projections.
- `src-tauri/Info.plist` - macOS Local Network purpose string merged into the bundle.
- `scripts/build-signed-macos.ts` - resolves a valid machine-local Apple
  Development identity and passes it to Tauri without persisting personal
  certificate data in repository configuration.
- `package.json` - explicit signed macOS development-build command.
- `output/native/task-011-signed-runtime-restart.jpeg` - durable signed generic-runtime restart proof.
- `output/native/task-011-native-goal-complete.jpeg` - durable two-confirmation goal-complete proof.
- `tasks/issues/011-run-confirmed-goal-loop.md` - implementation, review,
  verification, native boundary, and blocker evidence.
- `tasks/sprint-1.md`, `docs/blueprint-status.md`, and
  `docs/release-gates.md` - terminal Task 011 evidence and Task 012 promotion.

## Verification Summary

- Automated implementation, full verification, and independent review gates pass.
- The unchanged Apple-development-signed artifact completed a generic read-only
  restart and two separately confirmed native task iterations.
- The first iteration stopped at `continueAvailable`; the second stopped at
  repository-affirmed `goalComplete` and `no-ready-task`. No hidden retry,
  third task, provider authority promotion, or remaining Codex child was found.
- The earlier signed pre-event stalls remain truthful historical evidence. The
  external/environmental change that cleared them is unknown, so no code or
  Local Network repair claim is made.

## Learning Notes

- Proved: resolver/gate precedence, single-use confirmation, checkpoint
  carry-forward, atomic cancellation/finalization, signed restart, two real
  separately confirmed iterations, repository-authoritative verification, and
  terminal goal stop.
- Real: unchanged signed installed app, real Codex CLI 0.144.4 processes, a
  disposable Git repository, two proof artifacts, durable checkpoints, exact
  post-exit resolver decisions, and child cleanup.
- Manual: project selection, the generic live confirmation, and both task
  confirmations were driven through Computer Use against the signed app.
- Simulated: terminal-family breadth remains production-seam Rust coverage; the
  successful two-iteration path is now additionally proved natively.
- Residual: Task 901's proof uses the contract's semantic shell comparison and
  includes a conventional LF terminator; it is not claimed as a 22-byte literal.
  The cause of the earlier pre-event stalls remains unknown.

## Skill Trial Notes

- Source comparison: project-scoped installed skills
- Contract markers checked: resolver decision, confirmation, one task, checkpoint, stop states
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 012 owns end-to-end proof and release evidence.
