use crate::collaboration::{
    validate_portable_collaboration_metadata, CollaborationAccess, LocalSourceBinding,
    MissingCollaborationEffect, RemoteCompletionIntent, RemoteTaskBinding,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
};
use uuid::Uuid;

const PROTOCOL_VERSION: &str = "1.0.0";
const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_MARKDOWN: &str = "text/markdown; charset=utf-8";
const ENVELOPE_OPEN: &str = "<build-right-envelope>";
const ENVELOPE_CLOSE: &str = "</build-right-envelope>";
const MAX_FILE_COUNT: usize = 64;
const MAX_WORKSPACE_BYTES: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolverTaskStatus {
    Ready,
    Active,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectionInput {
    pub(crate) workspace_id: String,
    pub(crate) actor: String,
    pub(crate) task_id: String,
    pub(crate) title: String,
    pub(crate) status: ResolverTaskStatus,
    pub(crate) requirement_basis: Vec<String>,
    pub(crate) local: LocalSourceBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkspaceFile {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) content_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Ha2haWorkspace {
    pub(crate) workspace_id: String,
    pub(crate) task_path: String,
    pub(crate) local: LocalSourceBinding,
    pub(crate) files: Vec<WorkspaceFile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublishPlan {
    pub(crate) workspace: Ha2haWorkspace,
    pub(crate) remote_baseline: Vec<RemoteWorkspaceFile>,
    pub(crate) writes: Vec<WorkspaceFile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfirmedPublish {
    pub(crate) workspace: Ha2haWorkspace,
    pub(crate) remote_baseline: Vec<RemoteWorkspaceFile>,
    pub(crate) writes: Vec<WorkspaceFile>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishPreview {
    pub(crate) workspace_id: String,
    pub(crate) task_path: String,
    pub(crate) local: LocalSourceBinding,
    pub(crate) files: Vec<WorkspaceFile>,
    pub(crate) expected_effects: Vec<String>,
    pub(crate) explicit_confirmation_required: bool,
    pub(crate) preview_token: String,
}

#[derive(Clone, Debug)]
struct StoredPublishPlan {
    project_key: String,
    session_id: String,
    plan: PublishPlan,
}

#[derive(Debug, Default)]
pub(crate) struct PublishPlanStore {
    plans: Mutex<HashMap<String, StoredPublishPlan>>,
}

impl PublishPlanStore {
    pub(crate) fn issue(
        &self,
        project_key: &str,
        session_id: &str,
        plan: PublishPlan,
    ) -> Result<PublishPreview, EnvelopeError> {
        validate_publish_plan(&plan)?;
        let mut plans = self.plans.lock().map_err(|_| EnvelopeError::internal())?;
        // Only one publish confirmation may exist in this desktop process.
        // Project/session switches invalidate every prior preview and keep the
        // in-memory confirmation surface strictly bounded.
        plans.clear();
        let digest = publish_plan_digest(&plan)?;
        let token = format!("publish-{}-{}", &digest[..16], Uuid::new_v4().simple());
        plans.insert(
            token.clone(),
            StoredPublishPlan {
                project_key: project_key.into(),
                session_id: session_id.into(),
                plan: plan.clone(),
            },
        );
        Ok(PublishPreview {
            workspace_id: plan.workspace.workspace_id,
            task_path: plan.workspace.task_path,
            local: plan.workspace.local,
            files: plan.workspace.files,
            expected_effects: vec![
                "Create one complete HA2HA v1 workspace projection".into(),
                "Publish only the resolver-selected Build Right task".into(),
                "Perform no claim and start no provider runtime".into(),
            ],
            explicit_confirmation_required: true,
            preview_token: token,
        })
    }

    pub(crate) fn consume(
        &self,
        project_key: &str,
        session_id: &str,
        token: &str,
        confirmed: bool,
    ) -> Result<ConfirmedPublish, EnvelopeError> {
        if !confirmed {
            return Err(EnvelopeError::confirmation_required());
        }
        let mut plans = self.plans.lock().map_err(|_| EnvelopeError::internal())?;
        let plan = plans
            .remove(token)
            .ok_or_else(EnvelopeError::stale_preview)?;
        if plan.project_key != project_key || plan.session_id != session_id {
            return Err(EnvelopeError::stale_preview());
        }
        validate_publish_plan(&plan.plan)?;
        Ok(ConfirmedPublish {
            workspace: plan.plan.workspace,
            remote_baseline: plan.plan.remote_baseline,
            writes: plan.plan.writes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RemoteWorkspaceFile {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct JoinResult {
    pub(crate) workspace_id: String,
    pub(crate) actor: String,
    pub(crate) access: CollaborationAccess,
    pub(crate) task: RemoteTaskBinding,
    pub(crate) local: LocalSourceBinding,
    pub(crate) reconciled: bool,
    pub(crate) executable: bool,
    pub(crate) inspection_only: bool,
    pub(crate) repair: Option<EnvelopeRepair>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskClaimWrite {
    pub(crate) task_id: String,
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) content_type: String,
    pub(crate) actor: String,
    pub(crate) base_version: u64,
    pub(crate) expected_post_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PostRunEffectWrite {
    pub(crate) effect: MissingCollaborationEffect,
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) content_type: String,
    pub(crate) base_version: Option<u64>,
    pub(crate) expected_post_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PostRunReconciliationPlan {
    pub(crate) applied_effects: Vec<MissingCollaborationEffect>,
    pub(crate) writes: Vec<PostRunEffectWrite>,
    pub(crate) current_task_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EnvelopeErrorClass {
    InvalidInput,
    AccessDenied,
    SourceMismatch,
    Protocol,
    RepairRequired,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnvelopeRepair {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) next_action: String,
}

impl EnvelopeRepair {
    pub(crate) fn new(code: &str, message: &str, next_action: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            next_action: next_action.into(),
        }
    }

    pub(crate) fn partial_publish() -> Self {
        Self::new(
            "repair-partial-publish",
            "Some validated workspace files were created before transport failure",
            "Inspect the recorded paths, remove or complete the partial workspace explicitly, then generate a fresh publish preview",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnvelopeError {
    pub(crate) class: EnvelopeErrorClass,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) repair: Option<EnvelopeRepair>,
}

impl EnvelopeError {
    fn new(
        class: EnvelopeErrorClass,
        code: &str,
        message: &str,
        repair: Option<EnvelopeRepair>,
    ) -> Self {
        Self {
            class,
            code: code.into(),
            message: message.into(),
            repair,
        }
    }

    pub(crate) fn invalid_input(message: &str) -> Self {
        Self::new(
            EnvelopeErrorClass::InvalidInput,
            "invalid_envelope_input",
            message,
            None,
        )
    }

    pub(crate) fn access_denied() -> Self {
        Self::new(
            EnvelopeErrorClass::AccessDenied,
            "publish_access_denied",
            "Collaborator access is required to publish an execution envelope",
            Some(EnvelopeRepair::new(
                "reconnect-collaborator",
                "The current session is read-only",
                "Reconnect with collaborator access or continue inspection-only",
            )),
        )
    }

    pub(crate) fn duplicate_envelope() -> Self {
        Self::new(
            EnvelopeErrorClass::RepairRequired,
            "duplicate_envelope",
            "The workspace contains duplicate or ambiguous Build Right envelopes",
            Some(EnvelopeRepair::new(
                "remove-duplicate-envelope",
                "Exactly one source-bound envelope is supported",
                "Remove duplicate envelope tasks, then reconnect and inspect again",
            )),
        )
    }

    pub(crate) fn missing_task() -> Self {
        Self::new(
            EnvelopeErrorClass::RepairRequired,
            "missing_envelope_task",
            "The workspace does not contain a Build Right execution envelope",
            Some(EnvelopeRepair::new(
                "publish-envelope",
                "A single source-bound task envelope is required",
                "Publish the resolver-selected task, then reconnect",
            )),
        )
    }

    pub(crate) fn unsupported_remote_state() -> Self {
        Self::new(
            EnvelopeErrorClass::Protocol,
            "unsupported_remote_state",
            "The remote envelope state is not supported before claim integration",
            Some(EnvelopeRepair::new(
                "inspect-remote-state",
                "Task 015 joins only an unclaimed ready envelope",
                "Inspect the remote task and reconcile its state before execution",
            )),
        )
    }

    pub(crate) fn source_mismatch() -> Self {
        Self::new(
            EnvelopeErrorClass::SourceMismatch,
            "local_source_mismatch",
            "The remote envelope no longer matches the resolver-selected local task",
            Some(EnvelopeRepair::new(
                "republish-current-source",
                "Local task or Git binding changed",
                "Review local changes and publish a newly confirmed envelope",
            )),
        )
    }

    pub(crate) fn workspace_mismatch() -> Self {
        Self::new(
            EnvelopeErrorClass::Protocol,
            "workspace_id_mismatch",
            "The manifest or remote files do not match the connected workspace",
            Some(EnvelopeRepair::new(
                "reconnect-workspace",
                "Remote workspace identity is inconsistent",
                "Reconnect using the intended workspace and inspect its manifest",
            )),
        )
    }

    pub(crate) fn invalid_remote_shape(message: &str) -> Self {
        Self::new(
            EnvelopeErrorClass::Protocol,
            "invalid_remote_workspace",
            message,
            Some(EnvelopeRepair::new(
                "repair-workspace",
                "The remote workspace is not a supported HA2HA v1 envelope",
                "Repair the listed workspace files, then reconnect",
            )),
        )
    }

    pub(crate) fn incompatible_reconciliation() -> Self {
        Self::new(
            EnvelopeErrorClass::RepairRequired,
            "incompatible_reconciliation_divergence",
            "Remote collaboration state diverged from the durable local completion intent",
            Some(EnvelopeRepair::new(
                "inspect-reconciliation-divergence",
                "Repair can apply only missing compatible writes",
                "Inspect the exact remote file versions and resolve the divergence before retrying repair",
            )),
        )
    }

    fn confirmation_required() -> Self {
        Self::new(
            EnvelopeErrorClass::InvalidInput,
            "confirmation_required",
            "Publishing requires explicit confirmation of the exact preview",
            None,
        )
    }

    fn stale_preview() -> Self {
        Self::new(
            EnvelopeErrorClass::SourceMismatch,
            "stale_publish_preview",
            "The publish preview is missing, consumed, or bound to another session",
            Some(EnvelopeRepair::new(
                "preview-again",
                "Publish confirmation is one-use and source-bound",
                "Generate and review a fresh publish preview",
            )),
        )
    }

    fn internal() -> Self {
        Self::new(
            EnvelopeErrorClass::Internal,
            "envelope_internal_error",
            "The envelope operation failed safely",
            None,
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceManifest {
    capabilities: Vec<String>,
    conflict_policy: String,
    paths: ManifestPaths,
    protocol: String,
    protocol_version: String,
    routes: ManifestRoutes,
    schema_versions: BTreeMap<String, String>,
    title: String,
    workspace_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestPaths {
    decisions: String,
    evidence: String,
    logs: String,
    manifest_markdown: String,
    participants: String,
    status: String,
    tasks: String,
    workspace_manifest: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestRoutes {
    events: Option<String>,
    file: String,
    file_version: Option<String>,
    file_versions: Option<String>,
    raw_events: Option<String>,
    raw_file: String,
    raw_listing: String,
    tree: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskFrontmatter {
    id: String,
    title: String,
    state: String,
    owner: Option<String>,
    updated_by: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParticipantFrontmatter {
    id: String,
    human: Option<String>,
    agent_runtime: Option<String>,
    can_edit: bool,
    last_seen: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceFrontmatter {
    id: String,
    task: String,
    target: EvidenceTarget,
    kind: String,
    result: String,
    actor: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceTarget {
    workspace_id: String,
    path: String,
    version: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildRightEnvelope {
    version: u8,
    source_path: String,
    source_sha256: String,
    repository_id: String,
    git_head: Option<String>,
    git_dirty: bool,
    requirement_basis: Vec<String>,
}

pub(crate) fn project_workspace(input: ProjectionInput) -> Result<Ha2haWorkspace, EnvelopeError> {
    validate_component("workspaceId", &input.workspace_id)?;
    validate_component("actor", &input.actor)?;
    validate_component("taskId", &input.task_id)?;
    validate_text("title", &input.title, 240)?;
    if !matches!(
        input.status,
        ResolverTaskStatus::Ready | ResolverTaskStatus::Active
    ) {
        return Err(EnvelopeError::invalid_input(
            "Only resolver-selected ready or active tasks can be projected",
        ));
    }
    if input.requirement_basis.is_empty() {
        return Err(EnvelopeError::invalid_input(
            "The selected task must include requirement basis",
        ));
    }
    for value in &input.requirement_basis {
        validate_text("requirementBasis", value, 512)?;
    }
    validate_local_binding(&input.local)?;

    let task_path = format!("tasks/{}.md", input.task_id);
    let evidence_path = format!("evidence/{}/source-binding.md", input.task_id);
    let participant_path = format!("participants/{}.md", input.actor);
    let decision_path = "decisions/build-right-local-authority.md".to_string();
    let manifest = WorkspaceManifest {
        capabilities: vec!["raw-read".into(), "file-write".into()],
        conflict_policy: "baseVersion-required".into(),
        paths: ManifestPaths {
            decisions: "decisions/".into(),
            evidence: "evidence/".into(),
            logs: "logs/".into(),
            manifest_markdown: "HA2HA.md".into(),
            participants: "participants/".into(),
            status: "STATUS.md".into(),
            tasks: "tasks/".into(),
            workspace_manifest: ".ha2ha/workspace.json".into(),
        },
        protocol: "ha2ha".into(),
        protocol_version: PROTOCOL_VERSION.into(),
        routes: ManifestRoutes {
            events: None,
            file: format!("/api/workspaces/{}/files", input.workspace_id),
            file_version: None,
            file_versions: None,
            raw_events: None,
            raw_file: format!("/w/{}/raw/{{path}}", input.workspace_id),
            raw_listing: format!("/w/{}/raw", input.workspace_id),
            tree: format!("/api/workspaces/{}/tree", input.workspace_id),
        },
        schema_versions: BTreeMap::from([
            ("evidence".into(), PROTOCOL_VERSION.into()),
            ("task".into(), PROTOCOL_VERSION.into()),
            ("workspace".into(), PROTOCOL_VERSION.into()),
        ]),
        title: format!("Build Right {}", input.task_id),
        workspace_id: input.workspace_id.clone(),
    };
    let envelope = BuildRightEnvelope {
        version: 1,
        source_path: input.local.task_path.clone(),
        source_sha256: input.local.task_sha256.clone(),
        repository_id: input.local.repository_id.clone(),
        git_head: input.local.git_head.clone(),
        git_dirty: input.local.git_dirty,
        requirement_basis: input.requirement_basis,
    };
    let envelope_json = serde_json::to_string(&envelope).map_err(|_| EnvelopeError::internal())?;
    let task = format!(
        "---\nid: {}\ntitle: {}\nstate: ready\nowner: null\nupdated_by: {}\nevidence:\n  - {}\n---\n\n## Goal\n\n{}\n\nDecision: `{}` keeps Build Right repository truth authoritative.\n\n{}\n{}\n{}\n",
        yaml_scalar(&input.task_id),
        yaml_scalar(&input.title),
        yaml_scalar(&input.actor),
        evidence_path,
        input.title,
        decision_path,
        ENVELOPE_OPEN,
        envelope_json,
        ENVELOPE_CLOSE
    );
    let files = vec![
        WorkspaceFile {
            path: "HA2HA.md".into(),
            content: format!(
                "# Build Right HA2HA Workspace\n\nPurpose: inspect one source-bound Build Right execution envelope for {}.\n\nBuild Right repository truth remains authoritative. Publishing does not claim or execute the task.\n",
                input.task_id
            ),
            content_type: CONTENT_TYPE_MARKDOWN.into(),
        },
        WorkspaceFile {
            path: "STATUS.md".into(),
            content: format!(
                "# Status\n\n- {} is ready for source-bound inspection; no claim or execution has started.\n",
                input.task_id
            ),
            content_type: CONTENT_TYPE_MARKDOWN.into(),
        },
        WorkspaceFile {
            path: participant_path,
            content: format!(
                "---\nid: {}\nhuman: Build Right user\nagent_runtime: build-right-studio\ncan_edit: true\nlast_seen: 1970-01-01T00:00:00Z\n---\n\n## Current Focus\n\n- {}\n",
                yaml_scalar(&input.actor),
                input.task_id
            ),
            content_type: CONTENT_TYPE_MARKDOWN.into(),
        },
        WorkspaceFile {
            path: task_path.clone(),
            content: task,
            content_type: CONTENT_TYPE_MARKDOWN.into(),
        },
        WorkspaceFile {
            path: evidence_path.clone(),
            content: format!(
                "---\nid: {}\ntask: {}\ntarget:\n  workspaceId: {}\n  path: {}\n  version: 1\nkind: source-binding\nresult: unknown\nactor: {}\ncreated_at: 1970-01-01T00:00:00Z\n---\n\nInitial deterministic source-binding evidence. No provider output is included.\n",
                yaml_scalar(&format!("ev-{}-source-binding", input.task_id)),
                yaml_scalar(&input.task_id),
                yaml_scalar(&input.workspace_id),
                yaml_scalar(&task_path),
                yaml_scalar(&input.actor),
            ),
            content_type: CONTENT_TYPE_MARKDOWN.into(),
        },
        WorkspaceFile {
            path: decision_path,
            content: "# Build Right Local Authority\n\nAccepted: repository Markdown, Git, resolver gates, and verification remain authoritative. This HA2HA workspace is a portable inspection envelope, not a backlog mirror.\n".into(),
            content_type: CONTENT_TYPE_MARKDOWN.into(),
        },
        // The manifest is deliberately last so a partial multi-file transport
        // failure never presents the preceding writes as a joinable workspace.
        WorkspaceFile {
            path: ".ha2ha/workspace.json".into(),
            content: format!(
                "{}\n",
                serde_json::to_string_pretty(&manifest)
                    .map_err(|_| EnvelopeError::internal())?
            ),
            content_type: CONTENT_TYPE_JSON.into(),
        },
    ];
    let workspace = Ha2haWorkspace {
        workspace_id: input.workspace_id,
        task_path,
        local: input.local,
        files,
    };
    validate_workspace(&workspace)?;
    Ok(workspace)
}

pub(crate) fn project_publish_plan(
    input: ProjectionInput,
    mut remote_baseline: Vec<RemoteWorkspaceFile>,
) -> Result<PublishPlan, EnvelopeError> {
    let actor = input.actor.clone();
    let projection = project_workspace(input)?;
    remote_baseline.sort_by(|left, right| left.path.cmp(&right.path));
    let required_scaffold = [
        ".ha2ha/workspace.json",
        "HA2HA.md",
        "STATUS.md",
        projection
            .files
            .iter()
            .find(|file| file.path.starts_with("participants/"))
            .map(|file| file.path.as_str())
            .ok_or_else(|| {
                EnvelopeError::invalid_remote_shape("Projected participant is missing")
            })?,
    ];
    if remote_baseline.len() != required_scaffold.len()
        || required_scaffold.iter().any(|required| {
            remote_baseline
                .iter()
                .filter(|file| file.path == **required)
                .count()
                != 1
        })
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Publishing requires the deterministic MDSync HA2HA scaffold",
        ));
    }
    if remote_baseline
        .iter()
        .any(|file| file.version == 0 || file.content.len() > MAX_WORKSPACE_BYTES)
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Scaffold versions or contents are invalid",
        ));
    }
    validate_mdsync_scaffold(&remote_baseline, &projection.workspace_id, &actor)?;
    let mut writes = projection
        .files
        .iter()
        .filter(|file| {
            file.path.starts_with("decisions/")
                || file.path.starts_with("evidence/")
                || file.path.starts_with("tasks/")
        })
        .cloned()
        .collect::<Vec<_>>();
    // A partial publication remains non-joinable: the only file carrying the
    // envelope marker is created after its decision and evidence references.
    writes.sort_by_key(|file| {
        if file.path.starts_with("decisions/") {
            0
        } else if file.path.starts_with("evidence/") {
            1
        } else {
            2
        }
    });
    let mut files = remote_baseline
        .iter()
        .map(|file| WorkspaceFile {
            path: file.path.clone(),
            content: file.content.clone(),
            content_type: if file.path.ends_with(".json") {
                "application/json; charset=utf-8".into()
            } else {
                CONTENT_TYPE_MARKDOWN.into()
            },
        })
        .collect::<Vec<_>>();
    files.extend(writes.clone());
    let workspace = Ha2haWorkspace {
        workspace_id: projection.workspace_id,
        task_path: projection.task_path,
        local: projection.local,
        files,
    };
    let plan = PublishPlan {
        workspace,
        remote_baseline,
        writes,
    };
    validate_publish_plan(&plan)?;
    Ok(plan)
}

fn validate_mdsync_scaffold(
    baseline: &[RemoteWorkspaceFile],
    workspace_id: &str,
    actor: &str,
) -> Result<(), EnvelopeError> {
    let files = baseline
        .iter()
        .map(|file| (file.path.as_str(), file.content.as_str()))
        .collect::<BTreeMap<_, _>>();
    let manifest_content = files.get(".ha2ha/workspace.json").ok_or_else(|| {
        EnvelopeError::invalid_remote_shape("MDSync scaffold manifest is missing")
    })?;
    let manifest: WorkspaceManifest = serde_json::from_str(manifest_content)
        .map_err(|_| EnvelopeError::invalid_remote_shape("MDSync scaffold manifest is invalid"))?;
    validate_manifest(&manifest, workspace_id)?;
    validate_text("workspaceTitle", &manifest.title, 240)?;
    let expected_manifest_markdown = format!(
        "# {}\n\nThis workspace follows the HA2HA {} core workspace convention.\n\nMutating writes require an explicit actor and the current `baseVersion`. Capability URLs are never stored in workspace content.\n",
        manifest.title, PROTOCOL_VERSION
    );
    let expected_status = "# Status\n\n## Current work\n\n\n";
    let expected_participant = format!(
        "---\nid: {actor}\ncan_edit: true\n---\n\n## Current Focus\n\n- No task selected\n"
    );
    if files.get("HA2HA.md").copied() != Some(expected_manifest_markdown.as_str())
        || files.get("STATUS.md").copied() != Some(expected_status)
        || files
            .get(format!("participants/{actor}.md").as_str())
            .copied()
            != Some(expected_participant.as_str())
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Remote files do not match the deterministic empty-task MDSync HA2HA scaffold",
        ));
    }
    for (path, content) in &files {
        let lower = content.to_ascii_lowercase();
        if content.contains(ENVELOPE_OPEN)
            || content.contains(ENVELOPE_CLOSE)
            || [
                "authorization:",
                "bearer ",
                "access_token",
                "refresh_token",
                "provider_payload",
                "capability_url",
                "?edit=",
                "?k=",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            return Err(EnvelopeError::invalid_remote_shape(&format!(
                "Restricted material is present in scaffold file {path}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_publish_plan(plan: &PublishPlan) -> Result<(), EnvelopeError> {
    validate_workspace(&plan.workspace)?;
    if plan.writes.len() != 3
        || plan
            .writes
            .last()
            .is_none_or(|file| !file.path.starts_with("tasks/"))
        || plan
            .writes
            .iter()
            .any(|file| file.path == ".ha2ha/workspace.json")
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Publish writes must create decision, evidence, then task and never overwrite the manifest",
        ));
    }
    let baseline_paths = plan
        .remote_baseline
        .iter()
        .map(|file| file.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if baseline_paths.len() != plan.remote_baseline.len()
        || plan
            .writes
            .iter()
            .any(|file| baseline_paths.contains(file.path.as_str()))
    {
        return Err(EnvelopeError::duplicate_envelope());
    }
    Ok(())
}

pub(crate) fn validate_workspace(workspace: &Ha2haWorkspace) -> Result<(), EnvelopeError> {
    validate_component("workspaceId", &workspace.workspace_id)?;
    validate_local_binding(&workspace.local)?;
    if workspace.files.len() != 7 {
        return Err(EnvelopeError::invalid_remote_shape(
            "A Build Right projection must contain exactly seven minimal workspace files",
        ));
    }
    let total = workspace
        .files
        .iter()
        .try_fold(0_usize, |total, file| total.checked_add(file.content.len()))
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("Workspace size overflow"))?;
    if total > MAX_WORKSPACE_BYTES {
        return Err(EnvelopeError::invalid_remote_shape(
            "Workspace projection exceeds the bounded size",
        ));
    }
    let map = workspace
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    if map.len() != workspace.files.len() {
        return Err(EnvelopeError::duplicate_envelope());
    }
    for required in [
        ".ha2ha/workspace.json",
        "HA2HA.md",
        "STATUS.md",
        "decisions/build-right-local-authority.md",
    ] {
        if !map.contains_key(required) {
            return Err(EnvelopeError::invalid_remote_shape(
                "The workspace is missing a canonical file",
            ));
        }
    }
    for file in &workspace.files {
        validate_workspace_path(&file.path)?;
        validate_bounded_text("contentType", &file.content_type, 256)?;
        if file.content.contains('\0') {
            return Err(EnvelopeError::invalid_remote_shape(
                "Workspace files cannot contain null bytes",
            ));
        }
        validate_published_content(&file.path, &file.content)?;
    }
    let manifest: WorkspaceManifest = serde_json::from_str(
        &map.get(".ha2ha/workspace.json")
            .ok_or_else(EnvelopeError::missing_task)?
            .content,
    )
    .map_err(|_| EnvelopeError::invalid_remote_shape("Manifest JSON is invalid"))?;
    validate_manifest(&manifest, &workspace.workspace_id)?;
    let task_files = workspace
        .files
        .iter()
        .filter(|file| file.path.starts_with("tasks/") && file.path.ends_with(".md"))
        .collect::<Vec<_>>();
    if task_files.len() != 1 {
        return Err(if task_files.is_empty() {
            EnvelopeError::missing_task()
        } else {
            EnvelopeError::duplicate_envelope()
        });
    }
    if task_files[0].path != workspace.task_path {
        return Err(EnvelopeError::source_mismatch());
    }
    let (task, envelope) = parse_task(&task_files[0].content)?;
    validate_component("taskId", &task.id)?;
    validate_component("updatedBy", &task.updated_by)?;
    validate_text("taskTitle", &task.title, 240)?;
    for requirement in &envelope.requirement_basis {
        validate_text("requirementBasis", requirement, 512)?;
    }
    if task.state != "ready" {
        return Err(EnvelopeError::unsupported_remote_state());
    }
    if task.owner.is_some() {
        return Err(EnvelopeError::invalid_remote_shape(
            "A published Task 015 envelope must be unclaimed",
        ));
    }
    let evidence_path = task
        .evidence
        .first()
        .filter(|_| task.evidence.len() == 1)
        .ok_or_else(|| {
            EnvelopeError::invalid_remote_shape(
                "The task must reference one initial source-binding evidence file",
            )
        })?;
    let evidence_file = map.get(evidence_path.as_str()).ok_or_else(|| {
        EnvelopeError::invalid_remote_shape("Referenced source-binding evidence is missing")
    })?;
    let evidence: EvidenceFrontmatter = parse_frontmatter(&evidence_file.content)?;
    if evidence.id.is_empty()
        || evidence.task != task.id
        || evidence.target.workspace_id != workspace.workspace_id
        || evidence.target.path != workspace.task_path
        || evidence.target.version != 1
        || evidence.kind != "source-binding"
        || evidence.result != "unknown"
        || evidence.actor != task.updated_by
        || evidence.created_at.is_empty()
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Initial evidence does not bind the projected task",
        ));
    }
    let participants = workspace
        .files
        .iter()
        .filter(|file| file.path.starts_with("participants/") && file.path.ends_with(".md"))
        .collect::<Vec<_>>();
    if participants.len() != 1 {
        return Err(EnvelopeError::invalid_remote_shape(
            "The minimal workspace must contain one participant",
        ));
    }
    let participant: ParticipantFrontmatter = parse_frontmatter(&participants[0].content)?;
    if participant.id != task.updated_by
        || !participant.can_edit
        || participant.human.as_deref().is_some_and(str::is_empty)
        || participant
            .agent_runtime
            .as_deref()
            .is_some_and(str::is_empty)
        || participant.last_seen.as_deref().is_some_and(str::is_empty)
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Participant metadata does not match the publishing actor",
        ));
    }
    if envelope
        != (BuildRightEnvelope {
            version: 1,
            source_path: workspace.local.task_path.clone(),
            source_sha256: workspace.local.task_sha256.clone(),
            repository_id: workspace.local.repository_id.clone(),
            git_head: workspace.local.git_head.clone(),
            git_dirty: workspace.local.git_dirty,
            requirement_basis: envelope.requirement_basis.clone(),
        })
        || envelope.requirement_basis.is_empty()
    {
        return Err(EnvelopeError::source_mismatch());
    }
    let serialized = serde_json::to_string(&envelope).map_err(|_| EnvelopeError::internal())?;
    let lower = serialized.to_ascii_lowercase();
    if [
        "authorization",
        "bearer ",
        "access_token",
        "refresh_token",
        "provider_payload",
        "capability_url",
        "\"token\"",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "The Build Right envelope contains restricted capability or provider material",
        ));
    }
    Ok(())
}

pub(crate) fn join_workspace(
    workspace_id: &str,
    actor: &str,
    access: CollaborationAccess,
    local: LocalSourceBinding,
    files: Vec<RemoteWorkspaceFile>,
) -> Result<JoinResult, EnvelopeError> {
    validate_component("workspaceId", workspace_id)?;
    validate_component("actor", actor)?;
    if files.is_empty() || files.len() > MAX_FILE_COUNT {
        return Err(EnvelopeError::invalid_remote_shape(
            "Remote workspace file count is missing or oversized",
        ));
    }
    let mut seen = BTreeMap::new();
    for file in files {
        validate_workspace_path(&file.path)?;
        if file.version == 0 || seen.insert(file.path.clone(), file).is_some() {
            return Err(EnvelopeError::duplicate_envelope());
        }
    }
    let manifest_file = seen
        .get(".ha2ha/workspace.json")
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("Workspace manifest is missing"))?;
    let manifest: WorkspaceManifest = serde_json::from_str(&manifest_file.content)
        .map_err(|_| EnvelopeError::invalid_remote_shape("Manifest JSON is invalid"))?;
    validate_manifest(&manifest, workspace_id)?;
    let envelope_tasks = seen
        .values()
        .filter(|file| file.path.starts_with("tasks/") && file.content.contains(ENVELOPE_OPEN))
        .collect::<Vec<_>>();
    let task_file = match envelope_tasks.as_slice() {
        [] => return Err(EnvelopeError::missing_task()),
        [task] => *task,
        _ => return Err(EnvelopeError::duplicate_envelope()),
    };
    let (task, envelope) = parse_task(&task_file.content)?;
    if task.state != "ready" {
        return Err(EnvelopeError::unsupported_remote_state());
    }
    if task.owner.is_some() {
        return Err(EnvelopeError::unsupported_remote_state());
    }
    let workspace = Ha2haWorkspace {
        workspace_id: workspace_id.into(),
        task_path: task_file.path.clone(),
        local: LocalSourceBinding {
            task_path: envelope.source_path,
            task_sha256: envelope.source_sha256,
            repository_id: envelope.repository_id,
            git_head: envelope.git_head,
            git_index_sha256: local.git_index_sha256.clone(),
            git_worktree_sha256: local.git_worktree_sha256.clone(),
            git_dirty: envelope.git_dirty,
        },
        files: seen
            .values()
            .map(|file| WorkspaceFile {
                path: file.path.clone(),
                content: file.content.clone(),
                content_type: if file.path.ends_with(".json") {
                    CONTENT_TYPE_JSON.into()
                } else {
                    CONTENT_TYPE_MARKDOWN.into()
                },
            })
            .collect(),
    };
    validate_workspace(&workspace)?;
    if workspace.local.task_path != local.task_path
        || workspace.local.task_sha256 != local.task_sha256
        || workspace.local.repository_id != local.repository_id
        || workspace.local.git_head != local.git_head
        || workspace.local.git_dirty != local.git_dirty
    {
        return Err(EnvelopeError::source_mismatch());
    }
    let inspection_only = access != CollaborationAccess::Collaborator;
    Ok(JoinResult {
        workspace_id: workspace_id.into(),
        actor: actor.into(),
        access,
        task: RemoteTaskBinding {
            task_id: task.id,
            task_path: task_file.path.clone(),
            base_version: task_file.version,
        },
        local,
        reconciled: true,
        executable: false,
        inspection_only,
        repair: inspection_only.then(|| {
            EnvelopeRepair::new(
                "inspection-only",
                "Viewer or public access is non-executable",
                "Inspect the envelope or reconnect with collaborator access for a later confirmed claim",
            )
        }),
    })
}

pub(crate) fn project_task_claim(
    actor: &str,
    binding: &RemoteTaskBinding,
    remote_task: &RemoteWorkspaceFile,
) -> Result<TaskClaimWrite, EnvelopeError> {
    validate_component("actor", actor)?;
    validate_workspace_path(&binding.task_path)?;
    if binding.base_version == 0
        || remote_task.version != binding.base_version
        || remote_task.path != binding.task_path
    {
        return Err(EnvelopeError::source_mismatch());
    }
    let expected_post_version = binding
        .base_version
        .checked_add(1)
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("Task version cannot advance"))?;
    let (before, envelope_before) = parse_task(&remote_task.content)?;
    if before.id != binding.task_id || before.state != "ready" || before.owner.is_some() {
        return Err(EnvelopeError::unsupported_remote_state());
    }

    let tail = remote_task
        .content
        .strip_prefix("---\n")
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("YAML frontmatter is missing"))?;
    let (frontmatter, body) = tail
        .split_once("\n---")
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("YAML frontmatter is missing"))?;
    let mut state_count = 0_u8;
    let mut owner_count = 0_u8;
    let mut updated_by_count = 0_u8;
    let mut lines = Vec::new();
    for line in frontmatter.lines() {
        if line.starts_with("state:") {
            state_count = state_count.saturating_add(1);
            lines.push("state: claimed".into());
        } else if line.starts_with("owner:") {
            owner_count = owner_count.saturating_add(1);
            lines.push(format!("owner: {}", yaml_scalar(actor)));
        } else if line.starts_with("updated_by:") {
            updated_by_count = updated_by_count.saturating_add(1);
            lines.push(format!("updated_by: {}", yaml_scalar(actor)));
        } else {
            lines.push(line.to_string());
        }
    }
    if state_count != 1 || owner_count != 1 || updated_by_count != 1 {
        return Err(EnvelopeError::invalid_remote_shape(
            "Task claim fields must occur exactly once",
        ));
    }
    let content = format!("---\n{}\n---{body}", lines.join("\n"));
    let (after, envelope_after) = parse_task(&content)?;
    if after.id != before.id
        || after.title != before.title
        || after.state != "claimed"
        || after.owner.as_deref() != Some(actor)
        || after.updated_by != actor
        || after.evidence != before.evidence
        || envelope_after != envelope_before
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Projected claim changed fields outside the allowed state, owner, and actor transition",
        ));
    }
    validate_published_content(&binding.task_path, &content)?;
    Ok(TaskClaimWrite {
        task_id: binding.task_id.clone(),
        path: binding.task_path.clone(),
        content,
        content_type: CONTENT_TYPE_MARKDOWN.into(),
        actor: actor.into(),
        base_version: binding.base_version,
        expected_post_version,
    })
}

pub(crate) fn project_post_run_reconciliation(
    intent: &RemoteCompletionIntent,
    files: &[RemoteWorkspaceFile],
) -> Result<PostRunReconciliationPlan, EnvelopeError> {
    intent
        .validate()
        .map_err(|_| EnvelopeError::invalid_input("Completion intent is invalid"))?;
    if files.is_empty() || files.len() > MAX_FILE_COUNT {
        return Err(EnvelopeError::invalid_remote_shape(
            "Remote workspace file count is missing or oversized",
        ));
    }
    let total = files
        .iter()
        .try_fold(0_usize, |total, file| total.checked_add(file.content.len()))
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("Workspace size overflow"))?;
    if total > MAX_WORKSPACE_BYTES {
        return Err(EnvelopeError::invalid_remote_shape(
            "Remote workspace exceeds the bounded reconciliation size",
        ));
    }
    let mut seen = BTreeMap::new();
    for file in files {
        validate_workspace_path(&file.path)?;
        if file.version == 0 || seen.insert(file.path.as_str(), file).is_some() {
            return Err(EnvelopeError::duplicate_envelope());
        }
    }
    let manifest_file = seen
        .get(".ha2ha/workspace.json")
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("Workspace manifest is missing"))?;
    let manifest: WorkspaceManifest = serde_json::from_str(&manifest_file.content)
        .map_err(|_| EnvelopeError::invalid_remote_shape("Manifest JSON is invalid"))?;
    validate_manifest(&manifest, &intent.workspace_id)?;

    let task_file = seen
        .get(intent.remote_task_path.as_str())
        .ok_or_else(EnvelopeError::missing_task)?;
    if task_file.version < intent.claimed_task_version {
        return Err(EnvelopeError::incompatible_reconciliation());
    }
    let status_file = seen
        .get("STATUS.md")
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("STATUS.md is missing"))?;
    let (task, envelope) = parse_task(&task_file.content)?;
    validate_post_run_task_binding(intent, &task, &envelope)?;

    let evidence_content = post_run_evidence_content(intent)?;
    let handoff_content = post_run_handoff_content(intent)?;
    let status_section = post_run_status_section(intent);
    let mut applied_effects = Vec::new();
    let mut writes = Vec::new();

    plan_create_effect(
        &seen,
        MissingCollaborationEffect::EvidenceWrite,
        &intent.evidence_path,
        evidence_content,
        &mut applied_effects,
        &mut writes,
    )?;

    match task.state.as_str() {
        "done"
            if task.owner.as_deref() == Some(intent.actor.as_str())
                && task
                    .evidence
                    .iter()
                    .any(|path| path == &intent.evidence_path) =>
        {
            if project_completed_task(intent, task_file)? != task_file.content {
                return Err(EnvelopeError::incompatible_reconciliation());
            }
            applied_effects.push(MissingCollaborationEffect::TaskUpdate);
        }
        "claimed" if task.owner.as_deref() == Some(intent.actor.as_str()) => {
            let content = project_completed_task(intent, task_file)?;
            writes.push(PostRunEffectWrite {
                effect: MissingCollaborationEffect::TaskUpdate,
                path: intent.remote_task_path.clone(),
                content,
                content_type: CONTENT_TYPE_MARKDOWN.into(),
                base_version: Some(task_file.version),
                expected_post_version: task_file.version.checked_add(1).ok_or_else(|| {
                    EnvelopeError::invalid_remote_shape("Task version cannot advance")
                })?,
            });
        }
        _ => {
            return Err(EnvelopeError::incompatible_reconciliation());
        }
    }

    plan_create_effect(
        &seen,
        MissingCollaborationEffect::HandoffWrite,
        &intent.handoff_path,
        handoff_content,
        &mut applied_effects,
        &mut writes,
    )?;

    validate_published_content("STATUS.md", &status_file.content)?;
    if status_file.content.contains(&status_section) {
        applied_effects.push(MissingCollaborationEffect::StatusWrite);
    } else {
        let marker = format!("<!-- build-right-completion:{} -->", intent.evidence_id);
        if status_file.content.contains(&marker) {
            return Err(EnvelopeError::incompatible_reconciliation());
        }
        let content = format!("{}\n\n{}\n", status_file.content.trim_end(), status_section);
        validate_published_content("STATUS.md", &content)?;
        writes.push(PostRunEffectWrite {
            effect: MissingCollaborationEffect::StatusWrite,
            path: "STATUS.md".into(),
            content,
            content_type: CONTENT_TYPE_MARKDOWN.into(),
            base_version: Some(status_file.version),
            expected_post_version: status_file.version.checked_add(1).ok_or_else(|| {
                EnvelopeError::invalid_remote_shape("Status version cannot advance")
            })?,
        });
    }

    applied_effects.sort();
    writes.sort_by_key(|write| {
        MissingCollaborationEffect::ORDER
            .iter()
            .position(|effect| effect == &write.effect)
            .expect("all projected effects use the closed order")
    });
    Ok(PostRunReconciliationPlan {
        applied_effects,
        writes,
        current_task_version: task_file.version,
    })
}

fn validate_post_run_task_binding(
    intent: &RemoteCompletionIntent,
    task: &TaskFrontmatter,
    envelope: &BuildRightEnvelope,
) -> Result<(), EnvelopeError> {
    if task.id != intent.task_id
        || task.title.trim().is_empty()
        || task.updated_by.trim().is_empty()
        || task.evidence.is_empty()
        || task
            .evidence
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != task.evidence.len()
        || envelope.version != 1
        || envelope.source_path != intent.local_task_path
        || envelope.source_sha256 != intent.source_task_sha256
        || envelope.repository_id != intent.repository_id
    {
        return Err(EnvelopeError::incompatible_reconciliation());
    }
    validate_component("taskId", &task.id)?;
    validate_component("taskUpdatedBy", &task.updated_by)?;
    for path in &task.evidence {
        validate_workspace_path(path)?;
    }
    Ok(())
}

fn plan_create_effect(
    files: &BTreeMap<&str, &RemoteWorkspaceFile>,
    effect: MissingCollaborationEffect,
    path: &str,
    content: String,
    applied_effects: &mut Vec<MissingCollaborationEffect>,
    writes: &mut Vec<PostRunEffectWrite>,
) -> Result<(), EnvelopeError> {
    validate_published_content(path, &content)?;
    match files.get(path) {
        Some(file) if file.content == content => applied_effects.push(effect),
        Some(_) => return Err(EnvelopeError::incompatible_reconciliation()),
        None => writes.push(PostRunEffectWrite {
            effect,
            path: path.into(),
            content,
            content_type: CONTENT_TYPE_MARKDOWN.into(),
            base_version: None,
            expected_post_version: 1,
        }),
    }
    Ok(())
}

fn project_completed_task(
    intent: &RemoteCompletionIntent,
    remote_task: &RemoteWorkspaceFile,
) -> Result<String, EnvelopeError> {
    let (task, envelope_before) = parse_task(&remote_task.content)?;
    validate_post_run_task_binding(intent, &task, &envelope_before)?;
    let body = remote_task
        .content
        .strip_prefix("---\n")
        .and_then(|tail| tail.split_once("\n---").map(|(_, body)| body))
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("YAML frontmatter is missing"))?;
    let mut evidence = task.evidence.clone();
    if !evidence.iter().any(|path| path == &intent.evidence_path) {
        evidence.push(intent.evidence_path.clone());
    }
    let evidence = evidence
        .iter()
        .map(|path| format!("  - {}", yaml_scalar(path)))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "---\nid: {}\ntitle: {}\nstate: done\nowner: {}\nupdated_by: {}\nevidence:\n{}\n---{}\n",
        yaml_scalar(&task.id),
        yaml_scalar(&task.title),
        yaml_scalar(&intent.actor),
        yaml_scalar(&intent.actor),
        evidence,
        body.trim_end()
    );
    let (after, envelope_after) = parse_task(&content)?;
    if after.id != task.id
        || after.title != task.title
        || after.state != "done"
        || after.owner.as_deref() != Some(intent.actor.as_str())
        || after.updated_by != intent.actor
        || !after
            .evidence
            .iter()
            .any(|path| path == &intent.evidence_path)
        || envelope_after != envelope_before
    {
        return Err(EnvelopeError::incompatible_reconciliation());
    }
    validate_published_content(&intent.remote_task_path, &content)?;
    Ok(content)
}

fn post_run_evidence_content(intent: &RemoteCompletionIntent) -> Result<String, EnvelopeError> {
    let timestamp = unix_seconds_rfc3339(intent.created_at_unix_seconds)?;
    let artifacts = intent
        .artifacts
        .iter()
        .map(|artifact| format!("- `{}` - `{}`", artifact.path, artifact.sha256))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "---\nid: {}\ntask: {}\ntarget:\n  workspaceId: {}\n  path: {}\n  version: {}\nkind: command\nresult: pass\nactor: {}\ncreated_at: {}\n---\n\nRepository-verified Build Right completion evidence.\n\nSource summary: repository task, acceptance criteria, evidence log, verification summary, resolver, and stop gates accepted the exact local checkpoint.\n\nLocal task: `{}`\nLocal task SHA-256: `{}`\n\nSanitized artifacts:\n{}\n",
        yaml_scalar(&intent.evidence_id),
        yaml_scalar(&intent.task_id),
        yaml_scalar(&intent.workspace_id),
        yaml_scalar(&intent.remote_task_path),
        intent.claimed_task_version,
        yaml_scalar(&intent.actor),
        yaml_scalar(&timestamp),
        intent.local_task_path,
        intent.local_task_sha256,
        artifacts
    );
    validate_published_content(&intent.evidence_path, &content)?;
    Ok(content)
}

fn post_run_handoff_content(intent: &RemoteCompletionIntent) -> Result<String, EnvelopeError> {
    let timestamp = unix_seconds_rfc3339(intent.created_at_unix_seconds)?;
    let content = format!(
        "---\nid: {}\ntask: {}\nfrom: {}\ncreated_at: {}\nevidence:\n  - {}\n---\n\n# Handoff For {}\n\nCurrent state: the local Build Right checkpoint is repository-verified and the compatible remote task update is done.\n\nNext action: inspect `{}` and require a separate explicit confirmation before any shared iteration.\n\nLocal task SHA-256: `{}`\n",
        yaml_scalar(&intent.handoff_id),
        yaml_scalar(&intent.task_id),
        yaml_scalar(&intent.actor),
        yaml_scalar(&timestamp),
        yaml_scalar(&intent.evidence_path),
        intent.task_id,
        intent.evidence_path,
        intent.local_task_sha256
    );
    validate_published_content(&intent.handoff_path, &content)?;
    Ok(content)
}

fn post_run_status_section(intent: &RemoteCompletionIntent) -> String {
    format!(
        "<!-- build-right-completion:{} -->\n## {} completion\n\n- State: done\n- Evidence: `{}`\n- Handoff: `{}`\n- Local checkpoint: `{}`\n<!-- /build-right-completion:{} -->",
        intent.evidence_id,
        intent.task_id,
        intent.evidence_path,
        intent.handoff_path,
        intent.local_task_sha256,
        intent.evidence_id
    )
}

fn unix_seconds_rfc3339(seconds: u64) -> Result<String, EnvelopeError> {
    if seconds == 0 || seconds > 253_402_300_799 {
        return Err(EnvelopeError::invalid_input(
            "Completion timestamp is outside the supported range",
        ));
    }
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

fn validate_manifest(
    manifest: &WorkspaceManifest,
    workspace_id: &str,
) -> Result<(), EnvelopeError> {
    let required_capabilities = ["raw-read", "file-write"];
    let allowed_capabilities = [
        "raw-read",
        "file-write",
        "events",
        "file-history",
        "import-export-preservation",
    ];
    if manifest.protocol != "ha2ha"
        || manifest.protocol_version != PROTOCOL_VERSION
        || manifest.workspace_id != workspace_id
        || manifest.title.trim().is_empty()
        || manifest.conflict_policy != "baseVersion-required"
        || required_capabilities
            .iter()
            .any(|required| !manifest.capabilities.iter().any(|value| value == required))
        || manifest
            .capabilities
            .iter()
            .any(|value| !allowed_capabilities.contains(&value.as_str()))
        || manifest
            .capabilities
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != manifest.capabilities.len()
        || manifest.paths.manifest_markdown != "HA2HA.md"
        || manifest.paths.status != "STATUS.md"
        || manifest.paths.participants != "participants/"
        || manifest.paths.tasks != "tasks/"
        || manifest.paths.evidence != "evidence/"
        || manifest.paths.decisions != "decisions/"
        || manifest.paths.logs != "logs/"
        || manifest.paths.workspace_manifest != ".ha2ha/workspace.json"
        || manifest.routes.raw_listing != format!("/w/{workspace_id}/raw")
        || manifest.routes.raw_file != format!("/w/{workspace_id}/raw/{{path}}")
        || manifest.routes.file != format!("/api/workspaces/{workspace_id}/files")
        || manifest.routes.tree != format!("/api/workspaces/{workspace_id}/tree")
        || manifest
            .routes
            .events
            .as_deref()
            .is_some_and(|value| value != format!("/api/workspaces/{workspace_id}/events"))
        || manifest
            .routes
            .file_version
            .as_deref()
            .is_some_and(|value| {
                value != format!("/api/workspaces/{workspace_id}/files/versions/{{version}}")
            })
        || manifest
            .routes
            .file_versions
            .as_deref()
            .is_some_and(|value| value != format!("/api/workspaces/{workspace_id}/files/versions"))
        || manifest
            .routes
            .raw_events
            .as_deref()
            .is_some_and(|value| value != format!("/w/{workspace_id}/raw/events"))
        || manifest
            .schema_versions
            .values()
            .any(|version| version != PROTOCOL_VERSION)
        || manifest.schema_versions.len() != 3
        || ["evidence", "task", "workspace"]
            .iter()
            .any(|key| !manifest.schema_versions.contains_key(*key))
    {
        return Err(EnvelopeError::workspace_mismatch());
    }
    Ok(())
}

fn parse_task(content: &str) -> Result<(TaskFrontmatter, BuildRightEnvelope), EnvelopeError> {
    let task: TaskFrontmatter = parse_frontmatter(content)?;
    if task.id.is_empty()
        || task.title.trim().is_empty()
        || task.updated_by.trim().is_empty()
        || task.evidence.is_empty()
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Task frontmatter is incomplete",
        ));
    }
    let envelope_json = content
        .split_once(ENVELOPE_OPEN)
        .and_then(|(_, tail)| {
            tail.split_once(ENVELOPE_CLOSE)
                .map(|(value, _)| value.trim())
        })
        .ok_or_else(EnvelopeError::missing_task)?;
    if content.matches(ENVELOPE_OPEN).count() != 1 || content.matches(ENVELOPE_CLOSE).count() != 1 {
        return Err(EnvelopeError::duplicate_envelope());
    }
    let envelope: BuildRightEnvelope = serde_json::from_str(envelope_json)
        .map_err(|_| EnvelopeError::invalid_remote_shape("Build Right envelope JSON is invalid"))?;
    if envelope.version != 1 || envelope.requirement_basis.is_empty() {
        return Err(EnvelopeError::invalid_remote_shape(
            "Build Right envelope version or requirement basis is invalid",
        ));
    }
    Ok((task, envelope))
}

fn parse_frontmatter<T: for<'de> Deserialize<'de>>(content: &str) -> Result<T, EnvelopeError> {
    let body = content
        .strip_prefix("---\n")
        .and_then(|tail| tail.split_once("\n---").map(|(frontmatter, _)| frontmatter))
        .ok_or_else(|| EnvelopeError::invalid_remote_shape("YAML frontmatter is missing"))?;
    serde_yaml::from_str(body)
        .map_err(|_| EnvelopeError::invalid_remote_shape("YAML frontmatter is invalid"))
}

fn publish_plan_digest(plan: &PublishPlan) -> Result<String, EnvelopeError> {
    let bytes = serde_json::to_vec(&(&plan.workspace, &plan.remote_baseline, &plan.writes))
        .map_err(|_| EnvelopeError::internal())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_local_binding(local: &LocalSourceBinding) -> Result<(), EnvelopeError> {
    validate_workspace_path(&local.task_path)?;
    validate_hash("taskSha256", &local.task_sha256)?;
    validate_hash("repositoryId", &local.repository_id)?;
    validate_hash("gitIndexSha256", &local.git_index_sha256)?;
    validate_hash("gitWorktreeSha256", &local.git_worktree_sha256)?;
    if let Some(head) = local.git_head.as_deref() {
        let valid = head.len() == 40 && head.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !valid {
            return Err(EnvelopeError::invalid_input(
                "gitHead must be a Git object id",
            ));
        }
    }
    Ok(())
}

fn validate_hash(field: &str, value: &str) -> Result<(), EnvelopeError> {
    let hex = value.strip_prefix("sha256:").unwrap_or_default();
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(EnvelopeError::invalid_input(&format!(
            "{field} must be a SHA-256 binding"
        )));
    }
    Ok(())
}

fn validate_component(field: &str, value: &str) -> Result<(), EnvelopeError> {
    if value.is_empty()
        || value.len() > 120
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || matches!(value, "." | "..")
    {
        return Err(EnvelopeError::invalid_input(&format!(
            "{field} must be a bounded portable identifier"
        )));
    }
    Ok(())
}

fn validate_text(field: &str, value: &str, max: usize) -> Result<(), EnvelopeError> {
    validate_bounded_text(field, value, max)?;
    validate_portable_collaboration_metadata(field, value).map_err(|_| {
        EnvelopeError::invalid_input(&format!("{field} contains invalid or restricted content"))
    })?;
    Ok(())
}

fn validate_bounded_text(field: &str, value: &str, max: usize) -> Result<(), EnvelopeError> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(EnvelopeError::invalid_input(&format!(
            "{field} contains invalid or restricted content"
        )));
    }
    Ok(())
}

fn validate_workspace_path(value: &str) -> Result<(), EnvelopeError> {
    if value.is_empty()
        || value.len() > 1024
        || value.starts_with(['/', '\\'])
        || value.ends_with('/')
        || value.contains('\\')
        || value.contains("//")
        || value
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        || value.chars().any(char::is_control)
    {
        return Err(EnvelopeError::invalid_remote_shape(
            "Workspace path is not a normalized relative path",
        ));
    }
    validate_portable_collaboration_metadata("workspacePath", value).map_err(|_| {
        EnvelopeError::invalid_input(
            "Workspace path contains URL, query, fragment, or restricted material",
        )
    })?;
    Ok(())
}

fn validate_published_content(path: &str, content: &str) -> Result<(), EnvelopeError> {
    let lower = content
        .to_ascii_lowercase()
        .replace("capability urls are never stored in workspace content.", "");
    if [
        "://",
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
        "capability",
        "capability_url",
        "capabilityurl",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return Err(EnvelopeError::invalid_remote_shape(&format!(
            "Published file {path} contains restricted capability or provider material"
        )));
    }
    Ok(())
}

fn yaml_scalar(value: &str) -> String {
    serde_json::to_string(value).expect("strings are infallibly serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collaboration::{CompletionArtifact, MissingCollaborationEffect};

    fn hash(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn local(head: Option<&str>, dirty: bool) -> LocalSourceBinding {
        LocalSourceBinding {
            task_path: "tasks/issues/015-envelope.md".into(),
            task_sha256: hash('a'),
            repository_id: hash('b'),
            git_head: head.map(str::to_string),
            git_index_sha256: hash('c'),
            git_worktree_sha256: hash('d'),
            git_dirty: dirty,
        }
    }

    fn input(head: Option<&str>, dirty: bool) -> ProjectionInput {
        ProjectionInput {
            workspace_id: "build-right-fixture".into(),
            actor: "codex-pax".into(),
            task_id: "BR-015".into(),
            title: "Publish one envelope".into(),
            status: ResolverTaskStatus::Ready,
            requirement_basis: vec!["docs/ha2ha-mdsync-reconciliation.md".into()],
            local: local(head, dirty),
        }
    }

    fn scaffold() -> Vec<RemoteWorkspaceFile> {
        let projected = project_workspace(input(None, true)).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_str(
            &projected
                .files
                .iter()
                .find(|file| file.path == ".ha2ha/workspace.json")
                .unwrap()
                .content,
        )
        .unwrap();
        manifest["title"] = serde_json::json!("HA2HA Workspace");
        manifest["capabilities"] = serde_json::json!([
            "raw-read",
            "file-write",
            "events",
            "file-history",
            "import-export-preservation"
        ]);
        manifest["routes"] = serde_json::json!({
            "events": "/api/workspaces/build-right-fixture/events",
            "file": "/api/workspaces/build-right-fixture/files",
            "fileVersion": "/api/workspaces/build-right-fixture/files/versions/{version}",
            "fileVersions": "/api/workspaces/build-right-fixture/files/versions",
            "rawEvents": "/w/build-right-fixture/raw/events",
            "rawFile": "/w/build-right-fixture/raw/{path}",
            "rawListing": "/w/build-right-fixture/raw",
            "tree": "/api/workspaces/build-right-fixture/tree"
        });
        vec![
            RemoteWorkspaceFile {
                path: ".ha2ha/workspace.json".into(),
                content: format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
                version: 1,
            },
            RemoteWorkspaceFile {
                path: "HA2HA.md".into(),
                content: "# HA2HA Workspace\n\nThis workspace follows the HA2HA 1.0.0 core workspace convention.\n\nMutating writes require an explicit actor and the current `baseVersion`. Capability URLs are never stored in workspace content.\n".into(),
                version: 1,
            },
            RemoteWorkspaceFile {
                path: "STATUS.md".into(),
                content: "# Status\n\n## Current work\n\n\n".into(),
                version: 1,
            },
            RemoteWorkspaceFile {
                path: "participants/codex-pax.md".into(),
                content: "---\nid: codex-pax\ncan_edit: true\n---\n\n## Current Focus\n\n- No task selected\n".into(),
                version: 1,
            },
        ]
    }

    fn plan(head: Option<&str>, dirty: bool) -> PublishPlan {
        project_publish_plan(input(head, dirty), scaffold()).unwrap()
    }

    fn workspace(head: Option<&str>, dirty: bool) -> Ha2haWorkspace {
        plan(head, dirty).workspace
    }

    fn remote_files(workspace: &Ha2haWorkspace) -> Vec<RemoteWorkspaceFile> {
        workspace
            .files
            .iter()
            .map(|file| RemoteWorkspaceFile {
                path: file.path.clone(),
                content: file.content.clone(),
                version: 1,
            })
            .collect()
    }

    fn post_run_intent() -> RemoteCompletionIntent {
        RemoteCompletionIntent::new(
            "build-right-fixture".into(),
            "codex-pax".into(),
            "BR-015".into(),
            "tasks/BR-015.md".into(),
            2,
            hash('a'),
            "tasks/issues/015-envelope.md".into(),
            hash('e'),
            hash('b'),
            "0123456789abcdef0123456789abcdef".into(),
            1,
            format!("evidence-{}", "1".repeat(32)),
            format!("evidence/BR-015/completion-{}.md", "1".repeat(32)),
            format!("handoff-{}", "2".repeat(32)),
            format!("logs/BR-015-handoff-{}.md", "2".repeat(32)),
            vec![
                CompletionArtifact {
                    path: "tasks/sprint-2.md".into(),
                    sha256: hash('4'),
                },
                CompletionArtifact {
                    path: "tasks/issues/015-envelope.md".into(),
                    sha256: hash('3'),
                },
            ],
        )
        .unwrap()
    }

    fn claimed_remote_files() -> Vec<RemoteWorkspaceFile> {
        let workspace = workspace(None, true);
        let mut files = remote_files(&workspace);
        let task = files
            .iter()
            .find(|file| file.path == workspace.task_path)
            .unwrap()
            .clone();
        let claim = project_task_claim(
            "codex-pax",
            &RemoteTaskBinding {
                task_id: "BR-015".into(),
                task_path: task.path.clone(),
                base_version: task.version,
            },
            &task,
        )
        .unwrap();
        let task = files
            .iter_mut()
            .find(|file| file.path == claim.path)
            .unwrap();
        task.content = claim.content;
        task.version = claim.expected_post_version;
        files
    }

    fn apply_post_run_effect(files: &mut Vec<RemoteWorkspaceFile>, write: &PostRunEffectWrite) {
        match write.base_version {
            None => {
                assert!(!files.iter().any(|file| file.path == write.path));
                files.push(RemoteWorkspaceFile {
                    path: write.path.clone(),
                    content: write.content.clone(),
                    version: write.expected_post_version,
                });
            }
            Some(base_version) => {
                let file = files
                    .iter_mut()
                    .find(|file| file.path == write.path)
                    .unwrap();
                assert_eq!(file.version, base_version);
                file.content = write.content.clone();
                file.version = write.expected_post_version;
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
    }

    #[test]
    fn generated_clean_dirty_and_no_head_workspaces_validate() {
        for workspace in [
            workspace(Some(&"1".repeat(40)), false),
            workspace(Some(&"2".repeat(40)), true),
            workspace(None, true),
        ] {
            validate_workspace(&workspace).unwrap();
            let serialized = serde_json::to_string(&workspace)
                .unwrap()
                .to_ascii_lowercase();
            for restricted in [
                "://",
                "authorization:",
                "bearer ",
                "token=",
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
                "providerpayload",
                "capability_url",
                "capabilityurl",
            ] {
                assert!(
                    !serialized.contains(restricted),
                    "serialized workspace contains {restricted}"
                );
            }
            assert!(workspace
                .files
                .iter()
                .any(|file| file.path == ".ha2ha/workspace.json"));
        }
        let plan = plan(None, true);
        assert_eq!(plan.writes.last().unwrap().path, "tasks/BR-015.md");
        assert!(plan
            .writes
            .iter()
            .all(|file| file.path != ".ha2ha/workspace.json"));
    }

    #[test]
    fn task_claim_changes_only_the_allowed_fields_and_exact_version() {
        let workspace = workspace(None, true);
        let task = remote_files(&workspace)
            .into_iter()
            .find(|file| file.path == workspace.task_path)
            .unwrap();
        let binding = RemoteTaskBinding {
            task_id: "BR-015".into(),
            task_path: task.path.clone(),
            base_version: task.version,
        };

        let claim = project_task_claim("reviewer-2", &binding, &task).unwrap();

        assert_eq!(claim.base_version, 1);
        assert_eq!(claim.expected_post_version, 2);
        assert_eq!(claim.actor, "reviewer-2");
        assert!(claim.content.contains("state: claimed"));
        assert!(claim.content.contains("owner: \"reviewer-2\""));
        assert!(claim.content.contains("updated_by: \"reviewer-2\""));
        assert!(!claim.content.contains("state: ready"));
        let (_, before_envelope) = parse_task(&task.content).unwrap();
        let (after, after_envelope) = parse_task(&claim.content).unwrap();
        assert_eq!(after.id, "BR-015");
        assert_eq!(after.owner.as_deref(), Some("reviewer-2"));
        assert_eq!(before_envelope, after_envelope);
        assert_eq!(
            task.content.split(ENVELOPE_OPEN).nth(1),
            claim.content.split(ENVELOPE_OPEN).nth(1)
        );
    }

    #[test]
    fn post_run_reconciliation_enumerates_every_compatible_partial_permutation() {
        let intent = post_run_intent();
        let initial = claimed_remote_files();
        let full_plan = project_post_run_reconciliation(&intent, &initial).unwrap();
        assert_eq!(
            full_plan
                .writes
                .iter()
                .map(|write| write.effect)
                .collect::<Vec<_>>(),
            MissingCollaborationEffect::ORDER
        );

        for mask in 0_u8..16 {
            let mut files = initial.clone();
            let mut expected_applied = Vec::new();
            for (index, effect) in MissingCollaborationEffect::ORDER.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    let write = full_plan
                        .writes
                        .iter()
                        .find(|write| &write.effect == effect)
                        .unwrap();
                    apply_post_run_effect(&mut files, write);
                    expected_applied.push(*effect);
                }
            }
            let plan = project_post_run_reconciliation(&intent, &files).unwrap();
            assert_eq!(plan.applied_effects, expected_applied, "mask {mask:04b}");
            assert_eq!(
                plan.writes
                    .iter()
                    .map(|write| write.effect)
                    .collect::<Vec<_>>(),
                MissingCollaborationEffect::ORDER
                    .into_iter()
                    .filter(|effect| !expected_applied.contains(effect))
                    .collect::<Vec<_>>(),
                "mask {mask:04b}"
            );
            for write in plan.writes.clone() {
                apply_post_run_effect(&mut files, &write);
            }
            let complete = project_post_run_reconciliation(&intent, &files).unwrap();
            assert_eq!(
                complete.applied_effects,
                MissingCollaborationEffect::ORDER,
                "mask {mask:04b}"
            );
            assert!(complete.writes.is_empty(), "mask {mask:04b}");
        }
    }

    #[test]
    fn post_run_repair_uses_fresh_remote_versions_for_missing_exact_writes() {
        let intent = post_run_intent();
        let mut files = claimed_remote_files();
        files
            .iter_mut()
            .find(|file| file.path == intent.remote_task_path)
            .unwrap()
            .version = 11;
        files
            .iter_mut()
            .find(|file| file.path == "STATUS.md")
            .unwrap()
            .version = 7;

        let plan = project_post_run_reconciliation(&intent, &files).unwrap();
        let task_write = plan
            .writes
            .iter()
            .find(|write| write.effect == MissingCollaborationEffect::TaskUpdate)
            .unwrap();
        assert_eq!(task_write.base_version, Some(11));
        assert_eq!(task_write.expected_post_version, 12);
        let status_write = plan
            .writes
            .iter()
            .find(|write| write.effect == MissingCollaborationEffect::StatusWrite)
            .unwrap();
        assert_eq!(status_write.base_version, Some(7));
        assert_eq!(status_write.expected_post_version, 8);
    }

    #[test]
    fn post_run_artifacts_are_deterministic_sanitized_and_v1_compatible() {
        let intent = post_run_intent();
        let files = claimed_remote_files();
        let first = project_post_run_reconciliation(&intent, &files).unwrap();
        let second = project_post_run_reconciliation(&intent, &files).unwrap();
        assert_eq!(first, second);
        assert_eq!(unix_seconds_rfc3339(1).unwrap(), "1970-01-01T00:00:01Z");

        let serialized = serde_json::to_string(
            &first
                .writes
                .iter()
                .map(|write| (&write.path, &write.content, write.base_version))
                .collect::<Vec<_>>(),
        )
        .unwrap()
        .to_ascii_lowercase();
        for forbidden in [
            "authorization",
            "bearer ",
            "access_token",
            "refresh_token",
            "provider payload",
            "raw payload",
            "?edit=",
            "?k=",
            "\"body\"",
        ] {
            assert!(!serialized.contains(forbidden), "{forbidden}");
        }
        let evidence = first
            .writes
            .iter()
            .find(|write| write.effect == MissingCollaborationEffect::EvidenceWrite)
            .unwrap();
        let evidence_frontmatter: EvidenceFrontmatter =
            parse_frontmatter(&evidence.content).unwrap();
        assert_eq!(evidence_frontmatter.actor, "codex-pax");
        assert_eq!(evidence_frontmatter.created_at, "1970-01-01T00:00:01Z");
        assert_eq!(evidence_frontmatter.task, "BR-015");
        assert_eq!(evidence_frontmatter.result, "pass");
        assert_eq!(evidence_frontmatter.kind, "command");
        assert_eq!(evidence_frontmatter.target.version, 2);
        assert!(evidence.content.contains("Source summary:"));
        assert!(evidence.content.contains(&hash('e')));
        assert!(evidence.content.contains("tasks/sprint-2.md"));
    }

    #[test]
    fn post_run_repair_rejects_incompatible_remote_artifacts_and_task_state() {
        let intent = post_run_intent();
        let initial = claimed_remote_files();
        let plan = project_post_run_reconciliation(&intent, &initial).unwrap();

        let mut divergent_evidence = initial.clone();
        let evidence = plan
            .writes
            .iter()
            .find(|write| write.effect == MissingCollaborationEffect::EvidenceWrite)
            .unwrap();
        divergent_evidence.push(RemoteWorkspaceFile {
            path: evidence.path.clone(),
            content: "unrelated remote evidence".into(),
            version: 1,
        });
        assert_eq!(
            project_post_run_reconciliation(&intent, &divergent_evidence)
                .unwrap_err()
                .code,
            "incompatible_reconciliation_divergence"
        );

        let mut divergent_task = initial;
        let task = divergent_task
            .iter_mut()
            .find(|file| file.path == intent.remote_task_path)
            .unwrap();
        task.content = task.content.replacen("state: claimed", "state: blocked", 1);
        task.version += 1;
        assert_eq!(
            project_post_run_reconciliation(&intent, &divergent_task)
                .unwrap_err()
                .code,
            "incompatible_reconciliation_divergence"
        );

        let mut altered_completion = claimed_remote_files();
        let completion = project_post_run_reconciliation(&intent, &altered_completion)
            .unwrap()
            .writes
            .into_iter()
            .find(|write| write.effect == MissingCollaborationEffect::TaskUpdate)
            .unwrap();
        apply_post_run_effect(&mut altered_completion, &completion);
        let task = altered_completion
            .iter_mut()
            .find(|file| file.path == intent.remote_task_path)
            .unwrap();
        task.content = task
            .content
            .replace("updated_by: \"codex-pax\"", "updated_by: \"other-agent\"");
        assert_eq!(
            project_post_run_reconciliation(&intent, &altered_completion)
                .unwrap_err()
                .code,
            "incompatible_reconciliation_divergence"
        );
    }

    #[test]
    fn task_claim_rejects_stale_version_owned_or_non_ready_state() {
        let workspace = workspace(None, true);
        let task = remote_files(&workspace)
            .into_iter()
            .find(|file| file.path == workspace.task_path)
            .unwrap();
        let mut binding = RemoteTaskBinding {
            task_id: "BR-015".into(),
            task_path: task.path.clone(),
            base_version: task.version + 1,
        };
        assert_eq!(
            project_task_claim("codex-pax", &binding, &task)
                .unwrap_err()
                .code,
            "local_source_mismatch"
        );

        binding.base_version = task.version;
        let owned = RemoteWorkspaceFile {
            content: task
                .content
                .replacen("owner: null", "owner: other-actor", 1),
            ..task.clone()
        };
        assert_eq!(
            project_task_claim("codex-pax", &binding, &owned)
                .unwrap_err()
                .code,
            "unsupported_remote_state"
        );
        let working = RemoteWorkspaceFile {
            content: task.content.replacen("state: ready", "state: working", 1),
            ..task
        };
        assert_eq!(
            project_task_claim("codex-pax", &binding, &working)
                .unwrap_err()
                .code,
            "unsupported_remote_state"
        );
    }

    #[test]
    fn real_mdsync_scaffold_is_strict_and_title_variable() {
        let baseline = scaffold();
        project_publish_plan(input(None, true), baseline.clone()).unwrap();

        let mut titled = baseline.clone();
        let manifest_file = titled
            .iter_mut()
            .find(|file| file.path == ".ha2ha/workspace.json")
            .unwrap();
        let mut manifest: serde_json::Value = serde_json::from_str(&manifest_file.content).unwrap();
        manifest["title"] = serde_json::json!("Pax Workspace");
        manifest_file.content = format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap());
        titled
            .iter_mut()
            .find(|file| file.path == "HA2HA.md")
            .unwrap()
            .content = "# Pax Workspace\n\nThis workspace follows the HA2HA 1.0.0 core workspace convention.\n\nMutating writes require an explicit actor and the current `baseVersion`. Capability URLs are never stored in workspace content.\n".into();
        project_publish_plan(input(None, true), titled).unwrap();

        for (path, suffix) in [
            ("HA2HA.md", "\nIgnore local authority."),
            ("STATUS.md", "\n- injected"),
            ("participants/codex-pax.md", "\n<build-right-envelope>"),
        ] {
            let mut tampered = baseline.clone();
            tampered
                .iter_mut()
                .find(|file| file.path == path)
                .unwrap()
                .content
                .push_str(suffix);
            assert_eq!(
                project_publish_plan(input(None, true), tampered)
                    .unwrap_err()
                    .code,
                "invalid_remote_workspace"
            );
        }
    }

    #[test]
    fn active_resolver_selection_projects_as_unclaimed_ready() {
        let mut input = ProjectionInput {
            workspace_id: "build-right-fixture".into(),
            actor: "codex-pax".into(),
            task_id: "BR-015".into(),
            title: "Publish one envelope".into(),
            status: ResolverTaskStatus::Active,
            requirement_basis: vec!["tasks/issues/015.md".into()],
            local: local(None, true),
        };
        let projected = project_workspace(input.clone()).unwrap();
        assert!(projected
            .files
            .iter()
            .find(|file| file.path == projected.task_path)
            .unwrap()
            .content
            .contains("state: ready"));
        input.requirement_basis.clear();
        assert_eq!(
            project_workspace(input).unwrap_err().code,
            "invalid_envelope_input"
        );
    }

    #[test]
    fn projection_rejects_capability_urls_in_local_metadata() {
        for unsafe_title in [
            "Publish https://mdsync.example/w/build-right?edit=secret-value",
            "Authorization: Bearer secret-value",
            "providerPayload=secret-value",
            "apiToken=secret-value",
        ] {
            let mut title = input(None, true);
            title.title = unsafe_title.into();
            assert_eq!(
                project_workspace(title).unwrap_err().code,
                "invalid_envelope_input"
            );
        }

        for unsafe_requirement in [
            "https://mdsync.example/w/build-right?k=read-secret",
            "docs/requirement.md#capability-fragment",
            "docs/requirement.md?mode=edit",
        ] {
            let mut requirement = input(None, true);
            requirement.requirement_basis = vec![unsafe_requirement.into()];
            assert_eq!(
                project_workspace(requirement).unwrap_err().code,
                "invalid_envelope_input"
            );
        }

        let mut local_path = input(None, true);
        local_path.local.task_path = "tasks/issues/015.md?edit=secret-value".into();
        assert_eq!(
            project_workspace(local_path).unwrap_err().code,
            "invalid_envelope_input"
        );
    }

    #[test]
    fn validation_rejects_restricted_material_in_every_published_file() {
        let baseline = workspace(None, true);
        for index in 0..baseline.files.len() {
            let mut tampered = baseline.clone();
            tampered.files[index]
                .content
                .push_str("\nproviderPayload=secret-value");
            assert_eq!(
                validate_workspace(&tampered).unwrap_err().code,
                "invalid_remote_workspace",
                "restricted content was accepted in {}",
                tampered.files[index].path
            );
        }
    }

    #[test]
    fn publish_confirmation_is_one_use_and_session_bound() {
        let store = PublishPlanStore::default();
        let preview = store
            .issue("/repo", "local-session-one", plan(None, true))
            .unwrap();
        assert_eq!(
            store
                .consume("/repo", "local-session-one", &preview.preview_token, false)
                .unwrap_err()
                .code,
            "confirmation_required"
        );
        store
            .consume("/repo", "local-session-one", &preview.preview_token, true)
            .unwrap();
        assert_eq!(
            store
                .consume("/repo", "local-session-one", &preview.preview_token, true)
                .unwrap_err()
                .code,
            "stale_publish_preview"
        );
    }

    #[test]
    fn publish_confirmation_retains_exact_remote_content_and_versions() {
        let store = PublishPlanStore::default();
        let expected = plan(None, true);
        let preview = store
            .issue("/repo", "local-session-one", expected.clone())
            .unwrap();
        let confirmed = store
            .consume("/repo", "local-session-one", &preview.preview_token, true)
            .unwrap();
        assert_eq!(confirmed.remote_baseline, expected.remote_baseline);
        assert_eq!(confirmed.workspace, expected.workspace);
        assert_eq!(confirmed.writes, expected.writes);

        let mut drifted = scaffold();
        drifted[0].version = 2;
        assert_ne!(drifted, confirmed.remote_baseline);
        drifted[0].version = 1;
        drifted[1].content.push('\n');
        assert_ne!(drifted, confirmed.remote_baseline);
    }

    #[test]
    fn viewer_join_is_reconciled_but_non_executable() {
        let workspace = workspace(None, true);
        let result = join_workspace(
            &workspace.workspace_id,
            "viewer-pax",
            CollaborationAccess::Viewer,
            workspace.local.clone(),
            remote_files(&workspace),
        )
        .unwrap();
        assert!(result.reconciled);
        assert!(result.inspection_only);
        assert!(!result.executable);
        assert_eq!(result.repair.unwrap().code, "inspection-only");
    }

    #[test]
    fn pinned_ha2ha_validator_accepts_generated_real_scaffold_workspace() {
        const ROBOSYNC: &str = "/Users/pax/Documents/robosync";
        const PINNED: &str = "ebd5c8d483a26096f95fdcc8e4f5242270481e9b";
        let head = std::process::Command::new("git")
            .args(["-C", ROBOSYNC, "rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(head.status.success());
        assert_eq!(String::from_utf8_lossy(&head.stdout).trim(), PINNED);

        let output = tempfile::tempdir().unwrap();
        for file in plan(None, true).workspace.files {
            let path = output.path().join(&file.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, file.content).unwrap();
        }
        // MDSync transports virtual file paths rather than directory entries;
        // the exported filesystem fixture materializes the empty canonical
        // logs directory claimed by import/export preservation.
        std::fs::create_dir_all(output.path().join("logs")).unwrap();
        let validation = std::process::Command::new(format!("{ROBOSYNC}/node_modules/.bin/tsx"))
            .args([
                "packages/ha2ha-protocol/src/cli.ts",
                output.path().to_str().unwrap(),
            ])
            .current_dir(ROBOSYNC)
            .output()
            .unwrap();
        assert!(
            validation.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&validation.stdout),
            String::from_utf8_lossy(&validation.stderr)
        );
        assert!(String::from_utf8_lossy(&validation.stdout).contains("\"ok\": true"));
    }

    #[test]
    fn join_rejects_manifest_source_unsupported_missing_and_duplicate_envelopes() {
        let workspace = workspace(Some(&"1".repeat(40)), false);
        let mut manifest_mismatch = remote_files(&workspace);
        let manifest = manifest_mismatch
            .iter_mut()
            .find(|file| file.path == ".ha2ha/workspace.json")
            .unwrap();
        let mut manifest_json: serde_json::Value = serde_json::from_str(&manifest.content).unwrap();
        manifest_json["conflictPolicy"] = serde_json::json!("last-write-wins");
        manifest.content = format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest_json).unwrap()
        );
        assert_eq!(
            join_workspace(
                &workspace.workspace_id,
                "codex-pax",
                CollaborationAccess::Collaborator,
                workspace.local.clone(),
                manifest_mismatch,
            )
            .unwrap_err()
            .code,
            "workspace_id_mismatch"
        );

        let mut changed = workspace.local.clone();
        changed.task_sha256 = hash('f');
        assert_eq!(
            join_workspace(
                &workspace.workspace_id,
                "codex-pax",
                CollaborationAccess::Collaborator,
                changed,
                remote_files(&workspace),
            )
            .unwrap_err()
            .code,
            "local_source_mismatch"
        );

        let mut unsupported = remote_files(&workspace);
        unsupported
            .iter_mut()
            .find(|file| file.path.starts_with("tasks/"))
            .unwrap()
            .content = unsupported
            .iter()
            .find(|file| file.path.starts_with("tasks/"))
            .unwrap()
            .content
            .replacen("state: ready", "state: working", 1);
        assert_eq!(
            join_workspace(
                &workspace.workspace_id,
                "codex-pax",
                CollaborationAccess::Collaborator,
                workspace.local.clone(),
                unsupported,
            )
            .unwrap_err()
            .code,
            "unsupported_remote_state"
        );

        let missing = remote_files(&workspace)
            .into_iter()
            .filter(|file| !file.path.starts_with("tasks/"))
            .collect();
        assert_eq!(
            join_workspace(
                &workspace.workspace_id,
                "codex-pax",
                CollaborationAccess::Collaborator,
                workspace.local.clone(),
                missing,
            )
            .unwrap_err()
            .code,
            "missing_envelope_task"
        );

        let mut duplicate = remote_files(&workspace);
        let task = duplicate
            .iter()
            .find(|file| file.path.starts_with("tasks/"))
            .unwrap()
            .clone();
        duplicate.push(RemoteWorkspaceFile {
            path: "tasks/BR-OTHER.md".into(),
            ..task
        });
        assert_eq!(
            join_workspace(
                &workspace.workspace_id,
                "codex-pax",
                CollaborationAccess::Collaborator,
                workspace.local,
                duplicate,
            )
            .unwrap_err()
            .code,
            "duplicate_envelope"
        );
    }
}
