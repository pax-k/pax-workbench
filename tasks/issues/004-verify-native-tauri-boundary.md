# 004: Verify Native Tauri Boundary

Status: complete
Type: validation
Owner: AI

Assumption basis: repo-evidence-backed
Requirement basis: docs/release-gates.md; docs/execution-rules.md
Reversibility: easy
Learning objective: prove the authored Rust boundary compiles and its traversal defenses execute on this machine
Source under test: repo-local path

## Goal

Provide Rust/Cargo, execute the existing Rust boundary tests, and produce a
debug Tauri build without expanding command permissions.

## Non-Goals

- Exercise the full repository workflow.
- Execute helpers or Codex.
- Sign, publish, or distribute the application.
- Install the Rust toolchain without explicit environment-owner authorization.

## Required Reading

- docs/execution-rules.md
- docs/release-gates.md
- src-tauri/Cargo.toml
- src-tauri/src/lib.rs
- tasks/issues/002-verify-native-tauri-boundary.md

## Acceptance Criteria

- [x] `rustc`, Cargo, and rustup versions are recorded.
- [x] Rust tests pass, including lexical traversal, nested/root symlink
      inventory, and symlinked write-target cases.
- [x] `bun run tauri build --debug` succeeds.
- [x] Generated native artifacts are identified and ignored or retained
      intentionally.
- [x] No broader Tauri capabilities or filesystem permissions are introduced.

## Baseline Evidence

The external prerequisite is now satisfied: `bun run tauri info` detects Tauri
2, Xcode, rustc 1.97.1, Cargo 1.97.1, rustup 1.29.0, and the default stable
`aarch64-apple-darwin` toolchain. Rust tests exist but have never run.

## Solution-Fit Rationale

- Requirement served: deliver a real desktop shell and prove the native trust boundary.
- Constraints honored: external toolchain installation remains user/environment owned.
- Guarantees preserved: least privilege and explicit native effects.
- Cost accepted: platform-specific build prerequisite.
- Deferred capability: release signing and distribution.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run tauri build --debug`
- `bun run tauri info`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | `bun run tauri info` | waiting-external | Rust/Cargo/rustup absent |
| 2026-07-21 | official `rustup` installer with minimal stable profile | pass | Environment-owner authorization received; stable `aarch64-apple-darwin` installed |
| 2026-07-21 | `PATH=/Users/pax/.cargo/bin:$PATH bun run tauri info` | pass | rustc 1.97.1, Cargo 1.97.1, rustup 1.29.0, Xcode 26.6, and command-line tools detected |
| 2026-07-21 | `PATH=/Users/pax/.cargo/bin:$PATH cargo test --manifest-path src-tauri/Cargo.toml` | fail | Initial native compile found missing `src-tauri/icons/icon.png`; no tests executed |
| 2026-07-21 | deterministic `icon.svg` plus `sips -s format png ...` | pass | Added a 512x512 RGBA app icon derived from the existing Build Right three-bar mark |
| 2026-07-21 | `PATH=/Users/pax/.cargo/bin:$PATH cargo test --manifest-path src-tauri/Cargo.toml` | pass | 13 tests passed after adding dangling-symlink coverage; 0 failed |
| 2026-07-21 | `PATH=/Users/pax/.cargo/bin:$PATH bun run tauri build --debug` | pass | Built executable at `src-tauri/target/debug/pax-workbench` |
| 2026-07-21 | `bun run check` | pass | Typecheck, 11 frontend tests, and production bundle passed |
| 2026-07-21 | artifact and capability audit | pass | `src-tauri/target/` and `dist/` are ignored; `Cargo.lock`, `gen/schemas`, and icon source/assets retained; permissions remain `core:default` and `dialog:allow-open` |
| 2026-07-21 | independent Sol/high native-boundary review | pass | Found and verified repair of dangling leaf-symlink escape; final review reported no remaining Task 004 code findings |

## Files Changed

- `src-tauri/src/lib.rs` - reject all symlink write targets and prove dangling outside targets are never created.
- `src-tauri/Cargo.lock` - retain reproducible native dependency resolution for the application.
- `src-tauri/icons/icon.svg` - retain deterministic editable app-icon source.
- `src-tauri/icons/icon.png` - retain required 512x512 RGBA Tauri context asset.
- `src-tauri/gen/schemas/*` - retain Tauri-generated capability schemas for review and tooling.
- `src-tauri/target/` - generated compile/test/debug output, intentionally ignored.
- `dist/` - generated frontend build output, intentionally ignored.
- `/Users/pax/.cargo` and `/Users/pax/.rustup` - authorized external minimal stable Rust installation.

## Verification Summary

- rustc 1.97.1, Cargo 1.97.1, and rustup 1.29.0 are installed and recorded.
- All 13 Rust tests pass, including lexical traversal, nested/root inventory symlinks, existing and dangling write-target symlinks, and skill-contract boundary cases.
- The debug Tauri application and frontend production bundle build successfully.
- Generated output is classified intentionally, and the application capability remains limited to `core:default` and `dialog:allow-open`.

## Learning Notes

- Proved: the native boundary compiles; its authored traversal/symlink fixtures execute; a real arm64 debug application is produced; no broader capability is registered.
- Real/manual/simulated boundary: toolchain installation, Rust tests, Tauri compilation, and artifact inspection are real; no repository-session UI workflow was simulated or claimed.
- Residual risk: same-user concurrent path replacement between validation and mutation is non-atomic; Task 005 already requires expected-version checks and atomic replacement.
- Test next: perform Task 005's disposable-repository round trip with atomic/stale-write enforcement.

## Skill Trial Notes

- Source under test: repo-local `src-tauri` plus authorized stable `aarch64-apple-darwin` toolchain.
- Source comparison: not applicable.
- Contract markers checked: lexical traversal, inventory symlinks, regular/dangling write-target symlinks, debug build, artifact retention, and capability ACL.
- Trial status: pass.

## Blockers

- None.

## Follow-Ups

- Task 005 owns expected-version writes, atomic replacement, refresh, and the recorded concurrent path-replacement residual.
