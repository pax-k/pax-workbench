# Task 028 Post-Run Review Receipt Evidence

Date: 2026-07-23
Status: complete
Task: `tasks/issues/028-add-post-run-diff-and-evidence-review-receipt.md`

## Outcome

The workbench now projects every bounded controller result into one
founder-facing receipt. Repository outcome and verification lead; bounded
current-worktree changes, acceptance evidence, commands/checks, tracker and
fresh resolver state, risks, optional sanitized shared completion/repair state,
and raw normalized events remain connected without creating new authority.

## Native Evidence Boundary

`src-tauri/src/review_receipt.rs` adds one read-only command backed by the
existing injected Git read port. It:

- parses NUL-delimited porcelain status and rejects unsafe relative paths;
- reads tracked diffs without external diff drivers and synthesizes bounded
  textual evidence for regular untracked files;
- reports binary, oversized, aggregate-limit, filesystem, and Git failures
  explicitly;
- caps evidence at 200 paths, 64 KiB per file, and 256 KiB aggregate text;
- removes controls and redacts secret, bearer, capability, and query-bearing
  URL lines while preserving diff markers; and
- states that the view is the current worktree and does not attribute
  authorship to Codex.

The command cannot stage, commit, reset, checkout, push, publish, or invoke a
runtime.

## Product Projection

`src/lib/review-receipt.ts` and `src/components/ReviewReceipt.tsx` provide:

- distinct completed, failed, blocked, cancelled, and partial receipts;
- task criteria, evidence-log checks, status, loop state, next task/reason,
  risks, and follow-ups derived from refreshed repository evidence;
- optional shared access, exact local binding, claim version/result,
  evidence/handoff status, reconciliation/repair debt, and Codex-start proof;
- repair-cursor precedence so a repair refreshes the same receipt without
  rerunning Codex;
- expandable bounded diffs and raw normalized events; and
- explicit accept-for-handoff, request-revision, continue, and stop choices
  that record UI intent only.

## Automated Verification

| Command | Result |
| --- | --- |
| `bun run check` | pass: authority, typecheck, 142 tests, production build |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 229 tests |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | pass |
| `git diff --check` | pass |

Focused coverage includes all terminal tones, ANSI/control/capability/secret
redaction, bounded and binary-unavailable native evidence, unsafe paths,
partial evidence, shared repair debt, fresh-confirmation continuity, and
effect-free handoff intent.

## Signed-Native Acceptance

Artifact:
`src-tauri/target/debug/bundle/macos/Build Right Studio.app`

- The exact Apple-development-signed bundle selected Task 028 in the current
  repository and ran the deterministic controller fixture. Because Task 028
  was still `ready`, repository verification correctly produced the failed
  receipt. The receipt exposed current-worktree scope, bounded path evidence,
  criteria/status, `failureStop`, risks, raw events, and explicit no-effect
  decisions.
- A disposable signed-native Task 991 then ran through one explicit live
  confirmation. It created and verified one exact proof file, recorded task
  evidence, made its tracker terminal, and stopped. The same receipt rendered
  `Repository verification passed`, `goalComplete`, checked criteria, passed
  commands, changed paths, no next task, and the retained real adapter events.
- Selecting `Accept for handoff` added only
  `accepted for a separate handoff action`; no repository or runtime action
  followed.

The disposable repository is outside product authority and was used only to
obtain a real completed terminal path without mutating this task's source
during its own verification.

## Residual Boundaries

- Current-worktree evidence may include pre-existing changes by design and says
  so; Task 028 does not claim per-run authorship.
- Task 028 itself remains read-only. The now-complete Task 028A adds a separate
  previewed, confirmed, selected-path local Git handoff; accepting this receipt
  still performs no Git effect.
- Production notarization/distribution and founder-led acceptance remain later
  gates; Task 032 remains founder-owned.
