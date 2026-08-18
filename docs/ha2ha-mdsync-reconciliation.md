# HA2HA And MDSync Reconciliation

Status: accepted Sprint 2 architecture
Owner: founder + AI
Last updated: 2026-07-22

## Decision

Build Right, HA2HA, MDSync, Codex, and Build Right Studio keep separate
responsibilities:

| Layer | Authority |
| --- | --- |
| Repository Markdown, Git, and Build Right helpers | Product truth, task readiness, execution gates, verification, completion, and next-task selection |
| HA2HA | Portable actor, workspace, task-envelope, claim, version-conflict, evidence, decision, event, and handoff semantics |
| MDSync | Optional hosted HA2HA transport, capability access, file history, comments, activity, and browser inspection |
| Codex runtime adapter | Executes one confirmed local task and emits provider events; never advances authority by self-report |
| Build Right Studio | Reconciles local authority with optional remote collaboration, owns confirmation and repair UX, and stops at either contract's gate |

HA2HA is not a second planner. MDSync is not a cloud copy of the Build Right
backlog. Codex and other agents remain systems of action.

## Product Modes

### Local solo

The completed Sprint 1 behavior remains the default:

- no network or HA2HA workspace required;
- resolver-selected, explicitly confirmed, one-task execution;
- repository evidence controls completion;
- checkpoint recovery never starts work automatically.

### Shared HA2HA

The user explicitly publishes or joins an MDSync workspace and selects a stable
actor handle. Shared mode adds a collaboration gate around the same local
controller:

- Viewer access may inspect but cannot claim, write, or start shared execution.
- Collaborator access may claim and update the selected HA2HA task envelope.
- Capability material stays in the native process and is never sent to Codex.
- Disconnecting returns the project to local solo mode without changing local
  repository authority.

Shared mode coordinates independent human-agent contexts. It does not launch,
schedule, or supervise multiple agents.

## Execution Envelope

Only the current Build Right resolver-selected task is projected. Planned,
deferred, superseded, moved, split, and unrelated backlog rows remain local.

The portable HA2HA task contains core protocol fields plus a small extension:

```yaml
id: BR-013
title: Define collaboration contracts and native seams
state: ready
owner: null
updated_by: build-right-studio
build_right:
  source_path: tasks/issues/013-define-collaboration-contracts-and-native-seams.md
  source_sha256: <task-content-sha256>
  repository_id: <stable-local-repository-id>
  git_head: null
  git_dirty: true
  requirement_basis:
    - docs/ha2ha-mdsync-reconciliation.md
```

The Build Right extension is a workbench interoperability envelope, not a new
public HA2HA profile in Sprint 2. The protocol stays HA2HA v1-conformant. A
standardized profile should be proposed upstream only after dogfood proves the
fields and transitions.

## Identity And Version Binding

The shared execution binding contains:

- sanitized workspace id and origins;
- access class: `viewer`, `collaborator`, or `public`;
- stable actor handle;
- remote task path and integer file version;
- local task path and SHA-256;
- repository id and Git fingerprint, including nullable HEAD;
- confirmation preview token and run id;
- reconciliation state and repair hint.

Capability tokens are explicitly excluded.

The local and remote version domains remain separate:

```text
local source version = task SHA-256 + Git fingerprint
remote write version = HA2HA integer baseVersion
```

## Controller Sequence

```text
open repository
  -> run Build Right resolver/task-contract gates
  -> in shared mode read and reconcile remote envelope
  -> preview exact local + remote effects
  -> issue one-use confirmation bound to both baselines
  -> consume confirmation and rerun local gates
  -> claim remote task with baseVersion
  -> if conflict/error: stop before Codex
  -> run one Codex task
  -> refresh repository and rerun Build Right verification/stop gates
  -> commit local verified checkpoint
  -> append remote evidence/handoff/status with current baseVersion
  -> if remote sync succeeds: offer next confirmed iteration
  -> if remote sync fails: preserve local completion and stop repair-required
```

## Failure Semantics

| Failure point | Required result |
| --- | --- |
| Discovery, URL, access, or manifest invalid | No session; no token persistence; no Codex |
| Viewer attempts shared execution | Typed read-only stop; no remote mutation; no Codex |
| Local source changed after preview | Stale stop; no remote claim; no Codex |
| Remote `version_conflict` during claim | Conflict stop with sanitized latest coordinate; no retry beyond the protocol limit; no Codex |
| Remote claim succeeds but local start fails | Keep explicit claimed/handoff repair state; no hidden retry |
| Codex or local verification fails | Record/attempt a blocked handoff; local task does not complete |
| Local verification commits but remote evidence update fails | Local completion remains authoritative; mark `collaborationRepairRequired`; do not continue shared loop |
| App restarts with repair debt | Reconstruct local truth, fetch remote truth, show repair preview, and require explicit action |
| MDSync unavailable in local solo mode | No effect on local execution |

All remote mutations must be idempotent or safely repeatable from local
checkpoint identity. A repair may append missing evidence or reconcile status;
it must not rerun Codex.

## Native Security Boundary

- Parse pasted URLs and perform discovery in Rust.
- Allow HTTPS origins; allow HTTP only for explicit localhost development.
- Reject redirects, ambiguous `edit`/`k` capabilities, origin mismatch, empty
  capabilities, invalid workspace ids, and unsupported routes.
- Store capability material only in a native in-memory session indexed by an
  opaque session id.
- Return sanitized workspace/access metadata to React.
- Apply bounded timeouts, response-size limits, schema validation, and typed
  errors to every network call.
- Redact query parameters and authorization values before traces, tests, logs,
  evidence, provider prompts, and UI events.
- Clearing or switching projects destroys the active session.

## Persistence Boundary

Goal state may add a bounded optional collaboration cursor:

```text
workspace id
remote task path/version
actor handle
access class
reconciliation state
local task path/hash
last successful portable event/evidence ids
```

It must not store tokens, full URLs containing capabilities, remote file
contents, comments, provider payloads, or a shadow task status. Restart requires
the user to reconnect capability access before remote mutation.

## Package Boundary

Sprint 2 consumes the documented HA2HA/MDSync HTTP contract through a native
port. It does not import the Node-oriented SDK into the WebView and does not
fork public protocol semantics.

The native adapter must be verified against:

- deterministic fixtures matching `@mdsync/client` URL discovery and result
  contracts;
- HA2HA task/evidence/frontmatter fixtures;
- stale `baseVersion` and viewer-denial behavior;
- the deployed MDSync service in the final dogfood task.

Any required public protocol or server change belongs in the
`ha2ha-mdsync` repository first and must be released before this workbench pins
or claims it.

## Completion Authority

Shared-mode success requires both:

```text
Build Right: local task is repository-verified and tracker/gates are coherent
HA2HA: shared envelope/evidence is synchronized with no unresolved conflict
```

If only Build Right succeeds, the local task is complete but the shared loop is
not ready to continue. If only HA2HA succeeds, the local task is not complete.

## Deferred

- Whole-backlog mirroring or bidirectional planning sync.
- A public `ha2ha-build-right` profile.
- Provider-specific GitHub/Jira/CI/deployment synchronization.
- Persistent token/keychain support.
- Unattended, parallel, or multi-agent orchestration.
- Production signing, notarization, publishing, or distribution.
- Customer-value claims.
