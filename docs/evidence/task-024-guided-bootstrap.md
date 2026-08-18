# Task 024 Guided Bootstrap Evidence

Date: 2026-07-23
Status: complete
Source under test: repo-local path and Apple-development-signed native app

## Run Label

`task-024-signed-guided-bootstrap`

## Agent / Tool Surface

- Codex desktop implementation and verification
- Build Right preflight and engineering-principles guidance
- Native macOS app operated through accessibility-backed computer control
- No model router and no collaboration provider

## Target

Disposable empty Git repository:
`/private/tmp/build-right-task024-HRVdFL`

## Commands And Checks

- `bun run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun run tauri:build:signed`
- `codesign --verify --deep --strict --verbose=2 <app>`
- Native folder selection, founder input, artifact preview, confirmed apply,
  project-scoped skill setup, and preflight
- Filesystem/lock/contract readback and HA2HA/MDSync coordinate scan

## Artifacts And Result

- 12 canonical Markdown authority files were previewed and then created.
- Five inputs were labeled founder claims or decisions; customer validation was
  not claimed.
- Four first-party `skill-ui` contracts were created only inside the confirmed
  setup boundary and bound to the installed `skills-lock.json` hashes.
- Native preflight executed the authenticated helper and returned
  `ask-founder`, confidence `medium`, with the exact validation gaps.
- Collaboration remained disabled before bootstrap, after artifact creation,
  and after the non-ready preflight result.
- Bootstrap artifacts contained no HA2HA, MDSync, session, workspace, publish,
  claim, or remote-write coordinate.

## Fail-Closed Findings Repaired

1. Artifact completeness initially enabled collaboration before preflight and
   resolver truth. Eligibility now requires `ready-for-execution` preflight plus
   one executable resolver-selected envelope.
2. Canonicalizing the `bunx` symlink changed Bun dispatch semantics. Setup now
   invokes the trusted canonical `bun` executable with explicit `x`.
3. Blank-project setup previously stopped because no `skill-ui` contracts
   existed. Confirmed setup now creates missing trusted built-ins or rebinds
   only byte-matching built-ins; divergent repository contracts are preserved
   and fail closed.

## Evidence Classification

- Proved: automated state/effect contracts, native file creation, signed setup,
  provenance binding, preflight execution, and local-only gating.
- Manual: UI navigation and confirmation were performed by an AI operator.
- Simulated: stale, cancel, invalid input, and partial apply fixtures.
- Unproved: unaided founder usability, customer value, notarized distribution,
  and production release.

## Follow-Up

Task 025 is ready to implement the planning-only path. Task 032 remains the
founder-owned usability gate.
