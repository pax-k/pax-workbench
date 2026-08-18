import { describe, expect, it } from "vitest";
import type { ProjectFile } from "../types";
import { projectNavigationGroups, taskPathsFromTracker } from "./navigation";

const files: ProjectFile[] = [
  { path: "docs/blueprint-status.md", name: "Blueprint", kind: "document", status: "active" },
  { path: "docs/evidence/proof.md", name: "Proof", kind: "evidence", status: "complete" },
  { path: "tasks/sprint-3.md", name: "Sprint 3", kind: "task", status: "active" },
  { path: "tasks/issues/028-proof.md", name: "028 proof", kind: "task", status: "complete" },
  { path: "tasks/issues/029-layout.md", name: "029 layout", kind: "task", status: "ready" },
  { path: "tasks/issues/099-unassigned.md", name: "099 unassigned", kind: "task", status: "blocked" },
];

describe("project navigation model", () => {
  it("parses exact issue paths and groups tasks under their sprint tracker", () => {
    expect(taskPathsFromTracker("| 029 | Layout | ready | x | tasks/issues/029-layout.md |")).toEqual([
      "tasks/issues/029-layout.md",
    ]);
    const groups = projectNavigationGroups({
      files,
      trackerMarkdown: {
        "tasks/sprint-3.md": [
          "| 028 | Proof | complete | x | tasks/issues/028-proof.md |",
          "| 029 | Layout | ready | 028 | tasks/issues/029-layout.md |",
        ].join("\n"),
      },
      query: "",
      status: "all",
    });
    expect(groups.map((group) => group.label)).toEqual([
      "Project authority",
      "Sprint 3",
      "Unassigned tasks",
      "Evidence",
    ]);
    expect(groups[1].files.map((file) => file.path)).toEqual([
      "tasks/sprint-3.md",
      "tasks/issues/028-proof.md",
      "tasks/issues/029-layout.md",
    ]);
  });

  it("filters by status and path/name query without inventing task state", () => {
    const groups = projectNavigationGroups({
      files,
      trackerMarkdown: {
        "tasks/sprint-3.md": "| 029 | Layout | ready | 028 | tasks/issues/029-layout.md |",
      },
      query: "layout",
      status: "ready",
    });
    expect(groups).toHaveLength(1);
    expect(groups[0].files.map((file) => file.path)).toEqual([
      "tasks/issues/029-layout.md",
    ]);
  });
});
