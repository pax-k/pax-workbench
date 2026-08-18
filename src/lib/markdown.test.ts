import { describe, expect, it } from "vitest";
import { extractSprintRows, parseTask } from "./markdown";

describe("parseTask", () => {
  it("projects canonical task fields without changing the source", () => {
    const source = `# 023: Add onboarding state\n\nStatus: active\nOwner: AI\n\n## Goal\n\nMake state visible.\n\n## Acceptance Criteria\n\n- [x] Existing state parsed\n- [ ] Empty state shown\n`;
    expect(parseTask(source)).toEqual({
      id: "023",
      title: "Add onboarding state",
      status: "active",
      owner: "AI",
      requirementBasis: "unknown",
      goal: "Make state visible.",
      acceptanceCriteria: [
        { checked: true, text: "Existing state parsed" },
        { checked: false, text: "Empty state shown" },
      ],
    });
    expect(source).toContain("- [ ] Empty state shown");
  });

  it("returns explicit unknown values for incomplete Markdown", () => {
    expect(parseTask("notes only")).toMatchObject({
      id: "—",
      title: "Untitled task",
      status: "unknown",
      owner: "unknown",
      acceptanceCriteria: [],
    });
  });
});

describe("extractSprintRows", () => {
  it("extracts task rows and ignores table headers", () => {
    const source = `| ID | Title | Status | Evidence |\n| --- | --- | --- | --- |\n| 001 | Establish baseline | complete | task.md |\n| 002 | Build shell | ready | task-2.md |`;
    expect(extractSprintRows(source)).toEqual([
      { id: "001", title: "Establish baseline", status: "complete" },
      { id: "002", title: "Build shell", status: "ready" },
    ]);
  });
});
