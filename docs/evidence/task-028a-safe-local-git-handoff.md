# Task 028A Safe Local Git Handoff Evidence

Date: 2026-07-23
Status: complete
Task: `tasks/issues/028a-add-safe-local-git-handoff-and-commit-boundary.md`

## Outcome

The post-run receipt now offers a separate local Git handoff only after
`Accept for handoff` records intent. The founder must inspect current Git
truth, explicitly select eligible receipt paths, enter one reviewed message,
preview exact effects, and consume a fresh confirmation before one local
commit. Completion and optional HA2HA state never depend on this action.

## Native Boundary

`src-tauri/src/git_handoff.rs` owns two local-only commands:

- inspection recomputes NUL-delimited current status, repository identity,
  HEAD/index/worktree fingerprint, eligible receipt paths, exclusions, and
  exact staged effects;
- missing, stale, unrelated, pre-staged, symlink, non-file, outside-root,
  conflict, rename/copy, submodule, binary, oversized, and capability-like
  paths fail closed;
- a valid preview issues one expiring, one-use in-memory token bound to
  canonical repository identity, all Git baselines, exact paths, exact
  message, and selected file bytes;
- any pre-existing index content causes a typed stop before mutation;
- staging writes reviewed blobs with `hash-object --no-filters` and
  path-scoped `update-index`, preventing repository clean filters from
  executing;
- commit disables hooks, signing, fsmonitor, maintenance, and automatic GC,
  then reads back new HEAD, exact commit message, and exact committed paths;
- partial staging, commit failure, or post-commit verification failure reports
  exact staged/committed state and repair without claiming success.

The command surface has no push, fetch, remote, reset, checkout, revert,
delete, amend, rebase, merge, MDSync, or runtime operation.

## Product Boundary

`LocalGitHandoff.tsx` is rendered only after the receipt's effect-free handoff
intent. It requires four visible steps:

1. inspect current Git candidates and exclusions;
2. select exact eligible paths and enter a one-line message;
3. review the staged-path and local-commit preview;
4. explicitly confirm, then read the verified or repair result.

Changing paths/message invalidates the preview. The UI always states that
remote effects are none and repository completion is unchanged.

## Automated Verification

| Command | Result |
| --- | --- |
| `bun run check` | pass: authority, typecheck, 145 tests, production build |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 236 tests |
| `cargo check --manifest-path src-tauri/Cargo.toml` | pass |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass |
| `git diff --check` | pass |

Seven focused native fixtures use real disposable Git repositories. They prove
one-path commit and unrelated-dirty preservation, stale/token/index gates,
path/type/capability/conflict/gitlink exclusion, selected-blob verification,
and truthful commit failure with staged repair. Frontend tests prove the
separate inspect/select/preview/confirm flow and dirty-index stop.

## Signed-Native Acceptance

Exact artifact:
`src-tauri/target/debug/bundle/macos/Build Right Studio.app`

- Apple development signature verified on disk and satisfied its designated
  requirement.
- Exact binary SHA-256:
  `8116e781039b7a942eec9ea56e66b8d8334025fe6f772aeb638c5406023a49c2`.
- The exact signed app opened disposable Task 992 and performed one explicit
  live Codex invocation. Repository verification passed and rendered a
  completed receipt with both selected and unrelated current paths.
- `Accept for handoff` still caused no Git effect. The separate surface
  inspected all paths, selected only `task028a-selected-proof.txt`, previewed
  message `Task 028A signed native selected-path proof`, and consumed one
  confirmation.
- Native readback reported verified commit
  `ad4b8c99c0e18473d093f0aae77a19110ff52ea2`.
- Independent Git readback showed that commit contains exactly
  `task028a-selected-proof.txt`; its content/message match, the index is clean,
  `task028a-unrelated-dirty.txt` remains untracked and unchanged, all other
  dirty paths remain outside the commit, and the disposable repository has no
  remote.

The disposable repository is outside product authority. The live Codex task
was used only to produce a truthful completed receipt before exercising the
separate signed-native Git effect.

## Residual Boundaries

- Commit failure intentionally leaves only the selected paths staged and
  reports repair; the product does not add reset/unstage authority.
- Capability scanning is bounded and conservative; binary or oversized files
  are excluded rather than guessed safe.
- Push, PR, signing/notarization for distribution, and founder usability
  acceptance remain outside this task.
