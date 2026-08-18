import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import type { LocalGitHandoffPreview, ProjectSnapshot } from "../types";
import { LocalGitHandoff } from "./LocalGitHandoff";

vi.mock("../lib/bridge", async () => {
  const actual = await vi.importActual<typeof import("../lib/bridge")>("../lib/bridge");
  return {
    ...actual,
    previewLocalGitHandoff: vi.fn(),
    applyLocalGitHandoff: vi.fn(),
  };
});

const project: ProjectSnapshot = {
  root: "/tmp/review",
  name: "review",
  branch: "main",
  dirty: true,
  files: [],
  skills: [],
  errors: [],
};

const inspection: LocalGitHandoffPreview = {
  root: project.root,
  repository: { canonicalPath: project.root, repositoryId: "sha256:repo" },
  baseline: { head: "old-head", index: "sha256:index", worktree: "sha256:worktree" },
  currentStatus: [
    { path: "src/review.ts", status: " M" },
    { path: "unrelated.txt", status: " M" },
  ],
  candidates: [
    {
      path: "src/review.ts",
      status: " M",
      stagedEffect: "Stage this existing path and include it in one new local commit",
    },
  ],
  exclusions: [
    {
      path: "unrelated.txt",
      status: " M",
      code: "notInReviewReceipt",
      reason: "Current dirty path was not present in the supplied review receipt",
    },
  ],
  selectedPaths: [],
  proposedMessage: "",
  stagedEffects: [],
  previewToken: null,
  expiresAtMs: null,
  explicitConfirmationRequired: true,
  preExistingIndex: false,
  remoteEffects: [],
};

describe("LocalGitHandoff", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(bridge.previewLocalGitHandoff).mockResolvedValue(inspection);
  });

  it("requires separate inspection, selection, exact preview, and confirmation", async () => {
    const onProjectUpdate = vi.fn();
    render(
      <LocalGitHandoff
        root={project.root}
        receiptPaths={["src/review.ts"]}
        onProjectUpdate={onProjectUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Inspect local commit options" }));
    expect(await screen.findByLabelText(/src\/review.ts/)).toBeInTheDocument();
    expect(bridge.previewLocalGitHandoff).toHaveBeenNthCalledWith(
      1,
      project.root,
      ["src/review.ts"],
      [],
      "",
    );
    expect(screen.getByText(/1 excluded current or stale path/)).toBeInTheDocument();
    expect(bridge.applyLocalGitHandoff).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText(/src\/review.ts/));
    fireEvent.change(screen.getByLabelText("Reviewed commit message"), {
      target: { value: "Commit reviewed task result" },
    });
    vi.mocked(bridge.previewLocalGitHandoff).mockResolvedValueOnce({
      ...inspection,
      selectedPaths: ["src/review.ts"],
      proposedMessage: "Commit reviewed task result",
      stagedEffects: [
        "Stage `src/review.ts`",
        "Create one local commit with message `Commit reviewed task result`; no push or remote effect",
      ],
      previewToken: "git-handoff:bound",
      expiresAtMs: Date.now() + 60_000,
    });
    fireEvent.click(screen.getByRole("button", { name: "Preview selected local commit" }));
    const confirmation = await screen.findByRole("region", { name: "Local commit confirmation" });
    expect(within(confirmation).getByText("Stage `src/review.ts`")).toBeInTheDocument();
    expect(bridge.applyLocalGitHandoff).not.toHaveBeenCalled();

    vi.mocked(bridge.applyLocalGitHandoff).mockResolvedValue({
      success: true,
      outcome: "completed",
      commitCreated: true,
      previousHead: "old-head",
      newHead: "new-head",
      selectedPaths: ["src/review.ts"],
      stagedPaths: [],
      committedPaths: ["src/review.ts"],
      message: "Commit reviewed task result",
      repair: null,
      project: { ...project, dirty: false },
      remoteEffects: [],
    });
    fireEvent.click(within(confirmation).getByRole("button", { name: "Confirm and create local commit" }));
    expect(await screen.findByText("Local commit verified")).toBeInTheDocument();
    expect(bridge.applyLocalGitHandoff).toHaveBeenCalledWith(
      project.root,
      "git-handoff:bound",
      true,
    );
    expect(onProjectUpdate).toHaveBeenCalledWith({ ...project, dirty: false });
    expect(screen.getByText(/Remote effects: none/)).toBeInTheDocument();
  });

  it("shows a typed pre-mutation stop and never exposes confirmation", async () => {
    vi.mocked(bridge.previewLocalGitHandoff).mockRejectedValue({
      code: "git_handoff_index_not_clean",
      message: "The Git index already contains staged changes",
      path: project.root,
      committed: false,
    });
    render(
      <LocalGitHandoff
        root={project.root}
        receiptPaths={["src/review.ts"]}
        onProjectUpdate={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Inspect local commit options" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("git_handoff_index_not_clean");
    expect(screen.queryByRole("region", { name: "Local commit confirmation" })).not.toBeInTheDocument();
    await waitFor(() => expect(bridge.applyLocalGitHandoff).not.toHaveBeenCalled());
  });
});
