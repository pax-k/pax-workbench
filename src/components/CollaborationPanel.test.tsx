import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as bridge from "../lib/bridge";
import {
  createLocalSessionHandle,
  type SanitizedSessionMetadata,
} from "../lib/collaboration";
import type {
  GoalRecovery,
  Ha2haJoinResult,
  SharedBoundedTaskPreview,
  SharedBoundedTaskResult,
} from "../types";
import { CollaborationPanel, type CollaborationPanelProps } from "./CollaborationPanel";

vi.mock("../lib/bridge", () => ({
  applyHa2haPublish: vi.fn(),
  connectMdsyncSession: vi.fn(),
  disconnectMdsyncSession: vi.fn(),
  executeSharedBoundedTask: vi.fn(),
  joinHa2haWorkspace: vi.fn(),
  previewHa2haPublish: vi.fn(),
  previewSharedBoundedTask: vi.fn(),
  recoverGoalState: vi.fn(),
  repairCollaborationCompletion: vi.fn(),
}));

const localBinding = {
  taskPath: "tasks/issues/018-add-shared-collaboration-and-repair-ui.md",
  taskSha256: `sha256:${"a".repeat(64)}`,
  repositoryId: `sha256:${"b".repeat(64)}`,
  gitHead: null,
  gitIndexSha256: `sha256:${"c".repeat(64)}`,
  gitWorktreeSha256: `sha256:${"d".repeat(64)}`,
  gitDirty: true,
};

const collaboratorSession: SanitizedSessionMetadata = {
  sessionId: createLocalSessionHandle(`local-session-${"1".repeat(32)}`),
  workspaceId: "workspace-safe-018",
  webOrigin: "https://sync.example.test",
  apiOrigin: "https://sync-api.example.test",
  access: "collaborator",
  actor: "build-right-studio",
};

const viewerSession: SanitizedSessionMetadata = {
  ...collaboratorSession,
  sessionId: createLocalSessionHandle(`local-session-${"2".repeat(32)}`),
  access: "viewer",
};

function missingRecovery(): GoalRecovery {
  return {
    state: "missing",
    objective: null,
    repository: null,
    runId: null,
    eventCursor: null,
    checkpointTask: null,
    evidenceReferences: [],
    collaboration: null,
    stopConditions: [],
    reason: "No persisted goal exists",
    explicitConfirmationRequired: false,
    automaticExecutionStarted: false,
  };
}

function repairRecovery(): GoalRecovery {
  return {
    state: "resumable",
    objective: "Repair shared completion",
    repository: {
      canonicalPath: "/tmp/task-018",
      repositoryId: localBinding.repositoryId,
    },
    runId: "0123456789abcdef0123456789abcdef",
    eventCursor: 8,
    checkpointTask: localBinding.taskPath,
    evidenceReferences: [],
    collaboration: {
      state: "collaborationRepairRequired",
      intent: {
        workspaceId: collaboratorSession.workspaceId,
        access: "collaborator",
        actor: collaboratorSession.actor,
        taskId: "BR-018",
        remoteTaskPath: "tasks/BR-018.md",
        claimedTaskVersion: 7,
        sourceTaskSha256: localBinding.taskSha256,
        localTaskPath: localBinding.taskPath,
        localTaskSha256: localBinding.taskSha256,
        repositoryId: localBinding.repositoryId,
        runId: "0123456789abcdef0123456789abcdef",
        createdAtUnixSeconds: 1_784_800_000,
        evidenceId: `evidence-${"3".repeat(32)}`,
        evidencePath: "evidence/BR-018.md",
        handoffId: `handoff-${"4".repeat(32)}`,
        handoffPath: "logs/BR-018-handoff.md",
        artifacts: [],
      },
      currentTaskVersion: 7,
      missingEffects: ["taskUpdate", "handoffWrite", "statusWrite"],
    },
    stopConditions: ["collaboration repair required"],
    reason: "Local completion is authoritative; remote effects are missing",
    explicitConfirmationRequired: true,
    automaticExecutionStarted: false,
  };
}

function joined(access: "viewer" | "collaborator"): Ha2haJoinResult {
  return {
    workspaceId: collaboratorSession.workspaceId,
    actor: collaboratorSession.actor,
    access,
    task: {
      taskId: "BR-018",
      taskPath: "tasks/BR-018.md",
      baseVersion: 7,
    },
    local: localBinding,
    reconciled: true,
    executable: access === "collaborator",
    inspectionOnly: access !== "collaborator",
    repair: null,
  };
}

function sharedPreview(
  session: SanitizedSessionMetadata,
  executable: boolean,
): SharedBoundedTaskPreview {
  return {
    bounded: {
      decision: "execute-task",
      confidence: "high",
      nextAction: "Execute ready Task 018",
      blockingGates: [],
      selectedTask: localBinding.taskPath,
      executable,
      goal: "Add the collaboration UI.",
      nonGoals: ["Do not run hosted acceptance."],
      sourceUnderTest: "repo-local path",
      expectedEffects: [
        "Codex may edit files inside the selected repository",
        "Repository verification will be rerun after exit",
      ],
      liveHostWarning: "One confirmed local task may execute.",
      prompt: "native-owned prompt that the collaboration UI must not render",
      previewToken: executable ? "shared-preview-safe" : "",
      loopState: {
        state: executable ? "awaitingConfirmation" : "externalStop",
        nextTask: executable ? localBinding.taskPath : null,
        blockingGates: executable ? [] : ["read-only access"],
        expectedEffects: [],
        explicitConfirmationRequired: executable,
        automaticExecutionStarted: false,
        reason: executable ? "Awaiting exact confirmation" : "Read-only access",
      },
    },
    binding: {
      session,
      local: localBinding,
      remote: {
        taskId: "BR-018",
        taskPath: "tasks/BR-018.md",
        baseVersion: 7,
      },
      expectedRemoteMutation: {
        taskPath: "tasks/BR-018.md",
        baseVersion: 7,
        fromState: "ready",
        toState: "claimed",
        owner: session.actor,
        updatedBy: session.actor,
      },
    },
    stopConditions: [
      "Local source changes",
      "Remote version changes",
      "Read-only access cannot execute",
    ],
    executable,
    explicitConfirmationRequired: executable,
    previewToken: executable ? "shared-preview-safe" : "",
    repair: null,
  };
}

function sharedResult(
  overrides: Partial<SharedBoundedTaskResult>,
): SharedBoundedTaskResult {
  return {
    bounded: null,
    binding: sharedPreview(collaboratorSession, true).binding,
    claim: {
      status: "claimed",
      remoteVersion: 8,
      recoveredFromReadback: false,
    },
    completion: {
      status: "synchronized",
      outcome: {
        reconciliation: "reconciled",
        evidenceHandoff: {
          status: "synchronized",
          remoteVersion: 9,
          evidenceIds: [`evidence-${"3".repeat(32)}`],
          handoffId: `handoff-${"4".repeat(32)}`,
        },
      },
    },
    codexStarted: true,
    stoppedBeforeRuntime: false,
    sharedIterationBlocked: false,
    error: null,
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function panelProps(overrides: Partial<CollaborationPanelProps> = {}): CollaborationPanelProps {
  return {
    root: "/tmp/task-018",
    projectName: "task-018",
    nativeAvailable: true,
    disabled: false,
    goalRecovery: missingRecovery(),
    onEvent: vi.fn(),
    onRepositoryResult: vi.fn(),
    onSharedResult: vi.fn(),
    onGoalRecovery: vi.fn(),
    onBusyChange: vi.fn(),
    onProjectionChange: vi.fn(),
    ...overrides,
  };
}

function openPanel() {
  fireEvent.click(screen.getByRole("button", { name: /Collaboration.*Local solo/i }));
  return screen.getByRole("dialog", { name: "Collaboration authority" });
}

async function connectAndInspect(
  session: SanitizedSessionMetadata,
  joinResult: Ha2haJoinResult,
) {
  vi.mocked(bridge.connectMdsyncSession).mockResolvedValueOnce(session);
  vi.mocked(bridge.joinHa2haWorkspace).mockResolvedValueOnce(joinResult);
  const panel = openPanel();
  fireEvent.change(within(panel).getByLabelText("Workspace handoff"), {
    target: { value: "https://sync.example.test/workspaces/workspace-safe-018" },
  });
  fireEvent.click(within(panel).getByRole("button", { name: "Connect in native memory" }));
  await within(panel).findByRole("heading", { name: accessLabelForTest(session.access) });
  fireEvent.click(within(panel).getByRole("button", { name: "Join and inspect envelope" }));
  await within(panel).findByRole("region", { name: "Inspected HA2HA envelope" });
  return panel;
}

function accessLabelForTest(access: SanitizedSessionMetadata["access"]) {
  if (access === "collaborator") return "Collaborator";
  if (access === "viewer") return "Viewer";
  return "Viewer · public";
}

function domTextAndValues() {
  const values = [...document.querySelectorAll<HTMLInputElement>("input")]
    .map((input) => input.value)
    .join("\n");
  return `${document.documentElement.outerHTML}\n${values}`;
}

describe("CollaborationPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(bridge.disconnectMdsyncSession).mockResolvedValue();
    vi.mocked(bridge.recoverGoalState).mockResolvedValue(missingRecovery());
  });

  it("keeps the unchanged solo flow explicit and starts no collaboration operation", () => {
    render(<CollaborationPanel {...panelProps()} />);

    const panel = openPanel();
    expect(within(panel).getAllByText("Local solo").length).toBeGreaterThan(0);
    expect(within(panel).getByText("Authoritative")).toBeInTheDocument();
    expect(within(panel).getAllByText(/Repository Markdown, Git, and Build Right checks/).length).toBeGreaterThan(0);
    expect(bridge.connectMdsyncSession).not.toHaveBeenCalled();
    expect(bridge.previewSharedBoundedTask).not.toHaveBeenCalled();
    expect(bridge.executeSharedBoundedTask).not.toHaveBeenCalled();
  });

  it("moves focus into the panel and returns it to the trigger on Escape", async () => {
    render(<CollaborationPanel {...panelProps()} />);
    const trigger = screen.getByRole("button", { name: /Collaboration.*Local solo/i });
    fireEvent.click(trigger);
    const panel = screen.getByRole("dialog", { name: "Collaboration authority" });

    await waitFor(() => expect(panel).toHaveFocus());
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Collaboration authority" })).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("contains forward and reverse keyboard focus inside the modal surface", async () => {
    render(<CollaborationPanel {...panelProps()} />);
    fireEvent.click(screen.getByRole("button", { name: /Collaboration.*Local solo/i }));
    const panel = screen.getByRole("dialog", { name: "Collaboration authority" });
    const connect = within(panel).getByRole("button", { name: "Connect in native memory" });
    const close = within(panel).getByRole("button", { name: "Close collaboration panel" });

    await waitFor(() => expect(panel).toHaveFocus());
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(connect).toHaveFocus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(close).toHaveFocus();
  });

  it("clears the handoff and actor synchronously and leaks no opaque alias or marker family into the DOM", async () => {
    const pending = deferred<SanitizedSessionMetadata>();
    const opaqueAlias = "opaque-alias-018-7f3c91d2";
    const handoff = `https://sync.example.test/workspaces/workspace-safe-018?edit=${opaqueAlias}&k=second-opaque-value`;
    vi.mocked(bridge.connectMdsyncSession).mockReturnValueOnce(pending.promise);
    render(<CollaborationPanel {...panelProps()} />);

    const panel = openPanel();
    const handoffInput = within(panel).getByLabelText("Workspace handoff");
    const actorInput = within(panel).getByLabelText("Actor handle");
    fireEvent.change(handoffInput, { target: { value: handoff } });
    fireEvent.change(actorInput, { target: { value: "task-018-actor" } });
    fireEvent.click(within(panel).getByRole("button", { name: "Connect in native memory" }));

    expect(handoffInput).toHaveValue("");
    expect(actorInput).toHaveValue("");
    expect(bridge.connectMdsyncSession).toHaveBeenCalledWith(
      "/tmp/task-018",
      handoff,
      "task-018-actor",
    );
    for (const forbidden of [opaqueAlias, "?edit=", "&k=", "second-opaque-value", "Bearer opaque", "Authorization: opaque"]) {
      expect(domTextAndValues()).not.toContain(forbidden);
    }

    pending.resolve({ ...collaboratorSession, actor: "task-018-actor" });
    await within(panel).findByRole("heading", { name: "Collaborator" });
    for (const forbidden of [opaqueAlias, "?edit=", "&k=", "second-opaque-value"]) {
      expect(domTextAndValues()).not.toContain(forbidden);
    }
  });

  it("keeps Viewer inspection useful while denying publish and shared execution before mutation", async () => {
    vi.mocked(bridge.previewSharedBoundedTask).mockResolvedValueOnce(
      sharedPreview(viewerSession, false),
    );
    render(<CollaborationPanel {...panelProps()} />);

    const panel = await connectAndInspect(viewerSession, joined("viewer"));
    expect(within(panel).getByRole("button", { name: "Preview publish" })).toBeDisabled();
    fireEvent.click(within(panel).getByRole("button", { name: "Preview shared execution" }));

    const preview = await within(panel).findByRole("region", {
      name: "Shared execution confirmation",
    });
    expect(within(preview).getByRole("heading", { name: "Inspection only" })).toBeInTheDocument();
    expect(within(preview).getByText(/Read-only denial occurred before remote mutation and before Codex/)).toBeInTheDocument();
    expect(within(preview).queryByRole("button", { name: /execute one shared task/i })).not.toBeInTheDocument();
    expect(bridge.executeSharedBoundedTask).not.toHaveBeenCalled();
  });

  it("publishes only after a typed preview, never renders envelope bodies, and destroys the session on disconnect", async () => {
    vi.mocked(bridge.connectMdsyncSession).mockResolvedValueOnce(collaboratorSession);
    vi.mocked(bridge.previewHa2haPublish).mockResolvedValueOnce({
      workspaceId: collaboratorSession.workspaceId,
      taskPath: "tasks/BR-018.md",
      local: localBinding,
      files: [{
        path: "tasks/BR-018.md",
        content: "private-envelope-body-that-must-not-render",
        contentType: "text/markdown; charset=utf-8",
      }],
      expectedEffects: [
        "Create one complete HA2HA v1 workspace projection",
        "Perform no claim and start no provider runtime",
      ],
      explicitConfirmationRequired: true,
      previewToken: "publish-preview-opaque-control",
    });
    vi.mocked(bridge.applyHa2haPublish).mockResolvedValueOnce({
      workspaceId: collaboratorSession.workspaceId,
      taskPath: "tasks/BR-018.md",
      complete: true,
      writes: [{
        path: "tasks/BR-018.md",
        version: 1,
        recoveredFromReadback: false,
      }],
      failure: null,
      repair: null,
    });
    render(<CollaborationPanel {...panelProps()} />);

    const panel = openPanel();
    fireEvent.change(within(panel).getByLabelText("Workspace handoff"), {
      target: { value: "https://sync.example.test/workspaces/workspace-safe-018" },
    });
    fireEvent.click(within(panel).getByRole("button", { name: "Connect in native memory" }));
    await within(panel).findByRole("heading", { name: "Collaborator" });
    fireEvent.click(within(panel).getByRole("button", { name: "Preview publish" }));

    const preview = await within(panel).findByRole("region", {
      name: "HA2HA publish confirmation",
    });
    expect(within(preview).getByText("Create one complete HA2HA v1 workspace projection")).toBeInTheDocument();
    expect(within(preview).queryByText("private-envelope-body-that-must-not-render")).not.toBeInTheDocument();
    expect(domTextAndValues()).not.toContain("publish-preview-opaque-control");
    fireEvent.click(within(preview).getByRole("button", {
      name: "Publish this one envelope",
    }));

    await waitFor(() => expect(bridge.applyHa2haPublish).toHaveBeenCalledWith(
      "/tmp/task-018",
      collaboratorSession.sessionId,
      "publish-preview-opaque-control",
      true,
    ));
    expect(await within(panel).findByText(/Published one envelope in 1 bounded writes/)).toBeInTheDocument();
    fireEvent.click(within(panel).getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(bridge.disconnectMdsyncSession).toHaveBeenCalledWith(
      "/tmp/task-018",
      collaboratorSession.sessionId,
    ));
    expect(within(panel).getByText(/Disconnected from shared coordination/)).toBeInTheDocument();
    expect(within(panel).queryByText(collaboratorSession.workspaceId)).not.toBeInTheDocument();
    expect(bridge.executeSharedBoundedTask).not.toHaveBeenCalled();
  });

  it("shows a conflict stop before execution with no automatic retry", async () => {
    vi.mocked(bridge.previewSharedBoundedTask).mockResolvedValueOnce(
      sharedPreview(collaboratorSession, true),
    );
    vi.mocked(bridge.executeSharedBoundedTask).mockResolvedValueOnce(
      sharedResult({
        claim: {
          status: "stopped",
          failureClass: "versionConflict",
          latestRemoteVersion: 8,
          conflictCount: 1,
          repair: {
            code: "refresh-conflict",
            message: "The remote task changed at the confirmed version boundary",
            nextAction: "Refresh the shared preview and explicitly confirm the exact latest version",
          },
        },
        completion: { status: "notReached" },
        codexStarted: false,
        stoppedBeforeRuntime: true,
      }),
    );
    render(<CollaborationPanel {...panelProps()} />);

    const panel = await connectAndInspect(collaboratorSession, joined("collaborator"));
    fireEvent.click(within(panel).getByRole("button", { name: "Preview shared execution" }));
    const preview = await within(panel).findByRole("region", {
      name: "Shared execution confirmation",
    });
    expect(within(preview).queryByText(/native-owned prompt/)).not.toBeInTheDocument();
    fireEvent.click(within(preview).getByRole("button", {
      name: "Confirm and execute one shared task",
    }));

    const stop = await within(panel).findByRole("region", {
      name: "Conflict collaboration stop",
    });
    expect(within(stop).getByText(/Conflict detected before execution. Codex did not start/)).toBeInTheDocument();
    expect(bridge.executeSharedBoundedTask).toHaveBeenCalledTimes(1);
    expect(bridge.previewSharedBoundedTask).toHaveBeenCalledTimes(1);
    expect(bridge.repairCollaborationCompletion).not.toHaveBeenCalled();
  });

  it("maps a source mismatch to a distinct stale stop without rendering native error detail", async () => {
    const opaqueErrorAlias = "opaque-error-alias-018";
    vi.mocked(bridge.connectMdsyncSession).mockResolvedValueOnce(collaboratorSession);
    vi.mocked(bridge.joinHa2haWorkspace).mockRejectedValueOnce({
      surface: "envelope",
      error: {
        class: "sourceMismatch",
        code: "source_mismatch",
        message: `Unsafe detail ${opaqueErrorAlias}`,
      },
    });
    render(<CollaborationPanel {...panelProps()} />);

    const panel = openPanel();
    fireEvent.change(within(panel).getByLabelText("Workspace handoff"), {
      target: { value: "https://sync.example.test/workspaces/workspace-safe-018" },
    });
    fireEvent.click(within(panel).getByRole("button", { name: "Connect in native memory" }));
    await within(panel).findByRole("heading", { name: "Collaborator" });
    fireEvent.click(within(panel).getByRole("button", { name: "Join and inspect envelope" }));

    const stale = await within(panel).findByRole("region", {
      name: "Stale collaboration stop",
    });
    expect(within(stale).getByText(/Local or remote task truth changed/)).toBeInTheDocument();
    expect(panel.querySelector(".collaboration-status.state-stale")).toBeInTheDocument();
    expect(domTextAndValues()).not.toContain(opaqueErrorAlias);
    expect(bridge.previewSharedBoundedTask).not.toHaveBeenCalled();
    expect(bridge.executeSharedBoundedTask).not.toHaveBeenCalled();
  });

  it("previews and explicitly repairs only missing post-commit effects without starting Codex", async () => {
    vi.mocked(bridge.previewSharedBoundedTask).mockResolvedValueOnce(
      sharedPreview(collaboratorSession, true),
    );
    vi.mocked(bridge.executeSharedBoundedTask).mockResolvedValueOnce(
      sharedResult({
        completion: {
          status: "collaborationRepairRequired",
          outcome: {
            reconciliation: "repairRequired",
            evidenceHandoff: {
              status: "partial",
              remoteVersion: 8,
              missingEffects: ["taskUpdate", "handoffWrite", "statusWrite"],
              repair: {
                code: "retry-sync",
                message: "Verified local evidence still requires remote synchronization",
                nextAction: "Retry the bounded evidence synchronization",
              },
            },
          },
        },
        codexStarted: true,
        sharedIterationBlocked: true,
      }),
    );
    vi.mocked(bridge.repairCollaborationCompletion).mockResolvedValueOnce({
      completion: {
        status: "synchronized",
        outcome: {
          reconciliation: "reconciled",
          evidenceHandoff: {
            status: "synchronized",
            remoteVersion: 9,
            evidenceIds: [`evidence-${"3".repeat(32)}`],
            handoffId: `handoff-${"4".repeat(32)}`,
          },
        },
      },
      reconciledEffects: ["taskUpdate", "handoffWrite", "statusWrite"],
      explicitActionConsumed: true,
      codexStarted: false,
      sharedIterationBlocked: false,
    });
    render(<CollaborationPanel {...panelProps()} />);

    const panel = await connectAndInspect(collaboratorSession, joined("collaborator"));
    fireEvent.click(within(panel).getByRole("button", { name: "Preview shared execution" }));
    const preview = await within(panel).findByRole("region", {
      name: "Shared execution confirmation",
    });
    fireEvent.click(within(preview).getByRole("button", {
      name: "Confirm and execute one shared task",
    }));

    const repair = await within(panel).findByRole("region", {
      name: "Collaboration completion repair",
    });
    expect(within(repair).getByText("Local work may already be complete.")).toBeInTheDocument();
    expect(within(repair).getByText(/Repair applies only those effects, requires an explicit action, and never reruns Codex/)).toBeInTheDocument();
    expect(within(repair).getByText("Remote task link and state")).toBeInTheDocument();
    expect(within(repair).getByText("Handoff record")).toBeInTheDocument();
    expect(within(repair).getByText("Workspace status")).toBeInTheDocument();
    fireEvent.click(within(repair).getByRole("button", {
      name: "Apply only missing remote effects",
    }));

    await waitFor(() => expect(bridge.repairCollaborationCompletion).toHaveBeenCalledWith(
      "/tmp/task-018",
      collaboratorSession.sessionId,
      true,
    ));
    expect(await within(panel).findByText(/Missing remote effects were reconciled. Codex was not started/)).toBeInTheDocument();
  });

  it("reconstructs restart repair debt without a session and keeps repair disabled until exact reconnection", () => {
    render(<CollaborationPanel {...panelProps({ goalRecovery: repairRecovery() })} />);

    fireEvent.click(screen.getByRole("button", { name: /Shared repair · disconnected/i }));
    const panel = screen.getByRole("dialog", { name: "Collaboration authority" });
    expect(within(panel).getAllByText("Disconnected").length).toBeGreaterThan(0);
    expect(within(panel).getAllByText("Repair required").length).toBeGreaterThan(0);
    expect(within(panel).getByText(/Disconnected after restart/)).toBeInTheDocument();
    expect(within(panel).getByRole("button", {
      name: "Apply only missing remote effects",
    })).toBeDisabled();
    expect(within(panel).getByRole("button", {
      name: "Connect in native memory",
    })).toBeInTheDocument();
    expect(bridge.repairCollaborationCompletion).not.toHaveBeenCalled();
    expect(bridge.executeSharedBoundedTask).not.toHaveBeenCalled();
  });

  it("projects sync-pending restart state separately from repair-required and reconciled", () => {
    const recovery = repairRecovery();
    recovery.collaboration!.state = "syncPending";
    render(<CollaborationPanel {...panelProps({ goalRecovery: recovery })} />);

    fireEvent.click(screen.getByRole("button", { name: /Shared repair · disconnected/i }));
    const panel = screen.getByRole("dialog", { name: "Collaboration authority" });
    expect(within(panel).getAllByText("Sync pending").length).toBeGreaterThan(0);
    expect(panel).toHaveClass("state-syncPending");
    expect(panel).not.toHaveClass("state-repairRequired");
    expect(panel).not.toHaveClass("state-reconciled");
  });

  it("clears UI session state when the project root changes", async () => {
    vi.mocked(bridge.connectMdsyncSession).mockResolvedValueOnce(collaboratorSession);
    const props = panelProps();
    const view = render(<CollaborationPanel {...props} />);
    const panel = openPanel();
    fireEvent.change(within(panel).getByLabelText("Workspace handoff"), {
      target: { value: "https://sync.example.test/workspaces/workspace-safe-018" },
    });
    fireEvent.click(within(panel).getByRole("button", { name: "Connect in native memory" }));
    await within(panel).findByRole("heading", { name: "Collaborator" });

    view.rerender(<CollaborationPanel {...props} root="/tmp/task-019-other-project" projectName="other" />);
    await waitFor(() => expect(screen.getByRole("button", {
      name: /Collaboration.*Local solo/i,
    })).toBeInTheDocument());
    expect(screen.queryByText(collaboratorSession.workspaceId)).not.toBeInTheDocument();
    expect(bridge.disconnectMdsyncSession).not.toHaveBeenCalled();
  });
});
