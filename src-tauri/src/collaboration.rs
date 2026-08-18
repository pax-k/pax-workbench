use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_IDENTIFIER_BYTES: usize = 512;
const FORBIDDEN_SECRET_MARKERS: [&str; 25] = [
    "authorization",
    "bearer ",
    "token:",
    "token=",
    "\"token\"",
    "api_token",
    "api-token",
    "apitoken",
    "id_token",
    "id-token",
    "idtoken",
    "access_token",
    "access-token",
    "accesstoken",
    "refresh_token",
    "refresh-token",
    "refreshtoken",
    "edit=",
    "read=",
    "?k=",
    "&k=",
    "provider payload",
    "provider_payload",
    "provider-payload",
    "providerpayload",
];
const FORBIDDEN_CONTENT_MARKERS: [&str; 8] = [
    "authorization",
    "bearer ",
    "access_token",
    "refresh_token",
    "capability",
    "provider payload",
    "raw payload",
    "secret",
];

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CollaborationMode {
    Disabled,
    #[default]
    LocalOnly,
    Viewer,
    SharedCollaborator,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CollaborationAccess {
    Public,
    Viewer,
    Collaborator,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct LocalSessionHandle(String);

impl LocalSessionHandle {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, CollaborationFailure> {
        let value = value.into();
        let suffix = value.strip_prefix("local-session-").unwrap_or_default();
        if suffix.len() != 32
            || !suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "sessionId must be a locally minted non-capability handle",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) struct CapabilityMaterial(Box<str>);

impl CapabilityMaterial {
    pub(crate) fn new(value: impl Into<Box<str>>) -> Result<Self, CollaborationFailure> {
        let value = value.into();
        if value.is_empty() {
            return Err(invalid_state(
                "missing_capability",
                "Capability material cannot be empty",
            ));
        }
        Ok(Self(value))
    }
}

impl fmt::Debug for CapabilityMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityMaterial([REDACTED])")
    }
}

impl Drop for CapabilityMaterial {
    fn drop(&mut self) {
        // Best-effort in-place clearing; this type is deliberately neither
        // serializable nor cloneable and is consumed only by native requests.
        unsafe {
            self.0.as_bytes_mut().fill(0);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SanitizedSessionMetadata {
    pub(crate) session_id: LocalSessionHandle,
    pub(crate) workspace_id: String,
    pub(crate) web_origin: String,
    pub(crate) api_origin: String,
    pub(crate) access: CollaborationAccess,
    pub(crate) actor: String,
}

impl SanitizedSessionMetadata {
    pub(crate) fn new(
        session_id: LocalSessionHandle,
        workspace_id: String,
        web_origin: String,
        api_origin: String,
        access: CollaborationAccess,
        actor: String,
    ) -> Result<Self, CollaborationFailure> {
        validate_identifier("workspaceId", &workspace_id)?;
        validate_identifier("actor", &actor)?;
        validate_origin("webOrigin", &web_origin)?;
        validate_origin("apiOrigin", &api_origin)?;
        Ok(Self {
            session_id,
            workspace_id,
            web_origin,
            api_origin,
            access,
            actor,
        })
    }

    pub(crate) fn capability_alias_candidates(&self) -> [&str; 5] {
        [
            self.session_id.as_str(),
            self.workspace_id.as_str(),
            self.web_origin.as_str(),
            self.api_origin.as_str(),
            self.actor.as_str(),
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalSourceBinding {
    pub(crate) task_path: String,
    pub(crate) task_sha256: String,
    pub(crate) repository_id: String,
    pub(crate) git_head: Option<String>,
    pub(crate) git_index_sha256: String,
    pub(crate) git_worktree_sha256: String,
    pub(crate) git_dirty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RemoteTaskBinding {
    pub(crate) task_id: String,
    pub(crate) task_path: String,
    pub(crate) base_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RemoteClaimPreview {
    pub(crate) task_path: String,
    pub(crate) base_version: u64,
    pub(crate) from_state: String,
    pub(crate) to_state: String,
    pub(crate) owner: String,
    pub(crate) updated_by: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SharedExecutionBinding {
    pub(crate) session: SanitizedSessionMetadata,
    pub(crate) local: LocalSourceBinding,
    pub(crate) remote: RemoteTaskBinding,
    pub(crate) expected_remote_mutation: RemoteClaimPreview,
}

impl SharedExecutionBinding {
    pub(crate) fn new(
        session: SanitizedSessionMetadata,
        local: LocalSourceBinding,
        remote: RemoteTaskBinding,
    ) -> Result<Self, CollaborationFailure> {
        let expected_remote_mutation = RemoteClaimPreview {
            task_path: remote.task_path.clone(),
            base_version: remote.base_version,
            from_state: "ready".into(),
            to_state: "claimed".into(),
            owner: session.actor.clone(),
            updated_by: session.actor.clone(),
        };
        let binding = Self {
            session,
            local,
            remote,
            expected_remote_mutation,
        };
        validate_shared_execution_binding(&binding)?;
        validate_serialized_contract("sharedExecutionBinding", &binding)?;
        Ok(binding)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CollaborationFailureClass {
    InvalidInput,
    AccessDenied,
    SourceMismatch,
    VersionConflict,
    TransportUnavailable,
    Timeout,
    Cancelled,
    Protocol,
    RepairRequired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepairHint {
    code: String,
    message: String,
    next_action: String,
}

impl RepairHint {
    pub(crate) fn reconnect() -> Self {
        Self {
            code: "reconnect".into(),
            message: "Collaboration state must be refreshed".into(),
            next_action: "Reconnect and review current remote state".into(),
        }
    }

    pub(crate) fn retry_sync() -> Self {
        Self {
            code: "retry-sync".into(),
            message: "Verified local evidence still requires remote synchronization".into(),
            next_action: "Retry the bounded evidence synchronization".into(),
        }
    }

    pub(crate) fn reconcile_claimed_pre_spawn() -> Self {
        Self {
            code: "reconcile-claimed-pre-spawn".into(),
            message: "The remote task is claimed but Codex did not start".into(),
            next_action: "Inspect and explicitly release or reconcile the remote claim before generating a fresh preview; never reuse the consumed confirmation".into(),
        }
    }

    pub(crate) fn refresh_conflict() -> Self {
        Self {
            code: "refresh-conflict".into(),
            message: "The remote task changed at the confirmed version boundary".into(),
            next_action:
                "Refresh the shared preview and explicitly confirm the exact latest version".into(),
        }
    }

    pub(crate) fn inspect_repeated_conflict() -> Self {
        Self {
            code: "inspect-repeated-conflict".into(),
            message: "The remote task conflicted again after a fresh confirmation".into(),
            next_action:
                "Stop automatic claim attempts and inspect the remote task coordinate with a human"
                    .into(),
        }
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn next_action(&self) -> &str {
        &self.next_action
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ReconciliationState {
    Disabled,
    LocalOnly,
    Disconnected,
    Reconciled,
    Claimed,
    SyncPending,
    RepairRequired,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub(crate) enum ClaimResult {
    NotRequired,
    Claimed {
        remote_version: u64,
    },
    Stopped {
        failure_class: CollaborationFailureClass,
        latest_remote_version: Option<u64>,
        repair: RepairHint,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct EvidenceReferenceId(String);

impl EvidenceReferenceId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, CollaborationFailure> {
        parse_local_reference("evidenceId", "evidence-", value.into()).map(Self)
    }

    fn validate(&self) -> Result<(), CollaborationFailure> {
        Self::parse(self.0.clone()).map(|_| ())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct HandoffReferenceId(String);

impl HandoffReferenceId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, CollaborationFailure> {
        parse_local_reference("handoffId", "handoff-", value.into()).map(Self)
    }

    fn validate(&self) -> Result<(), CollaborationFailure> {
        Self::parse(self.0.clone()).map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MissingCollaborationEffect {
    EvidenceWrite,
    TaskUpdate,
    HandoffWrite,
    StatusWrite,
}

impl MissingCollaborationEffect {
    pub(crate) const ORDER: [Self; 4] = [
        Self::EvidenceWrite,
        Self::TaskUpdate,
        Self::HandoffWrite,
        Self::StatusWrite,
    ];
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionArtifact {
    pub(crate) path: String,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RemoteCompletionIntent {
    pub(crate) workspace_id: String,
    pub(crate) access: CollaborationAccess,
    pub(crate) actor: String,
    pub(crate) task_id: String,
    pub(crate) remote_task_path: String,
    pub(crate) claimed_task_version: u64,
    pub(crate) source_task_sha256: String,
    pub(crate) local_task_path: String,
    pub(crate) local_task_sha256: String,
    pub(crate) repository_id: String,
    pub(crate) run_id: String,
    pub(crate) created_at_unix_seconds: u64,
    pub(crate) evidence_id: String,
    pub(crate) evidence_path: String,
    pub(crate) handoff_id: String,
    pub(crate) handoff_path: String,
    pub(crate) artifacts: Vec<CompletionArtifact>,
}

impl RemoteCompletionIntent {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        workspace_id: String,
        actor: String,
        task_id: String,
        remote_task_path: String,
        claimed_task_version: u64,
        source_task_sha256: String,
        local_task_path: String,
        local_task_sha256: String,
        repository_id: String,
        run_id: String,
        created_at_unix_seconds: u64,
        evidence_id: String,
        evidence_path: String,
        handoff_id: String,
        handoff_path: String,
        mut artifacts: Vec<CompletionArtifact>,
    ) -> Result<Self, CollaborationFailure> {
        artifacts.sort_by(|left, right| left.path.cmp(&right.path));
        let intent = Self {
            workspace_id,
            access: CollaborationAccess::Collaborator,
            actor,
            task_id,
            remote_task_path,
            claimed_task_version,
            source_task_sha256,
            local_task_path,
            local_task_sha256,
            repository_id,
            run_id,
            created_at_unix_seconds,
            evidence_id,
            evidence_path,
            handoff_id,
            handoff_path,
            artifacts,
        };
        intent.validate()?;
        validate_serialized_contract("remoteCompletionIntent", &intent)?;
        Ok(intent)
    }

    pub(crate) fn validate(&self) -> Result<(), CollaborationFailure> {
        for (field, value) in [
            ("workspaceId", self.workspace_id.as_str()),
            ("actor", self.actor.as_str()),
            ("taskId", self.task_id.as_str()),
            ("remoteTaskPath", self.remote_task_path.as_str()),
            ("localTaskPath", self.local_task_path.as_str()),
            ("evidencePath", self.evidence_path.as_str()),
            ("handoffPath", self.handoff_path.as_str()),
        ] {
            validate_identifier(field, value)?;
        }
        for (field, value) in [
            ("sourceTaskSha256", self.source_task_sha256.as_str()),
            ("localTaskSha256", self.local_task_sha256.as_str()),
            ("repositoryId", self.repository_id.as_str()),
        ] {
            validate_sha256(field, value)?;
        }
        if self.access != CollaborationAccess::Collaborator
            || self.claimed_task_version == 0
            || self.created_at_unix_seconds == 0
            || self.created_at_unix_seconds > 253_402_300_799
            || self.run_id.len() != 32
            || !self
                .run_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.artifacts.is_empty()
            || self.artifacts.len() > 16
        {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "Completion intent access, version, timestamp, run, or artifact bounds are invalid",
            ));
        }
        EvidenceReferenceId::parse(self.evidence_id.clone())?;
        HandoffReferenceId::parse(self.handoff_id.clone())?;
        if !self.evidence_path.starts_with("evidence/")
            || !self.evidence_path.ends_with(".md")
            || !self.handoff_path.starts_with("logs/")
            || !self.handoff_path.ends_with(".md")
        {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "Completion intent paths must use the portable evidence and log directories",
            ));
        }
        let mut artifact_paths = std::collections::BTreeSet::new();
        for artifact in &self.artifacts {
            validate_identifier("artifactPath", &artifact.path)?;
            validate_sha256("artifactSha256", &artifact.sha256)?;
            if !artifact_paths.insert(artifact.path.as_str()) {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    "Completion intent artifact paths must be unique",
                ));
            }
        }
        if self
            .artifacts
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
        {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "Completion intent artifacts must use canonical path order",
            ));
        }
        Ok(())
    }

    pub(crate) fn capability_alias_candidates(&self) -> Vec<&str> {
        let mut candidates = vec![
            self.workspace_id.as_str(),
            self.actor.as_str(),
            self.task_id.as_str(),
            self.remote_task_path.as_str(),
            self.source_task_sha256.as_str(),
            self.local_task_path.as_str(),
            self.local_task_sha256.as_str(),
            self.repository_id.as_str(),
            self.run_id.as_str(),
            self.evidence_id.as_str(),
            self.evidence_path.as_str(),
            self.handoff_id.as_str(),
            self.handoff_path.as_str(),
        ];
        for artifact in &self.artifacts {
            candidates.push(artifact.path.as_str());
            candidates.push(artifact.sha256.as_str());
        }
        candidates
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub(crate) enum EvidenceHandoffResult {
    NotRequired,
    Synchronized {
        remote_version: u64,
        evidence_ids: Vec<EvidenceReferenceId>,
        handoff_id: Option<HandoffReferenceId>,
    },
    Partial {
        remote_version: Option<u64>,
        missing_effects: Vec<MissingCollaborationEffect>,
        repair: RepairHint,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreRunCollaborationContext {
    pub(crate) mode: CollaborationMode,
    pub(crate) session: Option<SanitizedSessionMetadata>,
    pub(crate) local: LocalSourceBinding,
    pub(crate) remote: Option<RemoteTaskBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PostLocalCommitCollaborationContext {
    pub(crate) mode: CollaborationMode,
    pub(crate) session: Option<SanitizedSessionMetadata>,
    pub(crate) local: LocalSourceBinding,
    pub(crate) remote: Option<RemoteTaskBinding>,
    pub(crate) run_id: String,
    pub(crate) intent: Option<RemoteCompletionIntent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreRunCollaborationOutcome {
    pub(crate) reconciliation: ReconciliationState,
    pub(crate) claim: ClaimResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PostLocalCommitCollaborationOutcome {
    pub(crate) reconciliation: ReconciliationState,
    pub(crate) evidence_handoff: EvidenceHandoffResult,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ControllerCollaborationPolicy {
    pub(crate) mode: CollaborationMode,
    pub(crate) session: Option<SanitizedSessionMetadata>,
    pub(crate) remote: Option<RemoteTaskBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollaborationFailure {
    class: CollaborationFailureClass,
    code: String,
    message: String,
    repair: Option<RepairHint>,
}

impl CollaborationFailure {
    pub(crate) fn capability_material_rejected() -> Self {
        typed_failure(
            CollaborationFailureClass::InvalidInput,
            "capability_material_rejected",
            "Capability material cannot cross the sanitized collaboration boundary",
            None,
        )
    }

    pub(crate) fn access_denied() -> Self {
        typed_failure(
            CollaborationFailureClass::AccessDenied,
            "access_denied",
            "Collaboration access does not permit this operation",
            None,
        )
    }

    pub(crate) fn source_mismatch() -> Self {
        typed_failure(
            CollaborationFailureClass::SourceMismatch,
            "source_mismatch",
            "Local and remote task source bindings do not match",
            Some(RepairHint::reconnect()),
        )
    }

    pub(crate) fn version_conflict() -> Self {
        typed_failure(
            CollaborationFailureClass::VersionConflict,
            "version_conflict",
            "Remote collaboration state changed after confirmation",
            Some(RepairHint::reconnect()),
        )
    }

    pub(crate) fn transport_unavailable() -> Self {
        typed_failure(
            CollaborationFailureClass::TransportUnavailable,
            "transport_unavailable",
            "Collaboration transport is unavailable",
            Some(RepairHint::reconnect()),
        )
    }

    pub(crate) fn timeout() -> Self {
        typed_failure(
            CollaborationFailureClass::Timeout,
            "timeout",
            "Collaboration transport exceeded its bounded timeout",
            Some(RepairHint::reconnect()),
        )
    }

    pub(crate) fn cancelled() -> Self {
        typed_failure(
            CollaborationFailureClass::Cancelled,
            "cancelled",
            "Collaboration operation was cancelled",
            None,
        )
    }

    pub(crate) fn protocol() -> Self {
        typed_failure(
            CollaborationFailureClass::Protocol,
            "protocol_error",
            "Collaboration response did not match the supported protocol",
            Some(RepairHint::reconnect()),
        )
    }

    pub(crate) fn repair_required() -> Self {
        typed_failure(
            CollaborationFailureClass::RepairRequired,
            "repair_required",
            "Verified local evidence still requires remote synchronization",
            Some(RepairHint::retry_sync()),
        )
    }

    pub(crate) fn claimed_pre_spawn_repair_required() -> Self {
        typed_failure(
            CollaborationFailureClass::RepairRequired,
            "claimed_pre_spawn_repair_required",
            "The remote task is claimed but Codex did not start",
            Some(RepairHint::reconcile_claimed_pre_spawn()),
        )
    }

    pub(crate) fn class(&self) -> CollaborationFailureClass {
        self.class
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn repair(&self) -> Option<&RepairHint> {
        self.repair.as_ref()
    }
}

pub(crate) trait CollaborationPort: Send + Sync {
    fn before_runtime(
        &self,
        context: &PreRunCollaborationContext,
        cancel: &AtomicBool,
    ) -> Result<PreRunCollaborationOutcome, CollaborationFailure>;

    fn after_local_commit(
        &self,
        context: &PostLocalCommitCollaborationContext,
    ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure>;

    fn validate_completion_intent_for_persistence(
        &self,
        intent: &RemoteCompletionIntent,
    ) -> Result<(), CollaborationFailure> {
        intent.validate()
    }
}

pub(crate) fn run_before_runtime(
    port: &dyn CollaborationPort,
    context: &PreRunCollaborationContext,
    cancel: &AtomicBool,
) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
    validate_context(
        context.mode,
        context.session.as_ref(),
        &context.local,
        context.remote.as_ref(),
    )?;
    if cancel.load(Ordering::Acquire) {
        return Err(CollaborationFailure::cancelled());
    }
    let outcome = match port.before_runtime(context, cancel) {
        Ok(outcome) => outcome,
        Err(failure) => {
            validate_failure(&failure)?;
            validate_serialized_contract("preRunFailure", &failure)?;
            return Err(failure);
        }
    };
    validate_claim_result(&outcome.claim)?;
    validate_serialized_contract("preRunOutcome", &outcome)?;
    Ok(outcome)
}

pub(crate) fn run_after_local_commit(
    port: &dyn CollaborationPort,
    context: &PostLocalCommitCollaborationContext,
) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
    validate_context(
        context.mode,
        context.session.as_ref(),
        &context.local,
        context.remote.as_ref(),
    )?;
    validate_identifier("runId", &context.run_id)?;
    match (context.mode, context.intent.as_ref()) {
        (CollaborationMode::SharedCollaborator, Some(intent)) => {
            intent.validate()?;
            if context.session.as_ref().is_none_or(|session| {
                session.workspace_id != intent.workspace_id
                    || session.actor != intent.actor
                    || session.access != intent.access
            }) || context.remote.as_ref().is_none_or(|remote| {
                remote.task_id != intent.task_id
                    || remote.task_path != intent.remote_task_path
                    || remote
                        .base_version
                        .checked_add(1)
                        .is_none_or(|version| version != intent.claimed_task_version)
            }) || context.local.task_path != intent.local_task_path
                || context.local.task_sha256 != intent.local_task_sha256
                || context.local.repository_id != intent.repository_id
                || context.run_id != intent.run_id
            {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    "Completion intent does not match the exact post-checkpoint binding",
                ));
            }
        }
        (CollaborationMode::Disabled | CollaborationMode::LocalOnly, None) => {}
        _ => {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "Only shared Collaborator mode may carry a completion intent",
            ))
        }
    }
    let outcome = match port.after_local_commit(context) {
        Ok(outcome) => outcome,
        Err(failure) => {
            validate_failure(&failure)?;
            validate_serialized_contract("postLocalCommitFailure", &failure)?;
            return Err(failure);
        }
    };
    validate_evidence_handoff_result(&outcome.evidence_handoff)?;
    match (
        &context.mode,
        &outcome.reconciliation,
        &outcome.evidence_handoff,
    ) {
        (
            CollaborationMode::Disabled | CollaborationMode::LocalOnly,
            reconciliation,
            EvidenceHandoffResult::NotRequired,
        ) if *reconciliation == local_reconciliation(context.mode) => {}
        (
            CollaborationMode::SharedCollaborator,
            ReconciliationState::Reconciled,
            EvidenceHandoffResult::Synchronized {
                remote_version,
                evidence_ids,
                handoff_id,
            },
        ) => {
            let intent = context
                .intent
                .as_ref()
                .expect("validated shared context has a completion intent");
            let exact_evidence =
                evidence_ids.as_slice() == [EvidenceReferenceId(intent.evidence_id.clone())];
            let exact_handoff =
                handoff_id.as_ref().map(|id| id.0.as_str()) == Some(intent.handoff_id.as_str());
            if *remote_version < intent.claimed_task_version || !exact_evidence || !exact_handoff {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    "Synchronized completion does not match the exact durable intent",
                ));
            }
        }
        (
            CollaborationMode::SharedCollaborator,
            ReconciliationState::RepairRequired,
            EvidenceHandoffResult::Partial { remote_version, .. },
        ) => {
            let claimed = context
                .intent
                .as_ref()
                .expect("validated shared context has a completion intent")
                .claimed_task_version;
            if remote_version.is_some_and(|version| version < claimed) {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    "Partial completion cannot regress the claimed remote task version",
                ));
            }
        }
        _ => {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "Post-commit result does not match the collaboration mode",
            ))
        }
    }
    validate_serialized_contract("postLocalCommitOutcome", &outcome)?;
    Ok(outcome)
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NoopCollaborationPort;

impl CollaborationPort for NoopCollaborationPort {
    fn before_runtime(
        &self,
        context: &PreRunCollaborationContext,
        _cancel: &AtomicBool,
    ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
        let _ = context;
        Ok(PreRunCollaborationOutcome {
            reconciliation: local_reconciliation(context.mode),
            claim: ClaimResult::NotRequired,
        })
    }

    fn after_local_commit(
        &self,
        context: &PostLocalCommitCollaborationContext,
    ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
        Ok(PostLocalCommitCollaborationOutcome {
            reconciliation: local_reconciliation(context.mode),
            evidence_handoff: EvidenceHandoffResult::NotRequired,
        })
    }
}

fn local_reconciliation(mode: CollaborationMode) -> ReconciliationState {
    match mode {
        CollaborationMode::Disabled => ReconciliationState::Disabled,
        CollaborationMode::LocalOnly => ReconciliationState::LocalOnly,
        CollaborationMode::Viewer | CollaborationMode::SharedCollaborator => {
            ReconciliationState::Disconnected
        }
    }
}

fn validate_context(
    mode: CollaborationMode,
    session: Option<&SanitizedSessionMetadata>,
    local: &LocalSourceBinding,
    remote: Option<&RemoteTaskBinding>,
) -> Result<(), CollaborationFailure> {
    for (field, value) in [
        ("taskPath", local.task_path.as_str()),
        ("taskSha256", local.task_sha256.as_str()),
        ("repositoryId", local.repository_id.as_str()),
        ("gitIndexSha256", local.git_index_sha256.as_str()),
        ("gitWorktreeSha256", local.git_worktree_sha256.as_str()),
    ] {
        validate_identifier(field, value)?;
    }
    if let Some(head) = local.git_head.as_deref() {
        validate_identifier("gitHead", head)?;
    }
    if matches!(
        mode,
        CollaborationMode::Disabled | CollaborationMode::LocalOnly
    ) && (session.is_some() || remote.is_some())
    {
        return Err(invalid_state(
            "local_mode_remote_state",
            "Disabled and local-only modes cannot carry session or remote task state",
        ));
    }
    if let Some(session) = session {
        for (field, value) in [
            ("workspaceId", session.workspace_id.as_str()),
            ("actor", session.actor.as_str()),
        ] {
            validate_identifier(field, value)?;
        }
        validate_origin("webOrigin", &session.web_origin)?;
        validate_origin("apiOrigin", &session.api_origin)?;
    }
    if let Some(remote) = remote {
        validate_identifier("remoteTaskId", &remote.task_id)?;
        validate_identifier("remoteTaskPath", &remote.task_path)?;
        if remote.base_version == 0 {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "remote baseVersion must be positive",
            ));
        }
    }
    match mode {
        CollaborationMode::Disabled | CollaborationMode::LocalOnly => {}
        CollaborationMode::Viewer => {
            let session = session.ok_or_else(|| {
                invalid_state(
                    "shared_mode_missing_state",
                    "Viewer mode requires a sanitized session",
                )
            })?;
            if session.access == CollaborationAccess::Collaborator {
                return Err(invalid_state(
                    "shared_mode_access_mismatch",
                    "Viewer mode cannot carry Collaborator access",
                ));
            }
            if remote.is_none() {
                return Err(invalid_state(
                    "shared_mode_missing_state",
                    "Viewer mode requires a remote task binding",
                ));
            }
        }
        CollaborationMode::SharedCollaborator => {
            let session = session.ok_or_else(|| {
                invalid_state(
                    "shared_mode_missing_state",
                    "Shared Collaborator mode requires a sanitized session",
                )
            })?;
            if session.access != CollaborationAccess::Collaborator || remote.is_none() {
                return Err(invalid_state(
                    "shared_mode_access_mismatch",
                    "Shared Collaborator mode requires Collaborator access and a remote task",
                ));
            }
        }
    }
    Ok(())
}

fn validate_shared_execution_binding(
    binding: &SharedExecutionBinding,
) -> Result<(), CollaborationFailure> {
    validate_context(
        if binding.session.access == CollaborationAccess::Collaborator {
            CollaborationMode::SharedCollaborator
        } else {
            CollaborationMode::Viewer
        },
        Some(&binding.session),
        &binding.local,
        Some(&binding.remote),
    )?;
    let mutation = &binding.expected_remote_mutation;
    if mutation.task_path != binding.remote.task_path
        || mutation.base_version != binding.remote.base_version
        || mutation.from_state != "ready"
        || mutation.to_state != "claimed"
        || mutation.owner != binding.session.actor
        || mutation.updated_by != binding.session.actor
    {
        return Err(invalid_state(
            "unsafe_collaboration_state",
            "Shared claim preview does not match its exact session and task binding",
        ));
    }
    Ok(())
}

fn validate_identifier(field: &str, value: &str) -> Result<(), CollaborationFailure> {
    let lower = value.to_ascii_lowercase();
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.chars().any(char::is_control)
        || FORBIDDEN_SECRET_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        || FORBIDDEN_CONTENT_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        || value.contains(['?', '#', '=', '{', '}', '[', ']'])
        || lower.contains("://")
    {
        return Err(invalid_state(
            "unsafe_collaboration_state",
            format!(
                "{field} is blank, oversized, contains control bytes, or resembles restricted material"
            ),
        ));
    }
    Ok(())
}

pub(crate) fn validate_portable_collaboration_metadata(
    field: &str,
    value: &str,
) -> Result<(), CollaborationFailure> {
    validate_identifier(field, value)
}

fn validate_message(field: &str, value: &str) -> Result<(), CollaborationFailure> {
    let lower = value.to_ascii_lowercase();
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.chars().any(char::is_control)
        || FORBIDDEN_SECRET_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        || FORBIDDEN_CONTENT_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        || value.contains(['?', '{', '}', '[', ']'])
        || lower.contains("://")
    {
        return Err(invalid_state(
            "unsafe_collaboration_state",
            format!("{field} contains content that is unsafe for collaboration state"),
        ));
    }
    Ok(())
}

fn validate_repair_hint(repair: &RepairHint) -> Result<(), CollaborationFailure> {
    let approved = [
        RepairHint::reconnect(),
        RepairHint::retry_sync(),
        RepairHint::reconcile_claimed_pre_spawn(),
        RepairHint::refresh_conflict(),
        RepairHint::inspect_repeated_conflict(),
    ];
    if !approved.contains(repair) {
        return Err(invalid_state(
            "unsafe_collaboration_state",
            "repair output must use an approved typed instruction",
        ));
    }
    Ok(())
}

fn validate_origin(field: &str, value: &str) -> Result<(), CollaborationFailure> {
    let lower = value.to_ascii_lowercase();
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.chars().any(char::is_control)
        || FORBIDDEN_SECRET_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        || FORBIDDEN_CONTENT_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
    {
        return Err(invalid_state(
            "unsafe_collaboration_origin",
            format!("{field} is not a sanitized origin"),
        ));
    }
    let Some((scheme, authority)) = value.split_once("://") else {
        return Err(invalid_state(
            "unsafe_collaboration_origin",
            format!("{field} must be a sanitized origin"),
        ));
    };
    if !matches!(scheme, "https" | "http")
        || authority.is_empty()
        || authority.contains(['?', '#', '@'])
        || authority.trim_end_matches('/').contains('/')
    {
        return Err(invalid_state(
            "unsafe_collaboration_origin",
            format!("{field} must contain only scheme and authority"),
        ));
    }
    Ok(())
}

fn invalid_state(code: &str, message: impl Into<String>) -> CollaborationFailure {
    CollaborationFailure {
        class: CollaborationFailureClass::InvalidInput,
        code: code.into(),
        message: message.into(),
        repair: None,
    }
}

fn typed_failure(
    class: CollaborationFailureClass,
    code: &str,
    message: &str,
    repair: Option<RepairHint>,
) -> CollaborationFailure {
    CollaborationFailure {
        class,
        code: code.into(),
        message: message.into(),
        repair,
    }
}

fn parse_local_reference(
    field: &str,
    prefix: &str,
    value: String,
) -> Result<String, CollaborationFailure> {
    let suffix = value.strip_prefix(prefix).unwrap_or_default();
    if suffix.len() != 32
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_state(
            "unsafe_collaboration_state",
            format!("{field} must be a locally minted non-capability reference"),
        ));
    }
    Ok(value)
}

fn validate_serialized_contract<T: Serialize>(
    field: &str,
    value: &T,
) -> Result<(), CollaborationFailure> {
    let value = serde_json::to_value(value).map_err(|_| {
        invalid_state(
            "unsafe_collaboration_state",
            format!("{field} could not be validated before serialization"),
        )
    })?;
    validate_json_value(field, &value)
}

fn validate_claim_result(claim: &ClaimResult) -> Result<(), CollaborationFailure> {
    if let ClaimResult::Stopped { repair, .. } = claim {
        validate_repair_hint(repair)?;
    }
    Ok(())
}

fn validate_evidence_handoff_result(
    handoff: &EvidenceHandoffResult,
) -> Result<(), CollaborationFailure> {
    match handoff {
        EvidenceHandoffResult::NotRequired => {}
        EvidenceHandoffResult::Synchronized {
            remote_version,
            evidence_ids,
            handoff_id,
        } => {
            if *remote_version == 0 || evidence_ids.is_empty() || handoff_id.is_none() {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    "Synchronized completion requires positive versions and evidence and handoff references",
                ));
            }
            for evidence_id in evidence_ids {
                evidence_id.validate()?;
            }
            if let Some(handoff_id) = handoff_id {
                handoff_id.validate()?;
            }
        }
        EvidenceHandoffResult::Partial {
            remote_version,
            missing_effects,
            repair,
        } => {
            if remote_version.is_some_and(|version| version == 0)
                || missing_effects.is_empty()
                || missing_effects.len() > MissingCollaborationEffect::ORDER.len()
                || missing_effects.windows(2).any(|pair| pair[0] >= pair[1])
                || missing_effects
                    .iter()
                    .any(|effect| !MissingCollaborationEffect::ORDER.contains(effect))
            {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    "Partial completion effects must be a nonempty canonical subset",
                ));
            }
            validate_repair_hint(repair)?;
        }
    }
    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), CollaborationFailure> {
    let hex = value.strip_prefix("sha256:").unwrap_or_default();
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_state(
            "unsafe_collaboration_state",
            format!("{field} must be a SHA-256 binding"),
        ));
    }
    Ok(())
}

fn validate_failure(failure: &CollaborationFailure) -> Result<(), CollaborationFailure> {
    if let Some(repair) = failure.repair() {
        validate_repair_hint(repair)?;
    }
    if failure.class != CollaborationFailureClass::InvalidInput {
        let approved = match failure.class {
            CollaborationFailureClass::InvalidInput => unreachable!(),
            CollaborationFailureClass::AccessDenied => CollaborationFailure::access_denied(),
            CollaborationFailureClass::SourceMismatch => CollaborationFailure::source_mismatch(),
            CollaborationFailureClass::VersionConflict => CollaborationFailure::version_conflict(),
            CollaborationFailureClass::TransportUnavailable => {
                CollaborationFailure::transport_unavailable()
            }
            CollaborationFailureClass::Timeout => CollaborationFailure::timeout(),
            CollaborationFailureClass::Cancelled => CollaborationFailure::cancelled(),
            CollaborationFailureClass::Protocol => CollaborationFailure::protocol(),
            CollaborationFailureClass::RepairRequired => CollaborationFailure::repair_required(),
        };
        if failure != &approved {
            return Err(invalid_state(
                "unsafe_collaboration_state",
                "adapter failures must use a fixed typed failure variant",
            ));
        }
    }
    Ok(())
}

fn validate_json_value(field: &str, value: &serde_json::Value) -> Result<(), CollaborationFailure> {
    match value {
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            Ok(())
        }
        serde_json::Value::String(value) => {
            if field.to_ascii_lowercase().ends_with("origin") {
                validate_origin(field, value)
            } else {
                validate_message(field, value)
            }
        }
        serde_json::Value::Array(values) => {
            if values.len() > 64 {
                return Err(invalid_state(
                    "unsafe_collaboration_state",
                    format!("{field} contains too many values"),
                ));
            }
            for value in values {
                validate_json_value(field, value)?;
            }
            Ok(())
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                let lower = key.to_ascii_lowercase();
                if FORBIDDEN_CONTENT_MARKERS
                    .iter()
                    .any(|marker| lower.contains(&marker.replace(' ', "")))
                    || lower.contains("body")
                    || lower.contains("url")
                    || lower.contains("header")
                {
                    return Err(invalid_state(
                        "unsafe_collaboration_state",
                        format!("{field} contains a forbidden field"),
                    ));
                }
                validate_json_value(key, value)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_binding() -> LocalSourceBinding {
        LocalSourceBinding {
            task_path: "tasks/issues/013-contract.md".into(),
            task_sha256: "sha256:task".into(),
            repository_id: "sha256:repository".into(),
            git_head: None,
            git_index_sha256: "sha256:index".into(),
            git_worktree_sha256: "sha256:worktree".into(),
            git_dirty: true,
        }
    }

    #[test]
    fn no_op_port_has_no_remote_effect_in_disabled_and_local_modes() {
        for mode in [CollaborationMode::Disabled, CollaborationMode::LocalOnly] {
            let before = run_before_runtime(
                &NoopCollaborationPort,
                &PreRunCollaborationContext {
                    mode,
                    session: None,
                    local: local_binding(),
                    remote: None,
                },
                &AtomicBool::new(false),
            )
            .unwrap();
            let after = run_after_local_commit(
                &NoopCollaborationPort,
                &PostLocalCommitCollaborationContext {
                    mode,
                    session: None,
                    local: local_binding(),
                    remote: None,
                    run_id: "0123456789abcdef0123456789abcdef".into(),
                    intent: None,
                },
            )
            .unwrap();

            assert_eq!(before.claim, ClaimResult::NotRequired);
            assert_eq!(after.evidence_handoff, EvidenceHandoffResult::NotRequired);
            assert_eq!(
                before.reconciliation,
                if mode == CollaborationMode::Disabled {
                    ReconciliationState::Disabled
                } else {
                    ReconciliationState::LocalOnly
                }
            );
        }
    }

    #[test]
    fn local_modes_reject_remote_or_session_state() {
        let error = run_before_runtime(
            &NoopCollaborationPort,
            &PreRunCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local: local_binding(),
                remote: Some(RemoteTaskBinding {
                    task_id: "BR-013".into(),
                    task_path: "tasks/BR-013.md".into(),
                    base_version: 4,
                }),
            },
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.code, "local_mode_remote_state");
    }

    #[test]
    fn sanitized_state_rejects_queries_headers_and_bearer_markers() {
        for unsafe_value in [
            "https://example.test/?edit=secret",
            "Authorization: secret",
            "Bearer secret",
            "workspace?k=secret",
        ] {
            let mut local = local_binding();
            local.repository_id = unsafe_value.into();
            let error = run_before_runtime(
                &NoopCollaborationPort,
                &PreRunCollaborationContext {
                    mode: CollaborationMode::LocalOnly,
                    session: None,
                    local,
                    remote: None,
                },
                &AtomicBool::new(false),
            )
            .unwrap_err();
            assert_eq!(error.code, "unsafe_collaboration_state");
            assert!(!error.message.contains(unsafe_value));
        }
    }

    struct UnsafeOutputPort;

    impl CollaborationPort for UnsafeOutputPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            _cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            Ok(PreRunCollaborationOutcome {
                reconciliation: ReconciliationState::Conflict,
                claim: ClaimResult::Stopped {
                    failure_class: CollaborationFailureClass::VersionConflict,
                    latest_remote_version: Some(9),
                    repair: RepairHint {
                        code: "retry".into(),
                        message: "raw provider payload opaque-value".into(),
                        next_action: "Reconnect".into(),
                    },
                },
            })
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            unreachable!()
        }
    }

    #[test]
    fn port_boundary_rejects_unsafe_adapter_outputs() {
        let error = run_before_runtime(
            &UnsafeOutputPort,
            &PreRunCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local: local_binding(),
                remote: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.code, "unsafe_collaboration_state");
        assert!(!error.message.contains("opaque-value"));
    }

    struct OpaqueOutputPort;

    impl CollaborationPort for OpaqueOutputPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            _cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            Ok(PreRunCollaborationOutcome {
                reconciliation: ReconciliationState::Conflict,
                claim: ClaimResult::Stopped {
                    failure_class: CollaborationFailureClass::VersionConflict,
                    latest_remote_version: Some(9),
                    repair: RepairHint {
                        code: "retry".into(),
                        message: "opaque-value-9f8e7d6c5b4a".into(),
                        next_action: "Reconnect".into(),
                    },
                },
            })
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            unreachable!()
        }
    }

    #[test]
    fn port_boundary_rejects_opaque_free_form_adapter_output() {
        let error = run_before_runtime(
            &OpaqueOutputPort,
            &PreRunCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local: local_binding(),
                remote: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.code, "unsafe_collaboration_state");
    }

    struct OpaqueFailurePort;

    impl CollaborationPort for OpaqueFailurePort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            _cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            Err(CollaborationFailure {
                class: CollaborationFailureClass::Protocol,
                code: "upstream".into(),
                message: "Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5".into(),
                repair: None,
            })
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            unreachable!()
        }
    }

    #[test]
    fn port_boundary_rejects_opaque_free_form_adapter_failure() {
        let error = run_before_runtime(
            &OpaqueFailurePort,
            &PreRunCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local: local_binding(),
                remote: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.code, "unsafe_collaboration_state");
        assert!(!error.message.contains("Q2Fw"));
    }

    struct OpaqueHandoffPort;

    impl CollaborationPort for OpaqueHandoffPort {
        fn before_runtime(
            &self,
            _context: &PreRunCollaborationContext,
            _cancel: &AtomicBool,
        ) -> Result<PreRunCollaborationOutcome, CollaborationFailure> {
            unreachable!()
        }

        fn after_local_commit(
            &self,
            _context: &PostLocalCommitCollaborationContext,
        ) -> Result<PostLocalCommitCollaborationOutcome, CollaborationFailure> {
            Ok(PostLocalCommitCollaborationOutcome {
                reconciliation: ReconciliationState::Reconciled,
                evidence_handoff: EvidenceHandoffResult::Synchronized {
                    remote_version: 7,
                    evidence_ids: vec![EvidenceReferenceId(
                        "Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5".into(),
                    )],
                    handoff_id: None,
                },
            })
        }
    }

    #[test]
    fn port_boundary_rejects_opaque_successful_handoff_identifiers() {
        let error = run_after_local_commit(
            &OpaqueHandoffPort,
            &PostLocalCommitCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local: local_binding(),
                remote: None,
                run_id: "0123456789abcdef0123456789abcdef".into(),
                intent: None,
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "unsafe_collaboration_state");
    }

    #[test]
    fn shared_session_rejects_an_untyped_opaque_session_handle() {
        let error = LocalSessionHandle::parse("Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5").unwrap_err();

        assert_eq!(error.code, "unsafe_collaboration_state");
    }

    #[test]
    fn opaque_capability_material_is_non_serializable_and_debug_redacted() {
        let material = CapabilityMaterial::new("Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5").unwrap();
        assert_eq!(format!("{material:?}"), "CapabilityMaterial([REDACTED])");
        assert_eq!(material.0.len(), "Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5".len());
    }

    #[test]
    fn production_policy_defaults_to_explicit_local_only_mode() {
        assert_eq!(
            ControllerCollaborationPolicy::default().mode,
            CollaborationMode::LocalOnly
        );
    }

    #[test]
    fn shared_execution_binding_is_exact_sanitized_and_capability_free() {
        let session = SanitizedSessionMetadata::new(
            LocalSessionHandle::parse(format!("local-session-{}", "a".repeat(32))).unwrap(),
            "workspace-1".into(),
            "https://app.example.test".into(),
            "https://api.example.test".into(),
            CollaborationAccess::Collaborator,
            "codex-pax".into(),
        )
        .unwrap();
        let binding = SharedExecutionBinding::new(
            session,
            local_binding(),
            RemoteTaskBinding {
                task_id: "BR-016".into(),
                task_path: "tasks/BR-016.md".into(),
                base_version: 7,
            },
        )
        .unwrap();

        assert_eq!(binding.expected_remote_mutation.base_version, 7);
        assert_eq!(binding.expected_remote_mutation.from_state, "ready");
        assert_eq!(binding.expected_remote_mutation.to_state, "claimed");
        assert_eq!(binding.expected_remote_mutation.owner, "codex-pax");
        let serialized = serde_json::to_string(&binding).unwrap();
        assert!(serialized.contains("\"baseVersion\":7"));
        for forbidden in ["Bearer ", "?edit=", "authorization", "secret-value"] {
            assert!(!serialized
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase()));
        }
    }

    #[test]
    fn cancellation_preempts_the_collaboration_port() {
        let cancel = AtomicBool::new(true);
        let error = run_before_runtime(
            &NoopCollaborationPort,
            &PreRunCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local: local_binding(),
                remote: None,
            },
            &cancel,
        )
        .unwrap_err();

        assert_eq!(error.class, CollaborationFailureClass::Cancelled);
        assert_eq!(error.code, "cancelled");
    }

    #[test]
    fn rejects_arbitrary_capability_urls_not_just_known_query_names() {
        let mut local = local_binding();
        local.task_path = "https://example.test/workspaces/w?opaque=credential".into();
        let error = run_before_runtime(
            &NoopCollaborationPort,
            &PreRunCollaborationContext {
                mode: CollaborationMode::LocalOnly,
                session: None,
                local,
                remote: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap_err();

        assert_eq!(error.code, "unsafe_collaboration_state");
    }

    #[test]
    fn core_contract_has_no_provider_framework_or_network_dependency() {
        let source = include_str!("collaboration.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production contract source")
            .to_ascii_lowercase();
        for forbidden in ["mdsync", "reqwest", "tauri::", "node:", "http client"] {
            assert!(!source.contains(forbidden), "{forbidden} leaked into core");
        }
    }
}
