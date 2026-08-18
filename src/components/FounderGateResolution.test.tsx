import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import type { ArtifactPlanPreview, HelperResult, ProjectSnapshot } from "../types";
import { FounderGateResolution } from "./FounderGateResolution";

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
  root: "/tmp/founder-gate",
  name: "founder-gate",
  branch: "main",
  dirty: false,
  files: [],
  skills: [],
  errors: [],
};

const preview: ArtifactPlanPreview = {
  root: project.root,
  targets: [
    { path: "docs/raw/founder-dump.md", content: "# Context", contentVersion: "sha256:1", diff: "+# Context", effect: "create" },
    { path: "docs/mvp-scope.md", content: "# MVP", expectedVersion: "sha256:mvp", contentVersion: "sha256:2", diff: "+confirmed", effect: "update" },
    { path: "docs/blueprint-status.md", content: "# Status", expectedVersion: "sha256:blueprint", contentVersion: "sha256:3", diff: "+validated", effect: "update" },
  ],
  baseline: { head: "unborn", index: "sha256:index", worktree: "sha256:worktree" },
  previewToken: "artifact-plan:founder",
  expiresAtMs: 1_800_000_000_000,
  explicitConfirmationRequired: true,
  effectClass: "planMutation",
  collaborationEffects: [],
};

const readyResult: HelperResult = {
  helperId: "preflight-check",
  mode: "all",
  taskPath: null,
  executable: "bun",
  argv: ["preflight-check.ts"],
  outcome: "completed",
  executed: true,
  success: true,
  exitStatus: 0,
  stdout: "{}",
  stderr: "",
  stdoutTruncated: false,
  stderrTruncated: false,
  decision: { decision: "ready-for-execution", confidence: "high", nextAction: "Continue", evidence: [], warnings: [] },
  failure: null,
  project,
};

describe("FounderGateResolution", () => {
  beforeEach(() => vi.clearAllMocks());

  it("requires founder input, previews exact files, confirms once, and reruns preflight", async () => {
    vi.mocked(bridge.readProjectFile)
      .mockResolvedValueOnce({ path: "docs/mvp-scope.md", content: "## Validation Required Before Product Truth", version: "sha256:mvp" })
      .mockResolvedValueOnce({ path: "docs/blueprint-status.md", content: "MVP | needs-validation", version: "sha256:blueprint" });
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    vi.mocked(bridge.applyArtifactPlan).mockResolvedValue({
      success: true,
      committedPaths: preview.targets.map((target) => target.path),
      alreadyCommittedPaths: [],
      unappliedPaths: [],
      failureCode: null,
      failureMessage: null,
      project,
      collaborationEffects: [],
    });
    vi.mocked(bridge.executeHelper).mockResolvedValue(readyResult);
    const onPreflight = vi.fn();

    render(<FounderGateResolution
      project={project}
      decision={{ decision: "ask-founder", confidence: "medium", nextAction: "Answer", evidence: [], warnings: ["scope requires validation"], founderQuestions: ["founder context dump is missing"] }}
      disabled={false}
      onBusyChange={vi.fn()}
      onProjectChange={vi.fn()}
      onPreflight={onPreflight}
      onNotice={vi.fn()}
    />);

    const prepare = screen.getByRole("button", { name: "Preview founder gate resolution" });
    expect(prepare).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Founder context"), { target: { value: "The founder confirms one local checklist workflow." } });
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(prepare);

    expect(await screen.findByRole("region", { name: "Founder gate preview" })).toBeInTheDocument();
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Confirm founder gate resolution" }));

    await waitFor(() => expect(bridge.applyArtifactPlan).toHaveBeenCalledWith(project.root, preview.previewToken, true));
    expect(bridge.executeHelper).toHaveBeenCalledWith(project.root, { helperId: "preflight-check", mode: "all" });
    expect(onPreflight).toHaveBeenCalledWith(readyResult);
  });
});
