import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as bridge from "../lib/bridge";
import type { ArtifactPlanPreview, HelperResult, ProjectSnapshot } from "../types";
import { DiscoverBootstrap } from "./DiscoverBootstrap";

vi.mock("../lib/bridge", async () => {
  const actual = await vi.importActual<typeof import("../lib/bridge")>("../lib/bridge");
  return {
    ...actual,
    previewArtifactPlan: vi.fn(),
    applyArtifactPlan: vi.fn(),
    executeHelper: vi.fn(),
  };
});

const project: ProjectSnapshot = {
  root: "/tmp/discover",
  name: "discover",
  branch: "main",
  dirty: false,
  files: [],
  skills: [],
  errors: [],
};

const preview: ArtifactPlanPreview = {
  root: project.root,
  targets: [{ path: "AGENTS.md", content: "# Rules", contentVersion: "sha256:x", diff: "+# Rules", effect: "create" }],
  baseline: { head: "unborn", index: "sha256:a", worktree: "sha256:b" },
  previewToken: "artifact-plan:test",
  expiresAtMs: 1_800_000_000_000,
  explicitConfirmationRequired: true,
  effectClass: "planMutation",
  collaborationEffects: [],
};

const appliedProject: ProjectSnapshot = {
  ...project,
  files: [{ path: "AGENTS.md", name: "AGENTS", kind: "instruction" }],
};

const preflightResult: HelperResult = {
  helperId: "preflight-check",
  mode: "all",
  taskPath: null,
  executable: "bun",
  argv: ["preflight-check.ts", "--cwd", project.root, "--mode", "all", "--format", "json"],
  outcome: "completed",
  executed: true,
  success: true,
  exitStatus: 0,
  stdout: "{}",
  stderr: "",
  stdoutTruncated: false,
  stderrTruncated: false,
  decision: {
    decision: "ready-for-execution",
    confidence: "high",
    nextAction: "Execute task 001",
    evidence: ["authority present"],
    warnings: [],
  },
  failure: null,
  project: appliedProject,
};

function props() {
  return {
    project,
    nativeAvailable: true,
    preflightAvailable: false,
    disabled: false,
    onBusyChange: vi.fn(),
    onProjectChange: vi.fn(),
    onPreflight: vi.fn(),
    onNotice: vi.fn(),
  };
}

function answerFounderQuestions() {
  fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "Signal Forge" } });
  fireEvent.change(screen.getByLabelText("Primary user"), { target: { value: "Independent founder" } });
  fireEvent.change(screen.getByLabelText("Current workflow"), { target: { value: "Turn an idea into verified work" } });
  fireEvent.change(screen.getByLabelText("Value moment"), { target: { value: "One evidence-backed ready task" } });
  fireEvent.change(screen.getByLabelText("Hard constraint"), { target: { value: "Markdown remains authority" } });
}

describe("DiscoverBootstrap", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not fabricate drafts or write before complete input and confirmation", async () => {
    render(<DiscoverBootstrap {...props()} />);
    expect(screen.getByRole("button", { name: /Preview 12 authority files/ })).toBeDisabled();
    expect(bridge.previewArtifactPlan).not.toHaveBeenCalled();
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();

    answerFounderQuestions();
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    fireEvent.click(screen.getByRole("button", { name: /Preview 12 authority files/ }));

    await screen.findByRole("region", { name: "Artifact creation preview" });
    expect(bridge.previewArtifactPlan).toHaveBeenCalledTimes(1);
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();
  });

  it("applies only after the second action and keeps shared coordinates absent", async () => {
    const callbacks = props();
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    vi.mocked(bridge.applyArtifactPlan).mockResolvedValue({
      success: true,
      committedPaths: ["AGENTS.md"],
      alreadyCommittedPaths: [],
      unappliedPaths: [],
      failureCode: null,
      failureMessage: null,
      project: appliedProject,
      collaborationEffects: [],
    });
    render(<DiscoverBootstrap {...callbacks} />);
    answerFounderQuestions();
    fireEvent.click(screen.getByRole("button", { name: /Preview 12 authority files/ }));
    await screen.findByRole("region", { name: "Artifact creation preview" });
    fireEvent.click(screen.getByRole("button", { name: "Confirm and create files" }));

    await waitFor(() => expect(bridge.applyArtifactPlan).toHaveBeenCalledWith(
      project.root,
      preview.previewToken,
      true,
    ));
    expect(JSON.stringify(vi.mocked(bridge.applyArtifactPlan).mock.calls)).not.toContain("session");
    expect(callbacks.onProjectChange).toHaveBeenCalled();
  });

  it("cancels a prepared preview without applying any files", async () => {
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    render(<DiscoverBootstrap {...props()} />);
    answerFounderQuestions();
    fireEvent.click(screen.getByRole("button", { name: /Preview 12 authority files/ }));
    await screen.findByRole("region", { name: "Artifact creation preview" });

    fireEvent.click(screen.getByRole("button", { name: "Cancel artifact preview" }));

    expect(screen.queryByRole("region", { name: "Artifact creation preview" })).not.toBeInTheDocument();
    expect(bridge.applyArtifactPlan).not.toHaveBeenCalled();
  });

  it("surfaces a typed stale-preview rejection and does not run preflight", async () => {
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    vi.mocked(bridge.applyArtifactPlan).mockRejectedValue({
      code: "artifact_plan_stale",
      message: "Repository baseline changed after preview",
    });
    render(<DiscoverBootstrap {...props()} preflightAvailable />);
    answerFounderQuestions();
    fireEvent.click(screen.getByRole("button", { name: /Preview 12 authority files/ }));
    await screen.findByRole("region", { name: "Artifact creation preview" });
    fireEvent.click(screen.getByRole("button", { name: "Confirm and create files" }));

    const error = await screen.findByRole("alert");
    expect(error).toHaveTextContent("artifact_plan_stale");
    expect(error).toHaveTextContent("Repository baseline changed after preview");
    expect(bridge.executeHelper).not.toHaveBeenCalled();
  });

  it("reports committed and unapplied paths from a partial create receipt", async () => {
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    vi.mocked(bridge.applyArtifactPlan).mockResolvedValue({
      success: false,
      committedPaths: ["AGENTS.md"],
      alreadyCommittedPaths: [],
      unappliedPaths: ["docs/source-index.md"],
      failureCode: "artifact_partial_apply",
      failureMessage: "A later create failed",
      project: appliedProject,
      collaborationEffects: [],
    });
    render(<DiscoverBootstrap {...props()} />);
    answerFounderQuestions();
    fireEvent.click(screen.getByRole("button", { name: /Preview 12 authority files/ }));
    await screen.findByRole("region", { name: "Artifact creation preview" });
    fireEvent.click(screen.getByRole("button", { name: "Confirm and create files" }));

    const error = await screen.findByRole("alert");
    expect(error).toHaveTextContent("artifact_partial_apply");
    expect(error).toHaveTextContent("Committed: AGENTS.md");
    expect(error).toHaveTextContent("Unapplied: docs/source-index.md");
  });

  it("runs project preflight after a successful confirmed apply", async () => {
    const callbacks = props();
    vi.mocked(bridge.previewArtifactPlan).mockResolvedValue(preview);
    vi.mocked(bridge.applyArtifactPlan).mockResolvedValue({
      success: true,
      committedPaths: ["AGENTS.md"],
      alreadyCommittedPaths: [],
      unappliedPaths: [],
      failureCode: null,
      failureMessage: null,
      project: appliedProject,
      collaborationEffects: [],
    });
    vi.mocked(bridge.executeHelper).mockResolvedValue(preflightResult);
    render(<DiscoverBootstrap {...callbacks} preflightAvailable />);
    answerFounderQuestions();
    fireEvent.click(screen.getByRole("button", { name: /Preview 12 authority files/ }));
    await screen.findByRole("region", { name: "Artifact creation preview" });
    fireEvent.click(screen.getByRole("button", { name: "Confirm and create files" }));

    await waitFor(() => expect(bridge.executeHelper).toHaveBeenCalledWith(project.root, {
      helperId: "preflight-check",
      mode: "all",
    }));
    expect(callbacks.onPreflight).toHaveBeenCalledWith(preflightResult);
    expect(callbacks.onProjectChange).toHaveBeenLastCalledWith(appliedProject);
  });
});
