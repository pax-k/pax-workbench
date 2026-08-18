import { describe, expect, it } from "vitest";
import type { GoalLoopProjection, GoalRecovery } from "../types";
import { createLocalSessionHandle, type CollaborationProjection } from "./collaboration";
import {
  PRODUCT_WORKFLOW_TRANSITIONS,
  assertSecretFreeProductContract,
  deriveProductWorkflowProjection,
  selectRepairGuidance,
  validateMutationPlan,
  validateMutationReceipt,
  type MutationPlan,
  type ProductProjectionInput,
} from "./product-workflow";

const goalLoop: GoalLoopProjection = {
  state: "awaitingConfirmation",
  nextTask: "tasks/issues/022-contracts.md",
  blockingGates: [],
  expectedEffects: ["Run one bounded task"],
  explicitConfirmationRequired: true,
  automaticExecutionStarted: false,
  reason: "Repository resolver selected one task",
};

const recovery: GoalRecovery = {
  state: "resumable",
  objective: "Complete one task",
  repository: { canonicalPath: "/tmp/project", repositoryId: "repo-123" },
  runId: "run-123",
  eventCursor: 4,
  checkpointTask: "tasks/issues/022-contracts.md",
  evidenceReferences: [],
  collaboration: null,
  stopConditions: [],
  reason: "Verified checkpoint can be reviewed",
  explicitConfirmationRequired: true,
  automaticExecutionStarted: false,
};

const localCollaboration: CollaborationProjection = {
  mode: "localOnly",
  session: null,
  local: {
    taskPath: "tasks/issues/022-contracts.md",
    taskSha256: "sha256:local",
    repositoryId: "repo-123",
    gitHead: null,
    gitIndexSha256: "sha256:index",
    gitWorktreeSha256: "sha256:worktree",
    gitDirty: true,
  },
  remote: null,
  reconciliation: "localOnly",
  repair: null,
};

function input(overrides: Partial<ProductProjectionInput> = {}): ProductProjectionInput {
  return {
    projectSelected: true,
    projectNeedsSetup: false,
    preflightRequired: false,
    founderInputRequired: false,
    planningReady: false,
    selectedTaskReady: true,
    operationRunning: false,
    resultNeedsReview: false,
    goalLoop: null,
    recovery: null,
    collaboration: localCollaboration,
    ...overrides,
  };
}

function plan(): MutationPlan {
  return {
    planId: `product-plan-${"1".repeat(32)}`,
    effectClass: "planningMutation",
    targets: [
      { path: "tasks/issues/023-create.md", operation: "create", summary: "Create one task" },
    ],
    baselines: [
      { target: "tasks/issues/023-create.md", kind: "absent", value: null },
    ],
    effects: ["Create one reviewed task file"],
    confirmation: {
      confirmationId: `product-confirmation-${"2".repeat(32)}`,
      issuedAtUnixMs: 1_000,
      expiresAtUnixMs: 61_000,
      oneUse: true,
    },
  };
}

describe("product workflow projection", () => {
  it("derives local state from existing goal and recovery contracts without automatic execution", () => {
    expect(deriveProductWorkflowProjection(input({ goalLoop }))).toEqual({
      state: "awaitingConfirmation",
      mode: "localSolo",
      projectionSource: "goalReceipt",
      primaryAction: "confirmOperation",
      allowedEffects: ["inspect", "buildMutation"],
      mutationAllowed: true,
      explicitConfirmationRequired: true,
      automaticExecutionStarted: false,
      local: { loopState: "awaitingConfirmation", recoveryState: null },
      shared: { mode: "localOnly", access: null, reconciliation: "localOnly" },
    });
    expect(deriveProductWorkflowProjection(input({ recovery, selectedTaskReady: false }))).toMatchObject({
      state: "resumable",
      primaryAction: "resumeVerifiedGoal",
      automaticExecutionStarted: false,
      local: { loopState: null, recoveryState: "resumable" },
    });
  });

  it("keeps Viewer mode inspection-only and makes repair debt dominant", () => {
    const viewer: CollaborationProjection = {
      ...localCollaboration,
      mode: "viewer",
      session: {
        sessionId: createLocalSessionHandle("local-session-11111111111111111111111111111111"),
        workspaceId: "workspace-1",
        webOrigin: "https://example.invalid",
        apiOrigin: "https://example.invalid",
        access: "viewer",
        actor: "viewer-1",
      },
      reconciliation: "repairRequired",
    };
    const projection = deriveProductWorkflowProjection(input({ collaboration: viewer, goalLoop }));
    expect(projection).toMatchObject({
      state: "repairRequired",
      mode: "viewerInspection",
      primaryAction: "inspectSharedState",
      allowedEffects: ["inspect"],
      mutationAllowed: false,
      explicitConfirmationRequired: false,
    });
  });

  it("defines closed transitions and prevents automatic continuation from goal completion", () => {
    expect(Object.keys(PRODUCT_WORKFLOW_TRANSITIONS)).toHaveLength(14);
    expect(PRODUCT_WORKFLOW_TRANSITIONS.goalComplete).toEqual([]);
    expect(PRODUCT_WORKFLOW_TRANSITIONS.awaitingConfirmation).toContain("operationRunning");
    expect(PRODUCT_WORKFLOW_TRANSITIONS.operationRunning).toContain("resultNeedsReview");
  });

  it("projects specific founder input before generic setup work", () => {
    expect(
      deriveProductWorkflowProjection(
        input({ projectNeedsSetup: true, founderInputRequired: true, selectedTaskReady: false }),
      ),
    ).toMatchObject({
      state: "preflightNeedsInput",
      primaryAction: "answerFounderQuestions",
      mutationAllowed: false,
    });
  });

  it("keeps validated preflight ahead of a ready task after setup", () => {
    expect(
      deriveProductWorkflowProjection(
        input({ preflightRequired: true, selectedTaskReady: true }),
      ),
    ).toMatchObject({
      state: "preflightRequired",
      primaryAction: "runPreflight",
      allowedEffects: ["inspect"],
      mutationAllowed: false,
    });
  });

  it("keeps a real active operation dominant over shared repair context", () => {
    const repairCollaboration: CollaborationProjection = {
      ...localCollaboration,
      mode: "sharedCollaborator",
      reconciliation: "repairRequired",
    };
    expect(
      deriveProductWorkflowProjection(
        input({ collaboration: repairCollaboration, operationRunning: true }),
      ),
    ).toMatchObject({
      state: "operationRunning",
      primaryAction: "inspectRunningOperation",
      mutationAllowed: false,
      automaticExecutionStarted: false,
      shared: { reconciliation: "repairRequired" },
    });
  });
});

describe("guided mutation and repair contracts", () => {
  it("accepts an exact expiring one-use plan and truthful full receipt", () => {
    const value = plan();
    expect(() => validateMutationPlan(value)).not.toThrow();
    expect(() =>
      validateMutationReceipt(value, {
        planId: value.planId,
        status: "applied",
        committedTargets: ["tasks/issues/023-create.md"],
        failedTargets: [],
        evidence: ["Repository readback matched the proposed content"],
        repositoryVerified: true,
        remoteAuthorityAdvanced: false,
      }),
    ).not.toThrow();
  });

  it("rejects remote effects in planning, expired confirmation, and dishonest receipts", () => {
    const sharedPlan = plan();
    sharedPlan.targets[0] = { ...sharedPlan.targets[0], operation: "publish" };
    expect(() => validateMutationPlan(sharedPlan)).toThrow("Planning mutation cannot include shared effects");

    const expired = plan();
    expired.confirmation.expiresAtUnixMs = expired.confirmation.issuedAtUnixMs;
    expect(() => validateMutationPlan(expired)).toThrow("confirmation lifetime");

    const value = plan();
    expect(() =>
      validateMutationReceipt(value, {
        planId: value.planId,
        status: "applied",
        committedTargets: [],
        failedTargets: [],
        evidence: [],
        repositoryVerified: false,
        remoteAuthorityAdvanced: false,
      }),
    ).toThrow("does not account");

    expect(() =>
      validateMutationReceipt(value, {
        planId: value.planId,
        status: "partial",
        committedTargets: ["tasks/issues/023-create.md"],
        failedTargets: [],
        evidence: [],
        repositoryVerified: false,
        remoteAuthorityAdvanced: false,
      }),
    ).toThrow("committed and failed");
  });

  it("shows Local Network settings only for matching evidence and labels other policy guidance hypothetical", () => {
    expect(
      selectRepairGuidance({
        failureClass: "runtime",
        code: "providerError",
        message: "Runtime failed",
        evidence: [{ source: "runtime", code: "providerError", summary: "Provider returned an error" }],
      }).action,
    ).toBe("inspectRuntimeDiagnostics");

    expect(
      selectRepairGuidance({
        failureClass: "networkPolicy",
        code: "connectivityUnknown",
        message: "Connection failed",
        evidence: [{ source: "system", code: "connectivityUnknown", summary: "Cause is unknown" }],
      }),
    ).toMatchObject({ action: "inspectNetworkPolicy", confidence: "hypothesis" });

    expect(
      selectRepairGuidance({
        failureClass: "networkPolicy",
        code: "permissionDenied",
        message: "Permission denied",
        evidence: [{ source: "system", code: "localNetworkDenied", summary: "OS permission result" }],
      }),
    ).toMatchObject({ action: "openLocalNetworkSettings", confidence: "evidence" });
  });

  it("rejects capabilities, authorization fields, provider payloads, and capability-bearing URLs", () => {
    for (const unsafe of [
      { authorizationHeader: "redacted" },
      { note: "provider payload follows" },
      { path: "https://host.invalid/workspace?edit=opaque" },
      { capability: "opaque" },
    ]) {
      expect(() => assertSecretFreeProductContract(unsafe)).toThrow("forbidden");
    }
  });
});
