# Execution Rules

## Authority Order

1. `AGENTS.md` and nested local instructions in the opened project.
2. `docs/source-index.md`.
3. `docs/mvp-scope.md`.
4. `docs/release-gates.md`.
5. Selected task file.
6. `docs/ha2ha-mdsync-reconciliation.md` when shared mode is enabled.

## AI May Decide

- Reversible implementation details inside an approved task.
- Test structure matching established project patterns.
- UI composition within the recorded design direction.
- Follow-up issue wording for unrelated discoveries.

## AI Must Ask

- Product promise, primary customer, or MVP boundary changes.
- Risky overwrites of substantial existing content.
- Secrets, paid services, production access, signing, publishing, or other
  irreversible external actions.

## Application Effect Boundary

- All filesystem paths are resolved relative to a user-selected project root.
- Backend commands reject absolute relative-path inputs and parent traversal.
- Inventory skips symlinked entries, and existing write targets are canonicalized
  before mutation.
- The frontend does not invoke shell commands directly.
- Helpers and agent runtimes are replaceable adapters with structured results.
- Helper and agent subprocesses run with the current user's host permissions.
  Selecting a project and setting `cwd` are not a filesystem sandbox; each
  subprocess action must be explicit and visible.
- Every action initiated by the application is visible in the run inspector.
- Demo events are labeled simulated and never presented as execution proof.

## Shared Collaboration Effect Boundary

- Shared HA2HA mode is opt-in; local solo mode must not require network access.
- Build Right resolver/task evidence remains local selection and completion authority.
- Only the resolver-selected executable task may be projected into a remote
  execution envelope; do not mirror the backlog.
- Parse MDSync URLs, perform discovery, retain bearer capabilities, and execute
  remote HTTP effects only in the native adapter.
- Never store or expose capability query values, authorization headers, or
  capability-bearing URLs in repository files, goal state, logs, evidence,
  diagnostics, UI events, screenshots, or Codex prompts/output.
- Bind shared execution confirmation to both the local task hash/Git
  fingerprint and remote HA2HA path/version.
- A remote claim conflict, access denial, stale binding, invalid manifest, or
  unavailable transport stops before Codex starts.
- Local repository verification commits before remote completion evidence.
  Failure after that commit creates repair-required collaboration debt and
  blocks only the next shared iteration; it does not revert local truth.
- Repair requires reconnected Collaborator access and explicit user action. It
  may write missing remote records but must not rerun Codex.

## Stop/Ask Gates

- Founder-owned product or scope decision is required.
- Open material conflict exists in `docs/conflicts.md`.
- Publishing, secrets, paid services, or production access is required.
- Verification fails or source state becomes stale.
- Native release is requested before Rust/Cargo and a Tauri build are verified.
- Shared execution lacks current Collaborator access or reconciled source/version binding.
- Remote claim conflicts or a prior verified task has unresolved collaboration repair debt.

## Evidence Destinations

- Task evidence: `tasks/issues/*.md`
- Blueprint status: `docs/blueprint-status.md`
- Durable decisions: `docs/decision-log.md`
- Product evidence: `docs/evidence/*.md`

## Required Verification

| Change Type | Required Checks | Evidence |
| --- | --- | --- |
| Markdown/task parser | Unit tests with representative and malformed input | selected task evidence log |
| React interface | Typecheck, component tests, production build, visual inspection | selected task evidence log |
| Tauri command boundary | Rust tests and native compile when toolchain exists | release gate and task evidence |
| Agent/helper adapter | Structured result test and explicit real/simulated label | selected task evidence log |
| HA2HA/MDSync native adapter | Contract fixtures, access/conflict tests, timeout/size bounds, and capability-leak scan | selected Sprint 2 task evidence |
| Shared bounded execution | No-spawn proof on conflict plus repository verification and remote readback | selected Sprint 2 task evidence |
| Partial remote sync repair | Local completion preservation, restart/reconnect, idempotent repair, and no-Codex proof | selected Sprint 2 task evidence |
