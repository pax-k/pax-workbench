import type {
  CollaborationAccess,
  CollaborationFailureClass,
  LocalSourceBinding,
  MissingCollaborationEffect,
  RemoteTaskBinding,
  RepairHint,
  SanitizedSessionMetadata,
} from "./lib/collaboration";

export type WorkflowState = "done" | "active" | "ready" | "waiting" | "blocked";

export interface ProjectFile {
  path: string;
  name: string;
  kind: "instruction" | "document" | "task" | "evidence" | "skill";
  status?: string | null;
}

export interface ProjectError {
  code: string;
  message: string;
  path?: string | null;
  committed: boolean;
}

export interface ProjectFileContent {
  path: string;
  content: string;
  version: string;
}

export interface SkillSummary {
  id: string;
  name: string;
  phase: "Discover" | "Plan" | "Build" | "Principles" | "Unknown";
  purpose: string;
  reads: string[];
  writes: string[];
  decisions: string[];
  helpers: string[];
  requiredEvidence: string[];
  stopStates: string[];
  renderer: "operating-card" | "generic-markdown";
  executable: boolean;
  source: string;
  installedPath: string;
  lockHash?: string;
}

export interface ProjectSnapshot {
  root: string;
  name: string;
  branch: string;
  dirty: boolean;
  files: ProjectFile[];
  skills: SkillSummary[];
  errors: ProjectError[];
}

export interface ProjectWriteResult {
  file: ProjectFileContent;
  project: ProjectSnapshot;
}

export interface ArtifactDraft {
  path: string;
  content: string;
  expectedVersion?: string;
}

export interface ArtifactGitBaseline {
  head: string;
  index: string;
  worktree: string;
}

export interface ArtifactPlanTarget extends ArtifactDraft {
  contentVersion: string;
  diff: string;
  effect: "create" | "update" | "alreadyCommitted";
}

export interface ArtifactPlanPreview {
  root: string;
  targets: ArtifactPlanTarget[];
  baseline: ArtifactGitBaseline;
  previewToken: string;
  expiresAtMs: number;
  explicitConfirmationRequired: true;
  effectClass: "planMutation";
  collaborationEffects: [];
}

export interface ArtifactApplyResult {
  success: boolean;
  committedPaths: string[];
  alreadyCommittedPaths: string[];
  unappliedPaths: string[];
  failureCode: "artifact_partial_apply" | null;
  failureMessage: string | null;
  project: ProjectSnapshot;
  collaborationEffects: [];
}

export type SkillSetupOperation = "install" | "update";

export interface SkillHashChange {
  skillId: string;
  currentHash: string | null;
  proposedHash: string | null;
  proposedState: "resolvedOnExecution";
}

export interface SkillSetupPreview {
  operation: SkillSetupOperation;
  targetProject: string;
  source: "pax-k/build-right";
  executable: "bun";
  cliVersion: "skills@1.5.19";
  argv: string[];
  skillIds: string[];
  expectedChangedPaths: string[];
  hashChanges: SkillHashChange[];
  explicitConfirmationRequired: true;
  previewToken: string;
}

export interface SkillProvenanceState {
  skillId: string;
  installedPath: string;
  installed: boolean;
  lockHash: string | null;
}

export interface SkillSetupRepair {
  code: string;
  message: string;
  nextAction: string;
}

export interface SkillSetupResult {
  operation: SkillSetupOperation;
  outcome: "cancelledBeforeExecution" | "completed" | "failed" | "cancelled" | "timedOut" | "startFailed" | "verificationFailed" | "cleanupFailed" | "stalePreview";
  executed: boolean;
  success: boolean;
  exitStatus: number | null;
  stdout: string;
  stderr: string;
  stdoutTruncated: boolean;
  stderrTruncated: boolean;
  changedPaths: string[];
  before: SkillProvenanceState[];
  after: SkillProvenanceState[];
  repair: SkillSetupRepair | null;
  project: ProjectSnapshot;
}

export interface SkillSetupCancellation {
  cancellationRequested: boolean;
  message: string;
}

export type HelperId = "preflight-check" | "feature-planning-check" | "continue-check" | "execution-check";
export type HelperExecutionMode = "next-task" | "task-contract" | "stop-gates" | "all";

export interface HelperInvocation {
  helperId: HelperId;
  mode?: HelperExecutionMode;
  taskPath?: string;
  featureRequest?: string;
}

export interface PlanningGate {
  type: string;
  source: string;
  reason: string;
}

export interface PlanningTaskCandidate {
  id: string;
  title: string;
  status: string;
  owner: string;
  path: string;
  tracker: string;
}

export interface HelperDecision {
  decision: string;
  confidence: string;
  nextAction: string;
  evidence: string[];
  warnings: string[];
  recommendedDestination?: string;
  blockingGates?: PlanningGate[];
  founderQuestions?: string[];
  researchTriggers?: string[];
  readyTaskCandidates?: PlanningTaskCandidate[];
}

export interface HelperResult {
  helperId: HelperId;
  mode: HelperExecutionMode | null;
  taskPath: string | null;
  executable: "bun";
  argv: string[];
  outcome: "completed" | "nonzeroExit" | "malformedOutput" | "verificationFailed" | "outputOverflow" | "cancelled" | "timedOut" | "missingRuntime" | "startFailed" | "cleanupFailed" | "unsupportedPlatform";
  executed: boolean;
  success: boolean;
  exitStatus: number | null;
  stdout: string;
  stderr: string;
  stdoutTruncated: boolean;
  stderrTruncated: boolean;
  decision: HelperDecision | null;
  failure: string | null;
  project: ProjectSnapshot;
}

export interface HelperCancellation {
  cancellationRequested: boolean;
  message: string;
}

export interface BoundedTaskPreview {
  decision: string;
  confidence: string;
  nextAction: string;
  blockingGates: string[];
  selectedTask: string | null;
  executable: boolean;
  goal: string;
  nonGoals: string[];
  sourceUnderTest: string;
  expectedEffects: string[];
  liveHostWarning: string;
  prompt: string;
  previewToken: string;
  loopState: GoalLoopProjection;
}

export type GoalLoopState = "awaitingConfirmation" | "continueAvailable" | "founderStop" | "externalStop" | "conflictStop" | "failureStop" | "staleStop" | "cancelledStop" | "noReadyTaskStop" | "invalidStateStop" | "goalComplete";

export interface GoalLoopProjection {
  state: GoalLoopState;
  nextTask: string | null;
  blockingGates: string[];
  expectedEffects: string[];
  explicitConfirmationRequired: boolean;
  automaticExecutionStarted: false;
  reason: string;
}

export type BoundedTaskOutcome = "verified" | "verificationFailed" | "waitExternal" | "stopped";

export interface BoundedTaskInvocation {
  previewToken: string;
  selectedTask: string;
  mode: RuntimeMode;
  confirmed: boolean;
}

export interface BoundedTaskResult {
  outcome: BoundedTaskOutcome;
  selectedTask: string | null;
  runtime: RuntimeResult | null;
  project: ProjectSnapshot;
  taskEvidence: ProjectFileContent | null;
  resolver: HelperResult | null;
  stopGates: HelperResult | null;
  refreshFailures: Array<{ surface: string; code: string; message: string }>;
  repositoryVerified: boolean;
  reason: string;
  loopState: GoalLoopProjection;
}

export interface PostRunReviewChangedFile {
  path: string;
  status: string;
  diff: string | null;
  diffUnavailableReason: string | null;
  truncated: boolean;
}

export interface PostRunReviewEvidence {
  scopeNote: string;
  changedFiles: PostRunReviewChangedFile[];
  truncated: boolean;
}

export interface GitHandoffStatus {
  path: string;
  status: string;
}

export interface GitHandoffCandidate {
  path: string;
  status: string;
  stagedEffect: string;
}

export interface GitHandoffExclusion {
  path: string;
  status: string;
  code: string;
  reason: string;
}

export interface LocalGitHandoffPreview {
  root: string;
  repository: {
    canonicalPath: string;
    repositoryId: string;
  };
  baseline: ArtifactGitBaseline;
  currentStatus: GitHandoffStatus[];
  candidates: GitHandoffCandidate[];
  exclusions: GitHandoffExclusion[];
  selectedPaths: string[];
  proposedMessage: string;
  stagedEffects: string[];
  previewToken: string | null;
  expiresAtMs: number | null;
  explicitConfirmationRequired: true;
  preExistingIndex: boolean;
  remoteEffects: [];
}

export interface GitHandoffRepair {
  code: string;
  message: string;
  nextAction: string;
}

export interface LocalGitHandoffResult {
  success: boolean;
  outcome:
    | "completed"
    | "stageFailed"
    | "stageVerificationFailed"
    | "commitFailed"
    | "verificationFailed";
  commitCreated: boolean;
  previousHead: string;
  newHead: string | null;
  selectedPaths: string[];
  stagedPaths: string[];
  committedPaths: string[];
  message: string;
  repair: GitHandoffRepair | null;
  project: ProjectSnapshot;
  remoteEffects: [];
}

export interface Ha2haWorkspaceFile {
  path: string;
  content: string;
  contentType: string;
}

export interface Ha2haPublishPreview {
  workspaceId: string;
  taskPath: string;
  local: LocalSourceBinding;
  files: Ha2haWorkspaceFile[];
  expectedEffects: string[];
  explicitConfirmationRequired: boolean;
  previewToken: string;
}

export interface Ha2haPublishedFile {
  path: string;
  version: number;
  recoveredFromReadback: boolean;
}

export interface NativeCollaborationError {
  class: string;
  code: string;
  message: string;
  conflict?: {
    latest?: {
      path: string;
      version: number;
      workspaceId: string;
    } | null;
  } | null;
}

export interface EnvelopeRepair {
  code: string;
  message: string;
  nextAction: string;
}

export interface Ha2haPublishResult {
  workspaceId: string;
  taskPath: string;
  complete: boolean;
  writes: Ha2haPublishedFile[];
  failure: NativeCollaborationError | null;
  repair: EnvelopeRepair | null;
}

export interface Ha2haJoinResult {
  workspaceId: string;
  actor: string;
  access: CollaborationAccess;
  task: RemoteTaskBinding;
  local: LocalSourceBinding;
  reconciled: boolean;
  executable: boolean;
  inspectionOnly: boolean;
  repair: EnvelopeRepair | null;
}

export interface RemoteClaimPreview {
  taskPath: string;
  baseVersion: number;
  fromState: string;
  toState: string;
  owner: string;
  updatedBy: string;
}

export interface SharedExecutionBinding {
  session: SanitizedSessionMetadata;
  local: LocalSourceBinding;
  remote: RemoteTaskBinding;
  expectedRemoteMutation: RemoteClaimPreview;
}

export interface SharedBoundedTaskPreview {
  bounded: BoundedTaskPreview;
  binding: SharedExecutionBinding;
  stopConditions: string[];
  executable: boolean;
  explicitConfirmationRequired: boolean;
  previewToken: string;
  repair: RepairHint | null;
}

export type SharedClaimState =
  | { status: "reconciled" }
  | {
      status: "claimed";
      remoteVersion: number;
      recoveredFromReadback: boolean;
    }
  | {
      status: "stopped";
      failureClass: CollaborationFailureClass;
      latestRemoteVersion: number | null;
      conflictCount: number;
      repair: RepairHint | null;
    }
  | {
      status: "claimedRepairRequired";
      remoteVersion: number;
      failureClass: CollaborationFailureClass;
      cause:
        | "claimFinalization"
        | "cancellation"
        | "goalStorage"
        | "runtimeCapability"
        | "runtimeStart"
        | "controllerFinalization";
      repair: RepairHint;
    };

export type SharedCompletionOutcome = {
  reconciliation:
    | "disabled"
    | "localOnly"
    | "disconnected"
    | "reconciled"
    | "claimed"
    | "syncPending"
    | "repairRequired"
    | "conflict";
  evidenceHandoff:
    | { status: "notRequired" }
    | {
        status: "synchronized";
        remoteVersion: number;
        evidenceIds: string[];
        handoffId: string | null;
      }
    | {
        status: "partial";
        remoteVersion: number | null;
        missingEffects: MissingCollaborationEffect[];
        repair: RepairHint;
      };
};

export type SharedCompletionState =
  | { status: "notReached" }
  | { status: "synchronized"; outcome: SharedCompletionOutcome }
  | {
      status: "collaborationRepairRequired";
      outcome: SharedCompletionOutcome;
    };

export interface SharedBoundedTaskResult {
  bounded: BoundedTaskResult | null;
  binding: SharedExecutionBinding;
  claim: SharedClaimState;
  completion: SharedCompletionState;
  codexStarted: boolean;
  stoppedBeforeRuntime: boolean;
  sharedIterationBlocked: boolean;
  error: ProjectError | null;
}

export interface CollaborationRepairResult {
  completion: SharedCompletionState;
  reconciledEffects: MissingCollaborationEffect[];
  explicitActionConsumed: boolean;
  codexStarted: false;
  sharedIterationBlocked: boolean;
}

export type GoalRecoveryState = "missing" | "resumable" | "missingRepository" | "movedRepository" | "replacedRepository" | "gitChanged" | "staleTask" | "interrupted" | "incompatible" | "corrupt" | "oversized" | "completed";

export interface GoalEvidenceReference {
  path: string;
  sha256: string;
}

export interface RepositoryIdentity {
  canonicalPath: string;
  repositoryId: string;
}

export interface GoalCompletionArtifact {
  path: string;
  sha256: string;
}

export interface GoalRemoteCompletionIntent {
  workspaceId: string;
  access: "collaborator";
  actor: string;
  taskId: string;
  remoteTaskPath: string;
  claimedTaskVersion: number;
  sourceTaskSha256: string;
  localTaskPath: string;
  localTaskSha256: string;
  repositoryId: string;
  runId: string;
  createdAtUnixSeconds: number;
  evidenceId: string;
  evidencePath: string;
  handoffId: string;
  handoffPath: string;
  artifacts: GoalCompletionArtifact[];
}

export type GoalReconciliationState =
  | "syncPending"
  | "collaborationRepairRequired"
  | "reconciled";

export interface GoalCollaborationCursor {
  state: GoalReconciliationState;
  intent: GoalRemoteCompletionIntent;
  currentTaskVersion: number;
  missingEffects: MissingCollaborationEffect[];
}

export interface GoalRecovery {
  state: GoalRecoveryState;
  objective: string | null;
  repository: RepositoryIdentity | null;
  runId: string | null;
  eventCursor: number | null;
  checkpointTask: string | null;
  evidenceReferences: GoalEvidenceReference[];
  collaboration: GoalCollaborationCursor | null;
  stopConditions: string[];
  reason: string;
  explicitConfirmationRequired: boolean;
  automaticExecutionStarted: false;
}

export type RuntimeMode = "fixture" | "live";

export interface RuntimeInvocation {
  mode: RuntimeMode;
  prompt?: string;
  confirmed: boolean;
}

export type RuntimeOutcome = "completed" | "confirmationRequired" | "invalidPrompt" | "nonzeroExit" | "malformedOutput" | "outputOverflow" | "cancelled" | "timedOut" | "missingRuntime" | "startFailed" | "cleanupFailed" | "capabilityUnavailable" | "unsupportedPlatform" | "providerError" | "channelFailed";

export interface EncodedPayload {
  encoding: "utf8" | "hex";
  data: string;
}

export interface RuntimeNormalizedEvent {
  sequence: number;
  kind: "session" | "turn" | "message" | "command" | "fileChange" | "tool" | "reasoning" | "usage" | "error" | "stderr" | "unknown" | "malformed";
  providerType: string | null;
  summary: string;
  rawPayload: EncodedPayload;
  provenance: "fixture" | "provider";
}

export interface RuntimeCapabilities {
  eventStream: boolean;
  cancellation: boolean;
  timeout: boolean;
  rawPayload: boolean;
  fixture: boolean;
  live: boolean;
  repositoryAuthority: false;
}

export interface RuntimeProvenance {
  adapter: "runtime-port/v1";
  provider: "codex-jsonl/v1";
  mode: RuntimeMode;
  executable: string;
  runtimeVersion: string | null;
  projectRoot: string;
  argv: string[];
  simulated: boolean;
}

export interface RuntimeResult {
  runId: string;
  outcome: RuntimeOutcome;
  executed: boolean;
  success: boolean;
  exitStatus: number | null;
  events: RuntimeNormalizedEvent[];
  stdout: EncodedPayload;
  stderr: EncodedPayload;
  stdoutTruncated: boolean;
  stderrTruncated: boolean;
  failure: string | null;
  capabilities: RuntimeCapabilities;
  provenance: RuntimeProvenance;
  repositoryAuthorityAdvanced: false;
}

export interface RuntimeRunHandle {
  runId: string;
  capabilities: RuntimeCapabilities;
  provenance: RuntimeProvenance;
}

export type RuntimeStreamMessage =
  | { type: "started"; handle: RuntimeRunHandle }
  | { type: "event"; runId: string; event: RuntimeNormalizedEvent };

export interface RuntimeCancellation {
  cancellationRequested: boolean;
  message: string;
}

export interface RunEvent {
  id: string;
  time: string;
  label: string;
  detail: string;
  kind: "read" | "decision" | "command" | "edit" | "verify" | "evidence";
  simulated?: boolean;
  provenance?: "real" | "adapter" | "manual" | "simulated";
}

export interface WorkflowCheckpoint {
  id: string;
  label: string;
  detail: string;
  state: WorkflowState;
}

export interface ParsedTask {
  id: string;
  title: string;
  status: string;
  owner: string;
  requirementBasis: string;
  goal: string;
  acceptanceCriteria: Array<{ text: string; checked: boolean }>;
}
