# 028A: Add Safe Local Git Handoff And Commit Boundary

Status: complete
Type: feature
Owner: AI

Assumption basis: founder-approved workflow design plus repo-evidence-backed Git constraints
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Task 028
Reversibility: moderate
Learning objective: prove a reviewed task result can become an explicit scoped local commit without staging unrelated work or enabling remote/destructive Git actions
Source under test: repo-local path

## Goal

Add a preview/confirm/apply boundary that can stage only explicitly selected
review-receipt paths and create one local commit with a reviewed message.

## Non-Goals

- Push, publish, open a pull request, reset, revert, checkout-overwrite, delete,
  amend, rebase, merge, or resolve conflicts.
- Stage unrelated dirty files, ignored files, capability material, or files
  outside the selected repository.
- Make a commit a prerequisite for repository verification or local completion.

## Required Reading

- docs/founder-facing-workflow.md
- docs/execution-rules.md
- docs/evidence/sprint-3-post-ha2ha-reconciliation.md
- tasks/issues/021-extract-native-repository-and-workflow-controller-modules.md
- tasks/issues/028-add-post-run-diff-and-evidence-review-receipt.md

## Acceptance Criteria

- [x] Preview lists exact repo-relative candidate paths, current Git
      fingerprint/status, exclusions, proposed message, and staged effects.
- [x] Only existing review-receipt paths inside the repository can be selected;
      unrelated dirty files, symlink escapes, missing paths, submodules,
      conflict states, and capability-like material fail closed.
- [x] Apply consumes one expiring one-use token bound to repository identity,
      Git HEAD/index/worktree baselines, exact paths, and exact message.
- [x] Staging is path-scoped; a pre-existing index is either preserved exactly
      or causes a typed stop before mutation.
- [x] Commit success is proved by readback of the new local HEAD and exact
      committed paths; partial/start/verification failure reports truthful
      repair without claiming success.
- [x] Local completion and optional HA2HA reconciliation never depend on commit
      success; no remote Git or MDSync mutation occurs.
- [x] Native contract/security tests, frontend review integration, full checks,
      and a disposable signed-native local-commit trial pass.

## Baseline Evidence

The founder workflow promises an optional explicit local commit after result
review, but the application currently exposes Git only as read-only identity,
fingerprint, status, and verification evidence.

## Solution-Fit Rationale

- Requirement served: complete the explicit local handoff promised by the
  founder-facing workflow.
- Constraints honored: repository containment, unrelated-work preservation,
  one-use confirmation, and no remote/destructive Git actions.
- Guarantees preserved: task completion and HA2HA synchronization remain
  independent of Git commit success.
- Cost accepted: one narrow native Git mutation surface and disposable-repo
  acceptance suite.
- Deferred capability: push, PR, branch management, rollback, and conflict
  resolution.

## Verification

- Focused path/index/fingerprint/token/commit/readback failure fixtures.
- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- Signed-native disposable-repository local commit trial.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml git_handoff --no-fail-fast` | pass | Seven real-repository fixtures cover exact selected commit, unrelated dirty preservation, stale/replay/unconfirmed/index stops, symlink/capability/missing/conflict/gitlink exclusions, staged-blob mismatch, and truthful commit failure. |
| 2026-07-23 | `bun run check` | pass | Authority check, TypeScript, 145 frontend tests, and production build passed. |
| 2026-07-23 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass | 236 native tests passed. |
| 2026-07-23 | `cargo check --manifest-path src-tauri/Cargo.toml` | pass | Native command and state registration compiled. |
| 2026-07-23 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`; `git diff --check` | pass | Rust formatting and patch hygiene passed. |
| 2026-07-23 | Apple-development-signed `Build Right Studio.app`; binary SHA-256 `8116e781039b7a942eec9ea56e66b8d8334025fe6f772aeb638c5406023a49c2` | pass | Exact signed app completed disposable Task 992, separately inspected receipt paths, selected one path, previewed one-use confirmation, and verified local commit `ad4b8c99c0e18473d093f0aae77a19110ff52ea2`. |
| 2026-07-23 | `git diff-tree --root --no-commit-id --name-only -r HEAD`; index/status/readback checks in disposable repo | pass | Commit contained only `task028a-selected-proof.txt`; unrelated dirty fixture and all prior dirty paths remained uncommitted; index was clean; message and new HEAD matched; no remote existed. |

## Files Changed

- `src-tauri/src/git_handoff.rs` - owns bounded inspection, exclusions,
  expiring one-use preview storage, filter-free path staging, hook-isolated
  local commit, and exact readback/repair.
- `src-tauri/src/lib.rs`, `src-tauri/src/command_contract.rs` - register the
  serialized two-command boundary and shared operation lock.
- `src/types.ts`, `src/lib/bridge.ts`, `src/lib/bridge.test.ts` - expose only
  typed local handoff previews/results and closed command arguments.
- `src/components/LocalGitHandoff.tsx`,
  `src/components/LocalGitHandoff.test.tsx`, `src/App.tsx`,
  `src/App.test.tsx`, `src/styles.css` - add the separate
  inspect/select/preview/confirm/readback UI without changing review intent.
- `docs/evidence/task-028a-safe-local-git-handoff.md` and affected authority
  documents - record the boundary, proof, decision, and next resolver state.

## Verification Summary

- Focused native security/repair fixtures, focused frontend/bridge integration,
  full frontend/native gates, production bundle, and exact signed-native
  selected-path commit all passed.
- The signed readback proved one new local HEAD and one exact committed path
  while the unrelated dirty fixture remained untracked and the index returned
  clean.
- No push, remote, MDSync, task-completion, reset, hook, clean-filter, signing,
  or destructive Git effect is reachable through the boundary.

## Learning Notes

- Proved: a completed signed-native receipt can separately create one reviewed
  local commit without absorbing unrelated worktree changes.
- Proved: preview/apply bind canonical repository identity, HEAD/index/worktree,
  exact paths/message, and staged content; pre-staged indexes stop before
  mutation.
- Simulated/fixture-backed: stale, replay, capability, symlink, conflict,
  submodule, staged-content mismatch, and commit-lock failures.
- Test next: Task 029 integrates this secondary handoff into the responsive
  information hierarchy.

## Skill Trial Notes

- Source comparison: Build Right execution and engineering guidance
- Contract markers checked: scoped effects, one-use confirmation, Git baseline, readback, no remote mutation
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 029 integrates the reviewed handoff into the product shell.
