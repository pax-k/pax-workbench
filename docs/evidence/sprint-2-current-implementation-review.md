# Sprint 2 Current Implementation Review

Date: 2026-07-22
Source mode: repository evidence
Scope: completed Sprint 1 implementation and HA2HA/MDSync integration readiness

## Outcome

Sprint 1 is a technically proved local control loop, not a prototype-only UI.
The existing native controller is the correct integration point for Sprint 2.
HA2HA must be added as optional portable collaboration state and MDSync as its
optional hosted transport; neither may replace Build Right or repository files
as planning and completion authority.

## Baseline Verification

| Check | Result | Evidence |
| --- | --- | --- |
| Frontend typecheck, tests, and production build | pass | `bun run check`; 40 tests passed; Vite production build succeeded |
| Native tests | pass | `cargo test --manifest-path src-tauri/Cargo.toml`; 120 tests passed |
| Sprint 1 status | pass | `tasks/sprint-1.md` is complete and tasks 003-012 are terminal |
| Whole-loop native proof | pass | `docs/evidence/manual-trials.md`; signed two-confirmation restart/resume trial |
| Git source state | caution | repository has no commits and all current files are untracked |

## Technical Feasibility Sources Reviewed

The planning helper identified a public-integration feasibility trigger. This
review resolved it from the current local checkout of the public
`pax-k/ha2ha-mdsync` source rather than general web material:

- `/Users/pax/Documents/robosync/docs/v1/ha2ha-protocol.md`
- `/Users/pax/Documents/robosync/docs/v1/workspace-conventions.md`
- `/Users/pax/Documents/robosync/docs/v3/collaboration-protocol.md`
- `/Users/pax/Documents/robosync/docs/v3/decisions/V3-DR-005-engineering-profile-and-provider-boundary.md`
- `/Users/pax/Documents/robosync/docs/v3/decisions/V3-DR-006-skills-scripts-and-harness-adapters.md`
- `/Users/pax/Documents/robosync/packages/ha2ha-client/src/**`
- `/Users/pax/Documents/robosync/packages/mdsync-client/src/**`
- `/Users/pax/Documents/robosync/skills/ha2ha/**`
- `/Users/pax/Documents/robosync/skills/mdsync/**`
- `/Users/pax/Documents/robosync/docs/v2/tasks/V2-012-url-based-ha2ha-agent-handoff.md`

The reviewed checkout records current live URL-only handoff, Viewer denial,
Collaborator claim/evidence, `version_conflict`, capability redaction,
revocation, and multi-context dogfood evidence. Workbench Task 019 still
requires exact-source/deployment readback because external hosted state can
change after this planning review.

## Reusable Implementation Boundaries

- `OperationRegistry` serializes local effects and provides one linearization
  point for cancellation versus terminal result commitment.
- Bounded-task confirmation tokens are single-use and bind execution to a
  revalidated resolver/task baseline.
- The controller reruns Build Right helpers before execution and after provider
  exit, then classifies repository evidence instead of trusting provider output.
- Goal persistence stores a bounded objective, repository identity, run cursor,
  verified checkpoint, and evidence hashes without copying task/planning state
  or provider payloads.
- Project Markdown writes use content-derived SHA-256 versions and reject stale
  or path-swapped writes.
- Runtime events retain raw provider payloads for local inspection while
  `repositoryAuthorityAdvanced` remains false.

## Integration Gaps

### Native responsibility seams

`src-tauri/src/lib.rs` is 12,819 lines and currently owns repository I/O, skill
setup, helper execution, runtime execution, goal persistence, bounded control,
and most tests. `src/App.tsx` is 1,252 lines and owns the corresponding UI.
Both are green, but adding network sessions, capability handling, remote
reconciliation, and repair directly to those files would create unrelated
reasons to change. Sprint 2 should extract only the collaboration-facing
controller and UI seams before adding behavior; it should not perform a broad
rewrite.

### Separate concurrency domains

Local project files use content hashes such as `sha256:<digest>`. HA2HA/MDSync
files use integer versions and `baseVersion`. These are independent contracts:

- local task source binding uses task SHA-256 plus Git fingerprint;
- remote coordination uses workspace id, path, actor, and integer version;
- no conversion between the two version types is valid.

### Package/runtime boundary

The current `@ha2ha/client` local transport and shared client code use Node APIs
including `node:fs`, `node:path`, and `node:crypto`. `@mdsync/client` consumes
that package. Neither should be imported into the Tauri WebView. Sprint 2 should
implement the documented HTTP/discovery contract behind a native Rust port and
validate it against MDSync fixtures/live conformance. Upstream package splitting
may remain a separate HA2HA concern; it is not required to prove the workbench
integration.

### Capability security

MDSync Viewer and Collaborator URLs are bearer capabilities. The workbench has
no current token/session boundary. Sprint 2 must keep capability material in a
native in-memory session, redact it from typed errors and UI events, and exclude
it from repository files, goal state, evidence, logs, diagnostics, and runtime
prompts.

### Source provenance

The current repository has no commit. A collaboration envelope may truthfully
record canonical source path, task path/hash, dirty state, and nullable Git
HEAD, but it must not invent a branch/commit proof. A later commit can improve
portability without changing the protocol contract.

## Final Review Decision

Proceed with an optional shared execution mode layered around the existing
bounded controller:

1. Local solo mode remains unchanged and requires no network.
2. Shared mode connects to or publishes one MDSync-hosted HA2HA workspace.
3. Only the resolver-selected executable task receives an HA2HA execution
   envelope; the full Build Right backlog is not mirrored.
4. A remote claim guarded by `baseVersion` must succeed before Codex starts.
5. Local verification and checkpoint commitment remain authoritative.
6. A failed post-commit remote update creates repair-required reconciliation
   debt and blocks another shared iteration; it never rolls back local truth.
7. Multi-agent coordination is proved through independent clients, while
   multi-agent orchestration remains excluded.

## Residual Risks

- The first Rust HTTP adapter may drift from public TypeScript SDK behavior;
  contract fixtures and a live dogfood gate are required.
- A remote claim can succeed immediately before a local pre-spawn failure; the
  repair contract must leave an explicit handoff/release path.
- MDSync availability is external state. Local solo mode must remain usable
  during outages.
- Customer value remains unproved; Sprint 2 is a technical integration sprint.
