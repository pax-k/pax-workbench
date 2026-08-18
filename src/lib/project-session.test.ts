import { describe, expect, it } from "vitest";
import { parseTask } from "./markdown";
import {
  deriveProjectSessionProjection,
  deriveWorkflowCheckpoints,
  isExecutionHelperTaskPath,
} from "./project-session";

describe("project session projection", () => {
  it("keeps absent selection explicit and never auto-selects authority", () => {
    expect(
      deriveProjectSessionProjection({
        isDemo: false,
        activeFilePath: "",
        markdown: "# Select",
        loadedMarkdown: "# Select",
        staleConflict: false,
        pendingNavigationPath: null,
        pendingProjectSwitch: false,
        operationRunning: false,
      }),
    ).toEqual({
      mode: "repository",
      selectedPath: null,
      selection: "absent",
      draft: "clean",
      pendingAction: "none",
      mutationBlocked: false,
      automaticSelectionPerformed: false,
    });
  });

  it("projects dirty navigation and stale reload without changing the source", () => {
    expect(
      deriveProjectSessionProjection({
        isDemo: false,
        activeFilePath: "docs/mvp-scope.md",
        markdown: "# Draft",
        loadedMarkdown: "# Disk",
        staleConflict: true,
        pendingNavigationPath: "tasks/sprint-3.md",
        pendingProjectSwitch: false,
        operationRunning: false,
      }),
    ).toMatchObject({
      selection: "stale",
      draft: "dirty",
      pendingAction: "navigate",
      mutationBlocked: true,
    });
  });

  it("derives workflow checkpoints from parsed repository Markdown", () => {
    const checkpoints = deriveWorkflowCheckpoints(
      parseTask("# 020: Extract\n\nStatus: ready\nOwner: AI\nRequirement basis: docs/audit.md\n"),
    );
    expect(checkpoints.map((checkpoint) => checkpoint.state)).toEqual([
      "done",
      "done",
      "ready",
      "ready",
      "waiting",
    ]);
  });

  it("accepts only supported inventoried task path shapes", () => {
    expect(isExecutionHelperTaskPath("tasks/issues/020-test.md")).toBe(true);
    expect(isExecutionHelperTaskPath("tasks/root-task.md")).toBe(true);
    expect(isExecutionHelperTaskPath("tasks/issues/nested/020-test.md")).toBe(false);
    expect(isExecutionHelperTaskPath("tasks/sprint-0.md")).toBe(false);
  });
});
