import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import type { ProjectSnapshot } from "../types";
import { ProjectFileNavigation } from "./ProjectFileNavigation";

vi.mock("../lib/bridge", async () => {
  const actual = await vi.importActual<typeof import("../lib/bridge")>("../lib/bridge");
  return { ...actual, readProjectFile: vi.fn() };
});

const project: ProjectSnapshot = {
  root: "/tmp/navigation",
  name: "navigation",
  branch: "main",
  dirty: true,
  skills: [],
  errors: [],
  files: [
    { path: "docs/blueprint-status.md", name: "Blueprint", kind: "document", status: "active" },
    { path: "tasks/sprint-3.md", name: "Sprint 3", kind: "task", status: "active" },
    { path: "tasks/issues/028-proof.md", name: "028 proof", kind: "task", status: "complete" },
    { path: "tasks/issues/029-layout.md", name: "029 layout", kind: "task", status: "ready" },
  ],
};

describe("ProjectFileNavigation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(bridge.readProjectFile).mockResolvedValue({
      path: "tasks/sprint-3.md",
      version: "v1",
      content: [
        "| 028 | Proof | complete | x | tasks/issues/028-proof.md |",
        "| 029 | Layout | ready | 028 | tasks/issues/029-layout.md |",
      ].join("\n"),
    });
  });

  it("loads sprint membership, supports filtering, and exposes history controls", async () => {
    const onOpen = vi.fn();
    render(
      <ProjectFileNavigation
        project={project}
        activeFilePath="tasks/issues/029-layout.md"
        selectedTaskPath="tasks/issues/029-layout.md"
        nativeAvailable
        disabled={false}
        canGoBack
        canGoForward={false}
        onBack={vi.fn()}
        onForward={vi.fn()}
        onOpen={onOpen}
      />,
    );
    await waitFor(() => expect(bridge.readProjectFile).toHaveBeenCalled());
    const sprint = screen.getByRole("button", { name: "Open file tasks/issues/029-layout.md" }).closest("details");
    expect(sprint).not.toBeNull();
    expect(within(sprint!).getByRole("button", { name: "Open file tasks/issues/029-layout.md" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Previous document" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Next document" })).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Filter project files by status"), {
      target: { value: "ready" },
    });
    expect(screen.queryByRole("button", { name: /028 proof/ })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Search project files"), {
      target: { value: "layout" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Open file tasks/issues/029-layout.md" }));
    expect(onOpen).toHaveBeenCalledWith("tasks/issues/029-layout.md");
  });
});
