import type {
  ProductAction,
  ProductWorkflowProjection,
} from "./product-workflow";

export type ActionPhase =
  | "Start"
  | "Discover"
  | "Plan"
  | "Review"
  | "Build"
  | "Continue"
  | "Repair"
  | "Complete";

export type ActionClassification = "inspection" | "mutation" | "diagnostic";

export interface ProductActionPresentation {
  phase: ActionPhase;
  label: string;
  classification: ActionClassification;
  effectLabel: string;
  consequence: string;
  confirmationLabel: string;
  visuallyDominant: true;
}

export const PRODUCT_ACTION_PRESENTATIONS: Readonly<Record<
  ProductAction,
  Omit<ProductActionPresentation, "visuallyDominant">
>> = {
  openOrCreateProject: {
    phase: "Start",
    label: "Open or create a project",
    classification: "inspection",
    effectLabel: "Inspect repository",
    consequence: "Reads repository identity, Git state, skills, and authority files.",
    confirmationLabel: "No mutation",
  },
  completeSetup: {
    phase: "Discover",
    label: "Complete project setup",
    classification: "mutation",
    effectLabel: "Planning mutation",
    consequence: "Previews exact authority files before any local write.",
    confirmationLabel: "Preview and confirmation required",
  },
  runPreflight: {
    phase: "Discover",
    label: "Run readiness preflight",
    classification: "inspection",
    effectLabel: "Readiness inspection",
    consequence: "Runs the validated project-scoped preflight helper and records its typed decision.",
    confirmationLabel: "No repository mutation",
  },
  answerFounderQuestions: {
    phase: "Discover",
    label: "Answer founder questions",
    classification: "inspection",
    effectLabel: "Founder input",
    consequence: "Records only the answers needed to resolve product truth.",
    confirmationLabel: "No repository write yet",
  },
  previewPlanningChanges: {
    phase: "Plan",
    label: "Plan the next feature",
    classification: "mutation",
    effectLabel: "Planning mutation",
    consequence: "Builds an editable, allowlisted task and tracker preview.",
    confirmationLabel: "Preview and confirmation required",
  },
  reviewSelectedTask: {
    phase: "Review",
    label: "Review resolver-selected task",
    classification: "inspection",
    effectLabel: "Resolver inspection",
    consequence: "Rereads repository truth and binds one exact task and effect set.",
    confirmationLabel: "Execution remains separate",
  },
  confirmOperation: {
    phase: "Build",
    label: "Confirm and execute one task",
    classification: "mutation",
    effectLabel: "Build mutation",
    consequence: "Runs one reviewed task with the displayed repository effects.",
    confirmationLabel: "One-use confirmation",
  },
  inspectRunningOperation: {
    phase: "Build",
    label: "Inspect running operation",
    classification: "inspection",
    effectLabel: "Live evidence",
    consequence: "Shows bounded progress without authorizing another effect.",
    confirmationLabel: "No additional confirmation",
  },
  reviewResult: {
    phase: "Review",
    label: "Review verified result",
    classification: "inspection",
    effectLabel: "Repository evidence",
    consequence: "Compares controller output with refreshed repository truth.",
    confirmationLabel: "No mutation",
  },
  reviewNextIteration: {
    phase: "Continue",
    label: "Review next iteration",
    classification: "inspection",
    effectLabel: "Fresh resolver pass",
    consequence: "Selects the next exact task without starting it.",
    confirmationLabel: "Fresh execution confirmation required later",
  },
  resumeVerifiedGoal: {
    phase: "Continue",
    label: "Resume from verified checkpoint",
    classification: "inspection",
    effectLabel: "Repository reinspection",
    consequence: "Revalidates the checkpoint, task, and Git fingerprint.",
    confirmationLabel: "No automatic execution",
  },
  repairSharedState: {
    phase: "Repair",
    label: "Repair shared completion",
    classification: "mutation",
    effectLabel: "External shared mutation",
    consequence: "Applies only typed missing remote effects and never reruns Codex.",
    confirmationLabel: "Explicit repair action required",
  },
  inspectBlocker: {
    phase: "Repair",
    label: "Inspect blocker evidence",
    classification: "inspection",
    effectLabel: "Stop-state inspection",
    consequence: "Explains the typed gate without guessing or changing authority.",
    confirmationLabel: "No mutation",
  },
  finishGoal: {
    phase: "Complete",
    label: "Review completed goal",
    classification: "inspection",
    effectLabel: "Terminal repository truth",
    consequence: "Keeps the verified terminal checkpoint closed.",
    confirmationLabel: "No further execution",
  },
  inspectSharedState: {
    phase: "Review",
    label: "Inspect shared state",
    classification: "inspection",
    effectLabel: "Viewer inspection",
    consequence: "Shows sanitized collaboration state with every mutation denied.",
    confirmationLabel: "Read only",
  },
};

export function deriveProductActionPresentation(
  workflow: ProductWorkflowProjection,
): ProductActionPresentation {
  return {
    ...PRODUCT_ACTION_PRESENTATIONS[workflow.primaryAction],
    visuallyDominant: true,
  };
}
