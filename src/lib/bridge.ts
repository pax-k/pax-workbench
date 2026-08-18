import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  BoundedTaskInvocation,
  BoundedTaskPreview,
  BoundedTaskResult,
  ArtifactApplyResult,
  ArtifactDraft,
  ArtifactPlanPreview,
  CollaborationRepairResult,
  GoalRecovery,
  Ha2haJoinResult,
  Ha2haPublishPreview,
  Ha2haPublishResult,
  HelperCancellation,
  HelperInvocation,
  HelperResult,
  ProjectFileContent,
  ProjectSnapshot,
  PostRunReviewEvidence,
  LocalGitHandoffPreview,
  LocalGitHandoffResult,
  ProjectWriteResult,
  RuntimeCancellation,
  RuntimeInvocation,
  RuntimeResult,
  RuntimeStreamMessage,
  SharedBoundedTaskPreview,
  SharedBoundedTaskResult,
  SkillSetupCancellation,
  SkillSetupOperation,
  SkillSetupPreview,
  SkillSetupResult,
} from "../types";
import type { SanitizedSessionMetadata } from "./collaboration";

export function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

export function describeProjectError(error: unknown) {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    const value = error as { code: unknown; message: unknown; path?: unknown };
    const path = typeof value.path === "string" ? ` (${value.path})` : "";
    const committed = "committed" in value && value.committed === true ? " (write may already be committed)" : "";
    return `${String(value.code)}: ${String(value.message)}${path}${committed}`;
  }
  return String(error);
}

export function projectErrorCode(error: unknown) {
  return typeof error === "object" && error !== null && "code" in error
    ? String((error as { code: unknown }).code)
    : null;
}

export function projectErrorCommitted(error: unknown) {
  return typeof error === "object" && error !== null && "committed" in error
    && (error as { committed: unknown }).committed === true;
}

export async function chooseProject(): Promise<ProjectSnapshot | null> {
  if (!isTauriRuntime()) return null;
  const root = await open({ directory: true, multiple: false, title: "Open engineering project" });
  if (!root || Array.isArray(root)) return null;
  return invoke<ProjectSnapshot>("inspect_project", { root });
}

export async function readProjectFile(root: string, path: string) {
  return invoke<ProjectFileContent>("read_project_file", { root, relativePath: path });
}

export async function writeProjectFile(root: string, path: string, content: string, expectedVersion: string) {
  return invoke<ProjectWriteResult>("write_project_file", { root, relativePath: path, content, expectedVersion });
}

export async function previewArtifactPlan(root: string, targets: ArtifactDraft[]) {
  return invoke<ArtifactPlanPreview>("preview_artifact_plan", { root, targets });
}

export async function applyArtifactPlan(
  root: string,
  previewToken: string,
  confirmed: boolean,
) {
  return invoke<ArtifactApplyResult>("apply_artifact_plan", {
    root,
    previewToken,
    confirmed,
  });
}

export async function refreshProject(root: string) {
  return invoke<ProjectSnapshot>("inspect_project", { root });
}

export async function inspectPostRunReview(root: string) {
  return invoke<PostRunReviewEvidence>("inspect_post_run_review", { root });
}

export async function previewLocalGitHandoff(
  root: string,
  receiptPaths: string[],
  selectedPaths: string[],
  proposedMessage: string,
) {
  return invoke<LocalGitHandoffPreview>("preview_local_git_handoff", {
    root,
    receiptPaths,
    selectedPaths,
    proposedMessage,
  });
}

export async function applyLocalGitHandoff(
  root: string,
  previewToken: string,
  confirmed: boolean,
) {
  return invoke<LocalGitHandoffResult>("apply_local_git_handoff", {
    root,
    previewToken,
    confirmed,
  });
}

export async function previewSkillSetup(root: string, operation: SkillSetupOperation) {
  return invoke<SkillSetupPreview>("preview_skill_setup", { root, operation });
}

export async function executeSkillSetup(root: string, operation: SkillSetupOperation, confirmed: boolean, previewToken: string) {
  return invoke<SkillSetupResult>("execute_skill_setup", { root, operation, confirmed, previewToken });
}

export async function cancelSkillSetup(root: string) {
  return invoke<SkillSetupCancellation>("cancel_skill_setup", { root });
}

export async function executeHelper(root: string, invocation: HelperInvocation) {
  return invoke<HelperResult>("execute_helper", { root, invocation });
}

export async function cancelHelper(root: string) {
  return invoke<HelperCancellation>("cancel_helper", { root });
}

export async function previewBoundedTask(root: string) {
  return invoke<BoundedTaskPreview>("preview_bounded_task", { root });
}

export async function recoverGoalState(root: string) {
  return invoke<GoalRecovery>("recover_goal_state", { root });
}

export async function clearGoalState() {
  return invoke<void>("clear_goal_state");
}

export async function connectMdsyncSession(root: string, workspaceUrl: string, actor: string) {
  return invoke<SanitizedSessionMetadata>("connect_mdsync_session", {
    root,
    workspaceUrl,
    actor,
  });
}

export async function disconnectMdsyncSession(root: string, sessionId: string) {
  return invoke<void>("disconnect_mdsync_session", { root, sessionId });
}

export async function previewHa2haPublish(root: string, sessionId: string) {
  return invoke<Ha2haPublishPreview>("preview_ha2ha_publish", { root, sessionId });
}

export async function applyHa2haPublish(
  root: string,
  sessionId: string,
  previewToken: string,
  confirmed: boolean,
) {
  return invoke<Ha2haPublishResult>("apply_ha2ha_publish", {
    root,
    sessionId,
    previewToken,
    confirmed,
  });
}

export async function joinHa2haWorkspace(root: string, sessionId: string) {
  return invoke<Ha2haJoinResult>("join_ha2ha_workspace", { root, sessionId });
}

export async function previewSharedBoundedTask(root: string, sessionId: string) {
  return invoke<SharedBoundedTaskPreview>("preview_shared_bounded_task", {
    root,
    sessionId,
  });
}

export async function executeSharedBoundedTask(
  root: string,
  sessionId: string,
  invocation: BoundedTaskInvocation,
  onMessage: (message: RuntimeStreamMessage) => void,
) {
  const onEvent = new Channel<RuntimeStreamMessage>();
  onEvent.onmessage = onMessage;
  return invoke<SharedBoundedTaskResult>("execute_shared_bounded_task", {
    root,
    sessionId,
    invocation,
    onEvent,
  });
}

export async function repairCollaborationCompletion(
  root: string,
  sessionId: string,
  confirmed: boolean,
) {
  return invoke<CollaborationRepairResult>("repair_collaboration_completion", {
    root,
    sessionId,
    confirmed,
  });
}

export async function executeBoundedTask(root: string, invocation: BoundedTaskInvocation, onMessage: (message: RuntimeStreamMessage) => void) {
  const onEvent = new Channel<RuntimeStreamMessage>();
  onEvent.onmessage = onMessage;
  return invoke<BoundedTaskResult>("execute_bounded_task", { root, invocation, onEvent });
}

export async function cancelBoundedTask(runId: string) {
  return invoke<RuntimeCancellation>("cancel_bounded_task", { runId });
}

export async function executeRuntime(root: string, invocation: RuntimeInvocation, onMessage: (message: RuntimeStreamMessage) => void) {
  const onEvent = new Channel<RuntimeStreamMessage>();
  onEvent.onmessage = onMessage;
  return invoke<RuntimeResult>("execute_runtime", { root, invocation, onEvent });
}

export async function cancelRuntime(runId: string) {
  return invoke<RuntimeCancellation>("cancel_runtime", { runId });
}
