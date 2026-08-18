import { beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";
import {
  applyArtifactPlan,
  applyLocalGitHandoff,
  applyHa2haPublish,
  connectMdsyncSession,
  disconnectMdsyncSession,
  executeSharedBoundedTask,
  joinHa2haWorkspace,
  previewHa2haPublish,
  previewArtifactPlan,
  previewLocalGitHandoff,
  previewSharedBoundedTask,
  repairCollaborationCompletion,
} from "./bridge";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: class TestChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("collaboration native bridge", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("passes the handoff only to the native connect command", async () => {
    const opaqueAlias = "opaque-native-boundary-018";
    const handoff = `https://sync.example.test/workspaces/ws-018?edit=${opaqueAlias}`;

    await connectMdsyncSession("/tmp/task-018", handoff, "build-right-studio");
    await disconnectMdsyncSession(
      "/tmp/task-018",
      `local-session-${"1".repeat(32)}`,
    );

    expect(invoke).toHaveBeenNthCalledWith(1, "connect_mdsync_session", {
      root: "/tmp/task-018",
      workspaceUrl: handoff,
      actor: "build-right-studio",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "disconnect_mdsync_session", {
      root: "/tmp/task-018",
      sessionId: `local-session-${"1".repeat(32)}`,
    });
    expect(JSON.stringify(vi.mocked(invoke).mock.calls[1])).not.toContain(opaqueAlias);
  });

  it("maps publish, inspect, preview, execute, and repair to closed typed command arguments", async () => {
    const root = "/tmp/task-018";
    const sessionId = `local-session-${"2".repeat(32)}`;

    await previewHa2haPublish(root, sessionId);
    await applyHa2haPublish(root, sessionId, "publish-preview-safe", true);
    await joinHa2haWorkspace(root, sessionId);
    await previewSharedBoundedTask(root, sessionId);
    await executeSharedBoundedTask(
      root,
      sessionId,
      {
        previewToken: "shared-preview-safe",
        selectedTask: "tasks/issues/018-add-shared-collaboration-and-repair-ui.md",
        mode: "fixture",
        confirmed: true,
      },
      vi.fn(),
    );
    await repairCollaborationCompletion(root, sessionId, true);

    expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual([
      "preview_ha2ha_publish",
      "apply_ha2ha_publish",
      "join_ha2ha_workspace",
      "preview_shared_bounded_task",
      "execute_shared_bounded_task",
      "repair_collaboration_completion",
    ]);
    expect(invoke).toHaveBeenNthCalledWith(2, "apply_ha2ha_publish", {
      root,
      sessionId,
      previewToken: "publish-preview-safe",
      confirmed: true,
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "join_ha2ha_workspace", {
      root,
      sessionId,
    });
    expect(invoke).toHaveBeenNthCalledWith(4, "preview_shared_bounded_task", {
      root,
      sessionId,
    });
    expect(invoke).toHaveBeenNthCalledWith(5, "execute_shared_bounded_task", {
      root,
      sessionId,
      invocation: {
        previewToken: "shared-preview-safe",
        selectedTask: "tasks/issues/018-add-shared-collaboration-and-repair-ui.md",
        mode: "fixture",
        confirmed: true,
      },
      onEvent: expect.any(Object),
    });
    expect(invoke).toHaveBeenNthCalledWith(6, "repair_collaboration_completion", {
      root,
      sessionId,
      confirmed: true,
    });
  });

  it("keeps artifact planning local and passes only exact drafts plus confirmation", async () => {
    const root = "/tmp/task-023";
    const targets = [
      { path: "docs/mvp-scope.md", content: "# Scope\n" },
      { path: "tasks/sprint-0.md", content: "# Sprint 0\n" },
    ];

    await previewArtifactPlan(root, targets);
    await applyArtifactPlan(root, "artifact-plan:bound", true);

    expect(invoke).toHaveBeenNthCalledWith(1, "preview_artifact_plan", {
      root,
      targets,
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "apply_artifact_plan", {
      root,
      previewToken: "artifact-plan:bound",
      confirmed: true,
    });
    expect(JSON.stringify(vi.mocked(invoke).mock.calls)).not.toContain("sessionId");
    expect(JSON.stringify(vi.mocked(invoke).mock.calls)).not.toContain("workspaceUrl");
  });

  it("keeps local Git handoff path-scoped and exposes no remote operation", async () => {
    const root = "/tmp/task-028a";
    await previewLocalGitHandoff(
      root,
      ["src/review.ts", "docs/evidence.md"],
      ["src/review.ts"],
      "Commit reviewed task result",
    );
    await applyLocalGitHandoff(root, "git-handoff:bound", true);

    expect(invoke).toHaveBeenNthCalledWith(1, "preview_local_git_handoff", {
      root,
      receiptPaths: ["src/review.ts", "docs/evidence.md"],
      selectedPaths: ["src/review.ts"],
      proposedMessage: "Commit reviewed task result",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "apply_local_git_handoff", {
      root,
      previewToken: "git-handoff:bound",
      confirmed: true,
    });
    expect(JSON.stringify(vi.mocked(invoke).mock.calls)).not.toMatch(
      /push|publish|remote|workspaceUrl|sessionId/,
    );
  });
});
