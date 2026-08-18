mod artifact_plan;
mod collaboration;
mod command_contract;
mod git_handoff;
mod ha2ha_envelope;
mod mdsync_transport;
mod product_workflow;
mod repository_service;
mod review_receipt;
mod workflow_controller;

use artifact_plan::{apply_artifact_plan, preview_artifact_plan, ArtifactPlanStore};
use collaboration::{
    run_after_local_commit, run_before_runtime, ClaimResult, CollaborationAccess,
    CollaborationFailure, CollaborationFailureClass, CollaborationMode, CollaborationPort,
    CompletionArtifact, ControllerCollaborationPolicy, EvidenceHandoffResult, EvidenceReferenceId,
    HandoffReferenceId, LocalSourceBinding, MissingCollaborationEffect, NoopCollaborationPort,
    PostLocalCommitCollaborationContext, PostLocalCommitCollaborationOutcome,
    PreRunCollaborationContext, PreRunCollaborationOutcome, ReconciliationState,
    RemoteCompletionIntent, RepairHint, SanitizedSessionMetadata, SharedExecutionBinding,
};
use fs2::FileExt;
use git_handoff::{apply_local_git_handoff, preview_local_git_handoff, LocalGitHandoffStore};
use ha2ha_envelope::{
    join_workspace, project_post_run_reconciliation, project_publish_plan, project_task_claim,
    EnvelopeError, EnvelopeRepair, JoinResult, PostRunEffectWrite, ProjectionInput,
    PublishPlanStore, PublishPreview, RemoteWorkspaceFile, ResolverTaskStatus, TaskClaimWrite,
    WorkspaceFile,
};
use mdsync_transport::{
    MdsyncFile, MdsyncFileListing, MdsyncSessionStore, MdsyncTransportError,
    MdsyncTransportErrorClass, MdsyncWriteInput, MdsyncWriteResult, MAX_WORKSPACE_URL_BYTES,
};
use repository_service::{GitReadFailureKind, NativeGitRead};
use review_receipt::{inspect_post_run_review_with, PostRunReviewEvidence, ReviewEvidenceFailure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Channel, Manager};

static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static LOCAL_OPERATIONS: OnceLock<Arc<OperationRegistry>> = OnceLock::new();

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFile {
    path: String,
    name: String,
    kind: String,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectError {
    code: String,
    message: String,
    path: Option<String>,
    committed: bool,
}

impl ProjectError {
    pub(crate) fn new(code: &str, message: impl Into<String>, path: Option<&Path>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            path: path.map(|value| value.to_string_lossy().to_string()),
            committed: false,
        }
    }

    pub(crate) fn after_commit(mut self) -> Self {
        self.committed = true;
        self
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFileContent {
    path: String,
    content: String,
    version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWriteResult {
    file: ProjectFileContent,
    project: ProjectSnapshot,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillSummary {
    id: String,
    name: String,
    phase: String,
    purpose: String,
    reads: Vec<String>,
    writes: Vec<String>,
    decisions: Vec<String>,
    helpers: Vec<String>,
    required_evidence: Vec<String>,
    stop_states: Vec<String>,
    renderer: String,
    executable: bool,
    source: String,
    installed_path: String,
    lock_hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SkillUiContract {
    version: u8,
    id: String,
    name: String,
    lifecycle_phase: String,
    purpose: String,
    reads: Vec<String>,
    writes: Vec<String>,
    decisions: Vec<String>,
    helpers: Vec<SkillHelper>,
    required_evidence: Vec<String>,
    stop_states: Vec<String>,
    renderer: String,
    provenance: SkillProvenance,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SkillHelper {
    id: String,
    execution: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SkillProvenance {
    source: String,
    installed_path: String,
    lock_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillLockEntry {
    source: String,
    computed_hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectSnapshot {
    root: String,
    name: String,
    branch: String,
    dirty: bool,
    files: Vec<ProjectFile>,
    skills: Vec<SkillSummary>,
    errors: Vec<ProjectError>,
}

const SKILL_SETUP_SOURCE: &str = "pax-k/build-right";
const SKILL_SETUP_CLI_VERSION: &str = "skills@1.5.19";
const SKILL_SETUP_OUTPUT_LIMIT: usize = 32 * 1024;
const SKILL_SETUP_TIMEOUT: Duration = Duration::from_secs(120);
const SKILL_SETUP_POLL_INTERVAL: Duration = Duration::from_millis(20);
const SKILL_SETUP_READER_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const HELPER_TIMEOUT: Duration = Duration::from_secs(30);
const BUILD_RIGHT_SKILL_IDS: [&str; 4] = [
    "build-right-preflight",
    "build-right-feature-planning",
    "build-right-execution",
    "build-right-engineering-principles",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum SkillSetupOperation {
    Install,
    Update,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillHashChange {
    skill_id: String,
    current_hash: Option<String>,
    proposed_hash: Option<String>,
    proposed_state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillSetupPreview {
    operation: SkillSetupOperation,
    target_project: String,
    source: String,
    executable: String,
    cli_version: String,
    argv: Vec<String>,
    skill_ids: Vec<String>,
    expected_changed_paths: Vec<String>,
    hash_changes: Vec<SkillHashChange>,
    explicit_confirmation_required: bool,
    preview_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillProvenanceState {
    skill_id: String,
    installed_path: String,
    installed: bool,
    lock_hash: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillSetupRepair {
    code: String,
    message: String,
    next_action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillSetupResult {
    operation: SkillSetupOperation,
    outcome: SkillSetupOutcome,
    executed: bool,
    success: bool,
    exit_status: Option<i32>,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
    changed_paths: Vec<String>,
    before: Vec<SkillProvenanceState>,
    after: Vec<SkillProvenanceState>,
    repair: Option<SkillSetupRepair>,
    project: ProjectSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum SkillSetupOutcome {
    CancelledBeforeExecution,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    StartFailed,
    VerificationFailed,
    CleanupFailed,
    StalePreview,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillSetupCancellation {
    cancellation_requested: bool,
    message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum HelperId {
    PreflightCheck,
    FeaturePlanningCheck,
    ContinueCheck,
    ExecutionCheck,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum HelperExecutionMode {
    NextTask,
    TaskContract,
    StopGates,
    All,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HelperInvocation {
    helper_id: HelperId,
    mode: Option<HelperExecutionMode>,
    task_path: Option<String>,
    feature_request: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum HelperOutcome {
    Completed,
    NonzeroExit,
    MalformedOutput,
    VerificationFailed,
    OutputOverflow,
    Cancelled,
    TimedOut,
    MissingRuntime,
    StartFailed,
    CleanupFailed,
    UnsupportedPlatform,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HelperDecision {
    decision: String,
    confidence: String,
    next_action: String,
    evidence: Vec<String>,
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recommended_destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocking_gates: Option<Vec<PlanningGate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    founder_questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research_triggers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready_task_candidates: Option<Vec<PlanningTaskCandidate>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanningGate {
    r#type: String,
    source: String,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanningTaskCandidate {
    id: String,
    title: String,
    status: String,
    owner: String,
    path: String,
    tracker: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HelperResult {
    helper_id: HelperId,
    mode: Option<HelperExecutionMode>,
    task_path: Option<String>,
    executable: String,
    argv: Vec<String>,
    outcome: HelperOutcome,
    executed: bool,
    success: bool,
    exit_status: Option<i32>,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
    decision: Option<HelperDecision>,
    failure: Option<String>,
    project: ProjectSnapshot,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HelperCancellation {
    cancellation_requested: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedTaskPreview {
    decision: String,
    confidence: String,
    next_action: String,
    blocking_gates: Vec<String>,
    selected_task: Option<String>,
    executable: bool,
    goal: String,
    non_goals: Vec<String>,
    source_under_test: String,
    expected_effects: Vec<String>,
    live_host_warning: String,
    prompt: String,
    preview_token: String,
    loop_state: GoalLoopProjection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundedTaskInvocation {
    preview_token: String,
    selected_task: String,
    mode: RuntimeMode,
    confirmed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum BoundedTaskOutcome {
    Verified,
    VerificationFailed,
    WaitExternal,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundedTaskCancellationPhase {
    PreRunRevalidation,
    ProviderRuntime,
    PostExitRefresh,
}

impl BoundedTaskCancellationPhase {
    fn projection_reason(self, fallback: &str) -> String {
        match self {
            Self::PreRunRevalidation => {
                "Explicit user cancellation stopped pre-run helper revalidation before provider spawn"
                    .into()
            }
            Self::ProviderRuntime => fallback.into(),
            Self::PostExitRefresh => "Explicit user cancellation stopped post-exit repository refresh after the provider completed; no retry or next task was started".into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedTaskResult {
    outcome: BoundedTaskOutcome,
    selected_task: Option<String>,
    runtime: Option<RuntimeResult>,
    project: ProjectSnapshot,
    task_evidence: Option<ProjectFileContent>,
    resolver: Option<HelperResult>,
    stop_gates: Option<HelperResult>,
    refresh_failures: Vec<BoundedTaskRefreshFailure>,
    repository_verified: bool,
    reason: String,
    loop_state: GoalLoopProjection,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SharedBoundedTaskPreview {
    bounded: BoundedTaskPreview,
    binding: SharedExecutionBinding,
    stop_conditions: Vec<String>,
    executable: bool,
    explicit_confirmation_required: bool,
    preview_token: String,
    repair: Option<RepairHint>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
enum SharedClaimState {
    Reconciled,
    Claimed {
        remote_version: u64,
        recovered_from_readback: bool,
    },
    Stopped {
        failure_class: CollaborationFailureClass,
        latest_remote_version: Option<u64>,
        conflict_count: u8,
        repair: Option<RepairHint>,
    },
    ClaimedRepairRequired {
        remote_version: u64,
        failure_class: CollaborationFailureClass,
        cause: SharedClaimRepairCause,
        repair: RepairHint,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum SharedClaimRepairCause {
    ClaimFinalization,
    Cancellation,
    GoalStorage,
    RuntimeCapability,
    RuntimeStart,
    ControllerFinalization,
}

impl SharedClaimState {
    fn mark_claimed_pre_spawn_repair(
        &mut self,
        failure_class: CollaborationFailureClass,
        cause: SharedClaimRepairCause,
    ) {
        if let Self::Claimed { remote_version, .. } = self {
            *self = Self::ClaimedRepairRequired {
                remote_version: *remote_version,
                failure_class,
                cause,
                repair: RepairHint::reconcile_claimed_pre_spawn(),
            };
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SharedBoundedTaskResult {
    bounded: Option<BoundedTaskResult>,
    binding: SharedExecutionBinding,
    claim: SharedClaimState,
    completion: SharedCompletionState,
    codex_started: bool,
    stopped_before_runtime: bool,
    shared_iteration_blocked: bool,
    error: Option<ProjectError>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
enum SharedCompletionState {
    NotReached,
    Synchronized {
        outcome: PostLocalCommitCollaborationOutcome,
    },
    CollaborationRepairRequired {
        outcome: PostLocalCommitCollaborationOutcome,
    },
}

fn shared_completion_state(
    outcome: Option<PostLocalCommitCollaborationOutcome>,
) -> SharedCompletionState {
    match outcome {
        Some(outcome)
            if matches!(
                outcome.evidence_handoff,
                EvidenceHandoffResult::Synchronized { .. }
            ) =>
        {
            SharedCompletionState::Synchronized { outcome }
        }
        Some(outcome) => SharedCompletionState::CollaborationRepairRequired { outcome },
        None => SharedCompletionState::NotReached,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollaborationRepairResult {
    completion: SharedCompletionState,
    reconciled_effects: Vec<MissingCollaborationEffect>,
    explicit_action_consumed: bool,
    codex_started: bool,
    shared_iteration_blocked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedTaskRefreshFailure {
    surface: String,
    code: String,
    message: String,
}

const GOAL_STATE_VERSION: u8 = 5;
const GOAL_STATE_MAX_BYTES: usize = 64 * 1024;
const GOAL_EVIDENCE_MAX: usize = 16;
const GOAL_FINGERPRINT_MAX_FILES: usize = 20_000;
const GOAL_FINGERPRINT_MAX_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum GoalLoopState {
    AwaitingConfirmation,
    ContinueAvailable,
    FounderStop,
    ExternalStop,
    ConflictStop,
    FailureStop,
    StaleStop,
    CancelledStop,
    NoReadyTaskStop,
    InvalidStateStop,
    GoalComplete,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoalLoopProjection {
    state: GoalLoopState,
    next_task: Option<String>,
    blocking_gates: Vec<String>,
    expected_effects: Vec<String>,
    explicit_confirmation_required: bool,
    automatic_execution_started: bool,
    reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryIdentity {
    canonical_path: String,
    repository_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GitFingerprint {
    head: String,
    index: String,
    worktree: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedRunCursor {
    run_id: String,
    event_cursor: u64,
    nonterminal: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GoalEvidenceReference {
    path: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VerifiedGoalCheckpoint {
    task_path: String,
    task_sha256: String,
    git: GitFingerprint,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum PersistedReconciliationState {
    SyncPending,
    CollaborationRepairRequired,
    Reconciled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedCollaborationCursor {
    state: PersistedReconciliationState,
    intent: RemoteCompletionIntent,
    current_task_version: u64,
    missing_effects: Vec<MissingCollaborationEffect>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedGoalRecord {
    version: u8,
    revision: u64,
    objective: String,
    repository: RepositoryIdentity,
    stop_conditions: Vec<String>,
    current_run: PersistedRunCursor,
    last_checkpoint: Option<VerifiedGoalCheckpoint>,
    evidence_references: Vec<GoalEvidenceReference>,
    collaboration: Option<PersistedCollaborationCursor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum GoalRecoveryState {
    Missing,
    Resumable,
    MissingRepository,
    MovedRepository,
    ReplacedRepository,
    GitChanged,
    StaleTask,
    Interrupted,
    Incompatible,
    Corrupt,
    Oversized,
    Completed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoalRecovery {
    state: GoalRecoveryState,
    objective: Option<String>,
    repository: Option<RepositoryIdentity>,
    run_id: Option<String>,
    event_cursor: Option<u64>,
    checkpoint_task: Option<String>,
    evidence_references: Vec<GoalEvidenceReference>,
    collaboration: Option<PersistedCollaborationCursor>,
    stop_conditions: Vec<String>,
    reason: String,
    explicit_confirmation_required: bool,
    automatic_execution_started: bool,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GoalStorageTestPhase {
    DirectoryOpened,
    LockOpened,
    StateOpened,
    TemporarySynced,
}

#[derive(Clone)]
struct GoalStateStorage {
    directory: PathBuf,
    #[cfg(test)]
    test_hook: Option<Arc<dyn Fn(GoalStorageTestPhase) + Send + Sync>>,
}

impl GoalStateStorage {
    fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            #[cfg(test)]
            test_hook: None,
        }
    }

    fn target(&self) -> PathBuf {
        self.directory.join("goal-state.json")
    }

    #[cfg(test)]
    fn with_test_hook(
        mut self,
        hook: impl Fn(GoalStorageTestPhase) + Send + Sync + 'static,
    ) -> Self {
        self.test_hook = Some(Arc::new(hook));
        self
    }

    #[cfg(test)]
    fn test_phase(&self, phase: GoalStorageTestPhase) {
        if let Some(hook) = &self.test_hook {
            hook(phase);
        }
    }

    #[cfg(all(unix, test))]
    fn prepare_directory(&self) -> Result<(), ProjectError> {
        self.open_directory().map(|_| ())
    }

    #[cfg(not(unix))]
    fn unsupported(&self) -> ProjectError {
        ProjectError::new(
            "goal_storage_unsupported",
            "Secure goal persistence requires descriptor-relative filesystem operations",
            Some(&self.directory),
        )
    }

    fn validate_collaboration_cursor(
        &self,
        record: &PersistedGoalRecord,
    ) -> Result<(), ProjectError> {
        let Some(cursor) = &record.collaboration else {
            return Ok(());
        };
        cursor.intent.validate().map_err(|_| {
            ProjectError::new(
                "goal_state_incompatible",
                "Goal collaboration cursor is not a bounded sanitized intent",
                Some(&self.target()),
            )
        })?;
        let canonical_missing = cursor
            .missing_effects
            .windows(2)
            .all(|pair| pair[0] < pair[1])
            && cursor
                .missing_effects
                .iter()
                .all(|effect| MissingCollaborationEffect::ORDER.contains(effect));
        let state_matches = match cursor.state {
            PersistedReconciliationState::SyncPending => {
                cursor.missing_effects == MissingCollaborationEffect::ORDER
            }
            PersistedReconciliationState::CollaborationRepairRequired => {
                !cursor.missing_effects.is_empty()
            }
            PersistedReconciliationState::Reconciled => cursor.missing_effects.is_empty(),
        };
        if !canonical_missing
            || !state_matches
            || cursor.current_task_version < cursor.intent.claimed_task_version
            || cursor.intent.repository_id != record.repository.repository_id
        {
            return Err(ProjectError::new(
                "goal_state_incompatible",
                "Goal collaboration cursor state, versions, or repository binding are invalid",
                Some(&self.target()),
            ));
        }
        Ok(())
    }

    fn validate_record(&self, record: &PersistedGoalRecord) -> Result<Vec<u8>, ProjectError> {
        if record.version != GOAL_STATE_VERSION
            || record.stop_conditions != goal_loop_stop_conditions()
        {
            return Err(ProjectError::new(
                "goal_state_incompatible",
                "Goal state must use the current schema and complete fixed stop-condition set",
                Some(&self.target()),
            ));
        }
        self.validate_collaboration_cursor(record)?;
        let bytes = serde_json::to_vec(record).map_err(|error| {
            ProjectError::new(
                "goal_state_encode_failed",
                error.to_string(),
                Some(&self.target()),
            )
        })?;
        if bytes.len() > GOAL_STATE_MAX_BYTES
            || record.evidence_references.len() > GOAL_EVIDENCE_MAX
        {
            return Err(ProjectError::new(
                "goal_state_oversized",
                "Goal state exceeds the bounded persistence contract",
                Some(&self.target()),
            ));
        }
        Ok(bytes)
    }

    fn parse_record(&self, bytes: &[u8]) -> Result<PersistedGoalRecord, ProjectError> {
        serde_json::from_slice(bytes).map_err(|error| {
            ProjectError::new(
                "goal_state_cas_failed",
                format!("Cannot update an incompatible persisted goal: {error}"),
                Some(&self.target()),
            )
        })
    }

    fn ensure_cas(
        &self,
        current: &PersistedGoalRecord,
        expected: &PersistedGoalRecord,
    ) -> Result<(), ProjectError> {
        if current.revision != expected.revision
            || current.current_run.run_id != expected.current_run.run_id
            || current.current_run.nonterminal != expected.current_run.nonterminal
            || current.current_run.event_cursor != expected.current_run.event_cursor
        {
            return Err(ProjectError::new(
                "goal_state_stale_write",
                "Persisted goal revision, run, terminal state, or event cursor changed",
                Some(&self.target()),
            ));
        }
        Ok(())
    }

    fn next_revision(&self, revision: u64) -> Result<u64, ProjectError> {
        revision.checked_add(1).ok_or_else(|| {
            ProjectError::new(
                "goal_state_revision_exhausted",
                "Persisted goal revision cannot advance",
                Some(&self.target()),
            )
        })
    }

    #[cfg(unix)]
    fn create_run(
        &self,
        expected: Option<&PersistedGoalRecord>,
        mut record: PersistedGoalRecord,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        self.with_lock(|directory| {
            let (previous, identity) = self.read_record_locked(directory)?;
            match (&previous, expected) {
                (None, None) => {}
                (Some(current), Some(expected)) => self.ensure_cas(current, expected)?,
                _ => {
                    return Err(ProjectError::new(
                        "goal_state_stale_write",
                        "Persisted goal changed after the run was prepared",
                        Some(&self.target()),
                    ))
                }
            }
            record.revision = previous
                .as_ref()
                .map(|previous| self.next_revision(previous.revision))
                .transpose()?
                .unwrap_or(1);
            if let Some(previous) = previous.filter(|previous| {
                previous.repository.canonical_path == record.repository.canonical_path
                    && previous.repository.repository_id == record.repository.repository_id
            }) {
                record.last_checkpoint = previous.last_checkpoint;
                record.evidence_references = previous.evidence_references;
                record.collaboration = previous.collaboration;
            }
            let bytes = self.validate_record(&record)?;
            self.write_record_locked(directory, &bytes, identity)?;
            Ok(record)
        })
    }

    #[cfg(not(unix))]
    fn create_run(
        &self,
        _expected: Option<&PersistedGoalRecord>,
        _record: PersistedGoalRecord,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        Err(self.unsupported())
    }

    #[cfg(unix)]
    fn advance_event(
        &self,
        expected: &PersistedGoalRecord,
        event_cursor: u64,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        self.with_lock(|directory| {
            let (Some(mut current), identity) = self.read_record_locked(directory)? else {
                return Err(ProjectError::new(
                    "goal_state_stale_write",
                    "Persisted goal was cleared before the event update",
                    Some(&self.target()),
                ));
            };
            self.ensure_cas(&current, expected)?;
            if !current.current_run.nonterminal {
                return Err(ProjectError::new(
                    "goal_state_terminal_write",
                    "A terminal run cannot accept more events",
                    Some(&self.target()),
                ));
            }
            if event_cursor < current.current_run.event_cursor {
                return Err(ProjectError::new(
                    "goal_state_cursor_regression",
                    "Goal event cursor cannot move backwards",
                    Some(&self.target()),
                ));
            }
            current.revision = self.next_revision(current.revision)?;
            current.current_run.event_cursor = event_cursor;
            let bytes = self.validate_record(&current)?;
            self.write_record_locked(directory, &bytes, identity)?;
            Ok(current)
        })
    }

    #[cfg(not(unix))]
    fn advance_event(
        &self,
        _expected: &PersistedGoalRecord,
        _event_cursor: u64,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        Err(self.unsupported())
    }

    #[cfg(unix)]
    fn finish_run(
        &self,
        expected: &PersistedGoalRecord,
        event_cursor: u64,
        checkpoint: VerifiedGoalCheckpoint,
        evidence_references: Vec<GoalEvidenceReference>,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        self.with_lock(|directory| {
            let (Some(mut current), identity) = self.read_record_locked(directory)? else {
                return Err(ProjectError::new(
                    "goal_state_stale_write",
                    "Persisted goal was cleared before the final checkpoint",
                    Some(&self.target()),
                ));
            };
            self.ensure_cas(&current, expected)?;
            if !current.current_run.nonterminal {
                return Err(ProjectError::new(
                    "goal_state_terminal_write",
                    "The run already reached a terminal checkpoint",
                    Some(&self.target()),
                ));
            }
            if event_cursor < current.current_run.event_cursor {
                return Err(ProjectError::new(
                    "goal_state_cursor_regression",
                    "Final checkpoint cannot regress the event cursor",
                    Some(&self.target()),
                ));
            }
            current.revision = self.next_revision(current.revision)?;
            current.current_run.event_cursor = event_cursor;
            current.current_run.nonterminal = false;
            current.last_checkpoint = Some(checkpoint);
            current.evidence_references = evidence_references;
            let bytes = self.validate_record(&current)?;
            self.write_record_locked(directory, &bytes, identity)?;
            Ok(current)
        })
    }

    #[cfg(unix)]
    fn begin_collaboration_reconciliation(
        &self,
        expected: &PersistedGoalRecord,
        intent: RemoteCompletionIntent,
        capability_guard: &dyn Fn(&RemoteCompletionIntent) -> Result<(), ProjectError>,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        capability_guard(&intent)?;
        self.with_lock(|directory| {
            let (Some(mut current), identity) = self.read_record_locked(directory)? else {
                return Err(ProjectError::new(
                    "goal_state_stale_write",
                    "Persisted goal was cleared before collaboration reconciliation",
                    Some(&self.target()),
                ));
            };
            self.ensure_cas(&current, expected)?;
            let checkpoint_matches = current.last_checkpoint.as_ref().is_some_and(|checkpoint| {
                checkpoint.task_path == intent.local_task_path
                    && checkpoint.task_sha256 == intent.local_task_sha256
            });
            if current.current_run.nonterminal || !checkpoint_matches {
                return Err(ProjectError::new(
                    "goal_collaboration_unverified_checkpoint",
                    "Only the exact repository-verified checkpoint may begin remote completion",
                    Some(&self.target()),
                ));
            }
            current.revision = self.next_revision(current.revision)?;
            current.collaboration = Some(PersistedCollaborationCursor {
                state: PersistedReconciliationState::SyncPending,
                current_task_version: intent.claimed_task_version,
                missing_effects: MissingCollaborationEffect::ORDER.to_vec(),
                intent,
            });
            let bytes = self.validate_record(&current)?;
            self.write_record_locked(directory, &bytes, identity)?;
            Ok(current)
        })
    }

    #[cfg(not(unix))]
    fn begin_collaboration_reconciliation(
        &self,
        _expected: &PersistedGoalRecord,
        _intent: RemoteCompletionIntent,
        _capability_guard: &dyn Fn(&RemoteCompletionIntent) -> Result<(), ProjectError>,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        Err(self.unsupported())
    }

    #[cfg(unix)]
    fn finish_collaboration_reconciliation(
        &self,
        expected: &PersistedGoalRecord,
        outcome: &PostLocalCommitCollaborationOutcome,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        self.with_lock(|directory| {
            let (Some(mut current), identity) = self.read_record_locked(directory)? else {
                return Err(ProjectError::new(
                    "goal_state_stale_write",
                    "Persisted goal was cleared before collaboration result persistence",
                    Some(&self.target()),
                ));
            };
            self.ensure_cas(&current, expected)?;
            let cursor = current.collaboration.as_mut().ok_or_else(|| {
                ProjectError::new(
                    "goal_collaboration_cursor_missing",
                    "Collaboration result has no durable completion intent",
                    Some(&self.target()),
                )
            })?;
            match (&outcome.reconciliation, &outcome.evidence_handoff) {
                (
                    ReconciliationState::Reconciled,
                    EvidenceHandoffResult::Synchronized { remote_version, .. },
                ) => {
                    cursor.state = PersistedReconciliationState::Reconciled;
                    cursor.current_task_version = *remote_version;
                    cursor.missing_effects.clear();
                }
                (
                    ReconciliationState::RepairRequired,
                    EvidenceHandoffResult::Partial {
                        remote_version,
                        missing_effects,
                        ..
                    },
                ) => {
                    cursor.state = PersistedReconciliationState::CollaborationRepairRequired;
                    if let Some(version) = remote_version {
                        cursor.current_task_version = *version;
                    }
                    cursor.missing_effects = missing_effects.clone();
                }
                _ => {
                    return Err(ProjectError::new(
                        "goal_collaboration_result_invalid",
                        "Collaboration result does not match the durable reconciliation states",
                        Some(&self.target()),
                    ))
                }
            }
            current.revision = self.next_revision(current.revision)?;
            let bytes = self.validate_record(&current)?;
            self.write_record_locked(directory, &bytes, identity)?;
            Ok(current)
        })
    }

    #[cfg(not(unix))]
    fn finish_collaboration_reconciliation(
        &self,
        _expected: &PersistedGoalRecord,
        _outcome: &PostLocalCommitCollaborationOutcome,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        Err(self.unsupported())
    }

    #[cfg(not(unix))]
    fn finish_run(
        &self,
        _expected: &PersistedGoalRecord,
        _event_cursor: u64,
        _checkpoint: VerifiedGoalCheckpoint,
        _evidence_references: Vec<GoalEvidenceReference>,
    ) -> Result<PersistedGoalRecord, ProjectError> {
        Err(self.unsupported())
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GoalFileIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
}

#[cfg(unix)]
impl GoalStateStorage {
    fn storage_error(&self, code: &str, error: impl std::fmt::Display) -> ProjectError {
        let code = if error.to_string().contains("symlink") {
            "goal_storage_symlink"
        } else {
            code
        };
        ProjectError::new(code, error.to_string(), Some(&self.directory))
    }

    fn component_name(component: &std::ffi::OsStr) -> Result<std::ffi::CString, ProjectError> {
        use std::os::unix::ffi::OsStrExt;
        std::ffi::CString::new(component.as_bytes()).map_err(|_| {
            ProjectError::new(
                "goal_storage_unavailable",
                "Goal storage path contains a NUL byte",
                None,
            )
        })
    }

    fn descriptor_identity(file: &File) -> std::io::Result<(GoalFileIdentity, libc::mode_t)> {
        use std::os::fd::AsRawFd;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let stat = unsafe { stat.assume_init() };
        Ok((
            GoalFileIdentity {
                device: stat.st_dev,
                inode: stat.st_ino,
            },
            stat.st_mode,
        ))
    }

    fn open_directory(&self) -> Result<File, ProjectError> {
        use std::os::fd::FromRawFd;
        if !self.directory.is_absolute() {
            return Err(ProjectError::new(
                "goal_storage_unavailable",
                "Goal storage directory must be absolute",
                Some(&self.directory),
            ));
        }
        let root = unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if root < 0 {
            return Err(
                self.storage_error("goal_storage_unavailable", std::io::Error::last_os_error())
            );
        }
        let mut current = unsafe { File::from_raw_fd(root) };
        for component in self.directory.components() {
            let Component::Normal(component) = component else {
                if matches!(component, Component::RootDir) {
                    continue;
                }
                return Err(ProjectError::new(
                    "goal_storage_unavailable",
                    "Goal storage path contains an unsupported component",
                    Some(&self.directory),
                ));
            };
            let name = Self::component_name(component)?;
            use std::os::fd::AsRawFd;
            let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
            let mut descriptor = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
            if descriptor < 0
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT)
            {
                if unsafe { libc::mkdirat(current.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                    let error = std::io::Error::last_os_error();
                    if error.raw_os_error() != Some(libc::EEXIST) {
                        return Err(self.storage_error("goal_storage_unavailable", error));
                    }
                }
                descriptor = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
            }
            if descriptor < 0 {
                let error = std::io::Error::last_os_error();
                let component_is_symlink = self
                    .named_identity(&current, &name)?
                    .is_some_and(|(_, mode)| mode & libc::S_IFMT == libc::S_IFLNK);
                let message = if error.raw_os_error() == Some(libc::ELOOP) || component_is_symlink {
                    "Goal storage directory traversal encountered a symlink".into()
                } else {
                    error.to_string()
                };
                return Err(self.storage_error("goal_storage_unavailable", message));
            }
            let next = unsafe { File::from_raw_fd(descriptor) };
            let (_, mode) = Self::descriptor_identity(&next)
                .map_err(|error| self.storage_error("goal_storage_unavailable", error))?;
            if mode & libc::S_IFMT != libc::S_IFDIR {
                return Err(self.storage_error(
                    "goal_storage_unavailable",
                    "Goal storage path component is not a directory",
                ));
            }
            current = next;
        }
        Ok(current)
    }

    fn named_identity(
        &self,
        directory: &File,
        name: &std::ffi::CStr,
    ) -> Result<Option<(GoalFileIdentity, libc::mode_t)>, ProjectError> {
        use std::os::fd::AsRawFd;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ENOENT) {
                return Ok(None);
            }
            return Err(self.storage_error("goal_storage_unavailable", error));
        }
        let stat = unsafe { stat.assume_init() };
        Ok(Some((
            GoalFileIdentity {
                device: stat.st_dev,
                inode: stat.st_ino,
            },
            stat.st_mode,
        )))
    }

    fn verify_named_regular(
        &self,
        directory: &File,
        name: &std::ffi::CStr,
        expected: GoalFileIdentity,
    ) -> Result<(), ProjectError> {
        let Some((actual, mode)) = self.named_identity(directory, name)? else {
            return Err(ProjectError::new(
                "goal_state_stale_write",
                "Goal storage entry disappeared during the operation",
                Some(&self.directory),
            ));
        };
        if mode & libc::S_IFMT == libc::S_IFLNK {
            return Err(ProjectError::new(
                "goal_storage_symlink",
                "Goal state and lock files may not be symlinks",
                Some(&self.directory),
            ));
        }
        if mode & libc::S_IFMT != libc::S_IFREG || actual != expected {
            return Err(ProjectError::new(
                "goal_state_stale_write",
                "Goal storage entry changed during the operation",
                Some(&self.directory),
            ));
        }
        Ok(())
    }

    fn open_named_regular(
        &self,
        directory: &File,
        name: &std::ffi::CStr,
        flags: libc::c_int,
        mode: libc::mode_t,
        code: &str,
    ) -> Result<(File, GoalFileIdentity), ProjectError> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                mode as libc::c_uint,
            )
        };
        if descriptor < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ENOENT) {
                return Err(ProjectError::new(
                    "goal_storage_entry_missing",
                    error.to_string(),
                    Some(&self.directory),
                ));
            }
            let message = if error.raw_os_error() == Some(libc::ELOOP) {
                "Goal state and lock files may not be symlinks".into()
            } else {
                error.to_string()
            };
            return Err(self.storage_error(code, message));
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let (identity, descriptor_mode) =
            Self::descriptor_identity(&file).map_err(|error| self.storage_error(code, error))?;
        if descriptor_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(ProjectError::new(
                "goal_storage_unavailable",
                "Goal state and lock entries must be regular files",
                Some(&self.directory),
            ));
        }
        self.verify_named_regular(directory, name, identity)?;
        Ok((file, identity))
    }

    fn with_lock<T>(
        &self,
        action: impl FnOnce(&File) -> Result<T, ProjectError>,
    ) -> Result<T, ProjectError> {
        let directory = self.open_directory()?;
        #[cfg(test)]
        self.test_phase(GoalStorageTestPhase::DirectoryOpened);
        let (directory_identity, _) = Self::descriptor_identity(&directory)
            .map_err(|error| self.storage_error("goal_storage_unavailable", error))?;
        let (lock, lock_identity) = self.open_named_regular(
            &directory,
            c"goal-state.lock",
            libc::O_CREAT | libc::O_RDWR,
            0o600,
            "goal_storage_lock_failed",
        )?;
        #[cfg(test)]
        self.test_phase(GoalStorageTestPhase::LockOpened);
        lock.lock_exclusive().map_err(|error| {
            ProjectError::new(
                "goal_storage_lock_failed",
                error.to_string(),
                Some(&self.directory),
            )
        })?;
        let verified_directory = self.open_directory()?;
        let (verified_identity, _) = Self::descriptor_identity(&verified_directory)
            .map_err(|error| self.storage_error("goal_storage_unavailable", error))?;
        if verified_identity != directory_identity {
            let _ = FileExt::unlock(&lock);
            return Err(ProjectError::new(
                "goal_storage_raced",
                "Goal storage directory changed while acquiring the lock",
                Some(&self.directory),
            ));
        }
        self.verify_named_regular(&directory, c"goal-state.lock", lock_identity)?;
        let result = action(&directory);
        let _ = FileExt::unlock(&lock);
        result
    }

    fn read_bytes_locked(
        &self,
        directory: &File,
    ) -> Result<(Option<Vec<u8>>, Option<GoalFileIdentity>), ProjectError> {
        use std::os::fd::AsRawFd;
        let (mut file, identity) = match self.open_named_regular(
            directory,
            c"goal-state.json",
            libc::O_RDONLY,
            0,
            "goal_state_read_failed",
        ) {
            Ok(result) => result,
            Err(error) if error.code == "goal_storage_entry_missing" => {
                return Ok((None, None));
            }
            Err(error) => return Err(error),
        };
        #[cfg(test)]
        self.test_phase(GoalStorageTestPhase::StateOpened);
        self.verify_named_regular(directory, c"goal-state.json", identity)?;
        let (_, mode) = Self::descriptor_identity(&file)
            .map_err(|error| self.storage_error("goal_state_read_failed", error))?;
        if mode & libc::S_IFMT != libc::S_IFREG {
            return Err(ProjectError::new(
                "goal_state_read_failed",
                "Persisted goal state must be a regular file",
                Some(&self.target()),
            ));
        }
        let length = unsafe { libc::lseek(file.as_raw_fd(), 0, libc::SEEK_END) };
        if length < 0 {
            return Err(
                self.storage_error("goal_state_read_failed", std::io::Error::last_os_error())
            );
        }
        if length as u64 > GOAL_STATE_MAX_BYTES as u64 {
            return Err(ProjectError::new(
                "goal_state_oversized",
                "Persisted goal state exceeds the read limit",
                Some(&self.target()),
            ));
        }
        file.seek(SeekFrom::Start(0)).map_err(|error| {
            ProjectError::new(
                "goal_state_read_failed",
                error.to_string(),
                Some(&self.target()),
            )
        })?;
        let mut bytes = Vec::with_capacity(length as usize);
        file.read_to_end(&mut bytes).map_err(|error| {
            ProjectError::new(
                "goal_state_read_failed",
                error.to_string(),
                Some(&self.target()),
            )
        })?;
        Ok((Some(bytes), Some(identity)))
    }

    fn read_record_locked(
        &self,
        directory: &File,
    ) -> Result<(Option<PersistedGoalRecord>, Option<GoalFileIdentity>), ProjectError> {
        let (bytes, identity) = self.read_bytes_locked(directory)?;
        Ok((
            bytes
                .as_deref()
                .map(|bytes| self.parse_record(bytes))
                .transpose()?,
            identity,
        ))
    }

    fn write_record_locked(
        &self,
        directory: &File,
        bytes: &[u8],
        expected: Option<GoalFileIdentity>,
    ) -> Result<(), ProjectError> {
        use std::os::fd::AsRawFd;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary_name =
            std::ffi::CString::new(format!(".goal-state.{}.{nonce}.tmp", std::process::id()))
                .expect("generated temporary name has no NUL");
        let (mut temporary, temporary_identity) = self.open_named_regular(
            directory,
            &temporary_name,
            libc::O_CREAT | libc::O_EXCL | libc::O_WRONLY,
            0o600,
            "goal_state_write_failed",
        )?;
        let result = (|| {
            temporary
                .write_all(bytes)
                .and_then(|_| temporary.sync_all())
                .map_err(|error| {
                    ProjectError::new(
                        "goal_state_write_failed",
                        error.to_string(),
                        Some(&self.target()),
                    )
                })?;
            #[cfg(test)]
            self.test_phase(GoalStorageTestPhase::TemporarySynced);
            match (
                expected,
                self.named_identity(directory, c"goal-state.json")?,
            ) {
                (None, None) => {}
                (Some(expected), Some((actual, mode)))
                    if actual == expected && mode & libc::S_IFMT == libc::S_IFREG => {}
                (_, Some((_, mode))) if mode & libc::S_IFMT == libc::S_IFLNK => {
                    return Err(ProjectError::new(
                        "goal_storage_symlink",
                        "Goal state may not be replaced through a symlink",
                        Some(&self.target()),
                    ))
                }
                _ => {
                    return Err(ProjectError::new(
                        "goal_state_stale_write",
                        "Goal state changed before atomic replacement",
                        Some(&self.target()),
                    ))
                }
            }
            self.verify_named_regular(directory, &temporary_name, temporary_identity)?;
            if unsafe {
                libc::renameat(
                    directory.as_raw_fd(),
                    temporary_name.as_ptr(),
                    directory.as_raw_fd(),
                    c"goal-state.json".as_ptr(),
                )
            } != 0
            {
                return Err(
                    self.storage_error("goal_state_write_failed", std::io::Error::last_os_error())
                );
            }
            directory.sync_all().map_err(|error| {
                ProjectError::new(
                    "goal_state_write_failed",
                    error.to_string(),
                    Some(&self.directory),
                )
            })
        })();
        if result.is_err() {
            unsafe {
                libc::unlinkat(directory.as_raw_fd(), temporary_name.as_ptr(), 0);
            }
        }
        result
    }

    fn read_bytes(&self) -> Result<Option<Vec<u8>>, ProjectError> {
        self.with_lock(|directory| self.read_bytes_locked(directory).map(|(bytes, _)| bytes))
    }

    fn read_record(&self) -> Result<Option<PersistedGoalRecord>, ProjectError> {
        self.with_lock(|directory| self.read_record_locked(directory).map(|(record, _)| record))
    }

    fn clear(&self) -> Result<(), ProjectError> {
        use std::os::fd::AsRawFd;
        self.with_lock(|directory| {
            let (_, identity) = self.read_bytes_locked(directory)?;
            let Some(identity) = identity else {
                return Ok(());
            };
            self.verify_named_regular(directory, c"goal-state.json", identity)?;
            if unsafe { libc::unlinkat(directory.as_raw_fd(), c"goal-state.json".as_ptr(), 0) } != 0
            {
                return Err(
                    self.storage_error("goal_state_clear_failed", std::io::Error::last_os_error())
                );
            }
            directory.sync_all().map_err(|error| {
                ProjectError::new(
                    "goal_state_clear_failed",
                    error.to_string(),
                    Some(&self.directory),
                )
            })
        })
    }

    #[cfg(test)]
    fn write_for_test(&self, mut record: PersistedGoalRecord) -> Result<(), ProjectError> {
        self.with_lock(|directory| {
            let (_, identity) = self.read_bytes_locked(directory)?;
            if record.revision == 0 {
                record.revision = 1;
            }
            let bytes = self.validate_record(&record)?;
            self.write_record_locked(directory, &bytes, identity)
        })
    }
}

#[cfg(not(unix))]
impl GoalStateStorage {
    fn read_bytes(&self) -> Result<Option<Vec<u8>>, ProjectError> {
        Err(self.unsupported())
    }

    fn read_record(&self) -> Result<Option<PersistedGoalRecord>, ProjectError> {
        Err(self.unsupported())
    }

    fn clear(&self) -> Result<(), ProjectError> {
        Err(self.unsupported())
    }
}

const CODEX_EXECUTABLE: &str = "/Users/pax/.nvm/versions/node/v24.14.0/bin/codex";
const RUNTIME_PROMPT_LIMIT: usize = 4 * 1024;
const RUNTIME_OUTPUT_LIMIT: usize = 256 * 1024;
const RUNTIME_TIMEOUT: Duration = Duration::from_secs(120);
const BOUNDED_TASK_RUNTIME_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const RUNTIME_VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const CODEX_FIXTURE_JSONL: &str = concat!(
    "{\"type\":\"thread.started\",\"thread_id\":\"fixture-thread\"}\n",
    "{\"type\":\"turn.started\"}\n",
    "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"Fixture response only; no provider process executed.\"}}\n",
    "{\"type\":\"fixture.future_event\",\"detail\":\"preserved as unknown\"}\n",
    "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":12,\"output_tokens\":8}}\n",
);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum RuntimeMode {
    Fixture,
    Live,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RuntimeInvocation {
    mode: RuntimeMode,
    prompt: Option<String>,
    confirmed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum RuntimeOutcome {
    Completed,
    ConfirmationRequired,
    InvalidPrompt,
    NonzeroExit,
    MalformedOutput,
    OutputOverflow,
    Cancelled,
    TimedOut,
    MissingRuntime,
    StartFailed,
    CleanupFailed,
    CapabilityUnavailable,
    UnsupportedPlatform,
    ProviderError,
    ChannelFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum RuntimeEventKind {
    Session,
    Turn,
    Message,
    Command,
    FileChange,
    Tool,
    Reasoning,
    Usage,
    Error,
    Stderr,
    Unknown,
    Malformed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum PayloadEncoding {
    Utf8,
    Hex,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EncodedPayload {
    encoding: PayloadEncoding,
    data: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeEvent {
    sequence: usize,
    kind: RuntimeEventKind,
    provider_type: Option<String>,
    summary: String,
    raw_payload: EncodedPayload,
    provenance: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeCapabilities {
    event_stream: bool,
    cancellation: bool,
    timeout: bool,
    raw_payload: bool,
    fixture: bool,
    live: bool,
    repository_authority: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeProvenance {
    adapter: String,
    provider: String,
    mode: RuntimeMode,
    executable: String,
    runtime_version: Option<String>,
    project_root: String,
    argv: Vec<String>,
    simulated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeResult {
    run_id: String,
    outcome: RuntimeOutcome,
    executed: bool,
    success: bool,
    exit_status: Option<i32>,
    events: Vec<RuntimeEvent>,
    stdout: EncodedPayload,
    stderr: EncodedPayload,
    stdout_truncated: bool,
    stderr_truncated: bool,
    failure: Option<String>,
    capabilities: RuntimeCapabilities,
    provenance: RuntimeProvenance,
    repository_authority_advanced: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeRunHandle {
    run_id: String,
    capabilities: RuntimeCapabilities,
    provenance: RuntimeProvenance,
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum RuntimeStreamMessage {
    Started { handle: RuntimeRunHandle },
    Event { run_id: String, event: RuntimeEvent },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeCancellation {
    cancellation_requested: bool,
    message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OperationKind {
    ArtifactPlan,
    GitHandoff,
    SkillSetup,
    Helper,
    Runtime,
    BoundedTask,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundedTaskFinalizationState {
    Open,
    CancellationAccepted,
    ResultCommitted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundedTaskFinalizationDecision {
    CancellationAccepted,
    ResultCommitted,
}

#[derive(Clone)]
struct ActiveOperation {
    root: PathBuf,
    kind: OperationKind,
    run_id: Option<String>,
    cancel: Arc<AtomicBool>,
    explicit_user_cancellation: Arc<AtomicBool>,
    bounded_task_finalization: Option<BoundedTaskFinalizationState>,
}

struct BoundedTaskConfirmation {
    root: PathBuf,
    token: String,
    baseline_token: String,
    scope: BoundedTaskConfirmationScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BoundedTaskConfirmationScope {
    Local,
    Shared(SharedExecutionBinding),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SharedConflictKey {
    root: PathBuf,
    session_id: String,
    workspace_id: String,
    task_path: String,
    actor: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SharedConflictRecord {
    key: SharedConflictKey,
    count: u8,
}

#[derive(Default)]
struct OperationRegistry {
    active: Mutex<Option<ActiveOperation>>,
    bounded_task_confirmation: Mutex<Option<BoundedTaskConfirmation>>,
    shared_conflict: Mutex<Option<SharedConflictRecord>>,
}

struct OperationLease {
    registry: Arc<OperationRegistry>,
    root: PathBuf,
    kind: OperationKind,
    run_id: Option<String>,
    cancel: Arc<AtomicBool>,
    explicit_user_cancellation: Arc<AtomicBool>,
}

impl OperationRegistry {
    fn begin(
        self: &Arc<Self>,
        root: &Path,
        kind: OperationKind,
        run_id: Option<String>,
    ) -> Result<OperationLease, ProjectError> {
        let mut active = self.active.lock().map_err(|_| {
            ProjectError::new(
                "operation_lock_failed",
                "Local operation registry is poisoned",
                Some(root),
            )
        })?;
        if active.is_some() {
            return Err(ProjectError::new(
                "operation_in_progress",
                "Another local operation is already in progress",
                Some(root),
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let explicit_user_cancellation = Arc::new(AtomicBool::new(false));
        *active = Some(ActiveOperation {
            root: root.to_path_buf(),
            kind,
            run_id: run_id.clone(),
            cancel: Arc::clone(&cancel),
            explicit_user_cancellation: Arc::clone(&explicit_user_cancellation),
            bounded_task_finalization: (kind == OperationKind::BoundedTask)
                .then_some(BoundedTaskFinalizationState::Open),
        });
        Ok(OperationLease {
            registry: Arc::clone(self),
            root: root.to_path_buf(),
            kind,
            run_id,
            cancel,
            explicit_user_cancellation,
        })
    }

    fn cancel_root(&self, root: &Path, kind: OperationKind) -> Result<bool, ProjectError> {
        let active = self.active.lock().map_err(|_| {
            ProjectError::new(
                "operation_lock_failed",
                "Local operation registry is poisoned",
                Some(root),
            )
        })?;
        match active.as_ref() {
            Some(process) if process.root == root && process.kind == kind => {
                process
                    .explicit_user_cancellation
                    .store(true, Ordering::Release);
                process.cancel.store(true, Ordering::Release);
                Ok(true)
            }
            Some(_) => Err(ProjectError::new(
                "operation_mismatch",
                "The active local operation does not match this cancellation request",
                Some(root),
            )),
            None => Ok(false),
        }
    }

    fn cancel_run(&self, run_id: &str) -> Result<bool, ProjectError> {
        let active = self.active.lock().map_err(|_| {
            ProjectError::new(
                "operation_lock_failed",
                "Local operation registry is poisoned",
                None,
            )
        })?;
        match active.as_ref() {
            Some(process)
                if process.kind == OperationKind::Runtime
                    && process.run_id.as_deref() == Some(run_id) =>
            {
                process
                    .explicit_user_cancellation
                    .store(true, Ordering::Release);
                process.cancel.store(true, Ordering::Release);
                Ok(true)
            }
            Some(_) => Err(ProjectError::new(
                "runtime_run_mismatch",
                "The runtime run ID does not match the active operation",
                None,
            )),
            None => Ok(false),
        }
    }

    fn cancel_bounded_task(&self, run_id: &str) -> Result<bool, ProjectError> {
        let mut active = self.active.lock().map_err(|_| {
            ProjectError::new(
                "operation_lock_failed",
                "Local operation registry is poisoned",
                None,
            )
        })?;
        match active.as_mut() {
            Some(process)
                if process.kind == OperationKind::BoundedTask
                    && process.run_id.as_deref() == Some(run_id) =>
            {
                match process.bounded_task_finalization {
                    Some(BoundedTaskFinalizationState::Open) => {
                        process
                            .explicit_user_cancellation
                            .store(true, Ordering::Release);
                        process.cancel.store(true, Ordering::Release);
                        // The registry lock is the single linearization point shared with
                        // result/checkpoint finalization.
                        process.bounded_task_finalization =
                            Some(BoundedTaskFinalizationState::CancellationAccepted);
                        Ok(true)
                    }
                    Some(BoundedTaskFinalizationState::CancellationAccepted) => Ok(true),
                    Some(BoundedTaskFinalizationState::ResultCommitted) => Ok(false),
                    None => Err(ProjectError::new(
                        "bounded_task_finalization_invalid",
                        "Active bounded task omitted its finalization state",
                        None,
                    )),
                }
            }
            Some(_) => Err(ProjectError::new(
                "bounded_task_run_mismatch",
                "The bounded task run ID does not match the active operation",
                None,
            )),
            None => Ok(false),
        }
    }

    fn invalidate_bounded_task_confirmation(&self, root: &Path) -> Result<(), ProjectError> {
        let mut confirmation = self.bounded_task_confirmation.lock().map_err(|_| {
            ProjectError::new(
                "controller_confirmation_lock_failed",
                "Bounded task confirmation registry is poisoned",
                Some(root),
            )
        })?;
        *confirmation = None;
        Ok(())
    }

    fn issue_bounded_task_confirmation(
        &self,
        root: &Path,
        baseline_token: String,
    ) -> Result<String, ProjectError> {
        self.issue_bounded_task_confirmation_with_scope(
            root,
            baseline_token,
            BoundedTaskConfirmationScope::Local,
        )
    }

    fn issue_shared_bounded_task_confirmation(
        &self,
        root: &Path,
        baseline_token: String,
        binding: SharedExecutionBinding,
    ) -> Result<String, ProjectError> {
        self.issue_bounded_task_confirmation_with_scope(
            root,
            baseline_token,
            BoundedTaskConfirmationScope::Shared(binding),
        )
    }

    fn issue_bounded_task_confirmation_with_scope(
        &self,
        root: &Path,
        baseline_token: String,
        scope: BoundedTaskConfirmationScope,
    ) -> Result<String, ProjectError> {
        let token = format!("confirmation:{}", native_runtime_run_id()?);
        let mut confirmation = self.bounded_task_confirmation.lock().map_err(|_| {
            ProjectError::new(
                "controller_confirmation_lock_failed",
                "Bounded task confirmation registry is poisoned",
                Some(root),
            )
        })?;
        *confirmation = Some(BoundedTaskConfirmation {
            root: root.to_path_buf(),
            token: token.clone(),
            baseline_token,
            scope,
        });
        Ok(token)
    }

    fn consume_bounded_task_confirmation(
        &self,
        root: &Path,
        token: &str,
        expected_scope: &BoundedTaskConfirmationScope,
    ) -> Result<String, ProjectError> {
        let mut confirmation = self.bounded_task_confirmation.lock().map_err(|_| {
            ProjectError::new(
                "controller_confirmation_lock_failed",
                "Bounded task confirmation registry is poisoned",
                Some(root),
            )
        })?;
        if confirmation.as_ref().is_none_or(|confirmation| {
            confirmation.root != root
                || confirmation.token != token
                || &confirmation.scope != expected_scope
        }) {
            return Err(ProjectError::new(
                "controller_confirmation_consumed_or_stale",
                "Bounded task confirmation was already consumed, replaced, issued for a different local/shared scope, or issued by a prior app process; preview and confirm again",
                Some(root),
            ));
        }
        Ok(confirmation
            .take()
            .expect("matching confirmation checked above")
            .baseline_token)
    }

    fn shared_confirmation_binding(
        &self,
        root: &Path,
        token: &str,
        session_id: &str,
    ) -> Result<SharedExecutionBinding, ProjectError> {
        let confirmation = self.bounded_task_confirmation.lock().map_err(|_| {
            ProjectError::new(
                "controller_confirmation_lock_failed",
                "Bounded task confirmation registry is poisoned",
                Some(root),
            )
        })?;
        let Some(BoundedTaskConfirmation {
            root: confirmation_root,
            token: confirmation_token,
            scope: BoundedTaskConfirmationScope::Shared(binding),
            ..
        }) = confirmation.as_ref()
        else {
            return Err(ProjectError::new(
                "controller_confirmation_consumed_or_stale",
                "A current shared preview and explicit confirmation are required",
                Some(root),
            ));
        };
        if confirmation_root != root
            || confirmation_token != token
            || binding.session.session_id.as_str() != session_id
        {
            return Err(ProjectError::new(
                "controller_confirmation_consumed_or_stale",
                "Shared confirmation does not match the selected project or native session",
                Some(root),
            ));
        }
        Ok(binding.clone())
    }

    fn record_shared_conflict(
        &self,
        root: &Path,
        binding: &SharedExecutionBinding,
    ) -> Result<(u8, RepairHint), ProjectError> {
        let key = SharedConflictKey {
            root: root.to_path_buf(),
            session_id: binding.session.session_id.as_str().into(),
            workspace_id: binding.session.workspace_id.clone(),
            task_path: binding.remote.task_path.clone(),
            actor: binding.session.actor.clone(),
        };
        let mut record = self.shared_conflict.lock().map_err(|_| {
            ProjectError::new(
                "controller_conflict_lock_failed",
                "Shared conflict registry is poisoned",
                Some(root),
            )
        })?;
        let count = match record.as_ref() {
            Some(record) if record.key == key => record.count.saturating_add(1),
            _ => 1,
        };
        *record = Some(SharedConflictRecord {
            key,
            count: count.min(2),
        });
        Ok((
            count.min(2),
            if count >= 2 {
                RepairHint::inspect_repeated_conflict()
            } else {
                RepairHint::refresh_conflict()
            },
        ))
    }

    fn clear_shared_conflict(
        &self,
        root: &Path,
        binding: &SharedExecutionBinding,
    ) -> Result<(), ProjectError> {
        let mut record = self.shared_conflict.lock().map_err(|_| {
            ProjectError::new(
                "controller_conflict_lock_failed",
                "Shared conflict registry is poisoned",
                Some(root),
            )
        })?;
        if record.as_ref().is_some_and(|record| {
            record.key.root == root
                && record.key.session_id == binding.session.session_id.as_str()
                && record.key.workspace_id == binding.session.workspace_id
                && record.key.task_path == binding.remote.task_path
                && record.key.actor == binding.session.actor
        }) {
            *record = None;
        }
        Ok(())
    }
}

impl OperationLease {
    fn finalize_bounded_task(
        &self,
        commit: impl FnOnce() -> Result<(), ProjectError>,
    ) -> Result<BoundedTaskFinalizationDecision, ProjectError> {
        let mut active = self.registry.active.lock().map_err(|_| {
            ProjectError::new(
                "operation_lock_failed",
                "Local operation registry is poisoned",
                Some(&self.root),
            )
        })?;
        let process = active
            .as_mut()
            .filter(|process| {
                process.root == self.root
                    && process.kind == OperationKind::BoundedTask
                    && process.run_id == self.run_id
                    && Arc::ptr_eq(&process.cancel, &self.cancel)
            })
            .ok_or_else(|| {
                ProjectError::new(
                    "bounded_task_finalization_mismatch",
                    "Bounded task finalization does not match the active native run",
                    Some(&self.root),
                )
            })?;
        match process.bounded_task_finalization {
            Some(BoundedTaskFinalizationState::CancellationAccepted) => {
                return Ok(BoundedTaskFinalizationDecision::CancellationAccepted)
            }
            Some(BoundedTaskFinalizationState::Open) => {
                // Commit the terminal result while holding the same lock cancellation uses,
                // then release it before storage I/O so there is no registry/storage lock cycle.
                process.bounded_task_finalization =
                    Some(BoundedTaskFinalizationState::ResultCommitted);
            }
            Some(BoundedTaskFinalizationState::ResultCommitted) => {
                return Err(ProjectError::new(
                    "bounded_task_finalization_replayed",
                    "Bounded task result finalization was already committed",
                    Some(&self.root),
                ))
            }
            None => {
                return Err(ProjectError::new(
                    "bounded_task_finalization_invalid",
                    "Active bounded task omitted its finalization state",
                    Some(&self.root),
                ))
            }
        }
        drop(active);
        commit()?;
        Ok(BoundedTaskFinalizationDecision::ResultCommitted)
    }
}

impl Drop for OperationLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.registry.active.lock() {
            if active.as_ref().is_some_and(|process| {
                process.root == self.root
                    && process.kind == self.kind
                    && process.run_id == self.run_id
                    && Arc::ptr_eq(&process.cancel, &self.cancel)
            }) {
                *active = None;
            }
        }
    }
}

pub(crate) fn operation_registry() -> Arc<OperationRegistry> {
    Arc::clone(LOCAL_OPERATIONS.get_or_init(|| Arc::new(OperationRegistry::default())))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessTermination {
    Completed,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessRunFailureKind {
    CancelledBeforeSpawn,
    MissingExecutable,
    Start,
    Cleanup,
}

#[derive(Debug)]
struct ProcessRunFailure {
    kind: ProcessRunFailureKind,
    message: String,
}

impl ProcessRunFailure {
    fn new(kind: ProcessRunFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

struct BoundedProcessOutput {
    status: std::process::ExitStatus,
    termination: ProcessTermination,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

fn canonical_root(root: &str) -> Result<PathBuf, String> {
    let path =
        fs::canonicalize(root).map_err(|error| format!("Cannot open project root: {error}"))?;
    if !path.is_dir() {
        return Err("Selected project root is not a directory".into());
    }
    Ok(path)
}

fn validate_relative(relative: &str) -> Result<&Path, String> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("Project file path must be non-empty and relative".into());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("Project file path cannot traverse outside the selected root".into());
    }
    Ok(path)
}

fn resolve_existing(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = validate_relative(relative)?;
    let candidate = fs::canonicalize(root.join(relative))
        .map_err(|error| format!("Cannot resolve project file: {error}"))?;
    if !candidate.starts_with(root) {
        return Err("Resolved file is outside the selected project root".into());
    }
    Ok(candidate)
}

fn resolve_regular_root_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("Cannot resolve project root: {error}"))?;
    let relative = validate_relative(relative)?;
    let candidate = canonical_root.join(relative);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|error| format!("Cannot inspect project authority file: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Project authority file must be a non-symlink regular file".into());
    }
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("Cannot resolve project authority file: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err("Project authority file resolves outside the selected root".into());
    }
    Ok(canonical)
}

pub(crate) fn validated_repository_root(root: &str) -> Result<PathBuf, ProjectError> {
    let canonical = canonical_root(root).map_err(|message| {
        ProjectError::new("invalid_project_root", message, Some(Path::new(root)))
    })?;
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&canonical)
        .output()
        .map_err(|error| {
            ProjectError::new(
                "not_repository_root",
                format!("Cannot resolve Git repository root: {error}"),
                Some(&canonical),
            )
        })?;
    if !output.status.success() {
        return Err(ProjectError::new(
            "not_repository_root",
            "Selected directory is not a Git repository root",
            Some(&canonical),
        ));
    }
    let reported = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_root = fs::canonicalize(&reported).map_err(|error| {
        ProjectError::new(
            "not_repository_root",
            format!("Cannot canonicalize Git repository root: {error}"),
            Some(Path::new(&reported)),
        )
    })?;
    if git_root != canonical {
        return Err(ProjectError::new(
            "not_repository_root",
            "Select the Git repository top-level directory, not a nested directory",
            Some(&canonical),
        ));
    }
    Ok(canonical)
}

fn validate_markdown_path(path: &Path) -> Result<(), String> {
    if path.extension().and_then(|value| value.to_str()) != Some("md") {
        return Err("Selected project file must be Markdown".into());
    }
    Ok(())
}

fn resolve_writable(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = validate_relative(relative)?;
    let candidate = root.join(relative);
    let parent = candidate
        .parent()
        .ok_or_else(|| "Project file has no parent directory".to_string())?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("Cannot resolve project file parent: {error}"))?;
    if !canonical_parent.starts_with(root) {
        return Err("Resolved file parent is outside the selected project root".into());
    }
    match fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err("Project write target cannot be a symlink".into());
            }
            let canonical_candidate = fs::canonicalize(&candidate)
                .map_err(|error| format!("Cannot resolve existing project file: {error}"))?;
            if !canonical_candidate.starts_with(root) {
                return Err("Existing project file resolves outside the selected root".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Cannot inspect project write target: {error}")),
    }
    Ok(candidate)
}

fn status_from_markdown(path: &Path) -> Result<Option<String>, std::io::Error> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .find_map(|line| line.strip_prefix("Status:").map(str::trim))
        .filter(|status| !status.is_empty())
        .map(str::to_string))
}

fn display_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled")
        .replace(['-', '_'], " ")
}

fn collect_markdown(
    root: &Path,
    directory: &str,
    files: &mut Vec<ProjectFile>,
    errors: &mut Vec<ProjectError>,
) {
    let start = root.join(directory);
    let start_metadata = match fs::symlink_metadata(&start) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            errors.push(ProjectError::new(
                "inventory_read_failed",
                format!("Cannot inspect inventory root: {error}"),
                Some(&start),
            ));
            return;
        }
    };
    if start_metadata.file_type().is_symlink() {
        return;
    }
    if !start_metadata.is_dir() {
        errors.push(ProjectError::new(
            "inventory_read_failed",
            "Inventory root is not a directory",
            Some(&start),
        ));
        return;
    }
    let mut pending = vec![start];
    while let Some(current) = pending.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push(ProjectError::new(
                    "inventory_read_failed",
                    format!("Cannot read inventory directory: {error}"),
                    Some(&current),
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    errors.push(ProjectError::new(
                        "inventory_read_failed",
                        format!("Cannot read inventory entry: {error}"),
                        Some(&current),
                    ));
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    errors.push(ProjectError::new(
                        "inventory_read_failed",
                        format!("Cannot inspect inventory entry: {error}"),
                        Some(&path),
                    ));
                    continue;
                }
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("md")
            {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let kind = if relative.contains("/evidence/") {
                    "evidence"
                } else if relative.starts_with("tasks/") {
                    "task"
                } else {
                    "document"
                };
                let status = match status_from_markdown(&path) {
                    Ok(status) => status,
                    Err(error) => {
                        errors.push(ProjectError::new(
                            "inventory_read_failed",
                            format!("Cannot read Markdown status: {error}"),
                            Some(&path),
                        ));
                        None
                    }
                };
                files.push(ProjectFile {
                    name: display_name(&path),
                    status,
                    path: relative,
                    kind: kind.to_string(),
                });
            }
        }
    }
}

fn collect_agent_instructions(
    root: &Path,
    files: &mut Vec<ProjectFile>,
    errors: &mut Vec<ProjectError>,
) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push(ProjectError::new(
                    "inventory_read_failed",
                    format!("Cannot read instruction inventory: {error}"),
                    Some(&current),
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    errors.push(ProjectError::new(
                        "inventory_read_failed",
                        format!("Cannot read instruction entry: {error}"),
                        Some(&current),
                    ));
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    errors.push(ProjectError::new(
                        "inventory_read_failed",
                        format!("Cannot inspect instruction entry: {error}"),
                        Some(&path),
                    ));
                    continue;
                }
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let name = entry.file_name();
                if !matches!(
                    name.to_str(),
                    Some(".git" | "node_modules" | "target" | "dist" | "output")
                ) {
                    pending.push(path);
                }
            } else if file_type.is_file() && entry.file_name() == "AGENTS.md" {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let status = match status_from_markdown(&path) {
                    Ok(status) => status,
                    Err(error) => {
                        errors.push(ProjectError::new(
                            "inventory_read_failed",
                            format!("Cannot read agent instructions: {error}"),
                            Some(&path),
                        ));
                        None
                    }
                };
                files.push(ProjectFile {
                    path: relative,
                    name: "Agent instructions".into(),
                    kind: "instruction".into(),
                    status,
                });
            }
        }
    }
}

fn generic_skill_fallback(id: String) -> SkillSummary {
    let installed_path = format!(".agents/skills/{id}/SKILL.md");
    SkillSummary {
        name: display_name(Path::new(&id)),
        id,
        phase: "Unknown".into(),
        purpose: "No validated first-party UI contract is available for this installed skill."
            .into(),
        reads: vec![],
        writes: vec![],
        decisions: vec![],
        helpers: vec![],
        required_evidence: vec![],
        stop_states: vec![],
        renderer: "generic-markdown".into(),
        executable: false,
        source: "unverified installed skill".into(),
        installed_path,
        lock_hash: None,
    }
}

fn first_party_spec(id: &str) -> Option<(&'static str, &'static [&'static str])> {
    match id {
        "build-right-preflight" => Some(("Discover", &["preflight-check"])),
        "build-right-feature-planning" => Some(("Plan", &["feature-planning-check"])),
        "build-right-execution" => Some(("Build", &["continue-check", "execution-check"])),
        "build-right-engineering-principles" => Some(("Principles", &[])),
        _ => None,
    }
}

fn lock_entry(root: &Path, id: &str) -> Result<SkillLockEntry, String> {
    let lock_path = resolve_regular_root_file(root, "skills-lock.json")?;
    let lock_file = fs::read_to_string(lock_path)
        .map_err(|error| format!("Cannot read skills lock: {error}"))?;
    let lock: serde_json::Value = serde_json::from_str(&lock_file)
        .map_err(|error| format!("Cannot parse skills lock: {error}"))?;
    lock.pointer(&format!("/skills/{id}/computedHash"))
        .ok_or_else(|| format!("No lock entry for skill {id}"))?;
    let entry = lock
        .pointer(&format!("/skills/{id}"))
        .cloned()
        .ok_or_else(|| format!("No lock entry for skill {id}"))?;
    let entry: SkillLockEntry = serde_json::from_value(entry)
        .map_err(|error| format!("Invalid lock entry for skill {id}: {error}"))?;
    if entry.source.trim().is_empty() || entry.computed_hash.trim().is_empty() {
        return Err(format!("Invalid lock provenance for skill {id}"));
    }
    Ok(entry)
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_strings(values: &[String]) -> bool {
    values.iter().all(|value| non_empty(value))
}

fn validate_installed_skill(root: &Path, id: &str) -> Result<(), String> {
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("Cannot resolve project root: {error}"))?;
    let skill_directory = root.join(".agents/skills").join(id);
    let directory_metadata = fs::symlink_metadata(&skill_directory)
        .map_err(|error| format!("Cannot inspect installed skill directory: {error}"))?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err("Installed skill directory must be a regular project-scoped directory".into());
    }
    let skill_path = skill_directory.join("SKILL.md");
    let skill_metadata = fs::symlink_metadata(&skill_path)
        .map_err(|error| format!("Cannot inspect installed skill file: {error}"))?;
    if skill_metadata.file_type().is_symlink() || !skill_metadata.is_file() {
        return Err("Installed SKILL.md must be a regular file".into());
    }
    let canonical_skill = fs::canonicalize(skill_path)
        .map_err(|error| format!("Cannot resolve installed skill file: {error}"))?;
    if !canonical_skill.starts_with(canonical_root) {
        return Err("Installed skill resolves outside the selected project root".into());
    }
    Ok(())
}

fn validate_skill_contract(root: &Path, id: &str) -> Result<SkillSummary, String> {
    let (expected_phase, allowed_helpers) = first_party_spec(id)
        .ok_or_else(|| "Unknown first-party skill UI contract identity".to_string())?;
    validate_installed_skill(root, id)?;
    let contract_relative = format!("skill-ui/{id}.json");
    let contract_path = resolve_regular_root_file(root, &contract_relative)?;
    let raw = fs::read_to_string(contract_path)
        .map_err(|error| format!("Cannot read skill UI contract: {error}"))?;
    let contract: SkillUiContract = serde_json::from_str(&raw)
        .map_err(|error| format!("Cannot parse skill UI contract: {error}"))?;
    if contract.version != 1 {
        return Err("Unsupported skill UI contract version".into());
    }
    if contract.id != id {
        return Err("Skill UI contract identity does not match installed skill".into());
    }
    if contract.lifecycle_phase != expected_phase {
        return Err("Skill UI lifecycle phase does not match first-party registry".into());
    }
    if contract.renderer != "operating-card" {
        return Err("First-party skills must use the operating-card renderer".into());
    }
    let expected_path = format!(".agents/skills/{id}/SKILL.md");
    if contract.provenance.installed_path != expected_path {
        return Err("Skill UI provenance does not match installed path".into());
    }
    let lock_entry = lock_entry(root, id)?;
    if contract.provenance.source != lock_entry.source
        || contract.provenance.lock_hash != lock_entry.computed_hash
    {
        return Err("Skill UI provenance does not match skills-lock source and hash".into());
    }
    if !non_empty(&contract.name)
        || !non_empty(&contract.purpose)
        || !valid_strings(&contract.reads)
        || !valid_strings(&contract.writes)
        || !valid_strings(&contract.decisions)
        || !valid_strings(&contract.required_evidence)
        || !valid_strings(&contract.stop_states)
        || !non_empty(&contract.provenance.source)
    {
        return Err("Skill UI contract contains blank semantic values".into());
    }
    if contract.helpers.iter().any(|helper| {
        helper.execution != "explicit-user-action"
            || !non_empty(&helper.id)
            || !allowed_helpers.contains(&helper.id.as_str())
    }) {
        return Err("Skill UI helper is not an allowed explicit action".into());
    }
    Ok(SkillSummary {
        id: contract.id,
        name: contract.name,
        phase: contract.lifecycle_phase,
        purpose: contract.purpose,
        reads: contract.reads,
        writes: contract.writes,
        decisions: contract.decisions,
        helpers: contract
            .helpers
            .into_iter()
            .map(|helper| helper.id)
            .collect(),
        required_evidence: contract.required_evidence,
        stop_states: contract.stop_states,
        renderer: contract.renderer,
        executable: false,
        source: contract.provenance.source,
        installed_path: contract.provenance.installed_path,
        lock_hash: Some(contract.provenance.lock_hash),
    })
}

#[cfg(test)]
fn collect_skills(root: &Path) -> Vec<SkillSummary> {
    collect_skills_with_errors(root).0
}

fn collect_skills_with_errors(root: &Path) -> (Vec<SkillSummary>, Vec<ProjectError>) {
    collect_skills_with_errors_using(root, |entry| entry.file_type())
}

fn collect_skills_with_errors_using<F>(
    root: &Path,
    inspect_entry: F,
) -> (Vec<SkillSummary>, Vec<ProjectError>)
where
    F: Fn(&fs::DirEntry) -> Result<fs::FileType, std::io::Error>,
{
    let skill_root = root.join(".agents/skills");
    let entries = match fs::read_dir(&skill_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return (vec![], vec![]),
        Err(error) => {
            return (
                vec![],
                vec![ProjectError::new(
                    "inventory_read_failed",
                    format!("Cannot read installed skills: {error}"),
                    Some(&skill_root),
                )],
            )
        }
    };
    let mut skills = Vec::new();
    let mut errors = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(ProjectError::new(
                    "inventory_read_failed",
                    format!("Cannot read installed skill entry: {error}"),
                    Some(&skill_root),
                ));
                continue;
            }
        };
        let entry_path = entry.path();
        let file_type = match inspect_entry(&entry) {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(ProjectError::new(
                    "inventory_read_failed",
                    format!("Cannot inspect installed skill entry: {error}"),
                    Some(&entry_path),
                ));
                continue;
            }
        };
        if !file_type.is_dir() {
            continue;
        }
        let skill_marker = entry_path.join("SKILL.md");
        let marker_metadata = match fs::symlink_metadata(&skill_marker) {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(ProjectError::new(
                    "inventory_read_failed",
                    format!("Cannot inspect installed SKILL.md: {error}"),
                    Some(&skill_marker),
                ));
                continue;
            }
        };
        if marker_metadata.file_type().is_symlink() || !marker_metadata.is_file() {
            errors.push(ProjectError::new(
                "inventory_read_failed",
                "Installed SKILL.md must be a non-symlink regular file",
                Some(&skill_marker),
            ));
            continue;
        }
        let id = match entry.file_name().into_string() {
            Ok(id) => id,
            Err(_) => {
                errors.push(ProjectError::new(
                    "inventory_read_failed",
                    "Installed skill directory name is not valid UTF-8",
                    Some(&entry_path),
                ));
                continue;
            }
        };
        match validate_skill_contract(root, &id) {
            Ok(skill) => skills.push(skill),
            Err(message) => {
                errors.push(ProjectError::new(
                    "invalid_skill_provenance",
                    message,
                    Some(&entry.path()),
                ));
                skills.push(generic_skill_fallback(id));
            }
        }
    }
    skills.sort_by(|left, right| left.phase.cmp(&right.phase));
    (skills, errors)
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, ProjectError> {
    repository_service::read_text_with(&NativeGitRead, root, args).map_err(|failure| match failure
        .kind
    {
        GitReadFailureKind::Unavailable => ProjectError::new(
            "git_unavailable",
            format!("Cannot run git: {}", failure.detail),
            Some(root),
        ),
        GitReadFailureKind::Failed => ProjectError::new(
            "git_inspection_failed",
            if failure.detail.is_empty() {
                "Git inspection failed".into()
            } else {
                failure.detail
            },
            Some(root),
        ),
    })
}

fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>, ProjectError> {
    repository_service::read_bytes_with(&NativeGitRead, root, args).map_err(|failure| {
        ProjectError::new(
            match failure.kind {
                GitReadFailureKind::Unavailable => "git_unavailable",
                GitReadFailureKind::Failed => "git_failed",
            },
            failure.detail,
            Some(root),
        )
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn repository_identity(root: &Path) -> Result<RepositoryIdentity, ProjectError> {
    let canonical = fs::canonicalize(root).map_err(|error| {
        ProjectError::new("goal_repository_missing", error.to_string(), Some(root))
    })?;
    let top = git_output(&canonical, &["rev-parse", "--show-toplevel"])?;
    let top = fs::canonicalize(top.trim()).map_err(|error| {
        ProjectError::new(
            "goal_repository_missing",
            error.to_string(),
            Some(&canonical),
        )
    })?;
    if top != canonical {
        return Err(ProjectError::new(
            "goal_repository_root_required",
            "Goal persistence requires the canonical Git worktree root",
            Some(&canonical),
        ));
    }
    let origin =
        git_output(&canonical, &["config", "--get", "remote.origin.url"]).unwrap_or_default();
    let root_commit =
        git_output(&canonical, &["rev-list", "--max-parents=0", "HEAD"]).unwrap_or_default();
    let git_dir = git_output(&canonical, &["rev-parse", "--absolute-git-dir"])?;
    let git_dir_path = fs::canonicalize(git_dir.trim()).map_err(|error| {
        ProjectError::new(
            "goal_repository_identity_failed",
            error.to_string(),
            Some(Path::new(git_dir.trim())),
        )
    })?;
    let mut stable = Sha256::new();
    stable.update(b"pax-workbench-repository-v2\0");
    stable.update(origin.trim().as_bytes());
    stable.update(b"\0");
    stable.update(root_commit.lines().next().unwrap_or_default().as_bytes());
    stable.update(b"\0");
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let worktree = fs::metadata(&canonical).map_err(|error| {
            ProjectError::new(
                "goal_repository_identity_failed",
                error.to_string(),
                Some(&canonical),
            )
        })?;
        let git = fs::metadata(&git_dir_path).map_err(|error| {
            ProjectError::new(
                "goal_repository_identity_failed",
                error.to_string(),
                Some(&git_dir_path),
            )
        })?;
        stable.update(worktree.dev().to_le_bytes());
        stable.update(worktree.ino().to_le_bytes());
        stable.update(git.dev().to_le_bytes());
        stable.update(git.ino().to_le_bytes());
    }
    #[cfg(not(unix))]
    {
        stable.update(canonical.to_string_lossy().as_bytes());
        stable.update(b"\0");
        stable.update(git_dir_path.to_string_lossy().as_bytes());
    }
    Ok(RepositoryIdentity {
        canonical_path: canonical.to_string_lossy().to_string(),
        repository_id: format!("sha256:{:x}", stable.finalize()),
    })
}

pub(crate) fn git_fingerprint(root: &Path) -> Result<GitFingerprint, ProjectError> {
    let head = git_output(root, &["rev-parse", "HEAD"]).unwrap_or_else(|_| "unborn".into());
    let index_bytes = git_bytes(root, &["diff", "--cached", "--binary", "--no-ext-diff"])?;
    let worktree_diff = git_bytes(root, &["diff", "--binary", "--no-ext-diff"])?;
    let paths = git_bytes(
        root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let entries = paths
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    if entries.len() > GOAL_FINGERPRINT_MAX_FILES {
        return Err(ProjectError::new(
            "goal_fingerprint_oversized",
            "Repository contains too many files for the bounded recovery fingerprint",
            Some(root),
        ));
    }
    let mut total = 0_u64;
    let mut worktree = Sha256::new();
    worktree.update(&worktree_diff);
    worktree.update(b"\0");
    for raw_path in entries {
        let relative = std::str::from_utf8(raw_path).map_err(|_| {
            ProjectError::new(
                "goal_fingerprint_invalid_path",
                "Git returned a non-UTF-8 path",
                Some(root),
            )
        })?;
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ProjectError::new(
                "goal_fingerprint_invalid_path",
                "Git returned a path outside the repository",
                Some(root),
            ));
        }
        let path = root.join(relative);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                worktree.update(b"[missing]\0");
                continue;
            }
            Err(error) => {
                return Err(ProjectError::new(
                    "goal_fingerprint_failed",
                    error.to_string(),
                    Some(&path),
                ));
            }
        };
        worktree.update(raw_path);
        worktree.update(b"\0");
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path).map_err(|error| {
                ProjectError::new("goal_fingerprint_failed", error.to_string(), Some(&path))
            })?;
            worktree.update(target.to_string_lossy().as_bytes());
        } else if metadata.is_file() {
            total = total.saturating_add(metadata.len());
            if total > GOAL_FINGERPRINT_MAX_BYTES {
                return Err(ProjectError::new(
                    "goal_fingerprint_oversized",
                    "Repository contents exceed the bounded recovery fingerprint limit",
                    Some(root),
                ));
            }
            let mut file = File::open(&path).map_err(|error| {
                ProjectError::new("goal_fingerprint_failed", error.to_string(), Some(&path))
            })?;
            let mut buffer = [0_u8; 32 * 1024];
            loop {
                let count = file.read(&mut buffer).map_err(|error| {
                    ProjectError::new("goal_fingerprint_failed", error.to_string(), Some(&path))
                })?;
                if count == 0 {
                    break;
                }
                worktree.update(&buffer[..count]);
            }
        }
        worktree.update(b"\0");
    }
    Ok(GitFingerprint {
        head: head.trim().to_string(),
        index: sha256_bytes(&index_bytes),
        worktree: format!("sha256:{:x}", worktree.finalize()),
    })
}

fn local_collaboration_binding(
    root: &Path,
    task_path: &str,
    task_text: &str,
) -> Result<LocalSourceBinding, ProjectError> {
    let repository = repository_identity(root)?;
    let git = git_fingerprint(root)?;
    Ok(LocalSourceBinding {
        task_path: task_path.into(),
        task_sha256: sha256_bytes(task_text.as_bytes()),
        repository_id: repository.repository_id,
        git_head: (git.head != "unborn").then_some(git.head),
        git_index_sha256: git.index,
        git_worktree_sha256: git.worktree,
        git_dirty: inspect_project_path(root).dirty,
    })
}

fn collaboration_project_error(
    root: &Path,
    surface: &str,
    failure: CollaborationFailure,
) -> ProjectError {
    let repair = failure
        .repair()
        .map(|repair| format!("; repair {}: {}", repair.code(), repair.next_action()))
        .unwrap_or_default();
    ProjectError::new(
        &format!("collaboration_{surface}_{}", failure.code()),
        format!("{:?}: {}{repair}", failure.class(), failure.message()),
        Some(root),
    )
}

fn collaboration_failure_for_class(class: CollaborationFailureClass) -> CollaborationFailure {
    match class {
        CollaborationFailureClass::InvalidInput | CollaborationFailureClass::Protocol => {
            CollaborationFailure::protocol()
        }
        CollaborationFailureClass::AccessDenied => CollaborationFailure::access_denied(),
        CollaborationFailureClass::SourceMismatch => CollaborationFailure::source_mismatch(),
        CollaborationFailureClass::VersionConflict => CollaborationFailure::version_conflict(),
        CollaborationFailureClass::TransportUnavailable => {
            CollaborationFailure::transport_unavailable()
        }
        CollaborationFailureClass::Timeout => CollaborationFailure::timeout(),
        CollaborationFailureClass::Cancelled => CollaborationFailure::cancelled(),
        CollaborationFailureClass::RepairRequired => CollaborationFailure::repair_required(),
    }
}

fn run_pre_run_collaboration_hook(
    port: &dyn CollaborationPort,
    policy: &ControllerCollaborationPolicy,
    root: &Path,
    task_path: &str,
    task_text: &str,
    cancel: &AtomicBool,
) -> Result<PreRunCollaborationOutcome, ProjectError> {
    let context = PreRunCollaborationContext {
        mode: policy.mode,
        session: policy.session.clone(),
        local: local_collaboration_binding(root, task_path, task_text)?,
        remote: policy.remote.clone(),
    };
    let outcome = run_before_runtime(port, &context, cancel)
        .map_err(|failure| collaboration_project_error(root, "pre_run", failure))?;
    match (&policy.mode, &outcome.claim) {
        (CollaborationMode::Disabled | CollaborationMode::LocalOnly, ClaimResult::NotRequired)
        | (CollaborationMode::SharedCollaborator, ClaimResult::Claimed { .. }) => Ok(outcome),
        (_, ClaimResult::Stopped { failure_class, .. }) => Err(collaboration_project_error(
            root,
            "pre_run",
            collaboration_failure_for_class(*failure_class),
        )),
        _ => Err(collaboration_project_error(
            root,
            "pre_run",
            CollaborationFailure::protocol(),
        )),
    }
}

fn run_post_commit_collaboration_hook(
    port: &dyn CollaborationPort,
    policy: &ControllerCollaborationPolicy,
    root: &Path,
    task: &ProjectFileContent,
    run_id: &str,
    intent: Option<RemoteCompletionIntent>,
) -> Result<PostLocalCommitCollaborationOutcome, ProjectError> {
    let context = PostLocalCommitCollaborationContext {
        mode: policy.mode,
        session: policy.session.clone(),
        local: local_collaboration_binding(root, &task.path, &task.content)?,
        remote: policy.remote.clone(),
        run_id: run_id.into(),
        intent,
    };
    run_after_local_commit(port, &context)
        .map_err(|failure| collaboration_project_error(root, "post_commit", failure))
}

fn deterministic_collaboration_reference(namespace: &str, material: &str) -> String {
    let digest = sha256_bytes(format!("{namespace}\0{material}").as_bytes());
    digest
        .strip_prefix("sha256:")
        .expect("sha256_bytes always prefixes its digest")
        .chars()
        .take(32)
        .collect()
}

fn build_remote_completion_intent(
    binding: &SharedExecutionBinding,
    completed_local: &LocalSourceBinding,
    claimed_task_version: u64,
    run_id: &str,
    created_at_unix_seconds: u64,
    artifacts: &[GoalEvidenceReference],
) -> Result<RemoteCompletionIntent, ProjectError> {
    if completed_local.task_path != binding.local.task_path
        || completed_local.repository_id != binding.local.repository_id
        || claimed_task_version
            != binding.remote.base_version.checked_add(1).ok_or_else(|| {
                ProjectError::new(
                    "collaboration_version_exhausted",
                    "Claimed remote task version cannot advance",
                    None,
                )
            })?
    {
        return Err(ProjectError::new(
            "collaboration_completion_binding_mismatch",
            "Repository checkpoint does not match the exact claimed shared task",
            None,
        ));
    }
    let material = format!(
        "{}\0{}\0{}\0{}\0{}",
        binding.session.workspace_id,
        binding.remote.task_path,
        completed_local.task_sha256,
        run_id,
        claimed_task_version
    );
    let evidence_suffix =
        deterministic_collaboration_reference("completion-evidence-v1", &material);
    let handoff_suffix = deterministic_collaboration_reference("completion-handoff-v1", &material);
    let evidence_id = format!("evidence-{evidence_suffix}");
    let handoff_id = format!("handoff-{handoff_suffix}");
    let evidence_path = format!(
        "evidence/{}/completion-{}.md",
        binding.remote.task_id, evidence_suffix
    );
    let handoff_path = format!(
        "logs/{}-handoff-{}.md",
        binding.remote.task_id, handoff_suffix
    );
    let artifacts = artifacts
        .iter()
        .map(|artifact| CompletionArtifact {
            path: artifact.path.clone(),
            sha256: artifact.sha256.clone(),
        })
        .collect();
    RemoteCompletionIntent::new(
        binding.session.workspace_id.clone(),
        binding.session.actor.clone(),
        binding.remote.task_id.clone(),
        binding.remote.task_path.clone(),
        claimed_task_version,
        binding.local.task_sha256.clone(),
        completed_local.task_path.clone(),
        completed_local.task_sha256.clone(),
        completed_local.repository_id.clone(),
        run_id.into(),
        created_at_unix_seconds,
        evidence_id,
        evidence_path,
        handoff_id,
        handoff_path,
        artifacts,
    )
    .map_err(|failure| {
        collaboration_project_error(Path::new("shared-completion"), "intent", failure)
    })
}

fn repository_affirms_goal_completion(
    root: &Path,
    record: &PersistedGoalRecord,
    checkpoint: &VerifiedGoalCheckpoint,
) -> bool {
    let task = match read_controller_task(root, &checkpoint.task_path) {
        Ok(task) if task_has_repository_verification(&task.content) => task,
        _ => return false,
    };
    if sha256_bytes(task.content.as_bytes()) != checkpoint.task_sha256 {
        return false;
    }
    record.evidence_references.iter().any(|reference| {
        if !reference.path.starts_with("tasks/sprint") || !reference.path.ends_with(".md") {
            return false;
        }
        let tracker = match read_controller_task(root, &reference.path) {
            Ok(tracker) if sha256_bytes(tracker.content.as_bytes()) == reference.sha256 => tracker,
            _ => return false,
        };
        tracker_affirms_goal_completion(root, &tracker.content, &checkpoint.task_path)
    })
}

fn tracker_affirms_goal_completion(root: &Path, tracker: &str, checkpoint_task: &str) -> bool {
    if !markdown_field(tracker, "Status:")
        .is_some_and(|status| status.eq_ignore_ascii_case("complete"))
    {
        return false;
    }
    let task_section = markdown_section(tracker, "Tasks");
    let rows = task_section
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let Some(header) = rows.first() else {
        return false;
    };
    let Some(status_index) = header
        .iter()
        .position(|cell| cell.eq_ignore_ascii_case("status"))
    else {
        return false;
    };
    let Some(evidence_index) = header
        .iter()
        .position(|cell| cell.eq_ignore_ascii_case("evidence"))
    else {
        return false;
    };
    let terminal = [
        "complete",
        "deferred",
        "moved",
        "canceled",
        "split",
        "superseded",
    ];
    let mut saw_checkpoint = false;
    let mut saw_task = false;
    for cells in rows.into_iter().skip(1) {
        if cells.iter().all(|cell| {
            !cell.is_empty()
                && cell
                    .chars()
                    .all(|character| character == '-' || character == ':')
        }) {
            continue;
        }
        if cells.len() <= status_index || cells.len() <= evidence_index {
            return false;
        }
        saw_task = true;
        let status = cells[status_index];
        if !terminal.contains(&status) {
            return false;
        }
        if status == "complete" {
            let Some(task_path) = normalized_tracker_evidence_path(cells[evidence_index]) else {
                return false;
            };
            let Ok(task) = read_controller_task(root, task_path) else {
                return false;
            };
            if !task_has_repository_verification(&task.content) {
                return false;
            }
            saw_checkpoint |= task_path == checkpoint_task;
        }
    }
    saw_task && saw_checkpoint
}

fn normalized_tracker_evidence_path(cell: &str) -> Option<&str> {
    let cell = cell.trim();
    let path = if let Some(path) = cell
        .strip_prefix('`')
        .and_then(|path| path.strip_suffix('`'))
    {
        path
    } else if cell.contains('`') {
        return None;
    } else {
        cell
    };
    if path.is_empty()
        || path.chars().any(char::is_whitespace)
        || path.contains(';')
        || path.contains(',')
        || !path.starts_with("tasks/issues/")
        || !path.ends_with(".md")
        || validate_markdown_path(Path::new(path)).is_err()
    {
        return None;
    }
    Some(path)
}

fn terminal_tracker_reference(
    root: &Path,
    project: &ProjectSnapshot,
    checkpoint_task: &str,
) -> Option<GoalEvidenceReference> {
    project.files.iter().find_map(|file| {
        if !file.path.starts_with("tasks/sprint-") || !file.path.ends_with(".md") {
            return None;
        }
        let tracker = read_controller_task(root, &file.path).ok()?;
        tracker_affirms_goal_completion(root, &tracker.content, checkpoint_task).then(|| {
            GoalEvidenceReference {
                path: tracker.path,
                sha256: sha256_bytes(tracker.content.as_bytes()),
            }
        })
    })
}

fn goal_recovery(
    storage: &GoalStateStorage,
    selected_root: &Path,
) -> Result<GoalRecovery, ProjectError> {
    let bytes = match storage.read_bytes() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Ok(goal_recovery_empty(
                GoalRecoveryState::Missing,
                "No persisted goal exists",
            ))
        }
        Err(error) if error.code == "goal_state_oversized" => {
            return Ok(goal_recovery_empty(
                GoalRecoveryState::Oversized,
                &error.message,
            ))
        }
        Err(error) => return Err(error),
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return Ok(goal_recovery_empty(
                GoalRecoveryState::Corrupt,
                "Persisted goal JSON is corrupt",
            ))
        }
    };
    if value.get("version").and_then(|version| version.as_u64()) != Some(GOAL_STATE_VERSION as u64)
    {
        return Ok(goal_recovery_empty(
            GoalRecoveryState::Incompatible,
            "Persisted goal schema version is incompatible",
        ));
    }
    let record: PersistedGoalRecord = match serde_json::from_value::<PersistedGoalRecord>(value) {
        Ok(record)
            if record.evidence_references.len() <= GOAL_EVIDENCE_MAX
                && record.stop_conditions == goal_loop_stop_conditions()
                && storage.validate_collaboration_cursor(&record).is_ok() =>
        {
            record
        }
        _ => {
            return Ok(goal_recovery_empty(
                GoalRecoveryState::Incompatible,
                "Persisted goal schema is incompatible",
            ))
        }
    };
    let projected = |state, reason: &str| GoalRecovery {
        state,
        objective: Some(record.objective.clone()),
        repository: Some(record.repository.clone()),
        run_id: Some(record.current_run.run_id.clone()),
        event_cursor: Some(record.current_run.event_cursor),
        checkpoint_task: record
            .last_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.task_path.clone()),
        evidence_references: record.evidence_references.clone(),
        collaboration: record.collaboration.clone(),
        stop_conditions: record.stop_conditions.clone(),
        reason: reason.into(),
        explicit_confirmation_required: state == GoalRecoveryState::Resumable,
        automatic_execution_started: false,
    };
    let persisted_root = Path::new(&record.repository.canonical_path);
    let selected = repository_identity(selected_root)?;
    if selected.canonical_path == record.repository.canonical_path {
        if selected.repository_id != record.repository.repository_id {
            return Ok(projected(
                GoalRecoveryState::ReplacedRepository,
                "The canonical path now contains a different repository filesystem identity",
            ));
        }
    } else if selected.repository_id == record.repository.repository_id {
        return Ok(projected(
            GoalRecoveryState::MovedRepository,
            "The repository identity matches at a different canonical path",
        ));
    } else {
        if !persisted_root.exists() {
            return Ok(projected(
                GoalRecoveryState::MissingRepository,
                "The persisted canonical repository path no longer exists",
            ));
        }
        return Ok(projected(
            GoalRecoveryState::MissingRepository,
            "The selected repository does not match the persisted canonical repository",
        ));
    }
    if record.current_run.nonterminal {
        return Ok(projected(
            GoalRecoveryState::Interrupted,
            "The prior process did not reach a verified checkpoint",
        ));
    }
    if let Some(checkpoint) = &record.last_checkpoint {
        let task = match read_controller_task(selected_root, &checkpoint.task_path) {
            Ok(task) => task,
            Err(_) => {
                return Ok(projected(
                    GoalRecoveryState::StaleTask,
                    "The checkpoint task is missing or no longer trusted",
                ))
            }
        };
        if sha256_bytes(task.content.as_bytes()) != checkpoint.task_sha256 {
            return Ok(projected(
                GoalRecoveryState::StaleTask,
                "Repository task truth changed after the checkpoint",
            ));
        }
        if git_fingerprint(selected_root)? != checkpoint.git {
            return Ok(projected(
                GoalRecoveryState::GitChanged,
                "HEAD, index, or worktree changed after the checkpoint",
            ));
        }
        if repository_affirms_goal_completion(selected_root, &record, checkpoint) {
            return Ok(projected(
                GoalRecoveryState::Completed,
                "Current repository task and terminal sprint truth affirm goal completion",
            ));
        }
    }
    Ok(projected(
        GoalRecoveryState::Resumable,
        "Repository identity, task truth, and Git fingerprint match the verified checkpoint",
    ))
}

fn goal_recovery_empty(state: GoalRecoveryState, reason: &str) -> GoalRecovery {
    GoalRecovery {
        state,
        objective: None,
        repository: None,
        run_id: None,
        event_cursor: None,
        checkpoint_task: None,
        evidence_references: Vec::new(),
        collaboration: None,
        stop_conditions: Vec::new(),
        reason: reason.into(),
        explicit_confirmation_required: false,
        automatic_execution_started: false,
    }
}

fn app_goal_storage(app: &tauri::AppHandle) -> Result<GoalStateStorage, ProjectError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| ProjectError::new("goal_storage_unavailable", error.to_string(), None))?;
    Ok(GoalStateStorage::new(directory))
}

fn ensure_shared_iteration_has_no_repair_debt(
    storage: &GoalStateStorage,
    root: &Path,
) -> Result<(), ProjectError> {
    let Some(record) = storage.read_record()? else {
        return Ok(());
    };
    let repository = repository_identity(root)?;
    if record.repository.canonical_path != repository.canonical_path
        || record.repository.repository_id != repository.repository_id
    {
        return Ok(());
    }
    if record
        .collaboration
        .as_ref()
        .is_some_and(|cursor| cursor.state != PersistedReconciliationState::Reconciled)
    {
        return Err(ProjectError::new(
            "collaboration_repair_required",
            "A prior repository-verified task has durable collaboration repair debt; reconnect as Collaborator and explicitly repair it before another shared iteration",
            Some(root),
        ));
    }
    Ok(())
}

#[tauri::command]
fn recover_goal_state(app: tauri::AppHandle, root: String) -> Result<GoalRecovery, ProjectError> {
    goal_recovery(&app_goal_storage(&app)?, Path::new(&root))
}

#[tauri::command]
fn clear_goal_state(app: tauri::AppHandle) -> Result<(), ProjectError> {
    app_goal_storage(&app)?.clear()
}

pub(crate) fn inspect_project_path(root: &Path) -> ProjectSnapshot {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    collect_agent_instructions(root, &mut files, &mut errors);
    collect_markdown(root, "docs", &mut files, &mut errors);
    collect_markdown(root, "tasks", &mut files, &mut errors);
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let (skills, skill_errors) = collect_skills_with_errors(root);
    errors.extend(skill_errors);
    let branch = match git_output(root, &["branch", "--show-current"]) {
        Ok(value) if value.is_empty() => "detached".into(),
        Ok(value) => value,
        Err(error) => {
            errors.push(error);
            "unavailable".into()
        }
    };
    let dirty = match git_output(root, &["status", "--porcelain"]) {
        Ok(value) => !value.is_empty(),
        Err(error) => {
            errors.push(error);
            false
        }
    };
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("project")
        .to_string();

    ProjectSnapshot {
        root: root.to_string_lossy().to_string(),
        name,
        branch,
        dirty,
        files,
        skills,
        errors,
    }
}

#[tauri::command]
fn inspect_project(root: String) -> Result<ProjectSnapshot, ProjectError> {
    let root = validated_repository_root(&root)?;
    Ok(inspect_project_path(&root))
}

#[tauri::command]
fn inspect_post_run_review(root: String) -> Result<PostRunReviewEvidence, ProjectError> {
    let root = validated_repository_root(&root)?;
    inspect_post_run_review_with(&NativeGitRead, &root).map_err(|failure| match failure {
        ReviewEvidenceFailure::Git(error) => ProjectError::new(
            match error.kind {
                GitReadFailureKind::Unavailable => "review_git_unavailable",
                GitReadFailureKind::Failed => "review_git_failed",
            },
            error.detail,
            Some(&root),
        ),
        ReviewEvidenceFailure::InvalidStatus(message) => {
            ProjectError::new("review_status_invalid", message, Some(&root))
        }
        ReviewEvidenceFailure::Filesystem(message) => {
            ProjectError::new("review_filesystem_failed", message, Some(&root))
        }
    })
}

#[tauri::command(rename = "inspect_project")]
fn inspect_project_command(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
) -> Result<ProjectSnapshot, ProjectError> {
    let root = validated_repository_root(&root)?;
    sessions
        .activate_project(&root.to_string_lossy())
        .map_err(|_| {
            ProjectError::new(
                "collaboration_project_busy",
                "Cannot switch projects while a remote mutation is in flight",
                Some(&root),
            )
        })?;
    Ok(inspect_project_path(&root))
}

fn mdsync_project_key(root: &str) -> Result<String, MdsyncTransportError> {
    validated_repository_root(root)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|_| MdsyncTransportError::invalid_project())
}

#[tauri::command]
fn connect_mdsync_session(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    workspace_url: String,
    actor: String,
) -> Result<collaboration::SanitizedSessionMetadata, MdsyncTransportError> {
    let workspace_url = zeroize::Zeroizing::new(workspace_url);
    if workspace_url.len() > MAX_WORKSPACE_URL_BYTES {
        return Err(MdsyncTransportError::workspace_url_too_large());
    }
    sessions.connect(mdsync_project_key(&root)?, workspace_url, actor)
}

#[tauri::command]
fn disconnect_mdsync_session(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
) -> Result<(), MdsyncTransportError> {
    sessions.disconnect(&mdsync_project_key(&root)?, &session_id)
}

#[tauri::command]
fn list_mdsync_files(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
) -> Result<MdsyncFileListing, MdsyncTransportError> {
    sessions.list_files(&mdsync_project_key(&root)?, &session_id)
}

#[tauri::command]
fn read_mdsync_file(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
    path: String,
) -> Result<MdsyncFile, MdsyncTransportError> {
    sessions.read_file(&mdsync_project_key(&root)?, &session_id, &path)
}

#[tauri::command]
fn write_mdsync_file(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
    input: MdsyncWriteInput,
) -> Result<MdsyncWriteResult, MdsyncTransportError> {
    sessions.write_file(&mdsync_project_key(&root)?, &session_id, input)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "surface", content = "error")]
enum Ha2haEnvelopeCommandError {
    Project(ProjectError),
    Transport(MdsyncTransportError),
    Envelope(EnvelopeError),
}

impl From<ProjectError> for Ha2haEnvelopeCommandError {
    fn from(value: ProjectError) -> Self {
        Self::Project(value)
    }
}

impl From<MdsyncTransportError> for Ha2haEnvelopeCommandError {
    fn from(value: MdsyncTransportError) -> Self {
        Self::Transport(value)
    }
}

impl From<EnvelopeError> for Ha2haEnvelopeCommandError {
    fn from(value: EnvelopeError) -> Self {
        Self::Envelope(value)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ha2haPublishedFile {
    path: String,
    version: u64,
    recovered_from_readback: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ha2haPublishResult {
    workspace_id: String,
    task_path: String,
    complete: bool,
    writes: Vec<Ha2haPublishedFile>,
    failure: Option<MdsyncTransportError>,
    repair: Option<EnvelopeRepair>,
}

fn apply_workspace_write_sequence<E>(
    files: &[WorkspaceFile],
    mut write: impl FnMut(&WorkspaceFile) -> Result<(String, u64, bool), E>,
) -> (Vec<Ha2haPublishedFile>, Option<E>) {
    let mut completed = Vec::with_capacity(files.len());
    for file in files {
        match write(file) {
            Ok((path, version, recovered_from_readback)) => completed.push(Ha2haPublishedFile {
                path,
                version,
                recovered_from_readback,
            }),
            Err(error) => return (completed, Some(error)),
        }
    }
    (completed, None)
}

fn task_envelope_id(path: &str) -> Result<String, EnvelopeError> {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| EnvelopeError::invalid_input("Selected task path has no portable id"))?;
    let numeric = stem
        .split('-')
        .next()
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| {
            EnvelopeError::invalid_input("Selected task filename must begin with its numeric id")
        })?;
    Ok(format!("BR-{numeric}"))
}

fn task_markdown_title(task: &str) -> Result<String, EnvelopeError> {
    task.lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .ok_or_else(|| EnvelopeError::invalid_input("Selected task must have a Markdown title"))
}

fn task_requirement_basis(task: &str) -> Result<Vec<String>, EnvelopeError> {
    let value = markdown_field(task, "Requirement basis:")
        .ok_or_else(|| EnvelopeError::invalid_input("Selected task has no requirement basis"))?;
    let basis = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if basis.is_empty() {
        return Err(EnvelopeError::invalid_input(
            "Selected task has no requirement basis",
        ));
    }
    Ok(basis)
}

fn current_ha2ha_projection_input(
    root: &Path,
    workspace_id: String,
    actor: String,
) -> Result<ProjectionInput, Ha2haEnvelopeCommandError> {
    // Use the same resolver and task-contract helpers as bounded execution,
    // while accepting the resolver's two executable states. This operation
    // intentionally never creates a runtime invocation.
    let cancel = AtomicBool::new(false);
    let resolver = controller_helper(
        root,
        HelperInvocation {
            helper_id: HelperId::ContinueCheck,
            mode: None,
            task_path: None,
            feature_request: None,
        },
        &cancel,
    )?;
    if !resolver.success {
        return Err(EnvelopeError::invalid_input(
            "Build Right resolver did not complete successfully",
        )
        .into());
    }
    let value: serde_json::Value = serde_json::from_str(&resolver.stdout)
        .map_err(|_| EnvelopeError::invalid_input("Build Right resolver output is malformed"))?;
    let decision = json_string(&value, "decision")
        .map_err(|_| EnvelopeError::invalid_input("Resolver decision is missing"))?;
    if !matches!(decision.as_str(), "execute-task" | "continue-active-task")
        || value
            .get("blockingGates")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|gates| !gates.is_empty())
    {
        return Err(EnvelopeError::invalid_input(
            "Only one resolver-selected ready or active task can be projected",
        )
        .into());
    }
    let selected_value = value
        .get("nextTask")
        .filter(|task| !task.is_null())
        .ok_or_else(|| EnvelopeError::invalid_input("Resolver selected no task"))?;
    let selected = json_string(selected_value, "path")
        .map_err(|_| EnvelopeError::invalid_input("Resolver task path is missing"))?;
    let resolver_status = json_string(selected_value, "status")
        .map_err(|_| EnvelopeError::invalid_input("Resolver task status is missing"))?;
    let owner = json_string(selected_value, "owner")
        .map_err(|_| EnvelopeError::invalid_input("Resolver task owner is missing"))?;
    let missing = json_string_array(
        selected_value.get("missingContractFields"),
        "missingContractFields",
    )
    .map_err(|_| EnvelopeError::invalid_input("Resolver task contract is malformed"))?;
    if !matches!(resolver_status.as_str(), "ready" | "active")
        || !owner.eq_ignore_ascii_case("ai")
        || !missing.is_empty()
    {
        return Err(EnvelopeError::invalid_input(
            "Resolver task must be ready or active, AI-owned, and contract-complete",
        )
        .into());
    }
    let contract = controller_helper(
        root,
        HelperInvocation {
            helper_id: HelperId::ExecutionCheck,
            mode: Some(HelperExecutionMode::TaskContract),
            task_path: Some(selected.clone()),
            feature_request: None,
        },
        &cancel,
    )?;
    if !contract.success
        || contract
            .decision
            .as_ref()
            .is_none_or(|decision| decision.decision != "proceed" || !decision.warnings.is_empty())
    {
        return Err(
            EnvelopeError::invalid_input("Resolver-selected task contract did not pass").into(),
        );
    }
    let task = read_controller_task(root, &selected)?;
    let status = markdown_field(&task.content, "Status:")
        .ok_or_else(|| EnvelopeError::invalid_input("Selected task status is missing"))?;
    if status != resolver_status {
        return Err(EnvelopeError::source_mismatch().into());
    }
    let status = match resolver_status.as_str() {
        "ready" => ResolverTaskStatus::Ready,
        "active" => ResolverTaskStatus::Active,
        _ => {
            return Err(EnvelopeError::invalid_input(
                "Only a resolver-selected ready or active task can be projected",
            )
            .into())
        }
    };
    Ok(ProjectionInput {
        workspace_id,
        actor,
        task_id: task_envelope_id(&selected)?,
        title: task_markdown_title(&task.content)?,
        status,
        requirement_basis: task_requirement_basis(&task.content)?,
        local: local_collaboration_binding(root, &selected, &task.content)?,
    })
}

fn read_remote_workspace_files(
    sessions: &MdsyncSessionStore,
    project_key: &str,
    session_id: &str,
) -> Result<Vec<RemoteWorkspaceFile>, Ha2haEnvelopeCommandError> {
    let paths = sessions
        .list_files(project_key, session_id)?
        .paths()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if paths.len() > 64 {
        return Err(EnvelopeError::invalid_remote_shape(
            "Remote workspace contains too many files for bounded inspection",
        )
        .into());
    }
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let file = sessions.read_file(project_key, session_id, &path)?;
        files.push(RemoteWorkspaceFile {
            path: file.path().into(),
            content: file.content().into(),
            version: file.version(),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

#[tauri::command]
fn preview_ha2ha_publish(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    plans: tauri::State<'_, PublishPlanStore>,
    root: String,
    session_id: String,
) -> Result<PublishPreview, Ha2haEnvelopeCommandError> {
    let root = validated_repository_root(&root)?;
    let project_key = root.to_string_lossy().to_string();
    let context = sessions.session_context(&project_key, &session_id)?;
    if context.access != collaboration::CollaborationAccess::Collaborator {
        return Err(EnvelopeError::access_denied().into());
    }
    let input = current_ha2ha_projection_input(&root, context.workspace_id, context.actor)?;
    let baseline = read_remote_workspace_files(&sessions, &project_key, &session_id)?;
    let plan = project_publish_plan(input, baseline)?;
    Ok(plans.issue(&project_key, &session_id, plan)?)
}

#[tauri::command]
fn apply_ha2ha_publish(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    plans: tauri::State<'_, PublishPlanStore>,
    root: String,
    session_id: String,
    preview_token: String,
    confirmed: bool,
) -> Result<Ha2haPublishResult, Ha2haEnvelopeCommandError> {
    let root = validated_repository_root(&root)?;
    let project_key = root.to_string_lossy().to_string();
    // Consume before any remote effect. A failed or cancelled apply cannot be
    // replayed and always requires a fresh authority-bound preview.
    let confirmed_plan = plans.consume(&project_key, &session_id, &preview_token, confirmed)?;
    let workspace = &confirmed_plan.workspace;
    let context = sessions.session_context(&project_key, &session_id)?;
    if context.access != collaboration::CollaborationAccess::Collaborator
        || context.workspace_id != workspace.workspace_id
    {
        return Err(EnvelopeError::access_denied().into());
    }
    let fresh_input = current_ha2ha_projection_input(&root, context.workspace_id, context.actor)?;
    let fresh_baseline = read_remote_workspace_files(&sessions, &project_key, &session_id)?;
    if fresh_baseline != confirmed_plan.remote_baseline {
        return Err(EnvelopeError::source_mismatch().into());
    }
    let fresh_plan = project_publish_plan(fresh_input, fresh_baseline)?;
    if fresh_plan.workspace != *workspace || fresh_plan.writes != confirmed_plan.writes {
        return Err(EnvelopeError::source_mismatch().into());
    }

    let (writes, failure) = apply_workspace_write_sequence(&confirmed_plan.writes, |file| {
        sessions
            .write_file_with_readback(
                &project_key,
                &session_id,
                MdsyncWriteInput {
                    path: file.path.clone(),
                    content: file.content.clone(),
                    content_type: Some(file.content_type.clone()),
                    base_version: None,
                },
                1,
            )
            .map(|result| {
                (
                    result.path().to_string(),
                    result.version(),
                    result.recovered_from_readback(),
                )
            })
    });
    if let Some(failure) = failure {
        return Ok(Ha2haPublishResult {
            workspace_id: workspace.workspace_id.clone(),
            task_path: workspace.task_path.clone(),
            complete: false,
            writes,
            failure: Some(failure),
            repair: Some(EnvelopeRepair::partial_publish()),
        });
    }
    Ok(Ha2haPublishResult {
        workspace_id: workspace.workspace_id.clone(),
        task_path: workspace.task_path.clone(),
        complete: true,
        writes,
        failure: None,
        repair: None,
    })
}

#[tauri::command]
fn join_ha2ha_workspace(
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
) -> Result<JoinResult, Ha2haEnvelopeCommandError> {
    let root = validated_repository_root(&root)?;
    let project_key = root.to_string_lossy().to_string();
    let context = sessions.session_context(&project_key, &session_id)?;
    let local_projection =
        current_ha2ha_projection_input(&root, context.workspace_id.clone(), context.actor.clone())?;
    let files = read_remote_workspace_files(&sessions, &project_key, &session_id)?;
    Ok(join_workspace(
        &context.workspace_id,
        &context.actor,
        context.access,
        local_projection.local,
        files,
    )?)
}

fn shared_execution_stop_conditions() -> Vec<String> {
    [
        "Local resolver, task contract, source hash, or Git binding changes",
        "Remote workspace manifest or execution envelope no longer validates",
        "Viewer or public access cannot mutate or execute",
        "Remote task path, owner, state, actor, or exact baseVersion changes",
        "Remote conflict, denial, timeout, cancellation, or unavailability",
        "A repeated conflict requires human inspection and never retries automatically",
        "A committed claim with failed final validation requires explicit reconciliation",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn shared_execution_binding(
    context: SanitizedSessionMetadata,
    join: JoinResult,
) -> Result<SharedExecutionBinding, ProjectError> {
    if !join.reconciled
        || join.workspace_id != context.workspace_id
        || join.actor != context.actor
        || join.access != context.access
    {
        return Err(ProjectError::new(
            "shared_preview_reconciliation_mismatch",
            "Remote join result does not match the sanitized native session",
            None,
        ));
    }
    SharedExecutionBinding::new(context, join.local, join.task).map_err(|failure| {
        collaboration_project_error(Path::new("shared-execution"), "preview", failure)
    })
}

fn build_shared_bounded_task_preview_with<FH, FJ>(
    root: &Path,
    session: SanitizedSessionMetadata,
    cancel: &AtomicBool,
    registry: &OperationRegistry,
    mut run_helper: FH,
    join_remote: FJ,
) -> Result<SharedBoundedTaskPreview, ProjectError>
where
    FH: FnMut(&Path, HelperInvocation, &AtomicBool) -> Result<HelperResult, ProjectError>,
    FJ: FnOnce(LocalSourceBinding) -> Result<JoinResult, Ha2haEnvelopeCommandError>,
{
    registry.invalidate_bounded_task_confirmation(root)?;
    let mut bounded = build_bounded_task_preview_with(root, cancel, &mut run_helper)?;
    let selected_task = bounded.selected_task.clone().ok_or_else(|| {
        ProjectError::new(
            "shared_preview_local_gate",
            "Shared execution requires one resolver-selected executable local task",
            Some(root),
        )
    })?;
    if !bounded.executable {
        return Err(ProjectError::new(
            "shared_preview_local_gate",
            "Shared execution cannot bypass a local resolver stop",
            Some(root),
        ));
    }
    let task = read_controller_task(root, &selected_task)?;
    let local = local_collaboration_binding(root, &selected_task, &task.content)?;
    let join = join_remote(local).map_err(|error| match error {
        Ha2haEnvelopeCommandError::Project(error) => error,
        Ha2haEnvelopeCommandError::Transport(_) => ProjectError::new(
            "shared_preview_transport_failed",
            "The remote workspace could not be read for a bounded shared preview",
            Some(root),
        ),
        Ha2haEnvelopeCommandError::Envelope(error) => ProjectError::new(
            &format!("shared_preview_{}", error.code),
            error.message,
            Some(root),
        ),
    })?;
    let repair = join.repair.as_ref().map(|_| RepairHint::reconnect());
    let binding = shared_execution_binding(session, join)?;
    let executable = binding.session.access == CollaborationAccess::Collaborator;
    let baseline_token = bounded.preview_token.clone();
    let preview_token = if executable {
        registry.issue_shared_bounded_task_confirmation(root, baseline_token, binding.clone())?
    } else {
        bounded.executable = false;
        bounded.loop_state.state = GoalLoopState::ExternalStop;
        bounded.loop_state.explicit_confirmation_required = false;
        bounded.loop_state.reason =
            "Viewer/public shared access is inspection-only; no mutation or Codex process may start"
                .into();
        String::new()
    };
    bounded.preview_token = preview_token.clone();
    Ok(SharedBoundedTaskPreview {
        bounded,
        binding,
        stop_conditions: shared_execution_stop_conditions(),
        executable,
        explicit_confirmation_required: executable,
        preview_token,
        repair,
    })
}

#[tauri::command]
fn preview_shared_bounded_task(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
) -> Result<SharedBoundedTaskPreview, ProjectError> {
    let root = validated_repository_root(&root)?;
    ensure_shared_iteration_has_no_repair_debt(&app_goal_storage(&app)?, &root)?;
    let registry = operation_registry();
    let lease = registry.begin(&root, OperationKind::Helper, None)?;
    let project_key = root.to_string_lossy().to_string();
    let context = sessions
        .sanitized_session_metadata(&project_key, &session_id)
        .map_err(|error| {
            collaboration_project_error(
                &root,
                "preview",
                collaboration_failure_from_transport(&error),
            )
        })?;
    build_shared_bounded_task_preview_with(
        &root,
        context.clone(),
        &lease.cancel,
        &registry,
        controller_helper,
        |local| {
            let files = read_remote_workspace_files(&sessions, &project_key, &session_id)?;
            Ok(join_workspace(
                &context.workspace_id,
                &context.actor,
                context.access,
                local,
                files,
            )?)
        },
    )
}

fn collaboration_failure_from_transport(error: &MdsyncTransportError) -> CollaborationFailure {
    match error.class() {
        MdsyncTransportErrorClass::CapabilityMaterial => {
            CollaborationFailure::capability_material_rejected()
        }
        MdsyncTransportErrorClass::AccessDenied => CollaborationFailure::access_denied(),
        MdsyncTransportErrorClass::Timeout => CollaborationFailure::timeout(),
        MdsyncTransportErrorClass::VersionConflict => CollaborationFailure::version_conflict(),
        MdsyncTransportErrorClass::SessionNotFound
        | MdsyncTransportErrorClass::Transport
        | MdsyncTransportErrorClass::ProjectBusy
        | MdsyncTransportErrorClass::SelectionChanged => {
            CollaborationFailure::transport_unavailable()
        }
        MdsyncTransportErrorClass::InvalidInput
        | MdsyncTransportErrorClass::Discovery
        | MdsyncTransportErrorClass::OriginMismatch
        | MdsyncTransportErrorClass::ResponseTooLarge
        | MdsyncTransportErrorClass::Protocol
        | MdsyncTransportErrorClass::Internal => CollaborationFailure::protocol(),
    }
}

fn collaboration_failure_from_envelope(error: &EnvelopeError) -> CollaborationFailure {
    match error.class {
        ha2ha_envelope::EnvelopeErrorClass::AccessDenied => CollaborationFailure::access_denied(),
        ha2ha_envelope::EnvelopeErrorClass::SourceMismatch => {
            CollaborationFailure::source_mismatch()
        }
        ha2ha_envelope::EnvelopeErrorClass::RepairRequired => {
            CollaborationFailure::repair_required()
        }
        ha2ha_envelope::EnvelopeErrorClass::InvalidInput
        | ha2ha_envelope::EnvelopeErrorClass::Protocol
        | ha2ha_envelope::EnvelopeErrorClass::Internal => CollaborationFailure::protocol(),
    }
}

struct MdsyncClaimPort<'a> {
    sessions: &'a MdsyncSessionStore,
    project_key: String,
    session_id: String,
    expected: SharedExecutionBinding,
    registry: Arc<OperationRegistry>,
    state: Mutex<SharedClaimState>,
    post_commit: Mutex<Option<PostLocalCommitCollaborationOutcome>>,
}

trait SharedClaimPort: CollaborationPort {
    fn shared_claim_state(&self) -> Result<SharedClaimState, ProjectError>;

    fn mark_claimed_pre_spawn_repair(
        &self,
        failure_class: CollaborationFailureClass,
        cause: SharedClaimRepairCause,
    ) -> Result<(), ProjectError>;

    fn post_commit_outcome(
        &self,
    ) -> Result<Option<PostLocalCommitCollaborationOutcome>, ProjectError> {
        Ok(None)
    }
}

impl<'a> MdsyncClaimPort<'a> {
    fn new(
        sessions: &'a MdsyncSessionStore,
        project_key: String,
        session_id: String,
        expected: SharedExecutionBinding,
        registry: Arc<OperationRegistry>,
    ) -> Self {
        Self {
            sessions,
            project_key,
            session_id,
            expected,
            registry,
            state: Mutex::new(SharedClaimState::Reconciled),
            post_commit: Mutex::new(None),
        }
    }

    fn state(&self) -> Result<SharedClaimState, ProjectError> {
        self.state.lock().map(|state| state.clone()).map_err(|_| {
            ProjectError::new(
                "shared_claim_state_failed",
                "Shared claim state is unavailable",
                Some(Path::new(&self.project_key)),
            )
        })
    }

    fn set_state(&self, state: SharedClaimState) -> Result<(), CollaborationFailure> {
        self.state
            .lock()
            .map(|mut current| *current = state)
            .map_err(|_| CollaborationFailure::protocol())
    }

    fn stopped(
        &self,
        failure_class: CollaborationFailureClass,
        latest_remote_version: Option<u64>,
        conflict_count: u8,
        repair: Option<RepairHint>,
    ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
        self.set_state(SharedClaimState::Stopped {
            failure_class,
            latest_remote_version,
            conflict_count,
            repair: repair.clone(),
        })?;
        Ok(PreRunCollaborationOutcome {
            reconciliation: if failure_class == CollaborationFailureClass::VersionConflict {
                ReconciliationState::Conflict
            } else {
                ReconciliationState::Reconciled
            },
            claim: ClaimResult::Stopped {
                failure_class,
                latest_remote_version,
                repair: repair.unwrap_or_else(RepairHint::reconnect),
            },
        })
    }

    fn claimed_repair(
        &self,
        remote_version: u64,
        failure_class: CollaborationFailureClass,
    ) -> Result<CollaborationFailure, CollaborationFailure> {
        self.set_state(SharedClaimState::ClaimedRepairRequired {
            remote_version,
            failure_class,
            cause: SharedClaimRepairCause::ClaimFinalization,
            repair: RepairHint::reconcile_claimed_pre_spawn(),
        })?;
        Ok(CollaborationFailure::claimed_pre_spawn_repair_required())
    }

    fn exact_readback(&self, write: &TaskClaimWrite) -> Result<bool, MdsyncTransportError> {
        self.sessions
            .read_file(&self.project_key, &self.session_id, &write.path)
            .map(|file| {
                file.matches_committed_write(
                    &self.expected.session.workspace_id,
                    &write.path,
                    &write.content,
                    &write.content_type,
                    &write.actor,
                    write.expected_post_version,
                )
            })
    }

    fn record_post_commit(
        &self,
        outcome: PostLocalCommitCollaborationOutcome,
    ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
        self.post_commit
            .lock()
            .map(|mut current| *current = Some(outcome.clone()))
            .map_err(|_| CollaborationFailure::protocol())?;
        Ok(outcome)
    }

    fn post_commit_partial(
        &self,
        intent: &RemoteCompletionIntent,
        remote_version: u64,
        completed_effects: &[MissingCollaborationEffect],
    ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
        self.record_post_commit(reconciliation_outcome_from_progress(
            intent,
            completed_effects,
            remote_version,
            false,
        )?)
    }
}

impl SharedClaimPort for MdsyncClaimPort<'_> {
    fn shared_claim_state(&self) -> Result<SharedClaimState, ProjectError> {
        self.state()
    }

    fn mark_claimed_pre_spawn_repair(
        &self,
        failure_class: CollaborationFailureClass,
        cause: SharedClaimRepairCause,
    ) -> Result<(), ProjectError> {
        self.state
            .lock()
            .map(|mut state| state.mark_claimed_pre_spawn_repair(failure_class, cause))
            .map_err(|_| {
                ProjectError::new(
                    "shared_claim_state_failed",
                    "Shared claim state is unavailable",
                    Some(Path::new(&self.project_key)),
                )
            })
    }

    fn post_commit_outcome(
        &self,
    ) -> Result<Option<PostLocalCommitCollaborationOutcome>, ProjectError> {
        self.post_commit
            .lock()
            .map(|state| state.clone())
            .map_err(|_| {
                ProjectError::new(
                    "shared_post_commit_state_failed",
                    "Shared post-commit state is unavailable",
                    Some(Path::new(&self.project_key)),
                )
            })
    }
}

impl CollaborationPort for MdsyncClaimPort<'_> {
    fn before_runtime(
        &self,
        context: &PreRunCollaborationContext,
        cancel: &AtomicBool,
    ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
        if context.mode != CollaborationMode::SharedCollaborator
            || context.session.as_ref() != Some(&self.expected.session)
            || context.local != self.expected.local
            || context.remote.as_ref() != Some(&self.expected.remote)
            || cancel.load(Ordering::Acquire)
        {
            return Err(if cancel.load(Ordering::Acquire) {
                CollaborationFailure::cancelled()
            } else {
                CollaborationFailure::source_mismatch()
            });
        }
        let fresh_session = self
            .sessions
            .sanitized_session_metadata(&self.project_key, &self.session_id)
            .map_err(|error| collaboration_failure_from_transport(&error))?;
        if fresh_session != self.expected.session
            || fresh_session.access != CollaborationAccess::Collaborator
        {
            return Err(CollaborationFailure::access_denied());
        }
        if cancel.load(Ordering::Acquire) {
            return Err(CollaborationFailure::cancelled());
        }
        let files = read_remote_workspace_files(self.sessions, &self.project_key, &self.session_id)
            .map_err(|error| match error {
                Ha2haEnvelopeCommandError::Transport(error) => {
                    collaboration_failure_from_transport(&error)
                }
                Ha2haEnvelopeCommandError::Envelope(error) => {
                    collaboration_failure_from_envelope(&error)
                }
                Ha2haEnvelopeCommandError::Project(_) => CollaborationFailure::protocol(),
            })?;
        if cancel.load(Ordering::Acquire) {
            return Err(CollaborationFailure::cancelled());
        }
        let joined = join_workspace(
            &fresh_session.workspace_id,
            &fresh_session.actor,
            fresh_session.access,
            context.local.clone(),
            files.clone(),
        )
        .map_err(|error| collaboration_failure_from_envelope(&error))?;
        if joined.task != self.expected.remote
            || joined.local != self.expected.local
            || joined.workspace_id != self.expected.session.workspace_id
            || joined.actor != self.expected.session.actor
            || joined.access != CollaborationAccess::Collaborator
        {
            return Err(CollaborationFailure::source_mismatch());
        }
        let remote_task = files
            .iter()
            .find(|file| file.path == self.expected.remote.task_path)
            .ok_or_else(CollaborationFailure::protocol)?;
        let write = project_task_claim(&fresh_session.actor, &self.expected.remote, remote_task)
            .map_err(|error| collaboration_failure_from_envelope(&error))?;
        if cancel.load(Ordering::Acquire) {
            return Err(CollaborationFailure::cancelled());
        }

        let mut recovered_from_readback = false;
        let committed_version = match self.sessions.write_file(
            &self.project_key,
            &self.session_id,
            MdsyncWriteInput {
                path: write.path.clone(),
                content: write.content.clone(),
                content_type: Some(write.content_type.clone()),
                base_version: Some(write.base_version),
            },
        ) {
            Ok(result)
                if result.path() == write.path
                    && result.version() == write.expected_post_version =>
            {
                result.version()
            }
            Ok(_) => match self.exact_readback(&write) {
                Ok(true) => {
                    recovered_from_readback = true;
                    write.expected_post_version
                }
                _ => {
                    return self.stopped(
                        CollaborationFailureClass::RepairRequired,
                        None,
                        0,
                        Some(RepairHint::reconnect()),
                    )
                }
            },
            Err(error) if error.class() == MdsyncTransportErrorClass::VersionConflict => {
                let (count, repair) = self
                    .registry
                    .record_shared_conflict(Path::new(&self.project_key), &self.expected)
                    .map_err(|_| CollaborationFailure::protocol())?;
                return self.stopped(
                    CollaborationFailureClass::VersionConflict,
                    error.latest_version(),
                    count,
                    Some(repair),
                );
            }
            Err(error) => match self.exact_readback(&write) {
                Ok(true) => {
                    recovered_from_readback = true;
                    write.expected_post_version
                }
                _ => {
                    return self.stopped(
                        collaboration_failure_from_transport(&error).class(),
                        None,
                        0,
                        Some(RepairHint::reconnect()),
                    )
                }
            },
        };
        self.set_state(SharedClaimState::Claimed {
            remote_version: committed_version,
            recovered_from_readback,
        })?;
        if cancel.load(Ordering::Acquire) {
            return Err(
                self.claimed_repair(committed_version, CollaborationFailureClass::Cancelled)?
            );
        }
        match self.exact_readback(&write) {
            Ok(true) => {}
            Ok(false) => {
                return Err(
                    self.claimed_repair(committed_version, CollaborationFailureClass::Protocol)?
                )
            }
            Err(error) => {
                return Err(self.claimed_repair(
                    committed_version,
                    collaboration_failure_from_transport(&error).class(),
                )?)
            }
        }
        let task = read_controller_task(Path::new(&self.project_key), &context.local.task_path)
            .map_err(|_| {
                self.claimed_repair(committed_version, CollaborationFailureClass::SourceMismatch)
                    .unwrap_or_else(|failure| failure)
            })?;
        let fresh_local = local_collaboration_binding(
            Path::new(&self.project_key),
            &context.local.task_path,
            &task.content,
        )
        .map_err(|_| {
            self.claimed_repair(committed_version, CollaborationFailureClass::SourceMismatch)
                .unwrap_or_else(|failure| failure)
        })?;
        if fresh_local != context.local || cancel.load(Ordering::Acquire) {
            return Err(self.claimed_repair(
                committed_version,
                if cancel.load(Ordering::Acquire) {
                    CollaborationFailureClass::Cancelled
                } else {
                    CollaborationFailureClass::SourceMismatch
                },
            )?);
        }
        self.registry
            .clear_shared_conflict(Path::new(&self.project_key), &self.expected)
            .map_err(|_| CollaborationFailure::protocol())?;
        Ok(PreRunCollaborationOutcome {
            reconciliation: ReconciliationState::Claimed,
            claim: ClaimResult::Claimed {
                remote_version: committed_version,
            },
        })
    }

    fn after_local_commit(
        &self,
        context: &PostLocalCommitCollaborationContext,
    ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
        let claimed_version = match self
            .state
            .lock()
            .map_err(|_| CollaborationFailure::protocol())?
            .clone()
        {
            SharedClaimState::Claimed { remote_version, .. } => remote_version,
            _ => return Err(CollaborationFailure::repair_required()),
        };
        let Some(intent) = context.intent.as_ref() else {
            return Err(CollaborationFailure::protocol());
        };
        if intent.claimed_task_version != claimed_version {
            return self.post_commit_partial(intent, claimed_version, &[]);
        }
        let fresh_session = match self
            .sessions
            .sanitized_session_metadata(&self.project_key, &self.session_id)
        {
            Ok(session)
                if session == self.expected.session
                    && session.access == CollaborationAccess::Collaborator =>
            {
                session
            }
            _ => return self.post_commit_partial(intent, claimed_version, &[]),
        };
        if fresh_session.workspace_id != intent.workspace_id || fresh_session.actor != intent.actor
        {
            return self.post_commit_partial(intent, claimed_version, &[]);
        }
        let files =
            match read_remote_workspace_files(self.sessions, &self.project_key, &self.session_id) {
                Ok(files) => files,
                Err(_) => return self.post_commit_partial(intent, claimed_version, &[]),
            };
        let plan = match project_post_run_reconciliation(intent, &files) {
            Ok(plan) => plan,
            Err(_) => return self.post_commit_partial(intent, claimed_version, &[]),
        };
        let (completed_effects, current_task_version, failure) =
            apply_post_run_write_sequence(&plan, |write| {
                self.sessions.write_file_with_readback(
                    &self.project_key,
                    &self.session_id,
                    MdsyncWriteInput {
                        path: write.path.clone(),
                        content: write.content.clone(),
                        content_type: Some(write.content_type.clone()),
                        base_version: write.base_version,
                    },
                    write.expected_post_version,
                )
            });
        if failure.is_some() {
            return self.post_commit_partial(intent, current_task_version, &completed_effects);
        }
        self.record_post_commit(reconciliation_outcome_from_progress(
            intent,
            &completed_effects,
            current_task_version,
            true,
        )?)
    }

    fn validate_completion_intent_for_persistence(
        &self,
        intent: &RemoteCompletionIntent,
    ) -> Result<(), CollaborationFailure> {
        self.sessions
            .validate_completion_intent_for_persistence(&self.project_key, &self.session_id, intent)
            .map_err(|error| collaboration_failure_from_transport(&error))
    }
}

fn apply_post_run_write_sequence<E, T>(
    plan: &ha2ha_envelope::PostRunReconciliationPlan,
    mut write: impl FnMut(&PostRunEffectWrite) -> Result<T, E>,
) -> (Vec<MissingCollaborationEffect>, u64, Option<E>) {
    let mut completed_effects = plan.applied_effects.clone();
    let mut current_task_version = plan.current_task_version;
    for effect_write in &plan.writes {
        match write(effect_write) {
            Ok(_) => {
                completed_effects.push(effect_write.effect);
                completed_effects.sort();
                if effect_write.effect == MissingCollaborationEffect::TaskUpdate {
                    current_task_version = effect_write.expected_post_version;
                }
            }
            Err(error) => return (completed_effects, current_task_version, Some(error)),
        }
    }
    (completed_effects, current_task_version, None)
}

fn skill_setup_argv(operation: SkillSetupOperation) -> Vec<String> {
    match operation {
        SkillSetupOperation::Install => {
            let mut argv = vec![
                "x".into(),
                SKILL_SETUP_CLI_VERSION.into(),
                "add".into(),
                SKILL_SETUP_SOURCE.into(),
            ];
            for id in BUILD_RIGHT_SKILL_IDS {
                argv.push("--skill".into());
                argv.push(id.into());
            }
            argv.extend([
                "--agent".into(),
                "codex".into(),
                "--yes".into(),
                "--copy".into(),
            ]);
            argv
        }
        SkillSetupOperation::Update => {
            let mut argv = vec!["x".into(), SKILL_SETUP_CLI_VERSION.into(), "update".into()];
            argv.extend(BUILD_RIGHT_SKILL_IDS.into_iter().map(String::from));
            argv.extend(["--project".into(), "--yes".into()]);
            argv
        }
    }
}

fn built_in_skill_ui_contract(id: &str) -> Option<&'static str> {
    match id {
        "build-right-preflight" => Some(include_str!("../../skill-ui/build-right-preflight.json")),
        "build-right-feature-planning" => Some(include_str!(
            "../../skill-ui/build-right-feature-planning.json"
        )),
        "build-right-execution" => Some(include_str!("../../skill-ui/build-right-execution.json")),
        "build-right-engineering-principles" => Some(include_str!(
            "../../skill-ui/build-right-engineering-principles.json"
        )),
        _ => None,
    }
}

fn bound_skill_ui_contract(id: &str, lock_hash: &str) -> Result<Vec<u8>, String> {
    let raw = built_in_skill_ui_contract(id)
        .ok_or_else(|| format!("No built-in first-party contract exists for {id}"))?;
    let mut contract: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| format!("Cannot parse built-in contract for {id}: {error}"))?;
    let provenance = contract
        .get_mut("provenance")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| format!("Built-in contract for {id} has no provenance object"))?;
    provenance.insert(
        "lockHash".into(),
        serde_json::Value::String(lock_hash.into()),
    );
    let mut bytes = serde_json::to_vec_pretty(&contract)
        .map_err(|error| format!("Cannot encode built-in contract for {id}: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn create_contract_no_overwrite(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "Skill UI contract has no parent directory".to_string())?;
    let temp = parent.join(format!(
        ".pax-skill-ui-{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("Cannot create temporary skill UI contract: {error}"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Cannot write temporary skill UI contract: {error}"))?;
    drop(file);
    let result = fs::hard_link(&temp, target)
        .map_err(|error| format!("Cannot create skill UI contract without overwrite: {error}"));
    let _ = fs::remove_file(&temp);
    result
}

fn replace_matching_contract(
    root: &Path,
    relative: &str,
    expected: &[u8],
    desired: &[u8],
) -> Result<(), String> {
    let target = root.join(relative);
    let mut selected = open_regular_no_follow(&target)
        .map_err(|error| format!("Cannot safely open existing skill UI contract: {error}"))?;
    selected
        .lock_exclusive()
        .map_err(|error| format!("Cannot lock existing skill UI contract: {error}"))?;
    let selected_identity = file_identity(&selected, &target)
        .map_err(|error| format!("Cannot identify existing skill UI contract: {error}"))?;
    let selected_bytes = read_file_bytes(&mut selected)
        .map_err(|error| format!("Cannot read existing skill UI contract: {error}"))?;
    if selected_bytes == desired {
        return Ok(());
    }
    if selected_bytes != expected {
        return Err(format!(
            "{relative} differs from the trusted built-in contract; it was not overwritten"
        ));
    }
    let parent = target
        .parent()
        .ok_or_else(|| "Skill UI contract has no parent directory".to_string())?;
    let mut replacement = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Cannot prepare skill UI contract replacement: {error}"))?;
    replacement
        .write_all(desired)
        .and_then(|_| replacement.as_file_mut().sync_all())
        .map_err(|error| format!("Cannot write skill UI contract replacement: {error}"))?;
    let mut current = open_regular_no_follow(&target)
        .map_err(|error| format!("Skill UI contract changed before replacement: {error}"))?;
    let current_identity = file_identity(&current, &target)
        .map_err(|error| format!("Cannot re-identify skill UI contract: {error}"))?;
    if current_identity != selected_identity {
        return Err("Skill UI contract identity changed before replacement".into());
    }
    let current_bytes = read_file_bytes(&mut current)
        .map_err(|error| format!("Cannot re-read skill UI contract: {error}"))?;
    if current_bytes != expected {
        return Err("Skill UI contract content changed before replacement".into());
    }
    replacement
        .persist(&target)
        .map_err(|error| format!("Cannot replace skill UI contract: {}", error.error))?;
    sync_parent_directory(parent)
        .map_err(|error| format!("Cannot sync skill UI contract directory: {error}"))
}

fn sync_first_party_skill_ui_contracts(
    root: &Path,
    before: &[SkillProvenanceState],
    after: &[SkillProvenanceState],
) -> Result<(), String> {
    let contract_root = root.join("skill-ui");
    match fs::symlink_metadata(&contract_root) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err("skill-ui must be a non-symlink directory".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&contract_root)
                .map_err(|error| format!("Cannot create skill-ui directory: {error}"))?;
        }
        Err(error) => return Err(format!("Cannot inspect skill-ui directory: {error}")),
    }
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("Cannot resolve project root: {error}"))?;
    let canonical_contract_root = fs::canonicalize(&contract_root)
        .map_err(|error| format!("Cannot resolve skill-ui directory: {error}"))?;
    if !canonical_contract_root.starts_with(&canonical_root) {
        return Err("skill-ui resolves outside the selected project".into());
    }
    for id in BUILD_RIGHT_SKILL_IDS {
        let after_hash = after
            .iter()
            .find(|state| state.skill_id == id)
            .and_then(|state| state.lock_hash.as_deref())
            .ok_or_else(|| format!("Installed skill {id} has no post-setup lock hash"))?;
        let desired = bound_skill_ui_contract(id, after_hash)?;
        let relative = format!("skill-ui/{id}.json");
        let target = root.join(&relative);
        match fs::symlink_metadata(&target) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                create_contract_no_overwrite(&target, &desired)?;
            }
            Err(error) => return Err(format!("Cannot inspect {relative}: {error}")),
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(format!("{relative} must be a non-symlink regular file"))
            }
            Ok(_) => {
                let before_hash = before
                    .iter()
                    .find(|state| state.skill_id == id)
                    .and_then(|state| state.lock_hash.as_deref())
                    .unwrap_or(after_hash);
                let expected = bound_skill_ui_contract(id, before_hash)?;
                replace_matching_contract(root, &relative, &expected, &desired)?;
            }
        }
    }
    Ok(())
}

fn read_skill_lock_value(root: &Path) -> Result<Option<serde_json::Value>, ProjectError> {
    let path = root.join("skills-lock.json");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ProjectError::new(
            "skill_setup_provenance_failed",
            format!("Cannot inspect skills-lock.json: {error}"),
            Some(&path),
        )),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(ProjectError::new(
                "skill_setup_provenance_failed",
                "skills-lock.json must be a non-symlink regular file",
                Some(&path),
            ))
        }
        Ok(_) => {
            let raw = fs::read_to_string(&path).map_err(|error| {
                ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot read skills-lock.json: {error}"),
                    Some(&path),
                )
            })?;
            serde_json::from_str(&raw).map(Some).map_err(|error| {
                ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot parse skills-lock.json: {error}"),
                    Some(&path),
                )
            })
        }
    }
}

fn skill_setup_provenance(root: &Path) -> Result<Vec<SkillProvenanceState>, ProjectError> {
    let lock = read_skill_lock_value(root)?;
    let mut states = Vec::new();
    for id in BUILD_RIGHT_SKILL_IDS {
        let lock_entry = lock
            .as_ref()
            .and_then(|value| value.pointer(&format!("/skills/{id}")));
        if let Some(entry) = lock_entry {
            let source = entry.get("source").and_then(|value| value.as_str());
            let computed_hash = entry.get("computedHash").and_then(|value| value.as_str());
            if source != Some(SKILL_SETUP_SOURCE) {
                return Err(ProjectError::new(
                    "unsupported_skill_source",
                    format!(
                        "Skill {id} is not locked to the supported {SKILL_SETUP_SOURCE} source"
                    ),
                    Some(Path::new("skills-lock.json")),
                ));
            }
            if computed_hash.is_none_or(|hash| hash.trim().is_empty()) {
                return Err(ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Skill {id} has no valid computedHash"),
                    Some(Path::new("skills-lock.json")),
                ));
            }
        }
        let installed_path = format!(".agents/skills/{id}/SKILL.md");
        let installed = match fs::symlink_metadata(root.join(&installed_path)) {
            Ok(metadata) => !metadata.file_type().is_symlink() && metadata.is_file(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                return Err(ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot inspect installed skill {id}: {error}"),
                    Some(Path::new(&installed_path)),
                ))
            }
        };
        let lock_hash = lock_entry
            .and_then(|entry| entry.get("computedHash"))
            .and_then(|value| value.as_str())
            .map(String::from);
        states.push(SkillProvenanceState {
            skill_id: id.into(),
            installed_path,
            installed,
            lock_hash,
        });
    }
    Ok(states)
}

fn skill_setup_preview_token_for(
    root: &Path,
    operation: SkillSetupOperation,
    provenance: &[SkillProvenanceState],
) -> Result<String, ProjectError> {
    let mut files = collect_setup_file_hashes(root)?;
    for id in BUILD_RIGHT_SKILL_IDS {
        let relative = format!("skill-ui/{id}.json");
        let path = root.join(&relative);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot inspect skill UI baseline: {error}"),
                    Some(&path),
                ))
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ProjectError::new(
                "skill_setup_provenance_failed",
                "Skill UI baseline must be a non-symlink regular file",
                Some(&path),
            ));
        }
        let bytes = fs::read(&path).map_err(|error| {
            ProjectError::new(
                "skill_setup_provenance_failed",
                format!("Cannot read skill UI baseline: {error}"),
                Some(&path),
            )
        })?;
        files.insert(relative, format!("sha256:{:x}", Sha256::digest(bytes)));
    }
    let mut hasher = Sha256::new();
    let operation_name = match operation {
        SkillSetupOperation::Install => "install",
        SkillSetupOperation::Update => "update",
    };
    for value in std::iter::once(root.to_string_lossy().to_string())
        .chain([
            operation_name.into(),
            "bun".into(),
            SKILL_SETUP_SOURCE.into(),
            SKILL_SETUP_CLI_VERSION.into(),
        ])
        .chain(skill_setup_argv(operation))
        .chain(provenance.iter().flat_map(|state| {
            [
                state.skill_id.clone(),
                state.installed_path.clone(),
                state.installed.to_string(),
                state.lock_hash.clone().unwrap_or_else(|| "missing".into()),
            ]
        }))
        .chain(files.into_iter().flat_map(|(path, hash)| [path, hash]))
    {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(test)]
fn skill_setup_preview_token(
    root: &Path,
    operation: SkillSetupOperation,
) -> Result<String, ProjectError> {
    let provenance = skill_setup_provenance(root)?;
    skill_setup_preview_token_for(root, operation, &provenance)
}

fn build_skill_setup_preview(
    root: &Path,
    operation: SkillSetupOperation,
    source: &str,
) -> Result<SkillSetupPreview, ProjectError> {
    if source != SKILL_SETUP_SOURCE {
        return Err(ProjectError::new(
            "unsupported_skill_source",
            "Only the fixed first-party pax-k/build-right source is supported",
            None,
        ));
    }
    let provenance = skill_setup_provenance(root)?;
    let preview_token = skill_setup_preview_token_for(root, operation, &provenance)?;
    Ok(SkillSetupPreview {
        operation,
        target_project: root.to_string_lossy().to_string(),
        source: SKILL_SETUP_SOURCE.into(),
        executable: "bun".into(),
        cli_version: SKILL_SETUP_CLI_VERSION.into(),
        argv: skill_setup_argv(operation),
        skill_ids: BUILD_RIGHT_SKILL_IDS
            .into_iter()
            .map(String::from)
            .collect(),
        expected_changed_paths: std::iter::once("skills-lock.json".into())
            .chain(
                BUILD_RIGHT_SKILL_IDS
                    .into_iter()
                    .map(|id| format!(".agents/skills/{id}/")),
            )
            .chain(
                BUILD_RIGHT_SKILL_IDS
                    .into_iter()
                    .map(|id| format!("skill-ui/{id}.json")),
            )
            .collect(),
        hash_changes: provenance
            .into_iter()
            .map(|state| SkillHashChange {
                skill_id: state.skill_id,
                current_hash: state.lock_hash,
                proposed_hash: None,
                proposed_state: "resolvedOnExecution".into(),
            })
            .collect(),
        explicit_confirmation_required: true,
        preview_token,
    })
}

#[tauri::command]
fn preview_skill_setup(
    root: String,
    operation: SkillSetupOperation,
) -> Result<SkillSetupPreview, ProjectError> {
    let root = validated_repository_root(&root)?;
    build_skill_setup_preview(&root, operation, SKILL_SETUP_SOURCE)
}

fn bounded_reader_with_limit<R: Read>(
    mut reader: R,
    output_limit: usize,
) -> Result<(Vec<u8>, bool), std::io::Error> {
    let mut retained = Vec::new();
    let mut truncated = false;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = output_limit.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    Ok((retained, truncated))
}

fn bounded_line_reader_with_limit<R, F>(
    reader: R,
    output_limit: usize,
    on_line: F,
) -> Result<(Vec<u8>, bool), std::io::Error>
where
    R: Read,
    F: Fn(&[u8]),
{
    let mut reader = reader;
    let mut retained = Vec::new();
    let mut truncated = false;
    let mut line = Vec::new();
    let mut line_overflow = false;
    let mut bytes_seen = 0_usize;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            let within_bound = bytes_seen < output_limit;
            bytes_seen = bytes_seen.saturating_add(1);
            if within_bound {
                retained.push(*byte);
            } else {
                truncated = true;
            }
            if *byte == b'\n' {
                if within_bound && !line_overflow && !line.iter().all(u8::is_ascii_whitespace) {
                    on_line(&line);
                }
                line.clear();
                line_overflow = false;
            } else if within_bound && !line_overflow {
                line.push(*byte);
            } else {
                line_overflow = true;
            }
        }
    }
    if !line.is_empty() && !line_overflow && !line.iter().all(u8::is_ascii_whitespace) {
        on_line(&line);
    }
    Ok((retained, truncated))
}

#[cfg(unix)]
fn terminate_process_group(process_group: u32) -> Result<(), String> {
    let result = unsafe { libc::kill(-(process_group as i32), libc::SIGKILL) };
    if result == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(format!(
            "Cannot terminate skill setup process group: {error}"
        ))
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_process_group: u32) -> Result<(), String> {
    Ok(())
}

fn stop_and_reap_process(
    child: &mut std::process::Child,
    process_group: u32,
) -> Result<std::process::ExitStatus, String> {
    let mut errors = Vec::new();
    if let Err(error) = terminate_process_group(process_group) {
        errors.push(error);
        if let Err(error) = child.kill() {
            errors.push(format!(
                "Cannot terminate direct skill setup process: {error}"
            ));
        }
    }
    let status = child.wait().map_err(|error| {
        errors.push(format!("Cannot reap stopped skill setup process: {error}"));
        errors.join("; ")
    })?;
    if errors.is_empty() {
        Ok(status)
    } else {
        Err(errors.join("; "))
    }
}

fn receive_bounded_output(
    receiver: mpsc::Receiver<Result<(Vec<u8>, bool), std::io::Error>>,
    stream: &str,
) -> Result<(Vec<u8>, bool), String> {
    receiver
        .recv_timeout(SKILL_SETUP_READER_DRAIN_TIMEOUT)
        .map_err(|error| format!("Timed out draining skill setup {stream}: {error}"))?
        .map_err(|error| format!("Cannot read skill setup {stream}: {error}"))
}

#[derive(Debug, Eq, PartialEq)]
enum StdinWriteOutcome {
    Completed,
    Stopped,
}

struct ManagedStdinWriter {
    stop: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<StdinWriteOutcome, String>>,
    handle: thread::JoinHandle<()>,
}

#[cfg(unix)]
fn set_nonblocking_stdin(stdin: &std::process::ChildStdin) -> Result<(), String> {
    use std::os::fd::AsRawFd;
    let descriptor = stdin.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 {
        return Err(format!(
            "Cannot inspect helper stdin flags: {}",
            std::io::Error::last_os_error()
        ));
    }
    if unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(format!(
            "Cannot make helper stdin nonblocking: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn wait_for_writable_stdin(
    stdin: &std::process::ChildStdin,
    stop: &AtomicBool,
) -> Result<bool, String> {
    use std::os::fd::AsRawFd;

    let mut descriptor = libc::pollfd {
        fd: stdin.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(false);
        }
        descriptor.revents = 0;
        let ready = unsafe {
            libc::poll(
                &mut descriptor,
                1,
                SKILL_SETUP_POLL_INTERVAL.as_millis() as libc::c_int,
            )
        };
        if ready > 0 {
            return Ok(true);
        }
        if ready == 0 {
            continue;
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(format!(
            "Cannot wait for helper stdin to become writable: {error}"
        ));
    }
}

#[cfg(not(unix))]
fn set_nonblocking_stdin(_stdin: &std::process::ChildStdin) -> Result<(), String> {
    Err("Managed helper stdin is supported only on Unix platforms".into())
}

fn spawn_managed_stdin_writer(
    mut stdin: std::process::ChildStdin,
    bytes: Vec<u8>,
) -> ManagedStdinWriter {
    let stop = Arc::new(AtomicBool::new(false));
    let writer_stop = Arc::clone(&stop);
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let result = set_nonblocking_stdin(&stdin).and_then(|()| {
            let mut offset = 0;
            while offset < bytes.len() {
                if writer_stop.load(Ordering::Acquire) {
                    return Ok(StdinWriteOutcome::Stopped);
                }
                match stdin.write(&bytes[offset..]) {
                    Ok(0) => {
                        return Err(
                            "Helper stdin closed before the verified snapshot was delivered".into(),
                        )
                    }
                    Ok(written) => {
                        offset += written;
                        if offset == bytes.len() {
                            return Ok(StdinWriteOutcome::Completed);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if !wait_for_writable_stdin(&stdin, &writer_stop)? {
                            return Ok(StdinWriteOutcome::Stopped);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(error) => {
                        if writer_stop.load(Ordering::Acquire) {
                            return Ok(StdinWriteOutcome::Stopped);
                        }
                        return Err(format!(
                            "Cannot write verified helper snapshot to Bun stdin: {error}"
                        ));
                    }
                }
            }
            Ok(StdinWriteOutcome::Completed)
        });
        let _ = sender.send(result);
    });
    ManagedStdinWriter {
        stop,
        receiver,
        handle,
    }
}

fn finish_managed_stdin_writer(writer: ManagedStdinWriter) -> Result<StdinWriteOutcome, String> {
    writer.stop.store(true, Ordering::Release);
    let result = writer
        .receiver
        .recv_timeout(SKILL_SETUP_READER_DRAIN_TIMEOUT)
        .map_err(|error| format!("Timed out stopping helper stdin writer: {error}"))?;
    writer
        .handle
        .join()
        .map_err(|_| "Helper stdin writer panicked".to_string())?;
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeJsExecutable {
    Bun,
}

impl NativeJsExecutable {
    fn file_name(self) -> &'static str {
        match self {
            Self::Bun => "bun",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Bun => "Bun",
        }
    }
}

fn trusted_native_executable(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    let metadata = fs::metadata(&canonical).ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let mode = metadata.mode();
        let effective_user = unsafe { libc::geteuid() };
        if mode & 0o111 == 0
            || mode & 0o022 != 0
            || (metadata.uid() != 0 && metadata.uid() != effective_user)
        {
            return None;
        }
    }
    Some(canonical)
}

fn resolve_native_js_executable_from(
    executable: NativeJsExecutable,
    path: Option<&std::ffi::OsStr>,
    home: Option<&Path>,
    standard_directories: &[&Path],
) -> Result<PathBuf, ProcessRunFailure> {
    let file_name = executable.file_name();
    let mut candidates = Vec::new();
    if let Some(path) = path {
        candidates.extend(
            std::env::split_paths(path)
                .filter(|directory| directory.is_absolute())
                .map(|directory| directory.join(file_name)),
        );
    }
    if let Some(home) = home.filter(|home| home.is_absolute()) {
        candidates.push(home.join(".bun/bin").join(file_name));
    }
    candidates.extend(
        standard_directories
            .iter()
            .filter(|directory| directory.is_absolute())
            .map(|directory| directory.join(file_name)),
    );

    let mut checked = std::collections::BTreeSet::new();
    for candidate in candidates {
        if checked.insert(candidate.clone()) {
            if let Some(executable) = trusted_native_executable(&candidate) {
                return Ok(executable);
            }
        }
    }
    Err(ProcessRunFailure::new(
        ProcessRunFailureKind::MissingExecutable,
        format!(
            "{} runtime was not found at a trusted executable in PATH or a standard install location; no process was started",
            executable.display_name()
        ),
    ))
}

fn resolve_native_js_executable(
    executable: NativeJsExecutable,
) -> Result<PathBuf, ProcessRunFailure> {
    let path = std::env::var_os("PATH");
    let home = std::env::var_os("HOME").map(PathBuf::from);
    resolve_native_js_executable_from(
        executable,
        path.as_deref(),
        home.as_deref(),
        &[Path::new("/opt/homebrew/bin"), Path::new("/usr/local/bin")],
    )
}

struct ResolvedCodexLauncher {
    executable: PathBuf,
    child_path: std::ffi::OsString,
}

fn resolve_codex_launcher_from(
    executable: &Path,
    inherited_path: Option<&std::ffi::OsStr>,
) -> Result<ResolvedCodexLauncher, ProcessRunFailure> {
    let Some(runtime_directory) = executable.parent().filter(|path| path.is_absolute()) else {
        return Err(ProcessRunFailure::new(
            ProcessRunFailureKind::MissingExecutable,
            "Codex runtime path is not an absolute allowlisted path; no process was started",
        ));
    };
    if trusted_native_executable(executable).is_none() {
        return Err(ProcessRunFailure::new(
            ProcessRunFailureKind::MissingExecutable,
            "Codex runtime is not a trusted executable; no process was started",
        ));
    }
    if trusted_native_executable(&runtime_directory.join("node")).is_none() {
        return Err(ProcessRunFailure::new(
            ProcessRunFailureKind::MissingExecutable,
            "Codex runtime has no trusted sibling Node interpreter; no process was started",
        ));
    }

    let mut directories = vec![runtime_directory.to_path_buf()];
    if let Some(inherited_path) = inherited_path {
        directories.extend(
            std::env::split_paths(inherited_path).filter(|directory| directory.is_absolute()),
        );
    }
    let mut seen = std::collections::BTreeSet::new();
    directories.retain(|directory| seen.insert(directory.clone()));
    let child_path = std::env::join_paths(directories).map_err(|_| {
        ProcessRunFailure::new(
            ProcessRunFailureKind::Start,
            "Cannot construct the closed Codex child PATH; no process was started",
        )
    })?;

    Ok(ResolvedCodexLauncher {
        executable: executable.to_path_buf(),
        child_path,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_codex_process_from(
    executable: &Path,
    inherited_path: Option<&std::ffi::OsStr>,
    argv: &[String],
    root: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
    stdout_line_handler: Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    output_limit: usize,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    let launcher = resolve_codex_launcher_from(executable, inherited_path)?;
    run_bounded_process_with_stdin_limit_and_path(
        launcher.executable.to_string_lossy().as_ref(),
        argv,
        root,
        timeout,
        cancel,
        None,
        stdout_line_handler,
        output_limit,
        Some(&launcher.child_path),
    )
}

fn run_codex_process(
    argv: &[String],
    root: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
    stdout_line_handler: Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    output_limit: usize,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    let inherited_path = std::env::var_os("PATH");
    run_codex_process_from(
        Path::new(CODEX_EXECUTABLE),
        inherited_path.as_deref(),
        argv,
        root,
        timeout,
        cancel,
        stdout_line_handler,
        output_limit,
    )
}

fn run_bounded_process(
    executable: &str,
    argv: &[String],
    root: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    run_bounded_process_with_stdin(executable, argv, root, timeout, cancel, None, None)
}

fn run_bounded_process_with_stdin(
    executable: &str,
    argv: &[String],
    root: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
    stdin_bytes: Option<&[u8]>,
    stdout_line_handler: Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    run_bounded_process_with_stdin_and_limit(
        executable,
        argv,
        root,
        timeout,
        cancel,
        stdin_bytes,
        stdout_line_handler,
        SKILL_SETUP_OUTPUT_LIMIT,
    )
}

fn run_bounded_process_with_stdin_and_limit(
    executable: &str,
    argv: &[String],
    root: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
    stdin_bytes: Option<&[u8]>,
    stdout_line_handler: Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    output_limit: usize,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    run_bounded_process_with_stdin_limit_and_path(
        executable,
        argv,
        root,
        timeout,
        cancel,
        stdin_bytes,
        stdout_line_handler,
        output_limit,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_bounded_process_with_stdin_limit_and_path(
    executable: &str,
    argv: &[String],
    root: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
    stdin_bytes: Option<&[u8]>,
    stdout_line_handler: Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    output_limit: usize,
    child_path: Option<&std::ffi::OsStr>,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    if cancel.load(Ordering::Acquire) {
        return Err(ProcessRunFailure::new(
            ProcessRunFailureKind::CancelledBeforeSpawn,
            "Skill setup was cancelled before process spawn",
        ));
    }
    let mut command = Command::new(executable);
    command
        .args(argv)
        .current_dir(root)
        .stdin(if stdin_bytes.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(child_path) = child_path {
        command.env("PATH", child_path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|error| {
        ProcessRunFailure::new(
            if error.kind() == std::io::ErrorKind::NotFound {
                ProcessRunFailureKind::MissingExecutable
            } else {
                ProcessRunFailureKind::Start
            },
            format!("Cannot start allowlisted process: {error}"),
        )
    })?;
    let process_group = child.id();
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let cleanup = stop_and_reap_process(&mut child, process_group).err();
            return Err(ProcessRunFailure::new(
                ProcessRunFailureKind::Cleanup,
                cleanup.unwrap_or_else(|| "Cannot capture skill setup stdout".into()),
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            drop(stdout);
            let cleanup = stop_and_reap_process(&mut child, process_group).err();
            return Err(ProcessRunFailure::new(
                ProcessRunFailureKind::Cleanup,
                cleanup.unwrap_or_else(|| "Cannot capture skill setup stderr".into()),
            ));
        }
    };
    let (stdout_sender, stdout_receiver) = mpsc::channel();
    let (stderr_sender, stderr_receiver) = mpsc::channel();
    thread::spawn(move || {
        let output = match stdout_line_handler {
            Some(handler) => {
                bounded_line_reader_with_limit(stdout, output_limit, |line| handler(line))
            }
            None => bounded_reader_with_limit(stdout, output_limit),
        };
        let _ = stdout_sender.send(output);
    });
    thread::spawn(move || {
        let _ = stderr_sender.send(bounded_reader_with_limit(stderr, output_limit));
    });
    let stdin_writer = match stdin_bytes {
        Some(bytes) => {
            let stdin = match child.stdin.take() {
                Some(stdin) => stdin,
                None => {
                    let cleanup = stop_and_reap_process(&mut child, process_group).err();
                    return Err(ProcessRunFailure::new(
                        ProcessRunFailureKind::Cleanup,
                        cleanup.unwrap_or_else(|| "Cannot open allowlisted process stdin".into()),
                    ));
                }
            };
            Some(spawn_managed_stdin_writer(stdin, bytes.to_vec()))
        }
        None => None,
    };
    let started = Instant::now();
    let process_result = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let cleanup = terminate_process_group(process_group);
                break cleanup
                    .map(|_| (status, ProcessTermination::Completed))
                    .map_err(|error| {
                        ProcessRunFailure::new(ProcessRunFailureKind::Cleanup, error)
                    });
            }
            Ok(None) => {}
            Err(error) => {
                let mut messages = vec![format!("Cannot inspect skill setup process: {error}")];
                if let Err(error) = stop_and_reap_process(&mut child, process_group) {
                    messages.push(error);
                }
                break Err(ProcessRunFailure::new(
                    ProcessRunFailureKind::Cleanup,
                    messages.join("; "),
                ));
            }
        }
        let termination = if cancel.load(Ordering::Acquire) {
            Some(ProcessTermination::Cancelled)
        } else if started.elapsed() >= timeout {
            Some(ProcessTermination::TimedOut)
        } else {
            None
        };
        if let Some(termination) = termination {
            if let Some(writer) = stdin_writer.as_ref() {
                writer.stop.store(true, Ordering::Release);
            }
            break stop_and_reap_process(&mut child, process_group)
                .map(|status| (status, termination))
                .map_err(|error| ProcessRunFailure::new(ProcessRunFailureKind::Cleanup, error));
        }
        thread::sleep(SKILL_SETUP_POLL_INTERVAL.min(timeout));
    };
    let stdin_result = stdin_writer.map(finish_managed_stdin_writer).transpose();
    let stdout_result = receive_bounded_output(stdout_receiver, "stdout");
    let stderr_result = receive_bounded_output(stderr_receiver, "stderr");
    let (status, termination) = process_result?;
    let stdin_outcome = stdin_result
        .map_err(|error| ProcessRunFailure::new(ProcessRunFailureKind::Cleanup, error))?;
    if termination == ProcessTermination::Completed
        && stdin_outcome.is_some_and(|outcome| outcome != StdinWriteOutcome::Completed)
    {
        return Err(ProcessRunFailure::new(
            ProcessRunFailureKind::Cleanup,
            "Allowlisted process exited before the verified stdin snapshot was fully delivered",
        ));
    }
    let (stdout, stdout_truncated) = stdout_result
        .map_err(|error| ProcessRunFailure::new(ProcessRunFailureKind::Cleanup, error))?;
    let (stderr, stderr_truncated) = stderr_result
        .map_err(|error| ProcessRunFailure::new(ProcessRunFailureKind::Cleanup, error))?;
    Ok(BoundedProcessOutput {
        status,
        termination,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
    })
}

fn run_skill_setup_process(
    root: &Path,
    argv: &[String],
    cancel: &AtomicBool,
) -> Result<BoundedProcessOutput, ProcessRunFailure> {
    let executable = resolve_native_js_executable(NativeJsExecutable::Bun)?;
    run_bounded_process(
        executable.to_string_lossy().as_ref(),
        argv,
        root,
        SKILL_SETUP_TIMEOUT,
        cancel,
    )
}

fn collect_setup_file_hashes(
    root: &Path,
) -> Result<std::collections::BTreeMap<String, String>, ProjectError> {
    let mut hashes = std::collections::BTreeMap::new();
    let mut pending = vec![root.join("skills-lock.json")];
    pending.extend(
        BUILD_RIGHT_SKILL_IDS
            .into_iter()
            .map(|id| root.join(".agents/skills").join(id)),
    );
    pending.extend(
        BUILD_RIGHT_SKILL_IDS
            .into_iter()
            .map(|id| root.join("skill-ui").join(format!("{id}.json"))),
    );
    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot inspect setup output: {error}"),
                    Some(&path),
                ))
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(ProjectError::new(
                "skill_setup_provenance_failed",
                "Setup output cannot be a symlink",
                Some(&path),
            ));
        }
        if metadata.is_dir() {
            let entries = fs::read_dir(&path).map_err(|error| {
                ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot read setup output: {error}"),
                    Some(&path),
                )
            })?;
            for entry in entries {
                pending.push(
                    entry
                        .map_err(|error| {
                            ProjectError::new(
                                "skill_setup_provenance_failed",
                                format!("Cannot read setup output entry: {error}"),
                                Some(&path),
                            )
                        })?
                        .path(),
                );
            }
        } else if metadata.is_file() {
            let bytes = fs::read(&path).map_err(|error| {
                ProjectError::new(
                    "skill_setup_provenance_failed",
                    format!("Cannot read setup output file: {error}"),
                    Some(&path),
                )
            })?;
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            hashes.insert(relative, format!("sha256:{:x}", Sha256::digest(bytes)));
        }
    }
    Ok(hashes)
}

fn changed_setup_paths(
    before: &std::collections::BTreeMap<String, String>,
    after: &std::collections::BTreeMap<String, String>,
) -> Vec<String> {
    let mut paths: Vec<String> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn relevant_skill_verification_errors(
    project: &ProjectSnapshot,
) -> std::collections::BTreeSet<String> {
    project
        .errors
        .iter()
        .filter(|error| {
            error.code == "invalid_skill_provenance"
                && error.path.as_deref().is_some_and(|path| {
                    BUILD_RIGHT_SKILL_IDS
                        .iter()
                        .any(|id| path.contains(&format!(".agents/skills/{id}")))
                })
        })
        .map(|error| {
            format!(
                "{}|{}",
                error.path.as_deref().unwrap_or_default(),
                error.message
            )
        })
        .collect()
}

fn execute_skill_setup_with<F>(
    root: &Path,
    operation: SkillSetupOperation,
    confirmed: bool,
    preview_token: &str,
    run: F,
) -> Result<SkillSetupResult, ProjectError>
where
    F: FnOnce(&Path, &[String]) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    let preview = build_skill_setup_preview(root, operation, SKILL_SETUP_SOURCE)?;
    let before = skill_setup_provenance(root)?;
    let before_files = collect_setup_file_hashes(root)?;
    let before_verification_errors =
        relevant_skill_verification_errors(&inspect_project_path(root));
    if !confirmed {
        return Ok(SkillSetupResult {
            operation,
            outcome: SkillSetupOutcome::CancelledBeforeExecution,
            executed: false,
            success: false,
            exit_status: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            changed_paths: vec![],
            after: skill_setup_provenance(root)?,
            before,
            repair: Some(SkillSetupRepair {
                code: "confirmation_required".into(),
                message: "Skill setup was cancelled before mutation".into(),
                next_action: "Review the preview and explicitly confirm to execute".into(),
            }),
            project: inspect_project_path(root),
        });
    }
    if preview.preview_token != preview_token {
        return Ok(SkillSetupResult {
            operation,
            outcome: SkillSetupOutcome::StalePreview,
            executed: false,
            success: false,
            exit_status: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            changed_paths: vec![],
            after: skill_setup_provenance(root)?,
            before,
            repair: Some(SkillSetupRepair {
                code: "stale_skill_setup_preview".into(),
                message:
                    "Repository skill provenance changed after preview; execution was not started"
                        .into(),
                next_action: "Refresh the setup preview and explicitly confirm the new baseline"
                    .into(),
            }),
            project: inspect_project_path(root),
        });
    }
    let process = run(root, &preview.argv);
    let after_result = skill_setup_provenance(root);
    let contract_sync_error = match (&process, &after_result) {
        (Ok(output), Ok(after))
            if output.termination == ProcessTermination::Completed && output.status.success() =>
        {
            sync_first_party_skill_ui_contracts(root, &before, after).err()
        }
        _ => None,
    };
    let after_files_result = collect_setup_file_hashes(root);
    let project = inspect_project_path(root);
    let changed_paths = after_files_result
        .as_ref()
        .map(|after_files| changed_setup_paths(&before_files, after_files))
        .unwrap_or_default();
    let verification_error = contract_sync_error.or_else(|| {
        after_result
            .as_ref()
            .err()
            .or_else(|| after_files_result.as_ref().err())
            .map(|error| error.message.clone())
    });
    if let Some(verification_error) = verification_error {
        let verification_outcome = match &process {
            Ok(output) if output.termination == ProcessTermination::Cancelled => {
                SkillSetupOutcome::Cancelled
            }
            Ok(output) if output.termination == ProcessTermination::TimedOut => {
                SkillSetupOutcome::TimedOut
            }
            _ => SkillSetupOutcome::VerificationFailed,
        };
        let executed = match &process {
            Ok(_) => true,
            Err(error) => error.kind == ProcessRunFailureKind::Cleanup,
        };
        let (exit_status, stdout, stderr, stdout_truncated, stderr_truncated) = match process {
            Ok(output) => (
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
                output.stdout_truncated,
                output.stderr_truncated,
            ),
            Err(error) => (None, String::new(), error.message, false, false),
        };
        return Ok(SkillSetupResult {
                operation,
                outcome: verification_outcome,
                executed,
                success: false,
                exit_status,
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
                changed_paths,
                before,
                after: after_result.unwrap_or_default(),
                repair: Some(SkillSetupRepair {
                    code: "post_setup_verification_failed".into(),
                    message: verification_error,
                    next_action: "Inspect repository errors and repair skills-lock.json or installed skill paths before retrying".into(),
                }),
                project,
            });
    }
    let after = after_result.expect("post-setup provenance was checked above");
    match process {
        Ok(output) => {
            let process_success =
                output.termination == ProcessTermination::Completed && output.status.success();
            let provenance_complete = after
                .iter()
                .all(|state| state.installed && state.lock_hash.is_some());
            let new_verification_errors: Vec<String> = relevant_skill_verification_errors(&project)
                .difference(&before_verification_errors)
                .cloned()
                .collect();
            let contracts_verified = new_verification_errors.is_empty();
            let success = process_success && provenance_complete && contracts_verified;
            let exit_status = output.status.code();
            Ok(SkillSetupResult {
                operation,
                outcome: if output.termination == ProcessTermination::Cancelled {
                    SkillSetupOutcome::Cancelled
                } else if output.termination == ProcessTermination::TimedOut {
                    SkillSetupOutcome::TimedOut
                } else if success {
                    SkillSetupOutcome::Completed
                } else if !output.status.success() {
                    SkillSetupOutcome::Failed
                } else {
                    SkillSetupOutcome::VerificationFailed
                },
                executed: true,
                success,
                exit_status,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                stdout_truncated: output.stdout_truncated,
                stderr_truncated: output.stderr_truncated,
                changed_paths,
                before,
                after,
                repair: if output.termination == ProcessTermination::Cancelled {
                    Some(SkillSetupRepair {
                        code: "skill_setup_cancelled".into(),
                        message: "Skill setup was cancelled and the process was stopped and reaped"
                            .into(),
                        next_action:
                            "Review refreshed repository provenance before previewing another setup"
                                .into(),
                    })
                } else if output.termination == ProcessTermination::TimedOut {
                    Some(SkillSetupRepair {
                        code: "skill_setup_timed_out".into(),
                        message: format!(
                            "Skill setup exceeded the {} second execution limit",
                            SKILL_SETUP_TIMEOUT.as_secs()
                        ),
                        next_action:
                            "Review bounded output and refreshed provenance before retrying".into(),
                    })
                } else if !process_success {
                    Some(SkillSetupRepair {
                        code: "skill_setup_failed".into(),
                        message: format!(
                            "Allowlisted skill setup exited with status {}",
                            exit_status.map_or_else(|| "signal".into(), |value| value.to_string())
                        ),
                        next_action:
                            "Inspect bounded stderr, repair the reported issue, then preview again"
                                .into(),
                    })
                } else if !provenance_complete {
                    Some(SkillSetupRepair {
                        code: "post_setup_verification_failed".into(),
                        message: "Skill setup exited successfully but complete installed paths and lock hashes were not found".into(),
                        next_action: "Inspect repository errors and repair project-scoped skill provenance before retrying".into(),
                    })
                } else if !contracts_verified {
                    Some(SkillSetupRepair {
                        code: "post_setup_contract_verification_failed".into(),
                        message: format!(
                            "Skill setup changed provenance without matching validated UI contracts: {}",
                            new_verification_errors.join("; ")
                        ),
                        next_action: "Refresh or repair the affected first-party skill UI contracts before retrying".into(),
                    })
                } else {
                    None
                },
                project,
            })
        }
        Err(error) => Ok(SkillSetupResult {
            operation,
            outcome: match error.kind {
                ProcessRunFailureKind::CancelledBeforeSpawn => SkillSetupOutcome::Cancelled,
                ProcessRunFailureKind::MissingExecutable => SkillSetupOutcome::StartFailed,
                ProcessRunFailureKind::Start => SkillSetupOutcome::StartFailed,
                ProcessRunFailureKind::Cleanup => SkillSetupOutcome::CleanupFailed,
            },
            executed: error.kind == ProcessRunFailureKind::Cleanup,
            success: false,
            exit_status: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            changed_paths,
            before,
            after,
            repair: Some(SkillSetupRepair {
                code: match error.kind {
                    ProcessRunFailureKind::CancelledBeforeSpawn => "skill_setup_cancelled",
                    ProcessRunFailureKind::MissingExecutable => "skill_setup_start_failed",
                    ProcessRunFailureKind::Start => "skill_setup_start_failed",
                    ProcessRunFailureKind::Cleanup => "skill_setup_cleanup_failed",
                }
                .into(),
                message: error.message,
                next_action: match error.kind {
                    ProcessRunFailureKind::CancelledBeforeSpawn => {
                        "Review refreshed repository provenance before previewing another setup"
                    }
                    ProcessRunFailureKind::MissingExecutable => {
                        "Verify Bun is installed, then preview and confirm again"
                    }
                    ProcessRunFailureKind::Start => {
                        "Verify Bun is installed, then preview and confirm again"
                    }
                    ProcessRunFailureKind::Cleanup => {
                        "Inspect process state and refreshed repository provenance before retrying"
                    }
                }
                .into(),
            }),
            project,
        }),
    }
}

#[tauri::command]
fn execute_skill_setup(
    root: String,
    operation: SkillSetupOperation,
    confirmed: bool,
    preview_token: String,
) -> Result<SkillSetupResult, ProjectError> {
    let root = validated_repository_root(&root)?;
    if !confirmed {
        return execute_skill_setup_with(&root, operation, false, &preview_token, |_, _| {
            unreachable!("unconfirmed setup cannot execute")
        });
    }
    let registry = operation_registry();
    let lease = registry.begin(&root, OperationKind::SkillSetup, None)?;
    execute_skill_setup_with(&root, operation, true, &preview_token, |root, argv| {
        run_skill_setup_process(root, argv, &lease.cancel)
    })
}

#[tauri::command]
fn cancel_skill_setup(root: String) -> Result<SkillSetupCancellation, ProjectError> {
    let root = validated_repository_root(&root)?;
    let requested = operation_registry().cancel_root(&root, OperationKind::SkillSetup)?;
    Ok(SkillSetupCancellation {
        cancellation_requested: requested,
        message: if requested {
            "Cancellation requested; waiting for the process to stop and repository truth to refresh".into()
        } else {
            "No skill setup is currently running for this project".into()
        },
    })
}

fn content_version(content: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(content))
}

#[cfg(unix)]
#[derive(Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(not(unix))]
#[derive(Debug, Eq, PartialEq)]
struct FileIdentity {
    canonical_path: PathBuf,
    length: u64,
    modified: Option<std::time::SystemTime>,
}

#[cfg(unix)]
fn file_identity(file: &File, _path: &Path) -> Result<FileIdentity, std::io::Error> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn file_identity(file: &File, path: &Path) -> Result<FileIdentity, std::io::Error> {
    let metadata = file.metadata()?;
    Ok(FileIdentity {
        canonical_path: fs::canonicalize(path)?,
        length: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> Result<File, std::io::Error> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(not(unix))]
fn open_regular_no_follow(path: &Path) -> Result<File, std::io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Project file cannot be a symlink",
        ));
    }
    OpenOptions::new().read(true).open(path)
}

fn read_file_bytes(file: &mut File) -> Result<Vec<u8>, std::io::Error> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> Result<(), std::io::Error> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> Result<(), std::io::Error> {
    Ok(())
}

fn read_project_file_path(
    root: &Path,
    relative_path: &str,
) -> Result<ProjectFileContent, ProjectError> {
    let path = resolve_existing(root, relative_path).map_err(|message| {
        ProjectError::new(
            "invalid_project_path",
            message,
            Some(Path::new(relative_path)),
        )
    })?;
    validate_markdown_path(&path).map_err(|message| {
        ProjectError::new("invalid_file_type", message, Some(Path::new(relative_path)))
    })?;
    let metadata = fs::metadata(&path).map_err(|error| {
        ProjectError::new(
            "read_failed",
            format!("Cannot inspect project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    if !metadata.is_file() {
        return Err(ProjectError::new(
            "invalid_file_type",
            "Selected project path is not a regular file",
            Some(Path::new(relative_path)),
        ));
    }
    let bytes = fs::read(&path).map_err(|error| {
        ProjectError::new(
            "read_failed",
            format!("Cannot read project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    let content = String::from_utf8(bytes.clone()).map_err(|error| {
        ProjectError::new(
            "invalid_encoding",
            format!("Project Markdown is not UTF-8: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    Ok(ProjectFileContent {
        path: relative_path.into(),
        content,
        version: content_version(&bytes),
    })
}

#[tauri::command]
fn read_project_file(
    root: String,
    relative_path: String,
) -> Result<ProjectFileContent, ProjectError> {
    let root = validated_repository_root(&root)?;
    read_project_file_path(&root, &relative_path)
}

fn write_project_file_inner<F>(
    root: &Path,
    relative_path: &str,
    content: &str,
    expected_version: &str,
    before_commit: F,
) -> Result<ProjectWriteResult, ProjectError>
where
    F: FnOnce(&Path),
{
    let path = resolve_writable(&root, &relative_path).map_err(|message| {
        ProjectError::new(
            "invalid_project_path",
            message,
            Some(Path::new(&relative_path)),
        )
    })?;
    validate_markdown_path(&path).map_err(|message| {
        ProjectError::new(
            "invalid_file_type",
            message,
            Some(Path::new(&relative_path)),
        )
    })?;
    let mut selected = open_regular_no_follow(&path).map_err(|error| {
        ProjectError::new(
            "invalid_project_path",
            format!("Cannot safely open selected project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    selected.lock_exclusive().map_err(|error| {
        ProjectError::new(
            "write_failed",
            format!("Cannot lock selected project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    let selected_identity = file_identity(&selected, &path).map_err(|error| {
        ProjectError::new(
            "write_failed",
            format!("Cannot identify selected project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    let current_bytes = read_file_bytes(&mut selected).map_err(|error| {
        ProjectError::new(
            "read_failed",
            format!("Cannot read selected project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    if content_version(&current_bytes) != expected_version {
        return Err(ProjectError::new(
            "stale_version",
            "Project file changed since it was selected; refresh before saving",
            Some(Path::new(&relative_path)),
        ));
    }
    let permissions = fs::metadata(&path)
        .map_err(|error| {
            ProjectError::new(
                "write_failed",
                format!("Cannot inspect project file: {error}"),
                Some(Path::new(&relative_path)),
            )
        })?
        .permissions();
    let parent = path.parent().expect("validated project file has a parent");
    let mut replacement = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        ProjectError::new(
            "write_failed",
            format!("Cannot create atomic replacement: {error}"),
            Some(Path::new(&relative_path)),
        )
    })?;
    replacement
        .as_file_mut()
        .set_permissions(permissions)
        .map_err(|error| {
            ProjectError::new(
                "write_failed",
                format!("Cannot preserve file permissions: {error}"),
                Some(Path::new(&relative_path)),
            )
        })?;
    replacement
        .write_all(content.as_bytes())
        .and_then(|_| replacement.as_file_mut().sync_all())
        .map_err(|error| {
            ProjectError::new(
                "write_failed",
                format!("Cannot write atomic replacement: {error}"),
                Some(Path::new(&relative_path)),
            )
        })?;
    before_commit(&path);
    let checked_path = resolve_writable(&root, &relative_path).map_err(|message| {
        ProjectError::new(
            "invalid_project_path",
            message,
            Some(Path::new(&relative_path)),
        )
    })?;
    let mut current_path = open_regular_no_follow(&checked_path).map_err(|error| {
        ProjectError::new(
            "path_changed",
            format!("Selected project path changed before commit: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    let current_identity = file_identity(&current_path, &checked_path).map_err(|error| {
        ProjectError::new(
            "path_changed",
            format!("Cannot verify selected project path identity: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    if current_identity != selected_identity {
        return Err(ProjectError::new(
            "path_changed",
            "Selected project path now refers to a different file",
            Some(Path::new(relative_path)),
        ));
    }
    let current_bytes = read_file_bytes(&mut current_path).map_err(|error| {
        ProjectError::new(
            "read_failed",
            format!("Cannot re-read selected project file: {error}"),
            Some(Path::new(relative_path)),
        )
    })?;
    if content_version(&current_bytes) != expected_version {
        return Err(ProjectError::new(
            "stale_version",
            "Project file changed while the replacement was prepared; refresh before saving",
            Some(Path::new(&relative_path)),
        ));
    }
    replacement.persist(&path).map_err(|error| {
        ProjectError::new(
            "write_failed",
            format!("Cannot replace project file: {}", error.error),
            Some(Path::new(&relative_path)),
        )
    })?;
    sync_parent_directory(parent).map_err(|error| {
        ProjectError::new(
            "post_persist_verification_failed",
            format!("Project file was replaced but its directory could not be synced: {error}"),
            Some(Path::new(relative_path)),
        )
        .after_commit()
    })?;

    let file = read_project_file_path(&root, &relative_path).map_err(|error| {
        ProjectError::new(
            "post_persist_verification_failed",
            format!(
                "Project file was replaced but readback failed: {}",
                error.message
            ),
            Some(Path::new(relative_path)),
        )
        .after_commit()
    })?;
    if file.version != content_version(content.as_bytes()) {
        return Err(ProjectError::new(
            "post_persist_verification_failed",
            "Project file was replaced but readback did not match the requested content",
            Some(Path::new(relative_path)),
        )
        .after_commit());
    }

    Ok(ProjectWriteResult {
        file,
        project: inspect_project_path(&root),
    })
}

fn write_project_file_serialized<F>(
    root: &Path,
    relative_path: &str,
    content: &str,
    expected_version: &str,
    before_commit: F,
) -> Result<ProjectWriteResult, ProjectError>
where
    F: FnOnce(&Path),
{
    let _guard = WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| {
            ProjectError::new(
                "write_lock_failed",
                "Repository write lock is poisoned",
                Some(root),
            )
        })?;
    write_project_file_inner(
        root,
        relative_path,
        content,
        expected_version,
        before_commit,
    )
}

#[tauri::command]
fn write_project_file(
    root: String,
    relative_path: String,
    content: String,
    expected_version: String,
) -> Result<ProjectWriteResult, ProjectError> {
    let root = validated_repository_root(&root)?;
    write_project_file_serialized(&root, &relative_path, &content, &expected_version, |_| {})
}

#[derive(Clone, Copy)]
struct HelperSpec {
    skill_id: &'static str,
    script_path: &'static str,
    sha256: &'static str,
    expected_length: usize,
}

#[derive(Debug)]
struct PreparedHelper {
    argv: Vec<String>,
    script_bytes: Vec<u8>,
}

fn helper_spec(helper_id: HelperId) -> HelperSpec {
    match helper_id {
        HelperId::PreflightCheck => HelperSpec {
            skill_id: "build-right-preflight",
            script_path: ".agents/skills/build-right-preflight/scripts/preflight-check.ts",
            sha256: "e01e10f2f669a50d61304daf3385c326c72758d522ef518c6353a00f650cbb8e",
            expected_length: 11_949,
        },
        HelperId::FeaturePlanningCheck => HelperSpec {
            skill_id: "build-right-feature-planning",
            script_path:
                ".agents/skills/build-right-feature-planning/scripts/feature-planning-check.ts",
            sha256: "57b8a08e09d5f992e97268046c70046dd9c00ba0ecd096a44f9c9fcc994c1e18",
            expected_length: 16_225,
        },
        HelperId::ContinueCheck => HelperSpec {
            skill_id: "build-right-execution",
            script_path: ".agents/skills/build-right-execution/scripts/continue-check.ts",
            sha256: "153c221252f637c24859bd0bb6ddc0136801ad350793364adafd88204176bb40",
            expected_length: 24_041,
        },
        HelperId::ExecutionCheck => HelperSpec {
            skill_id: "build-right-execution",
            script_path: ".agents/skills/build-right-execution/scripts/execution-check.ts",
            sha256: "4025c61d41c881dad9f79b8f339ea126d601c8b8b3e475d62aeb76b202b3b596",
            expected_length: 11_693,
        },
    }
}

fn helper_id_str(helper_id: HelperId) -> &'static str {
    match helper_id {
        HelperId::PreflightCheck => "preflight-check",
        HelperId::FeaturePlanningCheck => "feature-planning-check",
        HelperId::ContinueCheck => "continue-check",
        HelperId::ExecutionCheck => "execution-check",
    }
}

fn helper_mode_str(mode: HelperExecutionMode) -> &'static str {
    match mode {
        HelperExecutionMode::NextTask => "next-task",
        HelperExecutionMode::TaskContract => "task-contract",
        HelperExecutionMode::StopGates => "stop-gates",
        HelperExecutionMode::All => "all",
    }
}

fn validate_helper_contract(root: &Path, helper_id: HelperId) -> Result<(), ProjectError> {
    let spec = helper_spec(helper_id);
    let skill_id = spec.skill_id;
    let helper = helper_id_str(helper_id);
    let (skills, _) = collect_skills_with_errors(root);
    let declared = skills.iter().any(|skill| {
        skill.id == skill_id
            && skill.renderer == "operating-card"
            && skill.helpers.iter().any(|id| id == helper)
    });
    if !declared {
        return Err(ProjectError::new(
            "helper_contract_not_validated",
            format!("Helper {helper} is not declared by a validated first-party contract"),
            Some(Path::new(&format!("skill-ui/{skill_id}.json"))),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn read_helper_without_symlinks(
    root: &Path,
    relative: &str,
    expected_length: usize,
) -> Result<Vec<u8>, ProjectError> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;

    let relative = validate_relative(relative).map_err(|message| {
        ProjectError::new(
            "helper_script_untrusted",
            message,
            Some(Path::new(relative)),
        )
    })?;
    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ProjectError::new(
            "helper_script_untrusted",
            "Helper script path must contain only normal relative components",
            Some(relative),
        ));
    }

    let mut current = File::open(root).map_err(|error| {
        ProjectError::new(
            "helper_script_unavailable",
            format!("Cannot open project root: {error}"),
            Some(root),
        )
    })?;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            unreachable!()
        };
        let name = CString::new(name.as_bytes()).map_err(|_| {
            ProjectError::new(
                "helper_script_untrusted",
                "Helper path contains a null byte",
                Some(relative),
            )
        })?;
        let is_leaf = index + 1 == components.len();
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | if is_leaf { libc::O_NONBLOCK } else { 0 }
            | if is_leaf { 0 } else { libc::O_DIRECTORY };
        let descriptor = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
        if descriptor < 0 {
            let error = std::io::Error::last_os_error();
            let mut component_stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let stat_result = unsafe {
                libc::fstatat(
                    current.as_raw_fd(),
                    name.as_ptr(),
                    component_stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            let component_is_symlink = stat_result == 0
                && unsafe { component_stat.assume_init() }.st_mode & libc::S_IFMT == libc::S_IFLNK;
            let code = if error.raw_os_error() == Some(libc::ELOOP) || component_is_symlink {
                "helper_script_untrusted"
            } else {
                "helper_script_unavailable"
            };
            return Err(ProjectError::new(
                code,
                format!("Cannot securely open helper path component: {error}"),
                Some(relative),
            ));
        }
        current = unsafe { File::from_raw_fd(descriptor) };
    }
    let metadata = current.metadata().map_err(|error| {
        ProjectError::new(
            "helper_script_unavailable",
            format!("Cannot inspect opened helper: {error}"),
            Some(relative),
        )
    })?;
    if !metadata.is_file() {
        return Err(ProjectError::new(
            "helper_script_untrusted",
            "Helper script must be a non-symlink regular file",
            Some(relative),
        ));
    }
    if metadata.len() != expected_length as u64 {
        return Err(ProjectError::new(
            "helper_script_untrusted",
            format!(
                "Helper script length mismatch: expected {expected_length} bytes, found {}",
                metadata.len()
            ),
            Some(relative),
        ));
    }
    let hard_limit = expected_length.checked_add(1).ok_or_else(|| {
        ProjectError::new(
            "helper_script_untrusted",
            "Helper length limit overflow",
            Some(relative),
        )
    })?;
    let mut bytes = Vec::with_capacity(hard_limit);
    current
        .take(hard_limit as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            ProjectError::new(
                "helper_script_unavailable",
                format!("Cannot read opened helper: {error}"),
                Some(relative),
            )
        })?;
    if bytes.len() > expected_length {
        return Err(ProjectError::new(
            "helper_script_untrusted",
            format!("Helper script exceeds the expected {expected_length} byte limit"),
            Some(relative),
        ));
    }
    if bytes.len() != expected_length {
        return Err(ProjectError::new(
            "helper_script_untrusted",
            format!("Helper script length changed while reading: expected {expected_length} bytes, found {}", bytes.len()),
            Some(relative),
        ));
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_helper_without_symlinks(
    _root: &Path,
    relative: &str,
    _expected_length: usize,
) -> Result<Vec<u8>, ProjectError> {
    Err(ProjectError::new(
        "unsupported_platform",
        "Deterministic helper execution is supported only on Unix platforms",
        Some(Path::new(relative)),
    ))
}

fn verified_helper_bytes(root: &Path, helper_id: HelperId) -> Result<Vec<u8>, ProjectError> {
    let spec = helper_spec(helper_id);
    let bytes = read_helper_without_symlinks(root, spec.script_path, spec.expected_length)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != spec.sha256 {
        return Err(ProjectError::new(
            "helper_digest_mismatch",
            format!(
                "Helper {} bytes do not match the supported Build Right release",
                helper_id_str(helper_id)
            ),
            Some(Path::new(spec.script_path)),
        ));
    }
    Ok(bytes)
}

fn is_supported_execution_task_path(path: &str) -> bool {
    let components = Path::new(path).components().collect::<Vec<_>>();
    let normal = components
        .iter()
        .map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>();
    let Some(parts) = normal else { return false };
    let supported_root = matches!(
        parts.as_slice(),
        ["tasks", "issues", _] | ["tasks", _] | ["issues", _]
    );
    let Some(file_name) = parts.last() else {
        return false;
    };
    supported_root
        && file_name.ends_with(".md")
        && !file_name.ends_with("sprint-0.md")
        && !file_name.ends_with("post-release-backlog.md")
}

fn execution_task_inventory(root: &Path) -> Vec<String> {
    let mut inventory = Vec::new();
    for directory in ["tasks/issues", "tasks", "issues"] {
        let directory_path = root.join(directory);
        let metadata = match fs::symlink_metadata(&directory_path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => metadata,
            _ => continue,
        };
        let _ = metadata;
        let Ok(entries) = fs::read_dir(&directory_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(&entry.path())
                .to_string_lossy()
                .to_string();
            if is_supported_execution_task_path(&relative) {
                if open_relative_regular_without_symlinks(root, &relative).is_ok() {
                    inventory.push(relative);
                }
            }
        }
    }
    inventory.sort();
    inventory.dedup();
    inventory
}

fn validated_helper_task_path(root: &Path, task_path: &str) -> Result<String, ProjectError> {
    validate_relative(task_path).map_err(|message| {
        ProjectError::new("invalid_helper_task", message, Some(Path::new(task_path)))
    })?;
    if !is_supported_execution_task_path(task_path) {
        return Err(ProjectError::new(
            "invalid_helper_task",
            "Helper task path is outside execution-check's supported direct task inventory",
            Some(Path::new(task_path)),
        ));
    }
    if !execution_task_inventory(root)
        .iter()
        .any(|path| path == task_path)
    {
        return Err(ProjectError::new(
            "invalid_helper_task",
            "Helper task path is not in execution-check's current repository inventory",
            Some(Path::new(task_path)),
        ));
    }
    Ok(task_path.to_string())
}

fn prepare_helper(
    root: &Path,
    invocation: &HelperInvocation,
) -> Result<PreparedHelper, ProjectError> {
    validate_helper_contract(root, invocation.helper_id)?;
    let script_bytes = verified_helper_bytes(root, invocation.helper_id)?;
    let root_value = root.to_string_lossy().to_string();
    let argv = match invocation.helper_id {
        HelperId::PreflightCheck => {
            if invocation.mode.is_some()
                || invocation.task_path.is_some()
                || invocation.feature_request.is_some()
            {
                return Err(ProjectError::new(
                    "invalid_helper_arguments",
                    "preflight-check accepts no mode or task input",
                    None,
                ));
            }
            Ok(vec![
                "-".into(),
                "--cwd".into(),
                root_value,
                "--mode".into(),
                "all".into(),
                "--format".into(),
                "json".into(),
            ])
        }
        HelperId::FeaturePlanningCheck => {
            if invocation.mode.is_some() || invocation.task_path.is_some() {
                return Err(ProjectError::new(
                    "invalid_helper_arguments",
                    "feature-planning-check accepts only one feature request",
                    None,
                ));
            }
            let feature = invocation
                .feature_request
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty() && value.len() <= 2_000)
                .ok_or_else(|| ProjectError::new(
                    "invalid_helper_arguments",
                    "feature-planning-check requires a non-empty feature request of at most 2000 bytes",
                    None,
                ))?;
            if feature.chars().any(|character| {
                character == '\0'
                    || (character.is_control() && character != '\n' && character != '\t')
            }) {
                return Err(ProjectError::new(
                    "invalid_helper_arguments",
                    "feature request contains unsupported control characters",
                    None,
                ));
            }
            Ok(vec![
                "-".into(),
                "--cwd".into(),
                root_value,
                "--feature".into(),
                feature.into(),
                "--format".into(),
                "json".into(),
            ])
        }
        HelperId::ContinueCheck => {
            if invocation.mode.is_some()
                || invocation.task_path.is_some()
                || invocation.feature_request.is_some()
            {
                return Err(ProjectError::new(
                    "invalid_helper_arguments",
                    "continue-check accepts no mode or task input",
                    None,
                ));
            }
            Ok(vec![
                "-".into(),
                "--cwd".into(),
                root_value,
                "--format".into(),
                "json".into(),
                "--strict".into(),
            ])
        }
        HelperId::ExecutionCheck => {
            if invocation.feature_request.is_some() {
                return Err(ProjectError::new(
                    "invalid_helper_arguments",
                    "execution-check accepts no feature request",
                    None,
                ));
            }
            let mode = invocation.mode.ok_or_else(|| {
                ProjectError::new(
                    "invalid_helper_arguments",
                    "execution-check requires a closed execution mode",
                    None,
                )
            })?;
            let task_path = invocation.task_path.as_deref().ok_or_else(|| {
                ProjectError::new(
                    "invalid_helper_arguments",
                    "execution-check requires an inventoried Markdown task",
                    None,
                )
            })?;
            let task_path = validated_helper_task_path(root, task_path)?;
            Ok(vec![
                "-".into(),
                "--cwd".into(),
                root_value,
                "--mode".into(),
                helper_mode_str(mode).into(),
                "--task".into(),
                task_path,
                "--format".into(),
                "json".into(),
            ])
        }
    }?;
    Ok(PreparedHelper { argv, script_bytes })
}

fn json_string(value: &serde_json::Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(|item| item.as_str())
        .filter(|item| !item.trim().is_empty())
        .map(String::from)
        .ok_or_else(|| format!("missing or invalid {field}"))
}

fn json_string_array(
    value: Option<&serde_json::Value>,
    field: &str,
) -> Result<Vec<String>, String> {
    let array = value
        .and_then(|item| item.as_array())
        .ok_or_else(|| format!("missing or invalid {field}"))?;
    array
        .iter()
        .map(|item| {
            item.as_str()
                .map(String::from)
                .ok_or_else(|| format!("invalid {field} entry"))
        })
        .collect()
}

fn parse_preflight_output(stdout: &[u8]) -> Result<HelperDecision, String> {
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|error| error.to_string())?;
    let mut evidence = Vec::new();
    let inventory = value
        .get("inventory")
        .and_then(|item| item.as_object())
        .ok_or("missing or invalid inventory")?;
    for (key, item) in inventory {
        evidence.push(format!("{key}: {item}"));
    }
    evidence.extend(json_string_array(
        value.get("missingArtifacts"),
        "missingArtifacts",
    )?);
    let warnings = json_string_array(value.get("readinessWarnings"), "readinessWarnings")?;
    let founder_questions = json_string_array(value.get("founderInputGaps"), "founderInputGaps")?;
    Ok(HelperDecision {
        decision: json_string(&value, "decision")?,
        confidence: json_string(&value, "confidence")?,
        next_action: json_string(&value, "nextAction")?,
        evidence,
        warnings,
        recommended_destination: None,
        blocking_gates: None,
        founder_questions: Some(founder_questions),
        research_triggers: None,
        ready_task_candidates: None,
    })
}

fn parse_feature_planning_output(stdout: &[u8]) -> Result<HelperDecision, String> {
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|error| error.to_string())?;
    let gates = value
        .get("blockingGates")
        .and_then(|item| item.as_array())
        .ok_or("missing or invalid blockingGates")?
        .iter()
        .map(|item| {
            Ok(PlanningGate {
                r#type: json_string(item, "type")?,
                source: json_string(item, "source")?,
                reason: json_string(item, "reason")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let candidates = value
        .get("readyTaskCandidates")
        .and_then(|item| item.as_array())
        .ok_or("missing or invalid readyTaskCandidates")?
        .iter()
        .map(|item| {
            Ok(PlanningTaskCandidate {
                id: json_string(item, "id")?,
                title: json_string(item, "title")?,
                status: json_string(item, "status")?,
                owner: json_string(item, "owner")?,
                path: json_string(item, "path")?,
                tracker: json_string(item, "tracker")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let founder_questions = json_string_array(value.get("founderQuestions"), "founderQuestions")?;
    let research_triggers = json_string_array(value.get("researchTriggers"), "researchTriggers")?;
    let scanned = json_string_array(value.get("scannedArtifacts"), "scannedArtifacts")?;
    Ok(HelperDecision {
        decision: json_string(&value, "decision")?,
        confidence: json_string(&value, "confidence")?,
        next_action: json_string(&value, "nextAction")?,
        evidence: scanned,
        warnings: gates
            .iter()
            .map(|gate| format!("{}: {}", gate.source, gate.reason))
            .collect(),
        recommended_destination: Some(json_string(&value, "recommendedDestination")?),
        blocking_gates: Some(gates),
        founder_questions: Some(founder_questions),
        research_triggers: Some(research_triggers),
        ready_task_candidates: Some(candidates),
    })
}

fn parse_continue_output(stdout: &[u8]) -> Result<HelperDecision, String> {
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|error| error.to_string())?;
    let evidence_values = value
        .get("evidence")
        .and_then(|item| item.as_array())
        .ok_or("missing or invalid evidence")?;
    let evidence = evidence_values
        .iter()
        .map(|item| {
            let source = json_string(item, "source")?;
            let summary = json_string(item, "summary")?;
            Ok(format!("{source}: {summary}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let gates = value
        .get("blockingGates")
        .and_then(|item| item.as_array())
        .ok_or("missing or invalid blockingGates")?;
    let warnings = gates
        .iter()
        .map(|item| {
            let source = json_string(item, "source")?;
            let reason = json_string(item, "reason")?;
            Ok(format!("{source}: {reason}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(HelperDecision {
        decision: json_string(&value, "decision")?,
        confidence: json_string(&value, "confidence")?,
        next_action: json_string(&value, "nextAction")?,
        evidence,
        warnings,
        recommended_destination: None,
        blocking_gates: None,
        founder_questions: None,
        research_triggers: None,
        ready_task_candidates: None,
    })
}

fn parse_execution_output(stdout: &[u8]) -> Result<HelperDecision, String> {
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|error| error.to_string())?;
    let recommendation = json_string(&value, "recommendation")?;
    let mut evidence = Vec::new();
    if let Some(task) = value.get("selectedTask").filter(|item| !item.is_null()) {
        evidence.push(format!("selected task: {}", json_string(task, "path")?));
    }
    let ready = value
        .get("readyTaskCandidates")
        .and_then(|item| item.as_array())
        .ok_or("missing or invalid readyTaskCandidates")?;
    for task in ready {
        evidence.push(format!("ready task: {}", json_string(task, "path")?));
    }
    let mut warnings = json_string_array(value.get("contractMissing"), "contractMissing")?;
    warnings.extend(json_string_array(value.get("gateReasons"), "gateReasons")?);
    let decision = if recommendation.starts_with("Proceed with one bounded task:") {
        "proceed"
    } else {
        "stop"
    };
    let confidence = if value.get("selectedTask").is_none_or(|item| item.is_null()) {
        "low"
    } else if warnings.is_empty() {
        "high"
    } else {
        "medium"
    };
    Ok(HelperDecision {
        decision: decision.into(),
        confidence: confidence.into(),
        next_action: recommendation,
        evidence,
        warnings,
        recommended_destination: None,
        blocking_gates: None,
        founder_questions: None,
        research_triggers: None,
        ready_task_candidates: None,
    })
}

fn verify_helper_output_binding(
    root: &Path,
    invocation: &HelperInvocation,
    stdout: &[u8],
) -> Result<(), String> {
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|error| error.to_string())?;
    let expected_cwd = root.to_string_lossy();
    let returned_cwd = json_string(&value, "cwd")?;
    if returned_cwd != expected_cwd {
        return Err(format!(
            "helper cwd mismatch: expected {expected_cwd}, received {returned_cwd}"
        ));
    }
    match invocation.helper_id {
        HelperId::PreflightCheck => {
            let returned_mode = json_string(&value, "mode")?;
            if returned_mode != "all" {
                return Err(format!(
                    "helper mode mismatch: expected all, received {returned_mode}"
                ));
            }
        }
        HelperId::FeaturePlanningCheck => {
            let expected_feature = invocation.feature_request.as_deref().ok_or_else(|| {
                "feature-planning-check invocation has no feature request".to_string()
            })?;
            let returned_feature = json_string(&value, "featureRequest")?;
            if returned_feature != expected_feature.trim() {
                return Err(format!("helper featureRequest mismatch: expected {expected_feature}, received {returned_feature}"));
            }
        }
        HelperId::ContinueCheck => {}
        HelperId::ExecutionCheck => {
            let expected_mode = invocation
                .mode
                .ok_or_else(|| "execution-check invocation has no mode".to_string())?;
            let returned_mode = json_string(&value, "mode")?;
            if returned_mode != helper_mode_str(expected_mode) {
                return Err(format!(
                    "helper mode mismatch: expected {}, received {returned_mode}",
                    helper_mode_str(expected_mode)
                ));
            }
            let expected_task = invocation
                .task_path
                .as_deref()
                .ok_or_else(|| "execution-check invocation has no task".to_string())?;
            let selected = value
                .get("selectedTask")
                .filter(|item| !item.is_null())
                .ok_or_else(|| "execution-check output has no selectedTask".to_string())?;
            let returned_task = json_string(selected, "path")?;
            if returned_task != expected_task {
                return Err(format!("helper selectedTask.path mismatch: expected {expected_task}, received {returned_task}"));
            }
        }
    }
    Ok(())
}

fn parse_helper_output(helper_id: HelperId, stdout: &[u8]) -> Result<HelperDecision, String> {
    match helper_id {
        HelperId::PreflightCheck => parse_preflight_output(stdout),
        HelperId::FeaturePlanningCheck => parse_feature_planning_output(stdout),
        HelperId::ContinueCheck => parse_continue_output(stdout),
        HelperId::ExecutionCheck => parse_execution_output(stdout),
    }
}

fn execute_helper_with<F>(
    root: &Path,
    invocation: HelperInvocation,
    run: F,
) -> Result<HelperResult, ProjectError>
where
    F: FnOnce(&Path, &[String], &[u8]) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_helper_with_platform(root, invocation, cfg!(unix), run)
}

fn execute_helper_with_platform<F>(
    root: &Path,
    invocation: HelperInvocation,
    platform_supported: bool,
    run: F,
) -> Result<HelperResult, ProjectError>
where
    F: FnOnce(&Path, &[String], &[u8]) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    if !platform_supported {
        return Ok(HelperResult {
            helper_id: invocation.helper_id,
            mode: invocation.mode,
            task_path: invocation.task_path,
            executable: "bun".into(),
            argv: Vec::new(),
            outcome: HelperOutcome::UnsupportedPlatform,
            executed: false,
            success: false,
            exit_status: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            decision: None,
            failure: Some("Deterministic helper timeout and cancellation are supported only on Unix platforms; no process was started".into()),
            project: inspect_project_path(root),
        });
    }
    let prepared = prepare_helper(root, &invocation)?;
    let process = run(root, &prepared.argv, &prepared.script_bytes);
    let project = inspect_project_path(root);
    let mut result = HelperResult {
        helper_id: invocation.helper_id,
        mode: invocation.mode,
        task_path: invocation.task_path.clone(),
        executable: "bun".into(),
        argv: prepared.argv,
        outcome: HelperOutcome::StartFailed,
        executed: false,
        success: false,
        exit_status: None,
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        decision: None,
        failure: None,
        project,
    };
    match process {
        Ok(output) => {
            result.executed = true;
            result.exit_status = output.status.code();
            result.stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            result.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            result.stdout_truncated = output.stdout_truncated;
            result.stderr_truncated = output.stderr_truncated;
            if output.termination == ProcessTermination::Cancelled {
                result.outcome = HelperOutcome::Cancelled;
                result.failure =
                    Some("Helper invocation was cancelled and its process group was reaped".into());
            } else if output.termination == ProcessTermination::TimedOut {
                result.outcome = HelperOutcome::TimedOut;
                result.failure = Some(format!(
                    "Helper exceeded the {} second execution limit",
                    HELPER_TIMEOUT.as_secs()
                ));
            } else if output.stdout_truncated || output.stderr_truncated {
                result.outcome = HelperOutcome::OutputOverflow;
                result.failure = Some("Helper output exceeded the bounded capture limit".into());
            } else if !output.status.success() {
                result.outcome = HelperOutcome::NonzeroExit;
                result.failure = Some(format!(
                    "Helper exited with status {}",
                    result
                        .exit_status
                        .map_or_else(|| "signal".into(), |status| status.to_string())
                ));
            } else if let Err(error) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                result.outcome = HelperOutcome::MalformedOutput;
                result.failure = Some(format!("Cannot parse helper JSON output: {error}"));
            } else if let Err(error) =
                verify_helper_output_binding(root, &invocation, &output.stdout)
            {
                result.outcome = HelperOutcome::VerificationFailed;
                result.failure = Some(format!("Cannot verify helper JSON output binding: {error}"));
            } else {
                match parse_helper_output(invocation.helper_id, &output.stdout) {
                    Ok(decision) => {
                        result.outcome = HelperOutcome::Completed;
                        result.success = true;
                        result.decision = Some(decision);
                    }
                    Err(error) => {
                        result.outcome = HelperOutcome::MalformedOutput;
                        result.failure = Some(format!("Cannot parse helper JSON output: {error}"));
                    }
                }
            }
        }
        Err(error) => {
            result.outcome = match error.kind {
                ProcessRunFailureKind::CancelledBeforeSpawn => HelperOutcome::Cancelled,
                ProcessRunFailureKind::MissingExecutable => HelperOutcome::MissingRuntime,
                ProcessRunFailureKind::Start => HelperOutcome::StartFailed,
                ProcessRunFailureKind::Cleanup => HelperOutcome::CleanupFailed,
            };
            result.executed = error.kind == ProcessRunFailureKind::Cleanup;
            result.failure = Some(error.message);
        }
    }
    Ok(result)
}

#[tauri::command]
fn execute_helper(
    root: String,
    invocation: HelperInvocation,
) -> Result<HelperResult, ProjectError> {
    let root = validated_repository_root(&root)?;
    let registry = operation_registry();
    let lease = registry.begin(&root, OperationKind::Helper, None)?;
    execute_helper_with(&root, invocation, |root, argv, script_bytes| {
        let executable = resolve_native_js_executable(NativeJsExecutable::Bun)?;
        run_bounded_process_with_stdin(
            executable.to_string_lossy().as_ref(),
            argv,
            root,
            HELPER_TIMEOUT,
            &lease.cancel,
            Some(script_bytes),
            None,
        )
    })
}

#[tauri::command]
fn cancel_helper(root: String) -> Result<HelperCancellation, ProjectError> {
    let root = validated_repository_root(&root)?;
    let requested = operation_registry().cancel_root(&root, OperationKind::Helper)?;
    Ok(HelperCancellation {
        cancellation_requested: requested,
        message: if requested {
            "Cancellation requested; waiting for the helper process to stop and repository truth to refresh".into()
        } else {
            "No helper is currently running for this project".into()
        },
    })
}

fn markdown_field(text: &str, field: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.strip_prefix(field)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(String::from)
    })
}

fn markdown_section(text: &str, heading: &str) -> String {
    let marker = format!("## {heading}");
    let Some(after) = text.split_once(&marker).map(|(_, after)| after) else {
        return String::new();
    };
    after
        .split("\n## ")
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn markdown_bullets(text: &str, heading: &str) -> Vec<String> {
    markdown_section(text, heading)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- ").map(String::from))
        .collect()
}

#[cfg(unix)]
fn open_relative_regular_without_symlinks(
    root: &Path,
    relative: &str,
) -> Result<File, ProjectError> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;

    let relative_path = validate_relative(relative).map_err(|message| {
        ProjectError::new(
            "controller_task_untrusted",
            message,
            Some(Path::new(relative)),
        )
    })?;
    let components = relative_path.components().collect::<Vec<_>>();
    let mut current = File::open(root).map_err(|error| {
        ProjectError::new(
            "controller_task_read_failed",
            format!("Cannot open project root: {error}"),
            Some(root),
        )
    })?;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(ProjectError::new(
                "controller_task_untrusted",
                "Task path contains a non-normal component",
                Some(relative_path),
            ));
        };
        let name = CString::new(name.as_bytes()).map_err(|_| {
            ProjectError::new(
                "controller_task_untrusted",
                "Task path contains a null byte",
                Some(relative_path),
            )
        })?;
        let is_leaf = index + 1 == components.len();
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | if is_leaf { 0 } else { libc::O_DIRECTORY };
        let descriptor = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
        if descriptor < 0 {
            return Err(ProjectError::new(
                "controller_task_untrusted",
                format!(
                    "Cannot securely open task path component: {}",
                    std::io::Error::last_os_error()
                ),
                Some(relative_path),
            ));
        }
        current = unsafe { File::from_raw_fd(descriptor) };
    }
    if !current.metadata().is_ok_and(|metadata| metadata.is_file()) {
        return Err(ProjectError::new(
            "controller_task_untrusted",
            "Selected task must be a non-symlink regular file",
            Some(relative_path),
        ));
    }
    Ok(current)
}

#[cfg(not(unix))]
fn open_relative_regular_without_symlinks(
    root: &Path,
    relative: &str,
) -> Result<File, ProjectError> {
    let path = resolve_regular_root_file(root, relative).map_err(|message| {
        ProjectError::new(
            "controller_task_untrusted",
            message,
            Some(Path::new(relative)),
        )
    })?;
    open_regular_no_follow(&path).map_err(|error| {
        ProjectError::new(
            "controller_task_read_failed",
            format!("Cannot securely open selected task: {error}"),
            Some(Path::new(relative)),
        )
    })
}

fn read_controller_task(root: &Path, relative: &str) -> Result<ProjectFileContent, ProjectError> {
    validate_markdown_path(Path::new(relative)).map_err(|message| {
        ProjectError::new(
            "controller_task_untrusted",
            message,
            Some(Path::new(relative)),
        )
    })?;
    let mut file = open_relative_regular_without_symlinks(root, relative)?;
    let bytes = read_file_bytes(&mut file).map_err(|error| {
        ProjectError::new(
            "controller_task_read_failed",
            format!("Cannot read selected task: {error}"),
            Some(Path::new(relative)),
        )
    })?;
    let content = String::from_utf8(bytes.clone()).map_err(|_| {
        ProjectError::new(
            "controller_task_read_failed",
            "Selected task must be valid UTF-8 Markdown",
            Some(Path::new(relative)),
        )
    })?;
    Ok(ProjectFileContent {
        path: relative.into(),
        content,
        version: content_version(&bytes),
    })
}

fn bounded_task_prompt(task_path: &str) -> String {
    format!(
        "Use the repo-local build-right-execution skill. Execute exactly the selected task at {task_path}. Follow its full single-task workflow, verification ladder, evidence contract, and stop gates. Do not select or begin another task. Repository evidence and verification, never provider self-report, determine completion."
    )
}

fn controller_helper(
    root: &Path,
    invocation: HelperInvocation,
    cancel: &AtomicBool,
) -> Result<HelperResult, ProjectError> {
    execute_helper_with(root, invocation, |root, argv, script_bytes| {
        let executable = resolve_native_js_executable(NativeJsExecutable::Bun)?;
        run_bounded_process_with_stdin(
            executable.to_string_lossy().as_ref(),
            argv,
            root,
            HELPER_TIMEOUT,
            cancel,
            Some(script_bytes),
            None,
        )
    })
}

fn goal_loop_effects() -> Vec<String> {
    workflow_controller::EXPECTED_EFFECTS
        .iter()
        .map(|effect| (*effect).into())
        .collect()
}

fn goal_loop_stop_conditions() -> Vec<String> {
    workflow_controller::STOP_CONDITIONS
        .iter()
        .map(|condition| (*condition).into())
        .collect()
}

fn resolver_stop_state(decision: &str, blocking_gates: &[String]) -> GoalLoopState {
    match workflow_controller::resolver_stop_kind(decision, blocking_gates) {
        workflow_controller::ResolverStopKind::Founder => GoalLoopState::FounderStop,
        workflow_controller::ResolverStopKind::External => GoalLoopState::ExternalStop,
        workflow_controller::ResolverStopKind::Conflict => GoalLoopState::ConflictStop,
        workflow_controller::ResolverStopKind::NoReadyTask => GoalLoopState::NoReadyTaskStop,
        workflow_controller::ResolverStopKind::InvalidState => GoalLoopState::InvalidStateStop,
    }
}

fn resolver_next_task(resolver: &HelperResult) -> Option<String> {
    if !resolver.success
        || resolver
            .decision
            .as_ref()
            .is_none_or(|decision| decision.decision != "execute-task")
    {
        return None;
    }
    let path = workflow_controller::resolver_selected_task(&resolver.stdout)?;
    if validate_markdown_path(Path::new(&path)).is_err() {
        return None;
    }
    Some(path)
}

fn preview_loop_projection(
    decision: &str,
    next_action: &str,
    blocking_gates: &[String],
    selected_task: Option<&str>,
    executable: bool,
) -> GoalLoopProjection {
    if executable {
        return GoalLoopProjection {
            state: GoalLoopState::AwaitingConfirmation,
            next_task: selected_task.map(str::to_string),
            blocking_gates: blocking_gates.to_vec(),
            expected_effects: goal_loop_effects(),
            explicit_confirmation_required: true,
            automatic_execution_started: false,
            reason: "Repository truth was reconstructed; one resolver-selected task awaits explicit confirmation".into(),
        };
    }
    GoalLoopProjection {
        state: resolver_stop_state(decision, blocking_gates),
        next_task: None,
        blocking_gates: blocking_gates.to_vec(),
        expected_effects: Vec::new(),
        explicit_confirmation_required: false,
        automatic_execution_started: false,
        reason: next_action.into(),
    }
}

fn build_bounded_task_preview_with<F>(
    root: &Path,
    cancel: &AtomicBool,
    run_helper: &mut F,
) -> Result<BoundedTaskPreview, ProjectError>
where
    F: FnMut(&Path, HelperInvocation, &AtomicBool) -> Result<HelperResult, ProjectError>,
{
    let resolver = run_helper(
        root,
        HelperInvocation {
            helper_id: HelperId::ContinueCheck,
            mode: None,
            task_path: None,
            feature_request: None,
        },
        cancel,
    )?;
    if !resolver.success {
        return Err(ProjectError::new(
            "controller_resolver_failed",
            resolver
                .failure
                .unwrap_or_else(|| "Full continue-check did not complete".into()),
            Some(root),
        ));
    }
    let value: serde_json::Value = serde_json::from_str(&resolver.stdout).map_err(|error| {
        ProjectError::new(
            "controller_resolver_malformed",
            format!("Cannot parse continue-check output: {error}"),
            Some(root),
        )
    })?;
    let decision = json_string(&value, "decision")
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    let confidence = json_string(&value, "confidence")
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    let next_action = json_string(&value, "nextAction")
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    let gates = value
        .get("blockingGates")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ProjectError::new(
                "controller_resolver_malformed",
                "continue-check omitted blockingGates",
                Some(root),
            )
        })?;
    let blocking_gates = gates
        .iter()
        .map(|gate| {
            let source = json_string(gate, "source")?;
            let reason = json_string(gate, "reason")?;
            Ok(format!("{source}: {reason}"))
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    if decision != "execute-task" || !blocking_gates.is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(resolver.stdout.as_bytes());
        let loop_state =
            preview_loop_projection(&decision, &next_action, &blocking_gates, None, false);
        return Ok(BoundedTaskPreview {
            decision,
            confidence,
            next_action,
            blocking_gates,
            selected_task: None,
            executable: false,
            goal: String::new(),
            non_goals: Vec::new(),
            source_under_test: String::new(),
            expected_effects: Vec::new(),
            live_host_warning:
                "Resolver stop decision is non-executable; no provider process may start.".into(),
            prompt: String::new(),
            preview_token: format!("sha256:{:x}", hasher.finalize()),
            loop_state,
        });
    }
    let selected = value
        .get("nextTask")
        .filter(|task| !task.is_null())
        .ok_or_else(|| {
            ProjectError::new(
                "controller_no_selected_task",
                "execute-task returned no selected task",
                Some(root),
            )
        })?;
    let selected_task = json_string(selected, "path")
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    let status = json_string(selected, "status")
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    let owner = json_string(selected, "owner")
        .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    let missing = json_string_array(
        selected.get("missingContractFields"),
        "missingContractFields",
    )
    .map_err(|error| ProjectError::new("controller_resolver_malformed", error, Some(root)))?;
    if status != "ready" || !owner.eq_ignore_ascii_case("ai") || !missing.is_empty() {
        return Err(ProjectError::new(
            "controller_task_not_ready",
            format!("Selected task must be ready, AI-owned, and contract-complete; status={status}, owner={owner}, missing={}", missing.join(", ")),
            Some(Path::new(&selected_task)),
        ));
    }
    let contract = run_helper(
        root,
        HelperInvocation {
            helper_id: HelperId::ExecutionCheck,
            mode: Some(HelperExecutionMode::TaskContract),
            task_path: Some(selected_task.clone()),
            feature_request: None,
        },
        cancel,
    )?;
    if !contract.success
        || contract
            .decision
            .as_ref()
            .is_none_or(|result| result.decision != "proceed" || !result.warnings.is_empty())
    {
        return Err(ProjectError::new(
            "controller_task_contract_failed",
            contract
                .failure
                .unwrap_or_else(|| "execution-check rejected the selected task contract".into()),
            Some(Path::new(&selected_task)),
        ));
    }
    let task_file = read_controller_task(root, &selected_task)?;
    let task_text = task_file.content;
    let goal = markdown_section(&task_text, "Goal");
    let non_goals = markdown_bullets(&task_text, "Non-Goals");
    let source_under_test =
        markdown_field(&task_text, "Source under test:").unwrap_or_else(|| "missing".into());
    if goal.is_empty() || non_goals.is_empty() || source_under_test == "missing" {
        return Err(ProjectError::new(
            "controller_task_contract_failed",
            "Selected task preview fields are incomplete",
            Some(Path::new(&selected_task)),
        ));
    }
    let prompt = bounded_task_prompt(&selected_task);
    let mut hasher = Sha256::new();
    hasher.update(resolver.stdout.as_bytes());
    hasher.update(contract.stdout.as_bytes());
    hasher.update(task_text.as_bytes());
    hasher.update(prompt.as_bytes());
    let expected_effects = goal_loop_effects();
    let loop_state = preview_loop_projection(
        &decision,
        &next_action,
        &blocking_gates,
        Some(&selected_task),
        true,
    );
    Ok(BoundedTaskPreview {
        decision,
        confidence,
        next_action,
        blocking_gates,
        selected_task: Some(selected_task),
        executable: true,
        goal,
        non_goals,
        source_under_test,
        expected_effects,
        live_host_warning: "Live bounded execution uses Codex workspace-write with the current user's host permissions. It may mutate the selected repository and is not an operating-system filesystem sandbox.".into(),
        prompt,
        preview_token: format!("sha256:{:x}", hasher.finalize()),
        loop_state,
    })
}

fn build_bounded_task_preview(
    root: &Path,
    cancel: &AtomicBool,
) -> Result<BoundedTaskPreview, ProjectError> {
    build_bounded_task_preview_with(root, cancel, &mut controller_helper)
}

#[tauri::command]
fn preview_bounded_task(root: String) -> Result<BoundedTaskPreview, ProjectError> {
    preview_bounded_task_with_registry(root, operation_registry())
}

fn preview_bounded_task_with_registry(
    root: String,
    registry: Arc<OperationRegistry>,
) -> Result<BoundedTaskPreview, ProjectError> {
    let root = validated_repository_root(&root)?;
    let lease = registry.begin(&root, OperationKind::Helper, None)?;
    registry.invalidate_bounded_task_confirmation(&root)?;
    let mut preview = build_bounded_task_preview(&root, &lease.cancel)?;
    if preview.executable {
        preview.preview_token =
            registry.issue_bounded_task_confirmation(&root, preview.preview_token)?;
    }
    Ok(preview)
}

fn runtime_capabilities(platform_supported: bool) -> RuntimeCapabilities {
    RuntimeCapabilities {
        event_stream: true,
        cancellation: platform_supported,
        timeout: platform_supported,
        raw_payload: true,
        fixture: true,
        live: platform_supported,
        repository_authority: false,
    }
}

fn runtime_argv(root: &Path, prompt: &str) -> Vec<String> {
    vec![
        "exec".into(),
        "--json".into(),
        "--ephemeral".into(),
        "--ignore-user-config".into(),
        "--sandbox".into(),
        "read-only".into(),
        "--color".into(),
        "never".into(),
        "-C".into(),
        root.to_string_lossy().to_string(),
        "--".into(),
        prompt.into(),
    ]
}

fn bounded_task_runtime_argv(root: &Path, prompt: &str) -> Vec<String> {
    let mut argv = runtime_argv(root, prompt);
    let sandbox = argv
        .iter()
        .position(|argument| argument == "--sandbox")
        .expect("runtime argv owns a sandbox option");
    argv[sandbox + 1] = "workspace-write".into();
    // This closed adapter needs repo-local skills, not provider plugin catalogs or apps. Keeping
    // those defaults enabled adds remote/cache initialization before Codex emits thread.started.
    argv.splice(
        sandbox..sandbox,
        [
            "--disable".into(),
            "plugins".into(),
            "--disable".into(),
            "remote_plugin".into(),
            "--disable".into(),
            "apps".into(),
        ],
    );
    argv
}

fn validated_runtime_prompt(prompt: Option<&str>) -> Result<&str, String> {
    let prompt = prompt.ok_or_else(|| "A live runtime prompt is required".to_string())?;
    if prompt.trim().is_empty() {
        return Err("The live runtime prompt cannot be blank".into());
    }
    if prompt.as_bytes().len() > RUNTIME_PROMPT_LIMIT {
        return Err(format!(
            "The live runtime prompt exceeds the {RUNTIME_PROMPT_LIMIT} byte limit"
        ));
    }
    if prompt.contains('\0') {
        return Err("The live runtime prompt cannot contain a null byte".into());
    }
    if prompt.starts_with('-') {
        return Err("The live runtime prompt cannot begin with '-' because provider options are native-owned".into());
    }
    Ok(prompt)
}

fn runtime_event_kind(provider_type: &str, value: &serde_json::Value) -> RuntimeEventKind {
    match provider_type {
        "thread.started" | "thread.resumed" => RuntimeEventKind::Session,
        "turn.started" => RuntimeEventKind::Turn,
        "turn.completed" => RuntimeEventKind::Usage,
        "error" | "turn.failed" => RuntimeEventKind::Error,
        "item.started" | "item.updated" | "item.completed" => match value
            .get("item")
            .and_then(|item| item.get("type"))
            .and_then(serde_json::Value::as_str)
        {
            Some("agent_message") => RuntimeEventKind::Message,
            Some("command_execution") => RuntimeEventKind::Command,
            Some("file_change") => RuntimeEventKind::FileChange,
            Some("mcp_tool_call") | Some("web_search") => RuntimeEventKind::Tool,
            Some("reasoning") => RuntimeEventKind::Reasoning,
            _ => RuntimeEventKind::Unknown,
        },
        _ => RuntimeEventKind::Unknown,
    }
}

fn runtime_event_summary(
    kind: RuntimeEventKind,
    provider_type: &str,
    value: &serde_json::Value,
) -> String {
    let item = value.get("item").unwrap_or(value);
    match kind {
        RuntimeEventKind::Message => item
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(provider_type)
            .to_string(),
        RuntimeEventKind::Command => item
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(provider_type)
            .to_string(),
        RuntimeEventKind::FileChange => item
            .get("changes")
            .map(ToString::to_string)
            .unwrap_or_else(|| provider_type.into()),
        RuntimeEventKind::Error => value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(provider_type)
            .to_string(),
        RuntimeEventKind::Usage => value
            .get("usage")
            .map(ToString::to_string)
            .unwrap_or_else(|| provider_type.into()),
        _ => provider_type.into(),
    }
}

fn encode_payload(bytes: &[u8]) -> EncodedPayload {
    match std::str::from_utf8(bytes) {
        Ok(value) => EncodedPayload {
            encoding: PayloadEncoding::Utf8,
            data: value.into(),
        },
        Err(_) => EncodedPayload {
            encoding: PayloadEncoding::Hex,
            data: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
        },
    }
}

struct ParsedRuntimeStream {
    events: Vec<RuntimeEvent>,
    malformed: bool,
    provider_failed: bool,
    turn_completed: bool,
}

fn parse_runtime_line(
    line: &[u8],
    sequence: usize,
    provenance: &str,
) -> (RuntimeEvent, bool, bool, bool) {
    let raw_payload = encode_payload(line);
    let value = match std::str::from_utf8(line) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(text) {
            Ok(value) => value,
            Err(error) => {
                return (
                    RuntimeEvent {
                        sequence,
                        kind: RuntimeEventKind::Malformed,
                        provider_type: None,
                        summary: format!("Malformed provider JSONL: {error}"),
                        raw_payload,
                        provenance: provenance.into(),
                    },
                    true,
                    false,
                    false,
                );
            }
        },
        Err(error) => {
            return (
                RuntimeEvent {
                    sequence,
                    kind: RuntimeEventKind::Malformed,
                    provider_type: None,
                    summary: format!("Provider JSONL is not valid UTF-8: {error}"),
                    raw_payload,
                    provenance: provenance.into(),
                },
                true,
                false,
                false,
            );
        }
    };
    let Some(provider_type) = value.get("type").and_then(serde_json::Value::as_str) else {
        return (
            RuntimeEvent {
                sequence,
                kind: RuntimeEventKind::Malformed,
                provider_type: None,
                summary: "JSONL event has no string type".into(),
                raw_payload,
                provenance: provenance.into(),
            },
            true,
            false,
            false,
        );
    };
    let kind = runtime_event_kind(provider_type, &value);
    let provider_failed = matches!(provider_type, "error" | "turn.failed");
    let turn_completed = provider_type == "turn.completed";
    (
        RuntimeEvent {
            sequence,
            kind,
            provider_type: Some(provider_type.into()),
            summary: runtime_event_summary(kind, provider_type, &value),
            raw_payload,
            provenance: provenance.into(),
        },
        false,
        provider_failed,
        turn_completed,
    )
}

fn parse_runtime_jsonl(stdout: &[u8], provenance: &str) -> ParsedRuntimeStream {
    let mut parsed = ParsedRuntimeStream {
        events: Vec::new(),
        malformed: false,
        provider_failed: false,
        turn_completed: false,
    };
    for line in stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
    {
        let (event, malformed, provider_failed, turn_completed) =
            parse_runtime_line(line, parsed.events.len(), provenance);
        parsed.malformed |= malformed;
        parsed.provider_failed |= provider_failed;
        parsed.turn_completed |= turn_completed;
        parsed.events.push(event);
    }
    parsed
}

fn add_runtime_stderr_event(events: &mut Vec<RuntimeEvent>, stderr: &[u8], provenance: &str) {
    if stderr.is_empty() {
        return;
    }
    events.push(RuntimeEvent {
        sequence: events.len(),
        kind: RuntimeEventKind::Stderr,
        provider_type: None,
        summary: "Provider stderr captured separately from JSONL".into(),
        raw_payload: encode_payload(stderr),
        provenance: provenance.into(),
    });
}

fn runtime_result(
    root: &Path,
    mode: RuntimeMode,
    platform_supported: bool,
    argv: Vec<String>,
) -> RuntimeResult {
    RuntimeResult {
        run_id: String::new(),
        outcome: RuntimeOutcome::StartFailed,
        executed: false,
        success: false,
        exit_status: None,
        events: Vec::new(),
        stdout: encode_payload(&[]),
        stderr: encode_payload(&[]),
        stdout_truncated: false,
        stderr_truncated: false,
        failure: None,
        capabilities: runtime_capabilities(platform_supported),
        provenance: RuntimeProvenance {
            adapter: "runtime-port/v1".into(),
            provider: "codex-jsonl/v1".into(),
            mode,
            executable: CODEX_EXECUTABLE.into(),
            runtime_version: None,
            project_root: root.to_string_lossy().to_string(),
            argv,
            simulated: mode == RuntimeMode::Fixture,
        },
        repository_authority_advanced: false,
    }
}

#[cfg(unix)]
fn native_runtime_run_id() -> Result<String, ProjectError> {
    let mut bytes = [0_u8; 16];
    let result =
        unsafe { libc::getentropy(bytes.as_mut_ptr().cast::<libc::c_void>(), bytes.len()) };
    if result != 0 {
        return Err(ProjectError::new(
            "runtime_run_id_failed",
            format!(
                "Cannot obtain native randomness for runtime run ID: {}",
                std::io::Error::last_os_error()
            ),
            None,
        ));
    }
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(not(unix))]
fn native_runtime_run_id() -> Result<String, ProjectError> {
    use std::sync::atomic::AtomicU64;
    use std::time::{SystemTime, UNIX_EPOCH};
    static FALLBACK_RUN_COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = FALLBACK_RUN_COUNTER.fetch_add(1, Ordering::AcqRel);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(fallback_runtime_run_id(std::process::id(), counter, now))
}

#[cfg(any(not(unix), test))]
fn fallback_runtime_run_id(process_id: u32, counter: u64, timestamp_nanos: u128) -> String {
    let digest = Sha256::digest(format!("{process_id}:{counter}:{timestamp_nanos}").as_bytes());
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn runtime_process_failure(result: &mut RuntimeResult, error: ProcessRunFailure) {
    result.outcome = match error.kind {
        ProcessRunFailureKind::CancelledBeforeSpawn => RuntimeOutcome::Cancelled,
        ProcessRunFailureKind::MissingExecutable => RuntimeOutcome::MissingRuntime,
        ProcessRunFailureKind::Start => RuntimeOutcome::StartFailed,
        ProcessRunFailureKind::Cleanup => RuntimeOutcome::CleanupFailed,
    };
    result.executed = error.kind == ProcessRunFailureKind::Cleanup;
    result.failure = Some(error.message);
}

fn record_runtime_channel_failure(
    failure: &Mutex<Option<String>>,
    cancel: &AtomicBool,
    message: String,
) {
    if let Ok(mut current) = failure.lock() {
        if current.is_none() {
            *current = Some(message);
        }
    }
    cancel.store(true, Ordering::Release);
}

fn apply_runtime_channel_failure(result: &mut RuntimeResult, failure: &Mutex<Option<String>>) {
    let message = failure.lock().ok().and_then(|current| current.clone());
    if let Some(message) = message {
        result.success = false;
        if result.outcome == RuntimeOutcome::CleanupFailed {
            let cleanup_failure = result
                .failure
                .take()
                .unwrap_or_else(|| "Runtime process cleanup failed".into());
            result.failure = Some(format!(
                "{cleanup_failure}; runtime event channel also failed after start: {message}"
            ));
            return;
        }
        result.outcome = RuntimeOutcome::ChannelFailed;
        result.failure = Some(format!(
            "Runtime event channel failed after start: {message}"
        ));
    }
}

fn execute_runtime_with<FV, FR>(
    root: &Path,
    invocation: RuntimeInvocation,
    platform_supported: bool,
    run_version: FV,
    run_live: FR,
) -> RuntimeResult
where
    FV: FnOnce() -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(&[String]) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_runtime_with_argv(
        root,
        invocation,
        platform_supported,
        None,
        run_version,
        run_live,
    )
}

fn execute_runtime_with_argv<FV, FR>(
    root: &Path,
    invocation: RuntimeInvocation,
    platform_supported: bool,
    live_argv: Option<Vec<String>>,
    run_version: FV,
    run_live: FR,
) -> RuntimeResult
where
    FV: FnOnce() -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(&[String]) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_runtime_with_argv_and_timeout(
        root,
        invocation,
        platform_supported,
        live_argv,
        RUNTIME_TIMEOUT,
        run_version,
        |argv, _| run_live(argv),
    )
}

fn execute_bounded_task_runtime_with<FV, FR>(
    root: &Path,
    invocation: RuntimeInvocation,
    platform_supported: bool,
    live_argv: Option<Vec<String>>,
    run_version: FV,
    run_live: FR,
) -> RuntimeResult
where
    FV: FnOnce() -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(&[String], Duration) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_runtime_with_argv_and_timeout(
        root,
        invocation,
        platform_supported,
        live_argv,
        BOUNDED_TASK_RUNTIME_TIMEOUT,
        run_version,
        run_live,
    )
}

fn execute_generic_runtime_with<FV, FR>(
    root: &Path,
    invocation: RuntimeInvocation,
    platform_supported: bool,
    run_version: FV,
    run_live: FR,
) -> RuntimeResult
where
    FV: FnOnce() -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(&[String], Duration) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_runtime_with_argv_and_timeout(
        root,
        invocation,
        platform_supported,
        None,
        RUNTIME_TIMEOUT,
        run_version,
        run_live,
    )
}

fn execute_runtime_with_argv_and_timeout<FV, FR>(
    root: &Path,
    invocation: RuntimeInvocation,
    platform_supported: bool,
    live_argv: Option<Vec<String>>,
    execution_timeout: Duration,
    run_version: FV,
    run_live: FR,
) -> RuntimeResult
where
    FV: FnOnce() -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(&[String], Duration) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    if invocation.mode == RuntimeMode::Fixture {
        let mut result = runtime_result(root, invocation.mode, platform_supported, Vec::new());
        result.outcome = RuntimeOutcome::Completed;
        result.success = true;
        result.stdout = encode_payload(CODEX_FIXTURE_JSONL.as_bytes());
        result.events = parse_runtime_jsonl(CODEX_FIXTURE_JSONL.as_bytes(), "fixture").events;
        result.provenance.runtime_version = Some("fixture-schema 1".into());
        return result;
    }

    let prompt = match validated_runtime_prompt(invocation.prompt.as_deref()) {
        Ok(prompt) => prompt,
        Err(error) => {
            let mut result = runtime_result(root, invocation.mode, platform_supported, Vec::new());
            result.outcome = RuntimeOutcome::InvalidPrompt;
            result.failure = Some(error);
            return result;
        }
    };
    let argv = live_argv.unwrap_or_else(|| runtime_argv(root, prompt));
    let mut result = runtime_result(root, invocation.mode, platform_supported, argv);
    if !invocation.confirmed {
        result.outcome = RuntimeOutcome::ConfirmationRequired;
        result.failure = Some("Live runtime execution requires explicit user confirmation".into());
        return result;
    }
    if !platform_supported {
        result.outcome = RuntimeOutcome::UnsupportedPlatform;
        result.failure = Some("Codex runtime timeout and cancellation are supported only on Unix platforms; no process was started".into());
        return result;
    }

    let version = match run_version() {
        Ok(output)
            if output.termination == ProcessTermination::Completed
                && output.status.success()
                && !output.stdout_truncated
                && !output.stderr_truncated =>
        {
            match std::str::from_utf8(&output.stdout) {
                Ok(version) => version.trim().to_string(),
                Err(_) => {
                    result.outcome = RuntimeOutcome::CapabilityUnavailable;
                    result.failure = Some("Codex runtime version is not valid UTF-8".into());
                    return result;
                }
            }
        }
        Ok(output) if output.termination == ProcessTermination::Cancelled => {
            result.outcome = RuntimeOutcome::Cancelled;
            result.failure =
                Some("Codex runtime was cancelled during the version capability probe".into());
            return result;
        }
        Ok(output) if output.termination == ProcessTermination::TimedOut => {
            result.outcome = RuntimeOutcome::TimedOut;
            result.failure = Some(format!(
                "Codex runtime version probe exceeded the {} second limit",
                RUNTIME_VERSION_TIMEOUT.as_secs()
            ));
            return result;
        }
        Ok(output) if output.stdout_truncated || output.stderr_truncated => {
            result.outcome = RuntimeOutcome::OutputOverflow;
            result.failure =
                Some("Codex runtime version output exceeded the bounded capture limit".into());
            return result;
        }
        Ok(output) => {
            result.outcome = RuntimeOutcome::CapabilityUnavailable;
            result.failure = Some(format!(
                "Cannot verify Codex runtime capability: version probe exited {:?}",
                output.status.code()
            ));
            return result;
        }
        Err(error) => {
            runtime_process_failure(&mut result, error);
            // A cleanup failure here belongs to `codex --version`, not the
            // confirmed main `codex exec` process represented by `executed`.
            result.executed = false;
            return result;
        }
    };
    if !version.starts_with("codex-cli ") {
        result.outcome = RuntimeOutcome::CapabilityUnavailable;
        result.failure =
            Some("Codex runtime version probe returned an unsupported identity".into());
        return result;
    }
    result.provenance.runtime_version = Some(version);

    match run_live(&result.provenance.argv, execution_timeout) {
        Ok(output) => {
            result.executed = true;
            result.exit_status = output.status.code();
            result.stdout = encode_payload(&output.stdout);
            result.stderr = encode_payload(&output.stderr);
            result.stdout_truncated = output.stdout_truncated;
            result.stderr_truncated = output.stderr_truncated;
            let parsed = parse_runtime_jsonl(&output.stdout, "provider");
            let malformed = parsed.malformed;
            let provider_failed = parsed.provider_failed;
            let turn_completed = parsed.turn_completed;
            let mut events = parsed.events;
            add_runtime_stderr_event(&mut events, &output.stderr, "provider");
            result.events = events;
            if output.termination == ProcessTermination::Cancelled {
                result.outcome = RuntimeOutcome::Cancelled;
                result.failure =
                    Some("Codex runtime was cancelled and its process group was reaped".into());
            } else if output.termination == ProcessTermination::TimedOut {
                result.outcome = RuntimeOutcome::TimedOut;
                result.failure = Some(format!(
                    "Codex runtime exceeded the {} second execution limit",
                    execution_timeout.as_secs()
                ));
            } else if output.stdout_truncated || output.stderr_truncated {
                result.outcome = RuntimeOutcome::OutputOverflow;
                result.failure =
                    Some("Codex runtime output exceeded the bounded capture limit".into());
            } else if !output.status.success() {
                result.outcome = RuntimeOutcome::NonzeroExit;
                result.failure = Some(format!(
                    "Codex runtime exited with status {}",
                    result
                        .exit_status
                        .map_or_else(|| "signal".into(), |status| status.to_string())
                ));
            } else if malformed || result.events.is_empty() {
                result.outcome = RuntimeOutcome::MalformedOutput;
                result.failure = Some("Codex runtime returned malformed or empty JSONL".into());
            } else if provider_failed {
                result.outcome = RuntimeOutcome::ProviderError;
                result.failure =
                    Some("Codex runtime emitted a provider terminal failure event".into());
            } else if !turn_completed {
                result.outcome = RuntimeOutcome::MalformedOutput;
                result.failure =
                    Some("Codex runtime exited without a turn.completed terminal event".into());
            } else {
                result.outcome = RuntimeOutcome::Completed;
                result.success = true;
            }
        }
        Err(error) => runtime_process_failure(&mut result, error),
    }
    result
}

#[tauri::command]
fn execute_runtime(
    root: String,
    invocation: RuntimeInvocation,
    on_event: Channel<RuntimeStreamMessage>,
) -> Result<RuntimeResult, ProjectError> {
    execute_runtime_command_with(
        root,
        invocation,
        on_event,
        |root, cancel| {
            run_codex_process(
                &["--version".into()],
                root,
                RUNTIME_VERSION_TIMEOUT,
                cancel,
                None,
                SKILL_SETUP_OUTPUT_LIMIT,
            )
        },
        |root, argv, timeout, cancel, handler| {
            run_codex_process(argv, root, timeout, cancel, handler, RUNTIME_OUTPUT_LIMIT)
        },
    )
}

fn execute_runtime_command_with<FV, FR>(
    root: String,
    invocation: RuntimeInvocation,
    on_event: Channel<RuntimeStreamMessage>,
    run_version: FV,
    run_live: FR,
) -> Result<RuntimeResult, ProjectError>
where
    FV: FnOnce(&Path, &AtomicBool) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(
        &Path,
        &[String],
        Duration,
        &AtomicBool,
        Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    ) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    let root = if invocation.mode == RuntimeMode::Fixture {
        canonical_root(&root).map_err(|message| {
            ProjectError::new("invalid_project_root", message, Some(Path::new(&root)))
        })?
    } else {
        validated_repository_root(&root)?
    };
    let run_id = native_runtime_run_id()?;
    let registry = operation_registry();
    let lease = registry.begin(&root, OperationKind::Runtime, Some(run_id.clone()))?;
    let initial_argv = invocation
        .prompt
        .as_deref()
        .filter(|_| invocation.mode == RuntimeMode::Live)
        .map(|prompt| runtime_argv(&root, prompt))
        .unwrap_or_default();
    let initial = runtime_result(&root, invocation.mode, cfg!(unix), initial_argv);
    on_event
        .send(RuntimeStreamMessage::Started {
            handle: RuntimeRunHandle {
                run_id: run_id.clone(),
                capabilities: initial.capabilities,
                provenance: initial.provenance,
            },
        })
        .map_err(|error| {
            ProjectError::new(
                "runtime_channel_failed",
                format!("Cannot deliver native runtime handle: {error}"),
                Some(&root),
            )
        })?;
    let channel_failure = Arc::new(Mutex::new(None));

    let mut result = if invocation.mode == RuntimeMode::Fixture {
        execute_runtime_with(
            &root,
            invocation,
            cfg!(unix),
            || unreachable!("fixture runtime must not probe the Codex executable"),
            |_| unreachable!("fixture runtime must not spawn Codex"),
        )
    } else {
        let stream_channel = on_event.clone();
        let stream_run_id = run_id.clone();
        let sequence = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let stream_failure = Arc::clone(&channel_failure);
        let stream_cancel = Arc::clone(&lease.cancel);
        execute_generic_runtime_with(
            &root,
            invocation,
            cfg!(unix),
            || run_version(&root, &lease.cancel),
            |argv, timeout| {
                let sequence = Arc::clone(&sequence);
                let handler = Arc::new(move |line: &[u8]| {
                    let next = sequence.fetch_add(1, Ordering::AcqRel);
                    let event = parse_runtime_line(line, next, "provider").0;
                    if let Err(error) = stream_channel.send(RuntimeStreamMessage::Event {
                        run_id: stream_run_id.clone(),
                        event,
                    }) {
                        record_runtime_channel_failure(
                            &stream_failure,
                            &stream_cancel,
                            error.to_string(),
                        );
                    }
                });
                run_live(&root, argv, timeout, &lease.cancel, Some(handler))
            },
        )
    };
    result.run_id = run_id.clone();
    if result.provenance.simulated {
        for event in &result.events {
            if let Err(error) = on_event.send(RuntimeStreamMessage::Event {
                run_id: run_id.clone(),
                event: event.clone(),
            }) {
                record_runtime_channel_failure(&channel_failure, &lease.cancel, error.to_string());
                break;
            }
        }
    } else if let Some(event) = result
        .events
        .iter()
        .find(|event| event.kind == RuntimeEventKind::Stderr)
    {
        if let Err(error) = on_event.send(RuntimeStreamMessage::Event {
            run_id: run_id.clone(),
            event: event.clone(),
        }) {
            record_runtime_channel_failure(&channel_failure, &lease.cancel, error.to_string());
        }
    }
    apply_runtime_channel_failure(&mut result, &channel_failure);
    Ok(result)
}

#[tauri::command]
fn cancel_runtime(run_id: String) -> Result<RuntimeCancellation, ProjectError> {
    if run_id.len() != 32 || !run_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProjectError::new(
            "invalid_runtime_run_id",
            "Runtime cancellation requires a native-issued run ID",
            None,
        ));
    }
    let requested = operation_registry().cancel_run(&run_id)?;
    Ok(RuntimeCancellation {
        cancellation_requested: requested,
        message: if requested {
            "Cancellation requested; waiting for the Codex process group to stop".into()
        } else {
            "No runtime is currently running for this project".into()
        },
    })
}

fn task_has_repository_verification(task: &str) -> bool {
    let status_complete = markdown_field(task, "Status:")
        .is_some_and(|status| status.eq_ignore_ascii_case("complete"));
    let criteria = markdown_section(task, "Acceptance Criteria");
    let checklist = criteria
        .lines()
        .filter(|line| line.trim_start().starts_with("- ["))
        .collect::<Vec<_>>();
    let all_checked = !checklist.is_empty()
        && checklist.iter().all(|line| {
            let line = line.trim_start();
            line.starts_with("- [x]") || line.starts_with("- [X]")
        });
    let evidence = markdown_section(task, "Evidence Log");
    let verification = markdown_section(task, "Verification Summary");
    status_complete
        && all_checked
        && evidence.contains("| pass |")
        && !verification.is_empty()
        && !verification.to_ascii_lowercase().contains("not run yet")
}

fn helper_outcome_code(outcome: HelperOutcome) -> &'static str {
    match outcome {
        HelperOutcome::Completed => "completed",
        HelperOutcome::NonzeroExit => "nonzeroExit",
        HelperOutcome::MalformedOutput => "malformedOutput",
        HelperOutcome::VerificationFailed => "verificationFailed",
        HelperOutcome::OutputOverflow => "outputOverflow",
        HelperOutcome::Cancelled => "cancelled",
        HelperOutcome::TimedOut => "timedOut",
        HelperOutcome::MissingRuntime => "missingRuntime",
        HelperOutcome::StartFailed => "startFailed",
        HelperOutcome::CleanupFailed => "cleanupFailed",
        HelperOutcome::UnsupportedPlatform => "unsupportedPlatform",
    }
}

fn classify_bounded_task(
    runtime: &RuntimeResult,
    task_text: &str,
    resolver: &HelperResult,
    stop_gates: &HelperResult,
    refresh_failures: &[BoundedTaskRefreshFailure],
) -> (BoundedTaskOutcome, bool, String) {
    let resolver_decision = resolver
        .decision
        .as_ref()
        .map(|decision| decision.decision.as_str())
        .unwrap_or("invalid-state");
    if !runtime.success {
        return (
            BoundedTaskOutcome::Stopped,
            false,
            format!("Runtime stopped with {:?}; repository was refreshed and no other task was selected", runtime.outcome),
        );
    }
    if !refresh_failures.is_empty() {
        return (
            BoundedTaskOutcome::Stopped,
            false,
            format!(
                "Required repository refresh failed: {}",
                refresh_failures
                    .iter()
                    .map(|failure| format!("{}/{}", failure.surface, failure.code))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    if !resolver.success {
        return (
            BoundedTaskOutcome::Stopped,
            false,
            format!(
                "Resolver refresh stopped with {}",
                helper_outcome_code(resolver.outcome)
            ),
        );
    }
    if !stop_gates.success {
        return (
            BoundedTaskOutcome::Stopped,
            false,
            format!(
                "Stop-gates refresh stopped with {}",
                helper_outcome_code(stop_gates.outcome)
            ),
        );
    }
    if resolver_decision == "wait-external" {
        return (
            BoundedTaskOutcome::WaitExternal,
            false,
            resolver
                .decision
                .as_ref()
                .map(|decision| decision.next_action.clone())
                .unwrap_or_else(|| "External state is required".into()),
        );
    }
    if !task_has_repository_verification(task_text) {
        return (
            BoundedTaskOutcome::VerificationFailed,
            false,
            "Fully refreshed task status, acceptance criteria, evidence log, or verification summary did not prove completion; provider claims were ignored".into(),
        );
    }
    let resolver_clear = resolver
        .decision
        .as_ref()
        .is_some_and(|decision| decision.warnings.is_empty());
    let completed_stop_terminal = stop_gates.success
        && stop_gates.decision.as_ref().is_some_and(|decision| {
            decision.decision == "stop"
                && decision.confidence == "medium"
                && decision.warnings == ["selected task status is complete"]
        });
    if resolver_clear && completed_stop_terminal {
        (
            BoundedTaskOutcome::Verified,
            true,
            "Repository task status, acceptance criteria, evidence log, verification summary, resolver, and stop gates passed".into(),
        )
    } else {
        (
            BoundedTaskOutcome::Stopped,
            false,
            "Required resolver or stop-gates terminal did not match the completed-task contract"
                .into(),
        )
    }
}

fn resolver_blocking_gates(resolver: &HelperResult) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&resolver.stdout) else {
        return Vec::new();
    };
    value
        .get("blockingGates")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|gate| {
            let source = gate.get("source")?.as_str()?;
            let reason = gate.get("reason")?.as_str()?;
            Some(format!("{source}: {reason}"))
        })
        .collect()
}

fn result_loop_projection(
    root: &Path,
    outcome: BoundedTaskOutcome,
    repository_verified: bool,
    reason: &str,
    project: &ProjectSnapshot,
    selected_task: Option<&str>,
    resolver: Option<&HelperResult>,
    cancellation_phase: Option<BoundedTaskCancellationPhase>,
) -> GoalLoopProjection {
    let resolver_gates = resolver.map(resolver_blocking_gates).unwrap_or_default();
    let terminal = |state, reason: String| GoalLoopProjection {
        state,
        next_task: None,
        blocking_gates: resolver_gates.clone(),
        expected_effects: Vec::new(),
        explicit_confirmation_required: false,
        automatic_execution_started: false,
        reason,
    };

    if let Some(cancellation_phase) = cancellation_phase {
        return terminal(
            GoalLoopState::CancelledStop,
            cancellation_phase.projection_reason(reason),
        );
    }
    if reason.contains("changed after preview")
        || reason.contains("stale")
        || reason.contains("task source changed")
    {
        return terminal(GoalLoopState::StaleStop, reason.into());
    }
    if repository_verified {
        if selected_task
            .and_then(|task| terminal_tracker_reference(root, project, task))
            .is_some()
        {
            return terminal(
                GoalLoopState::GoalComplete,
                "Repository task and terminal tracker evidence affirm goal completion".into(),
            );
        }
        if let Some(resolver) = resolver {
            let decision = resolver
                .decision
                .as_ref()
                .map(|decision| decision.decision.as_str())
                .unwrap_or("invalid-state");
            if decision == "execute-task" {
                if !resolver_gates.is_empty() {
                    return terminal(
                        resolver_stop_state(decision, &resolver_gates),
                        "The refreshed resolver reported blocking gates; continuation is forbidden"
                            .into(),
                    );
                }
                if let Some(next_task) = resolver_next_task(resolver) {
                    return GoalLoopProjection {
                        state: GoalLoopState::ContinueAvailable,
                        next_task: Some(next_task),
                        blocking_gates: resolver_gates.clone(),
                        expected_effects: goal_loop_effects(),
                        explicit_confirmation_required: true,
                        automatic_execution_started: false,
                        reason: "The verified checkpoint was recorded and the refreshed resolver selected one next ready AI-owned task".into(),
                    };
                }
                return terminal(
                    GoalLoopState::InvalidStateStop,
                    "The refreshed resolver did not provide one valid ready AI-owned next task"
                        .into(),
                );
            }
            return terminal(
                resolver_stop_state(decision, &resolver_gates),
                resolver
                    .decision
                    .as_ref()
                    .map(|decision| decision.next_action.clone())
                    .unwrap_or_else(|| reason.into()),
            );
        }
        return terminal(
            GoalLoopState::FailureStop,
            "The verified task did not produce a usable refreshed resolver result".into(),
        );
    }
    if let Some(resolver) = resolver {
        let decision = resolver
            .decision
            .as_ref()
            .map(|decision| decision.decision.as_str())
            .unwrap_or("invalid-state");
        if matches!(
            decision,
            "ask-founder" | "wait-external" | "no-ready-task" | "invalid-state"
        ) || !resolver_gates.is_empty()
        {
            return terminal(
                resolver_stop_state(decision, &resolver_gates),
                reason.into(),
            );
        }
    }
    terminal(
        if outcome == BoundedTaskOutcome::WaitExternal {
            GoalLoopState::ExternalStop
        } else {
            GoalLoopState::FailureStop
        },
        reason.into(),
    )
}

fn finish_bounded_task_with<F>(
    root: &Path,
    selected_task: Option<String>,
    runtime: Option<RuntimeResult>,
    initial_reason: Option<String>,
    initial_cancellation_phase: Option<BoundedTaskCancellationPhase>,
    cancel: &AtomicBool,
    explicit_user_cancellation: &AtomicBool,
    run_helper: &mut F,
) -> BoundedTaskResult
where
    F: FnMut(&Path, HelperInvocation, &AtomicBool) -> Result<HelperResult, ProjectError>,
{
    let mut refresh_failures = Vec::new();
    let project = inspect_project_path(root);
    for error in &project.errors {
        refresh_failures.push(BoundedTaskRefreshFailure {
            surface: if error.code.starts_with("git_") {
                "git"
            } else {
                "snapshot"
            }
            .into(),
            code: error.code.clone(),
            message: error.message.clone(),
        });
    }
    let task_evidence =
        selected_task
            .as_deref()
            .and_then(|path| match read_controller_task(root, path) {
                Ok(task) => Some(task),
                Err(error) => {
                    refresh_failures.push(BoundedTaskRefreshFailure {
                        surface: "taskEvidence".into(),
                        code: error.code,
                        message: error.message,
                    });
                    None
                }
            });
    let resolver = if cancel.load(Ordering::Acquire) {
        refresh_failures.push(BoundedTaskRefreshFailure {
            surface: "resolver".into(),
            code: "refresh_cancelled".into(),
            message: "Cancellation was already requested; post-exit resolver was not launched"
                .into(),
        });
        None
    } else {
        match run_helper(
            root,
            HelperInvocation {
                helper_id: HelperId::ContinueCheck,
                mode: None,
                task_path: None,
                feature_request: None,
            },
            cancel,
        ) {
            Ok(result) => {
                if !result.success {
                    refresh_failures.push(BoundedTaskRefreshFailure {
                        surface: "resolver".into(),
                        code: helper_outcome_code(result.outcome).into(),
                        message: result
                            .failure
                            .clone()
                            .unwrap_or_else(|| "Resolver refresh failed".into()),
                    });
                }
                Some(result)
            }
            Err(error) => {
                refresh_failures.push(BoundedTaskRefreshFailure {
                    surface: "resolver".into(),
                    code: error.code,
                    message: error.message,
                });
                None
            }
        }
    };
    let stop_gates = if selected_task.is_none() {
        None
    } else if cancel.load(Ordering::Acquire) {
        refresh_failures.push(BoundedTaskRefreshFailure {
            surface: "stopGates".into(),
            code: "refresh_cancelled".into(),
            message: "Cancellation was requested; post-exit stop-gates helper was not launched"
                .into(),
        });
        None
    } else {
        let path = selected_task.as_ref().expect("selected task checked above");
        match run_helper(
            root,
            HelperInvocation {
                helper_id: HelperId::ExecutionCheck,
                mode: Some(HelperExecutionMode::StopGates),
                task_path: Some(path.clone()),
                feature_request: None,
            },
            cancel,
        ) {
            Ok(result) => {
                if !result.success {
                    refresh_failures.push(BoundedTaskRefreshFailure {
                        surface: "stopGates".into(),
                        code: helper_outcome_code(result.outcome).into(),
                        message: result
                            .failure
                            .clone()
                            .unwrap_or_else(|| "Stop-gates refresh failed".into()),
                    });
                }
                Some(result)
            }
            Err(error) => {
                refresh_failures.push(BoundedTaskRefreshFailure {
                    surface: "stopGates".into(),
                    code: error.code,
                    message: error.message,
                });
                None
            }
        }
    };
    let (outcome, repository_verified, reason) = match (
        runtime.as_ref(),
        task_evidence.as_ref(),
        resolver.as_ref(),
        stop_gates.as_ref(),
    ) {
        (Some(runtime), Some(task), Some(resolver), Some(stop_gates)) => classify_bounded_task(
            runtime,
            &task.content,
            resolver,
            stop_gates,
            &refresh_failures,
        ),
        _ => (
            BoundedTaskOutcome::Stopped,
            false,
            initial_reason.unwrap_or_else(|| {
                "Controller stopped with partial repository evidence; no other task was selected"
                    .into()
            }),
        ),
    };
    let cancellation_phase = initial_cancellation_phase
        .or_else(|| {
            runtime
                .as_ref()
                .is_some_and(|runtime| runtime.outcome == RuntimeOutcome::Cancelled)
                .then_some(BoundedTaskCancellationPhase::ProviderRuntime)
        })
        .or_else(|| {
            explicit_user_cancellation
                .load(Ordering::Acquire)
                .then_some(BoundedTaskCancellationPhase::PostExitRefresh)
        });
    let loop_state = result_loop_projection(
        root,
        outcome,
        repository_verified,
        &reason,
        &project,
        selected_task.as_deref(),
        resolver.as_ref(),
        cancellation_phase,
    );
    BoundedTaskResult {
        outcome,
        selected_task,
        runtime,
        project,
        task_evidence,
        resolver,
        stop_gates,
        refresh_failures,
        repository_verified,
        reason,
        loop_state,
    }
}

fn finalize_bounded_task_result<F>(
    lease: &OperationLease,
    root: &Path,
    mut result: BoundedTaskResult,
    cancellation_phase: BoundedTaskCancellationPhase,
    commit_verified: F,
) -> Result<BoundedTaskResult, ProjectError>
where
    F: FnOnce(&BoundedTaskResult) -> Result<(), ProjectError>,
{
    let finalization = lease.finalize_bounded_task(|| {
        if result.repository_verified {
            commit_verified(&result)?;
        }
        Ok(())
    })?;
    if finalization == BoundedTaskFinalizationDecision::CancellationAccepted {
        result.outcome = BoundedTaskOutcome::Stopped;
        result.repository_verified = false;
        if result.loop_state.state != GoalLoopState::CancelledStop {
            result.loop_state = result_loop_projection(
                root,
                result.outcome,
                false,
                &result.reason,
                &result.project,
                result.selected_task.as_deref(),
                result.resolver.as_ref(),
                Some(cancellation_phase),
            );
        }
        result.reason = result.loop_state.reason.clone();
    }
    Ok(result)
}

#[cfg(test)]
fn finish_bounded_task(
    root: &Path,
    selected_task: Option<String>,
    runtime: Option<RuntimeResult>,
    initial_reason: Option<String>,
    cancel: &AtomicBool,
) -> BoundedTaskResult {
    let initial_cancellation_phase = (runtime.is_none() && cancel.load(Ordering::Acquire))
        .then_some(BoundedTaskCancellationPhase::PreRunRevalidation);
    finish_bounded_task_with(
        root,
        selected_task,
        runtime,
        initial_reason,
        initial_cancellation_phase,
        cancel,
        cancel,
        &mut controller_helper,
    )
}

#[tauri::command]
async fn execute_bounded_task(
    app: tauri::AppHandle,
    root: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
) -> Result<BoundedTaskResult, ProjectError> {
    let storage = app_goal_storage(&app)?;
    let error_root = PathBuf::from(&root);
    // Process polling may last for the full controller timeout. Running it on Tauri's command
    // dispatch thread would also block WebView event delivery and cancellation commands.
    run_bounded_task_worker(error_root, move || {
        execute_bounded_task_command_with_storage(
            root,
            invocation,
            on_event,
            Some(storage),
            operation_registry(),
            |root, cancel| {
                run_codex_process(
                    &["--version".into()],
                    root,
                    RUNTIME_VERSION_TIMEOUT,
                    cancel,
                    None,
                    SKILL_SETUP_OUTPUT_LIMIT,
                )
            },
            |root, argv, timeout, cancel, handler| {
                run_codex_process(argv, root, timeout, cancel, handler, RUNTIME_OUTPUT_LIMIT)
            },
        )
    })
    .await
}

fn claimed_pre_spawn_repair_classification(
    execution: &Result<BoundedTaskResult, ProjectError>,
) -> (CollaborationFailureClass, SharedClaimRepairCause) {
    match execution {
        Err(error) if error.code.contains("cancel") => (
            CollaborationFailureClass::Cancelled,
            SharedClaimRepairCause::Cancellation,
        ),
        Err(error) if error.code.starts_with("goal_") => (
            CollaborationFailureClass::RepairRequired,
            SharedClaimRepairCause::GoalStorage,
        ),
        Err(_) => (
            CollaborationFailureClass::RepairRequired,
            SharedClaimRepairCause::ControllerFinalization,
        ),
        Ok(bounded)
            if bounded.loop_state.state == GoalLoopState::CancelledStop
                || bounded
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.outcome == RuntimeOutcome::Cancelled) =>
        {
            (
                CollaborationFailureClass::Cancelled,
                SharedClaimRepairCause::Cancellation,
            )
        }
        Ok(bounded) => match bounded.runtime.as_ref() {
            Some(runtime)
                if matches!(
                    runtime.outcome,
                    RuntimeOutcome::CleanupFailed
                        | RuntimeOutcome::CapabilityUnavailable
                        | RuntimeOutcome::UnsupportedPlatform
                        | RuntimeOutcome::OutputOverflow
                        | RuntimeOutcome::TimedOut
                ) =>
            {
                (
                    CollaborationFailureClass::RepairRequired,
                    SharedClaimRepairCause::RuntimeCapability,
                )
            }
            Some(runtime)
                if matches!(
                    runtime.outcome,
                    RuntimeOutcome::StartFailed
                        | RuntimeOutcome::MissingRuntime
                        | RuntimeOutcome::ConfirmationRequired
                        | RuntimeOutcome::InvalidPrompt
                ) && runtime.provenance.runtime_version.is_some() =>
            {
                (
                    CollaborationFailureClass::RepairRequired,
                    SharedClaimRepairCause::RuntimeStart,
                )
            }
            Some(runtime)
                if matches!(
                    runtime.outcome,
                    RuntimeOutcome::StartFailed
                        | RuntimeOutcome::MissingRuntime
                        | RuntimeOutcome::ConfirmationRequired
                        | RuntimeOutcome::InvalidPrompt
                ) =>
            {
                (
                    CollaborationFailureClass::RepairRequired,
                    SharedClaimRepairCause::RuntimeCapability,
                )
            }
            _ => (
                CollaborationFailureClass::RepairRequired,
                SharedClaimRepairCause::ControllerFinalization,
            ),
        },
    }
}

fn execute_shared_bounded_task_with_claim_port<FV, FR, FH, P>(
    root: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
    storage: Option<GoalStateStorage>,
    registry: Arc<OperationRegistry>,
    binding: SharedExecutionBinding,
    run_version: FV,
    run_live: FR,
    run_helper: FH,
    port: &P,
) -> Result<SharedBoundedTaskResult, ProjectError>
where
    FV: FnOnce(&Path, &AtomicBool) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(
        &Path,
        &[String],
        Duration,
        &AtomicBool,
        Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    ) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FH: FnMut(&Path, HelperInvocation, &AtomicBool) -> Result<HelperResult, ProjectError>,
    P: SharedClaimPort,
{
    let codex_started = Arc::new(AtomicBool::new(false));
    let observed_codex_started = Arc::clone(&codex_started);
    let policy = ControllerCollaborationPolicy {
        mode: CollaborationMode::SharedCollaborator,
        session: Some(binding.session.clone()),
        remote: Some(binding.remote.clone()),
    };
    let mut execution = execute_bounded_task_command_with_storage_helper_and_collaboration(
        root,
        invocation,
        on_event,
        storage,
        registry,
        run_version,
        move |root, argv, timeout, cancel, handler| {
            let result = run_live(root, argv, timeout, cancel, handler);
            let process_started = match &result {
                Ok(_) => true,
                Err(error) => error.kind == ProcessRunFailureKind::Cleanup,
            };
            if process_started {
                observed_codex_started.store(true, Ordering::Release);
            }
            result
        },
        run_helper,
        port,
        policy,
        BoundedTaskConfirmationScope::Shared(binding.clone()),
    );
    let codex_started = codex_started.load(Ordering::Acquire);
    if !codex_started {
        let (failure_class, cause) = claimed_pre_spawn_repair_classification(&execution);
        port.mark_claimed_pre_spawn_repair(failure_class, cause)?;
    }
    let claim = port.shared_claim_state()?;
    let completion = shared_completion_state(port.post_commit_outcome()?);
    let shared_iteration_blocked = matches!(
        &completion,
        SharedCompletionState::CollaborationRepairRequired { .. }
    );
    if shared_iteration_blocked {
        if let Ok(bounded) = &mut execution {
            bounded.loop_state.state = GoalLoopState::FailureStop;
            bounded.loop_state.next_task = None;
            bounded.loop_state.blocking_gates = vec!["collaborationRepairRequired".into()];
            bounded.loop_state.expected_effects.clear();
            bounded.loop_state.explicit_confirmation_required = false;
            bounded.loop_state.automatic_execution_started = false;
            bounded.loop_state.reason =
                "Local completion is authoritative; shared continuation is blocked until explicit collaboration repair"
                    .into();
        }
    }
    match execution {
        Ok(bounded) => Ok(SharedBoundedTaskResult {
            bounded: Some(bounded),
            binding,
            claim,
            completion,
            codex_started,
            stopped_before_runtime: !codex_started,
            shared_iteration_blocked,
            error: None,
        }),
        Err(error) => Ok(SharedBoundedTaskResult {
            bounded: None,
            binding,
            claim,
            completion,
            codex_started,
            stopped_before_runtime: !codex_started,
            shared_iteration_blocked,
            error: Some(error),
        }),
    }
}

#[tauri::command]
async fn execute_shared_bounded_task(
    app: tauri::AppHandle,
    root: String,
    session_id: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
) -> Result<SharedBoundedTaskResult, ProjectError> {
    let root_path = validated_repository_root(&root)?;
    let storage = app_goal_storage(&app)?;
    ensure_shared_iteration_has_no_repair_debt(&storage, &root_path)?;
    let registry = operation_registry();
    let binding =
        registry.shared_confirmation_binding(&root_path, &invocation.preview_token, &session_id)?;
    let worker_app = app.clone();
    run_bounded_task_worker(root_path.clone(), move || {
        let sessions = worker_app.state::<MdsyncSessionStore>();
        let port = MdsyncClaimPort::new(
            &sessions,
            root_path.to_string_lossy().to_string(),
            session_id,
            binding.clone(),
            Arc::clone(&registry),
        );
        execute_shared_bounded_task_with_claim_port(
            root,
            invocation,
            on_event,
            Some(storage),
            Arc::clone(&registry),
            binding,
            |root, cancel| {
                run_codex_process(
                    &["--version".into()],
                    root,
                    RUNTIME_VERSION_TIMEOUT,
                    cancel,
                    None,
                    SKILL_SETUP_OUTPUT_LIMIT,
                )
            },
            |root, argv, timeout, cancel, handler| {
                run_codex_process(argv, root, timeout, cancel, handler, RUNTIME_OUTPUT_LIMIT)
            },
            controller_helper,
            &port,
        )
    })
    .await
}

fn verify_repair_local_authority(
    root: &Path,
    intent: &RemoteCompletionIntent,
) -> Result<(), ProjectError> {
    let repository = repository_identity(root)?;
    if repository.repository_id != intent.repository_id {
        return Err(ProjectError::new(
            "collaboration_repair_repository_mismatch",
            "The selected repository no longer matches the durable repair intent",
            Some(root),
        ));
    }
    let task = read_controller_task(root, &intent.local_task_path)?;
    if sha256_bytes(task.content.as_bytes()) != intent.local_task_sha256
        || !task_has_repository_verification(&task.content)
    {
        return Err(ProjectError::new(
            "collaboration_repair_checkpoint_stale",
            "The exact repository-verified task checkpoint is missing or changed",
            Some(root),
        ));
    }
    for artifact in &intent.artifacts {
        let file = read_controller_task(root, &artifact.path)?;
        if sha256_bytes(file.content.as_bytes()) != artifact.sha256 {
            return Err(ProjectError::new(
                "collaboration_repair_artifact_stale",
                "A sanitized checkpoint artifact changed after local verification",
                Some(root),
            ));
        }
    }
    Ok(())
}

fn reconciliation_outcome_from_progress(
    intent: &RemoteCompletionIntent,
    completed_effects: &[MissingCollaborationEffect],
    current_task_version: u64,
    complete: bool,
) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
    if complete {
        Ok(PostLocalCommitCollaborationOutcome {
            reconciliation: ReconciliationState::Reconciled,
            evidence_handoff: EvidenceHandoffResult::Synchronized {
                remote_version: current_task_version,
                evidence_ids: vec![EvidenceReferenceId::parse(intent.evidence_id.clone())?],
                handoff_id: Some(HandoffReferenceId::parse(intent.handoff_id.clone())?),
            },
        })
    } else {
        let missing_effects = MissingCollaborationEffect::ORDER
            .into_iter()
            .filter(|effect| !completed_effects.contains(effect))
            .collect::<Vec<_>>();
        Ok(PostLocalCommitCollaborationOutcome {
            reconciliation: ReconciliationState::RepairRequired,
            evidence_handoff: EvidenceHandoffResult::Partial {
                remote_version: Some(current_task_version),
                missing_effects,
                repair: RepairHint::retry_sync(),
            },
        })
    }
}

#[tauri::command]
fn repair_collaboration_completion(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, MdsyncSessionStore>,
    root: String,
    session_id: String,
    confirmed: bool,
) -> Result<CollaborationRepairResult, ProjectError> {
    let root = validated_repository_root(&root)?;
    if !confirmed {
        return Err(ProjectError::new(
            "collaboration_repair_confirmation_required",
            "Repair requires an explicit action against the reconnected Collaborator session",
            Some(&root),
        ));
    }
    let storage = app_goal_storage(&app)?;
    let record = storage.read_record()?.ok_or_else(|| {
        ProjectError::new(
            "collaboration_repair_debt_missing",
            "No durable collaboration repair debt exists",
            Some(&root),
        )
    })?;
    let cursor = record.collaboration.as_ref().ok_or_else(|| {
        ProjectError::new(
            "collaboration_repair_debt_missing",
            "No durable collaboration repair debt exists",
            Some(&root),
        )
    })?;
    if cursor.state == PersistedReconciliationState::Reconciled {
        return Err(ProjectError::new(
            "collaboration_repair_not_required",
            "The durable collaboration cursor is already reconciled",
            Some(&root),
        ));
    }
    verify_repair_local_authority(&root, &cursor.intent)?;
    let project_key = root.to_string_lossy().to_string();
    let session = sessions
        .sanitized_session_metadata(&project_key, &session_id)
        .map_err(|error| {
            collaboration_project_error(
                &root,
                "repair",
                collaboration_failure_from_transport(&error),
            )
        })?;
    sessions
        .validate_completion_intent_for_persistence(&project_key, &session_id, &cursor.intent)
        .map_err(|error| {
            collaboration_project_error(
                &root,
                "repair",
                collaboration_failure_from_transport(&error),
            )
        })?;
    if session.access != CollaborationAccess::Collaborator
        || session.workspace_id != cursor.intent.workspace_id
        || session.actor != cursor.intent.actor
    {
        return Err(ProjectError::new(
            "collaboration_repair_session_mismatch",
            "Repair requires a reconnected Collaborator session for the exact workspace and actor",
            Some(&root),
        ));
    }
    let files =
        read_remote_workspace_files(&sessions, &project_key, &session_id).map_err(|error| {
            match error {
                Ha2haEnvelopeCommandError::Project(error) => error,
                Ha2haEnvelopeCommandError::Transport(error) => collaboration_project_error(
                    &root,
                    "repair",
                    collaboration_failure_from_transport(&error),
                ),
                Ha2haEnvelopeCommandError::Envelope(error) => ProjectError::new(
                    &format!("collaboration_repair_{}", error.code),
                    error.message,
                    Some(&root),
                ),
            }
        })?;
    let plan = project_post_run_reconciliation(&cursor.intent, &files).map_err(|error| {
        ProjectError::new(
            &format!("collaboration_repair_{}", error.code),
            error.message,
            Some(&root),
        )
    })?;
    let (completed_effects, current_task_version, failure) =
        apply_post_run_write_sequence(&plan, |write| {
            sessions.write_file_with_readback(
                &project_key,
                &session_id,
                MdsyncWriteInput {
                    path: write.path.clone(),
                    content: write.content.clone(),
                    content_type: Some(write.content_type.clone()),
                    base_version: write.base_version,
                },
                write.expected_post_version,
            )
        });
    let outcome = reconciliation_outcome_from_progress(
        &cursor.intent,
        &completed_effects,
        current_task_version,
        failure.is_none(),
    )
    .map_err(|failure| collaboration_project_error(&root, "repair", failure))?;
    storage
        .finish_collaboration_reconciliation(&record, &outcome)
        .map_err(|error| {
            if completed_effects.len() > plan.applied_effects.len() {
                error.after_commit()
            } else {
                error
            }
        })?;
    let completion = shared_completion_state(Some(outcome));
    let shared_iteration_blocked = matches!(
        &completion,
        SharedCompletionState::CollaborationRepairRequired { .. }
    );
    Ok(CollaborationRepairResult {
        completion,
        reconciled_effects: completed_effects,
        explicit_action_consumed: true,
        codex_started: false,
        shared_iteration_blocked,
    })
}

async fn run_bounded_task_worker<T, F>(root: PathBuf, worker: F) -> Result<T, ProjectError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ProjectError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(worker)
        .await
        .map_err(|error| {
            ProjectError::new(
                "controller_worker_failed",
                format!("Bounded task worker failed: {error}"),
                Some(&root),
            )
        })?
}

#[cfg(test)]
fn execute_bounded_task_command_with<FV, FR>(
    root: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
    run_version: FV,
    run_live: FR,
) -> Result<BoundedTaskResult, ProjectError>
where
    FV: FnOnce(&Path, &AtomicBool) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(
        &Path,
        &[String],
        Duration,
        &AtomicBool,
        Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    ) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_bounded_task_command_with_storage(
        root,
        invocation,
        on_event,
        None,
        operation_registry(),
        run_version,
        run_live,
    )
}

fn execute_bounded_task_command_with_storage<FV, FR>(
    root: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
    storage: Option<GoalStateStorage>,
    registry: Arc<OperationRegistry>,
    run_version: FV,
    run_live: FR,
) -> Result<BoundedTaskResult, ProjectError>
where
    FV: FnOnce(&Path, &AtomicBool) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(
        &Path,
        &[String],
        Duration,
        &AtomicBool,
        Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    ) -> Result<BoundedProcessOutput, ProcessRunFailure>,
{
    execute_bounded_task_command_with_storage_and_helper(
        root,
        invocation,
        on_event,
        storage,
        registry,
        run_version,
        run_live,
        controller_helper,
    )
}

fn execute_bounded_task_command_with_storage_and_helper<FV, FR, FH>(
    root: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
    storage: Option<GoalStateStorage>,
    registry: Arc<OperationRegistry>,
    run_version: FV,
    run_live: FR,
    run_helper: FH,
) -> Result<BoundedTaskResult, ProjectError>
where
    FV: FnOnce(&Path, &AtomicBool) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(
        &Path,
        &[String],
        Duration,
        &AtomicBool,
        Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    ) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FH: FnMut(&Path, HelperInvocation, &AtomicBool) -> Result<HelperResult, ProjectError>,
{
    execute_bounded_task_command_with_storage_helper_and_collaboration(
        root,
        invocation,
        on_event,
        storage,
        registry,
        run_version,
        run_live,
        run_helper,
        &NoopCollaborationPort,
        ControllerCollaborationPolicy::default(),
        BoundedTaskConfirmationScope::Local,
    )
}

fn execute_bounded_task_command_with_storage_helper_and_collaboration<FV, FR, FH>(
    root: String,
    invocation: BoundedTaskInvocation,
    on_event: Channel<RuntimeStreamMessage>,
    storage: Option<GoalStateStorage>,
    registry: Arc<OperationRegistry>,
    run_version: FV,
    run_live: FR,
    mut run_helper: FH,
    collaboration_port: &dyn CollaborationPort,
    collaboration_policy: ControllerCollaborationPolicy,
    confirmation_scope: BoundedTaskConfirmationScope,
) -> Result<BoundedTaskResult, ProjectError>
where
    FV: FnOnce(&Path, &AtomicBool) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FR: FnOnce(
        &Path,
        &[String],
        Duration,
        &AtomicBool,
        Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
    ) -> Result<BoundedProcessOutput, ProcessRunFailure>,
    FH: FnMut(&Path, HelperInvocation, &AtomicBool) -> Result<HelperResult, ProjectError>,
{
    let root = validated_repository_root(&root)?;
    if !invocation.confirmed {
        return Err(ProjectError::new(
            "controller_confirmation_required",
            "Bounded task execution requires explicit confirmation",
            Some(&root),
        ));
    }
    let run_id = native_runtime_run_id()?;
    let lease = registry.begin(&root, OperationKind::BoundedTask, Some(run_id.clone()))?;
    let baseline_preview_token = registry.consume_bounded_task_confirmation(
        &root,
        &invocation.preview_token,
        &confirmation_scope,
    )?;
    let expected_goal = storage
        .as_ref()
        .map(GoalStateStorage::read_record)
        .transpose()?
        .flatten();
    let initial = runtime_result(&root, invocation.mode, cfg!(unix), Vec::new());
    on_event
        .send(RuntimeStreamMessage::Started {
            handle: RuntimeRunHandle {
                run_id: run_id.clone(),
                capabilities: initial.capabilities,
                provenance: initial.provenance,
            },
        })
        .map_err(|error| {
            ProjectError::new(
                "runtime_channel_failed",
                format!("Cannot deliver bounded task run handle: {error}"),
                Some(&root),
            )
        })?;

    let preview = match build_bounded_task_preview_with(&root, &lease.cancel, &mut run_helper) {
        Ok(preview) => preview,
        Err(error) => {
            let cancellation_phase = lease
                .explicit_user_cancellation
                .load(Ordering::Acquire)
                .then_some(BoundedTaskCancellationPhase::PreRunRevalidation);
            let result = finish_bounded_task_with(
                &root,
                Some(invocation.selected_task),
                None,
                Some(format!("{}: {}", error.code, error.message)),
                cancellation_phase,
                &lease.cancel,
                &lease.explicit_user_cancellation,
                &mut run_helper,
            );
            return finalize_bounded_task_result(
                &lease,
                &root,
                result,
                BoundedTaskCancellationPhase::PreRunRevalidation,
                |_| Ok(()),
            );
        }
    };
    let Some(selected_task) = preview.selected_task.clone() else {
        let cancellation_phase = lease
            .explicit_user_cancellation
            .load(Ordering::Acquire)
            .then_some(BoundedTaskCancellationPhase::PreRunRevalidation);
        let result = finish_bounded_task_with(
            &root,
            None,
            None,
            Some(format!(
                "Resolver decision {}: {}",
                preview.decision, preview.next_action
            )),
            cancellation_phase,
            &lease.cancel,
            &lease.explicit_user_cancellation,
            &mut run_helper,
        );
        return finalize_bounded_task_result(
            &lease,
            &root,
            result,
            BoundedTaskCancellationPhase::PreRunRevalidation,
            |_| Ok(()),
        );
    };
    if preview.preview_token != baseline_preview_token
        || selected_task != invocation.selected_task
        || !preview.executable
    {
        let cancellation_phase = lease
            .explicit_user_cancellation
            .load(Ordering::Acquire)
            .then_some(BoundedTaskCancellationPhase::PreRunRevalidation);
        let result = finish_bounded_task_with(
            &root,
            Some(selected_task),
            None,
            Some("Resolver, task contract, selected task, or task source changed after preview; no provider process was started".into()),
            cancellation_phase,
            &lease.cancel,
            &lease.explicit_user_cancellation,
            &mut run_helper,
        );
        return finalize_bounded_task_result(
            &lease,
            &root,
            result,
            BoundedTaskCancellationPhase::PreRunRevalidation,
            |_| Ok(()),
        );
    }
    let preview_task = read_controller_task(&root, &selected_task)?;
    let pre_run_collaboration = run_pre_run_collaboration_hook(
        collaboration_port,
        &collaboration_policy,
        &root,
        &selected_task,
        &preview_task.content,
        &lease.cancel,
    )?;
    let shared_completion_binding = match &confirmation_scope {
        BoundedTaskConfirmationScope::Shared(binding) => Some(binding.clone()),
        BoundedTaskConfirmationScope::Local => None,
    };
    if lease.cancel.load(Ordering::Acquire) {
        let result = finish_bounded_task_with(
            &root,
            Some(selected_task),
            None,
            Some(
                "Execution was cancelled after collaboration finalization and before any provider process started"
                    .into(),
            ),
            Some(BoundedTaskCancellationPhase::PreRunRevalidation),
            &lease.cancel,
            &lease.explicit_user_cancellation,
            &mut run_helper,
        );
        return finalize_bounded_task_result(
            &lease,
            &root,
            result,
            BoundedTaskCancellationPhase::PreRunRevalidation,
            |_| Ok(()),
        );
    }
    let persisted = if let Some(storage) = &storage {
        let repository = repository_identity(&root)?;
        let continuing_goal = expected_goal.as_ref().filter(|goal| {
            goal.repository.canonical_path == repository.canonical_path
                && goal.repository.repository_id == repository.repository_id
        });
        let record = PersistedGoalRecord {
            version: GOAL_STATE_VERSION,
            revision: 0,
            objective: continuing_goal
                .map(|goal| goal.objective.clone())
                .unwrap_or_else(|| preview.goal.clone()),
            repository,
            stop_conditions: goal_loop_stop_conditions(),
            current_run: PersistedRunCursor {
                run_id: run_id.clone(),
                event_cursor: 0,
                nonterminal: true,
            },
            last_checkpoint: None,
            evidence_references: Vec::new(),
            collaboration: None,
        };
        let record = storage.create_run(expected_goal.as_ref(), record)?;
        Some(Arc::new(Mutex::new(record)))
    } else {
        None
    };
    let argv = if invocation.mode == RuntimeMode::Live {
        bounded_task_runtime_argv(&root, &preview.prompt)
    } else {
        Vec::new()
    };
    let channel_failure = Arc::new(Mutex::new(None));
    let stream_channel = on_event.clone();
    let stream_run_id = run_id.clone();
    let sequence = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stream_failure = Arc::clone(&channel_failure);
    let stream_cancel = Arc::clone(&lease.cancel);
    let stream_storage = storage.clone();
    let stream_persisted = persisted.clone();
    let runtime_invocation = RuntimeInvocation {
        mode: invocation.mode,
        prompt: (invocation.mode == RuntimeMode::Live).then_some(preview.prompt.clone()),
        confirmed: true,
    };
    let mut runtime = execute_bounded_task_runtime_with(
        &root,
        runtime_invocation,
        cfg!(unix),
        (invocation.mode == RuntimeMode::Live).then_some(argv),
        || run_version(&root, &lease.cancel),
        |argv, timeout| {
            let sequence = Arc::clone(&sequence);
            let handler = Arc::new(move |line: &[u8]| {
                let next = sequence.fetch_add(1, Ordering::AcqRel);
                let event = parse_runtime_line(line, next, "provider").0;
                if let (Some(storage), Some(persisted)) = (&stream_storage, &stream_persisted) {
                    if let Ok(mut record) = persisted.lock() {
                        match storage.advance_event(&record, (next + 1) as u64) {
                            Ok(updated) => *record = updated,
                            Err(error) => record_runtime_channel_failure(
                                &stream_failure,
                                &stream_cancel,
                                format!(
                                    "Goal event persistence failed: {}: {}",
                                    error.code, error.message
                                ),
                            ),
                        }
                    }
                }
                if let Err(error) = stream_channel.send(RuntimeStreamMessage::Event {
                    run_id: stream_run_id.clone(),
                    event,
                }) {
                    record_runtime_channel_failure(
                        &stream_failure,
                        &stream_cancel,
                        error.to_string(),
                    );
                }
            });
            run_live(&root, argv, timeout, &lease.cancel, Some(handler))
        },
    );
    runtime.run_id = run_id.clone();
    if runtime.provenance.simulated {
        for event in &runtime.events {
            if let Err(error) = on_event.send(RuntimeStreamMessage::Event {
                run_id: run_id.clone(),
                event: event.clone(),
            }) {
                record_runtime_channel_failure(&channel_failure, &lease.cancel, error.to_string());
                break;
            }
        }
    }
    apply_runtime_channel_failure(&mut runtime, &channel_failure);

    let result = finish_bounded_task_with(
        &root,
        Some(selected_task),
        Some(runtime),
        None,
        None,
        &lease.cancel,
        &lease.explicit_user_cancellation,
        &mut run_helper,
    );
    finalize_bounded_task_result(
        &lease,
        &root,
        result,
        BoundedTaskCancellationPhase::PostExitRefresh,
        |result| {
            if let (Some(storage), Some(persisted)) = (&storage, &persisted) {
                let task = result.task_evidence.as_ref().ok_or_else(|| {
                    ProjectError::new(
                        "goal_checkpoint_missing_task",
                        "Verified result omitted task evidence",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let mut record = persisted.lock().map_err(|_| {
                    ProjectError::new(
                        "goal_checkpoint_lock_failed",
                        "Goal checkpoint lock is poisoned",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let event_cursor = result
                    .runtime
                    .as_ref()
                    .map(|runtime| runtime.events.len() as u64)
                    .unwrap_or(record.current_run.event_cursor)
                    .max(record.current_run.event_cursor);
                let checkpoint = VerifiedGoalCheckpoint {
                    task_path: task.path.clone(),
                    task_sha256: sha256_bytes(task.content.as_bytes()),
                    git: git_fingerprint(&root)?,
                };
                let mut evidence_references = vec![GoalEvidenceReference {
                    path: task.path.clone(),
                    sha256: sha256_bytes(task.content.as_bytes()),
                }];
                if let Some(tracker) =
                    terminal_tracker_reference(&root, &result.project, &task.path)
                {
                    evidence_references.push(tracker);
                }
                *record = storage
                    .finish_run(&record, event_cursor, checkpoint, evidence_references)
                    .map_err(ProjectError::after_commit)?;
            }
            let task = result.task_evidence.as_ref().ok_or_else(|| {
                ProjectError::new(
                    "collaboration_post_commit_missing_task",
                    "Verified result omitted task evidence for the collaboration hook",
                    Some(&root),
                )
                .after_commit()
            })?;
            let intent = if collaboration_policy.mode == CollaborationMode::SharedCollaborator {
                let binding = shared_completion_binding.as_ref().ok_or_else(|| {
                    ProjectError::new(
                        "collaboration_completion_binding_missing",
                        "Shared completion has no confirmation-bound collaboration state",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let claimed_task_version = match pre_run_collaboration.claim {
                    ClaimResult::Claimed { remote_version } => remote_version,
                    _ => {
                        return Err(ProjectError::new(
                            "collaboration_completion_claim_missing",
                            "Shared completion has no exact committed claim version",
                            Some(&root),
                        )
                        .after_commit())
                    }
                };
                let storage = storage.as_ref().ok_or_else(|| {
                    ProjectError::new(
                        "goal_collaboration_storage_required",
                        "Shared completion requires durable reconciliation persistence",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let persisted = persisted.as_ref().ok_or_else(|| {
                    ProjectError::new(
                        "goal_collaboration_cursor_missing",
                        "Shared completion has no persisted checkpoint",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let completed_local =
                    local_collaboration_binding(&root, &task.path, &task.content)?;
                let created_at_unix_seconds = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| {
                        ProjectError::new(
                            "collaboration_timestamp_failed",
                            "System time cannot produce a portable completion timestamp",
                            Some(&root),
                        )
                        .after_commit()
                    })?
                    .as_secs();
                let mut record = persisted.lock().map_err(|_| {
                    ProjectError::new(
                        "goal_checkpoint_lock_failed",
                        "Goal checkpoint lock is poisoned",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let intent = build_remote_completion_intent(
                    binding,
                    &completed_local,
                    claimed_task_version,
                    &run_id,
                    created_at_unix_seconds,
                    &record.evidence_references,
                )
                .map_err(ProjectError::after_commit)?;
                let capability_guard = |candidate: &RemoteCompletionIntent| {
                    collaboration_port
                        .validate_completion_intent_for_persistence(candidate)
                        .map_err(|failure| collaboration_project_error(&root, "intent", failure))
                };
                *record = storage
                    .begin_collaboration_reconciliation(&record, intent.clone(), &capability_guard)
                    .map_err(ProjectError::after_commit)?;
                Some(intent)
            } else {
                None
            };
            let post_commit = run_post_commit_collaboration_hook(
                collaboration_port,
                &collaboration_policy,
                &root,
                task,
                &run_id,
                intent,
            )
            .map_err(ProjectError::after_commit)?;
            if collaboration_policy.mode == CollaborationMode::SharedCollaborator {
                let storage = storage.as_ref().ok_or_else(|| {
                    ProjectError::new(
                        "goal_collaboration_storage_required",
                        "Shared completion requires durable reconciliation persistence",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let persisted = persisted.as_ref().ok_or_else(|| {
                    ProjectError::new(
                        "goal_collaboration_cursor_missing",
                        "Shared completion has no persisted reconciliation cursor",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                let mut record = persisted.lock().map_err(|_| {
                    ProjectError::new(
                        "goal_checkpoint_lock_failed",
                        "Goal checkpoint lock is poisoned",
                        Some(&root),
                    )
                    .after_commit()
                })?;
                *record = storage
                    .finish_collaboration_reconciliation(&record, &post_commit)
                    .map_err(ProjectError::after_commit)?;
            }
            Ok(())
        },
    )
}

#[tauri::command]
fn cancel_bounded_task(run_id: String) -> Result<RuntimeCancellation, ProjectError> {
    if run_id.len() != 32 || !run_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProjectError::new(
            "invalid_runtime_run_id",
            "Bounded task cancellation requires a native-issued run ID",
            None,
        ));
    }
    let requested = operation_registry().cancel_bounded_task(&run_id)?;
    Ok(RuntimeCancellation {
        cancellation_requested: requested,
        message: if requested {
            "Cancellation requested; the controller will reap Codex, refresh repository truth, and stop".into()
        } else {
            "No bounded task is currently running".into()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SKILL: &str = "build-right-preflight";
    const TEST_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[derive(Clone, Copy)]
    enum FakeSharedPreRun {
        Claimed,
        Conflict,
        Failure(CollaborationFailureClass),
        CancelAfterClaim,
    }

    struct FakeSharedPort {
        behavior: FakeSharedPreRun,
        before_calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl CollaborationPort for FakeSharedPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            self.before_calls.fetch_add(1, Ordering::AcqRel);
            match self.behavior {
                FakeSharedPreRun::Claimed => Ok(PreRunCollaborationOutcome {
                    reconciliation: ReconciliationState::Claimed,
                    claim: ClaimResult::Claimed { remote_version: 8 },
                }),
                FakeSharedPreRun::Conflict => Ok(PreRunCollaborationOutcome {
                    reconciliation: ReconciliationState::Conflict,
                    claim: ClaimResult::Stopped {
                        failure_class: CollaborationFailureClass::VersionConflict,
                        latest_remote_version: Some(8),
                        repair: RepairHint::refresh_conflict(),
                    },
                }),
                FakeSharedPreRun::Failure(class) => Err(collaboration_failure_for_class(class)),
                FakeSharedPreRun::CancelAfterClaim => {
                    cancel.store(true, Ordering::Release);
                    Ok(PreRunCollaborationOutcome {
                        reconciliation: ReconciliationState::Claimed,
                        claim: ClaimResult::Claimed { remote_version: 8 },
                    })
                }
            }
        }

        fn after_local_commit(
            &self,
            context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            let intent = context
                .intent
                .as_ref()
                .ok_or_else(CollaborationFailure::protocol)?;
            Ok(PostLocalCommitCollaborationOutcome {
                reconciliation: ReconciliationState::Reconciled,
                evidence_handoff: EvidenceHandoffResult::Synchronized {
                    remote_version: intent.claimed_task_version + 1,
                    evidence_ids: vec![EvidenceReferenceId::parse(intent.evidence_id.clone())?],
                    handoff_id: Some(HandoffReferenceId::parse(intent.handoff_id.clone())?),
                },
            })
        }
    }

    struct StatefulSharedClaimPort {
        cancel_after_claim: bool,
        before_calls: Arc<std::sync::atomic::AtomicUsize>,
        state: Mutex<SharedClaimState>,
    }

    impl StatefulSharedClaimPort {
        fn new(cancel_after_claim: bool) -> Self {
            Self {
                cancel_after_claim,
                before_calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                state: Mutex::new(SharedClaimState::Reconciled),
            }
        }
    }

    impl CollaborationPort for StatefulSharedClaimPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            self.before_calls.fetch_add(1, Ordering::AcqRel);
            *self
                .state
                .lock()
                .map_err(|_| CollaborationFailure::protocol())? = SharedClaimState::Claimed {
                remote_version: 8,
                recovered_from_readback: false,
            };
            if self.cancel_after_claim {
                cancel.store(true, Ordering::Release);
            }
            Ok(PreRunCollaborationOutcome {
                reconciliation: ReconciliationState::Claimed,
                claim: ClaimResult::Claimed { remote_version: 8 },
            })
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            Ok(PostLocalCommitCollaborationOutcome {
                reconciliation: ReconciliationState::Claimed,
                evidence_handoff: EvidenceHandoffResult::NotRequired,
            })
        }
    }

    impl SharedClaimPort for StatefulSharedClaimPort {
        fn shared_claim_state(&self) -> Result<SharedClaimState, ProjectError> {
            self.state.lock().map(|state| state.clone()).map_err(|_| {
                ProjectError::new(
                    "shared_claim_state_failed",
                    "Shared claim state is unavailable",
                    None,
                )
            })
        }

        fn mark_claimed_pre_spawn_repair(
            &self,
            failure_class: CollaborationFailureClass,
            cause: SharedClaimRepairCause,
        ) -> Result<(), ProjectError> {
            self.state
                .lock()
                .map(|mut state| state.mark_claimed_pre_spawn_repair(failure_class, cause))
                .map_err(|_| {
                    ProjectError::new(
                        "shared_claim_state_failed",
                        "Shared claim state is unavailable",
                        None,
                    )
                })
        }
    }

    struct RepairDebtSharedPort {
        before_calls: Arc<std::sync::atomic::AtomicUsize>,
        post_calls: Arc<std::sync::atomic::AtomicUsize>,
        state: Mutex<SharedClaimState>,
        post_commit: Mutex<Option<PostLocalCommitCollaborationOutcome>>,
    }

    impl RepairDebtSharedPort {
        fn new() -> Self {
            Self {
                before_calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                post_calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                state: Mutex::new(SharedClaimState::Reconciled),
                post_commit: Mutex::new(None),
            }
        }
    }

    impl CollaborationPort for RepairDebtSharedPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            _cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            self.before_calls.fetch_add(1, Ordering::AcqRel);
            *self
                .state
                .lock()
                .map_err(|_| CollaborationFailure::protocol())? = SharedClaimState::Claimed {
                remote_version: 8,
                recovered_from_readback: false,
            };
            Ok(PreRunCollaborationOutcome {
                reconciliation: ReconciliationState::Claimed,
                claim: ClaimResult::Claimed { remote_version: 8 },
            })
        }

        fn after_local_commit(
            &self,
            context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            self.post_calls.fetch_add(1, Ordering::AcqRel);
            let intent = context
                .intent
                .as_ref()
                .ok_or_else(CollaborationFailure::protocol)?;
            if intent.claimed_task_version != 8 {
                return Err(CollaborationFailure::protocol());
            }
            let outcome = PostLocalCommitCollaborationOutcome {
                reconciliation: ReconciliationState::RepairRequired,
                evidence_handoff: EvidenceHandoffResult::Partial {
                    remote_version: Some(8),
                    missing_effects: vec![
                        MissingCollaborationEffect::TaskUpdate,
                        MissingCollaborationEffect::HandoffWrite,
                        MissingCollaborationEffect::StatusWrite,
                    ],
                    repair: RepairHint::retry_sync(),
                },
            };
            *self
                .post_commit
                .lock()
                .map_err(|_| CollaborationFailure::protocol())? = Some(outcome.clone());
            Ok(outcome)
        }
    }

    impl SharedClaimPort for RepairDebtSharedPort {
        fn shared_claim_state(&self) -> Result<SharedClaimState, ProjectError> {
            self.state.lock().map(|state| state.clone()).map_err(|_| {
                ProjectError::new(
                    "shared_claim_state_failed",
                    "Shared claim state is unavailable",
                    None,
                )
            })
        }

        fn mark_claimed_pre_spawn_repair(
            &self,
            failure_class: CollaborationFailureClass,
            cause: SharedClaimRepairCause,
        ) -> Result<(), ProjectError> {
            self.state
                .lock()
                .map(|mut state| state.mark_claimed_pre_spawn_repair(failure_class, cause))
                .map_err(|_| {
                    ProjectError::new(
                        "shared_claim_state_failed",
                        "Shared claim state is unavailable",
                        None,
                    )
                })
        }

        fn post_commit_outcome(
            &self,
        ) -> Result<Option<PostLocalCommitCollaborationOutcome>, ProjectError> {
            self.post_commit
                .lock()
                .map(|value| value.clone())
                .map_err(|_| {
                    ProjectError::new(
                        "shared_claim_state_failed",
                        "Shared completion state is unavailable",
                        None,
                    )
                })
        }
    }

    fn task016_test_session(access: CollaborationAccess) -> SanitizedSessionMetadata {
        SanitizedSessionMetadata::new(
            collaboration::LocalSessionHandle::parse(format!("local-session-{}", "b".repeat(32)))
                .unwrap(),
            "workspace-task016".into(),
            "https://app.example.test".into(),
            "https://api.example.test".into(),
            access,
            "codex-task016".into(),
        )
        .unwrap()
    }

    fn task016_test_binding(root: &Path, access: CollaborationAccess) -> SharedExecutionBinding {
        let task = read_controller_task(root, "tasks/issues/009-fixture.md").unwrap();
        SharedExecutionBinding::new(
            task016_test_session(access),
            local_collaboration_binding(root, &task.path, &task.content).unwrap(),
            collaboration::RemoteTaskBinding {
                task_id: "BR-009".into(),
                task_path: "tasks/BR-009.md".into(),
                base_version: 7,
            },
        )
        .unwrap()
    }

    fn task016_shared_wrapper_inputs(
        root: &Path,
    ) -> (
        PathBuf,
        Arc<OperationRegistry>,
        SharedExecutionBinding,
        BoundedTaskInvocation,
    ) {
        let canonical = validated_repository_root(&root.to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = build_bounded_task_preview(&canonical, &AtomicBool::new(false)).unwrap();
        let binding = task016_test_binding(&canonical, CollaborationAccess::Collaborator);
        let token = registry
            .issue_shared_bounded_task_confirmation(
                &canonical,
                preview.preview_token,
                binding.clone(),
            )
            .unwrap();
        (
            canonical,
            registry,
            binding,
            BoundedTaskInvocation {
                preview_token: token,
                selected_task: "tasks/issues/009-fixture.md".into(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
        )
    }

    fn assert_claimed_pre_spawn_repair(
        result: &SharedBoundedTaskResult,
        failure_class: CollaborationFailureClass,
        cause: SharedClaimRepairCause,
    ) {
        match &result.claim {
            SharedClaimState::ClaimedRepairRequired {
                remote_version,
                failure_class: actual_class,
                cause: actual_cause,
                repair,
            } => {
                assert_eq!(*remote_version, 8);
                assert_eq!(*actual_class, failure_class);
                assert_eq!(*actual_cause, cause);
                assert_eq!(repair.code(), "reconcile-claimed-pre-spawn");
                assert!(repair.next_action().contains("release or reconcile"));
                assert!(repair.next_action().contains("never reuse"));
            }
            state => panic!("expected claimed repair state, got {state:?}"),
        }
        assert!(!result.codex_started);
        assert!(result.stopped_before_runtime);
    }

    fn execute_task016_fake_port(
        root: &Path,
        behavior: FakeSharedPreRun,
    ) -> (
        Result<BoundedTaskResult, ProjectError>,
        Arc<std::sync::atomic::AtomicUsize>,
    ) {
        let canonical = validated_repository_root(&root.to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = build_bounded_task_preview(&canonical, &AtomicBool::new(false)).unwrap();
        let binding = task016_test_binding(&canonical, CollaborationAccess::Collaborator);
        let token = registry
            .issue_shared_bounded_task_confirmation(
                &canonical,
                preview.preview_token,
                binding.clone(),
            )
            .unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let port = FakeSharedPort {
            behavior,
            before_calls: Arc::clone(&calls),
        };
        let result = execute_bounded_task_command_with_storage_helper_and_collaboration(
            canonical.to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: token,
                selected_task: "tasks/issues/009-fixture.md".into(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            None,
            registry,
            |_, _| panic!("remote pre-run stop must not probe or spawn Codex"),
            |_, _, _, _, _| panic!("remote pre-run stop must not spawn Codex"),
            controller_helper,
            &port,
            ControllerCollaborationPolicy {
                mode: CollaborationMode::SharedCollaborator,
                session: Some(binding.session.clone()),
                remote: Some(binding.remote.clone()),
            },
            BoundedTaskConfirmationScope::Shared(binding),
        );
        (result, calls)
    }

    #[test]
    fn shared_confirmation_is_exact_scope_bound_and_cannot_execute_solo() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let registry = OperationRegistry::default();
        let binding = task016_test_binding(project.path(), CollaborationAccess::Collaborator);
        let token = registry
            .issue_shared_bounded_task_confirmation(
                project.path(),
                "local-baseline".into(),
                binding.clone(),
            )
            .unwrap();

        assert!(registry
            .consume_bounded_task_confirmation(
                project.path(),
                &token,
                &BoundedTaskConfirmationScope::Local,
            )
            .is_err());
        assert_eq!(
            registry
                .consume_bounded_task_confirmation(
                    project.path(),
                    &token,
                    &BoundedTaskConfirmationScope::Shared(binding),
                )
                .unwrap(),
            "local-baseline"
        );
        assert!(registry
            .consume_bounded_task_confirmation(
                project.path(),
                &token,
                &BoundedTaskConfirmationScope::Local,
            )
            .is_err());
    }

    #[test]
    fn viewer_and_public_shared_previews_are_inspection_only() {
        for access in [CollaborationAccess::Viewer, CollaborationAccess::Public] {
            let project = tempfile::tempdir().unwrap();
            write_controller_repository(project.path());
            let registry = OperationRegistry::default();
            let session = task016_test_session(access);
            let preview = build_shared_bounded_task_preview_with(
                project.path(),
                session.clone(),
                &AtomicBool::new(false),
                &registry,
                controller_helper,
                |local| {
                    Ok(JoinResult {
                        workspace_id: session.workspace_id.clone(),
                        actor: session.actor.clone(),
                        access: session.access,
                        task: collaboration::RemoteTaskBinding {
                            task_id: "BR-009".into(),
                            task_path: "tasks/BR-009.md".into(),
                            base_version: 7,
                        },
                        local,
                        reconciled: true,
                        executable: false,
                        inspection_only: true,
                        repair: Some(EnvelopeRepair::new(
                            "inspection-only",
                            "Read-only access cannot claim tasks",
                            "Reconnect with collaborator access",
                        )),
                    })
                },
            )
            .unwrap();

            assert!(!preview.executable);
            assert!(!preview.explicit_confirmation_required);
            assert!(preview.preview_token.is_empty());
            assert_eq!(preview.binding.remote.base_version, 7);
            assert_eq!(preview.binding.session.access, access);
            assert_eq!(
                preview.bounded.loop_state.state,
                GoalLoopState::ExternalStop
            );
            assert!(registry
                .shared_confirmation_binding(project.path(), "", session.session_id.as_str())
                .is_err());
        }
    }

    #[test]
    fn every_remote_pre_run_stop_family_runs_once_and_spawns_zero_codex() {
        let families = [
            FakeSharedPreRun::Conflict,
            FakeSharedPreRun::Failure(CollaborationFailureClass::AccessDenied),
            FakeSharedPreRun::Failure(CollaborationFailureClass::SourceMismatch),
            FakeSharedPreRun::Failure(CollaborationFailureClass::TransportUnavailable),
            FakeSharedPreRun::Failure(CollaborationFailureClass::Timeout),
            FakeSharedPreRun::Failure(CollaborationFailureClass::Cancelled),
            FakeSharedPreRun::Failure(CollaborationFailureClass::Protocol),
        ];
        for behavior in families {
            let project = tempfile::tempdir().unwrap();
            write_controller_repository(project.path());
            let (result, calls) = execute_task016_fake_port(project.path(), behavior);
            assert!(result.is_err());
            assert_eq!(calls.load(Ordering::Acquire), 1);
        }
    }

    #[test]
    fn clean_remote_claim_runs_exactly_one_codex_process() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let canonical = validated_repository_root(&project.path().to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = build_bounded_task_preview(&canonical, &AtomicBool::new(false)).unwrap();
        let binding = task016_test_binding(&canonical, CollaborationAccess::Collaborator);
        let token = registry
            .issue_shared_bounded_task_confirmation(
                &canonical,
                preview.preview_token,
                binding.clone(),
            )
            .unwrap();
        let before_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let port = FakeSharedPort {
            behavior: FakeSharedPreRun::Claimed,
            before_calls: Arc::clone(&before_calls),
        };
        let version_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_version_calls = Arc::clone(&version_calls);
        let codex_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_codex_calls = Arc::clone(&codex_calls);

        let result = execute_bounded_task_command_with_storage_helper_and_collaboration(
            canonical.to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: token,
                selected_task: "tasks/issues/009-fixture.md".into(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            registry,
            move |_, _| {
                observed_version_calls.fetch_add(1, Ordering::AcqRel);
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                observed_codex_calls.fetch_add(1, Ordering::AcqRel);
                fs::write(
                    root.join("tasks/issues/009-fixture.md"),
                    controller_task_text("complete", true),
                )
                .unwrap();
                fs::write(
                    root.join("tasks/sprint-1.md"),
                    "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
                )
                .unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            controller_helper,
            &port,
            ControllerCollaborationPolicy {
                mode: CollaborationMode::SharedCollaborator,
                session: Some(binding.session.clone()),
                remote: Some(binding.remote.clone()),
            },
            BoundedTaskConfirmationScope::Shared(binding),
        )
        .unwrap();

        assert_eq!(before_calls.load(Ordering::Acquire), 1);
        assert_eq!(version_calls.load(Ordering::Acquire), 1);
        assert_eq!(codex_calls.load(Ordering::Acquire), 1);
        assert_eq!(result.outcome, BoundedTaskOutcome::Verified);
        assert!(result.repository_verified);
        let persisted: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert_eq!(
            persisted.collaboration.as_ref().unwrap().state,
            PersistedReconciliationState::Reconciled
        );
        assert!(persisted
            .collaboration
            .as_ref()
            .unwrap()
            .missing_effects
            .is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn verified_shared_completion_persists_repair_debt_and_blocks_only_shared_continuation() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let (root, registry, binding, invocation) = task016_shared_wrapper_inputs(project.path());
        let port = RepairDebtSharedPort::new();

        let result = execute_shared_bounded_task_with_claim_port(
            root.to_string_lossy().to_string(),
            invocation,
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            Arc::clone(&registry),
            binding,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |root, _, _, _, _| {
                fs::write(
                    root.join("tasks/issues/009-fixture.md"),
                    controller_task_text("complete", true),
                )
                .unwrap();
                fs::write(
                    root.join("tasks/sprint-1.md"),
                    "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
                )
                .unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            controller_helper,
            &port,
        )
        .unwrap();

        assert_eq!(port.before_calls.load(Ordering::Acquire), 1);
        assert_eq!(port.post_calls.load(Ordering::Acquire), 1);
        assert!(result.codex_started);
        assert!(result.shared_iteration_blocked);
        assert!(matches!(
            result.completion,
            SharedCompletionState::CollaborationRepairRequired { .. }
        ));
        let bounded = result.bounded.unwrap();
        assert!(bounded.repository_verified);
        assert_eq!(bounded.outcome, BoundedTaskOutcome::Verified);
        assert_eq!(bounded.loop_state.state, GoalLoopState::FailureStop);
        assert_eq!(
            bounded.loop_state.blocking_gates,
            ["collaborationRepairRequired"]
        );

        let persisted: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert!(!persisted.current_run.nonterminal);
        assert!(persisted.last_checkpoint.is_some());
        let cursor = persisted.collaboration.as_ref().unwrap();
        assert_eq!(
            cursor.state,
            PersistedReconciliationState::CollaborationRepairRequired
        );
        assert_eq!(
            cursor.missing_effects,
            vec![
                MissingCollaborationEffect::TaskUpdate,
                MissingCollaborationEffect::HandoffWrite,
                MissingCollaborationEffect::StatusWrite,
            ]
        );
        assert_eq!(
            ensure_shared_iteration_has_no_repair_debt(&storage, &root)
                .unwrap_err()
                .code,
            "collaboration_repair_required"
        );
        let local_preview = preview_bounded_task_with_registry(
            root.to_string_lossy().to_string(),
            Arc::new(OperationRegistry::default()),
        )
        .unwrap();
        assert!(!local_preview.executable);
        let recovery = goal_recovery(&storage, &root).unwrap();
        assert_eq!(
            recovery.collaboration.as_ref().unwrap().state,
            PersistedReconciliationState::CollaborationRepairRequired
        );
        assert!(!recovery.automatic_execution_started);
    }

    #[test]
    fn cancellation_linearizes_after_claim_and_before_any_codex_spawn() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let (result, calls) =
            execute_task016_fake_port(project.path(), FakeSharedPreRun::CancelAfterClaim);
        let result = result.unwrap();

        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert!(result.runtime.is_none());
        assert!(!result.repository_verified);
        assert_eq!(result.outcome, BoundedTaskOutcome::Stopped);
    }

    struct BlockingClaimPort {
        entered: Mutex<Option<std::sync::mpsc::Sender<()>>>,
        release: Arc<std::sync::Barrier>,
    }

    impl CollaborationPort for BlockingClaimPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            self.entered
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(())
                .unwrap();
            self.release.wait();
            assert!(cancel.load(Ordering::Acquire));
            Ok(PreRunCollaborationOutcome {
                reconciliation: ReconciliationState::Claimed,
                claim: ClaimResult::Claimed { remote_version: 8 },
            })
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            unreachable!("a cancelled pre-run claim must not reach post-commit collaboration")
        }
    }

    #[test]
    fn concurrent_cancellation_linearizes_at_remote_claim_boundary() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let canonical = validated_repository_root(&project.path().to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = build_bounded_task_preview(&canonical, &AtomicBool::new(false)).unwrap();
        let binding = task016_test_binding(&canonical, CollaborationAccess::Collaborator);
        let token = registry
            .issue_shared_bounded_task_confirmation(
                &canonical,
                preview.preview_token,
                binding.clone(),
            )
            .unwrap();
        let (run_id_tx, run_id_rx) = std::sync::mpsc::channel();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let release = Arc::new(std::sync::Barrier::new(2));
        let thread_release = Arc::clone(&release);
        let thread_registry = Arc::clone(&registry);
        let thread_binding = binding.clone();
        let root = canonical.to_string_lossy().to_string();

        let worker = std::thread::spawn(move || {
            let port = BlockingClaimPort {
                entered: Mutex::new(Some(entered_tx)),
                release: thread_release,
            };
            execute_bounded_task_command_with_storage_helper_and_collaboration(
                root,
                BoundedTaskInvocation {
                    preview_token: token,
                    selected_task: "tasks/issues/009-fixture.md".into(),
                    mode: RuntimeMode::Live,
                    confirmed: true,
                },
                Channel::new(move |message| {
                    let message: serde_json::Value = message.deserialize().unwrap();
                    if message.get("type").and_then(serde_json::Value::as_str) == Some("started") {
                        run_id_tx
                            .send(
                                message
                                    .get("handle")
                                    .and_then(|handle| handle.get("runId"))
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap()
                                    .to_string(),
                            )
                            .unwrap();
                    }
                    Ok(())
                }),
                None,
                thread_registry,
                |_, _| panic!("cancellation at the claim boundary must not probe Codex"),
                |_, _, _, _, _| panic!("cancellation at the claim boundary must not spawn Codex"),
                controller_helper,
                &port,
                ControllerCollaborationPolicy {
                    mode: CollaborationMode::SharedCollaborator,
                    session: Some(thread_binding.session.clone()),
                    remote: Some(thread_binding.remote.clone()),
                },
                BoundedTaskConfirmationScope::Shared(thread_binding),
            )
        });

        let run_id = run_id_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(registry.cancel_bounded_task(&run_id).unwrap());
        release.wait();
        let result = worker.join().unwrap().unwrap();

        assert_eq!(result.outcome, BoundedTaskOutcome::Stopped);
        assert!(result.runtime.is_none());
        assert!(!result.repository_verified);
        assert_eq!(result.loop_state.state, GoalLoopState::CancelledStop);
    }

    struct RecordingConflictPort {
        root: PathBuf,
        registry: Arc<OperationRegistry>,
        binding: SharedExecutionBinding,
        calls: Arc<std::sync::atomic::AtomicUsize>,
        repairs: Arc<Mutex<Vec<RepairHint>>>,
    }

    impl CollaborationPort for RecordingConflictPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            _cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            let (_, repair) = self
                .registry
                .record_shared_conflict(&self.root, &self.binding)
                .map_err(|_| CollaborationFailure::protocol())?;
            self.repairs.lock().unwrap().push(repair.clone());
            Ok(PreRunCollaborationOutcome {
                reconciliation: ReconciliationState::Conflict,
                claim: ClaimResult::Stopped {
                    failure_class: CollaborationFailureClass::VersionConflict,
                    latest_remote_version: Some(self.binding.remote.base_version + 1),
                    repair,
                },
            })
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            unreachable!("a conflicting claim must not reach post-commit collaboration")
        }
    }

    fn execute_recorded_conflict(
        root: &Path,
        registry: Arc<OperationRegistry>,
        binding: SharedExecutionBinding,
        token: String,
        port: &RecordingConflictPort,
    ) -> Result<BoundedTaskResult, ProjectError> {
        execute_bounded_task_command_with_storage_helper_and_collaboration(
            root.to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: token,
                selected_task: "tasks/issues/009-fixture.md".into(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            None,
            registry,
            |_, _| panic!("a conflicting claim must not probe Codex"),
            |_, _, _, _, _| panic!("a conflicting claim must not spawn Codex"),
            controller_helper,
            port,
            ControllerCollaborationPolicy {
                mode: CollaborationMode::SharedCollaborator,
                session: Some(binding.session.clone()),
                remote: Some(binding.remote.clone()),
            },
            BoundedTaskConfirmationScope::Shared(binding),
        )
    }

    #[test]
    fn repeated_conflict_requires_fresh_confirmation_then_human_inspection() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let root = validated_repository_root(&project.path().to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = build_bounded_task_preview(&root, &AtomicBool::new(false)).unwrap();
        let first_binding = task016_test_binding(&root, CollaborationAccess::Collaborator);
        let first_token = registry
            .issue_shared_bounded_task_confirmation(
                &root,
                preview.preview_token,
                first_binding.clone(),
            )
            .unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let repairs = Arc::new(Mutex::new(Vec::new()));
        let first_port = RecordingConflictPort {
            root: root.clone(),
            registry: Arc::clone(&registry),
            binding: first_binding.clone(),
            calls: Arc::clone(&calls),
            repairs: Arc::clone(&repairs),
        };

        assert!(execute_recorded_conflict(
            &root,
            Arc::clone(&registry),
            first_binding.clone(),
            first_token.clone(),
            &first_port,
        )
        .is_err());
        assert_eq!(calls.load(Ordering::Acquire), 1);
        let replay = execute_recorded_conflict(
            &root,
            Arc::clone(&registry),
            first_binding,
            first_token,
            &first_port,
        )
        .unwrap_err();
        assert_eq!(replay.code, "controller_confirmation_consumed_or_stale");
        assert_eq!(calls.load(Ordering::Acquire), 1);

        let fresh_preview = build_bounded_task_preview(&root, &AtomicBool::new(false)).unwrap();
        let mut fresh_remote =
            task016_test_binding(&root, CollaborationAccess::Collaborator).remote;
        fresh_remote.base_version = 8;
        let fresh_binding = SharedExecutionBinding::new(
            task016_test_session(CollaborationAccess::Collaborator),
            local_collaboration_binding(
                &root,
                "tasks/issues/009-fixture.md",
                &read_controller_task(&root, "tasks/issues/009-fixture.md")
                    .unwrap()
                    .content,
            )
            .unwrap(),
            fresh_remote,
        )
        .unwrap();
        let fresh_token = registry
            .issue_shared_bounded_task_confirmation(
                &root,
                fresh_preview.preview_token,
                fresh_binding.clone(),
            )
            .unwrap();
        let second_port = RecordingConflictPort {
            root: root.clone(),
            registry: Arc::clone(&registry),
            binding: fresh_binding.clone(),
            calls: Arc::clone(&calls),
            repairs: Arc::clone(&repairs),
        };
        assert!(execute_recorded_conflict(
            &root,
            Arc::clone(&registry),
            fresh_binding,
            fresh_token,
            &second_port,
        )
        .is_err());

        assert_eq!(calls.load(Ordering::Acquire), 2);
        let repairs = repairs.lock().unwrap();
        assert_eq!(repairs.len(), 2);
        assert_eq!(repairs[0].code(), "refresh-conflict");
        assert_eq!(repairs[1].code(), "inspect-repeated-conflict");
        assert!(repairs[1].next_action().contains("human"));
    }

    #[test]
    fn claimed_state_survives_pre_spawn_finalization_failure_without_secrets() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let sessions = MdsyncSessionStore::default();
        let binding = task016_test_binding(project.path(), CollaborationAccess::Collaborator);
        let port = MdsyncClaimPort::new(
            &sessions,
            project.path().to_string_lossy().to_string(),
            binding.session.session_id.as_str().into(),
            binding,
            Arc::new(OperationRegistry::default()),
        );
        port.set_state(SharedClaimState::Claimed {
            remote_version: 8,
            recovered_from_readback: false,
        })
        .unwrap();
        let failure = port
            .claimed_repair(8, CollaborationFailureClass::SourceMismatch)
            .unwrap();
        let state = port.state().unwrap();

        assert_eq!(failure.class(), CollaborationFailureClass::RepairRequired);
        assert!(matches!(
            state,
            SharedClaimState::ClaimedRepairRequired {
                remote_version: 8,
                failure_class: CollaborationFailureClass::SourceMismatch,
                ..
            }
        ));
        let serialized = serde_json::to_string(&state).unwrap().to_ascii_lowercase();
        for forbidden in ["authorization", "bearer ", "token=", "?edit=", "secret"] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    fn shared_wrapper_marks_committed_claim_for_repair_on_post_claim_cancellation() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let (root, registry, binding, invocation) = task016_shared_wrapper_inputs(project.path());
        let port = StatefulSharedClaimPort::new(true);
        let codex_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_codex_calls = Arc::clone(&codex_calls);

        let result = execute_shared_bounded_task_with_claim_port(
            root.to_string_lossy().to_string(),
            invocation,
            Channel::new(|_| Ok(())),
            None,
            registry,
            binding,
            |_, _| panic!("post-claim cancellation must not probe Codex"),
            move |_, _, _, _, _| {
                observed_codex_calls.fetch_add(1, Ordering::AcqRel);
                panic!("post-claim cancellation must not execute Codex")
            },
            controller_helper,
            &port,
        )
        .unwrap();

        assert_eq!(port.before_calls.load(Ordering::Acquire), 1);
        assert_eq!(codex_calls.load(Ordering::Acquire), 0);
        assert_claimed_pre_spawn_repair(
            &result,
            CollaborationFailureClass::Cancelled,
            SharedClaimRepairCause::Cancellation,
        );
    }

    #[cfg(unix)]
    #[test]
    fn shared_wrapper_marks_committed_claim_for_repair_on_goal_storage_failure() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let (root, registry, binding, invocation) = task016_shared_wrapper_inputs(project.path());
        let port = StatefulSharedClaimPort::new(false);
        let storage_directory = root.join("task016-goal-storage");
        let hook_directory = storage_directory.clone();
        let directory_opens = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_directory_opens = Arc::clone(&directory_opens);
        let storage = GoalStateStorage::new(storage_directory).with_test_hook(move |phase| {
            if phase == GoalStorageTestPhase::DirectoryOpened
                && observed_directory_opens.fetch_add(1, Ordering::AcqRel) == 1
            {
                let lock = hook_directory.join("goal-state.lock");
                fs::remove_file(&lock).unwrap();
                fs::create_dir(&lock).unwrap();
            }
        });
        let codex_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_codex_calls = Arc::clone(&codex_calls);

        let result = execute_shared_bounded_task_with_claim_port(
            root.to_string_lossy().to_string(),
            invocation,
            Channel::new(|_| Ok(())),
            Some(storage),
            registry,
            binding,
            |_, _| panic!("post-claim storage failure must not probe Codex"),
            move |_, _, _, _, _| {
                observed_codex_calls.fetch_add(1, Ordering::AcqRel);
                panic!("post-claim storage failure must not execute Codex")
            },
            controller_helper,
            &port,
        )
        .unwrap();

        assert_eq!(port.before_calls.load(Ordering::Acquire), 1);
        assert_eq!(directory_opens.load(Ordering::Acquire), 2);
        assert_eq!(codex_calls.load(Ordering::Acquire), 0);
        assert!(result
            .error
            .as_ref()
            .is_some_and(|error| error.code.starts_with("goal_")));
        assert_claimed_pre_spawn_repair(
            &result,
            CollaborationFailureClass::RepairRequired,
            SharedClaimRepairCause::GoalStorage,
        );
    }

    #[test]
    fn shared_wrapper_marks_committed_claim_for_repair_on_version_probe_failure() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let (root, registry, binding, invocation) = task016_shared_wrapper_inputs(project.path());
        let port = StatefulSharedClaimPort::new(false);
        let version_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_version_calls = Arc::clone(&version_calls);
        let codex_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_codex_calls = Arc::clone(&codex_calls);

        let result = execute_shared_bounded_task_with_claim_port(
            root.to_string_lossy().to_string(),
            invocation,
            Channel::new(|_| Ok(())),
            None,
            registry,
            binding,
            move |_, _| {
                observed_version_calls.fetch_add(1, Ordering::AcqRel);
                Err(ProcessRunFailure::new(
                    ProcessRunFailureKind::Cleanup,
                    "version-probe-sensitive-marker",
                ))
            },
            move |_, _, _, _, _| {
                observed_codex_calls.fetch_add(1, Ordering::AcqRel);
                panic!("a failed version probe must not execute Codex")
            },
            controller_helper,
            &port,
        )
        .unwrap();

        assert_eq!(port.before_calls.load(Ordering::Acquire), 1);
        assert_eq!(version_calls.load(Ordering::Acquire), 1);
        assert_eq!(codex_calls.load(Ordering::Acquire), 0);
        assert_claimed_pre_spawn_repair(
            &result,
            CollaborationFailureClass::RepairRequired,
            SharedClaimRepairCause::RuntimeCapability,
        );
        assert!(result
            .bounded
            .as_ref()
            .and_then(|bounded| bounded.runtime.as_ref())
            .is_some_and(|runtime| {
                runtime.outcome == RuntimeOutcome::CleanupFailed && !runtime.executed
            }));
        let serialized_claim = serde_json::to_string(&result.claim).unwrap();
        assert!(!serialized_claim.contains("version-probe-sensitive-marker"));
    }

    #[test]
    fn shared_wrapper_does_not_claim_pre_spawn_repair_after_codex_started() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let (root, registry, binding, invocation) = task016_shared_wrapper_inputs(project.path());
        let port = StatefulSharedClaimPort::new(false);
        let codex_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_codex_calls = Arc::clone(&codex_calls);

        let result = execute_shared_bounded_task_with_claim_port(
            root.to_string_lossy().to_string(),
            invocation,
            Channel::new(|_| Ok(())),
            None,
            registry,
            binding,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |_, _, _, _, _| {
                observed_codex_calls.fetch_add(1, Ordering::AcqRel);
                Err(ProcessRunFailure::new(
                    ProcessRunFailureKind::Cleanup,
                    "spawned provider cleanup failed",
                ))
            },
            controller_helper,
            &port,
        )
        .unwrap();

        assert_eq!(codex_calls.load(Ordering::Acquire), 1);
        assert!(result.codex_started);
        assert!(!result.stopped_before_runtime);
        assert!(matches!(
            result.claim,
            SharedClaimState::Claimed {
                remote_version: 8,
                recovered_from_readback: false,
            }
        ));
    }

    #[test]
    fn ha2ha_publish_write_sequence_stops_with_exact_partial_progress() {
        let files = vec![
            WorkspaceFile {
                path: "decisions/local-authority.md".into(),
                content: "decision".into(),
                content_type: "text/markdown".into(),
            },
            WorkspaceFile {
                path: "evidence/BR-015/source.md".into(),
                content: "evidence".into(),
                content_type: "text/markdown".into(),
            },
            WorkspaceFile {
                path: "tasks/BR-015.md".into(),
                content: "task".into(),
                content_type: "text/markdown".into(),
            },
        ];
        let mut calls = 0_u64;
        let (completed, failure) = apply_workspace_write_sequence(&files, |file| {
            calls += 1;
            if calls == 2 {
                Err("transport-failed")
            } else {
                Ok((file.path.clone(), calls, false))
            }
        });
        assert_eq!(calls, 2);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].path, "decisions/local-authority.md");
        assert_eq!(failure, Some("transport-failed"));
        assert!(!completed
            .iter()
            .any(|write| write.path.starts_with("tasks/")));
    }

    #[test]
    fn ha2ha_publish_records_reconciled_task_commit_and_completes() {
        let files = vec![
            WorkspaceFile {
                path: "decisions/local-authority.md".into(),
                content: "decision".into(),
                content_type: "text/markdown".into(),
            },
            WorkspaceFile {
                path: "evidence/BR-015/source.md".into(),
                content: "evidence".into(),
                content_type: "text/markdown".into(),
            },
            WorkspaceFile {
                path: "tasks/BR-015.md".into(),
                content: "task".into(),
                content_type: "text/markdown".into(),
            },
        ];
        let (completed, failure) = apply_workspace_write_sequence::<()>(&files, |file| {
            Ok((file.path.clone(), 1, file.path.starts_with("tasks/")))
        });
        assert!(failure.is_none());
        assert_eq!(completed.len(), 3);
        assert!(!completed[0].recovered_from_readback);
        assert!(completed[2].recovered_from_readback);
    }

    fn init_repo(root: &Path) {
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .unwrap()
            .success());
    }

    fn commit_repo(root: &Path) {
        for args in [
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Test User"],
            vec!["add", "."],
            vec!["commit", "-qm", "fixture"],
        ] {
            assert!(Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success());
        }
    }

    fn goal_repo() -> tempfile::TempDir {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        fs::write(
            project.path().join("tasks/issues/010.md"),
            "# 010: Goal\n\nStatus: complete\n\n## Acceptance Criteria\n\n- [x] done\n\n## Evidence Log\n\n| test | pass |\n\n## Verification Summary\n\nPassed.\n",
        )
        .unwrap();
        fs::write(
            project.path().join("tasks/sprint-1.md"),
            "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Evidence |\n| --- | --- | --- | --- |\n| 010 | Goal | complete | tasks/issues/010.md |\n",
        )
        .unwrap();
        commit_repo(project.path());
        project
    }

    fn goal_storage(state_dir: &tempfile::TempDir) -> GoalStateStorage {
        GoalStateStorage::new(fs::canonicalize(state_dir.path()).unwrap().join("app-data"))
    }

    fn persisted_goal(root: &Path, nonterminal: bool) -> PersistedGoalRecord {
        let task = fs::read_to_string(root.join("tasks/issues/010.md")).unwrap();
        PersistedGoalRecord {
            version: GOAL_STATE_VERSION,
            revision: 1,
            objective: "Safely resume one bounded goal".into(),
            repository: repository_identity(root).unwrap(),
            stop_conditions: goal_loop_stop_conditions(),
            current_run: PersistedRunCursor {
                run_id: "0123456789abcdef0123456789abcdef".into(),
                event_cursor: 7,
                nonterminal,
            },
            last_checkpoint: (!nonterminal).then(|| VerifiedGoalCheckpoint {
                task_path: "tasks/issues/010.md".into(),
                task_sha256: sha256_bytes(task.as_bytes()),
                git: git_fingerprint(root).unwrap(),
            }),
            evidence_references: vec![GoalEvidenceReference {
                path: "tasks/issues/010.md".into(),
                sha256: sha256_bytes(task.as_bytes()),
            }],
            collaboration: None,
        }
    }

    #[test]
    fn goal_persistence_recovers_clean_checkpoint_without_execution() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(project.path(), false))
            .unwrap();
        let clean = goal_recovery(&storage, project.path()).unwrap();
        assert_eq!(clean.state, GoalRecoveryState::Resumable);
        assert!(clean.explicit_confirmation_required);
        assert!(!clean.automatic_execution_started);
    }

    #[test]
    fn goal_persistence_detects_interrupted_and_stale_task_states() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(project.path(), true))
            .unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Interrupted
        );

        storage
            .write_for_test(persisted_goal(project.path(), false))
            .unwrap();
        fs::write(project.path().join("tasks/issues/010.md"), "# changed\n").unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::StaleTask
        );
    }

    #[test]
    fn goal_persistence_detects_head_index_and_worktree_changes() {
        for mutation in ["head", "index", "worktree"] {
            let project = goal_repo();
            let state_dir = tempfile::tempdir().unwrap();
            let storage = goal_storage(&state_dir);
            storage
                .write_for_test(persisted_goal(project.path(), false))
                .unwrap();
            match mutation {
                "head" => {
                    fs::write(project.path().join("other.txt"), "head").unwrap();
                    commit_repo(project.path());
                }
                "index" => {
                    fs::write(project.path().join("other.txt"), "index").unwrap();
                    assert!(Command::new("git")
                        .args(["add", "other.txt"])
                        .current_dir(project.path())
                        .status()
                        .unwrap()
                        .success());
                }
                _ => fs::write(project.path().join("other.txt"), "worktree").unwrap(),
            }
            assert_eq!(
                goal_recovery(&storage, project.path()).unwrap().state,
                GoalRecoveryState::GitChanged,
                "{mutation}"
            );
        }
    }

    #[test]
    fn goal_persistence_detects_missing_and_moved_repositories() {
        let parent = tempfile::tempdir().unwrap();
        let original = parent.path().join("original");
        fs::create_dir(&original).unwrap();
        init_repo(&original);
        assert!(Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://example.invalid/same.git"
            ])
            .current_dir(&original)
            .status()
            .unwrap()
            .success());
        fs::create_dir_all(original.join("tasks/issues")).unwrap();
        fs::write(original.join("tasks/issues/010.md"), "# task\n").unwrap();
        commit_repo(&original);
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(&original, false))
            .unwrap();
        let moved = parent.path().join("moved");
        fs::rename(&original, &moved).unwrap();
        assert_eq!(
            goal_recovery(&storage, &moved).unwrap().state,
            GoalRecoveryState::MovedRepository
        );

        assert!(Command::new("git")
            .args(["clone", "-q"])
            .arg(&moved)
            .arg(&original)
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .args([
                "remote",
                "set-url",
                "origin",
                "https://example.invalid/same.git",
            ])
            .current_dir(&original)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            goal_recovery(&storage, &original).unwrap().state,
            GoalRecoveryState::ReplacedRepository
        );

        let other = goal_repo();
        assert_eq!(
            goal_recovery(&storage, other.path()).unwrap().state,
            GoalRecoveryState::MissingRepository
        );

        #[cfg(unix)]
        {
            let unborn = parent.path().join("unborn");
            fs::create_dir(&unborn).unwrap();
            init_repo(&unborn);
            fs::create_dir_all(unborn.join("tasks/issues")).unwrap();
            fs::write(unborn.join("tasks/issues/010.md"), "# unborn\n").unwrap();
            storage
                .write_for_test(persisted_goal(&unborn, false))
                .unwrap();
            let unborn_moved = parent.path().join("unborn-moved");
            fs::rename(&unborn, &unborn_moved).unwrap();
            assert_eq!(
                goal_recovery(&storage, &unborn_moved).unwrap().state,
                GoalRecoveryState::MovedRepository
            );
        }
    }

    #[test]
    fn goal_persistence_classifies_corrupt_incompatible_and_oversized_records() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage.prepare_directory().unwrap();
        fs::write(storage.target(), b"{bad").unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Corrupt
        );
        fs::write(storage.target(), br#"{"version":2}"#).unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Incompatible
        );
        let mut value = serde_json::to_value(persisted_goal(project.path(), false)).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("status".into(), serde_json::json!("complete"));
        fs::write(storage.target(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Incompatible
        );
        fs::write(storage.target(), vec![b'x'; GOAL_STATE_MAX_BYTES + 1]).unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Oversized
        );
    }

    #[test]
    fn goal_record_is_bounded_and_serializes_no_shadow_authority_or_provider_payloads() {
        let project = goal_repo();
        let record = persisted_goal(project.path(), false);
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(
            value
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                "collaboration",
                "currentRun",
                "evidenceReferences",
                "lastCheckpoint",
                "objective",
                "repository",
                "revision",
                "stopConditions",
                "version"
            ]
        );
        let json = serde_json::to_string(&record).unwrap();
        for forbidden in [
            "sprint",
            "status",
            "gate",
            "rawPayload",
            "secret",
            "provider",
        ] {
            assert!(!json.contains(forbidden), "{forbidden}");
        }
        assert!(record.evidence_references.len() <= GOAL_EVIDENCE_MAX);
        assert!(json.len() <= GOAL_STATE_MAX_BYTES);

        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let mut oversized = record;
        oversized.evidence_references = (0..=GOAL_EVIDENCE_MAX)
            .map(|index| GoalEvidenceReference {
                path: format!("evidence/{index}"),
                sha256: "sha256:bounded".into(),
            })
            .collect();
        assert_eq!(
            storage.write_for_test(oversized).unwrap_err().code,
            "goal_state_oversized"
        );
    }

    fn persisted_completion_intent(root: &Path) -> RemoteCompletionIntent {
        let record = persisted_goal(root, false);
        let checkpoint = record.last_checkpoint.as_ref().unwrap();
        RemoteCompletionIntent::new(
            "workspace-goal-repair".into(),
            "codex-pax".into(),
            "BR-010".into(),
            "tasks/BR-010.md".into(),
            2,
            format!("sha256:{}", "a".repeat(64)),
            checkpoint.task_path.clone(),
            checkpoint.task_sha256.clone(),
            record.repository.repository_id.clone(),
            record.current_run.run_id.clone(),
            1,
            format!("evidence-{}", "1".repeat(32)),
            format!("evidence/BR-010/completion-{}.md", "1".repeat(32)),
            format!("handoff-{}", "2".repeat(32)),
            format!("logs/BR-010-handoff-{}.md", "2".repeat(32)),
            record
                .evidence_references
                .iter()
                .map(|artifact| CompletionArtifact {
                    path: artifact.path.clone(),
                    sha256: artifact.sha256.clone(),
                })
                .collect(),
        )
        .unwrap()
    }

    fn partial_completion_outcome(remote_version: u64) -> PostLocalCommitCollaborationOutcome {
        PostLocalCommitCollaborationOutcome {
            reconciliation: ReconciliationState::RepairRequired,
            evidence_handoff: EvidenceHandoffResult::Partial {
                remote_version: Some(remote_version),
                missing_effects: vec![
                    MissingCollaborationEffect::TaskUpdate,
                    MissingCollaborationEffect::HandoffWrite,
                    MissingCollaborationEffect::StatusWrite,
                ],
                repair: RepairHint::retry_sync(),
            },
        }
    }

    fn synchronized_completion_outcome(remote_version: u64) -> PostLocalCommitCollaborationOutcome {
        PostLocalCommitCollaborationOutcome {
            reconciliation: ReconciliationState::Reconciled,
            evidence_handoff: EvidenceHandoffResult::Synchronized {
                remote_version,
                evidence_ids: vec![EvidenceReferenceId::parse(format!(
                    "evidence-{}",
                    "1".repeat(32)
                ))
                .unwrap()],
                handoff_id: Some(
                    HandoffReferenceId::parse(format!("handoff-{}", "2".repeat(32))).unwrap(),
                ),
            },
        }
    }

    #[cfg(unix)]
    #[test]
    fn forged_internal_completion_intent_capability_alias_cannot_reach_goal_or_webview() {
        const OPAQUE_ALIAS: &str = "Q8nV3xK7mP2rT9yL4wC6dF1hJ5sB0gZu";
        let project = goal_repo();
        let project_key = project.path().to_string_lossy().to_string();
        let session_id = format!("local-session-{}", "9".repeat(32));
        let sessions = MdsyncSessionStore::default();
        sessions
            .insert_forged_session_for_test(
                &project_key,
                &session_id,
                "workspace-goal-repair",
                "codex-pax",
                OPAQUE_ALIAS,
            )
            .unwrap();
        let metadata = sessions
            .sanitized_session_metadata(&project_key, &session_id)
            .unwrap();
        assert!(!serde_json::to_string(&metadata)
            .unwrap()
            .contains(OPAQUE_ALIAS));

        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(project.path(), false))
            .unwrap();
        let checkpointed = storage.read_record().unwrap().unwrap();
        let before = storage.read_bytes().unwrap().unwrap();
        let mut intent = persisted_completion_intent(project.path());
        intent.task_id = OPAQUE_ALIAS.into();
        assert!(intent.validate().is_ok());

        let error = storage
            .begin_collaboration_reconciliation(&checkpointed, intent, &|candidate| {
                sessions
                    .validate_completion_intent_for_persistence(
                        &project_key,
                        &session_id,
                        candidate,
                    )
                    .map_err(|error| {
                        collaboration_project_error(
                            project.path(),
                            "intent",
                            collaboration_failure_from_transport(&error),
                        )
                    })
            })
            .unwrap_err();

        assert_eq!(
            error.code,
            "collaboration_intent_capability_material_rejected"
        );
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains(OPAQUE_ALIAS));
        let after = storage.read_bytes().unwrap().unwrap();
        assert_eq!(after, before);
        assert!(!String::from_utf8(after).unwrap().contains(OPAQUE_ALIAS));
        assert!(storage
            .read_record()
            .unwrap()
            .unwrap()
            .collaboration
            .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn goal_collaboration_cursor_persists_repair_debt_across_restart_and_local_solo_runs() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(project.path(), false))
            .unwrap();
        let checkpointed = storage.read_record().unwrap().unwrap();
        let intent = persisted_completion_intent(project.path());
        let pending = storage
            .begin_collaboration_reconciliation(&checkpointed, intent.clone(), &|_| Ok(()))
            .unwrap();
        assert_eq!(
            pending.collaboration.as_ref().unwrap().state,
            PersistedReconciliationState::SyncPending
        );
        assert_eq!(
            pending.collaboration.as_ref().unwrap().missing_effects,
            MissingCollaborationEffect::ORDER
        );

        let debt = storage
            .finish_collaboration_reconciliation(&pending, &partial_completion_outcome(3))
            .unwrap();
        let cursor = debt.collaboration.as_ref().unwrap();
        assert_eq!(
            cursor.state,
            PersistedReconciliationState::CollaborationRepairRequired
        );
        assert_eq!(
            cursor.missing_effects,
            vec![
                MissingCollaborationEffect::TaskUpdate,
                MissingCollaborationEffect::HandoffWrite,
                MissingCollaborationEffect::StatusWrite,
            ]
        );
        let recovery = goal_recovery(&storage, project.path()).unwrap();
        assert_eq!(
            recovery.collaboration.as_ref().unwrap().state,
            PersistedReconciliationState::CollaborationRepairRequired
        );
        assert!(!recovery.automatic_execution_started);
        assert_eq!(
            ensure_shared_iteration_has_no_repair_debt(&storage, project.path())
                .unwrap_err()
                .code,
            "collaboration_repair_required"
        );

        let mut local_run = persisted_goal(project.path(), true);
        local_run.revision = 0;
        local_run.current_run.run_id = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
        local_run.last_checkpoint = None;
        local_run.evidence_references.clear();
        let local_run = storage.create_run(Some(&debt), local_run).unwrap();
        assert_eq!(local_run.collaboration, debt.collaboration);
        assert!(local_run.current_run.nonterminal);
    }

    #[cfg(unix)]
    #[test]
    fn goal_collaboration_cursor_is_cas_bound_and_reconciled_cursor_unblocks_shared_mode() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(project.path(), false))
            .unwrap();
        let checkpointed = storage.read_record().unwrap().unwrap();
        let pending = storage
            .begin_collaboration_reconciliation(
                &checkpointed,
                persisted_completion_intent(project.path()),
                &|_| Ok(()),
            )
            .unwrap();
        assert_eq!(
            storage
                .begin_collaboration_reconciliation(
                    &checkpointed,
                    persisted_completion_intent(project.path()),
                    &|_| Ok(()),
                )
                .unwrap_err()
                .code,
            "goal_state_stale_write"
        );
        let reconciled = storage
            .finish_collaboration_reconciliation(&pending, &synchronized_completion_outcome(3))
            .unwrap();
        assert_eq!(
            reconciled.collaboration.as_ref().unwrap().state,
            PersistedReconciliationState::Reconciled
        );
        assert!(reconciled
            .collaboration
            .as_ref()
            .unwrap()
            .missing_effects
            .is_empty());
        ensure_shared_iteration_has_no_repair_debt(&storage, project.path()).unwrap();
        assert_eq!(
            storage
                .finish_collaboration_reconciliation(&pending, &synchronized_completion_outcome(3),)
                .unwrap_err()
                .code,
            "goal_state_stale_write"
        );
    }

    #[test]
    fn goal_collaboration_schema_is_bounded_and_rejects_capabilities_bodies_and_shadow_status() {
        let project = goal_repo();
        let mut record = persisted_goal(project.path(), false);
        record.collaboration = Some(PersistedCollaborationCursor {
            state: PersistedReconciliationState::CollaborationRepairRequired,
            intent: persisted_completion_intent(project.path()),
            current_task_version: 2,
            missing_effects: MissingCollaborationEffect::ORDER.to_vec(),
        });
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let bytes = storage.validate_record(&record).unwrap();
        let json = String::from_utf8(bytes).unwrap();
        assert!(json.len() <= GOAL_STATE_MAX_BYTES);
        for forbidden in [
            "sessionId",
            "webOrigin",
            "apiOrigin",
            "authorization",
            "Bearer ",
            "?edit=",
            "?k=",
            "remoteBody",
            "providerPayload",
            "taskStatus",
        ] {
            assert!(!json.contains(forbidden), "{forbidden}");
        }

        for (field, unsafe_value) in [
            ("actor", "Bearer opaque-value"),
            ("remoteTaskPath", "tasks/BR-010.md?edit=opaque"),
            ("evidencePath", "https://example.test/private"),
        ] {
            let mut value = serde_json::to_value(&record).unwrap();
            value["collaboration"]["intent"][field] = serde_json::json!(unsafe_value);
            assert!(serde_json::from_value::<PersistedGoalRecord>(value.clone()).is_ok());
            let parsed: PersistedGoalRecord = serde_json::from_value(value).unwrap();
            assert_eq!(
                storage.validate_record(&parsed).unwrap_err().code,
                "goal_state_incompatible",
                "{field}"
            );
        }

        let mut unknown_body = serde_json::to_value(&record).unwrap();
        unknown_body["collaboration"]["intent"]["remoteBody"] = serde_json::json!("opaque body");
        assert!(serde_json::from_value::<PersistedGoalRecord>(unknown_body).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn goal_collaboration_recovery_fails_closed_on_corrupt_or_unsafe_cursor() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage.prepare_directory().unwrap();
        let mut record = persisted_goal(project.path(), false);
        record.collaboration = Some(PersistedCollaborationCursor {
            state: PersistedReconciliationState::CollaborationRepairRequired,
            intent: persisted_completion_intent(project.path()),
            current_task_version: 2,
            missing_effects: MissingCollaborationEffect::ORDER.to_vec(),
        });
        let valid = serde_json::to_value(&record).unwrap();

        let mut cases = Vec::new();
        let mut stale_version = valid.clone();
        stale_version["collaboration"]["currentTaskVersion"] = serde_json::json!(1);
        cases.push(stale_version);
        let mut duplicate_effect = valid.clone();
        duplicate_effect["collaboration"]["missingEffects"] =
            serde_json::json!(["evidenceWrite", "evidenceWrite"]);
        cases.push(duplicate_effect);
        let mut unsafe_query = valid.clone();
        unsafe_query["collaboration"]["intent"]["remoteTaskPath"] =
            serde_json::json!("tasks/BR-010.md?edit=opaque");
        cases.push(unsafe_query);
        let mut unknown_body = valid;
        unknown_body["collaboration"]["intent"]["remoteBody"] =
            serde_json::json!("opaque remote body");
        cases.push(unknown_body);

        for value in cases {
            fs::write(storage.target(), serde_json::to_vec(&value).unwrap()).unwrap();
            let recovery = goal_recovery(&storage, project.path()).unwrap();
            assert_eq!(recovery.state, GoalRecoveryState::Incompatible);
            assert!(recovery.collaboration.is_none());
            assert!(!recovery.automatic_execution_started);
        }
    }

    #[test]
    fn collaboration_post_run_sequence_stops_at_every_write_failure_with_exact_prefix() {
        let effects = MissingCollaborationEffect::ORDER;
        let plan = ha2ha_envelope::PostRunReconciliationPlan {
            applied_effects: Vec::new(),
            writes: effects
                .iter()
                .enumerate()
                .map(|(index, effect)| PostRunEffectWrite {
                    effect: *effect,
                    path: format!("effect-{index}.md"),
                    content: format!("effect {index}"),
                    content_type: "text/markdown; charset=utf-8".into(),
                    base_version: if *effect == MissingCollaborationEffect::TaskUpdate {
                        Some(2)
                    } else {
                        None
                    },
                    expected_post_version: if *effect == MissingCollaborationEffect::TaskUpdate {
                        3
                    } else {
                        1
                    },
                })
                .collect(),
            current_task_version: 2,
        };

        for fail_at in 0..=effects.len() {
            let mut attempts = 0;
            let (completed, task_version, failure) = apply_post_run_write_sequence(&plan, |_| {
                let index = attempts;
                attempts += 1;
                if index == fail_at {
                    Err("write failed")
                } else {
                    Ok(())
                }
            });
            assert_eq!(completed, effects[..fail_at].to_vec(), "fail_at={fail_at}");
            assert_eq!(
                attempts,
                if fail_at < effects.len() {
                    fail_at + 1
                } else {
                    effects.len()
                },
                "fail_at={fail_at}"
            );
            assert_eq!(failure.is_some(), fail_at < effects.len());
            assert_eq!(task_version, if fail_at >= 2 { 3 } else { 2 });
        }
    }

    #[test]
    fn explicit_collaboration_repair_command_has_no_codex_runtime_path() {
        let source = include_str!("lib.rs");
        let start = source
            .find("#[tauri::command]\nfn repair_collaboration_completion(")
            .unwrap();
        let end = source[start..]
            .find("\nasync fn run_bounded_task_worker")
            .map(|offset| start + offset)
            .unwrap();
        let repair_source = &source[start..end];
        for forbidden in [
            "CODEX_EXECUTABLE",
            "execute_runtime_with(",
            "execute_bounded_task_command",
            "run_bounded_process(",
            "Command::new(",
        ] {
            assert!(!repair_source.contains(forbidden), "{forbidden}");
        }
        assert!(repair_source.contains("codex_started: false"));
        assert!(repair_source.contains("confirmed"));
    }

    #[test]
    fn goal_storage_atomic_recovery_ignores_temporary_files_and_serializes_writers() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        storage
            .write_for_test(persisted_goal(project.path(), false))
            .unwrap();
        fs::write(
            storage.directory.join(".goal-state.crashed.tmp"),
            b"partial",
        )
        .unwrap();
        assert_eq!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Resumable
        );
        let mut threads = Vec::new();
        for index in 0..8 {
            let storage = storage.clone();
            let mut record = persisted_goal(project.path(), false);
            record.objective = format!("objective-{index}");
            threads.push(thread::spawn(move || {
                storage.write_for_test(record).unwrap()
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        let final_record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert!(final_record.objective.starts_with("objective-"));
    }

    #[test]
    fn goal_storage_cas_rejects_stale_runs_cursor_regression_and_post_clear_writes() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let mut run_a = persisted_goal(project.path(), true);
        run_a.revision = 0;
        run_a.current_run.run_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
        run_a.current_run.event_cursor = 0;
        let run_a = storage.create_run(None, run_a).unwrap();
        let stale_a = run_a.clone();
        let run_a = storage.advance_event(&run_a, 3).unwrap();

        let mut run_b = persisted_goal(project.path(), true);
        run_b.revision = 0;
        run_b.current_run.run_id = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
        run_b.current_run.event_cursor = 0;
        let run_b = storage.create_run(Some(&run_a), run_b).unwrap();
        assert!(run_b.revision > run_a.revision);

        assert_eq!(
            storage.advance_event(&stale_a, 4).unwrap_err().code,
            "goal_state_stale_write"
        );
        assert_eq!(
            storage
                .finish_run(
                    &run_a,
                    4,
                    persisted_goal(project.path(), false)
                        .last_checkpoint
                        .unwrap(),
                    Vec::new(),
                )
                .unwrap_err()
                .code,
            "goal_state_stale_write"
        );

        let run_b = storage.advance_event(&run_b, 5).unwrap();
        assert_eq!(
            storage.advance_event(&run_b, 4).unwrap_err().code,
            "goal_state_cursor_regression"
        );
        storage.clear().unwrap();
        let mut recreated = persisted_goal(project.path(), true);
        recreated.revision = 0;
        recreated.current_run.run_id = "cccccccccccccccccccccccccccccccc".into();
        assert_eq!(
            storage
                .create_run(Some(&run_b), recreated)
                .unwrap_err()
                .code,
            "goal_state_stale_write"
        );
        assert_eq!(
            storage.advance_event(&run_b, 6).unwrap_err().code,
            "goal_state_stale_write"
        );
        assert_eq!(
            storage
                .finish_run(
                    &run_b,
                    6,
                    persisted_goal(project.path(), false)
                        .last_checkpoint
                        .unwrap(),
                    Vec::new(),
                )
                .unwrap_err()
                .code,
            "goal_state_stale_write"
        );
        assert!(storage.read_bytes().unwrap().is_none());
    }

    #[test]
    fn goal_storage_next_run_preserves_verified_checkpoint_evidence_references() {
        let project = goal_repo();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let mut checkpointed = persisted_goal(project.path(), false);
        checkpointed.revision = 0;
        let checkpointed = storage.create_run(None, checkpointed).unwrap();
        let expected_references = checkpointed.evidence_references.clone();

        let mut next = persisted_goal(project.path(), true);
        next.revision = 0;
        next.current_run.run_id = "11111111111111111111111111111111".into();
        next.last_checkpoint = None;
        next.evidence_references.clear();
        let next = storage.create_run(Some(&checkpointed), next).unwrap();

        assert_eq!(next.last_checkpoint, checkpointed.last_checkpoint);
        assert_eq!(next.evidence_references, expected_references);
    }

    #[cfg(unix)]
    #[test]
    fn goal_storage_barrier_symlink_swaps_fail_closed_for_read_write_and_clear() {
        use std::os::unix::fs::symlink;
        use std::sync::Barrier;

        #[derive(Clone, Copy)]
        enum Operation {
            Read,
            Write,
            Clear,
        }

        for (phase, operation) in [
            (GoalStorageTestPhase::DirectoryOpened, Operation::Read),
            (GoalStorageTestPhase::DirectoryOpened, Operation::Write),
            (GoalStorageTestPhase::DirectoryOpened, Operation::Clear),
            (GoalStorageTestPhase::LockOpened, Operation::Read),
            (GoalStorageTestPhase::LockOpened, Operation::Write),
            (GoalStorageTestPhase::LockOpened, Operation::Clear),
            (GoalStorageTestPhase::StateOpened, Operation::Read),
            (GoalStorageTestPhase::StateOpened, Operation::Write),
            (GoalStorageTestPhase::StateOpened, Operation::Clear),
        ] {
            let project = goal_repo();
            let state_dir = tempfile::tempdir().unwrap();
            let storage = goal_storage(&state_dir);
            let mut initial = persisted_goal(project.path(), true);
            initial.revision = 0;
            initial.current_run.event_cursor = 0;
            let expected = storage.create_run(None, initial).unwrap();
            let outside = fs::canonicalize(state_dir.path()).unwrap().join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("sentinel"), b"outside-unchanged").unwrap();
            fs::write(outside.join("goal-state.json"), b"outside-state").unwrap();
            fs::write(outside.join("goal-state.lock"), b"outside-lock").unwrap();

            let entered = Arc::new(Barrier::new(2));
            let released = Arc::new(Barrier::new(2));
            let fired = Arc::new(AtomicBool::new(false));
            let hooked = storage.clone().with_test_hook({
                let entered = Arc::clone(&entered);
                let released = Arc::clone(&released);
                let fired = Arc::clone(&fired);
                move |observed| {
                    if observed == phase && !fired.swap(true, Ordering::AcqRel) {
                        entered.wait();
                        released.wait();
                    }
                }
            });
            let expected_for_thread = expected.clone();
            let worker = thread::spawn(move || match operation {
                Operation::Read => hooked.read_bytes().map(|_| ()),
                Operation::Write => hooked.advance_event(&expected_for_thread, 1).map(|_| ()),
                Operation::Clear => hooked.clear(),
            });
            entered.wait();
            match phase {
                GoalStorageTestPhase::DirectoryOpened => {
                    let displaced = storage.directory.with_extension("displaced");
                    fs::rename(&storage.directory, &displaced).unwrap();
                    symlink(&outside, &storage.directory).unwrap();
                }
                GoalStorageTestPhase::LockOpened => {
                    fs::remove_file(storage.directory.join("goal-state.lock")).unwrap();
                    symlink(
                        outside.join("goal-state.lock"),
                        storage.directory.join("goal-state.lock"),
                    )
                    .unwrap();
                }
                GoalStorageTestPhase::StateOpened => {
                    fs::rename(
                        storage.directory.join("goal-state.json"),
                        storage.directory.join("goal-state.displaced"),
                    )
                    .unwrap();
                    symlink(
                        outside.join("goal-state.json"),
                        storage.directory.join("goal-state.json"),
                    )
                    .unwrap();
                }
                GoalStorageTestPhase::TemporarySynced => unreachable!(),
            }
            released.wait();
            let error = worker.join().unwrap().unwrap_err();
            assert!(
                matches!(
                    error.code.as_str(),
                    "goal_storage_symlink" | "goal_storage_raced" | "goal_state_stale_write"
                ),
                "phase={phase:?} code={}",
                error.code
            );
            assert_eq!(
                fs::read(outside.join("sentinel")).unwrap(),
                b"outside-unchanged"
            );
            assert_eq!(
                fs::read(outside.join("goal-state.json")).unwrap(),
                b"outside-state"
            );
            assert_eq!(
                fs::read(outside.join("goal-state.lock")).unwrap(),
                b"outside-lock"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn goal_storage_rejects_regular_and_symlink_temp_source_swaps_after_sync() {
        use std::os::unix::fs::symlink;
        use std::sync::Barrier;

        for symlink_swap in [false, true] {
            let project = goal_repo();
            let state_dir = tempfile::tempdir().unwrap();
            let storage = goal_storage(&state_dir);
            let mut initial = persisted_goal(project.path(), true);
            initial.revision = 0;
            initial.current_run.event_cursor = 0;
            let expected = storage.create_run(None, initial).unwrap();
            let original_state = storage.read_bytes().unwrap().unwrap();
            let outside = fs::canonicalize(state_dir.path())
                .unwrap()
                .join("outside-temp-source");
            fs::write(&outside, b"outside-unchanged").unwrap();

            let entered = Arc::new(Barrier::new(2));
            let released = Arc::new(Barrier::new(2));
            let fired = Arc::new(AtomicBool::new(false));
            let hooked = storage.clone().with_test_hook({
                let entered = Arc::clone(&entered);
                let released = Arc::clone(&released);
                let fired = Arc::clone(&fired);
                move |phase| {
                    if phase == GoalStorageTestPhase::TemporarySynced
                        && !fired.swap(true, Ordering::AcqRel)
                    {
                        entered.wait();
                        released.wait();
                    }
                }
            });
            let worker = thread::spawn(move || hooked.advance_event(&expected, 1));
            entered.wait();
            let temporary = fs::read_dir(&storage.directory)
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name().is_some_and(|name| {
                        let name = name.to_string_lossy();
                        name.starts_with(".goal-state.") && name.ends_with(".tmp")
                    })
                })
                .expect("synced temporary pathname");
            let displaced = storage.directory.join(if symlink_swap {
                "opened-temp-symlink-swap"
            } else {
                "opened-temp-regular-swap"
            });
            fs::rename(&temporary, displaced).unwrap();
            if symlink_swap {
                symlink(&outside, &temporary).unwrap();
            } else {
                fs::write(&temporary, b"attacker-regular-replacement").unwrap();
            }
            released.wait();
            let error = worker.join().unwrap().unwrap_err();
            assert_eq!(
                error.code,
                if symlink_swap {
                    "goal_storage_symlink"
                } else {
                    "goal_state_stale_write"
                }
            );
            assert_eq!(storage.read_bytes().unwrap().unwrap(), original_state);
            assert_eq!(fs::read(&outside).unwrap(), b"outside-unchanged");
        }
    }

    #[cfg(unix)]
    #[test]
    fn goal_storage_rejects_symlinked_state_and_storage_directory() {
        use std::os::unix::fs::symlink;
        let state_dir = tempfile::tempdir().unwrap();
        let state_root = fs::canonicalize(state_dir.path()).unwrap();
        let real = state_root.join("real");
        fs::create_dir(&real).unwrap();
        let linked_dir = state_root.join("linked");
        symlink(&real, &linked_dir).unwrap();
        assert_eq!(
            GoalStateStorage::new(linked_dir)
                .read_bytes()
                .unwrap_err()
                .code,
            "goal_storage_symlink"
        );

        let storage = GoalStateStorage::new(real);
        fs::write(state_root.join("outside"), b"{}").unwrap();
        symlink(state_root.join("outside"), storage.target()).unwrap();
        assert_eq!(
            storage.read_bytes().unwrap_err().code,
            "goal_storage_symlink"
        );
        fs::remove_file(storage.target()).unwrap();
        symlink(state_root.join("missing-target"), storage.target()).unwrap();
        assert_eq!(
            storage.read_bytes().unwrap_err().code,
            "goal_storage_symlink"
        );
    }

    fn write_contract_fixture(root: &Path, contract: serde_json::Value) {
        fs::create_dir_all(root.join("skill-ui")).unwrap();
        fs::create_dir_all(root.join(".agents/skills").join(TEST_SKILL)).unwrap();
        fs::write(
            root.join(".agents/skills")
                .join(TEST_SKILL)
                .join("SKILL.md"),
            "# Test skill",
        )
        .unwrap();
        fs::write(
            root.join("skills-lock.json"),
            serde_json::json!({ "version": 1, "skills": { TEST_SKILL: { "source": "test/source", "computedHash": TEST_HASH } } }).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("skill-ui").join(format!("{TEST_SKILL}.json")),
            contract.to_string(),
        )
        .unwrap();
    }

    fn valid_contract() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "id": TEST_SKILL,
            "name": "Build Right preflight",
            "lifecyclePhase": "Discover",
            "purpose": "Validate repository readiness.",
            "reads": ["docs/"],
            "writes": ["tasks/"],
            "decisions": ["ready"],
            "helpers": [{ "id": "preflight-check", "execution": "explicit-user-action" }],
            "requiredEvidence": ["readiness result"],
            "stopStates": ["blocked"],
            "renderer": "operating-card",
            "provenance": {
                "source": "test/source",
                "installedPath": ".agents/skills/build-right-preflight/SKILL.md",
                "lockHash": TEST_HASH
            }
        })
    }

    fn write_helper_fixture(root: &Path) {
        fs::create_dir_all(root.join("skill-ui")).unwrap();
        fs::create_dir_all(root.join("tasks/issues")).unwrap();
        let entries = [
            ("build-right-preflight", "Discover", vec!["preflight-check"]),
            (
                "build-right-execution",
                "Build",
                vec!["continue-check", "execution-check"],
            ),
        ];
        let mut locked = serde_json::Map::new();
        for (id, phase, helpers) in entries {
            let hash = format!("hash-{id}");
            let skill_root = root.join(".agents/skills").join(id);
            fs::create_dir_all(skill_root.join("scripts")).unwrap();
            fs::write(
                skill_root.join("SKILL.md"),
                format!("---\nname: {id}\ndescription: fixture for {id}\n---\n\n# {id}\n\nDeclared helpers are authorized only by the matching validated UI contract.\n"),
            ).unwrap();
            for helper in &helpers {
                let bytes: &[u8] = match *helper {
                    "preflight-check" => include_bytes!(
                        "../../.agents/skills/build-right-preflight/scripts/preflight-check.ts"
                    ),
                    "continue-check" => include_bytes!(
                        "../../.agents/skills/build-right-execution/scripts/continue-check.ts"
                    ),
                    "execution-check" => include_bytes!(
                        "../../.agents/skills/build-right-execution/scripts/execution-check.ts"
                    ),
                    _ => unreachable!(),
                };
                fs::write(
                    skill_root.join("scripts").join(format!("{helper}.ts")),
                    bytes,
                )
                .unwrap();
            }
            fs::write(
                root.join("skill-ui").join(format!("{id}.json")),
                serde_json::json!({
                    "version": 1,
                    "id": id,
                    "name": id,
                    "lifecyclePhase": phase,
                    "purpose": "fixture helper contract",
                    "reads": ["docs/"],
                    "writes": ["tasks/"],
                    "decisions": ["ready"],
                    "helpers": helpers.iter().map(|helper| serde_json::json!({ "id": helper, "execution": "explicit-user-action" })).collect::<Vec<_>>(),
                    "requiredEvidence": ["result"],
                    "stopStates": ["blocked"],
                    "renderer": "operating-card",
                    "provenance": {
                        "source": SKILL_SETUP_SOURCE,
                        "installedPath": format!(".agents/skills/{id}/SKILL.md"),
                        "lockHash": hash
                    }
                }).to_string(),
            ).unwrap();
            locked.insert(
                id.into(),
                serde_json::json!({ "source": SKILL_SETUP_SOURCE, "computedHash": hash }),
            );
        }
        fs::write(
            root.join("skills-lock.json"),
            serde_json::json!({ "version": 1, "skills": locked }).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("tasks/issues/007-test.md"),
            "# 007: Test\n\nStatus: ready\n",
        )
        .unwrap();
    }

    fn helper_invocation(helper_id: HelperId) -> HelperInvocation {
        HelperInvocation {
            helper_id,
            mode: None,
            task_path: None,
            feature_request: None,
        }
    }

    fn helper_process_output(
        success: bool,
        stdout: &[u8],
        termination: ProcessTermination,
        truncated: bool,
    ) -> BoundedProcessOutput {
        let status = Command::new(if success {
            "/usr/bin/true"
        } else {
            "/usr/bin/false"
        })
        .status()
        .unwrap();
        BoundedProcessOutput {
            status,
            termination,
            stdout: stdout.to_vec(),
            stderr: if success {
                vec![]
            } else {
                b"fixture failure".to_vec()
            },
            stdout_truncated: truncated,
            stderr_truncated: false,
        }
    }

    fn runtime_output(
        success: bool,
        stdout: &[u8],
        stderr: &[u8],
        termination: ProcessTermination,
        truncated: bool,
    ) -> BoundedProcessOutput {
        let status = Command::new(if success {
            "/usr/bin/true"
        } else {
            "/usr/bin/false"
        })
        .status()
        .unwrap();
        BoundedProcessOutput {
            status,
            termination,
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
            stdout_truncated: truncated,
            stderr_truncated: false,
        }
    }

    fn runtime_invocation(mode: RuntimeMode) -> RuntimeInvocation {
        RuntimeInvocation {
            mode,
            prompt: (mode == RuntimeMode::Live).then(|| "Inspect only; report evidence.".into()),
            confirmed: mode == RuntimeMode::Live,
        }
    }

    fn controller_helper_result(
        root: &Path,
        decision: &str,
        warnings: Vec<String>,
    ) -> HelperResult {
        HelperResult {
            helper_id: HelperId::ContinueCheck,
            mode: None,
            task_path: None,
            executable: "bun".into(),
            argv: Vec::new(),
            outcome: HelperOutcome::Completed,
            executed: true,
            success: true,
            exit_status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            decision: Some(HelperDecision {
                decision: decision.into(),
                confidence: if decision == "stop" { "medium" } else { "high" }.into(),
                next_action: "stop after one task".into(),
                evidence: Vec::new(),
                warnings,
                recommended_destination: None,
                blocking_gates: None,
                founder_questions: None,
                research_triggers: None,
                ready_task_candidates: None,
            }),
            failure: None,
            project: inspect_project_path(root),
        }
    }

    fn loop_resolver(
        root: &Path,
        decision: &str,
        next_task: Option<&str>,
        gates: &[(&str, &str)],
    ) -> HelperResult {
        let mut resolver = controller_helper_result(root, decision, Vec::new());
        resolver.stdout = serde_json::json!({
            "decision": decision,
            "confidence": "high",
            "nextAction": format!("Handle {decision}"),
            "nextTask": next_task.map(|path| serde_json::json!({
                "path": path,
                "status": "ready",
                "owner": "AI",
                "missingContractFields": []
            })),
            "blockingGates": gates.iter().map(|(source, reason)| serde_json::json!({
                "source": source,
                "reason": reason
            })).collect::<Vec<_>>()
        })
        .to_string();
        resolver
    }

    fn controller_task_text(status: &str, checked: bool) -> String {
        format!(
            "# 009: Fixture task\n\nStatus: {status}\nType: feature\nOwner: AI\n\nAssumption basis: repo-evidence-backed\nRequirement basis: docs/execution-rules.md\nReversibility: easy\nLearning objective: controller fixture\nSource under test: repo-local path\n\n## Goal\n\nExercise one task.\n\n## Non-Goals\n\n- Select another task.\n\n## Required Reading\n\n- docs/execution-rules.md\n\n## Acceptance Criteria\n\n- [{}] repository proof\n\n## Baseline Evidence\n\nFixture baseline.\n\n## Verification\n\nRun fixture checks.\n\n## Evidence Log\n\n| command | result | notes |\n| --- | --- | --- |\n{}\n\n## Verification Summary\n\n{}\n\n## Blockers\n\n- None.\n\n## Follow-Ups\n\n- None.\n",
            if checked { "x" } else { " " },
            if checked { "| fixture | pass | proved |" } else { "" },
            if checked { "Focused checks passed." } else { "Not run yet." },
        )
    }

    fn write_controller_repository(root: &Path) {
        init_repo(root);
        write_helper_fixture(root);
        let _ = fs::remove_file(root.join("tasks/issues/007-test.md"));
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/execution-rules.md"), "# Execution Rules\n").unwrap();
        fs::write(root.join("docs/blueprint-status.md"), "Status: active\n").unwrap();
        fs::write(root.join("docs/release-gates.md"), "Status: active\n").unwrap();
        fs::write(
            root.join("docs/conflicts.md"),
            "Status: resolved\n\n## Conflicts\n\n| Conflict | Status | Owner |\n| --- | --- | --- |\n| None | resolved | AI |\n",
        )
        .unwrap();
        fs::write(
            root.join("tasks/issues/009-fixture.md"),
            controller_task_text("ready", false),
        )
        .unwrap();
        fs::write(
            root.join("tasks/sprint-1.md"),
            "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | ready | - | tasks/issues/009-fixture.md |\n",
        )
        .unwrap();
    }

    fn prepare_second_controller_task(root: &Path) {
        fs::write(
            root.join("tasks/issues/009-fixture.md"),
            controller_task_text("complete", true),
        )
        .unwrap();
        fs::write(
            root.join("tasks/issues/010-fixture.md"),
            controller_task_text("ready", false).replace("# 009:", "# 010:"),
        )
        .unwrap();
        fs::write(
            root.join("tasks/sprint-1.md"),
            "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | First fixture | complete | - | tasks/issues/009-fixture.md |\n| 010 | Second fixture | ready | 009 | tasks/issues/010-fixture.md |\n",
        )
        .unwrap();
    }

    fn complete_second_controller_task(root: &Path) {
        fs::write(
            root.join("tasks/issues/010-fixture.md"),
            controller_task_text("complete", true).replace("# 009:", "# 010:"),
        )
        .unwrap();
        fs::write(
            root.join("tasks/sprint-1.md"),
            "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | First fixture | complete | - | tasks/issues/009-fixture.md |\n| 010 | Second fixture | complete | 009 | tasks/issues/010-fixture.md |\n",
        )
        .unwrap();
    }

    fn run_controller_fixture_trial(root: &Path, effect: impl FnOnce(&Path)) -> BoundedTaskResult {
        let cancel = AtomicBool::new(false);
        let preview = build_bounded_task_preview(root, &cancel).unwrap();
        assert!(preview.executable);
        let preview_again = build_bounded_task_preview(root, &cancel).unwrap();
        assert_eq!(preview.preview_token, preview_again.preview_token);
        assert_eq!(preview.selected_task, preview_again.selected_task);
        let runtime = execute_runtime_with_argv(
            root,
            RuntimeInvocation {
                mode: RuntimeMode::Fixture,
                prompt: None,
                confirmed: true,
            },
            true,
            None,
            || panic!("fixture must not probe runtime"),
            |_| panic!("fixture must not spawn provider"),
        );
        effect(root);
        finish_bounded_task(
            root,
            preview.selected_task,
            Some(runtime),
            None,
            &AtomicBool::new(false),
        )
    }

    fn run_persisted_controller_recovery_trial(
        tracker: &str,
        additional_tasks: &[(&str, String)],
    ) -> (BoundedTaskResult, PersistedGoalRecord, GoalRecovery) {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let canonical = validated_repository_root(&project.path().to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            canonical.to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let tracker = tracker.to_string();
        let additional_tasks = additional_tasks
            .iter()
            .map(|(path, content)| ((*path).to_string(), content.clone()))
            .collect::<Vec<_>>();
        let result = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                fs::write(
                    root.join("tasks/issues/009-fixture.md"),
                    controller_task_text("complete", true),
                )
                .unwrap();
                for (path, content) in additional_tasks {
                    fs::write(root.join(path), content).unwrap();
                }
                fs::write(root.join("tasks/sprint-1.md"), tracker).unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        let record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        let recovery = goal_recovery(&storage, project.path()).unwrap();
        (result, record, recovery)
    }

    #[test]
    fn bounded_controller_owns_isolated_workspace_write_argv_without_changing_generic_runtime() {
        let root = Path::new("/tmp/controller-root");
        let prompt = bounded_task_prompt("tasks/issues/009-test.md");
        let generic = runtime_argv(root, &prompt);
        let bounded = bounded_task_runtime_argv(root, &prompt);
        assert!(generic
            .windows(2)
            .any(|pair| pair == ["--sandbox", "read-only"]));
        assert!(bounded
            .windows(2)
            .any(|pair| pair == ["--sandbox", "workspace-write"]));
        for feature in ["plugins", "remote_plugin", "apps"] {
            assert!(!generic
                .windows(2)
                .any(|pair| pair == ["--disable", feature]));
            assert!(bounded
                .windows(2)
                .any(|pair| pair == ["--disable", feature]));
        }
        assert_eq!(bounded.last(), Some(&prompt));
        assert!(prompt.contains("Execute exactly the selected task at tasks/issues/009-test.md"));
        assert!(prompt.contains("Do not select or begin another task"));
    }

    #[test]
    fn goal_loop_types_two_confirmed_iterations_then_every_terminal_stop_family() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let snapshot = inspect_project_path(project.path());

        for next in ["tasks/issues/010.md", "tasks/issues/011.md"] {
            let resolver = loop_resolver(project.path(), "execute-task", Some(next), &[]);
            let transition = result_loop_projection(
                project.path(),
                BoundedTaskOutcome::Verified,
                true,
                "verified",
                &snapshot,
                Some("tasks/issues/009.md"),
                Some(&resolver),
                None,
            );
            assert_eq!(transition.state, GoalLoopState::ContinueAvailable);
            assert_eq!(transition.next_task.as_deref(), Some(next));
            assert!(transition.explicit_confirmation_required);
            assert!(!transition.automatic_execution_started);
        }

        let resolver_cases = [
            ("ask-founder", &[][..], GoalLoopState::FounderStop),
            ("wait-external", &[][..], GoalLoopState::ExternalStop),
            ("no-ready-task", &[][..], GoalLoopState::NoReadyTaskStop),
            ("invalid-state", &[][..], GoalLoopState::InvalidStateStop),
            (
                "invalid-state",
                &[("docs/conflicts.md", "open material conflict")][..],
                GoalLoopState::ConflictStop,
            ),
        ];
        for (decision, gates, expected) in resolver_cases {
            let resolver = loop_resolver(project.path(), decision, None, gates);
            let transition = result_loop_projection(
                project.path(),
                BoundedTaskOutcome::Verified,
                true,
                decision,
                &snapshot,
                Some("tasks/issues/009.md"),
                Some(&resolver),
                None,
            );
            assert_eq!(transition.state, expected, "{decision}");
            assert!(!transition.explicit_confirmation_required);
            assert!(!transition.automatic_execution_started);
        }

        let mut cancelled = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        cancelled.outcome = RuntimeOutcome::Cancelled;
        let terminal_cases = [
            (
                BoundedTaskOutcome::Stopped,
                Some(&cancelled),
                "cancelled",
                GoalLoopState::CancelledStop,
            ),
            (
                BoundedTaskOutcome::Stopped,
                None,
                "task source changed after preview",
                GoalLoopState::StaleStop,
            ),
            (
                BoundedTaskOutcome::VerificationFailed,
                None,
                "verification failed",
                GoalLoopState::FailureStop,
            ),
        ];
        for (outcome, runtime, reason, expected) in terminal_cases {
            let cancellation_phase = runtime
                .is_some_and(|runtime| runtime.outcome == RuntimeOutcome::Cancelled)
                .then_some(BoundedTaskCancellationPhase::ProviderRuntime);
            let transition = result_loop_projection(
                project.path(),
                outcome,
                false,
                reason,
                &snapshot,
                Some("tasks/issues/009.md"),
                None,
                cancellation_phase,
            );
            assert_eq!(transition.state, expected);
            assert!(!transition.explicit_confirmation_required);
        }

        write_controller_repository(project.path());
        fs::write(
            project.path().join("tasks/issues/009-fixture.md"),
            controller_task_text("complete", true),
        )
        .unwrap();
        fs::write(
            project.path().join("tasks/sprint-1.md"),
            "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
        )
        .unwrap();
        let completed_snapshot = inspect_project_path(project.path());
        let completed = result_loop_projection(
            project.path(),
            BoundedTaskOutcome::Verified,
            true,
            "verified",
            &completed_snapshot,
            Some("tasks/issues/009-fixture.md"),
            None,
            None,
        );
        assert_eq!(completed.state, GoalLoopState::GoalComplete);
        assert!(!completed.explicit_confirmation_required);
    }

    #[test]
    fn goal_loop_never_continues_when_execute_task_carries_a_blocking_gate() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let snapshot = inspect_project_path(project.path());
        for (gate, expected) in [
            (
                ("docs/conflicts.md", "open material conflict"),
                GoalLoopState::ConflictStop,
            ),
            (
                ("tasks/sprint-1.md", "unresolved blocking gate"),
                GoalLoopState::InvalidStateStop,
            ),
        ] {
            let resolver = loop_resolver(
                project.path(),
                "execute-task",
                Some("tasks/issues/010.md"),
                &[gate],
            );

            let transition = result_loop_projection(
                project.path(),
                BoundedTaskOutcome::Verified,
                true,
                "verified",
                &snapshot,
                Some("tasks/issues/009.md"),
                Some(&resolver),
                None,
            );

            assert_eq!(transition.state, expected);
            assert!(transition.next_task.is_none());
            assert!(!transition.explicit_confirmation_required);
        }
    }

    #[test]
    fn bounded_controller_rejects_sequential_replay_of_one_confirmation() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let token = preview.preview_token;
        let selected_task = preview.selected_task.unwrap();
        let effects = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let restarted_effects = Arc::clone(&effects);
        let after_restart = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: token.clone(),
                selected_task: selected_task.clone(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            None,
            Arc::new(OperationRegistry::default()),
            move |_, _| {
                restarted_effects.fetch_add(100, Ordering::AcqRel);
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |_, _, _, _, _| unreachable!("an app restart must invalidate prior confirmation"),
        );
        assert!(matches!(
            after_restart,
            Err(error) if error.code == "controller_confirmation_consumed_or_stale"
        ));

        let first_effects = Arc::clone(&effects);
        execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: token.clone(),
                selected_task: selected_task.clone(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            None,
            Arc::clone(&registry),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |_, _, _, _, _| {
                first_effects.fetch_add(1, Ordering::AcqRel);
                Ok(runtime_output(
                    false,
                    b"",
                    b"fixture failure",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();

        let replay_effects = Arc::clone(&effects);
        let replay = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: token,
                selected_task,
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            None,
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |_, _, _, _, _| {
                replay_effects.fetch_add(1, Ordering::AcqRel);
                Ok(runtime_output(
                    false,
                    b"",
                    b"fixture failure",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        );

        assert!(matches!(
            replay,
            Err(error) if error.code == "controller_confirmation_consumed_or_stale"
        ));
        assert_eq!(effects.load(Ordering::Acquire), 1);
    }

    #[test]
    fn bounded_controller_production_path_runs_two_fresh_confirmations_to_goal_complete() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let registry = Arc::new(OperationRegistry::default());
        let effects = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let first_preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        assert_eq!(
            first_preview.selected_task.as_deref(),
            Some("tasks/issues/009-fixture.md")
        );
        let first_token = first_preview.preview_token.clone();
        let first_effects = Arc::clone(&effects);
        let first = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: first_preview.preview_token,
                selected_task: first_preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            Arc::clone(&registry),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                first_effects.fetch_add(1, Ordering::AcqRel);
                prepare_second_controller_task(root);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(
            first.outcome,
            BoundedTaskOutcome::Verified,
            "{}",
            first.reason
        );
        assert_eq!(first.loop_state.state, GoalLoopState::ContinueAvailable);
        assert_eq!(
            first.loop_state.next_task.as_deref(),
            Some("tasks/issues/010-fixture.md")
        );
        assert!(first.loop_state.explicit_confirmation_required);
        let first_record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert_eq!(
            first_record
                .last_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.task_path.as_str()),
            Some("tasks/issues/009-fixture.md")
        );

        let second_preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        assert_ne!(second_preview.preview_token, first_token);
        assert_eq!(
            second_preview.selected_task.as_deref(),
            Some("tasks/issues/010-fixture.md")
        );
        let second_effects = Arc::clone(&effects);
        let second = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: second_preview.preview_token,
                selected_task: second_preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                second_effects.fetch_add(1, Ordering::AcqRel);
                complete_second_controller_task(root);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(
            second.outcome,
            BoundedTaskOutcome::Verified,
            "{}",
            second.reason
        );
        assert_eq!(second.loop_state.state, GoalLoopState::GoalComplete);
        assert!(!second.loop_state.explicit_confirmation_required);
        assert_eq!(effects.load(Ordering::Acquire), 2);
        let final_record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert!(final_record.revision > first_record.revision);
        assert!(!final_record.current_run.nonterminal);
        assert_eq!(
            final_record
                .last_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.task_path.as_str()),
            Some("tasks/issues/010-fixture.md")
        );
        assert!(final_record
            .evidence_references
            .iter()
            .any(|reference| reference.path == "tasks/sprint-1.md"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_controller_production_runner_drains_and_persists_harmless_jsonl() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        fs::write(
            project.path().join(".completed-task.fixture"),
            controller_task_text("complete", true),
        )
        .unwrap();
        fs::write(
            project.path().join(".completed-sprint.fixture"),
            "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
        )
        .unwrap();
        fs::write(
            project.path().join("runtime-emitter.sh"),
            "#!/bin/sh\nset -eu\ncp .completed-task.fixture tasks/issues/009-fixture.md\ncp .completed-sprint.fixture tasks/sprint-1.md\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"local-fixture\"}' '{\"type\":\"turn.completed\",\"usage\":{}}'\n",
        )
        .unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let delivered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let channel_delivered = Arc::clone(&delivered);

        let result = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(move |_| {
                channel_delivered.fetch_add(1, Ordering::AcqRel);
                Ok(())
            }),
            Some(storage.clone()),
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |root, _, timeout, cancel, handler| {
                run_bounded_process_with_stdin_and_limit(
                    "/bin/sh",
                    &["runtime-emitter.sh".into()],
                    root,
                    timeout.min(Duration::from_secs(2)),
                    cancel,
                    None,
                    handler,
                    RUNTIME_OUTPUT_LIMIT,
                )
            },
        )
        .unwrap();

        assert_eq!(
            result.outcome,
            BoundedTaskOutcome::Verified,
            "{}",
            result.reason
        );
        assert_eq!(result.loop_state.state, GoalLoopState::GoalComplete);
        assert_eq!(delivered.load(Ordering::Acquire), 3);
        let persisted = storage.read_record().unwrap().unwrap();
        assert_eq!(persisted.current_run.event_cursor, 2);
        assert!(!persisted.current_run.nonterminal);
    }

    #[test]
    fn bounded_controller_worker_offloads_and_accepts_concurrent_cancellation() {
        let project = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(project.path()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let run_id = "abcdefabcdefabcdefabcdefabcdefab".to_string();
        let lease = registry
            .begin(&root, OperationKind::BoundedTask, Some(run_id.clone()))
            .unwrap();
        let worker_cancel = Arc::clone(&lease.cancel);
        let worker_explicit_cancel = Arc::clone(&lease.explicit_user_cancellation);
        let cancel_registry = Arc::clone(&registry);
        let (worker_entered_tx, worker_entered_rx) = mpsc::channel();
        let caller_thread = thread::current().id();

        let (worker_thread, cancellation_requested) = tauri::async_runtime::block_on(async move {
            let cancellation = tauri::async_runtime::spawn(async move {
                worker_entered_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("blocking controller worker did not start");
                cancel_registry.cancel_bounded_task(&run_id).unwrap()
            });
            let worker_thread = run_bounded_task_worker(root.clone(), move || {
                let _lease = lease;
                let worker_thread = thread::current().id();
                worker_entered_tx.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(2);
                while !worker_cancel.load(Ordering::Acquire) && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(1));
                }
                assert!(worker_cancel.load(Ordering::Acquire));
                assert!(worker_explicit_cancel.load(Ordering::Acquire));
                Ok(worker_thread)
            })
            .await
            .unwrap();
            (worker_thread, cancellation.await.unwrap())
        });

        assert_ne!(worker_thread, caller_thread);
        assert!(cancellation_requested);
        assert!(!registry
            .cancel_bounded_task("abcdefabcdefabcdefabcdefabcdefab")
            .unwrap());
    }

    #[test]
    fn bounded_controller_production_path_stops_on_refreshed_conflict_gate() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let registry = Arc::new(OperationRegistry::default());
        let effects = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let runtime_effects = Arc::clone(&effects);

        let result = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage),
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                runtime_effects.fetch_add(1, Ordering::AcqRel);
                prepare_second_controller_task(root);
                fs::write(
                    root.join("docs/conflicts.md"),
                    "Status: active\n\n## Conflicts\n\n| Conflict | Status | Owner |\n| --- | --- | --- |\n| Reconcile task authority | open | AI |\n",
                )
                .unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();

        assert_eq!(
            result.outcome,
            BoundedTaskOutcome::Stopped,
            "{}",
            result.reason
        );
        assert_eq!(result.loop_state.state, GoalLoopState::ConflictStop);
        assert!(!result.loop_state.blocking_gates.is_empty());
        assert!(result.loop_state.next_task.is_none());
        assert!(!result.loop_state.explicit_confirmation_required);
        assert_eq!(effects.load(Ordering::Acquire), 1);
    }

    #[test]
    fn bounded_controller_production_path_preserves_checkpoint_and_consumes_failed_next_confirmation(
    ) {
        for (label, termination, expected_state) in [
            (
                "nonzero",
                ProcessTermination::Completed,
                GoalLoopState::FailureStop,
            ),
            (
                "cancelled",
                ProcessTermination::Cancelled,
                GoalLoopState::CancelledStop,
            ),
        ] {
            let project = tempfile::tempdir().unwrap();
            write_controller_repository(project.path());
            let state_dir = tempfile::tempdir().unwrap();
            let storage = goal_storage(&state_dir);
            let registry = Arc::new(OperationRegistry::default());
            let effects = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            let first_preview = preview_bounded_task_with_registry(
                project.path().to_string_lossy().to_string(),
                Arc::clone(&registry),
            )
            .unwrap();
            let first_effects = Arc::clone(&effects);
            execute_bounded_task_command_with_storage(
                project.path().to_string_lossy().to_string(),
                BoundedTaskInvocation {
                    preview_token: first_preview.preview_token,
                    selected_task: first_preview.selected_task.unwrap(),
                    mode: RuntimeMode::Live,
                    confirmed: true,
                },
                Channel::new(|_| Ok(())),
                Some(storage.clone()),
                Arc::clone(&registry),
                |_, _| {
                    Ok(runtime_output(
                        true,
                        b"codex-cli 0.144.4\n",
                        b"",
                        ProcessTermination::Completed,
                        false,
                    ))
                },
                move |root, _, _, _, _| {
                    first_effects.fetch_add(1, Ordering::AcqRel);
                    prepare_second_controller_task(root);
                    Ok(runtime_output(
                        true,
                        CODEX_FIXTURE_JSONL.as_bytes(),
                        b"",
                        ProcessTermination::Completed,
                        false,
                    ))
                },
            )
            .unwrap();
            let checkpointed: PersistedGoalRecord =
                serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
            let expected_checkpoint = checkpointed.last_checkpoint.clone();
            let expected_references = checkpointed.evidence_references.clone();

            let second_preview = preview_bounded_task_with_registry(
                project.path().to_string_lossy().to_string(),
                Arc::clone(&registry),
            )
            .unwrap();
            let replay_token = second_preview.preview_token.clone();
            let replay_task = second_preview.selected_task.clone().unwrap();
            let second_effects = Arc::clone(&effects);
            let failed = execute_bounded_task_command_with_storage(
                project.path().to_string_lossy().to_string(),
                BoundedTaskInvocation {
                    preview_token: second_preview.preview_token,
                    selected_task: second_preview.selected_task.unwrap(),
                    mode: RuntimeMode::Live,
                    confirmed: true,
                },
                Channel::new(|_| Ok(())),
                Some(storage.clone()),
                Arc::clone(&registry),
                |_, _| {
                    Ok(runtime_output(
                        true,
                        b"codex-cli 0.144.4\n",
                        b"",
                        ProcessTermination::Completed,
                        false,
                    ))
                },
                move |_, _, _, _, _| {
                    second_effects.fetch_add(1, Ordering::AcqRel);
                    Ok(runtime_output(
                        false,
                        b"",
                        b"fixture failure",
                        termination,
                        false,
                    ))
                },
            )
            .unwrap();
            assert_eq!(failed.loop_state.state, expected_state, "{label}");
            let after_failure: PersistedGoalRecord =
                serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
            assert!(after_failure.revision > checkpointed.revision, "{label}");
            assert_eq!(
                after_failure.last_checkpoint, expected_checkpoint,
                "{label}"
            );
            assert_eq!(
                after_failure.evidence_references, expected_references,
                "{label}"
            );
            assert!(after_failure.current_run.nonterminal, "{label}");

            let replay_effects = Arc::clone(&effects);
            let replay = execute_bounded_task_command_with_storage(
                project.path().to_string_lossy().to_string(),
                BoundedTaskInvocation {
                    preview_token: replay_token,
                    selected_task: replay_task,
                    mode: RuntimeMode::Live,
                    confirmed: true,
                },
                Channel::new(|_| Ok(())),
                Some(storage.clone()),
                Arc::clone(&registry),
                |_, _| {
                    replay_effects.fetch_add(100, Ordering::AcqRel);
                    Ok(runtime_output(
                        true,
                        b"codex-cli 0.144.4\n",
                        b"",
                        ProcessTermination::Completed,
                        false,
                    ))
                },
                |_, _, _, _, _| unreachable!("consumed confirmation must not spawn Codex"),
            );
            assert!(matches!(
                replay,
                Err(error) if error.code == "controller_confirmation_consumed_or_stale"
            ));
            assert_eq!(effects.load(Ordering::Acquire), 2, "{label}");
        }
    }

    #[test]
    fn bounded_controller_uses_its_fixed_timeout_without_changing_generic_runtime() {
        let project = tempfile::tempdir().unwrap();
        let invocation = runtime_invocation(RuntimeMode::Live);
        let argv = bounded_task_runtime_argv(project.path(), invocation.prompt.as_deref().unwrap());
        let result = execute_bounded_task_runtime_with(
            project.path(),
            invocation,
            true,
            Some(argv),
            || {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |_, timeout| {
                assert_eq!(timeout, Duration::from_secs(30 * 60));
                Ok(runtime_output(
                    true,
                    b"",
                    b"",
                    ProcessTermination::TimedOut,
                    false,
                ))
            },
        );

        assert_eq!(RUNTIME_TIMEOUT, Duration::from_secs(120));
        assert_eq!(BOUNDED_TASK_RUNTIME_TIMEOUT, Duration::from_secs(30 * 60));
        assert_eq!(result.outcome, RuntimeOutcome::TimedOut);
        assert_eq!(
            result.failure.as_deref(),
            Some("Codex runtime exceeded the 1800 second execution limit")
        );
    }

    #[test]
    fn production_runtime_and_bounded_controller_paths_supply_distinct_timeouts() {
        let runtime_project = tempfile::tempdir().unwrap();
        init_repo(runtime_project.path());
        let generic_timeout = Arc::new(Mutex::new(None));
        let observed_generic_timeout = Arc::clone(&generic_timeout);
        let generic = execute_runtime_command_with(
            runtime_project.path().to_string_lossy().to_string(),
            runtime_invocation(RuntimeMode::Live),
            Channel::new(|_| Ok(())),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |_, _, timeout, _, _| {
                *observed_generic_timeout.lock().unwrap() = Some(timeout);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(generic.outcome, RuntimeOutcome::Completed);
        assert_eq!(
            *generic_timeout.lock().unwrap(),
            Some(Duration::from_secs(120))
        );

        let controller_project = tempfile::tempdir().unwrap();
        write_controller_repository(controller_project.path());
        let preview =
            preview_bounded_task(controller_project.path().to_string_lossy().to_string()).unwrap();
        let selected_task = preview.selected_task.unwrap();
        let controller_timeout = Arc::new(Mutex::new(None));
        let observed_controller_timeout = Arc::clone(&controller_timeout);
        let controller = execute_bounded_task_command_with(
            controller_project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                mode: RuntimeMode::Live,
                selected_task,
                preview_token: preview.preview_token,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, timeout, _, _| {
                *observed_controller_timeout.lock().unwrap() = Some(timeout);
                fs::write(
                    root.join("tasks/issues/009-fixture.md"),
                    controller_task_text("complete", true),
                )
                .unwrap();
                fs::write(
                    root.join("tasks/sprint-1.md"),
                    "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
                )
                .unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(
            controller.outcome,
            BoundedTaskOutcome::Verified,
            "{}; runtime={:?}; refresh_failures={:?}",
            controller.reason,
            controller
                .runtime
                .as_ref()
                .map(|runtime| (runtime.outcome, runtime.failure.as_deref())),
            controller.refresh_failures
        );
        assert_eq!(
            *controller_timeout.lock().unwrap(),
            Some(Duration::from_secs(30 * 60))
        );
    }

    #[test]
    fn bounded_controller_real_helper_fixture_verifies_completed_repository_evidence() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let effects = std::sync::atomic::AtomicUsize::new(0);
        let result = run_controller_fixture_trial(project.path(), |root| {
            effects.fetch_add(1, Ordering::AcqRel);
            fs::write(
                root.join("tasks/issues/009-fixture.md"),
                controller_task_text("complete", true),
            )
            .unwrap();
            fs::write(
                root.join("tasks/sprint-1.md"),
                "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
            )
            .unwrap();
        });
        assert_eq!(effects.load(Ordering::Acquire), 1);
        assert_eq!(
            result.outcome,
            BoundedTaskOutcome::Verified,
            "{}; refresh={:?}; runtime={:?}",
            result.reason,
            result.refresh_failures,
            result
                .runtime
                .as_ref()
                .map(|runtime| (runtime.outcome, runtime.failure.as_deref()))
        );
        assert!(result.repository_verified);
        assert!(result.refresh_failures.is_empty());
        let stop = result.stop_gates.unwrap().decision.unwrap();
        assert_eq!(stop.decision, "stop");
        assert_eq!(stop.confidence, "medium");
        assert_eq!(stop.warnings, ["selected task status is complete"]);
        assert_eq!(
            result.resolver.unwrap().decision.unwrap().decision,
            "no-ready-task"
        );
    }

    #[test]
    fn persisted_controller_never_promotes_no_ready_task_to_goal_completion() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let canonical = validated_repository_root(&project.path().to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            canonical.to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let result = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |root, _, _, _, _| {
                fs::write(
                    root.join("tasks/issues/009-fixture.md"),
                    controller_task_text("complete", true),
                )
                .unwrap();
                fs::write(
                    root.join("tasks/sprint-1.md"),
                    "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n| 010 | Planned task | planned | 009 | tasks/issues/010.md |\n",
                )
                .unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(
            result.outcome,
            BoundedTaskOutcome::Verified,
            "{}; refresh={:?}; runtime={:?}",
            result.reason,
            result.refresh_failures,
            result
                .runtime
                .as_ref()
                .map(|runtime| (runtime.outcome, runtime.failure.as_deref()))
        );
        assert_eq!(
            result.resolver.unwrap().decision.unwrap().decision,
            "no-ready-task"
        );
        let record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert!(record.last_checkpoint.is_some());
        assert!(!serde_json::to_string(&record)
            .unwrap()
            .contains("goalCompleted"));
        assert_ne!(
            goal_recovery(&storage, project.path()).unwrap().state,
            GoalRecoveryState::Completed
        );
    }

    #[test]
    fn persisted_controller_rejects_cross_row_and_ambiguous_checkpoint_evidence() {
        let verified_other = controller_task_text("complete", true).replace("# 009:", "# 008:");
        let cases = [
            (
                "deferred title only",
                "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 010 | tasks/issues/009-fixture.md | deferred | - | |\n",
                Vec::new(),
            ),
            (
                "deferred extra token beside verified complete row",
                "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 008 | Other | complete | - | tasks/issues/008-fixture.md |\n| 009 | Deferred | deferred | - | tasks/issues/008-fixture.md tasks/issues/009-fixture.md |\n",
                vec![("tasks/issues/008-fixture.md", verified_other.clone())],
            ),
            (
                "mismatched complete evidence",
                "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Selected | complete | - | tasks/issues/008-fixture.md |\n",
                vec![("tasks/issues/008-fixture.md", verified_other.clone())],
            ),
            (
                "multiple complete evidence paths",
                "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Selected | complete | - | tasks/issues/009-fixture.md tasks/issues/008-fixture.md |\n",
                vec![("tasks/issues/008-fixture.md", verified_other.clone())],
            ),
        ];
        for (label, tracker, additional_tasks) in cases {
            let (result, record, recovery) =
                run_persisted_controller_recovery_trial(tracker, &additional_tasks);
            assert_eq!(
                result.outcome,
                BoundedTaskOutcome::Verified,
                "{label}: {}; refresh={:?}",
                result.reason,
                result.refresh_failures
            );
            assert!(
                record
                    .evidence_references
                    .iter()
                    .all(|reference| reference.path != "tasks/sprint-1.md"),
                "{label}"
            );
            assert_eq!(recovery.state, GoalRecoveryState::Resumable, "{label}");
        }
    }

    #[test]
    fn persisted_controller_recovers_completed_only_from_terminal_repository_authority() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let canonical = validated_repository_root(&project.path().to_string_lossy()).unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            canonical.to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let result = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            registry,
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |root, _, _, _, _| {
                fs::write(
                    root.join("tasks/issues/009-fixture.md"),
                    controller_task_text("complete", true),
                )
                .unwrap();
                fs::write(
                    root.join("tasks/sprint-1.md"),
                    "# Sprint 1\n\nStatus: complete\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n",
                )
                .unwrap();
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(
            result.outcome,
            BoundedTaskOutcome::Verified,
            "{}; refresh={:?}",
            result.reason,
            result.refresh_failures
        );
        let bytes = storage.read_bytes().unwrap().unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("goalCompleted"));
        let record: PersistedGoalRecord = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(record.stop_conditions, goal_loop_stop_conditions());
        assert!(record
            .evidence_references
            .iter()
            .any(|reference| reference.path == "tasks/sprint-1.md"));
        let recovery = goal_recovery(&storage, project.path()).unwrap();
        assert_eq!(recovery.state, GoalRecoveryState::Completed);
        assert!(!recovery.explicit_confirmation_required);
        assert!(!recovery.automatic_execution_started);
    }

    #[test]
    fn bounded_controller_real_helper_fixture_reports_verification_failure() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let result = run_controller_fixture_trial(project.path(), |_| {});
        assert_eq!(result.outcome, BoundedTaskOutcome::VerificationFailed);
        assert!(!result.repository_verified);
        assert!(result.refresh_failures.is_empty());
        assert_eq!(
            result.resolver.unwrap().decision.unwrap().decision,
            "execute-task"
        );
    }

    #[test]
    fn bounded_controller_real_helper_fixture_stops_at_wait_external() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let result = run_controller_fixture_trial(project.path(), |root| {
            fs::write(
                root.join("tasks/issues/009-fixture.md"),
                controller_task_text("complete", true),
            )
            .unwrap();
            fs::write(
                root.join("tasks/issues/010-external.md"),
                controller_task_text("ready", false)
                    .replace("# 009:", "# 010:")
                    .replace("Owner: AI", "Owner: External"),
            )
            .unwrap();
            fs::write(
                root.join("tasks/sprint-1.md"),
                "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | complete | - | tasks/issues/009-fixture.md |\n| 010 | External review | ready | 009 | tasks/issues/010-external.md |\n",
            )
            .unwrap();
        });
        assert_eq!(result.outcome, BoundedTaskOutcome::WaitExternal);
        assert!(result.refresh_failures.is_empty());
        assert_eq!(
            result.resolver.unwrap().decision.unwrap().decision,
            "wait-external"
        );
    }

    #[test]
    fn bounded_controller_real_helper_preview_returns_typed_non_executable_stop() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        fs::write(
            project.path().join("tasks/issues/010-external.md"),
            controller_task_text("ready", false)
                .replace("# 009:", "# 010:")
                .replace("Owner: AI", "Owner: External"),
        )
        .unwrap();
        fs::write(
            project.path().join("tasks/sprint-1.md"),
            "# Sprint 1\n\nStatus: active\n\n## Tasks\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 009 | Fixture task | ready | - | tasks/issues/009-fixture.md |\n| 010 | External review | ready | - | tasks/issues/010-external.md |\n",
        )
        .unwrap();
        let preview = build_bounded_task_preview(project.path(), &AtomicBool::new(false)).unwrap();
        assert_eq!(preview.decision, "wait-external");
        assert_eq!(preview.confidence, "medium");
        assert!(!preview.executable);
        assert!(preview.selected_task.is_none());
        assert!(!preview.blocking_gates.is_empty());
        assert!(preview.prompt.is_empty());
    }

    #[test]
    fn bounded_controller_classifies_repository_evidence_not_provider_claims() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let resolver = controller_helper_result(project.path(), "no-ready-task", Vec::new());
        let stop_gates = controller_helper_result(
            project.path(),
            "stop",
            vec!["selected task status is complete".into()],
        );
        let mut runtime = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        runtime.success = true;
        runtime.outcome = RuntimeOutcome::Completed;
        let incomplete = "Status: active\n\n## Acceptance Criteria\n\n- [ ] proof\n\n## Evidence Log\n\n| result | pass |\n\n## Verification Summary\n\nNot run yet.";
        let classified = classify_bounded_task(&runtime, incomplete, &resolver, &stop_gates, &[]);
        assert_eq!(classified.0, BoundedTaskOutcome::VerificationFailed);
        assert!(!classified.1);

        let complete = "Status: complete\n\n## Acceptance Criteria\n\n- [x] proof\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nFocused checks passed.";
        let classified = classify_bounded_task(&runtime, complete, &resolver, &stop_gates, &[]);
        assert_eq!(classified.0, BoundedTaskOutcome::Verified);
        assert!(classified.1);

        let unexpected_stop = controller_helper_result(
            project.path(),
            "stop",
            vec![
                "selected task status is complete".into(),
                "other gate".into(),
            ],
        );
        assert_eq!(
            classify_bounded_task(&runtime, complete, &resolver, &unexpected_stop, &[]).0,
            BoundedTaskOutcome::Stopped
        );
    }

    #[test]
    fn bounded_controller_helper_failures_are_stopped_not_verification_failed() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let mut runtime = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        runtime.success = true;
        runtime.outcome = RuntimeOutcome::Completed;
        let task = "Status: complete\n\n## Acceptance Criteria\n\n- [x] proof\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nPassed.";
        let resolver = controller_helper_result(project.path(), "no-ready-task", Vec::new());
        for outcome in [HelperOutcome::TimedOut, HelperOutcome::CleanupFailed] {
            let mut stop_gates = controller_helper_result(
                project.path(),
                "stop",
                vec!["selected task status is complete".into()],
            );
            stop_gates.success = false;
            stop_gates.outcome = outcome;
            stop_gates.failure = Some(format!("{outcome:?}"));
            assert_eq!(
                classify_bounded_task(&runtime, task, &resolver, &stop_gates, &[]).0,
                BoundedTaskOutcome::Stopped
            );
            assert!(matches!(
                helper_outcome_code(outcome),
                "timedOut" | "cleanupFailed"
            ));
        }
    }

    #[test]
    fn bounded_controller_stop_gate_cancellation_is_stopped() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let mut runtime = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        runtime.success = true;
        runtime.outcome = RuntimeOutcome::Completed;
        let task = "Status: complete\n\n## Acceptance Criteria\n\n- [x] proof\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nPassed.";
        let resolver = controller_helper_result(project.path(), "no-ready-task", Vec::new());
        let mut stop_gates = controller_helper_result(project.path(), "stop", Vec::new());
        stop_gates.success = false;
        stop_gates.outcome = HelperOutcome::Cancelled;
        stop_gates.failure = Some("cancelled".into());
        assert_eq!(
            classify_bounded_task(&runtime, task, &resolver, &stop_gates, &[]).0,
            BoundedTaskOutcome::Stopped
        );
    }

    #[test]
    fn bounded_controller_git_or_snapshot_refresh_failure_cannot_verify() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let mut runtime = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        runtime.success = true;
        runtime.outcome = RuntimeOutcome::Completed;
        let task = "Status: complete\n\n## Acceptance Criteria\n\n- [x] proof\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nPassed.";
        let resolver = controller_helper_result(project.path(), "no-ready-task", Vec::new());
        let stop_gates = controller_helper_result(
            project.path(),
            "stop",
            vec!["selected task status is complete".into()],
        );
        for surface in ["git", "snapshot"] {
            let refresh_failures = vec![BoundedTaskRefreshFailure {
                surface: surface.into(),
                code: format!("{surface}_inspection_failed"),
                message: format!("{surface} unavailable"),
            }];
            let classified =
                classify_bounded_task(&runtime, task, &resolver, &stop_gates, &refresh_failures);
            assert_eq!(classified.0, BoundedTaskOutcome::Stopped);
            assert!(!classified.1);
            assert!(classified.2.contains(surface));
        }
    }

    #[test]
    fn bounded_controller_stops_for_runtime_failure_and_wait_external() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let stop_gates = controller_helper_result(
            project.path(),
            "stop",
            vec!["selected task status is complete".into()],
        );
        let mut runtime = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        runtime.outcome = RuntimeOutcome::Cancelled;
        let task = "Status: complete\n\n## Acceptance Criteria\n\n- [x] proof\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nPassed.";
        let resolver = controller_helper_result(project.path(), "no-ready-task", Vec::new());
        assert_eq!(
            classify_bounded_task(&runtime, task, &resolver, &stop_gates, &[]).0,
            BoundedTaskOutcome::Stopped
        );
        runtime.success = true;
        runtime.outcome = RuntimeOutcome::Completed;
        let waiting = controller_helper_result(project.path(), "wait-external", Vec::new());
        assert_eq!(
            classify_bounded_task(&runtime, task, &waiting, &stop_gates, &[]).0,
            BoundedTaskOutcome::WaitExternal
        );
    }

    #[test]
    fn bounded_controller_serializes_and_scopes_cancellation_to_its_native_run() {
        let project = tempfile::tempdir().unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let run_id = "0123456789abcdef0123456789abcdef";
        let lease = registry
            .begin(
                project.path(),
                OperationKind::BoundedTask,
                Some(run_id.into()),
            )
            .unwrap();
        let competing = registry.begin(project.path(), OperationKind::Helper, None);
        assert!(matches!(competing, Err(error) if error.code == "operation_in_progress"));
        assert!(registry.cancel_bounded_task(run_id).unwrap());
        assert!(lease.cancel.load(Ordering::Acquire));
        assert!(lease.explicit_user_cancellation.load(Ordering::Acquire));
        assert_eq!(
            registry.cancel_run(run_id).unwrap_err().code,
            "runtime_run_mismatch"
        );
        drop(lease);
        assert!(registry
            .begin(project.path(), OperationKind::Helper, None)
            .is_ok());
    }

    #[test]
    fn bounded_controller_internal_cleanup_stop_remains_a_failure() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let mut runtime = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        runtime.outcome = RuntimeOutcome::ChannelFailed;
        runtime.failure = Some("fixture channel closed".into());
        let process_stop = AtomicBool::new(true);
        let explicit_user_cancellation = AtomicBool::new(false);
        let result = finish_bounded_task_with(
            project.path(),
            Some("tasks/issues/009-fixture.md".into()),
            Some(runtime),
            None,
            None,
            &process_stop,
            &explicit_user_cancellation,
            &mut |_, _, _| unreachable!("internal cleanup stop must skip refresh helpers"),
        );

        assert_eq!(result.loop_state.state, GoalLoopState::FailureStop);
        assert!(result
            .runtime
            .as_ref()
            .is_some_and(|runtime| { runtime.outcome == RuntimeOutcome::ChannelFailed }));
    }

    #[test]
    fn bounded_controller_pre_run_cancellation_still_collects_partial_repository_terminal() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let cancelled = AtomicBool::new(true);
        assert!(build_bounded_task_preview(project.path(), &cancelled).is_err());
        let result = finish_bounded_task(
            project.path(),
            Some("tasks/issues/009-fixture.md".into()),
            None,
            Some("cancelled before provider spawn".into()),
            &cancelled,
        );
        assert_eq!(result.outcome, BoundedTaskOutcome::Stopped);
        assert!(result.runtime.is_none());
        assert!(result.task_evidence.is_some());
        assert!(result.resolver.is_none());
        assert!(result.stop_gates.is_none());
        assert_eq!(result.refresh_failures.len(), 2);
        assert!(result
            .refresh_failures
            .iter()
            .all(|failure| failure.code == "refresh_cancelled"));
        assert!(result.reason.contains("cancelled before provider spawn"));
    }

    #[test]
    fn bounded_controller_production_command_projects_pre_run_helper_cancellation() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let run_id = Arc::new(Mutex::new(None::<String>));
        let channel_run_id = Arc::clone(&run_id);
        let helper_run_id = Arc::clone(&run_id);
        let helper_registry = Arc::clone(&registry);
        let helper_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_helper_invocations = Arc::clone(&helper_invocations);
        let version_probes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_version_probes = Arc::clone(&version_probes);
        let provider_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_provider_invocations = Arc::clone(&provider_invocations);

        let result = execute_bounded_task_command_with_storage_and_helper(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(move |message| {
                let message: serde_json::Value = message.deserialize().unwrap();
                if message.get("type").and_then(serde_json::Value::as_str) == Some("started") {
                    *channel_run_id.lock().unwrap() = Some(
                        message
                            .get("handle")
                            .and_then(|handle| handle.get("runId"))
                            .and_then(serde_json::Value::as_str)
                            .expect("Started message must carry a run ID")
                            .into(),
                    );
                }
                Ok(())
            }),
            None,
            Arc::clone(&registry),
            move |_, _| {
                observed_version_probes.fetch_add(1, Ordering::AcqRel);
                unreachable!("pre-run cancellation must not probe the provider")
            },
            move |_, _, _, _, _| {
                observed_provider_invocations.fetch_add(1, Ordering::AcqRel);
                unreachable!("pre-run cancellation must not invoke the provider")
            },
            move |root, invocation, cancel| {
                observed_helper_invocations.fetch_add(1, Ordering::AcqRel);
                let run_id = helper_run_id
                    .lock()
                    .unwrap()
                    .clone()
                    .expect("Started must deliver the native run ID before revalidation");
                assert!(helper_registry.cancel_bounded_task(&run_id)?);
                controller_helper(root, invocation, cancel)
            },
        )
        .unwrap();

        assert_eq!(helper_invocations.load(Ordering::Acquire), 1);
        assert_eq!(version_probes.load(Ordering::Acquire), 0);
        assert_eq!(provider_invocations.load(Ordering::Acquire), 0);
        assert!(result.runtime.is_none());
        assert_eq!(result.outcome, BoundedTaskOutcome::Stopped);
        assert_eq!(result.loop_state.state, GoalLoopState::CancelledStop);
        assert!(result
            .loop_state
            .reason
            .contains("pre-run helper revalidation before provider spawn"));
        assert!(result.loop_state.next_task.is_none());
        assert!(!result.loop_state.explicit_confirmation_required);
        assert!(result
            .refresh_failures
            .iter()
            .any(|failure| failure.surface == "resolver" && failure.code == "refresh_cancelled"));
        assert!(result
            .refresh_failures
            .iter()
            .any(|failure| failure.surface == "stopGates" && failure.code == "refresh_cancelled"));
        let cleanup = registry
            .begin(project.path(), OperationKind::Helper, None)
            .expect("controller lease must be cleaned up after cancellation");
        drop(cleanup);
    }

    #[test]
    fn bounded_controller_production_command_projects_post_exit_helper_cancellation() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let registry = Arc::new(OperationRegistry::default());
        let preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let run_id = Arc::new(Mutex::new(None::<String>));
        let channel_run_id = Arc::clone(&run_id);
        let helper_run_id = Arc::clone(&run_id);
        let helper_registry = Arc::clone(&registry);
        let helper_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_helper_invocations = Arc::clone(&helper_invocations);
        let version_probes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_version_probes = Arc::clone(&version_probes);
        let provider_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let provider_for_helper = Arc::clone(&provider_invocations);
        let observed_provider_invocations = Arc::clone(&provider_invocations);

        let result = execute_bounded_task_command_with_storage_and_helper(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(move |message| {
                let message: serde_json::Value = message.deserialize().unwrap();
                if message.get("type").and_then(serde_json::Value::as_str) == Some("started") {
                    *channel_run_id.lock().unwrap() = Some(
                        message
                            .get("handle")
                            .and_then(|handle| handle.get("runId"))
                            .and_then(serde_json::Value::as_str)
                            .expect("Started message must carry a run ID")
                            .into(),
                    );
                }
                Ok(())
            }),
            Some(storage.clone()),
            Arc::clone(&registry),
            move |_, _| {
                observed_version_probes.fetch_add(1, Ordering::AcqRel);
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                observed_provider_invocations.fetch_add(1, Ordering::AcqRel);
                prepare_second_controller_task(root);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, invocation, cancel| {
                let invocation_index =
                    observed_helper_invocations.fetch_add(1, Ordering::AcqRel) + 1;
                if invocation_index == 3 {
                    assert_eq!(provider_for_helper.load(Ordering::Acquire), 1);
                    let run_id = helper_run_id
                        .lock()
                        .unwrap()
                        .clone()
                        .expect("Started must deliver the native run ID before refresh");
                    assert!(helper_registry.cancel_bounded_task(&run_id)?);
                }
                controller_helper(root, invocation, cancel)
            },
        )
        .unwrap();

        assert_eq!(helper_invocations.load(Ordering::Acquire), 3);
        assert_eq!(version_probes.load(Ordering::Acquire), 1);
        assert_eq!(provider_invocations.load(Ordering::Acquire), 1);
        assert_eq!(
            result.runtime.as_ref().map(|runtime| runtime.outcome),
            Some(RuntimeOutcome::Completed)
        );
        assert_eq!(result.outcome, BoundedTaskOutcome::Stopped);
        assert!(!result.repository_verified);
        assert_eq!(result.loop_state.state, GoalLoopState::CancelledStop);
        assert!(result
            .loop_state
            .reason
            .contains("post-exit repository refresh after the provider completed"));
        assert!(result.loop_state.next_task.is_none());
        assert!(!result.loop_state.explicit_confirmation_required);
        assert!(!result.loop_state.automatic_execution_started);
        assert!(result.task_evidence.as_ref().is_some_and(|task| {
            task.path == "tasks/issues/009-fixture.md" && task.content.contains("Status: complete")
        }));
        assert!(result.resolver.as_ref().is_some_and(|resolver| {
            resolver.outcome == HelperOutcome::Cancelled && !resolver.success
        }));
        assert!(result.stop_gates.is_none());
        assert!(result
            .refresh_failures
            .iter()
            .any(|failure| failure.surface == "resolver" && failure.code == "cancelled"));
        assert!(result
            .refresh_failures
            .iter()
            .any(|failure| failure.surface == "stopGates" && failure.code == "refresh_cancelled"));
        assert!(project.path().join("tasks/issues/010-fixture.md").is_file());
        let record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert!(record.current_run.nonterminal);
        assert!(record.last_checkpoint.is_none());
        let cleanup = registry
            .begin(project.path(), OperationKind::Helper, None)
            .expect("controller lease must be cleaned up after cancellation");
        drop(cleanup);
    }

    #[test]
    fn bounded_controller_cancel_after_successful_final_refresh_preserves_prior_checkpoint() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let storage = goal_storage(&state_dir);
        let registry = Arc::new(OperationRegistry::default());
        let provider_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let first_preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let first_provider_invocations = Arc::clone(&provider_invocations);
        let first = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: first_preview.preview_token,
                selected_task: first_preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(|_| Ok(())),
            Some(storage.clone()),
            Arc::clone(&registry),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                first_provider_invocations.fetch_add(1, Ordering::AcqRel);
                prepare_second_controller_task(root);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(first.loop_state.state, GoalLoopState::ContinueAvailable);
        let first_record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        let prior_checkpoint = first_record.last_checkpoint.clone();
        let prior_evidence = first_record.evidence_references.clone();

        let second_preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();
        let run_id = Arc::new(Mutex::new(None::<String>));
        let channel_run_id = Arc::clone(&run_id);
        let helper_run_id = Arc::clone(&run_id);
        let helper_registry = Arc::clone(&registry);
        let helper_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_helper_invocations = Arc::clone(&helper_invocations);
        let cancellation_response = Arc::new(Mutex::new(None::<bool>));
        let observed_cancellation_response = Arc::clone(&cancellation_response);
        let second_provider_invocations = Arc::clone(&provider_invocations);
        let provider_for_helper = Arc::clone(&provider_invocations);

        let result = execute_bounded_task_command_with_storage_and_helper(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: second_preview.preview_token,
                selected_task: second_preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(move |message| {
                let message: serde_json::Value = message.deserialize().unwrap();
                if message.get("type").and_then(serde_json::Value::as_str) == Some("started") {
                    *channel_run_id.lock().unwrap() = Some(
                        message
                            .get("handle")
                            .and_then(|handle| handle.get("runId"))
                            .and_then(serde_json::Value::as_str)
                            .expect("Started message must carry a run ID")
                            .into(),
                    );
                }
                Ok(())
            }),
            Some(storage.clone()),
            Arc::clone(&registry),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                second_provider_invocations.fetch_add(1, Ordering::AcqRel);
                complete_second_controller_task(root);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, invocation, cancel| {
                let result = controller_helper(root, invocation, cancel)?;
                let invocation_index =
                    observed_helper_invocations.fetch_add(1, Ordering::AcqRel) + 1;
                if invocation_index == 4 {
                    assert_eq!(provider_for_helper.load(Ordering::Acquire), 2);
                    let run_id = helper_run_id
                        .lock()
                        .unwrap()
                        .clone()
                        .expect("Started must deliver the native run ID before final refresh");
                    *observed_cancellation_response.lock().unwrap() =
                        Some(helper_registry.cancel_bounded_task(&run_id)?);
                }
                Ok(result)
            },
        )
        .unwrap();

        assert_eq!(helper_invocations.load(Ordering::Acquire), 4);
        assert_eq!(provider_invocations.load(Ordering::Acquire), 2);
        assert_eq!(*cancellation_response.lock().unwrap(), Some(true));
        assert_eq!(result.outcome, BoundedTaskOutcome::Stopped);
        assert!(!result.repository_verified);
        assert_eq!(result.loop_state.state, GoalLoopState::CancelledStop);
        assert!(result.loop_state.next_task.is_none());
        assert!(!result.loop_state.explicit_confirmation_required);
        let after_cancel: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert!(after_cancel.revision > first_record.revision);
        assert!(after_cancel.current_run.nonterminal);
        assert_eq!(after_cancel.last_checkpoint, prior_checkpoint);
        assert_eq!(after_cancel.evidence_references, prior_evidence);
        let cleanup = registry
            .begin(project.path(), OperationKind::Helper, None)
            .expect("controller lease must be cleaned up after cancellation");
        drop(cleanup);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_controller_checkpoint_commit_rejects_later_cancellation() {
        let project = tempfile::tempdir().unwrap();
        write_controller_repository(project.path());
        let state_dir = tempfile::tempdir().unwrap();
        let registry = Arc::new(OperationRegistry::default());
        let run_id = Arc::new(Mutex::new(None::<String>));
        let channel_run_id = Arc::clone(&run_id);
        let hook_run_id = Arc::clone(&run_id);
        let hook_registry = Arc::clone(&registry);
        let cancellation_response = Arc::new(Mutex::new(None::<bool>));
        let observed_cancellation_response = Arc::clone(&cancellation_response);
        let temporary_syncs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_temporary_syncs = Arc::clone(&temporary_syncs);
        let storage = goal_storage(&state_dir).with_test_hook(move |phase| {
            if phase == GoalStorageTestPhase::TemporarySynced
                && observed_temporary_syncs.fetch_add(1, Ordering::AcqRel) + 1 == 2
            {
                let run_id = hook_run_id
                    .lock()
                    .unwrap()
                    .clone()
                    .expect("Started must deliver the native run ID before checkpoint commit");
                *observed_cancellation_response.lock().unwrap() =
                    Some(hook_registry.cancel_bounded_task(&run_id).unwrap());
            }
        });
        let provider_invocations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_provider_invocations = Arc::clone(&provider_invocations);
        let preview = preview_bounded_task_with_registry(
            project.path().to_string_lossy().to_string(),
            Arc::clone(&registry),
        )
        .unwrap();

        let result = execute_bounded_task_command_with_storage(
            project.path().to_string_lossy().to_string(),
            BoundedTaskInvocation {
                preview_token: preview.preview_token,
                selected_task: preview.selected_task.unwrap(),
                mode: RuntimeMode::Live,
                confirmed: true,
            },
            Channel::new(move |message| {
                let message: serde_json::Value = message.deserialize().unwrap();
                if message.get("type").and_then(serde_json::Value::as_str) == Some("started") {
                    *channel_run_id.lock().unwrap() = Some(
                        message
                            .get("handle")
                            .and_then(|handle| handle.get("runId"))
                            .and_then(serde_json::Value::as_str)
                            .expect("Started message must carry a run ID")
                            .into(),
                    );
                }
                Ok(())
            }),
            Some(storage.clone()),
            Arc::clone(&registry),
            |_, _| {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            move |root, _, _, _, _| {
                observed_provider_invocations.fetch_add(1, Ordering::AcqRel);
                prepare_second_controller_task(root);
                Ok(runtime_output(
                    true,
                    CODEX_FIXTURE_JSONL.as_bytes(),
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();

        assert_eq!(provider_invocations.load(Ordering::Acquire), 1);
        assert_eq!(temporary_syncs.load(Ordering::Acquire), 2);
        assert_eq!(*cancellation_response.lock().unwrap(), Some(false));
        assert_eq!(result.outcome, BoundedTaskOutcome::Verified);
        assert!(result.repository_verified);
        assert_eq!(result.loop_state.state, GoalLoopState::ContinueAvailable);
        assert_eq!(
            result.loop_state.next_task.as_deref(),
            Some("tasks/issues/010-fixture.md")
        );
        let record: PersistedGoalRecord =
            serde_json::from_slice(&storage.read_bytes().unwrap().unwrap()).unwrap();
        assert_eq!(record.revision, 2);
        assert!(!record.current_run.nonterminal);
        assert_eq!(
            record
                .last_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.task_path.as_str()),
            Some("tasks/issues/009-fixture.md")
        );
        let run_id = run_id.lock().unwrap().clone().unwrap();
        assert!(!registry.cancel_bounded_task(&run_id).unwrap());
        let cleanup = registry
            .begin(project.path(), OperationKind::Helper, None)
            .expect("controller lease must be cleaned up after finalization");
        drop(cleanup);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_controller_in_flight_post_helper_cancellation_reaps_descendants() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().to_path_buf();
        let pid_path = root.join("post-helper-descendant.pid");
        let process_pid_path = pid_path.clone();
        let lease_cancel = Arc::new(AtomicBool::new(false));
        let helper_cancel = Arc::clone(&lease_cancel);
        let helper = thread::spawn(move || {
            run_bounded_process(
                "/bin/sh",
                &descendant_fixture_args(&process_pid_path),
                &root,
                Duration::from_secs(5),
                &helper_cancel,
            )
            .unwrap()
        });
        let descendant = wait_for_descendant_pid(&pid_path);
        lease_cancel.store(true, Ordering::Release);
        let output = helper.join().unwrap();
        assert_eq!(output.termination, ProcessTermination::Cancelled);
        assert_process_gone(descendant);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_controller_rejects_symlinked_task_evidence() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        fs::write(outside.path().join("009.md"), "Status: complete").unwrap();
        symlink(
            outside.path().join("009.md"),
            project.path().join("tasks/issues/009.md"),
        )
        .unwrap();
        let result = read_controller_task(project.path(), "tasks/issues/009.md");
        assert!(matches!(result, Err(error) if error.code == "controller_task_untrusted"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_controller_rejects_symlinked_task_ancestor() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(project.path().join("tasks")).unwrap();
        fs::create_dir(outside.path().join("issues")).unwrap();
        fs::write(outside.path().join("issues/009.md"), "Status: complete").unwrap();
        symlink(
            outside.path().join("issues"),
            project.path().join("tasks/issues"),
        )
        .unwrap();
        let result = read_controller_task(project.path(), "tasks/issues/009.md");
        assert!(matches!(result, Err(error) if error.code == "controller_task_untrusted"));
        assert!(execution_task_inventory(project.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn bounded_controller_open_task_descriptor_survives_ancestor_swap() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        fs::create_dir(outside.path().join("issues")).unwrap();
        fs::write(project.path().join("tasks/issues/009.md"), "safe").unwrap();
        fs::write(outside.path().join("issues/009.md"), "outside").unwrap();
        let mut opened =
            open_relative_regular_without_symlinks(project.path(), "tasks/issues/009.md").unwrap();
        fs::rename(
            project.path().join("tasks/issues"),
            project.path().join("tasks/issues-safe"),
        )
        .unwrap();
        symlink(
            outside.path().join("issues"),
            project.path().join("tasks/issues"),
        )
        .unwrap();
        let bytes = read_file_bytes(&mut opened).unwrap();
        assert_eq!(bytes, b"safe");
        assert!(read_controller_task(project.path(), "tasks/issues/009.md").is_err());
    }

    #[test]
    fn runtime_fixture_is_spawn_free_typed_and_non_authoritative() {
        let project = tempfile::tempdir().unwrap();
        let result = execute_runtime_with(
            project.path(),
            runtime_invocation(RuntimeMode::Fixture),
            true,
            || panic!("fixture must not probe a provider runtime"),
            |_| panic!("fixture must not spawn a provider runtime"),
        );
        assert_eq!(result.outcome, RuntimeOutcome::Completed);
        assert!(!result.executed);
        assert!(result.success);
        assert!(result.provenance.simulated);
        assert!(result.provenance.argv.is_empty());
        assert!(!result.repository_authority_advanced);
        assert!(!result.capabilities.repository_authority);
        assert!(result
            .events
            .iter()
            .any(|event| event.kind == RuntimeEventKind::Unknown));
        assert!(result
            .events
            .iter()
            .all(|event| event.provenance == "fixture"));
    }

    #[test]
    fn runtime_live_argv_is_closed_and_prompt_injection_is_rejected_before_spawn() {
        let project = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(project.path()).unwrap();
        let valid = execute_runtime_with(
            &root,
            runtime_invocation(RuntimeMode::Live),
            true,
            || {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |argv| {
                assert_eq!(argv, runtime_argv(&root, "Inspect only; report evidence."));
                assert_eq!(argv[argv.len() - 1], "Inspect only; report evidence.");
                Ok(runtime_output(
                    true,
                    b"{\"type\":\"turn.completed\",\"usage\":{}}\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        );
        assert_eq!(valid.outcome, RuntimeOutcome::Completed);
        assert_eq!(valid.provenance.executable, CODEX_EXECUTABLE);
        assert_eq!(
            valid.provenance.runtime_version.as_deref(),
            Some("codex-cli 0.144.4")
        );
        assert_eq!(
            valid.provenance.argv[0..11],
            [
                "exec",
                "--json",
                "--ephemeral",
                "--ignore-user-config",
                "--sandbox",
                "read-only",
                "--color",
                "never",
                "-C",
                root.to_str().unwrap(),
                "--"
            ]
        );

        let reserved = execute_runtime_with(
            &root,
            RuntimeInvocation {
                mode: RuntimeMode::Live,
                prompt: Some("resume".into()),
                confirmed: true,
            },
            true,
            || {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |argv| {
                assert_eq!(&argv[argv.len() - 2..], ["--", "resume"]);
                Ok(runtime_output(
                    true,
                    b"{\"type\":\"turn.completed\",\"usage\":{}}\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        );
        assert_eq!(reserved.outcome, RuntimeOutcome::Completed);

        for prompt in ["--dangerously-bypass-approvals-and-sandbox", "", "\0bad"] {
            let result = execute_runtime_with(
                &root,
                RuntimeInvocation {
                    mode: RuntimeMode::Live,
                    prompt: Some(prompt.into()),
                    confirmed: true,
                },
                true,
                || panic!("invalid prompt must not probe runtime"),
                |_| panic!("invalid prompt must not spawn runtime"),
            );
            assert_eq!(result.outcome, RuntimeOutcome::InvalidPrompt);
            assert!(!result.executed);
        }
        let oversized = "x".repeat(RUNTIME_PROMPT_LIMIT + 1);
        assert!(validated_runtime_prompt(Some(&oversized)).is_err());
    }

    #[test]
    fn runtime_jsonl_normalizes_known_unknown_malformed_and_stderr_with_raw_payload() {
        let raw = concat!(
            "{\"type\":\"thread.started\",\"thread_id\":\"abc\"}\n",
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done\"}}\n",
            "{\"type\":\"future.event\",\"secret\":7}\n",
            "not-json\n",
        );
        let parsed = parse_runtime_jsonl(raw.as_bytes(), "provider");
        let malformed = parsed.malformed;
        let mut events = parsed.events;
        add_runtime_stderr_event(&mut events, b"warning", "provider");
        assert!(malformed);
        assert_eq!(
            events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            vec![
                RuntimeEventKind::Session,
                RuntimeEventKind::Message,
                RuntimeEventKind::Unknown,
                RuntimeEventKind::Malformed,
                RuntimeEventKind::Stderr,
            ]
        );
        assert_eq!(events[1].summary, "done");
        assert_eq!(events[2].raw_payload.encoding, PayloadEncoding::Utf8);
        assert_eq!(
            events[2].raw_payload.data,
            "{\"type\":\"future.event\",\"secret\":7}"
        );
        assert_eq!(events[4].raw_payload.data, "warning");

        let invalid =
            parse_runtime_jsonl(b"{\"type\":\"item.completed\",\"bad\":\xff}\n", "provider");
        assert!(invalid.malformed);
        assert_eq!(invalid.events[0].kind, RuntimeEventKind::Malformed);
        assert_eq!(invalid.events[0].raw_payload.encoding, PayloadEncoding::Hex);
        assert_eq!(
            invalid.events[0].raw_payload.data,
            "7b2274797065223a226974656d2e636f6d706c65746564222c22626164223aff7d"
        );
    }

    #[test]
    fn runtime_stream_message_serializes_the_run_identity_in_camel_case() {
        let event = parse_runtime_jsonl(b"{\"type\":\"turn.started\"}\n", "fixture")
            .events
            .into_iter()
            .next()
            .unwrap();
        let value = serde_json::to_value(RuntimeStreamMessage::Event {
            run_id: "0123456789abcdef0123456789abcdef".into(),
            event,
        })
        .unwrap();
        assert_eq!(value["type"], "event");
        assert_eq!(value["runId"], "0123456789abcdef0123456789abcdef");
        assert!(value.get("run_id").is_none());
    }

    #[test]
    fn runtime_terminal_results_cover_failures_bounds_timeout_cancel_and_capability() {
        let project = tempfile::tempdir().unwrap();
        let version = || {
            Ok(runtime_output(
                true,
                b"codex-cli 0.144.4\n",
                b"",
                ProcessTermination::Completed,
                false,
            ))
        };
        let cases = [
            (
                runtime_output(
                    true,
                    b"not-json\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ),
                RuntimeOutcome::MalformedOutput,
            ),
            (
                runtime_output(
                    false,
                    b"{\"type\":\"error\"}\n",
                    b"failure",
                    ProcessTermination::Completed,
                    false,
                ),
                RuntimeOutcome::NonzeroExit,
            ),
            (
                runtime_output(
                    true,
                    b"{\"type\":\"turn.completed\"}\n",
                    b"",
                    ProcessTermination::Completed,
                    true,
                ),
                RuntimeOutcome::OutputOverflow,
            ),
            (
                runtime_output(true, b"", b"", ProcessTermination::TimedOut, false),
                RuntimeOutcome::TimedOut,
            ),
            (
                runtime_output(true, b"", b"", ProcessTermination::Cancelled, false),
                RuntimeOutcome::Cancelled,
            ),
        ];
        for (output, outcome) in cases {
            let result = execute_runtime_with(
                project.path(),
                runtime_invocation(RuntimeMode::Live),
                true,
                version,
                |_| Ok(output),
            );
            assert_eq!(result.outcome, outcome);
            assert!(!result.success);
            assert!(!result.repository_authority_advanced);
        }

        let missing = execute_runtime_with(
            project.path(),
            runtime_invocation(RuntimeMode::Live),
            true,
            || {
                Err(ProcessRunFailure::new(
                    ProcessRunFailureKind::MissingExecutable,
                    "missing",
                ))
            },
            |_| panic!("missing runtime cannot execute"),
        );
        assert_eq!(missing.outcome, RuntimeOutcome::MissingRuntime);

        let capability = execute_runtime_with(
            project.path(),
            runtime_invocation(RuntimeMode::Live),
            true,
            || {
                Ok(runtime_output(
                    true,
                    b"not-codex\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |_| panic!("unknown runtime identity cannot execute"),
        );
        assert_eq!(capability.outcome, RuntimeOutcome::CapabilityUnavailable);

        let unsupported = execute_runtime_with(
            project.path(),
            runtime_invocation(RuntimeMode::Live),
            false,
            || panic!("unsupported platform cannot probe runtime"),
            |_| panic!("unsupported platform cannot execute"),
        );
        assert_eq!(unsupported.outcome, RuntimeOutcome::UnsupportedPlatform);
        assert!(!unsupported.capabilities.live);
    }

    #[test]
    fn runtime_zero_exit_requires_success_terminal_and_rejects_provider_failure_events() {
        let project = tempfile::tempdir().unwrap();
        for provider_type in ["error", "turn.failed"] {
            let stdout = format!("{{\"type\":\"{provider_type}\",\"message\":\"failed\"}}\n{{\"type\":\"turn.completed\"}}\n");
            let result = execute_runtime_with(
                project.path(),
                runtime_invocation(RuntimeMode::Live),
                true,
                || {
                    Ok(runtime_output(
                        true,
                        b"codex-cli 0.144.4\n",
                        b"",
                        ProcessTermination::Completed,
                        false,
                    ))
                },
                |_| {
                    Ok(runtime_output(
                        true,
                        stdout.as_bytes(),
                        b"",
                        ProcessTermination::Completed,
                        false,
                    ))
                },
            );
            assert_eq!(result.outcome, RuntimeOutcome::ProviderError);
            assert!(!result.success);
        }

        let missing_terminal = execute_runtime_with(
            project.path(),
            runtime_invocation(RuntimeMode::Live),
            true,
            || {
                Ok(runtime_output(
                    true,
                    b"codex-cli 0.144.4\n",
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
            |_| {
                Ok(runtime_output(true, b"{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"claimed success\"}}\n", b"", ProcessTermination::Completed, false))
            },
        );
        assert_eq!(missing_terminal.outcome, RuntimeOutcome::MalformedOutput);
        assert!(!missing_terminal.success);
    }

    #[cfg(unix)]
    #[test]
    fn runtime_process_streams_complete_bounded_lines_before_terminal_completion() {
        let project = tempfile::tempdir().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let process_cancel = Arc::clone(&cancel);
        let (line_sender, line_receiver) = mpsc::channel::<Vec<u8>>();
        let finished = Arc::new(AtomicBool::new(false));
        let process_finished = Arc::clone(&finished);
        let root = project.path().to_path_buf();
        let process = thread::spawn(move || {
            let handler = Arc::new(move |line: &[u8]| {
                let _ = line_sender.send(line.to_vec());
            });
            let output = run_bounded_process_with_stdin(
                "/bin/sh",
                &[
                    "-c".into(),
                    "printf '%s\\n' '{\"type\":\"turn.started\"}'; sleep 0.3; printf '%s\\n' '{\"type\":\"turn.completed\"}'".into(),
                ],
                &root,
                Duration::from_secs(2),
                &process_cancel,
                None,
                Some(handler),
            ).unwrap();
            process_finished.store(true, Ordering::Release);
            output
        });

        assert_eq!(
            line_receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            b"{\"type\":\"turn.started\"}"
        );
        assert!(!finished.load(Ordering::Acquire));
        assert_eq!(
            line_receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            b"{\"type\":\"turn.completed\"}"
        );
        let output = process.join().unwrap();
        assert_eq!(output.termination, ProcessTermination::Completed);
        assert!(output.status.success());
    }

    #[cfg(unix)]
    #[test]
    fn runtime_post_start_channel_failure_cancels_reaps_and_is_typed() {
        let project = tempfile::tempdir().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let process_cancel = Arc::clone(&cancel);
        let failure = Arc::new(Mutex::new(None));
        let process_failure = Arc::clone(&failure);
        let handler_cancel = Arc::clone(&cancel);
        let handler = Arc::new(move |_line: &[u8]| {
            record_runtime_channel_failure(
                &process_failure,
                &handler_cancel,
                "fixture channel closed".into(),
            );
        });
        let started = Instant::now();
        let output = run_bounded_process_with_stdin(
            "/bin/sh",
            &[
                "-c".into(),
                "printf '%s\\n' '{\"type\":\"turn.started\"}'; sleep 5".into(),
            ],
            project.path(),
            Duration::from_secs(10),
            &process_cancel,
            None,
            Some(handler),
        )
        .unwrap();
        assert_eq!(output.termination, ProcessTermination::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(2));

        let mut result = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        result.executed = true;
        apply_runtime_channel_failure(&mut result, &failure);
        assert_eq!(result.outcome, RuntimeOutcome::ChannelFailed);
        assert!(!result.success);
        assert!(result
            .failure
            .as_deref()
            .unwrap()
            .contains("fixture channel closed"));
    }

    #[test]
    fn runtime_channel_failure_preserves_cleanup_failure_as_primary() {
        let project = tempfile::tempdir().unwrap();
        let failure = Mutex::new(Some("fixture channel closed".into()));
        let mut result = runtime_result(project.path(), RuntimeMode::Live, true, Vec::new());
        result.executed = true;
        result.outcome = RuntimeOutcome::CleanupFailed;
        result.failure = Some("failed to reap runtime process group".into());

        apply_runtime_channel_failure(&mut result, &failure);

        assert_eq!(result.outcome, RuntimeOutcome::CleanupFailed);
        assert!(!result.success);
        let diagnostic = result.failure.as_deref().unwrap();
        assert!(diagnostic.contains("failed to reap runtime process group"));
        assert!(diagnostic.contains("fixture channel closed"));
    }

    #[test]
    fn runtime_line_reader_caps_huge_single_and_unterminated_lines_while_draining() {
        for input in [
            vec![b'x'; 1024 * 1024],
            [vec![b'x'; 1024 * 1024], vec![b'\n']].concat(),
        ] {
            let emitted = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
            let captured = Arc::clone(&emitted);
            let (retained, truncated) = bounded_line_reader_with_limit(
                input.as_slice(),
                SKILL_SETUP_OUTPUT_LIMIT,
                move |line| {
                    captured.lock().unwrap().push(line.to_vec());
                },
            )
            .unwrap();
            assert_eq!(retained.len(), SKILL_SETUP_OUTPUT_LIMIT);
            assert!(truncated);
            assert!(emitted.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn runtime_line_reader_keeps_valid_jsonl_beyond_the_helper_capture_limit() {
        let event = b"{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"bounded evidence payload\"}}\n";
        let mut input = Vec::new();
        while input.len() <= 64 * 1024 {
            input.extend_from_slice(event);
        }
        input.extend_from_slice(b"{\"type\":\"turn.completed\",\"usage\":{}}\n");
        let emitted = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let captured = Arc::clone(&emitted);

        let (retained, truncated) =
            bounded_line_reader_with_limit(input.as_slice(), RUNTIME_OUTPUT_LIMIT, move |line| {
                captured.lock().unwrap().push(line.to_vec());
            })
            .unwrap();

        assert_eq!(retained, input);
        assert!(!truncated);
        assert_eq!(
            emitted.lock().unwrap().last().map(Vec::as_slice),
            Some(b"{\"type\":\"turn.completed\",\"usage\":{}}".as_slice())
        );
    }

    #[test]
    fn runtime_line_reader_caps_aggregate_jsonl_at_the_runtime_limit() {
        let event = b"{\"type\":\"turn.started\"}\n";
        let mut input = Vec::new();
        while input.len() <= RUNTIME_OUTPUT_LIMIT + event.len() {
            input.extend_from_slice(event);
        }
        let emitted = Arc::new(Mutex::new(0_usize));
        let captured = Arc::clone(&emitted);

        let (retained, truncated) =
            bounded_line_reader_with_limit(input.as_slice(), RUNTIME_OUTPUT_LIMIT, move |_| {
                *captured.lock().unwrap() += 1;
            })
            .unwrap();

        assert_eq!(retained.len(), RUNTIME_OUTPUT_LIMIT);
        assert!(truncated);
        assert!(*emitted.lock().unwrap() > 0);
    }

    #[cfg(unix)]
    #[test]
    fn runtime_run_ids_are_native_random_and_cancellation_safe() {
        let first = native_runtime_run_id().unwrap();
        let second = native_runtime_run_id().unwrap();
        assert_eq!(first.len(), 32);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn runtime_fallback_ids_keep_fixture_handles_portable_and_distinct() {
        let first = fallback_runtime_run_id(7, 1, 100);
        let second = fallback_runtime_run_id(7, 2, 100);
        assert_eq!(first.len(), 32);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn runtime_registry_rejects_duplicates_and_scopes_cancellation_to_the_active_root() {
        let registry = Arc::new(OperationRegistry::default());
        let first = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let run_id = "0123456789abcdef0123456789abcdef".to_string();
        let lease = registry
            .begin(first.path(), OperationKind::Runtime, Some(run_id.clone()))
            .unwrap();
        let duplicate = match registry.begin(other.path(), OperationKind::Helper, None) {
            Ok(_) => panic!("duplicate runtime invocation unexpectedly started"),
            Err(error) => error,
        };
        assert_eq!(duplicate.code, "operation_in_progress");
        assert_eq!(
            registry
                .cancel_root(other.path(), OperationKind::Helper)
                .unwrap_err()
                .code,
            "operation_mismatch"
        );
        assert!(registry.cancel_run(&run_id).unwrap());
        assert!(lease.cancel.load(Ordering::Acquire));
        drop(lease);
        assert!(!registry.cancel_run(&run_id).unwrap());
    }

    #[test]
    fn helper_registry_builds_only_fixed_argv_and_rejects_injection() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let canonical_root = fs::canonicalize(project.path()).unwrap();
        let canonical = canonical_root.to_string_lossy().to_string();
        let validated_preflight = collect_skills(&canonical_root)
            .into_iter()
            .find(|skill| skill.id == "build-right-preflight")
            .unwrap();
        assert!(!validated_preflight.executable);
        assert_eq!(validated_preflight.helpers, vec!["preflight-check"]);

        assert_eq!(
            prepare_helper(
                &canonical_root,
                &helper_invocation(HelperId::PreflightCheck)
            )
            .unwrap()
            .argv,
            vec!["-", "--cwd", &canonical, "--mode", "all", "--format", "json",]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            prepare_helper(&canonical_root, &helper_invocation(HelperId::ContinueCheck))
                .unwrap()
                .argv,
            vec!["-", "--cwd", &canonical, "--format", "json", "--strict",]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        );
        let execution = HelperInvocation {
            helper_id: HelperId::ExecutionCheck,
            mode: Some(HelperExecutionMode::TaskContract),
            task_path: Some("tasks/issues/007-test.md".into()),
            feature_request: None,
        };
        assert_eq!(
            prepare_helper(&canonical_root, &execution).unwrap().argv,
            vec![
                "-",
                "--cwd",
                &canonical,
                "--mode",
                "task-contract",
                "--task",
                "tasks/issues/007-test.md",
                "--format",
                "json",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>(),
        );
        let injected = HelperInvocation {
            task_path: Some("../escape.md".into()),
            ..execution
        };
        assert_eq!(
            prepare_helper(&canonical_root, &injected).unwrap_err().code,
            "invalid_helper_task"
        );

        let contract_path = project.path().join("skill-ui/build-right-preflight.json");
        let mut contract: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&contract_path).unwrap()).unwrap();
        contract["helpers"] = serde_json::json!([]);
        fs::write(&contract_path, contract.to_string()).unwrap();
        let undeclared = prepare_helper(
            &canonical_root,
            &helper_invocation(HelperId::PreflightCheck),
        );
        assert_eq!(
            undeclared.unwrap_err().code,
            "helper_contract_not_validated"
        );
    }

    #[test]
    fn helper_registry_hashes_match_the_supported_installed_release() {
        let fixtures = [
            (
                HelperId::PreflightCheck,
                include_bytes!(
                    "../../.agents/skills/build-right-preflight/scripts/preflight-check.ts"
                )
                .as_slice(),
            ),
            (
                HelperId::ContinueCheck,
                include_bytes!(
                    "../../.agents/skills/build-right-execution/scripts/continue-check.ts"
                )
                .as_slice(),
            ),
            (
                HelperId::ExecutionCheck,
                include_bytes!(
                    "../../.agents/skills/build-right-execution/scripts/execution-check.ts"
                )
                .as_slice(),
            ),
        ];
        for (helper_id, bytes) in fixtures {
            assert_eq!(bytes.len(), helper_spec(helper_id).expected_length);
            assert_eq!(
                format!("{:x}", Sha256::digest(bytes)),
                helper_spec(helper_id).sha256
            );
        }
    }

    #[test]
    fn helper_rejects_replaced_script_and_non_regular_leaf() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        let script = root.join(helper_spec(HelperId::PreflightCheck).script_path);
        fs::write(&script, "console.log('untrusted')").unwrap();
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_script_untrusted",
        );
        let mut same_length_replacement =
            include_bytes!("../../.agents/skills/build-right-preflight/scripts/preflight-check.ts")
                .to_vec();
        same_length_replacement[0] ^= 1;
        fs::write(&script, same_length_replacement).unwrap();
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_digest_mismatch",
        );
        fs::remove_file(&script).unwrap();
        fs::create_dir(&script).unwrap();
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_script_untrusted",
        );
    }

    #[cfg(unix)]
    #[test]
    fn helper_rejects_fifo_without_blocking_and_oversized_sparse_regular_file() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        let script = root.join(helper_spec(HelperId::PreflightCheck).script_path);
        fs::remove_file(&script).unwrap();
        let fifo_path = CString::new(script.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
        let started = Instant::now();
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_script_untrusted",
        );
        assert!(started.elapsed() < Duration::from_secs(1));

        fs::remove_file(&script).unwrap();
        let sparse = File::create(&script).unwrap();
        sparse.set_len(1024 * 1024 * 1024).unwrap();
        drop(sparse);
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_script_untrusted",
        );
    }

    #[cfg(unix)]
    #[test]
    fn helper_rejects_symlink_leaf_and_component() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        let spec = helper_spec(HelperId::PreflightCheck);
        let script = root.join(spec.script_path);
        let trusted_copy = root.join("trusted-helper.ts");
        fs::rename(&script, &trusted_copy).unwrap();
        symlink(&trusted_copy, &script).unwrap();
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_script_untrusted",
        );

        fs::remove_file(&script).unwrap();
        fs::rename(&trusted_copy, &script).unwrap();
        let scripts = script.parent().unwrap();
        let real_scripts = root.join("real-scripts");
        fs::rename(scripts, &real_scripts).unwrap();
        symlink(&real_scripts, scripts).unwrap();
        assert_eq!(
            prepare_helper(&root, &helper_invocation(HelperId::PreflightCheck))
                .unwrap_err()
                .code,
            "helper_script_untrusted",
        );
    }

    #[test]
    fn verified_helper_snapshot_survives_repository_substitution() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        let expected =
            include_bytes!("../../.agents/skills/build-right-preflight/scripts/preflight-check.ts")
                .as_slice();
        let output = serde_json::json!({
            "cwd": root.to_string_lossy(), "mode": "all",
            "decision": "ready-for-execution", "confidence": "high", "nextAction": "Run one task",
            "inventory": {}, "missingArtifacts": [], "readinessWarnings": [], "founderInputGaps": []
        });
        let result = execute_helper_with(
            &root,
            helper_invocation(HelperId::PreflightCheck),
            |root, argv, verified_bytes| {
                fs::write(
                    root.join(helper_spec(HelperId::PreflightCheck).script_path),
                    "console.log('substituted')",
                )
                .unwrap();
                assert_eq!(argv.first().map(String::as_str), Some("-"));
                assert_eq!(verified_bytes, expected);
                Ok(helper_process_output(
                    true,
                    output.to_string().as_bytes(),
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert_eq!(result.outcome, HelperOutcome::Completed);
        assert!(result.success);
    }

    #[test]
    fn execution_task_authorization_matches_the_installed_helper_inventory() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        fs::create_dir_all(project.path().join("issues")).unwrap();
        fs::create_dir_all(project.path().join("tasks/issues/nested")).unwrap();
        fs::write(project.path().join("tasks/root-task.md"), "# Root task").unwrap();
        fs::write(project.path().join("issues/root-issue.md"), "# Root issue").unwrap();
        fs::write(project.path().join("tasks/sprint-0.md"), "# Tracker").unwrap();
        fs::write(
            project.path().join("tasks/post-release-backlog.md"),
            "# Excluded",
        )
        .unwrap();
        fs::write(
            project.path().join("tasks/issues/nested/not-supported.md"),
            "# Nested",
        )
        .unwrap();
        let root = fs::canonicalize(project.path()).unwrap();

        for supported in [
            "tasks/issues/007-test.md",
            "tasks/root-task.md",
            "issues/root-issue.md",
        ] {
            assert_eq!(
                validated_helper_task_path(&root, supported).unwrap(),
                supported
            );
        }
        for rejected in [
            "tasks/sprint-0.md",
            "tasks/post-release-backlog.md",
            "tasks/issues/nested/not-supported.md",
        ] {
            assert_eq!(
                validated_helper_task_path(&root, rejected)
                    .unwrap_err()
                    .code,
                "invalid_helper_task"
            );
        }
    }

    #[test]
    fn helper_execution_output_binding_rejects_cwd_mode_and_task_mismatches() {
        let root = Path::new("/tmp/canonical-project");
        let invocation = HelperInvocation {
            helper_id: HelperId::ExecutionCheck,
            mode: Some(HelperExecutionMode::TaskContract),
            task_path: Some("tasks/issues/007-test.md".into()),
            feature_request: None,
        };
        let valid = serde_json::json!({
            "cwd": "/tmp/canonical-project", "mode": "task-contract",
            "selectedTask": { "path": "tasks/issues/007-test.md" }
        });
        assert!(
            verify_helper_output_binding(root, &invocation, valid.to_string().as_bytes()).is_ok()
        );
        for mismatch in [
            serde_json::json!({ "cwd": "/tmp/other", "mode": "task-contract", "selectedTask": { "path": "tasks/issues/007-test.md" } }),
            serde_json::json!({ "cwd": "/tmp/canonical-project", "mode": "all", "selectedTask": { "path": "tasks/issues/007-test.md" } }),
            serde_json::json!({ "cwd": "/tmp/canonical-project", "mode": "task-contract", "selectedTask": { "path": "tasks/issues/008-other.md" } }),
        ] {
            assert!(verify_helper_output_binding(
                root,
                &invocation,
                mismatch.to_string().as_bytes()
            )
            .is_err());
        }
    }

    #[test]
    fn helper_output_binding_mismatch_is_typed_and_refreshes_repository_truth() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        let invocation = HelperInvocation {
            helper_id: HelperId::ExecutionCheck,
            mode: Some(HelperExecutionMode::TaskContract),
            task_path: Some("tasks/issues/007-test.md".into()),
            feature_request: None,
        };
        let mismatched = serde_json::json!({
            "cwd": root.to_string_lossy(), "mode": "task-contract",
            "selectedTask": { "path": "tasks/issues/008-other.md" },
            "readyTaskCandidates": [], "contractMissing": [], "gateReasons": [],
            "recommendation": "Proceed with one bounded task: tasks/issues/008-other.md"
        });
        let result = execute_helper_with(&root, invocation, |_, _, _| {
            fs::create_dir_all(root.join("docs")).unwrap();
            fs::write(root.join("docs/refreshed.md"), "# Refreshed").unwrap();
            Ok(helper_process_output(
                true,
                mismatched.to_string().as_bytes(),
                ProcessTermination::Completed,
                false,
            ))
        })
        .unwrap();
        assert_eq!(result.outcome, HelperOutcome::VerificationFailed);
        assert!(!result.success);
        assert!(result
            .project
            .files
            .iter()
            .any(|file| file.path == "docs/refreshed.md"));
    }

    #[test]
    fn helper_unsupported_platform_is_terminal_without_spawning() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let result = execute_helper_with_platform(
            project.path(),
            helper_invocation(HelperId::PreflightCheck),
            false,
            |_, _, _| panic!("unsupported platform must not spawn a helper"),
        )
        .unwrap();
        assert_eq!(result.outcome, HelperOutcome::UnsupportedPlatform);
        assert!(!result.executed);
        assert!(!result.success);
    }

    #[cfg(unix)]
    #[test]
    fn helper_stdin_never_read_times_out_and_reaps_descendants() {
        let project = tempfile::tempdir().unwrap();
        let pid_path = project.path().join("descendant.pid");
        let cancel = AtomicBool::new(false);
        let input = vec![b'x'; 1024 * 1024];
        let started = Instant::now();
        let output = run_bounded_process_with_stdin(
            "/bin/sh",
            &descendant_fixture_args(&pid_path),
            project.path(),
            Duration::from_millis(150),
            &cancel,
            Some(&input),
            None,
        )
        .unwrap();
        assert_eq!(output.termination, ProcessTermination::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_process_gone(wait_for_descendant_pid(&pid_path));
    }

    #[cfg(unix)]
    #[test]
    fn helper_stdin_never_read_cancels_and_reaps_descendants() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().to_path_buf();
        let pid_path = root.join("descendant.pid");
        let process_pid_path = pid_path.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let process_cancel = Arc::clone(&cancel);
        let process = thread::spawn(move || {
            let input = vec![b'x'; 1024 * 1024];
            run_bounded_process_with_stdin(
                "/bin/sh",
                &descendant_fixture_args(&process_pid_path),
                &root,
                Duration::from_secs(5),
                &process_cancel,
                Some(&input),
                None,
            )
            .unwrap()
        });
        let descendant = wait_for_descendant_pid(&pid_path);
        cancel.store(true, Ordering::Release);
        let output = process.join().unwrap();
        assert_eq!(output.termination, ProcessTermination::Cancelled);
        assert_process_gone(descendant);
    }

    #[cfg(unix)]
    #[test]
    fn helper_bidirectional_backpressure_drains_output_before_stdin_is_read() {
        let project = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let input = vec![b'x'; 1024 * 1024];
        let argv = vec![
            "-c".into(),
            "head -c 131072 /dev/zero; head -c 131072 /dev/zero >&2; cat >/dev/null".into(),
        ];
        let output = run_bounded_process_with_stdin(
            "/bin/sh",
            &argv,
            project.path(),
            Duration::from_secs(5),
            &cancel,
            Some(&input),
            None,
        )
        .unwrap();
        assert_eq!(output.termination, ProcessTermination::Completed);
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), SKILL_SETUP_OUTPUT_LIMIT);
        assert_eq!(output.stderr.len(), SKILL_SETUP_OUTPUT_LIMIT);
        assert!(output.stdout_truncated);
        assert!(output.stderr_truncated);
    }

    #[test]
    fn helper_parsers_accept_current_preflight_planning_continue_and_execution_fixtures() {
        let preflight = serde_json::json!({
            "cwd": "/tmp/project", "mode": "all",
            "decision": "ready-for-execution", "confidence": "high", "nextAction": "Run one task",
            "inventory": { "docsMarkdownFiles": 4 }, "missingArtifacts": [],
            "readinessWarnings": [], "founderInputGaps": []
        });
        let continue_check = serde_json::json!({
            "cwd": "/tmp/project",
            "decision": "execute-task", "confidence": "high", "nextAction": "Execute task 007",
            "evidence": [{ "source": "tasks/sprint-0.md", "summary": "status active" }],
            "blockingGates": []
        });
        let planning = serde_json::json!({
            "cwd": "/tmp/project", "featureRequest": "Add guided planning",
            "decision": "update-sprint", "confidence": "medium",
            "recommendedDestination": "tasks/sprint-3.md",
            "blockingGates": [], "founderQuestions": [], "researchTriggers": [],
            "readyTaskCandidates": [], "nextAction": "Create bounded tasks",
            "scannedArtifacts": ["docs/mvp-scope.md", "tasks/sprint-3.md"]
        });
        let execution = serde_json::json!({
            "cwd": "/tmp/project", "mode": "task-contract",
            "selectedTask": { "path": "tasks/issues/007-test.md" },
            "readyTaskCandidates": [{ "path": "tasks/issues/007-test.md" }],
            "contractMissing": [], "gateReasons": [],
            "recommendation": "Proceed with one bounded task: tasks/issues/007-test.md"
        });
        assert_eq!(
            parse_preflight_output(preflight.to_string().as_bytes())
                .unwrap()
                .decision,
            "ready-for-execution"
        );
        assert_eq!(
            parse_continue_output(continue_check.to_string().as_bytes())
                .unwrap()
                .decision,
            "execute-task"
        );
        let planning_decision =
            parse_feature_planning_output(planning.to_string().as_bytes()).unwrap();
        assert_eq!(planning_decision.decision, "update-sprint");
        assert_eq!(
            planning_decision.recommended_destination.as_deref(),
            Some("tasks/sprint-3.md")
        );
        let decision = parse_execution_output(execution.to_string().as_bytes()).unwrap();
        assert_eq!(decision.decision, "proceed");
        assert_eq!(decision.confidence, "high");
    }

    #[test]
    fn helper_terminal_results_cover_malformed_timeout_cancellation_overflow_missing_runtime_and_nonzero(
    ) {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let canonical_root = fs::canonicalize(project.path()).unwrap();

        let cases = [
            (
                helper_process_output(true, b"not json", ProcessTermination::Completed, false),
                HelperOutcome::MalformedOutput,
            ),
            (
                helper_process_output(true, b"{}", ProcessTermination::TimedOut, false),
                HelperOutcome::TimedOut,
            ),
            (
                helper_process_output(true, b"{}", ProcessTermination::Cancelled, false),
                HelperOutcome::Cancelled,
            ),
            (
                helper_process_output(true, b"{}", ProcessTermination::Completed, true),
                HelperOutcome::OutputOverflow,
            ),
            (
                helper_process_output(false, b"{}", ProcessTermination::Completed, false),
                HelperOutcome::NonzeroExit,
            ),
        ];
        for (output, expected) in cases {
            let result = execute_helper_with(
                &canonical_root,
                helper_invocation(HelperId::PreflightCheck),
                |_, _, _| Ok(output),
            )
            .unwrap();
            assert_eq!(result.outcome, expected);
            assert!(!result.success);
            assert_eq!(
                result.project.root,
                fs::canonicalize(project.path()).unwrap().to_string_lossy()
            );
        }
        let missing = execute_helper_with(
            &canonical_root,
            helper_invocation(HelperId::PreflightCheck),
            |_, _, _| {
                Err(ProcessRunFailure::new(
                    ProcessRunFailureKind::MissingExecutable,
                    "bun missing",
                ))
            },
        )
        .unwrap();
        assert_eq!(missing.outcome, HelperOutcome::MissingRuntime);
    }

    #[test]
    fn helper_refreshes_repository_after_every_terminal_outcome() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_helper_fixture(project.path());
        let canonical_root = fs::canonicalize(project.path()).unwrap();
        let result = execute_helper_with(
            &canonical_root,
            helper_invocation(HelperId::PreflightCheck),
            |root, _, _| {
                fs::create_dir_all(root.join("docs")).unwrap();
                fs::write(root.join("docs/refresh.md"), "# refreshed").unwrap();
                Ok(helper_process_output(
                    false,
                    b"",
                    ProcessTermination::Completed,
                    false,
                ))
            },
        )
        .unwrap();
        assert!(result
            .project
            .files
            .iter()
            .any(|file| file.path == "docs/refresh.md"));
        assert_eq!(result.outcome, HelperOutcome::NonzeroExit);
    }

    #[test]
    fn helper_registry_rejects_duplicate_invokes_and_supports_cancellation() {
        let registry = Arc::new(OperationRegistry::default());
        let first_root = tempfile::tempdir().unwrap();
        let other_root = tempfile::tempdir().unwrap();
        let lease = registry
            .begin(first_root.path(), OperationKind::Helper, None)
            .unwrap();
        let duplicate = match registry.begin(other_root.path(), OperationKind::Helper, None) {
            Ok(_) => panic!("duplicate helper invocation unexpectedly started"),
            Err(error) => error,
        };
        assert_eq!(duplicate.code, "operation_in_progress");
        assert!(registry
            .cancel_root(first_root.path(), OperationKind::Helper)
            .unwrap());
        assert!(lease.cancel.load(Ordering::Acquire));
        drop(lease);
        assert!(registry
            .begin(other_root.path(), OperationKind::Helper, None)
            .is_ok());
    }

    #[test]
    fn missing_bun_runtime_is_classified_without_shell_fallback() {
        let cancel = AtomicBool::new(false);
        let error = match run_bounded_process(
            "/definitely/missing/bun",
            &[],
            Path::new("/tmp"),
            Duration::from_millis(10),
            &cancel,
        ) {
            Ok(_) => panic!("missing Bun runtime unexpectedly started"),
            Err(error) => error,
        };
        assert_eq!(error.kind, ProcessRunFailureKind::MissingExecutable);
    }

    #[cfg(unix)]
    #[test]
    fn bun_resolution_survives_launchservices_path_with_trusted_home_fallback() {
        use std::os::unix::fs::PermissionsExt;

        let home = tempfile::tempdir().unwrap();
        let bun = home.path().join(".bun/bin/bun");
        fs::create_dir_all(bun.parent().unwrap()).unwrap();
        fs::copy("/usr/bin/true", &bun).unwrap();
        fs::set_permissions(&bun, fs::Permissions::from_mode(0o755)).unwrap();

        let resolved = resolve_native_js_executable_from(
            NativeJsExecutable::Bun,
            Some(std::ffi::OsStr::new("/usr/bin:/bin")),
            Some(home.path()),
            &[],
        )
        .unwrap();

        assert_eq!(resolved, fs::canonicalize(bun).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn bun_resolution_rejects_untrusted_or_missing_candidates_without_shell_fallback() {
        use std::os::unix::fs::PermissionsExt;

        let home = tempfile::tempdir().unwrap();
        let bun = home.path().join(".bun/bin/bun");
        fs::create_dir_all(bun.parent().unwrap()).unwrap();
        fs::copy("/usr/bin/true", &bun).unwrap();
        fs::set_permissions(&bun, fs::Permissions::from_mode(0o777)).unwrap();

        let error = resolve_native_js_executable_from(
            NativeJsExecutable::Bun,
            None,
            Some(home.path()),
            &[],
        )
        .unwrap_err();

        assert_eq!(error.kind, ProcessRunFailureKind::MissingExecutable);
        assert!(!error
            .message
            .contains(home.path().to_string_lossy().as_ref()));
    }

    #[cfg(unix)]
    #[test]
    fn codex_launcher_survives_launchservices_path_with_trusted_sibling_node() {
        use std::os::unix::fs::PermissionsExt;

        let runtime = tempfile::tempdir().unwrap();
        let bin = runtime.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let node = bin.join("node");
        let codex = bin.join("codex");
        fs::write(&node, "#!/bin/sh\nexec /bin/sh \"$@\"\n").unwrap();
        fs::write(
            &codex,
            "#!/usr/bin/env node\nprintf 'codex-cli fixture\\n'\n",
        )
        .unwrap();
        fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();

        let cancel = AtomicBool::new(false);
        let output = run_codex_process_from(
            &codex,
            Some(std::ffi::OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin")),
            &["--version".into()],
            runtime.path(),
            Duration::from_secs(2),
            &cancel,
            None,
            SKILL_SETUP_OUTPUT_LIMIT,
        )
        .unwrap();

        assert!(output.status.success());
        assert_eq!(output.stdout, b"codex-cli fixture\n");
    }

    fn process_output(success: bool) -> BoundedProcessOutput {
        let status = Command::new(if success {
            "/usr/bin/true"
        } else {
            "/usr/bin/false"
        })
        .status()
        .unwrap();
        BoundedProcessOutput {
            status,
            termination: ProcessTermination::Completed,
            stdout: if success {
                b"setup complete".to_vec()
            } else {
                vec![]
            },
            stderr: if success {
                vec![]
            } else {
                b"fixture failure".to_vec()
            },
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    fn write_setup_state(root: &Path, suffix: &str) {
        let mut skills = serde_json::Map::new();
        for id in BUILD_RIGHT_SKILL_IDS {
            let directory = root.join(".agents/skills").join(id);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("SKILL.md"), format!("# {id} {suffix}\n")).unwrap();
            skills.insert(
                id.into(),
                serde_json::json!({
                    "source": SKILL_SETUP_SOURCE,
                    "computedHash": format!("hash-{id}-{suffix}")
                }),
            );
        }
        fs::write(
            root.join("skills-lock.json"),
            serde_json::json!({ "version": 1, "skills": skills }).to_string(),
        )
        .unwrap();
    }

    fn write_setup_contracts(root: &Path, suffix: &str) {
        fs::create_dir_all(root.join("skill-ui")).unwrap();
        for id in BUILD_RIGHT_SKILL_IDS {
            let phase = first_party_spec(id).unwrap().0;
            fs::write(
                root.join("skill-ui").join(format!("{id}.json")),
                serde_json::json!({
                    "version": 1,
                    "id": id,
                    "name": id,
                    "lifecyclePhase": phase,
                    "purpose": "Fixture contract",
                    "reads": ["docs/"],
                    "writes": ["tasks/"],
                    "decisions": ["ready"],
                    "helpers": [],
                    "requiredEvidence": ["fixture"],
                    "stopStates": ["blocked"],
                    "renderer": "operating-card",
                    "provenance": {
                        "source": SKILL_SETUP_SOURCE,
                        "installedPath": format!(".agents/skills/{id}/SKILL.md"),
                        "lockHash": format!("hash-{id}-{suffix}")
                    }
                })
                .to_string(),
            )
            .unwrap();
        }
    }

    #[test]
    fn skill_setup_preview_is_read_only_and_exposes_closed_contract() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());

        let preview = build_skill_setup_preview(
            project.path(),
            SkillSetupOperation::Install,
            SKILL_SETUP_SOURCE,
        )
        .unwrap();

        assert_eq!(preview.executable, "bun");
        assert_eq!(preview.cli_version, "skills@1.5.19");
        assert!(preview.preview_token.starts_with("sha256:"));
        assert_eq!(preview.argv, skill_setup_argv(SkillSetupOperation::Install));
        assert_eq!(preview.argv.first().map(String::as_str), Some("x"));
        assert!(preview.hash_changes.iter().all(|change| {
            change.current_hash.is_none()
                && change.proposed_hash.is_none()
                && change.proposed_state == "resolvedOnExecution"
        }));
        assert!(!project.path().join("skills-lock.json").exists());
        assert!(!project.path().join(".agents/skills").exists());
    }

    #[test]
    fn skill_setup_cancellation_never_calls_process_runner() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Install,
            false,
            "",
            |_, _| panic!("cancelled setup must not execute"),
        )
        .unwrap();

        assert!(!result.executed);
        assert!(!result.success);
        assert_eq!(result.repair.unwrap().code, "confirmation_required");
    }

    #[test]
    fn skill_setup_rejects_confirmation_after_lock_baseline_changes() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_setup_state(project.path(), "a");
        let preview_token =
            skill_setup_preview_token(project.path(), SkillSetupOperation::Update).unwrap();
        write_setup_state(project.path(), "b");

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Update,
            true,
            &preview_token,
            |_, _| panic!("stale setup preview must not spawn"),
        )
        .unwrap();

        assert_eq!(result.outcome, SkillSetupOutcome::StalePreview);
        assert!(!result.executed);
        assert!(!result.success);
        assert_eq!(result.repair.unwrap().code, "stale_skill_setup_preview");
        assert!(result.after.iter().all(|state| state
            .lock_hash
            .as_deref()
            .is_some_and(|hash| hash.ends_with("-b"))));
    }

    #[test]
    fn skill_setup_rejects_unsupported_source_and_operation_injection() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());

        let error = build_skill_setup_preview(
            project.path(),
            SkillSetupOperation::Install,
            "attacker/repo; touch /tmp/injected",
        )
        .unwrap_err();
        assert_eq!(error.code, "unsupported_skill_source");
        assert!(
            serde_json::from_str::<SkillSetupOperation>("\"install; touch /tmp/injected\"")
                .is_err()
        );
        assert!(!skill_setup_argv(SkillSetupOperation::Install)
            .iter()
            .any(|token| token.contains(';')));
    }

    #[test]
    fn failed_skill_setup_returns_repair_and_refreshed_repository() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Update,
            true,
            &skill_setup_preview_token(project.path(), SkillSetupOperation::Update).unwrap(),
            |root, argv| {
                assert_eq!(argv, skill_setup_argv(SkillSetupOperation::Update));
                fs::write(root.join("failure-marker.txt"), "refresh proof").unwrap();
                Ok(process_output(false))
            },
        )
        .unwrap();

        assert!(result.executed);
        assert!(!result.success);
        assert_eq!(result.exit_status, Some(1));
        assert_eq!(result.stderr, "fixture failure");
        assert_eq!(result.repair.unwrap().code, "skill_setup_failed");
        assert!(result.project.dirty);
    }

    #[test]
    fn successful_skill_setup_refreshes_exact_hashes_and_changed_paths() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_setup_state(project.path(), "before");

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Update,
            true,
            &skill_setup_preview_token(project.path(), SkillSetupOperation::Update).unwrap(),
            |root, _| {
                write_setup_state(root, "after");
                Ok(process_output(true))
            },
        )
        .unwrap();

        assert!(result.success);
        assert_eq!(result.after.len(), BUILD_RIGHT_SKILL_IDS.len());
        assert!(result.after.iter().all(|state| {
            state.installed
                && state.lock_hash.as_deref()
                    == Some(format!("hash-{}-after", state.skill_id).as_str())
        }));
        assert!(result.changed_paths.contains(&"skills-lock.json".into()));
        for id in BUILD_RIGHT_SKILL_IDS {
            assert!(result
                .changed_paths
                .contains(&format!(".agents/skills/{id}/SKILL.md")));
            assert!(result
                .changed_paths
                .contains(&format!("skill-ui/{id}.json")));
            let contract = validate_skill_contract(project.path(), id).unwrap();
            assert_eq!(
                contract.lock_hash.as_deref(),
                Some(format!("hash-{id}-after").as_str())
            );
        }
    }

    #[test]
    fn changed_hashes_with_stale_ui_contracts_cannot_report_success() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_setup_state(project.path(), "before");
        write_setup_contracts(project.path(), "before");
        assert!(
            relevant_skill_verification_errors(&inspect_project_path(project.path())).is_empty()
        );
        let preview_token =
            skill_setup_preview_token(project.path(), SkillSetupOperation::Update).unwrap();

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Update,
            true,
            &preview_token,
            |root, _| {
                write_setup_state(root, "after");
                Ok(process_output(true))
            },
        )
        .unwrap();

        assert!(result.executed);
        assert!(!result.success);
        assert_eq!(result.outcome, SkillSetupOutcome::VerificationFailed);
        assert_eq!(
            result.repair.unwrap().code,
            "post_setup_verification_failed"
        );
        assert!(!relevant_skill_verification_errors(&result.project).is_empty());
        assert!(result.changed_paths.contains(&"skills-lock.json".into()));
    }

    #[test]
    fn setup_output_capture_is_bounded_while_draining_input() {
        let input = vec![b'x'; SKILL_SETUP_OUTPUT_LIMIT + 17];
        let (output, truncated) =
            bounded_reader_with_limit(input.as_slice(), SKILL_SETUP_OUTPUT_LIMIT).unwrap();
        assert_eq!(output.len(), SKILL_SETUP_OUTPUT_LIMIT);
        assert!(truncated);
    }

    #[test]
    fn skill_setup_process_times_out_and_reaps_child() {
        let project = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let started = Instant::now();

        let output = run_bounded_process(
            "/bin/sleep",
            &["2".into()],
            project.path(),
            Duration::from_millis(30),
            &cancel,
        )
        .unwrap();

        assert_eq!(output.termination, ProcessTermination::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn skill_setup_process_honors_in_flight_cancellation() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().to_path_buf();
        let cancel = Arc::new(AtomicBool::new(false));
        let process_cancel = Arc::clone(&cancel);
        let process = thread::spawn(move || {
            run_bounded_process(
                "/bin/sleep",
                &["2".into()],
                &root,
                Duration::from_secs(2),
                &process_cancel,
            )
            .unwrap()
        });
        thread::sleep(Duration::from_millis(40));
        cancel.store(true, Ordering::Release);

        let output = process.join().unwrap();
        assert_eq!(output.termination, ProcessTermination::Cancelled);
    }

    #[test]
    fn skill_setup_checks_cancellation_before_spawning() {
        let project = tempfile::tempdir().unwrap();
        let marker = project.path().join("must-not-exist");
        let cancel = AtomicBool::new(true);

        let error = match run_bounded_process(
            "/usr/bin/touch",
            &[marker.to_string_lossy().to_string()],
            project.path(),
            Duration::from_secs(1),
            &cancel,
        ) {
            Ok(_) => panic!("pre-cancelled skill setup unexpectedly spawned"),
            Err(error) => error,
        };

        assert_eq!(error.kind, ProcessRunFailureKind::CancelledBeforeSpawn);
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    fn wait_for_descendant_pid(path: &Path) -> u32 {
        for _ in 0..100 {
            if let Ok(value) = fs::read_to_string(path) {
                if let Ok(pid) = value.trim().parse() {
                    return pid;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("descendant PID fixture was not written");
    }

    #[cfg(unix)]
    fn assert_process_gone(pid: u32) {
        for _ in 0..100 {
            let exists = unsafe { libc::kill(pid as i32, 0) } == 0;
            if !exists && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("descendant process {pid} survived process-group cleanup");
    }

    #[cfg(unix)]
    fn descendant_fixture_args(pid_path: &Path) -> Vec<String> {
        vec![
            "-c".into(),
            "sleep 30 & child=$!; echo $child > \"$1\"; wait".into(),
            "skill-setup-tree-fixture".into(),
            pid_path.to_string_lossy().to_string(),
        ]
    }

    #[cfg(unix)]
    #[test]
    fn skill_setup_timeout_terminates_descendants_that_inherit_pipes() {
        let project = tempfile::tempdir().unwrap();
        let pid_path = project.path().join("descendant.pid");
        let cancel = AtomicBool::new(false);

        let output = run_bounded_process(
            "/bin/sh",
            &descendant_fixture_args(&pid_path),
            project.path(),
            Duration::from_millis(150),
            &cancel,
        )
        .unwrap();

        assert_eq!(output.termination, ProcessTermination::TimedOut);
        assert_process_gone(wait_for_descendant_pid(&pid_path));
    }

    #[cfg(unix)]
    #[test]
    fn skill_setup_cancellation_terminates_descendants_that_inherit_pipes() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().to_path_buf();
        let pid_path = root.join("descendant.pid");
        let process_pid_path = pid_path.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let process_cancel = Arc::clone(&cancel);
        let process = thread::spawn(move || {
            run_bounded_process(
                "/bin/sh",
                &descendant_fixture_args(&process_pid_path),
                &root,
                Duration::from_secs(5),
                &process_cancel,
            )
            .unwrap()
        });
        let descendant = wait_for_descendant_pid(&pid_path);
        cancel.store(true, Ordering::Release);

        let output = process.join().unwrap();
        assert_eq!(output.termination, ProcessTermination::Cancelled);
        assert_process_gone(descendant);
    }

    #[test]
    fn skill_setup_registry_rejects_duplicate_invokes_without_holding_mutex() {
        let registry = Arc::new(OperationRegistry::default());
        let first_root = tempfile::tempdir().unwrap();
        let second_root = tempfile::tempdir().unwrap();
        let first = registry
            .begin(first_root.path(), OperationKind::SkillSetup, None)
            .unwrap();

        let duplicate = match registry.begin(
            second_root.path(),
            OperationKind::Runtime,
            Some("abcdefabcdefabcdefabcdefabcdefab".into()),
        ) {
            Ok(_) => panic!("duplicate skill setup unexpectedly acquired a lease"),
            Err(error) => error,
        };
        assert_eq!(duplicate.code, "operation_in_progress");
        assert!(registry
            .cancel_root(first_root.path(), OperationKind::SkillSetup)
            .unwrap());
        drop(first);
        assert!(registry
            .begin(second_root.path(), OperationKind::SkillSetup, None)
            .is_ok());
    }

    #[test]
    fn malformed_post_lock_preserves_content_derived_changed_paths() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_setup_state(project.path(), "before");

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Update,
            true,
            &skill_setup_preview_token(project.path(), SkillSetupOperation::Update).unwrap(),
            |root, _| {
                fs::write(
                    root.join(".agents/skills/build-right-preflight/SKILL.md"),
                    "# mutated before malformed lock\n",
                )
                .unwrap();
                fs::write(root.join("skills-lock.json"), "{ malformed").unwrap();
                Ok(process_output(true))
            },
        )
        .unwrap();

        assert_eq!(result.outcome, SkillSetupOutcome::VerificationFailed);
        assert!(result.changed_paths.contains(&"skills-lock.json".into()));
        assert!(result
            .changed_paths
            .contains(&".agents/skills/build-right-preflight/SKILL.md".into()));
        assert_eq!(
            result.repair.unwrap().code,
            "post_setup_verification_failed"
        );
    }

    #[test]
    fn cancelled_setup_still_refreshes_provenance_and_repository_truth() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        write_setup_state(project.path(), "before");

        let result = execute_skill_setup_with(
            project.path(),
            SkillSetupOperation::Update,
            true,
            &skill_setup_preview_token(project.path(), SkillSetupOperation::Update).unwrap(),
            |root, _| {
                write_setup_state(root, "cancelled");
                let mut output = process_output(false);
                output.termination = ProcessTermination::Cancelled;
                Ok(output)
            },
        )
        .unwrap();

        assert_eq!(result.outcome, SkillSetupOutcome::Cancelled);
        assert!(result.project.dirty);
        assert!(result.after.iter().all(|state| state
            .lock_hash
            .as_deref()
            .is_some_and(|hash| hash.ends_with("-cancelled"))));
        assert!(!result.changed_paths.is_empty());
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(validate_relative("../outside.md").is_err());
        assert!(validate_relative("docs/../../outside.md").is_err());
    }

    #[test]
    fn accepts_repo_relative_markdown() {
        assert_eq!(
            validate_relative("docs/mvp-scope.md").unwrap(),
            Path::new("docs/mvp-scope.md")
        );
    }

    #[test]
    fn rejects_stale_writes_without_changing_content() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        fs::create_dir(project.path().join("docs")).unwrap();
        let file = project.path().join("docs/plan.md");
        fs::write(&file, "# Current\n").unwrap();
        let selected = read_project_file_path(&root, "docs/plan.md").unwrap();
        fs::write(&file, "# Changed elsewhere\n").unwrap();

        let error = write_project_file(
            project.path().to_string_lossy().to_string(),
            "docs/plan.md".into(),
            "# Stale editor\n".into(),
            selected.version,
        )
        .unwrap_err();

        assert_eq!(error.code, "stale_version");
        assert_eq!(fs::read_to_string(file).unwrap(), "# Changed elsewhere\n");
    }

    #[test]
    fn empty_git_repository_returns_an_empty_snapshot() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let snapshot = inspect_project(project.path().to_string_lossy().to_string()).unwrap();

        assert!(snapshot.files.is_empty());
        assert!(snapshot.skills.is_empty());
        assert_ne!(snapshot.branch, "unavailable");
        assert!(snapshot.errors.is_empty());
    }

    #[test]
    fn rejects_a_nested_directory_instead_of_silently_widening_to_git_root() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        fs::create_dir(project.path().join("nested")).unwrap();

        let error = inspect_project(project.path().join("nested").to_string_lossy().to_string())
            .unwrap_err();

        assert_eq!(error.code, "not_repository_root");
        assert!(!error.committed);
    }

    #[test]
    fn inventories_agent_instructions_docs_and_task_trackers() {
        let project = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join("docs")).unwrap();
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        fs::write(project.path().join("AGENTS.md"), "# Instructions\n").unwrap();
        fs::write(project.path().join("docs/scope.md"), "# Scope\n").unwrap();
        fs::write(project.path().join("tasks/sprint-1.md"), "# Sprint\n").unwrap();
        fs::write(project.path().join("tasks/issues/005.md"), "# Task\n").unwrap();

        let snapshot = inspect_project_path(project.path());
        assert!(snapshot
            .files
            .iter()
            .any(|file| file.path == "AGENTS.md" && file.kind == "instruction"));
        assert!(snapshot
            .files
            .iter()
            .any(|file| file.path == "docs/scope.md" && file.kind == "document"));
        assert!(snapshot
            .files
            .iter()
            .any(|file| file.path == "tasks/sprint-1.md" && file.kind == "task"));
        assert!(snapshot
            .files
            .iter()
            .any(|file| file.path == "tasks/issues/005.md" && file.kind == "task"));
    }

    #[test]
    fn inventory_failures_are_accumulated_as_structured_errors() {
        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("docs"), "not a directory").unwrap();

        let snapshot = inspect_project_path(project.path());

        assert!(snapshot
            .errors
            .iter()
            .any(|error| error.code == "inventory_read_failed"
                && error.path.as_deref().unwrap().ends_with("docs")));
    }

    #[test]
    fn skill_inventory_accumulates_an_entry_level_failure() {
        let project = tempfile::tempdir().unwrap();
        let skill_directory = project.path().join(".agents/skills/test-skill");
        fs::create_dir_all(&skill_directory).unwrap();
        fs::write(skill_directory.join("SKILL.md"), "# Test skill\n").unwrap();

        let (skills, errors) = collect_skills_with_errors_using(project.path(), |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "injected entry inspection failure",
            ))
        });

        assert!(skills.is_empty());
        assert!(errors.iter().any(|error| {
            error.code == "inventory_read_failed"
                && error.message.contains("injected entry inspection failure")
        }));
    }

    #[test]
    fn successful_save_returns_refreshed_file_git_and_projected_task_state() {
        let project = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(project.path()).unwrap();
        init_repo(project.path());
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        let relative = "tasks/issues/005.md";
        fs::write(project.path().join(relative), "# Task\n\nStatus: ready\n").unwrap();
        for args in [
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Test User"],
            vec!["add", relative],
            vec!["commit", "-qm", "baseline"],
        ] {
            assert!(Command::new("git")
                .args(args)
                .current_dir(project.path())
                .status()
                .unwrap()
                .success());
        }
        let selected = read_project_file_path(&root, relative).unwrap();

        let result = write_project_file(
            project.path().to_string_lossy().to_string(),
            relative.into(),
            "# Task\n\nStatus: active\n".into(),
            selected.version.clone(),
        )
        .unwrap();

        assert_ne!(result.file.version, selected.version);
        assert_eq!(result.file.content, "# Task\n\nStatus: active\n");
        assert!(result.project.dirty);
        assert_eq!(
            result
                .project
                .files
                .iter()
                .find(|file| file.path == relative)
                .and_then(|file| file.status.as_deref()),
            Some("active")
        );
        assert!(fs::read_dir(project.path().join("tasks/issues"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tmp")));
    }

    #[test]
    fn rejects_a_competing_writer_during_atomic_replacement() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        fs::create_dir(root.join("docs")).unwrap();
        fs::write(root.join("docs/plan.md"), "# Original\n").unwrap();
        let selected = read_project_file_path(&root, "docs/plan.md").unwrap();

        let error = write_project_file_serialized(
            &root,
            "docs/plan.md",
            "# App edit\n",
            &selected.version,
            |path| fs::write(path, "# Competing edit\n").unwrap(),
        )
        .unwrap_err();

        assert_eq!(error.code, "stale_version");
        assert!(!error.committed);
        assert_eq!(
            fs::read_to_string(root.join("docs/plan.md")).unwrap(),
            "# Competing edit\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_leaf_path_swap_during_atomic_replacement() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        fs::create_dir(root.join("docs")).unwrap();
        fs::write(root.join("docs/plan.md"), "# Original\n").unwrap();
        let selected = read_project_file_path(&root, "docs/plan.md").unwrap();

        let error = write_project_file_serialized(
            &root,
            "docs/plan.md",
            "# App edit\n",
            &selected.version,
            |path| {
                fs::rename(path, path.with_extension("backup")).unwrap();
                fs::write(path, "# Replacement inode\n").unwrap();
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "path_changed");
        assert!(!error.committed);
        assert_eq!(
            fs::read_to_string(root.join("docs/plan.md")).unwrap(),
            "# Replacement inode\n"
        );
    }

    #[test]
    fn serializes_two_in_app_writes_and_rejects_the_loser_as_stale() {
        use std::sync::mpsc;
        use std::time::Duration;

        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        fs::create_dir(root.join("docs")).unwrap();
        fs::write(root.join("docs/plan.md"), "# Original\n").unwrap();
        let version = read_project_file_path(&root, "docs/plan.md")
            .unwrap()
            .version;
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (second_tx, second_rx) = mpsc::channel();

        let first_root = root.clone();
        let first_version = version.clone();
        let first = std::thread::spawn(move || {
            write_project_file_serialized(
                &first_root,
                "docs/plan.md",
                "# First\n",
                &first_version,
                |_| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
        });
        entered_rx.recv().unwrap();

        let second_root = root.clone();
        let second = std::thread::spawn(move || {
            let result = write_project_file_serialized(
                &second_root,
                "docs/plan.md",
                "# Second\n",
                &version,
                |_| {},
            );
            second_tx.send(result.map(|_| ())).unwrap();
        });
        assert!(second_rx.recv_timeout(Duration::from_millis(100)).is_err());
        release_tx.send(()).unwrap();
        first.join().unwrap().unwrap();
        let second_error = second_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap_err();
        second.join().unwrap();

        assert_eq!(second_error.code, "stale_version");
        assert_eq!(
            fs::read_to_string(root.join("docs/plan.md")).unwrap(),
            "# First\n"
        );
    }

    #[test]
    fn post_persist_errors_are_marked_as_committed() {
        let error = ProjectError::new(
            "post_persist_verification_failed",
            "readback failed",
            Some(Path::new("docs/plan.md")),
        )
        .after_commit();

        assert!(error.committed);
    }

    #[test]
    fn validates_a_first_party_skill_contract_with_locked_provenance() {
        let project = tempfile::tempdir().unwrap();
        write_contract_fixture(project.path(), valid_contract());

        let skill = validate_skill_contract(project.path(), TEST_SKILL).unwrap();
        assert_eq!(skill.phase, "Discover");
        assert_eq!(skill.helpers, vec!["preflight-check"]);
        assert_eq!(skill.lock_hash.as_deref(), Some(TEST_HASH));
        assert!(!skill.executable);
    }

    #[test]
    fn rejects_unknown_contract_versions_and_unapproved_helpers() {
        let project = tempfile::tempdir().unwrap();
        let mut wrong_version = valid_contract();
        wrong_version["version"] = serde_json::json!(2);
        write_contract_fixture(project.path(), wrong_version);
        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("version"));

        let project = tempfile::tempdir().unwrap();
        let mut malformed = valid_contract();
        malformed.as_object_mut().unwrap().remove("renderer");
        write_contract_fixture(project.path(), malformed);
        assert!(validate_skill_contract(project.path(), TEST_SKILL).is_err());

        let project = tempfile::tempdir().unwrap();
        let mut executable_helper = valid_contract();
        executable_helper["helpers"] =
            serde_json::json!([{ "id": "shell", "execution": "automatic" }]);
        write_contract_fixture(project.path(), executable_helper);
        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("helper"));
    }

    #[test]
    fn rejects_provenance_lock_mismatches() {
        let project = tempfile::tempdir().unwrap();
        let mut mismatch = valid_contract();
        mismatch["provenance"]["lockHash"] = serde_json::json!("different");
        write_contract_fixture(project.path(), mismatch);

        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("skills-lock"));

        let project = tempfile::tempdir().unwrap();
        let mut source_mismatch = valid_contract();
        source_mismatch["provenance"]["source"] = serde_json::json!("wrong/source");
        write_contract_fixture(project.path(), source_mismatch);
        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("skills-lock"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_regular_and_dangling_symlinks_for_lock_and_ui_contract_files() {
        use std::os::unix::fs::symlink;

        for relative in ["skills-lock.json", "skill-ui/build-right-preflight.json"] {
            for dangling in [false, true] {
                let project = tempfile::tempdir().unwrap();
                let outside = tempfile::tempdir().unwrap();
                write_contract_fixture(project.path(), valid_contract());
                let protected = project.path().join(relative);
                fs::remove_file(&protected).unwrap();
                let target = outside.path().join(if dangling {
                    "missing.json"
                } else {
                    "authority.json"
                });
                if !dangling {
                    fs::write(&target, "{}").unwrap();
                }
                symlink(&target, &protected).unwrap();

                let error = validate_skill_contract(project.path(), TEST_SKILL).unwrap_err();
                assert!(
                    error.contains("non-symlink regular file"),
                    "{relative}: {error}"
                );
            }
        }
    }

    #[test]
    fn rejects_cross_skill_helpers_and_blank_semantic_values() {
        let project = tempfile::tempdir().unwrap();
        let mut cross_skill = valid_contract();
        cross_skill["helpers"] =
            serde_json::json!([{ "id": "execution-check", "execution": "explicit-user-action" }]);
        write_contract_fixture(project.path(), cross_skill);
        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("helper"));

        let project = tempfile::tempdir().unwrap();
        let mut blank = valid_contract();
        blank["purpose"] = serde_json::json!("");
        write_contract_fixture(project.path(), blank);
        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("blank"));

        let project = tempfile::tempdir().unwrap();
        let mut blank_array_value = valid_contract();
        blank_array_value["reads"] = serde_json::json!([""]);
        write_contract_fixture(project.path(), blank_array_value);
        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("blank"));
    }

    #[test]
    fn unknown_installed_skills_get_a_non_executable_generic_fallback() {
        let project = tempfile::tempdir().unwrap();
        let skill_dir = project.path().join(".agents/skills/unknown-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Unknown").unwrap();

        let skills = collect_skills(project.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].renderer, "generic-markdown");
        assert!(!skills[0].executable);
        assert!(skills[0].helpers.is_empty());
    }

    #[test]
    fn unknown_skills_cannot_supply_a_first_party_contract() {
        let project = tempfile::tempdir().unwrap();
        let skill_dir = project.path().join(".agents/skills/unknown-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Unknown").unwrap();
        fs::create_dir_all(project.path().join("skill-ui")).unwrap();
        let mut contract = valid_contract();
        contract["id"] = serde_json::json!("unknown-skill");
        contract["provenance"]["installedPath"] =
            serde_json::json!(".agents/skills/unknown-skill/SKILL.md");
        fs::write(
            project.path().join("skill-ui/unknown-skill.json"),
            contract.to_string(),
        )
        .unwrap();
        fs::write(
            project.path().join("skills-lock.json"),
            serde_json::json!({ "version": 1, "skills": { "unknown-skill": { "source": "test/source", "computedHash": TEST_HASH } } }).to_string(),
        )
        .unwrap();

        let skills = collect_skills(project.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].renderer, "generic-markdown");
        assert!(!skills[0].executable);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_first_party_skill_installations() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write_contract_fixture(project.path(), valid_contract());
        fs::write(outside.path().join("SKILL.md"), "# Outside skill").unwrap();
        let installed = project.path().join(".agents/skills").join(TEST_SKILL);
        fs::remove_file(installed.join("SKILL.md")).unwrap();
        fs::remove_dir(&installed).unwrap();
        symlink(outside.path(), &installed).unwrap();

        assert!(validate_skill_contract(project.path(), TEST_SKILL)
            .unwrap_err()
            .contains("regular project-scoped directory"));
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinked_inventory_directories() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(project.path().join("docs")).unwrap();
        fs::write(outside.path().join("outside.md"), "# Outside").unwrap();
        symlink(outside.path(), project.path().join("docs/external")).unwrap();

        let mut files = Vec::new();
        let mut errors = Vec::new();
        collect_markdown(project.path(), "docs", &mut files, &mut errors);
        assert!(files.is_empty());
        assert!(errors.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinked_inventory_root() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.md"), "# Outside").unwrap();
        symlink(outside.path(), project.path().join("docs")).unwrap();

        let mut files = Vec::new();
        let mut errors = Vec::new();
        collect_markdown(project.path(), "docs", &mut files, &mut errors);
        assert!(files.is_empty());
        assert!(errors.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_write_target_outside_root() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        init_repo(project.path());
        fs::create_dir(project.path().join("docs")).unwrap();
        let outside_file = outside.path().join("outside.md");
        fs::write(&outside_file, "# Outside").unwrap();
        symlink(&outside_file, project.path().join("docs/linked.md")).unwrap();

        assert!(resolve_writable(project.path(), "docs/linked.md").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_dangling_symlink_without_creating_outside_target() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        init_repo(project.path());
        fs::create_dir(project.path().join("docs")).unwrap();
        let outside_file = outside.path().join("new.md");
        symlink(&outside_file, project.path().join("docs/linked.md")).unwrap();

        let result = write_project_file(
            project.path().to_string_lossy().to_string(),
            "docs/linked.md".into(),
            "# Must stay inside".into(),
            "sha256:unused".into(),
        );

        assert!(result.unwrap_err().message.contains("cannot be a symlink"));
        assert!(!outside_file.exists());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    debug_assert_eq!(command_contract::NATIVE_COMMAND_NAMES.len(), 31);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(MdsyncSessionStore::default())
        .manage(PublishPlanStore::default())
        .manage(ArtifactPlanStore::default())
        .manage(LocalGitHandoffStore::default())
        .invoke_handler(tauri::generate_handler![
            inspect_project_command,
            inspect_post_run_review,
            preview_local_git_handoff,
            apply_local_git_handoff,
            read_project_file,
            write_project_file,
            preview_skill_setup,
            execute_skill_setup,
            cancel_skill_setup,
            execute_helper,
            cancel_helper,
            preview_bounded_task,
            preview_shared_bounded_task,
            recover_goal_state,
            clear_goal_state,
            execute_bounded_task,
            execute_shared_bounded_task,
            repair_collaboration_completion,
            cancel_bounded_task,
            execute_runtime,
            cancel_runtime,
            connect_mdsync_session,
            disconnect_mdsync_session,
            list_mdsync_files,
            read_mdsync_file,
            write_mdsync_file,
            preview_ha2ha_publish,
            apply_ha2ha_publish,
            join_ha2ha_workspace,
            preview_artifact_plan,
            apply_artifact_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running Build Right Studio");
}
