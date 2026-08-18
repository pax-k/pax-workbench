# 003: Validate First-Party Skill UI Contracts

Status: complete
Type: contract
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/raw/product-discussion.md; docs/mvp-scope.md
Reversibility: easy
Learning objective: prove that the UI can describe first-party skills without inferring permissions or executable behavior from prose
Source under test: repo-local path

## Goal

Define and validate a versioned machine-readable UI contract for the four
installed Build Right skills, with a safe generic fallback for unknown skills.

## Non-Goals

- Execute helpers or agent runtimes.
- Install or update skills.
- Define a public third-party marketplace standard.
- Change upstream `.agents/skills/**` sources.

## Required Reading

- docs/raw/product-discussion.md
- docs/mvp-scope.md
- docs/execution-rules.md
- skills-lock.json
- src/types.ts
- src-tauri/src/lib.rs

## Acceptance Criteria

- [x] A versioned schema defines identity, lifecycle phase, purpose, reads,
      writes, decisions, helpers, required evidence, stop states, renderer, and
      source provenance.
- [x] Runtime validation rejects missing, unknown-version, and executable fields
      that do not match the allowed contract shape.
- [x] Validated contracts exist for preflight, feature planning, execution, and
      engineering principles.
- [x] Contract provenance is traceable to the installed skill path and
      `skills-lock.json` hash.
- [x] Unknown skills render through a generic non-executable fallback.
- [x] Tests cover valid contracts, malformed contracts, provenance mismatch,
      and fallback behavior.
- [x] Existing demo/operating cards consume validated contract data rather than
      a hardcoded skill-purpose switch.

## Baseline Evidence

`src-tauri/src/lib.rs` currently classifies skill IDs through a hardcoded
`skill_contract` switch, while the product discussion requires a validated
machine-readable companion contract.

## Solution-Fit Rationale

- Requirement served: make skill operating rules and provenance visible.
- Constraints honored: no permissions or commands inferred from Markdown prose.
- Guarantees preserved: explicit effects, safe fallback, repo-visible source.
- Cost accepted: first-party contract fixtures and validation maintenance.
- Deferred capability: public third-party contract distribution.

## Verification

- Focused contract-validator and fallback tests.
- `bun run typecheck`
- `bun run test`
- `bun run build`

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-21 | `bun run test -- src/lib/skill-contracts.test.ts` | pass | 6 focused tests cover valid contracts, malformed/version/executable fields, lock provenance, first-party identity, helper ownership, semantic blanks, and fallback behavior |
| 2026-07-21 | `bun run typecheck` | pass | TypeScript project references completed without errors |
| 2026-07-21 | `bun run test` | pass | 3 files and 11 tests passed |
| 2026-07-21 | `bun run build` | pass | Production bundle completed; 1,752 modules transformed |
| 2026-07-21 | `git diff --check` | pass | No whitespace errors found |
| 2026-07-21 | Playwright at `1536x768` and `1180x768` | pass | All contract panels, including helpers, evidence/stops, and provenance, remain visible; evidence in `output/playwright/task-003-skill-contracts*.png` |
| 2026-07-21 | Independent Sol/high contract review | pass | Final review reported no findings after trust-boundary, helper-ownership, and validator-parity corrections |
| 2026-07-21 | `command -v cargo`; `command -v rustc` | skipped | Both commands returned no path; Rust compilation and native tests remain Task 004's external-toolchain gate |

## Files Changed

- `skill-ui/build-right-preflight.json` - versioned Discover contract.
- `skill-ui/build-right-feature-planning.json` - versioned Plan contract with the installed `feature-planning-check` helper.
- `skill-ui/build-right-execution.json` - versioned Build contract.
- `skill-ui/build-right-engineering-principles.json` - versioned Principles contract.
- `src/types.ts` - expanded validated skill-summary projection.
- `src/lib/skill-contracts.ts` - strict first-party registry, lock provenance validation, and safe fallback.
- `src/lib/skill-contracts.test.ts` - focused contract and fallback regressions.
- `src/lib/demo.ts` - replaces hardcoded skill-purpose data with validated summaries.
- `src/App.tsx` - renders contract reads, writes, decisions, helpers, evidence, stops, and provenance.
- `src/styles.css` - keeps the expanded operating card visible across verified desktop widths.
- `src-tauri/Cargo.toml` - adds `serde_json` for the native contract parser.
- `src-tauri/src/lib.rs` - replaces the hardcoded classifier with first-party validation, canonical installed-path checks, and non-executable fallback tests.
- `output/playwright/task-003-skill-contracts.png` - desktop visual evidence.
- `output/playwright/task-003-skill-contracts-narrow.png` - narrow desktop visual evidence.

## Verification Summary

- Focused and full Bun verification passed.
- The browser demo consumes the same validated JSON summaries and visibly exposes every required field.
- Native Rust validator tests were authored and independently source-reviewed, but were not compiled or executed because Cargo/rustc are unavailable. Task 004 owns that release evidence.

## Learning Notes

- Proved: four explicitly registered first-party contracts validate against lockfile source/hash and installed-path expectations; unknown or malformed skills remain generic and non-executable; operating cards consume validated data.
- Real/manual/simulated boundary: Bun validation and browser rendering are real; the browser uses the repository demo projection; native Tauri discovery and Rust tests were not executed or simulated.
- Test next: compile the native validator in Task 004, then consume the contract in project skill setup and helper execution.

## Skill Trial Notes

- Source under test: repo-local `.agents/skills/*/SKILL.md`, `skills-lock.json`, and `skill-ui/*.json`.
- Source comparison: pass for declared installed path plus lockfile source/hash; content hashing remains supplied by the existing lockfile.
- Contract markers checked: version, identity, lifecycle, reads/writes, decisions, per-skill helpers, evidence, stops, renderer, provenance, and non-executable fallback.
- Trial status: pass for available TypeScript/browser surfaces; native compilation explicitly deferred to Task 004.

## Blockers

- None.

## Follow-Ups

- Task 006 consumes the contract for skill setup.
- Task 007 consumes contract-declared helpers.
