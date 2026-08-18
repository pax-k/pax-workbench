import { describe, expect, it } from "vitest";
import type { HelperDecision, ProjectSnapshot } from "../types";
import {
  buildPlanningDrafts,
  planningCanPropose,
  validateFeatureRequest,
} from "./feature-planning";

const project: ProjectSnapshot = {
  root: "/tmp/plan",
  name: "plan",
  branch: "main",
  dirty: false,
  files: [
    { path: "tasks/sprint-3.md", name: "Sprint 3", kind: "task" },
    { path: "tasks/issues/025-current.md", name: "Current", kind: "task" },
  ],
  skills: [],
  errors: [],
};

const decision: HelperDecision = {
  decision: "update-sprint",
  confidence: "medium",
  nextAction: "Create bounded work",
  evidence: [],
  warnings: [],
  recommendedDestination: "tasks/sprint-3.md",
  blockingGates: [],
  founderQuestions: [],
  researchTriggers: [],
  readyTaskCandidates: [],
};

const tracker = `# Sprint 3

## Tasks

| ID | Title | Status | Depends On | Evidence |
| --- | --- | --- | --- | --- |
| 025 | Current | complete | 024 | tasks/issues/025-current.md |

## Sprint Exit

- One ready task.
`;

describe("feature planning proposal model", () => {
  it("validates bounded founder input and refuses gated proposals", () => {
    expect(validateFeatureRequest("")).toMatch(/Describe/);
    expect(validateFeatureRequest("Ship keyboard navigation")).toBeNull();
    expect(planningCanPropose({ ...decision, founderQuestions: ["Which user?"] })).toBe(false);
    expect(buildPlanningDrafts(project, "Ship keyboard navigation", { ...decision, blockingGates: [{ type: "conflict", source: "docs/conflicts.md", reason: "unresolved" }] }, tracker, "v1")).toEqual([]);
  });

  it("creates one editable ready task plus an exact-version tracker update", () => {
    const drafts = buildPlanningDrafts(project, "Add a keyboard-first command palette.", decision, tracker, "sha256:tracker", "2026-07-23");
    expect(drafts).toHaveLength(2);
    expect(drafts[0].path).toBe("tasks/issues/026-add-a-keyboard-first-command-palette.md");
    expect(drafts[0].content).toContain("Status: ready");
    expect(drafts[0].content).toContain("## Required Reading");
    expect(drafts[0].content).toContain("## Baseline Evidence");
    expect(drafts[0].content).toContain("## Follow-Ups");
    expect(drafts[1]).toMatchObject({ path: "tasks/sprint-3.md", expectedVersion: "sha256:tracker" });
    expect(drafts[1].content).toContain("| 026 | Add a keyboard-first command palette | ready |");
    expect(JSON.stringify(drafts)).not.toMatch(/HA2HA|MDSync/i);
  });
});
