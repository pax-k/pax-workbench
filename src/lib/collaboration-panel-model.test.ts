import { describe, expect, it } from "vitest";
import {
  deriveCollaborationPanelProjection,
  recoveryCollaborationSurface,
  sharedResultProjection,
} from "./collaboration-panel-model";

describe("collaboration panel projection", () => {
  it("keeps absent session local-only and projects recovery debt without automatic execution", () => {
    const recovery = {
      state: "resumable" as const,
      objective: "One task",
      repository: null,
      runId: null,
      eventCursor: null,
      checkpointTask: null,
      evidenceReferences: [],
      collaboration: {
        state: "collaborationRepairRequired" as const,
        intent: {
          workspaceId: "workspace-1",
          access: "collaborator" as const,
          actor: "actor-1",
          taskId: "BR-020",
          remoteTaskPath: "tasks/BR-020.md",
          claimedTaskVersion: 3,
          sourceTaskSha256: "sha256:source",
          localTaskPath: "tasks/issues/020-extract.md",
          localTaskSha256: "sha256:local",
          repositoryId: "repo-1",
          runId: "run-1",
          createdAtUnixSeconds: 1,
          evidenceId: "evidence-1",
          evidencePath: "evidence/1.json",
          handoffId: "handoff-1",
          handoffPath: "logs/1.jsonl",
          artifacts: [],
        },
        currentTaskVersion: 4,
        missingEffects: ["evidenceWrite" as const],
      },
      stopConditions: [],
      reason: "Repair required",
      explicitConfirmationRequired: true,
      automaticExecutionStarted: false as const,
    };
    expect(recoveryCollaborationSurface(recovery)).toBe("repairRequired");
    expect(
      deriveCollaborationPanelProjection({
        session: null,
        joined: null,
        sharedPreview: null,
        goalRecovery: recovery,
        surfaceState: "repairRequired",
        repairKind: "completion",
        missingEffects: ["evidenceWrite"],
        busy: null,
      }),
    ).toMatchObject({
      triggerLabel: "Shared repair · disconnected",
      localTaskPath: "tasks/issues/020-extract.md",
      remoteTaskPath: "tasks/BR-020.md",
      remoteVersion: 4,
      completionDebt: true,
      canRepair: false,
      product: {
        mode: "localOnly",
        session: null,
        reconciliation: "repairRequired",
      },
    });
  });

  it("projects Viewer access as inspection-only product input", () => {
    const projection = deriveCollaborationPanelProjection({
      session: {
        sessionId: "local-session-11111111111111111111111111111111" as never,
        workspaceId: "workspace-1",
        webOrigin: "https://example.invalid",
        apiOrigin: "https://example.invalid",
        access: "viewer",
        actor: "viewer-1",
      },
      joined: null,
      sharedPreview: null,
      goalRecovery: null,
      surfaceState: "disconnected",
      repairKind: null,
      missingEffects: [],
      busy: null,
    });
    expect(projection).toMatchObject({
      currentAccess: "viewer",
      isViewer: true,
      canRepair: false,
      product: { mode: "viewer", session: { access: "viewer" } },
    });
  });

  it("maps conflict and stale shared stops before runtime", () => {
    const base = {
      bounded: null,
      binding: {} as never,
      completion: { status: "notReached" as const },
      codexStarted: false,
      stoppedBeforeRuntime: true,
      sharedIterationBlocked: true,
      error: null,
    };
    expect(
      sharedResultProjection({
        ...base,
        claim: {
          status: "stopped",
          failureClass: "versionConflict",
          latestRemoteVersion: 3,
          conflictCount: 1,
          repair: null,
        },
      }),
    ).toMatchObject({ state: "conflict", repairKind: null });
    expect(
      sharedResultProjection({
        ...base,
        claim: {
          status: "stopped",
          failureClass: "sourceMismatch",
          latestRemoteVersion: null,
          conflictCount: 0,
          repair: null,
        },
      }),
    ).toMatchObject({ state: "stale", repairKind: null });
  });
});
