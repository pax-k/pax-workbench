import { describe, expect, it } from "vitest";
import type { GoalRecovery, ProjectSnapshot } from "../types";
import { deriveGoalShellProjection } from "./goal-shell";
import { deriveProductWorkflowProjection } from "./product-workflow";

const project: ProjectSnapshot = {
  root: "/tmp/project",
  name: "project",
  branch: "main",
  dirty: false,
  files: [{ path: "tasks/issues/026-shell.md", name: "026 shell", kind: "task", status: "ready" }],
  skills: [],
  errors: [],
};

function recovery(state: GoalRecovery["state"]): GoalRecovery {
  return {
    state,
    objective: "Ship the founder workflow",
    repository: { canonicalPath: project.root, repositoryId: "sha256:repo" },
    runId: "run",
    eventCursor: 2,
    checkpointTask: "tasks/issues/026-shell.md",
    evidenceReferences: [],
    collaboration: null,
    stopConditions: [],
    reason: `${state} evidence`,
    explicitConfirmationRequired: state === "resumable",
    automaticExecutionStarted: false,
  };
}

function projection(goalRecovery: GoalRecovery | null, overrides = {}) {
  return deriveProductWorkflowProjection({
    projectSelected: true,
    projectNeedsSetup: false,
    preflightRequired: false,
    founderInputRequired: false,
    planningReady: false,
    selectedTaskReady: false,
    operationRunning: false,
    resultNeedsReview: false,
    goalLoop: null,
    recovery: goalRecovery,
    collaboration: { mode: "localOnly", session: null, reconciliation: "localOnly" },
    ...overrides,
  });
}

it.each([
  ["resumable", "resumable"],
  ["missingRepository", "missingPath"],
  ["movedRepository", "missingPath"],
  ["gitChanged", "staleRepository"],
  ["staleTask", "staleRepository"],
  ["completed", "complete"],
] as const)("presents %s recovery as distinct %s shell state", (recoveryState, shellState) => {
  const value = recovery(recoveryState);
  expect(deriveGoalShellProjection({
    project,
    projectSelected: true,
    workflow: projection(value),
    recovery: value,
    preview: null,
    result: null,
  })).toMatchObject({
    state: shellState,
    title: "Ship the founder workflow",
    automaticExecutionStarted: false,
  });
});

describe("goal-centered task projection", () => {
  it("uses resolver output and repository status, never an editor selection", () => {
    const shell = deriveGoalShellProjection({
      project,
      projectSelected: true,
      workflow: projection(null, { selectedTaskReady: true }),
      recovery: null,
      preview: {
        decision: "execute-task",
        confidence: "high",
        nextAction: "Execute",
        blockingGates: [],
        selectedTask: "tasks/issues/026-shell.md",
        executable: true,
        goal: "Prove goal-centered navigation",
        nonGoals: [],
        sourceUnderTest: "repo",
        expectedEffects: [],
        liveHostWarning: "warning",
        prompt: "prompt",
        previewToken: "token",
        loopState: {
          state: "awaitingConfirmation",
          nextTask: "tasks/issues/026-shell.md",
          blockingGates: [],
          expectedEffects: [],
          explicitConfirmationRequired: true,
          automaticExecutionStarted: false,
          reason: "selected",
        },
      },
      result: null,
    });
    expect(shell).toMatchObject({
      state: "reviewRequired",
      selectedTaskPath: "tasks/issues/026-shell.md",
      selectedTaskStatus: "ready",
      title: "Prove goal-centered navigation",
    });
  });

  it("composes shared repair over local projection without inventing task authority", () => {
    const workflow = deriveProductWorkflowProjection({
      projectSelected: true,
      projectNeedsSetup: false,
      preflightRequired: false,
      founderInputRequired: false,
      planningReady: false,
      selectedTaskReady: false,
      operationRunning: false,
      resultNeedsReview: false,
      goalLoop: null,
      recovery: null,
      collaboration: { mode: "sharedCollaborator", session: { access: "collaborator" }, reconciliation: "repairRequired" },
    });
    expect(deriveGoalShellProjection({
      project,
      projectSelected: true,
      workflow,
      recovery: null,
      preview: null,
      result: null,
    })).toMatchObject({
      state: "sharedRepair",
      selectedTaskPath: null,
      primaryActionLabel: "Repair shared completion",
      sharedContextLabel: "Collaborator · repairRequired",
    });
  });

  it.each([
    ["localOnly", "localSolo", "Local solo · localOnly"],
    ["disconnected", "viewerInspection", "Viewer · disconnected"],
    ["conflict", "sharedCollaborator", "Collaborator · conflict"],
    ["syncPending", "sharedCollaborator", "Collaborator · syncPending"],
    ["reconciled", "sharedCollaborator", "Collaborator · reconciled"],
  ] as const)("composes %s collaboration evidence without replacing local authority", (reconciliation, mode, label) => {
    const workflow = deriveProductWorkflowProjection({
      projectSelected: true,
      projectNeedsSetup: false,
      preflightRequired: false,
      founderInputRequired: false,
      planningReady: true,
      selectedTaskReady: false,
      operationRunning: false,
      resultNeedsReview: false,
      goalLoop: null,
      recovery: null,
      collaboration: {
        mode: mode === "localSolo" ? "localOnly" : mode === "viewerInspection" ? "viewer" : "sharedCollaborator",
        session: mode === "localSolo" ? null : { access: mode === "viewerInspection" ? "viewer" : "collaborator" },
        reconciliation,
      },
    });
    const shell = deriveGoalShellProjection({
      project,
      projectSelected: true,
      workflow,
      recovery: null,
      preview: null,
      result: null,
    });
    expect(shell.sharedContextLabel).toBe(label);
    expect(shell.selectedTaskPath).toBeNull();
  });
});
