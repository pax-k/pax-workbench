# 030: Enforce Accessibility And Visual Behavior

Status: complete
Type: quality
Owner: AI

Assumption basis: UI/UX audit plus accepted visual contract
Requirement basis: docs/founder-facing-workflow.md; docs/evidence/sprint-3-post-ha2ha-reconciliation.md; Task 029
Reversibility: easy
Learning objective: prove critical product flows remain operable and legible beyond pointer-first default-display use
Source under test: repo-local path

## Goal

Enforce keyboard, semantic, contrast, zoom, high-contrast, reduced-motion, and
critical-state visual behavior for the founder workflow.

## Non-Goals

- Claim formal conformance certification.
- Redesign the visual identity.
- Treat automated checks as a replacement for manual assistive-technology QA.

## Required Reading

- docs/founder-facing-workflow.md
- docs/evidence/founder-workflow-ui-ux-audit.md
- tasks/issues/029-rework-navigation-information-architecture-and-responsive-layout.md

## Acceptance Criteria

- [x] The primary bootstrap, plan, review, execute, resume, and repair paths are
      keyboard operable with logical order, visible focus, and no traps.
- [x] Collaboration connection/secure-input clearing, Viewer inspection,
      shared confirmation, conflict, disconnect, reconnect, and repair are
      keyboard and screen-reader operable without leaking capability values.
- [x] Landmarks, headings, labels, statuses, dialogs, tabs, and live updates have
      appropriate semantics and accessible names.
- [x] Meaning is never conveyed by color alone; focus/text/status contrast is
      checked against the accepted palette.
- [x] Metadata text is at least 11px and body/control text at least 13px, except
      justified decorative/nonessential cases documented in evidence.
- [x] The critical paths remain usable at 200% zoom, macOS increased contrast,
      reduced motion, and a 900x700 window.
- [x] Automated accessibility and deterministic visual regression checks run in
      the normal verification suite, with manual VoiceOver evidence recorded.

## Baseline Evidence

The repo has no keyboard, screen-reader, zoom, high-contrast, or visual
regression suite. Current styles include many 7-9px labels, including essential
collaboration access, binding, version, and repair metadata.

## Solution-Fit Rationale

The task converts readability and interaction expectations into repeatable
release evidence while retaining the chosen product character.

## Verification

- Automated semantic/accessibility scans for critical states.
- Keyboard interaction tests and viewport/zoom visual snapshots.
- `bun run check`
- Manual signed-native VoiceOver, contrast, reduced-motion, and zoom record.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| 2026-07-23 | `docs/evidence/task-030-accessibility-visual-behavior.md` | pass | Axe, keyboard, visual preference, 900x700, signed-native, and VoiceOver evidence |

## Files Changed

- `package.json`
- `bun.lock`
- `src/accessibility.test.tsx`
- `src/App.test.tsx`
- `src/components/CollaborationPanel.tsx`
- `src/components/CollaborationPanel.test.tsx`
- `src/lib/accessibility-contract.test.ts`
- `src/styles.css`
- `scripts/check-authority-drift.ts`
- `scripts/check-authority-drift.test.ts`
- `docs/evidence/task-030-accessibility-visual-behavior.md`
- Sprint 3 authority documents

## Verification Summary

- `bun run check`: pass, including 157 tests and production build.
- Rust tests/check/format: pass, including 236 tests.
- Fresh signed bundle, deterministic browser preference/viewport checks, and
  manual VoiceOver keyboard smoke: pass.

## Learning Notes

- Proved: automated semantic scans, modal keyboard containment, type floors,
  visual media contracts, responsive zoom layout, signed-native AX tree, and
  full regressions.
- Manual: VoiceOver keyboard traversal and signed-native visual legibility.
- Simulated: fixture-backed adverse collaboration states.

## Skill Trial Notes

- Source comparison: frontend-design guidance
- Contract markers checked: semantics, keyboard, contrast, type, motion, QA
- Trial status: n/a

## Blockers

- None.

## Follow-Ups

- Task 032 runs the founder-led cohesive local and optional-shared trial.
