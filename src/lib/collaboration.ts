export type CollaborationMode = "disabled" | "localOnly" | "viewer" | "sharedCollaborator";
export type CollaborationAccess = "public" | "viewer" | "collaborator";

declare const localSessionHandleBrand: unique symbol;
export type LocalSessionHandle = string & {
  readonly [localSessionHandleBrand]: "LocalSessionHandle";
};

export function createLocalSessionHandle(value: string): LocalSessionHandle {
  if (!/^local-session-[0-9a-f]{32}$/.test(value)) {
    throw new Error("Collaboration sessionId must be a locally minted handle");
  }
  return value as LocalSessionHandle;
}

export interface SanitizedSessionMetadata {
  sessionId: LocalSessionHandle;
  workspaceId: string;
  webOrigin: string;
  apiOrigin: string;
  access: CollaborationAccess;
  actor: string;
}

export interface LocalSourceBinding {
  taskPath: string;
  taskSha256: string;
  repositoryId: string;
  gitHead: string | null;
  gitIndexSha256: string;
  gitWorktreeSha256: string;
  gitDirty: boolean;
}

export interface RemoteTaskBinding {
  taskId: string;
  taskPath: string;
  baseVersion: number;
}

declare const evidenceReferenceBrand: unique symbol;
export type EvidenceReferenceId = string & {
  readonly [evidenceReferenceBrand]: "EvidenceReferenceId";
};

declare const handoffReferenceBrand: unique symbol;
export type HandoffReferenceId = string & {
  readonly [handoffReferenceBrand]: "HandoffReferenceId";
};

export type MissingCollaborationEffect =
  | "evidenceWrite"
  | "taskUpdate"
  | "handoffWrite"
  | "statusWrite";

export function createEvidenceReferenceId(value: string): EvidenceReferenceId {
  if (!/^evidence-[0-9a-f]{32}$/.test(value)) {
    throw new Error("evidenceId must be a locally minted non-capability reference");
  }
  return value as EvidenceReferenceId;
}

export function createHandoffReferenceId(value: string): HandoffReferenceId {
  if (!/^handoff-[0-9a-f]{32}$/.test(value)) {
    throw new Error("handoffId must be a locally minted non-capability reference");
  }
  return value as HandoffReferenceId;
}

export type CollaborationFailureClass =
  | "invalidInput"
  | "accessDenied"
  | "sourceMismatch"
  | "versionConflict"
  | "transportUnavailable"
  | "timeout"
  | "cancelled"
  | "protocol"
  | "repairRequired";

export type ReconciliationState =
  | "disabled"
  | "localOnly"
  | "disconnected"
  | "reconciled"
  | "claimed"
  | "syncPending"
  | "repairRequired"
  | "conflict";

export type RepairHintCode =
  | "reconnect"
  | "retry-sync"
  | "reconcile-claimed-pre-spawn"
  | "refresh-conflict"
  | "inspect-repeated-conflict";

export interface RepairHint {
  code: RepairHintCode;
  message:
    | "Collaboration state must be refreshed"
    | "Verified local evidence still requires remote synchronization"
    | "The remote task is claimed but Codex did not start"
    | "The remote task changed at the confirmed version boundary"
    | "The remote task conflicted again after a fresh confirmation";
  nextAction: string;
}

export type ClaimResult =
  | { status: "notRequired" }
  | { status: "claimed"; remoteVersion: number }
  | {
      status: "stopped";
      failureClass: CollaborationFailureClass;
      latestRemoteVersion: number | null;
      repair: RepairHint;
    };

export type EvidenceHandoffResult =
  | { status: "notRequired" }
  | {
      status: "synchronized";
      remoteVersion: number;
      evidenceIds: EvidenceReferenceId[];
      handoffId: HandoffReferenceId | null;
    }
  | {
      status: "partial";
      remoteVersion: number | null;
      missingEffects: MissingCollaborationEffect[];
      repair: RepairHint;
    };

export interface CollaborationProjection {
  mode: CollaborationMode;
  session: SanitizedSessionMetadata | null;
  local: LocalSourceBinding;
  remote: RemoteTaskBinding | null;
  reconciliation: ReconciliationState;
  repair: RepairHint | null;
}

const forbiddenMarkers = [
  "authorization",
  "bearer ",
  "token",
  "access_token",
  "refresh_token",
  "capability",
  "provider payload",
  "raw payload",
  "secret",
];

const approvedRepairMessages = new Set<RepairHint["message"]>([
  "Collaboration state must be refreshed",
  "Verified local evidence still requires remote synchronization",
  "The remote task is claimed but Codex did not start",
  "The remote task changed at the confirmed version boundary",
  "The remote task conflicted again after a fresh confirmation",
]);

export type CollaborationSurfaceState =
  | "solo"
  | "disconnected"
  | "reconciled"
  | "conflict"
  | "stale"
  | "syncPending"
  | "repairRequired";

export interface SafeCollaborationError {
  state: CollaborationSurfaceState;
  label: string;
  detail: string;
}

const collaborationErrorText: Record<
  Exclude<CollaborationSurfaceState, "solo" | "reconciled" | "syncPending">,
  Omit<SafeCollaborationError, "state">
> = {
  disconnected: {
    label: "Disconnected",
    detail: "The native collaboration session is unavailable. Reconnect to inspect remote state.",
  },
  conflict: {
    label: "Conflict",
    detail: "Remote state changed at the confirmed version boundary. Codex did not start.",
  },
  stale: {
    label: "Stale binding",
    detail: "Local or remote task truth changed. Refresh both sides before creating a new preview.",
  },
  repairRequired: {
    label: "Repair required",
    detail: "Shared continuation is stopped until the bounded remote repair is inspected and explicitly applied.",
  },
};

function collectSafeErrorSignals(value: unknown, output: string[], depth = 0): void {
  if (depth > 4 || value === null || value === undefined) return;
  if (typeof value === "string") {
    if (/^[a-zA-Z][a-zA-Z0-9_-]{0,79}$/.test(value)) output.push(value.toLowerCase());
    return;
  }
  if (Array.isArray(value)) {
    value.slice(0, 8).forEach((item) => collectSafeErrorSignals(item, output, depth + 1));
    return;
  }
  if (typeof value !== "object") return;
  Object.entries(value).slice(0, 24).forEach(([key, item]) => {
    if (["code", "class", "failureClass", "surface", "status"].includes(key)) {
      collectSafeErrorSignals(item, output, depth + 1);
    } else if (key === "error" || key === "conflict") {
      collectSafeErrorSignals(item, output, depth + 1);
    }
  });
}

export function classifyCollaborationError(value: unknown): SafeCollaborationError {
  const signals: string[] = [];
  collectSafeErrorSignals(value, signals);
  const joined = signals.join(" ");
  const state: SafeCollaborationError["state"] =
    joined.includes("versionconflict") || joined.includes("version_conflict") || joined.includes("conflict")
      ? "conflict"
      : joined.includes("sourcemismatch")
          || joined.includes("source_mismatch")
          || joined.includes("stale")
          || joined.includes("manifest")
        ? "stale"
        : joined.includes("repair")
          ? "repairRequired"
          : "disconnected";
  return { state, ...collaborationErrorText[state] };
}

export function collaborationEffectLabel(effect: MissingCollaborationEffect) {
  switch (effect) {
    case "evidenceWrite":
      return "Evidence record";
    case "taskUpdate":
      return "Remote task link and state";
    case "handoffWrite":
      return "Handoff record";
    case "statusWrite":
      return "Workspace status";
  }
}

export function compactBindingHash(value: string) {
  const normalized = value.startsWith("sha256:") ? value.slice("sha256:".length) : value;
  return normalized.length > 16 ? `${normalized.slice(0, 12)}…${normalized.slice(-4)}` : normalized;
}

export function assertSecretFreeCollaborationProjection(value: unknown) {
  validateProjectionValue(value, "projection");
}

function validateProjectionValue(value: unknown, field: string): void {
  if (value === null || typeof value === "boolean" || typeof value === "number") return;
  if (typeof value === "string") {
    const lower = value.toLowerCase();
    const marker = forbiddenMarkers.find((candidate) => lower.includes(candidate));
    const isOrigin = field === "webOrigin" || field === "apiOrigin";
    const cleanOrigin = /^(https?):\/\/[^/?#@]+\/?$/.test(value);
    const invalidSessionHandle =
      field === "sessionId" && !/^local-session-[0-9a-f]{32}$/.test(value);
    const invalidEvidenceReference =
      field === "evidenceIds" && !/^evidence-[0-9a-f]{32}$/.test(value);
    const invalidHandoffReference =
      field === "handoffId" && !/^handoff-[0-9a-f]{32}$/.test(value);
    const invalidMissingEffect =
      field === "missingEffects"
      && !["evidenceWrite", "taskUpdate", "handoffWrite", "statusWrite"].includes(value);
    const invalidRepairMessage =
      field === "message" && !approvedRepairMessages.has(value as RepairHint["message"]);
    if (
      marker
      || invalidSessionHandle
      || invalidEvidenceReference
      || invalidHandoffReference
      || invalidMissingEffect
      || invalidRepairMessage
      || value.length > 512
      || /[\u0000-\u001f\u007f]/.test(value)
      || /[?{}[\]]/.test(value)
      || (!isOrigin && lower.includes("://"))
      || (isOrigin && !cleanOrigin)
    ) {
      throw new Error(`Collaboration projection field ${field} contains forbidden content`);
    }
    return;
  }
  if (Array.isArray(value)) {
    if (value.length > 64) throw new Error(`Collaboration projection field ${field} is oversized`);
    value.forEach((item) => validateProjectionValue(item, field));
    return;
  }
  if (typeof value === "object") {
    Object.entries(value).forEach(([key, item]) => {
      const lower = key.toLowerCase();
      if (
        forbiddenMarkers.some((marker) => lower.includes(marker.replaceAll(" ", "")))
        || lower.includes("body")
        || lower.includes("url")
        || lower.includes("header")
      ) {
        throw new Error(`Collaboration projection contains forbidden field ${key}`);
      }
      validateProjectionValue(item, key);
    });
    return;
  }
  throw new Error(`Collaboration projection field ${field} has an unsupported value`);
}
