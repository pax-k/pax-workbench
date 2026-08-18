# Task 020 Frontend Extraction Evidence

Date: 2026-07-23
Source under test: repo-local path
Outcome: pass

## Proved

- Project/session and collaboration state are derived in focused pure modules.
- Root presentation calls explicit effect adapters; native bridge responses,
  repository files, and controller results remain authoritative.
- Active operation dominates repair context, Viewer remains inspection-only,
  and stale or absent project selection stays explicit.
- Existing behavior is covered by 59 focused Vitest tests, 91 full frontend
  tests, 212 Rust tests, typecheck, production build, Rust check, and format.

## Live Browser Smoke

Chromium opened the real Vite render at `http://127.0.0.1:1420`, displayed the
selected project and workflow, and opened the collaboration authority region
with focus and expanded-button semantics. The console contained only the React
development-tools information message.

This smoke proves rendered frontend composition. The browser path explicitly
uses simulated project data and disabled repository writes, so it does not
prove native effects.

Artifact: `output/playwright/task-020-collaboration-panel.png`

## Review Boundary

Independent subagent review was unavailable for this run. Equivalent
verification comprised focused compatibility suites, full frontend/native
regressions, live UI interaction, and a local diff review.
