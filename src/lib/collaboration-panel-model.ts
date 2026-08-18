import type {
  CollaborationAccess,
  CollaborationSurfaceState,
  MissingCollaborationEffect,
  SanitizedSessionMetadata,
} from "./collaboration";
import type {
  GoalRecovery,
  Ha2haJoinResult,
  SharedBoundedTaskResult,
  SharedCompletionState,
  SharedExecutionBinding,
} from "../types";
import type { ProductCollaborationInput } from "./product-workflow";

export type CollaborationBusyAction =
  | "connect"
  | "disconnect"
  | "inspect"
  | "publish-preview"
  | "publish"
  | "shared-preview"
  | "shared-execute"
  | "repair";

export type CollaborationRepairKind = "completion" | "claim" | "publish" | null;

export interface SafePublishPreview {
  taskPath: string;
  localTaskPath: string;
  localTaskSha256: string;
  expectedEffects: string[];
  previewToken: string;
}

export interface SafeSharedPreview {
  binding: SharedExecutionBinding;
  selectedTask: string;
  expectedEffects: string[];
  stopConditions: string[];
  executable: boolean;
  explicitConfirmationRequired: boolean;
  previewToken: string;
}

export const collaborationSurfaceCopy: Record<
  CollaborationSurfaceState,
  { label: string; detail: string }
> = {
  solo: {
    label: "Local solo",
    detail: "Repository Markdown, Git, and Build Right checks remain the only completion authority.",
  },
  disconnected: {
    label: "Disconnected",
    detail: "No native remote session is available. Local execution remains unchanged.",
  },
  reconciled: {
    label: "Reconciled",
    detail: "The inspected remote envelope matches the current local task binding.",
  },
  conflict: {
    label: "Conflict",
    detail: "Remote state changed at the confirmed version boundary. Shared execution stopped.",
  },
  stale: {
    label: "Stale",
    detail: "Local or remote task truth changed after the previous binding was created.",
  },
  syncPending: {
    label: "Sync pending",
    detail: "Local truth is preserved while bounded remote effects remain incomplete.",
  },
  repairRequired: {
    label: "Repair required",
    detail: "Shared continuation is stopped until missing remote effects are explicitly repaired.",
  },
};

export function collaborationAccessLabel(access: CollaborationAccess | null) {
  if (access === "collaborator") return "Collaborator";
  if (access === "viewer") return "Viewer";
  if (access === "public") return "Viewer · public";
  return "Disconnected";
}

export function recoveryCollaborationSurface(
  recovery: GoalRecovery | null,
): CollaborationSurfaceState {
  if (!recovery?.collaboration) return "solo";
  if (recovery.collaboration.state === "reconciled") return "disconnected";
  if (recovery.collaboration.state === "syncPending") return "syncPending";
  return "repairRequired";
}

export function completionMissingEffects(completion: SharedCompletionState) {
  if (completion.status === "notReached") return [];
  return completion.outcome.evidenceHandoff.status === "partial"
    ? completion.outcome.evidenceHandoff.missingEffects
    : [];
}

export function sharedResultProjection(result: SharedBoundedTaskResult): {
  state: CollaborationSurfaceState;
  repairKind: CollaborationRepairKind;
  missingEffects: MissingCollaborationEffect[];
  detail: string;
} {
  if (result.claim.status === "stopped") {
    if (result.codexStarted) {
      return {
        state: "repairRequired",
        repairKind: "claim",
        missingEffects: [],
        detail: "A shared-execution invariant failed after runtime start. Shared continuation is blocked.",
      };
    }
    if (result.claim.failureClass === "versionConflict") {
      return {
        state: "conflict",
        repairKind: null,
        missingEffects: [],
        detail: "Conflict detected before execution. Codex did not start.",
      };
    }
    if (result.claim.failureClass === "sourceMismatch") {
      return {
        state: "stale",
        repairKind: null,
        missingEffects: [],
        detail: "The local or remote binding changed before execution. Codex did not start.",
      };
    }
    return {
      state: "disconnected",
      repairKind: null,
      missingEffects: [],
      detail: "The shared pre-run gate stopped before execution. Codex did not start.",
    };
  }
  if (result.claim.status === "claimedRepairRequired") {
    return {
      state: "repairRequired",
      repairKind: "claim",
      missingEffects: [],
      detail: "The remote task may be claimed, but Codex did not start. Inspect the remote claim explicitly.",
    };
  }
  if (result.completion.status === "collaborationRepairRequired") {
    return {
      state: "repairRequired",
      repairKind: "completion",
      missingEffects: completionMissingEffects(result.completion),
      detail: "Local verification is authoritative; only missing remote effects remain.",
    };
  }
  if (result.completion.status === "synchronized") {
    return {
      state: "reconciled",
      repairKind: null,
      missingEffects: [],
      detail: "Local verification and remote evidence are reconciled.",
    };
  }
  return {
    state: result.codexStarted ? "syncPending" : "reconciled",
    repairKind: null,
    missingEffects: [],
    detail: result.codexStarted
      ? "The shared runtime ended before a terminal synchronization result was available."
      : "The shared preview completed without starting Codex.",
  };
}

export interface CollaborationPanelProjection {
  triggerLabel: string;
  localTaskPath: string;
  localTaskHash: string | null;
  remoteTaskPath: string | null;
  remoteVersion: number | null;
  currentAccess: CollaborationAccess | null;
  isViewer: boolean;
  completionDebt: boolean;
  canRepair: boolean;
  product: ProductCollaborationInput;
}

export function deriveCollaborationPanelProjection(input: {
  session: SanitizedSessionMetadata | null;
  joined: Ha2haJoinResult | null;
  sharedPreview: SafeSharedPreview | null;
  goalRecovery: GoalRecovery | null;
  surfaceState: CollaborationSurfaceState;
  repairKind: CollaborationRepairKind;
  missingEffects: MissingCollaborationEffect[];
  busy: CollaborationBusyAction | null;
}): CollaborationPanelProjection {
  const recoveryBinding = input.goalRecovery?.collaboration?.intent ?? null;
  const currentBinding = input.sharedPreview?.binding ?? null;
  const localTaskPath =
    currentBinding?.local.taskPath
    ?? input.joined?.local.taskPath
    ?? recoveryBinding?.localTaskPath
    ?? "Resolver-selected local task";
  const localTaskHash =
    currentBinding?.local.taskSha256
    ?? input.joined?.local.taskSha256
    ?? recoveryBinding?.localTaskSha256
    ?? null;
  const remoteTaskPath =
    currentBinding?.remote.taskPath
    ?? input.joined?.task.taskPath
    ?? recoveryBinding?.remoteTaskPath
    ?? null;
  const remoteVersion =
    currentBinding?.remote.baseVersion
    ?? input.joined?.task.baseVersion
    ?? input.goalRecovery?.collaboration?.currentTaskVersion
    ?? null;
  const currentAccess = input.session?.access ?? null;
  const isViewer = currentAccess === "viewer" || currentAccess === "public";
  const completionDebt =
    input.repairKind === "completion"
    || Boolean(
      input.goalRecovery?.collaboration
      && input.goalRecovery.collaboration.state !== "reconciled",
    );
  const canRepair =
    completionDebt
    && input.session?.access === "collaborator"
    && input.missingEffects.length > 0
    && !input.busy;
  const triggerLabel = input.session
    ? `Shared · ${collaborationAccessLabel(input.session.access)}`
    : input.surfaceState === "solo"
      ? "Local solo"
      : completionDebt
        ? "Shared repair · disconnected"
        : `Local solo · ${collaborationSurfaceCopy[input.surfaceState].label}`;

  return {
    triggerLabel,
    localTaskPath,
    localTaskHash,
    remoteTaskPath,
    remoteVersion,
    currentAccess,
    isViewer,
    completionDebt,
    canRepair,
    product: {
      mode: input.session
        ? isViewer
          ? "viewer"
          : "sharedCollaborator"
        : "localOnly",
      session: input.session ? { access: input.session.access } : null,
      reconciliation:
        input.surfaceState === "solo"
          ? "localOnly"
          : input.surfaceState === "stale"
            ? "disconnected"
            : input.surfaceState,
    },
  };
}
