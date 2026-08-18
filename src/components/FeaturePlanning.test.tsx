import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import type { ArtifactPlanPreview, HelperResult, ProjectSnapshot } from "../types";
import { FeaturePlanning } from "./FeaturePlanning";

vi.mock("../lib/bridge", async () => {
  const actual = await vi.importActual<typeof import("../lib/bridge")>("../lib/bridge");
  return {
    ...actual,
    readProjectFile: vi.fn(),
    previewArtifactPlan: vi.fn(),
    applyArtifactPlan: vi.fn(),
    executeHelper: vi.fn(),
  };
});

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
const planningResult: HelperResult = {
  helperId: "feature-planning-check",
  mode: null,
  taskPath: null,
  executable: "bun",
  argv: ["-", "--feature", "Add command palette"],
  outcome: "completed",
  executed: true,
  success: true,
  exitStatus: 0,
  stdout: "{}",
  stderr: "",
  stdoutTruncated: false,
  stderrTruncated: false,
  decision: {
    decision: "update-sprint",
    confidence: "medium",
    nextAction: "Update active sprint",
    evidence: [],
    warnings: [],
    recommendedDestination: "tasks/sprint-3.md",
    blockingGates: [],
    founderQuestions: [],
    researchTriggers: [],
    readyTaskCandidates: [],
  },
  failure: null,
  project,
};
const preview: ArtifactPlanPreview = {
  root: project.root,
  targets: [
    { path: "tasks/issues/026-add-command-palette.md", content: "# 026", contentVersion: "a", diff: "+# 026", effect: "create" },
    { path: "tasks/sprint-3.md", content: "# Sprint 3", contentVersion: "b", diff: "-old\n+new", effect: "update" },
  ],
  baseline: { head: "h", index: "i", worktree: "w" },
  previewToken: "artifact-plan:test",
  expiresAtMs: 1,
  explicitConfirmationRequired: true,
  effectClass: "planMutation",
  collaborationEffects: [],
};
function props() {
  return {
    project,
    nativeAvailable: true,
    disabled: false,
    onBusyChange: vi.fn(),
    onProjectChange: vi.fn(),
    onNotice: vi.fn(),
  };
}

describe("FeaturePlanning", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(bridge.readProjectFile).mockReset();
    vi.mocked(bridge.previewArtifactPlan).mockReset();
    vi.mocked(bridge.applyArtifactPlan).mockReset();
    vi.mocked(bridge.executeHelper).mockReset();
  });

  it("shows typed questions and never proposes or writes through a gate", async () => {
    vi.mocked(bridge.executeHelper).mockResolvedValue({
      ...planningResult,
      decision: { ...planningResult.decision!, decision: "ask-founder", founderQuestions: ["Which user owns this outcome?"] },
    });
    render(<FeaturePlanning {...props()} />);
    fireEvent.change(screen.getByLabelText("Feature request"), { target: { value: "Add command palette" } });
    fireEvent.click(screen.getByRole("button", { name: "Run repository planning check" }));
    expect(await screen.findByText("Which user owns this outcome?")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Draft bounded/ })).not.toBeInTheDocument();
    expect(bridge.previewArtifactPlan).not.toHaveBeenCalled();
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();
  });

  it("requires helper, editable proposal, exact preview, confirmation, and two readback checks", async () => {
    vi.mocked(bridge.executeHelper)
      .mockResolvedValueOnce(planningResult)
      .mockResolvedValueOnce({ ...planningResult, decision: { ...planningResult.decision!, decision: "create-ready-tasks", readyTaskCandidates: [{ id: "026", title: "Add command palette", status: "ready", owner: "AI", path: "tasks/issues/026-add-command-palette.md", tracker: "tasks/sprint-3.md" }] } })
      .mockResolvedValueOnce({ ...planningResult, helperId: "continue-check", decision: { ...planningResult.decision!, decision: "execute-task", nextAction: "Execute 026" } });
    vi.mocked(bridge.readProjectFile).mockResolvedValue({
      path: "tasks/sprint-3.md",
      content: "# Sprint 3\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 025 | Current | complete | 024 | tasks/issues/025-current.md |\n",
      version: "tracker-v1",
    });
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    vi.mocked(bridge.applyArtifactPlan).mockResolvedValue({
      success: true,
      committedPaths: ["tasks/issues/026-add-command-palette.md", "tasks/sprint-3.md"],
      alreadyCommittedPaths: [],
      unappliedPaths: [],
      failureCode: null,
      failureMessage: null,
      project,
      collaborationEffects: [],
    });
    render(<FeaturePlanning {...props()} />);
    fireEvent.change(screen.getByLabelText("Feature request"), { target: { value: "Add command palette" } });
    fireEvent.click(screen.getByRole("button", { name: "Run repository planning check" }));
    fireEvent.click(await screen.findByRole("button", { name: /Draft bounded task/ }));
    const editor = await screen.findByLabelText(/Proposed content for tasks\/issues/);
    fireEvent.change(editor, { target: { value: `${String((editor as HTMLTextAreaElement).value)}\nFounder edit.\n` } });
    fireEvent.click(screen.getByRole("button", { name: "Preview exact diffs" }));
    await waitFor(() => expect(screen.getByText((_, element) => element?.tagName === "PRE" && element.textContent === "-old\n+new")).toBeInTheDocument());
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Confirm and apply" }));
    await screen.findByRole("region", { name: "Planning verification receipt" });
    expect(bridge.applyArtifactPlan).toHaveBeenCalledWith(project.root, "artifact-plan:test", true);
    expect(bridge.executeHelper).toHaveBeenNthCalledWith(2, project.root, { helperId: "feature-planning-check", featureRequest: "Add command palette" });
    expect(bridge.executeHelper).toHaveBeenNthCalledWith(3, project.root, { helperId: "continue-check" });
    expect(screen.getByText("Shared publications: 0.")).toBeInTheDocument();
  });

  it("cancels without applying and invalidates state when input changes", async () => {
    vi.mocked(bridge.executeHelper).mockResolvedValue(planningResult);
    render(<FeaturePlanning {...props()} />);
    fireEvent.change(screen.getByLabelText("Feature request"), { target: { value: "Add command palette" } });
    fireEvent.click(screen.getByRole("button", { name: "Run repository planning check" }));
    await screen.findByRole("heading", { name: /update-sprint · medium/ });
    fireEvent.click(screen.getByRole("button", { name: "Cancel and clear" }));
    await waitFor(() => expect(screen.queryByRole("heading", { name: /update-sprint · medium/ })).not.toBeInTheDocument());
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();
  });

  it("resumes from repository truth by showing an existing ready candidate without fabricating another proposal", async () => {
    vi.mocked(bridge.executeHelper).mockResolvedValue({
      ...planningResult,
      decision: {
        ...planningResult.decision!,
        decision: "create-ready-tasks",
        readyTaskCandidates: [{ id: "026", title: "Existing task", status: "ready", owner: "AI", path: "tasks/issues/026-existing.md", tracker: "tasks/sprint-3.md" }],
      },
    });
    render(<FeaturePlanning {...props()} />);
    fireEvent.change(screen.getByLabelText("Feature request"), { target: { value: "Resume planned feature" } });
    fireEvent.click(screen.getByRole("button", { name: "Run repository planning check" }));
    expect(await screen.findByText("tasks/issues/026-existing.md")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Draft bounded/ })).not.toBeInTheDocument();
    expect(bridge.previewArtifactPlan).not.toHaveBeenCalled();
  });
});
