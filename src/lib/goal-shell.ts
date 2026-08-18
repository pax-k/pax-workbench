import type {
  BoundedTaskPreview,
  BoundedTaskResult,
  GoalRecovery,
  ProjectSnapshot,
  WorkflowCheckpoint,
} from "../types";
import type { ProductWorkflowProjection } from "./product-workflow";

export type GoalShellState =
  | "empty"
  | "ready"
  | "resumable"
  | "blocked"
  | "reviewRequired"
  | "complete"
  | "missingPath"
  | "staleRepository"
  | "sharedRepair";

export interface GoalShellProjection {
  state: GoalShellState;
  title: string;
  statusLabel: string;
  guidance: string;
  primaryActionLabel: string;
  sharedContextLabel: string;
  selectedTaskPath: string | null;
  selectedTaskStatus: string | null;
  checkpoints: WorkflowCheckpoint[];
  automaticExecutionStarted: false;
}

export interface GoalShellInput {
  project: ProjectSnapshot;
  projectSelected: boolean;
  workflow: ProductWorkflowProjection;
  recovery: GoalRecovery | null;
  preview: BoundedTaskPreview | null;
  result: BoundedTaskResult | null;
}

const staleRecovery = new Set(["gitChanged", "staleTask", "replacedRepository"]);
const missingRecovery = new Set(["missingRepository", "movedRepository"]);

function stateOf(input: GoalShellInput): GoalShellState {
  if (!input.projectSelected) return "empty";
  if (input.recovery && missingRecovery.has(input.recovery.state)) return "missingPath";
  if (input.recovery && staleRecovery.has(input.recovery.state)) return "staleRepository";
  if (input.workflow.state === "goalComplete") return "complete";
  if (input.workflow.state === "repairRequired") return "sharedRepair";
  if (input.workflow.state === "resumable") return "resumable";
  if (input.preview || input.result) return "reviewRequired";
  if (
    input.workflow.state === "resultNeedsReview"
    || input.workflow.state === "continueAvailable"
    || input.workflow.state === "awaitingConfirmation"
  ) return "reviewRequired";
  if (input.workflow.state === "blocked") return "blocked";
  return "ready";
}

function selectedTaskPathOf(input: GoalShellInput): string | null {
  return input.preview?.selectedTask
    ?? input.result?.loopState.nextTask
    ?? input.result?.selectedTask
    ?? input.recovery?.checkpointTask
    ?? null;
}

const primaryActionLabels: Record<ProductWorkflowProjection["primaryAction"], string> = {
  openOrCreateProject: "Open or create a project",
  completeSetup: "Complete project setup",
  runPreflight: "Run readiness preflight",
  answerFounderQuestions: "Answer founder questions",
  previewPlanningChanges: "Plan the next feature",
  reviewSelectedTask: "Review resolver-selected task",
  confirmOperation: "Confirm one bounded task",
  inspectRunningOperation: "Inspect running operation",
  reviewResult: "Review verified result",
  reviewNextIteration: "Review next iteration",
  resumeVerifiedGoal: "Resume from verified checkpoint",
  repairSharedState: "Repair shared completion",
  inspectBlocker: "Inspect blocker evidence",
  finishGoal: "Review completed goal",
  inspectSharedState: "Inspect shared state",
};

const stateCopy: Record<GoalShellState, { status: string; guidance: string }> = {
  empty: { status: "No project", guidance: "Open a repository to inspect current authority." },
  ready: { status: "Ready", guidance: "Repository truth is ready for the next explicit action." },
  resumable: { status: "Resumable", guidance: "Review the last truthful checkpoint; no execution resumes automatically." },
  blocked: { status: "Blocked", guidance: "Inspect the recorded blocker before changing project state." },
  reviewRequired: { status: "Review required", guidance: "Review resolver or controller evidence before confirming another effect." },
  complete: { status: "Complete", guidance: "Repository verification reports the goal complete." },
  missingPath: { status: "Project path missing", guidance: "Relocate or reopen the repository before any action." },
  staleRepository: { status: "Repository changed", guidance: "Re-inspect current repository truth before fresh confirmation." },
  sharedRepair: { status: "Shared repair required", guidance: "Local authority remains intact while explicit shared repair is pending." },
};

export function deriveGoalShellProjection(input: GoalShellInput): GoalShellProjection {
  const state = stateOf(input);
  const selectedTaskPath = selectedTaskPathOf(input);
  const selectedTaskStatus = selectedTaskPath
    ? input.project.files.find((file) => file.path === selectedTaskPath)?.status ?? null
    : null;
  const copy = stateCopy[state];
  const taskLabel = selectedTaskPath?.split("/").at(-1)?.replace(/\.md$/u, "") ?? "Task unresolved";
  const taskState = state === "blocked" || state === "missingPath" || state === "staleRepository"
    ? "blocked"
    : selectedTaskPath
      ? "active"
      : "waiting";
  const verifyState = state === "complete"
    ? "done"
    : state === "reviewRequired" || state === "sharedRepair"
      ? "active"
      : "waiting";

  return {
    state,
    title: input.preview?.goal
      || input.recovery?.objective
      || (input.projectSelected ? `Advance ${input.project.name}` : "Open a project"),
    statusLabel: copy.status,
    guidance: copy.guidance,
    primaryActionLabel: primaryActionLabels[input.workflow.primaryAction],
    sharedContextLabel: input.workflow.shared
      ? `${input.workflow.mode === "localSolo" ? "Local solo" : input.workflow.mode === "viewerInspection" ? "Viewer" : "Collaborator"} · ${input.workflow.shared.reconciliation}`
      : "Local solo · unavailable",
    selectedTaskPath,
    selectedTaskStatus,
    automaticExecutionStarted: false,
    checkpoints: [
      { id: "project", label: "Project", detail: input.projectSelected ? "Repository inspected" : "Not opened", state: input.projectSelected ? "done" : "active" },
      { id: "goal", label: "Goal", detail: copy.status, state: state === "empty" ? "waiting" : state === "complete" ? "done" : "active" },
      { id: "task", label: taskLabel, detail: selectedTaskStatus ?? "Resolver evidence pending", state: taskState },
      { id: "verify", label: "Review", detail: copy.guidance, state: verifyState },
      { id: "next", label: "Next action", detail: primaryActionLabels[input.workflow.primaryAction], state: state === "complete" ? "done" : "ready" },
    ],
  };
}
