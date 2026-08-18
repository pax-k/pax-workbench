# 014: Implement Secure Native MDSync Session Transport

Status: complete
Type: integration
Owner: AI

Assumption basis: founder-claimed plus repo-evidence-backed
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; tasks/issues/013-define-collaboration-contracts-and-native-seams.md
Reversibility: moderate
Learning objective: prove capability-bearing MDSync discovery and file operations can remain native, bounded, typed, and secret-free at the UI/persistence boundary
Source under test: repo-local workbench plus public MDSync HTTP contract

## Goal

Implement a native Rust MDSync session adapter that safely parses a pasted
workspace URL, validates discovery/access, retains capabilities only in memory,
and exposes sanitized HA2HA file operations through the collaboration port.

## Non-Goals

- Import `@ha2ha/client` or `@mdsync/client` into the WebView.
- Persist tokens or reconnect automatically after restart.
- Publish a Build Right envelope.
- Change the public MDSync service.
- Add provider-specific GitHub, Jira, CI, or deploy synchronization.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- tasks/issues/013-define-collaboration-contracts-and-native-seams.md
- `/Users/pax/Documents/robosync/packages/mdsync-client/src/url-bootstrap.ts`
- `/Users/pax/Documents/robosync/packages/mdsync-client/src/client.ts`
- `/Users/pax/Documents/robosync/docs/v2/tasks/V2-012-url-based-ha2ha-agent-handoff.md`

## Acceptance Criteria

- [x] Native URL parsing accepts supported HTTPS workspace routes and explicit
      localhost HTTP only; ambiguous/empty capabilities and origin mismatches
      fail closed.
- [x] Discovery rejects redirects, malformed/oversized JSON, unsupported
      versions, and unexpected Web/API origins.
- [x] Viewer/public/edit access is represented explicitly and enforced before
      mutation.
- [x] Capability material exists only in a native in-memory session addressed
      by an opaque id and is destroyed on disconnect/project switch/app exit.
- [x] Sanitized session results expose workspace id, origins, access class, and
      actor but never query strings, authorization values, or tokens.
- [x] Read/list/write operations use bounded timeouts and response sizes,
      validate result shapes, and preserve typed `version_conflict` details.
- [x] Every error/log/debug/result path passes capability-leak tests.
- [x] Deterministic fixtures match the current public client discovery and
      authorization contract before any live request is attempted.

## Baseline Evidence

The workbench has no network dependency, native HTTP adapter, remote session,
or capability type. The public MDSync client already defines strict URL
discovery, Viewer/Collaborator access, and `baseVersion` behavior.

## Solution-Fit Rationale

- Requirement served: connect the native workbench to hosted HA2HA workspaces.
- Constraints honored: no Node code in WebView and no secret persistence.
- Guarantees preserved: least privilege, explicit effects, typed failures, and
  offline local mode.
- Cost accepted: maintain a small native adapter against a public HTTP contract.
- Deferred capability: keychain persistence and general provider plugins.

## Verification

- Native URL/discovery/access/timeout/size/redaction contract tests.
- Deterministic mock-server tests for read, write, denial, and conflict.
- Capability scans over serialized types, logs, goal state, and UI messages.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | GPT-5.6 router v0.3 task profile | pass | Security/auth/secrets/distributed-state implementation routed to `gpt56_router_sol_engineer`, GPT-5.6 Sol/medium, with independent Sol/high review required. |
| 2026-07-23 | Public `mdsync-client` contract review at clean commit `ebd5c8d483a26096f95fdcc8e4f5242270481e9b` | pass | Compared `url-bootstrap.ts`, `client.ts`, `request.ts`, `parsers.ts`, `errors.ts`, and `types.ts`; deterministic wire fixtures match discovery v1, access, authorization, file, and conflict schemas. |
| 2026-07-23 | First independent GPT-5.6 Sol/high security review | fail closed | Closure denied for two high findings (untrusted API capability forwarding and project-switch races) and two medium findings (incomplete temporary zeroization and unbounded URL/capability input). Task 015 remained planned. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml mdsync_transport -- --nocapture` | pass | Thirteen real loopback/unit tests cover the original transport contract plus exact production origin-pair pinning, attacker-to-private API rejection, bounded raw/decoded capability input, a slow-connect selection-generation race, a slow-write project-switch lease, and query-bearing URL zeroization/redaction. No hosted request ran. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | 145 Rust tests passed; native compilation and formatting passed. Forward-declared Sprint 2 contract variants retain non-failing dead-code warnings until Tasks 015-017 consume them. |
| 2026-07-23 | `bun run check` | pass | Typecheck, 54 frontend tests, and the production Vite build passed. |
| 2026-07-23 | `bun run tauri build --debug --bundles app` | pass | Fresh current-source macOS debug application bundle compiled with the Rustls-backed bounded native HTTP adapter. |
| 2026-07-23 | Current debug bundle, repository selection, Task 014 resolver preview, and deterministic controller fixture | pass | Native app selected only Task 014, preserved local-solo behavior, refreshed all five repository authority surfaces, and started no second task. The expected pre-closeout `verificationFailed` remained repository-authoritative. |
| 2026-07-23 | `output/native/task-014-native-local-solo-smoke.jpeg` | pass | 1229x768 screenshot, SHA-256 `a2351148a94f4b3add3b10a6d9fd107f771171f9d0d4dd19262595cb0f2c4f7c`, from the fresh current-source bundle. |
| 2026-07-23 | Fresh repaired-source native executable, Task 014 resolver preview, and deterministic controller fixture | pass | Reproduced exact Task 014 selection, `verificationFailed`, all five refresh surfaces (`snapshot`, `git`, `taskEvidence`, `resolver`, `stopGates`), `failureStop`, and “No second task started.” |
| 2026-07-23 | `output/native/task-014-security-repair-native-smoke.jpeg` | pass | 1229x768 JPEG screenshot, SHA-256 `14c9c902427440f98d6bc57cd0acc1dc789d5bd802b9593e20803bb810280ed8`, captured from the fresh repaired-source native executable. |
| 2026-07-23 | Second independent GPT-5.6 Sol/high security review | approved | No unresolved critical, high, or medium findings. F-014-01 through F-014-04 were verified closed; tests, upstream provenance, native artifact, and authority state were independently checked. Hosted conformance remains explicitly deferred to Task 019. |

## Files Changed

- `src-tauri/src/mdsync_transport.rs` - strict native URL/discovery/session/file
  adapter, sanitized result/error types, lifecycle management, and loopback
  contract fixtures.
- `src-tauri/src/collaboration.rs` - validated constructors/accessors needed by
  native session metadata without widening capability access.
- `src-tauri/src/lib.rs` - native session store, project-switch cleanup, and
  bounded Tauri connect/disconnect/list/read/write commands.
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `output/native/task-014-native-local-solo-smoke.jpeg`
- `output/native/task-014-security-repair-native-smoke.jpeg`
- `tasks/issues/014-implement-secure-native-mdsync-session-transport.md`
- `docs/blueprint-status.md`
- `docs/release-gates.md`

## Verification Summary

- Focused native transport: pass, 13 real loopback/unit tests, including all
  four adversarial repair families from the first security review.
- Full Rust regression: pass, 145 tests; compile and format pass.
- Full frontend regression: pass, 54 tests plus typecheck and production build.
- Fresh macOS debug bundle and resolver-selected Task 014 local-solo smoke:
  pass.
- Hosted MDSync: intentionally not run; Task 019 owns capability-bearing live
  acceptance after Tasks 015-018 complete.

## Learning Notes

- Proved: native URL/discovery/access/timeout/overflow/session lifecycle and
  file-operation contracts against real loopback HTTP, plus current-source
  native bundle compatibility.
- Proved: the command-owned URL, query-bearing parsed URL backing allocation,
  decoded capability values, stored capability, and bearer construction
  temporary are zeroized; authorization headers are marked sensitive.
  Capability values are absent from serialized metadata, results, errors, and
  debug output. The unavoidable external IPC/deserializer allocation before
  the command boundary is outside this module's ownership and is not claimed
  as zeroized.
- Simulated: fixture responses model the public MDSync contract; no hosted
  capability-bearing request ran.
- Test next: publish/join a valid Build Right execution envelope.

## Skill Trial Notes

- Source comparison: public MDSync contract at clean commit
  `ebd5c8d483a26096f95fdcc8e4f5242270481e9b`.
- Contract markers checked: discovery, access, redaction, baseVersion, failures
- Router result: Sol/medium engineer with Sol/high independent review.
- Trial status: first review denied closure; all four findings were repaired,
  and the second independent Sol/high review approved closure with no
  medium-or-higher findings.

## Blockers

- None.

## Follow-Ups

- Task 015 owns envelope creation and reconciliation.
