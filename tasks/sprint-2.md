# Sprint 2: Shared HA2HA Collaboration

Status: complete
Purpose: add optional MDSync-hosted HA2HA coordination around the technically
proved local Build Right loop without introducing a second task authority,
leaking bearer capabilities, or weakening local solo execution.

## Requirement Basis

- Founder instruction on 2026-07-22 to make HA2HA/MDSync integration Sprint 2.
- `docs/ha2ha-mdsync-reconciliation.md`.
- `docs/evidence/sprint-2-current-implementation-review.md`.
- Completed Sprint 1 evidence in `docs/evidence/manual-trials.md`.

## Tasks

| ID | Title | Status | Depends On | Evidence |
| --- | --- | --- | --- | --- |
| 013 | Define collaboration contracts and native controller seams | complete | Sprint 1 complete | tasks/issues/013-define-collaboration-contracts-and-native-seams.md |
| 014 | Implement secure native MDSync session transport | complete | 013 | tasks/issues/014-implement-secure-native-mdsync-session-transport.md |
| 015 | Publish and join Build Right HA2HA execution envelopes | complete | 014 | tasks/issues/015-publish-and-join-build-right-ha2ha-execution-envelopes.md |
| 016 | Bind remote HA2HA claims to confirmed execution | complete | 015 | tasks/issues/016-bind-remote-ha2ha-claims-to-confirmed-execution.md |
| 017 | Reconcile post-run evidence and repair partial sync | complete | 016 | tasks/issues/017-reconcile-post-run-evidence-and-repair-partial-sync.md |
| 018 | Add shared collaboration and repair UI | complete | 014-017 | tasks/issues/018-add-shared-collaboration-and-repair-ui.md |
| 019 | Prove local and hosted collaboration end to end | complete | 013-018 | tasks/issues/019-prove-local-and-hosted-collaboration-end-to-end.md |

## Dependency Policy

- Execute one task at a time through `build-right-execution`.
- Promote a planned task only after every dependency is complete and the task
  contract is reconciled against current code and the pinned MDSync contract.
- Local solo mode must remain green after every task.
- Capability-bearing live tests run only in task 019 or an explicitly scoped
  prerequisite smoke; deterministic fixtures are required first.
- A needed HA2HA/MDSync public-contract change routes upstream before dependent
  workbench work continues.

## Sprint Exit

Sprint 2 is complete only when:

- local solo Sprint 1 behavior still passes;
- Viewer access is read-only and Collaborator access can safely claim;
- a remote conflict prevents Codex from starting;
- a local verified completion with failed remote sync stops at a recoverable,
  non-secret repair state;
- restart never persists capability material or starts work automatically;
- two independent agent contexts coordinate through one hosted HA2HA envelope;
- live MDSync evidence is redacted, revocable, and independently reviewed;
- tasks 013-019 are terminal and release gates distinguish local, deterministic,
  live-hosted, simulated, and unproved claims.
