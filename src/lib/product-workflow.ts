import type {
  GoalLoopProjection,
  GoalLoopState,
  GoalRecovery,
  GoalRecoveryState,
} from "../types";
import type {
  CollaborationAccess,
  CollaborationMode,
  CollaborationProjection,
  ReconciliationState,
} from "./collaboration";

export type ProductWorkspaceState =
  | "noProject"
  | "projectNeedsSetup"
  | "preflightRequired"
  | "preflightNeedsInput"
  | "planningReady"
  | "taskReadyForReview"
  | "awaitingConfirmation"
  | "operationRunning"
  | "resultNeedsReview"
  | "continueAvailable"
  | "resumable"
  | "repairRequired"
  | "blocked"
  | "goalComplete";

export type ProductMode = "localSolo" | "viewerInspection" | "sharedCollaborator";

export type ProductAction =
  | "openOrCreateProject"
  | "completeSetup"
  | "runPreflight"
  | "answerFounderQuestions"
  | "previewPlanningChanges"
  | "reviewSelectedTask"
  | "confirmOperation"
  | "inspectRunningOperation"
  | "reviewResult"
  | "reviewNextIteration"
  | "resumeVerifiedGoal"
  | "repairSharedState"
  | "inspectBlocker"
  | "finishGoal"
  | "inspectSharedState";

export type EffectClass =
  | "inspect"
  | "planningMutation"
  | "buildMutation"
  | "gitMutation"
  | "externalShared"
  | "developerDiagnostic";

export type ProjectionAuthority = "repository" | "goalReceipt" | "application";

export interface ProductWorkflowProjection {
  state: ProductWorkspaceState;
  mode: ProductMode;
  projectionSource: ProjectionAuthority;
  primaryAction: ProductAction;
  allowedEffects: EffectClass[];
  mutationAllowed: boolean;
  explicitConfirmationRequired: boolean;
  automaticExecutionStarted: false;
  local: {
    loopState: GoalLoopState | null;
    recoveryState: GoalRecoveryState | null;
  };
  shared: {
    mode: CollaborationMode;
    access: CollaborationAccess | null;
    reconciliation: ReconciliationState;
  } | null;
}

export interface ProductProjectionInput {
  projectSelected: boolean;
  projectNeedsSetup: boolean;
  preflightRequired: boolean;
  founderInputRequired: boolean;
  planningReady: boolean;
  selectedTaskReady: boolean;
  operationRunning: boolean;
  resultNeedsReview: boolean;
  goalLoop: GoalLoopProjection | null;
  recovery: GoalRecovery | null;
  collaboration: ProductCollaborationInput | null;
}

export type ProductCollaborationInput = Pick<
  CollaborationProjection,
  "mode" | "reconciliation"
> & {
  session: Pick<NonNullable<CollaborationProjection["session"]>, "access"> | null;
};

const terminalStopStates = new Set<GoalLoopState>([
  "founderStop",
  "externalStop",
  "conflictStop",
  "failureStop",
  "staleStop",
  "cancelledStop",
  "noReadyTaskStop",
  "invalidStateStop",
]);

export const PRODUCT_WORKFLOW_TRANSITIONS: Readonly<
  Record<ProductWorkspaceState, readonly ProductWorkspaceState[]>
> = {
  noProject: ["projectNeedsSetup", "planningReady", "resumable"],
  projectNeedsSetup: ["preflightRequired", "preflightNeedsInput", "planningReady", "blocked"],
  preflightRequired: ["projectNeedsSetup", "preflightNeedsInput", "planningReady", "taskReadyForReview", "blocked"],
  preflightNeedsInput: ["projectNeedsSetup", "planningReady", "blocked"],
  planningReady: ["taskReadyForReview", "preflightNeedsInput", "blocked"],
  taskReadyForReview: ["awaitingConfirmation", "planningReady", "blocked"],
  awaitingConfirmation: ["operationRunning", "taskReadyForReview", "blocked"],
  operationRunning: ["resultNeedsReview", "repairRequired", "blocked"],
  resultNeedsReview: ["continueAvailable", "repairRequired", "blocked", "goalComplete"],
  continueAvailable: ["taskReadyForReview", "awaitingConfirmation", "goalComplete", "blocked"],
  resumable: ["taskReadyForReview", "awaitingConfirmation", "repairRequired", "blocked", "goalComplete"],
  repairRequired: ["resultNeedsReview", "continueAvailable", "blocked"],
  blocked: ["projectNeedsSetup", "planningReady", "taskReadyForReview", "resumable", "repairRequired"],
  goalComplete: [],
};

function modeOf(collaboration: ProductCollaborationInput | null): ProductMode {
  if (collaboration?.mode === "viewer") return "viewerInspection";
  if (collaboration?.mode === "sharedCollaborator") return "sharedCollaborator";
  return "localSolo";
}

function stateOf(input: ProductProjectionInput): ProductWorkspaceState {
  if (!input.projectSelected) return "noProject";
  if (input.operationRunning) return "operationRunning";
  if (input.goalLoop?.state === "goalComplete" || input.recovery?.state === "completed") return "goalComplete";
  if (
    input.recovery?.collaboration?.state === "collaborationRepairRequired"
    || input.collaboration?.reconciliation === "repairRequired"
  ) return "repairRequired";
  if (input.goalLoop?.state === "awaitingConfirmation") return "awaitingConfirmation";
  if (input.goalLoop?.state === "continueAvailable") return "continueAvailable";
  if (input.resultNeedsReview) return "resultNeedsReview";
  if (
    input.recovery?.state === "resumable"
    || input.recovery?.state === "staleTask"
    || input.recovery?.state === "interrupted"
  ) return "resumable";
  if (input.founderInputRequired) return "preflightNeedsInput";
  if (input.projectNeedsSetup) return "projectNeedsSetup";
  if (input.preflightRequired) return "preflightRequired";
  if (input.goalLoop && terminalStopStates.has(input.goalLoop.state)) return "blocked";
  if (input.selectedTaskReady) return "taskReadyForReview";
  if (input.planningReady) return "planningReady";
  return "blocked";
}

const stateAction: Record<ProductWorkspaceState, ProductAction> = {
  noProject: "openOrCreateProject",
  projectNeedsSetup: "completeSetup",
  preflightRequired: "runPreflight",
  preflightNeedsInput: "answerFounderQuestions",
  planningReady: "previewPlanningChanges",
  taskReadyForReview: "reviewSelectedTask",
  awaitingConfirmation: "confirmOperation",
  operationRunning: "inspectRunningOperation",
  resultNeedsReview: "reviewResult",
  continueAvailable: "reviewNextIteration",
  resumable: "resumeVerifiedGoal",
  repairRequired: "repairSharedState",
  blocked: "inspectBlocker",
  goalComplete: "finishGoal",
};

const allowedEffects: Record<ProductWorkspaceState, EffectClass[]> = {
  noProject: ["inspect"],
  projectNeedsSetup: ["inspect", "planningMutation"],
  preflightRequired: ["inspect"],
  preflightNeedsInput: ["inspect"],
  planningReady: ["inspect", "planningMutation"],
  taskReadyForReview: ["inspect"],
  awaitingConfirmation: ["inspect", "buildMutation", "externalShared"],
  operationRunning: ["inspect"],
  resultNeedsReview: ["inspect", "gitMutation"],
  continueAvailable: ["inspect"],
  resumable: ["inspect"],
  repairRequired: ["inspect", "externalShared"],
  blocked: ["inspect"],
  goalComplete: ["inspect", "gitMutation"],
};

export function deriveProductWorkflowProjection(
  input: ProductProjectionInput,
): ProductWorkflowProjection {
  const state = stateOf(input);
  const mode = modeOf(input.collaboration);
  const viewer = mode === "viewerInspection";
  const stateHasMutation = allowedEffects[state].some(
    (effect) =>
      effect === "planningMutation"
      || effect === "buildMutation"
      || effect === "gitMutation"
      || effect === "externalShared",
  );
  const effects = viewer
    ? allowedEffects[state].filter((effect) => effect === "inspect")
    : allowedEffects[state].filter(
        (effect) => effect !== "externalShared" || mode === "sharedCollaborator",
      );
  const mutating = effects.some(
    (effect) =>
      effect === "planningMutation"
      || effect === "buildMutation"
      || effect === "gitMutation"
      || effect === "externalShared",
  );

  return {
    state,
    mode,
    projectionSource:
      input.goalLoop || input.recovery
        ? "goalReceipt"
        : input.projectSelected
          ? "repository"
          : "application",
    primaryAction: viewer && stateHasMutation ? "inspectSharedState" : stateAction[state],
    allowedEffects: effects,
    mutationAllowed: !viewer && mutating,
    explicitConfirmationRequired: !viewer && mutating,
    automaticExecutionStarted: false,
    local: {
      loopState: input.goalLoop?.state ?? null,
      recoveryState: input.recovery?.state ?? null,
    },
    shared: input.collaboration
      ? {
          mode: input.collaboration.mode,
          access: input.collaboration.session?.access ?? null,
          reconciliation: input.collaboration.reconciliation,
        }
      : null,
  };
}

export type MutationEffectClass = Exclude<EffectClass, "inspect" | "developerDiagnostic">;

export interface EffectTarget {
  path: string;
  operation: "create" | "update" | "execute" | "stage" | "commit" | "publish" | "repair";
  summary: string;
}

export interface ExpectedBaseline {
  target: string;
  kind: "absent" | "contentVersion" | "gitFingerprint" | "remoteVersion";
  value: string | number | null;
}

export interface OneUseConfirmation {
  confirmationId: string;
  issuedAtUnixMs: number;
  expiresAtUnixMs: number;
  oneUse: true;
}

export interface MutationPlan {
  planId: string;
  effectClass: MutationEffectClass;
  targets: EffectTarget[];
  baselines: ExpectedBaseline[];
  effects: string[];
  confirmation: OneUseConfirmation;
}

export type MutationReceiptStatus = "applied" | "partial" | "failed" | "cancelled" | "stale";

export interface MutationReceipt {
  planId: string;
  status: MutationReceiptStatus;
  committedTargets: string[];
  failedTargets: string[];
  evidence: string[];
  repositoryVerified: boolean;
  remoteAuthorityAdvanced: false;
}

export type ProductFailureClass =
  | "repository"
  | "contract"
  | "helper"
  | "runtime"
  | "git"
  | "networkPolicy"
  | "collaboration"
  | "cancellation"
  | "staleState";

export interface FailureEvidence {
  source: "repository" | "contract" | "helper" | "runtime" | "git" | "system" | "collaboration";
  code: string;
  summary: string;
}

export interface ProductFailure {
  failureClass: ProductFailureClass;
  code: string;
  message: string;
  evidence: FailureEvidence[];
}

export type RepairAction =
  | "refreshRepository"
  | "inspectContract"
  | "repairHelper"
  | "reauthenticateRuntime"
  | "inspectRuntimeDiagnostics"
  | "resolveGitState"
  | "openLocalNetworkSettings"
  | "inspectNetworkPolicy"
  | "reconnectCollaboration"
  | "retryAfterCancellation"
  | "refreshStaleState";

export interface RepairGuidance {
  action: RepairAction;
  confidence: "evidence" | "hypothesis";
  message: string;
}

export function selectRepairGuidance(failure: ProductFailure): RepairGuidance {
  switch (failure.failureClass) {
    case "repository":
      return { action: "refreshRepository", confidence: "evidence", message: "Refresh repository authority and inspect the reported path or verification failure." };
    case "contract":
      return { action: "inspectContract", confidence: "evidence", message: "Inspect the rejected contract field and regenerate a valid bounded preview." };
    case "helper":
      return { action: "repairHelper", confidence: "evidence", message: "Inspect helper provenance, availability, and structured output before retrying." };
    case "runtime":
      return failure.code === "authenticationRequired"
        ? { action: "reauthenticateRuntime", confidence: "evidence", message: "Restore runtime authentication, then create a fresh confirmation." }
        : { action: "inspectRuntimeDiagnostics", confidence: "evidence", message: "Inspect the bounded runtime evidence without assuming a network-policy cause." };
    case "git":
      return { action: "resolveGitState", confidence: "evidence", message: "Refresh and resolve the reported Git baseline before creating a new mutation preview." };
    case "networkPolicy":
      return failure.evidence.some((item) => item.code === "localNetworkDenied")
        ? { action: "openLocalNetworkSettings", confidence: "evidence", message: "Local Network access was denied by the operating system. Review the app permission before retrying." }
        : { action: "inspectNetworkPolicy", confidence: "hypothesis", message: "Network policy may be involved; inspect system and runtime evidence before changing permissions." };
    case "collaboration":
      return { action: "reconnectCollaboration", confidence: "evidence", message: "Reconnect the matching collaboration session and follow the existing typed repair contract." };
    case "cancellation":
      return { action: "retryAfterCancellation", confidence: "evidence", message: "The operation was cancelled. Review partial evidence before creating a fresh preview." };
    case "staleState":
      return { action: "refreshStaleState", confidence: "evidence", message: "Refresh every bound baseline and create a new one-use confirmation." };
  }
}

export function validateMutationPlan(plan: MutationPlan): void {
  if (!/^product-plan-[0-9a-f]{32}$/.test(plan.planId)) throw new Error("Mutation plan ID is invalid");
  if (!/^product-confirmation-[0-9a-f]{32}$/.test(plan.confirmation.confirmationId)) {
    throw new Error("Mutation confirmation ID is invalid");
  }
  if (
    plan.confirmation.oneUse !== true
    || !Number.isSafeInteger(plan.confirmation.issuedAtUnixMs)
    || !Number.isSafeInteger(plan.confirmation.expiresAtUnixMs)
    || plan.confirmation.expiresAtUnixMs <= plan.confirmation.issuedAtUnixMs
    || plan.confirmation.expiresAtUnixMs - plan.confirmation.issuedAtUnixMs > 15 * 60 * 1000
  ) throw new Error("Mutation confirmation lifetime is invalid");
  if (plan.targets.length === 0 || plan.targets.length > 64) throw new Error("Mutation targets are invalid");
  if (plan.effects.length === 0 || plan.effects.length > 64) throw new Error("Mutation effect summary is invalid");
  const paths = new Set<string>();
  for (const target of plan.targets) {
    if (
      target.path.startsWith("/")
      || target.path.split("/").includes("..")
      || target.path.length === 0
      || target.path.length > 512
      || paths.has(target.path)
    ) throw new Error("Mutation target is invalid");
    paths.add(target.path);
  }
  if (plan.baselines.length !== plan.targets.length) throw new Error("Every mutation target requires one baseline");
  const baselineTargets = new Set(plan.baselines.map((baseline) => baseline.target));
  if (baselineTargets.size !== paths.size) throw new Error("Every mutation target requires one distinct baseline");
  for (const baseline of plan.baselines) {
    if (!paths.has(baseline.target)) throw new Error("Mutation baseline does not match a target");
  }
  if (
    plan.effectClass === "planningMutation"
    && plan.targets.some((target) => target.operation === "publish" || target.operation === "repair")
  ) throw new Error("Planning mutation cannot include shared effects");
  assertSecretFreeProductContract(plan);
}

export function validateMutationReceipt(plan: MutationPlan, receipt: MutationReceipt): void {
  if (receipt.planId !== plan.planId) throw new Error("Mutation receipt does not match its plan");
  const planned = new Set(plan.targets.map((target) => target.path));
  const reported = [...receipt.committedTargets, ...receipt.failedTargets];
  if (new Set(reported).size !== reported.length || reported.some((target) => !planned.has(target))) {
    throw new Error("Mutation receipt contains an unplanned or duplicate target");
  }
  const accountsForAllTargets = reported.length === planned.size;
  if (receipt.status === "applied" && (receipt.failedTargets.length > 0 || receipt.committedTargets.length !== planned.size)) {
    throw new Error("Applied receipt does not account for every target");
  }
  if (
    receipt.status === "partial"
    && (!accountsForAllTargets || receipt.committedTargets.length === 0 || receipt.failedTargets.length === 0)
  ) {
    throw new Error("Partial receipt must report committed and failed targets");
  }
  if (
    (receipt.status === "failed" || receipt.status === "stale")
    && (!accountsForAllTargets || receipt.committedTargets.length > 0)
  ) throw new Error(`${receipt.status} receipt must account for every target without claiming a commit`);
  assertSecretFreeProductContract(receipt);
}

const forbiddenContractMarkers = [
  "authorization",
  "bearer ",
  "access_token",
  "refresh_token",
  "capability",
  "provider payload",
  "secret",
];

export function assertSecretFreeProductContract(value: unknown): void {
  const visit = (item: unknown, field: string, depth: number): void => {
    if (depth > 12) throw new Error("Product contract is too deeply nested");
    if (item === null || typeof item === "boolean" || typeof item === "number") return;
    if (typeof item === "string") {
      const lower = item.toLowerCase();
      if (
        item.length > 1024
        || /[\u0000-\u001f\u007f]/.test(item)
        || lower.includes("://")
        || forbiddenContractMarkers.some((marker) => lower.includes(marker))
      ) throw new Error(`Product contract field ${field} contains forbidden content`);
      return;
    }
    if (Array.isArray(item)) {
      if (item.length > 128) throw new Error(`Product contract field ${field} is oversized`);
      item.forEach((child) => visit(child, field, depth + 1));
      return;
    }
    if (typeof item === "object") {
      for (const [key, child] of Object.entries(item)) {
        const lower = key.toLowerCase();
        if (
          lower.includes("authorization")
          || lower.includes("capability")
          || lower.includes("header")
          || lower.includes("providerpayload")
          || lower.includes("secret")
          || lower === "url"
        ) throw new Error(`Product contract contains forbidden field ${key}`);
        visit(child, key, depth + 1);
      }
      return;
    }
    throw new Error(`Product contract field ${field} has an unsupported value`);
  };
  visit(value, "contract", 0);
}
