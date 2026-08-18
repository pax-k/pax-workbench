# Sprint 1: Controlled MVP Loop

Status: complete
Purpose: turn the verified frontend prototype into one safe, observable,
resumable Build Right execution loop against a real repository.

## Tasks

| ID | Title | Status | Depends On | Evidence |
| --- | --- | --- | --- | --- |
| 002 | Verify native Tauri boundary (broad original) | superseded | — | tasks/issues/002-verify-native-tauri-boundary.md |
| 003 | Validate first-party skill UI contracts | complete | 001 | tasks/issues/003-validate-skill-ui-contracts.md |
| 004 | Verify native Tauri boundary | complete | Rust/Cargo satisfied | tasks/issues/004-verify-native-tauri-boundary.md |
| 005 | Complete safe repository session | complete | 004 | tasks/issues/005-complete-safe-repository-session.md |
| 006 | Add explicit skill setup adapter | complete | 003, 005 | tasks/issues/006-add-explicit-skill-setup-adapter.md |
| 007 | Implement deterministic helper execution | complete | 003, 005, 006 | tasks/issues/007-implement-deterministic-helper-execution.md |
| 008 | Implement Codex runtime adapter | complete | 005, 007 | tasks/issues/008-implement-codex-runtime-adapter.md |
| 009 | Execute one bounded task | complete | 008 | tasks/issues/009-execute-one-bounded-task.md |
| 010 | Persist checkpointed goal state | complete | 009 | tasks/issues/010-persist-checkpointed-goal-state.md |
| 011 | Run confirmed goal loop | complete | 009, 010 | tasks/issues/011-run-confirmed-goal-loop.md |
| 012 | Prove the MVP end to end | complete | 003-011 complete | tasks/issues/012-prove-mvp-end-to-end.md |

## Dependency Policy

- Task 003 is complete.
- Task 004 is complete after native tests, debug build, artifact audit, and independent review.
- Task 005 is complete after structured boundary tests, independent review, and
  a real compiled-app disposable-repository round trip.
- Task 006 is complete after closed-adapter tests, independent review, and real
  compiled-app cancellation, repair, and successful-install trials.
- Task 007 is reconciled against validated helper declarations and the installed
  parser/CLI surfaces and is complete after adversarial lifecycle tests,
  independent review, and a real compiled-app three-helper smoke.
- Task 008 is complete after bounded fixture and real compiled-app JSONL runs,
  exact-version/argv evidence, process cleanup, and independent review.
- Task 009 is complete after reviewed controller state-machine coverage and a
  real compiled-app reversible task trial with repository-verified completion.
- Task 010 is reconciled against the completed controller and current in-memory
  checkpoint surfaces and is complete after reviewed persistence security tests,
  compiled close/reopen recovery, and Git-stale detection.
- Task 011 is complete after the unchanged signed artifact passed a generic
  restart and two separately confirmed native iterations, full verification,
  cleanup checks, durable evidence capture, and independent review. Earlier
  stalls remain historical evidence; no unsupported causal repair is claimed.
- Task 012 is complete after one cohesive post-dependency signed-app trial,
  full closeout verification, authority reconciliation, and independent
  release approval with no blocking findings.
- Promote one planned row to `ready` only after every dependency is complete and
  its task contract is reconciled against current repository evidence.
- Execute one task at a time through `build-right-execution`.

## Sprint Exit

Sprint 1 is complete only when tasks 003-012 are terminal and task 012 records
the real/manual/simulated boundary for the whole MVP loop.

Exit result: passed on 2026-07-22. Tasks 003-012 are terminal, Tasks 003-011 are
complete, and Task 012 records the cohesive signed loop plus every real,
manual, simulated, and unproven boundary.
