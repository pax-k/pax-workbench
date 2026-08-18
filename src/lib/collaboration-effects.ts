import {
  applyHa2haPublish,
  connectMdsyncSession,
  disconnectMdsyncSession,
  executeSharedBoundedTask,
  joinHa2haWorkspace,
  previewHa2haPublish,
  previewSharedBoundedTask,
  recoverGoalState,
  repairCollaborationCompletion,
} from "./bridge";
import { assertSecretFreeCollaborationProjection } from "./collaboration";
import type {
  RuntimeMode,
  RuntimeStreamMessage,
} from "../types";
import type {
  SafePublishPreview,
  SafeSharedPreview,
} from "./collaboration-panel-model";

export const collaborationEffects = {
  async connect(root: string, workspaceUrl: string, actor: string) {
    const result = await connectMdsyncSession(root, workspaceUrl, actor);
    assertSecretFreeCollaborationProjection(result);
    return result;
  },

  disconnect(root: string, sessionId: string) {
    return disconnectMdsyncSession(root, sessionId);
  },

  async inspect(root: string, sessionId: string) {
    const result = await joinHa2haWorkspace(root, sessionId);
    assertSecretFreeCollaborationProjection(result);
    return result;
  },

  async previewPublish(root: string, sessionId: string): Promise<SafePublishPreview> {
    const result = await previewHa2haPublish(root, sessionId);
    const preview = {
      taskPath: result.taskPath,
      localTaskPath: result.local.taskPath,
      localTaskSha256: result.local.taskSha256,
      expectedEffects: [...result.expectedEffects],
      previewToken: result.previewToken,
    };
    assertSecretFreeCollaborationProjection({
      taskPath: preview.taskPath,
      localTaskPath: preview.localTaskPath,
      localTaskSha256: preview.localTaskSha256,
      expectedEffects: preview.expectedEffects,
    });
    return preview;
  },

  async publish(root: string, sessionId: string, previewToken: string) {
    const result = await applyHa2haPublish(root, sessionId, previewToken, true);
    assertSecretFreeCollaborationProjection({
      workspaceId: result.workspaceId,
      taskPath: result.taskPath,
      complete: result.complete,
      writes: result.writes,
    });
    return result;
  },

  async previewShared(root: string, sessionId: string): Promise<SafeSharedPreview> {
    const result = await previewSharedBoundedTask(root, sessionId);
    const preview = {
      binding: result.binding,
      selectedTask: result.bounded.selectedTask ?? result.binding.local.taskPath,
      expectedEffects: [...result.bounded.expectedEffects],
      stopConditions: [...result.stopConditions],
      executable: result.executable,
      explicitConfirmationRequired: result.explicitConfirmationRequired,
      previewToken: result.previewToken,
    };
    assertSecretFreeCollaborationProjection({
      binding: preview.binding,
      selectedTask: preview.selectedTask,
      expectedEffects: preview.expectedEffects,
      stopConditions: preview.stopConditions,
      executable: preview.executable,
    });
    return preview;
  },

  async executeShared(
    root: string,
    sessionId: string,
    preview: SafeSharedPreview,
    mode: RuntimeMode,
    onMessage: (message: RuntimeStreamMessage) => void,
  ) {
    const result = await executeSharedBoundedTask(
      root,
      sessionId,
      {
        mode,
        previewToken: preview.previewToken,
        selectedTask: preview.selectedTask,
        confirmed: true,
      },
      onMessage,
    );
    const claimProjection = result.claim.status === "stopped"
      ? {
          status: result.claim.status,
          failureClass: result.claim.failureClass,
          latestRemoteVersion: result.claim.latestRemoteVersion,
          conflictCount: result.claim.conflictCount,
        }
      : result.claim.status === "claimedRepairRequired"
        ? {
            status: result.claim.status,
            failureClass: result.claim.failureClass,
            remoteVersion: result.claim.remoteVersion,
          }
        : result.claim;
    const completionProjection = result.completion.status === "notReached"
      ? result.completion
      : result.completion.outcome.evidenceHandoff.status === "partial"
        ? {
            status: result.completion.status,
            reconciliation: result.completion.outcome.reconciliation,
            evidenceHandoff: {
              status: "partial",
              remoteVersion: result.completion.outcome.evidenceHandoff.remoteVersion,
              missingEffects: result.completion.outcome.evidenceHandoff.missingEffects,
            },
          }
        : {
            status: result.completion.status,
            reconciliation: result.completion.outcome.reconciliation,
            evidenceHandoff: result.completion.outcome.evidenceHandoff,
          };
    assertSecretFreeCollaborationProjection({
      binding: result.binding,
      claim: claimProjection,
      completion: completionProjection,
      codexStarted: result.codexStarted,
      stoppedBeforeRuntime: result.stoppedBeforeRuntime,
      sharedIterationBlocked: result.sharedIterationBlocked,
    });
    return result;
  },

  async repair(root: string, sessionId: string) {
    const result = await repairCollaborationCompletion(root, sessionId, true);
    assertSecretFreeCollaborationProjection(result);
    return result;
  },

  recover: recoverGoalState,
};
