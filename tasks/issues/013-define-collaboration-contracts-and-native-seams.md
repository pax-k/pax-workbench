# 013: Define Collaboration Contracts And Native Controller Seams

Status: complete
Type: architecture
Owner: AI

Assumption basis: founder-claimed plus repo-evidence-backed
Requirement basis: docs/ha2ha-mdsync-reconciliation.md; docs/evidence/sprint-2-current-implementation-review.md
Reversibility: easy
Learning objective: prove the completed bounded controller can accept optional collaboration policy without coupling local execution to MDSync or widening authority
Source under test: repo-local path

## Goal

Define typed provider-neutral collaboration contracts and extract the smallest
native controller/UI seams needed by Sprint 2 while preserving all Sprint 1
behavior and tests.

## Non-Goals

- Make network requests.
- Parse or retain capability URLs.
- Create a remote workspace or claim a task.
- Rewrite all of `src-tauri/src/lib.rs` or `src/App.tsx`.
- Change Build Right task formats or helper behavior.

## Required Reading

- docs/ha2ha-mdsync-reconciliation.md
- docs/evidence/sprint-2-current-implementation-review.md
- docs/execution-rules.md
- tasks/issues/009-execute-one-bounded-task.md
- tasks/issues/010-persist-checkpointed-goal-state.md
- tasks/issues/011-run-confirmed-goal-loop.md

## Acceptance Criteria

- [x] A provider-neutral collaboration port defines disabled/local-only,
      viewer, and shared-collaborator modes without naming MDSync in the core
      controller.
- [x] Typed contracts cover sanitized session metadata, local source binding,
      remote task binding, claim result, evidence/handoff result,
      reconciliation state, repair hint, and failure class.
- [x] The core model keeps local task SHA/Git fingerprint separate from remote
      integer `baseVersion`.
- [x] Collaboration state cannot contain tokens, authorization headers,
      capability-bearing URLs, remote file bodies, or provider payloads.
- [x] The bounded controller exposes explicit pre-run and post-local-commit
      collaboration hooks with deterministic no-op implementations.
- [x] Existing controller, goal, runtime, project, and UI behavior remains
      compatible; the extraction is limited to modules touched by Sprint 2.
- [x] Tests prove local solo mode invokes no collaboration effect and produces
      the same terminal decisions as the Sprint 1 baseline.
- [x] Package/module dependency direction is documented and checked where a
      local test can enforce it.

## Baseline Evidence

`bun run check` passes 40 frontend tests and the production build; `cargo test
--manifest-path src-tauri/Cargo.toml` passes 120 tests. Collaboration types,
ports, and hooks do not exist. The native and UI roots are 12,819 and 1,252
lines respectively.

## Solution-Fit Rationale

- Requirement served: create an explicit seam for optional shared execution.
- Constraints honored: repository/Build Right authority and offline solo mode.
- Guarantees preserved: one-use confirmation, typed failures, no hidden state,
  and repository-evidence completion.
- Cost accepted: a small contract/module extraction before behavior is added.
- Deferred capability: concrete transport, remote mutation, and UI.

## Verification

- Focused Rust contract/controller tests.
- Focused TypeScript contract/projection tests.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml collaboration -- --nocapture`; `bun run test -- src/lib/collaboration.test.ts` | pass | Twelve focused Rust tests and fourteen focused TypeScript tests cover local-only no-effect behavior, typed local session/evidence/handoff handles, closed missing-effect/repair/failure variants, opaque capability redaction/non-serialization, unsafe adapter-output/failure/handoff rejection, explicit production local-only policy, version separation, and dependency direction. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass | 132 Rust tests passed; native compilation and formatting passed. Existing bounded-controller terminal coverage remained green. |
| 2026-07-23 | `bun run check` | pass | Typecheck, 54 frontend tests, and the production Vite build passed. |
| 2026-07-23 | `bun run tauri build --debug --bundles app` | pass | Fresh current-source macOS debug application bundle compiled successfully. |
| 2026-07-23 | Independent Sol/high closure review | findings repaired | F-013-01 identified incomplete secret/capability validation; F-013-02 identified a hardwired no-op collaboration port. Validation now covers arbitrary capability URLs, sensitive field classes, bounded serialized adapter outcomes, and unsafe adapter output. The controller now accepts an injected `CollaborationPort` and `ControllerCollaborationPolicy`, while the production wrapper supplies the deterministic local-only default. |
| 2026-07-23 | Independent Sol/high closure re-review | findings repaired | F-013-01 still allowed opaque material in nominally sanitized strings; F-013-02R found the wrapper defaulted to `Disabled`; F-013-03 found premature blueprint completion language. Capability material is now a distinct native-only redacted/non-serializable type, session metadata requires a locally minted typed handle, repair instructions are closed product-owned variants, the default is explicitly `LocalOnly`, and blueprint truth matches the tracker. |
| 2026-07-23 | Independent Sol/high third closure review | finding repaired | F-013-01 found a remaining capability extraction callback and crate-public free-form adapter failure strings. Capability material now has no general extraction API, and external ports can construct only fixed typed failure variants with private fields and product-owned messages. An adversarial opaque adapter-failure test proves the boundary returns a safe internal failure instead of the secret. |
| 2026-07-23 | Independent Sol/high fourth closure review | finding repaired | F-013-01B found free-form successful handoff identifiers and missing-effect strings. Evidence and handoff references are now validated nominal locally minted types, missing effects are a closed enum, and Rust/TypeScript adversarial tests reject opaque successful-output values. |
| 2026-07-23 | Independent Sol/high fifth closure review | approved | No critical, high, or medium findings remain. The reviewer verified nominal handoff identifiers, closed outputs/failures, capability isolation, explicit local-only production policy, authority coherence, all focused tests, and the final native artifact. |
| 2026-07-23 | Shell-launched repaired debug bundle, repository selection, Task 013 preview, and deterministic controller fixture | pass | Native app selected only Task 013 through the injected collaboration boundary, started no network/remote effect, refreshed snapshot/Git/task evidence/resolver/stop gates, and stopped at the expected `verificationFailed` because closure review remained pending. The run proved no second task started, and its orchestration receipt was explicitly discarded without repository mutation. |
| 2026-07-23 | `output/native/task-013-injected-port-native-smoke.jpeg` | pass | 1229x768 current-source screenshot, SHA-256 `d7640c1ec95805761ad39d3fa8357157b151542581f30d2fb60e7298a46c930d`, showing one selected Task 013 fixture, full refresh evidence, repository-authoritative failure stop, and no second task. |
| 2026-07-23 | `output/native/task-013-typed-capability-native-smoke.jpeg` | pass | 1229x768 current-source screenshot, SHA-256 `5e7bbeb179743565e81330328f07d637fc7a1274de233185ce98ecd2f81b47cc`, showing one Task 013 fixture through the explicit local-only production policy, all five refreshed authority surfaces, a truthful pre-closeout failure stop, and no second task. |
| 2026-07-23 | `output/native/task-013-closed-failure-boundary-native-smoke.jpeg` | pass | 1229x768 current-source screenshot, SHA-256 `847da4660ba0d00847676a39923edda5dec0ffc81c33f9f2085204f7a1b63f8a`, showing exact Task 013 selection after closing the last free-form failure boundary, all five refresh surfaces, and no second task. |
| 2026-07-23 | `output/native/task-013-all-output-boundaries-native-smoke.jpeg` | pass | 1229x768 current-source screenshot, SHA-256 `c79ecff2ded5c1561bb59ef1b502a333bd488fd8488582835287b1473a52fe94`, showing exact Task 013 selection after nominalizing successful handoff output, all five refreshed authority surfaces, and no second task. |

## Files Changed

- `src-tauri/src/collaboration.rs` - provider-neutral contracts, validation,
  no-op port, and dependency enforcement tests.
- `src-tauri/src/lib.rs` - explicit local pre-run and post-local-commit hook
  seams around the existing bounded controller.
- `src/lib/collaboration.ts` - sanitized frontend projection contracts.
- `src/lib/collaboration.test.ts` - local-only, redaction, and version-domain tests.
- `docs/decision-log.md` - accepted module ownership and dependency direction.
- `output/native/task-013-injected-port-native-smoke.jpeg` - repaired
  current-source native smoke evidence.
- `output/native/task-013-typed-capability-native-smoke.jpeg` - typed
  capability-boundary and explicit local-only native smoke evidence.
- `output/native/task-013-closed-failure-boundary-native-smoke.jpeg` - final
  current-source native fixture after closing adapter failure construction.
- `output/native/task-013-all-output-boundaries-native-smoke.jpeg` - final
  current-source fixture after closing successful output construction.
- `tasks/issues/013-define-collaboration-contracts-and-native-seams.md`
- `tasks/sprint-2.md`
- `docs/blueprint-status.md`
- `docs/release-gates.md`

## Verification Summary

- Focused native/TypeScript collaboration contracts: pass, 26 tests.
- Full Rust regression: pass, 132 tests; compile and format pass.
- Full frontend regression: pass, 54 tests plus typecheck and production build.
- Fresh macOS debug bundle: pass.
- Native local-only controller smoke: pass for exact Task 013 selection,
  deterministic no-effect pre-run hook, all repository refresh surfaces, and
  one expected pre-closeout failure stop.

## Learning Notes

- Proved: the existing bounded controller now crosses typed pre-run and
  post-local-commit collaboration seams through an injected port/policy while
  local solo mode performs no collaboration effect and preserves Sprint 1
  terminal behavior.
- Proved: opaque capability material is structurally excluded from serializable
  collaboration state, while safe session handles and repair instructions are
  constructed through narrower product-owned types.
- Real: current native bundle, installed helper execution, repository
  selection, resolver/task-contract preview, controller refresh, and
  repository-authoritative stop.
- Simulated: shared session, remote task, claim, evidence/handoff, and repair
  outcomes remain contract shapes only; no network request ran.
- Test next: native discovery, access, capability lifetime, bounded HTTP, and
  redaction behavior in Task 014.

## Skill Trial Notes

- Source under test: repo-local path
- Source comparison: project-scoped installed skills
- Contract markers checked: authority, ports, state, failures, tests, evidence
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 014 implements the first concrete transport and is promoted to `ready`.
