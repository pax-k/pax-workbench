# 008: Implement Codex Runtime Adapter

Status: complete
Type: integration
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/raw/product-discussion.md; docs/mvp-scope.md
Reversibility: easy
Learning objective: establish whether one Codex JSONL invocation can be controlled and observed through a stable local contract
Source under test: repo-local path

## Goal

Implement one Codex adapter that invokes `codex exec --json -C <project>` and
normalizes JSONL, stderr, exit, cancellation, and malformed events into an
app-owned runtime event contract.

## Non-Goals

- Orchestrate multiple agents.
- Automatically select or execute a task.
- Support additional agent providers.
- Treat an agent success message as repository proof.

## Required Reading

- docs/execution-rules.md
- docs/raw/product-discussion.md
- tasks/issues/005-complete-safe-repository-session.md
- tasks/issues/007-implement-deterministic-helper-execution.md

## Acceptance Criteria

- [x] A provider-neutral runtime port defines start, event stream, cancel,
      terminal result, and capability/error semantics.
- [x] The Codex adapter passes project path and prompt as explicit arguments,
      with no shell interpolation.
- [x] JSONL fixtures cover representative events, unknown event types, malformed
      lines, stderr, nonzero exit, bounded buffers, and cancellation.
- [x] Raw provider payload remains inspectable while UI consumers receive stable
      normalized events.
- [x] Runtime version and exact invocation are captured in evidence.
- [x] The UI can start a clearly labeled dry/fixture run and a user-confirmed
      live run without conflating the two.
- [x] No tracker, task, or release state advances from provider self-report.

## Baseline Evidence

The repository has no runtime adapter. Agent events and checkpoint progression
come only from static demo fixtures.

Read-only reconciliation against the installed Codex CLI establishes the first
closed live invocation contract:

- runtime: `/Users/pax/.nvm/versions/node/v24.14.0/bin/codex`
- version observed: `codex-cli 0.144.4`
- argv: `codex exec --json --ephemeral --ignore-user-config --sandbox read-only
  --color never -C <canonical-project-root> -- <bounded-confirmed-prompt>`

The native adapter owns the executable and every option token. The selected
repository supplies only its already validated canonical root; the frontend may
supply one length-bounded prompt as a single argument after explicit live-run
confirmation. No shell, config override, additional directory, model/provider,
dangerous bypass, or arbitrary flag crosses the boundary. Fixture/dry mode does
not spawn Codex. Live output is JSONL and provider self-report cannot advance
repository task, tracker, or release authority.

## Solution-Fit Rationale

- Requirement served: execute exactly one bounded task through Codex.
- Constraints honored: replaceable provider adapter and explicit live invocation.
- Guarantees preserved: raw evidence, cancellation, structured failures, no self-claim authority.
- Cost accepted: provider event compatibility fixtures.
- Deferred capability: additional providers and multi-agent orchestration.

## Verification

- Provider-neutral port and Codex fixture tests.
- Malformed, cancel, failure, and output-bound tests.
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run check`
- One no-mutation live adapter smoke with exact version recorded.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | `command -v codex`; `codex --version` | pass | Resolved `/Users/pax/.nvm/versions/node/v24.14.0/bin/codex`; observed `codex-cli 0.144.4`. |
| 2026-07-21 | `codex exec --help` | pass | Verified `--json`, `--ephemeral`, `--sandbox read-only`, `--color never`, `-C`, and explicit prompt argument; dangerous/config/provider expansion remains outside the closed adapter. |
| 2026-07-21 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 74/74 native tests passed, including provider terminals, cancellation/reaping, channel loss, raw UTF-8/hex evidence, 32 KiB helper bounds, and 256 KiB runtime JSONL bounds. |
| 2026-07-21 | `bun run check` | pass | Typecheck, 30/30 frontend tests, and production Vite build passed. |
| 2026-07-21 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `git diff --check` | pass | Rust formatting and whitespace checks were clean. |
| 2026-07-21 | `bun run tauri build --debug --bundles app` | pass | Built the compiled debug macOS app used for native acceptance. |
| 2026-07-21 | `output/native/task-008-native-runtime-fixture.jpeg` | pass | Compiled app emitted five deterministic simulated events with `executed=false`, no argv/spawn, a native run handle, and no authority advancement; SHA-256 `7924af7895b26952b38d51e53a4d3157cd822f8ee8062e96d478880f7797baf0`. |
| 2026-07-21 | First confirmed compiled-app live smoke | expected diagnostic failure | The closed process timed out at 120 seconds and was reaped. Investigation proved inherited user config introduced unrelated MCP/plugin startup; `--ignore-user-config` now preserves auth while excluding that surface. |
| 2026-07-21 | Second confirmed compiled-app live smoke | expected diagnostic failure | Corrected argv exited 0 but exceeded the helper-derived 32 KiB aggregate cap. The runtime now has a separate 256 KiB bound; helpers remain at 32 KiB. |
| 2026-07-21 | `output/native/task-008-native-runtime-live.jpeg`; `output/native/task-008-native-runtime-live-events.jpeg` | pass | Real authorized Codex run completed with exit 0, exact corrected argv, streamed normalized events, no remaining child, and `repositoryAuthorityAdvanced=false`; SHA-256 `22a08d69aa2379bf0b4c900d95f58990786af95310b533f684b84579285254ef` and `3648261ce035c23adec28843342930ef2acde0581b2cc0aad9940e003a0848be`. |
| 2026-07-21 | Independent Task 008 reviews | pass | Final reviews found no critical or medium findings after cleanup-priority, wire-format, inherited-config, and runtime-bound remediations. |

## Files Changed

- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/lib/bridge.ts`
- `src/App.tsx`
- `src/App.test.tsx`
- `output/native/task-008-native-runtime-fixture.jpeg`
- `output/native/task-008-native-runtime-live.jpeg`
- `output/native/task-008-native-runtime-live-events.jpeg`
- `tasks/issues/008-implement-codex-runtime-adapter.md`
- `tasks/issues/009-execute-one-bounded-task.md`
- `tasks/sprint-1.md`
- `docs/release-gates.md`
- `docs/blueprint-status.md`

## Verification Summary

- Native: 74/74 tests pass; canonical Rust formatting passes.
- Frontend: 30/30 tests, typecheck, and production build pass.
- Compiled app: deterministic fixture and authorized live JSONL run pass.
- Process lifecycle: final live child exited and no descendant remained.
- Authority: provider self-report changed no task, tracker, checkpoint, or release state.
- Review: independent final review reports no findings.

## Learning Notes

- Proved: a compiled native run can issue a native handle, stream bounded Codex
  JSONL, normalize terminal state, reap its process group, and preserve
  repository authority.
- Real: runtime/version probes, the final authenticated Codex invocation,
  process lifecycle, exact argv, normalized provider events, and screenshots.
- Manual: the user authorized authenticated sprint invocations; the compiled
  app repository picker and explicit live confirmation were exercised through
  Computer Use.
- Simulated: only the labeled dry fixture and its five deterministic events.
- Learned: user config is outside the closed runtime contract, so
  `--ignore-user-config` is native-owned; helper output and runtime JSONL need
  separate bounded ceilings.
- Residual risk: 256 KiB is an empirical fail-closed runtime ceiling; a larger
  legitimate future run returns `outputOverflow`. Non-Unix live execution
  remains explicitly unsupported.
- Test next: bind the adapter to one resolver-selected task in Task 009.

## Skill Trial Notes

- Source comparison: not applicable
- Contract markers checked: runtime version, argv, event schema, cancellation, exit
- Trial status: pass; fixture and real compiled-app runs remained visibly distinct

## Blockers

- None.

## Follow-Ups

- Task 009 owns task selection and evidence-backed completion.
