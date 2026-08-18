# 002: Verify Native Tauri Boundary

Status: superseded
Type: validation
Owner: environment owner

Assumption basis: repo-evidence-backed
Requirement basis: docs/release-gates.md
Reversibility: easy
Learning objective: prove the authored Rust boundary compiles and rejects traversal in a real native build
Source under test: repo-local path

## Goal

Install or provide a Rust/Cargo toolchain, run the native unit tests and Tauri
build, then exercise repository selection, file switching, scoped Save, and the
preflight helper against a disposable real repository.

## Supersession

This task combined environment provisioning, native compilation, repository
round-trip behavior, and helper execution. It is superseded by the ordered
Sprint 1 tasks `004`, `005`, and `007`. No implementation should execute from
this task.

## Non-Goals

- Production signing or publishing.
- Unattended agent execution.
- Expanding app-mediated filesystem permissions.

## Required Reading

- docs/execution-rules.md
- docs/release-gates.md
- src-tauri/src/lib.rs
- tasks/issues/001-build-local-workbench-mvp.md

## Acceptance Criteria

- [ ] Rust and Cargo versions are recorded.
- [ ] Rust unit tests, including symlink boundary cases, pass.
- [ ] A debug Tauri build succeeds.
- [ ] One disposable repository round-trip is recorded without outside-root access.
- [ ] Helper subprocess host-permission behavior remains explicit in the UI and evidence.

## Baseline Evidence

`bun run tauri info` reports Xcode present but `rustc`, Cargo, and rustup absent.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run tauri build --debug`
- Manual disposable-repository trial with path-boundary fixtures

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | `bun run tauri info` | blocked-environment | Rust/Cargo/rustup absent |

## Learning Notes

- Proved: Tauri configuration is detected by the installed CLI.
- Simulated: native command behavior has not run.
- Test next: compile and execute the authored boundary tests.

## Skill Trial Notes

- Source comparison: not applicable
- Contract markers checked: source path, external gate, verification commands
- Trial status: partial-needs-rerun

## Blockers

- Rust/Cargo toolchain installation or availability.

## Follow-Ups

- `tasks/issues/004-verify-native-tauri-boundary.md`.
- `tasks/issues/005-complete-safe-repository-session.md`.
- `tasks/issues/007-implement-deterministic-helper-execution.md`.
