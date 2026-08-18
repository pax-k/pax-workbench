import { describe, expect, it } from "vitest";
import {
  PRODUCT_ACTION_PRESENTATIONS,
  deriveProductActionPresentation,
} from "./action-hierarchy";
import {
  deriveProductWorkflowProjection,
  type ProductProjectionInput,
} from "./product-workflow";

function input(overrides: Partial<ProductProjectionInput> = {}): ProductProjectionInput {
  return {
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
    collaboration: { mode: "localOnly", session: null, reconciliation: "localOnly" },
    ...overrides,
  };
}

describe("product action hierarchy", () => {
  it("renders one dominant action with explicit effect, consequence, and confirmation copy", () => {
    expect(
      deriveProductActionPresentation(deriveProductWorkflowProjection(input())),
    ).toEqual({
      phase: "Plan",
      label: "Plan the next feature",
      classification: "mutation",
      effectLabel: "Planning mutation",
      consequence: "Builds an editable, allowlisted task and tracker preview.",
      confirmationLabel: "Preview and confirmation required",
      visuallyDominant: true,
    });
  });

  it("keeps complete, blocked, viewer, and resumable actions inspection-only", () => {
    const cases = [
      input({ recovery: {
        state: "completed",
        objective: "done",
        repository: null,
        runId: null,
        eventCursor: 1,
        checkpointTask: null,
        evidenceReferences: [],
        collaboration: null,
        stopConditions: [],
        reason: "done",
        explicitConfirmationRequired: false,
        automaticExecutionStarted: false,
      } }),
      input({ projectNeedsSetup: false, planningReady: false }),
      input({ collaboration: { mode: "viewer", session: { access: "viewer" }, reconciliation: "repairRequired" } }),
      input({ recovery: {
        state: "resumable",
        objective: "resume",
        repository: null,
        runId: null,
        eventCursor: 1,
        checkpointTask: null,
        evidenceReferences: [],
        collaboration: null,
        stopConditions: [],
        reason: "resume",
        explicitConfirmationRequired: true,
        automaticExecutionStarted: false,
      } }),
    ];
    for (const value of cases) {
      expect(
        deriveProductActionPresentation(deriveProductWorkflowProjection(value)).classification,
      ).toBe("inspection");
    }
  });

  it("labels shared repair as a separately confirmed mutation", () => {
    expect(
      deriveProductActionPresentation(
        deriveProductWorkflowProjection(input({
          collaboration: {
            mode: "sharedCollaborator",
            session: { access: "collaborator" },
            reconciliation: "repairRequired",
          },
        })),
      ),
    ).toMatchObject({
      phase: "Repair",
      classification: "mutation",
      effectLabel: "External shared mutation",
      confirmationLabel: "Explicit repair action required",
    });
  });

  it("contains no destructive product affordance and classifies every action", () => {
    const presentations = Object.values(PRODUCT_ACTION_PRESENTATIONS);
    expect(presentations).toHaveLength(15);
    for (const presentation of presentations) {
      expect(["inspection", "mutation", "diagnostic"]).toContain(presentation.classification);
      expect(presentation.label).not.toMatch(/\b(?:delete|reset|revert|push|force|discard)\b/iu);
      expect(presentation.consequence.trim()).not.toBe("");
      expect(presentation.confirmationLabel.trim()).not.toBe("");
    }
  });
});
