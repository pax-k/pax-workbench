import type { ProjectSnapshot, RunEvent, WorkflowCheckpoint } from "../types";
import { firstPartySkillSummaries } from "./skill-contracts";

export const demoTaskMarkdown = `# 001: Build Local Workbench MVP

Status: active
Type: foundation
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/mvp-scope.md
Reversibility: easy

## Goal

Create a runnable workbench that makes one repository's Build Right artifacts,
workflow state, and run evidence legible without a shadow task database.

## Acceptance Criteria

- [x] Founder discussion captured as repository Markdown.
- [x] Product scope and execution gates are explicit.
- [ ] Workbench interface passes its validation ladder.
- [ ] Native Tauri boundary is verified when Rust is available.

## Verification

- bun run typecheck
- bun run test
- bun run build
`;

export const demoProject: ProjectSnapshot = {
  root: "/Users/pax/Documents/Repos/pax-workbench",
  name: "pax-workbench",
  branch: "main",
  dirty: true,
  files: [
    { path: "docs/blueprint-status.md", name: "Blueprint status", kind: "document", status: "ready" },
    { path: "docs/mvp-scope.md", name: "MVP scope", kind: "document", status: "active" },
    { path: "docs/decision-log.md", name: "Decision log", kind: "document", status: "active" },
    { path: "docs/evidence/preflight.md", name: "Preflight evidence", kind: "evidence", status: "recorded" },
    { path: "tasks/sprint-0.md", name: "Sprint 0", kind: "task", status: "active" },
    { path: "tasks/issues/001-build-local-workbench-mvp.md", name: "001 · Local workbench", kind: "task", status: "active" },
  ],
  skills: firstPartySkillSummaries,
  errors: [],
};

export const initialEvents: RunEvent[] = [
  { id: "e1", time: "10:42", label: "Project rules read", detail: "Loaded authority order and current task boundary.", kind: "read", simulated: true, provenance: "simulated" },
  { id: "e2", time: "10:42", label: "Resolver decision", detail: "execute-task · confidence high", kind: "decision", simulated: true, provenance: "simulated" },
  { id: "e3", time: "10:43", label: "Baseline captured", detail: "No app package or source existed; Bun available.", kind: "evidence", simulated: true, provenance: "simulated" },
];

export const simulatedEvents: RunEvent[] = [
  { id: "e4", time: "10:44", label: "Implementation planned", detail: "React workbench + typed native command boundary.", kind: "decision", simulated: true, provenance: "simulated" },
  { id: "e5", time: "10:45", label: "Files changed", detail: "Frontend shell, parser, bridge, Tauri commands, and tests.", kind: "edit", simulated: true, provenance: "simulated" },
  { id: "e6", time: "10:47", label: "Verification started", detail: "typecheck → tests → production build", kind: "command", simulated: true, provenance: "simulated" },
  { id: "e7", time: "10:48", label: "Evidence recorded", detail: "Task receipt updated from durable command results.", kind: "verify", simulated: true, provenance: "simulated" },
];

export const checkpoints: WorkflowCheckpoint[] = [
  { id: "discover", label: "Preflight", detail: "Product truth ready", state: "done" },
  { id: "plan", label: "Plan", detail: "Task 001 bounded", state: "done" },
  { id: "task", label: "Task 001", detail: "Implementation active", state: "active" },
  { id: "verify", label: "Verify", detail: "Checks pending", state: "ready" },
  { id: "next", label: "Next action", detail: "Resolver pending", state: "waiting" },
];
