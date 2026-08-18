# Task 023 Artifact Plan Evidence

Date: 2026-07-23
Source under test: repo-local path
Outcome: pass

## Proved

- Exact drafts are restricted to canonical planning/task Markdown paths and
  bounded by count, file size, aggregate size, string content, and duplicate
  rejection.
- Absolute/traversing/disallowed paths, symlink parents, non-files, and
  differing existing files stop before a write.
- Preview returns exact content, additive diff, content hash, effect, Git
  baseline, expiry, and an opaque one-use token.
- Apply consumes a repository-bound unexpired token, rechecks the Git baseline,
  shares the local operation registry, and creates files without overwrite.
- Exact prior content supports a fresh idempotent preview. Injected failure
  after one real create returns exact committed/unapplied paths and no false
  success.
- Bridge arguments and results contain no collaboration session or workspace
  coordinate; the native result explicitly reports zero collaboration effects.

## Verification

- `bun run check`: 92 tests, authority check, typecheck, production build.
- `cargo test`: 226 tests.
- Rust check and format: pass.
- Tauri debug build: pass.
- Produced native binary startup: pass.

The live startup proves command registration and native runtime health. The
new effect has no Task 023 UI by design; real-filesystem focused tests prove
the create/apply behavior until Task 024 exposes and manually exercises it.

## Review Boundary

Independent subagent review was unavailable. Security fixtures, full
regressions, debug packaging, live startup, and local diff review substituted.
