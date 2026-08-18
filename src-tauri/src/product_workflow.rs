#![allow(dead_code)] // Task 022 defines contracts; Tasks 020-023 wire them into UI/native commands.

use crate::{
    collaboration::{CollaborationAccess, CollaborationMode, ReconciliationState},
    GoalLoopState, GoalRecoveryState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    path::{Component, Path},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProductWorkspaceState {
    NoProject,
    ProjectNeedsSetup,
    PreflightNeedsInput,
    PlanningReady,
    TaskReadyForReview,
    AwaitingConfirmation,
    OperationRunning,
    ResultNeedsReview,
    ContinueAvailable,
    Resumable,
    RepairRequired,
    Blocked,
    GoalComplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProductMode {
    LocalSolo,
    ViewerInspection,
    SharedCollaborator,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProductAction {
    OpenOrCreateProject,
    CompleteSetup,
    AnswerFounderQuestions,
    PreviewPlanningChanges,
    ReviewSelectedTask,
    ConfirmOperation,
    InspectRunningOperation,
    ReviewResult,
    ReviewNextIteration,
    ResumeVerifiedGoal,
    RepairSharedState,
    InspectBlocker,
    FinishGoal,
    InspectSharedState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EffectClass {
    Inspect,
    PlanningMutation,
    BuildMutation,
    GitMutation,
    ExternalShared,
    DeveloperDiagnostic,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProjectionAuthority {
    Repository,
    GoalReceipt,
    Application,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductLocalProjection {
    loop_state: Option<GoalLoopState>,
    recovery_state: Option<GoalRecoveryState>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductSharedProjection {
    mode: CollaborationMode,
    access: Option<CollaborationAccess>,
    reconciliation: ReconciliationState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductWorkflowProjection {
    state: ProductWorkspaceState,
    mode: ProductMode,
    projection_source: ProjectionAuthority,
    primary_action: ProductAction,
    allowed_effects: Vec<EffectClass>,
    mutation_allowed: bool,
    explicit_confirmation_required: bool,
    automatic_execution_started: bool,
    local: ProductLocalProjection,
    shared: Option<ProductSharedProjection>,
}

pub(crate) struct ProductProjectionInput {
    pub(crate) project_selected: bool,
    pub(crate) project_needs_setup: bool,
    pub(crate) founder_input_required: bool,
    pub(crate) planning_ready: bool,
    pub(crate) selected_task_ready: bool,
    pub(crate) operation_running: bool,
    pub(crate) result_needs_review: bool,
    pub(crate) goal_loop_state: Option<GoalLoopState>,
    pub(crate) recovery_state: Option<GoalRecoveryState>,
    pub(crate) collaboration_mode: Option<CollaborationMode>,
    pub(crate) collaboration_access: Option<CollaborationAccess>,
    pub(crate) reconciliation: Option<ReconciliationState>,
}

fn is_terminal_stop(state: GoalLoopState) -> bool {
    matches!(
        state,
        GoalLoopState::FounderStop
            | GoalLoopState::ExternalStop
            | GoalLoopState::ConflictStop
            | GoalLoopState::FailureStop
            | GoalLoopState::StaleStop
            | GoalLoopState::CancelledStop
            | GoalLoopState::NoReadyTaskStop
            | GoalLoopState::InvalidStateStop
    )
}

fn product_state(input: &ProductProjectionInput) -> ProductWorkspaceState {
    if !input.project_selected {
        return ProductWorkspaceState::NoProject;
    }
    if input.operation_running {
        return ProductWorkspaceState::OperationRunning;
    }
    if input.goal_loop_state == Some(GoalLoopState::GoalComplete)
        || input.recovery_state == Some(GoalRecoveryState::Completed)
    {
        return ProductWorkspaceState::GoalComplete;
    }
    if input.reconciliation == Some(ReconciliationState::RepairRequired) {
        return ProductWorkspaceState::RepairRequired;
    }
    if input.result_needs_review {
        return ProductWorkspaceState::ResultNeedsReview;
    }
    if input.recovery_state == Some(GoalRecoveryState::Resumable) {
        return ProductWorkspaceState::Resumable;
    }
    if input.founder_input_required {
        return ProductWorkspaceState::PreflightNeedsInput;
    }
    if input.project_needs_setup {
        return ProductWorkspaceState::ProjectNeedsSetup;
    }
    if input.goal_loop_state == Some(GoalLoopState::AwaitingConfirmation) {
        return ProductWorkspaceState::AwaitingConfirmation;
    }
    if input.goal_loop_state == Some(GoalLoopState::ContinueAvailable) {
        return ProductWorkspaceState::ContinueAvailable;
    }
    if input.goal_loop_state.is_some_and(is_terminal_stop) {
        return ProductWorkspaceState::Blocked;
    }
    if input.selected_task_ready {
        return ProductWorkspaceState::TaskReadyForReview;
    }
    if input.planning_ready {
        return ProductWorkspaceState::PlanningReady;
    }
    ProductWorkspaceState::Blocked
}

fn product_action(state: ProductWorkspaceState) -> ProductAction {
    match state {
        ProductWorkspaceState::NoProject => ProductAction::OpenOrCreateProject,
        ProductWorkspaceState::ProjectNeedsSetup => ProductAction::CompleteSetup,
        ProductWorkspaceState::PreflightNeedsInput => ProductAction::AnswerFounderQuestions,
        ProductWorkspaceState::PlanningReady => ProductAction::PreviewPlanningChanges,
        ProductWorkspaceState::TaskReadyForReview => ProductAction::ReviewSelectedTask,
        ProductWorkspaceState::AwaitingConfirmation => ProductAction::ConfirmOperation,
        ProductWorkspaceState::OperationRunning => ProductAction::InspectRunningOperation,
        ProductWorkspaceState::ResultNeedsReview => ProductAction::ReviewResult,
        ProductWorkspaceState::ContinueAvailable => ProductAction::ReviewNextIteration,
        ProductWorkspaceState::Resumable => ProductAction::ResumeVerifiedGoal,
        ProductWorkspaceState::RepairRequired => ProductAction::RepairSharedState,
        ProductWorkspaceState::Blocked => ProductAction::InspectBlocker,
        ProductWorkspaceState::GoalComplete => ProductAction::FinishGoal,
    }
}

fn state_effects(state: ProductWorkspaceState) -> Vec<EffectClass> {
    match state {
        ProductWorkspaceState::NoProject
        | ProductWorkspaceState::PreflightNeedsInput
        | ProductWorkspaceState::TaskReadyForReview
        | ProductWorkspaceState::OperationRunning
        | ProductWorkspaceState::ContinueAvailable
        | ProductWorkspaceState::Resumable
        | ProductWorkspaceState::Blocked => vec![EffectClass::Inspect],
        ProductWorkspaceState::ProjectNeedsSetup | ProductWorkspaceState::PlanningReady => {
            vec![EffectClass::Inspect, EffectClass::PlanningMutation]
        }
        ProductWorkspaceState::AwaitingConfirmation => vec![
            EffectClass::Inspect,
            EffectClass::BuildMutation,
            EffectClass::ExternalShared,
        ],
        ProductWorkspaceState::ResultNeedsReview | ProductWorkspaceState::GoalComplete => {
            vec![EffectClass::Inspect, EffectClass::GitMutation]
        }
        ProductWorkspaceState::RepairRequired => {
            vec![EffectClass::Inspect, EffectClass::ExternalShared]
        }
    }
}

pub(crate) fn derive_product_workflow_projection(
    input: ProductProjectionInput,
) -> ProductWorkflowProjection {
    let state = product_state(&input);
    let mode = match input.collaboration_mode {
        Some(CollaborationMode::Viewer) => ProductMode::ViewerInspection,
        Some(CollaborationMode::SharedCollaborator) => ProductMode::SharedCollaborator,
        _ => ProductMode::LocalSolo,
    };
    let state_has_mutation = state_effects(state)
        .iter()
        .any(|effect| *effect != EffectClass::Inspect);
    let allowed_effects = state_effects(state)
        .into_iter()
        .filter(|effect| {
            if mode == ProductMode::ViewerInspection {
                *effect == EffectClass::Inspect
            } else {
                *effect != EffectClass::ExternalShared || mode == ProductMode::SharedCollaborator
            }
        })
        .collect::<Vec<_>>();
    let mutation_allowed = mode != ProductMode::ViewerInspection
        && allowed_effects
            .iter()
            .any(|effect| *effect != EffectClass::Inspect);
    let primary_action = if mode == ProductMode::ViewerInspection && state_has_mutation {
        ProductAction::InspectSharedState
    } else {
        product_action(state)
    };
    let shared =
        input
            .collaboration_mode
            .zip(input.reconciliation)
            .map(|(mode, reconciliation)| ProductSharedProjection {
                mode,
                access: input.collaboration_access,
                reconciliation,
            });

    ProductWorkflowProjection {
        state,
        mode,
        projection_source: if input.goal_loop_state.is_some() || input.recovery_state.is_some() {
            ProjectionAuthority::GoalReceipt
        } else if input.project_selected {
            ProjectionAuthority::Repository
        } else {
            ProjectionAuthority::Application
        },
        primary_action,
        allowed_effects,
        mutation_allowed,
        explicit_confirmation_required: mutation_allowed,
        automatic_execution_started: false,
        local: ProductLocalProjection {
            loop_state: input.goal_loop_state,
            recovery_state: input.recovery_state,
        },
        shared,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TargetOperation {
    Create,
    Update,
    Execute,
    Stage,
    Commit,
    Publish,
    Repair,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EffectTarget {
    path: String,
    operation: TargetOperation,
    summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum BaselineKind {
    Absent,
    ContentVersion,
    GitFingerprint,
    RemoteVersion,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExpectedBaseline {
    target: String,
    kind: BaselineKind,
    value: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OneUseConfirmation {
    confirmation_id: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    one_use: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MutationPlan {
    plan_id: String,
    effect_class: EffectClass,
    targets: Vec<EffectTarget>,
    baselines: Vec<ExpectedBaseline>,
    effects: Vec<String>,
    confirmation: OneUseConfirmation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MutationReceiptStatus {
    Applied,
    Partial,
    Failed,
    Cancelled,
    Stale,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MutationReceipt {
    plan_id: String,
    status: MutationReceiptStatus,
    committed_targets: Vec<String>,
    failed_targets: Vec<String>,
    evidence: Vec<String>,
    repository_verified: bool,
    remote_authority_advanced: bool,
}

fn is_lower_hex(value: &str, prefix: &str, count: usize) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == count
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn validate_safe_value(value: &Value, field: &str, depth: usize) -> Result<(), String> {
    if depth > 12 {
        return Err("product contract is too deeply nested".into());
    }
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
        Value::String(value) => {
            let lower = value.to_ascii_lowercase();
            let forbidden = [
                "authorization",
                "bearer ",
                "access_token",
                "refresh_token",
                "capability",
                "provider payload",
                "secret",
            ];
            if value.len() > 1024
                || value.chars().any(char::is_control)
                || lower.contains("://")
                || forbidden.iter().any(|marker| lower.contains(marker))
            {
                return Err(format!(
                    "product contract field {field} contains forbidden content"
                ));
            }
            Ok(())
        }
        Value::Array(values) => {
            if values.len() > 128 {
                return Err(format!("product contract field {field} is oversized"));
            }
            for value in values {
                validate_safe_value(value, field, depth + 1)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for (key, value) in values {
                let lower = key.to_ascii_lowercase();
                if lower.contains("authorization")
                    || lower.contains("capability")
                    || lower.contains("header")
                    || lower.contains("providerpayload")
                    || lower.contains("secret")
                    || lower == "url"
                {
                    return Err(format!("product contract contains forbidden field {key}"));
                }
                validate_safe_value(value, key, depth + 1)?;
            }
            Ok(())
        }
    }
}

impl MutationPlan {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !is_lower_hex(&self.plan_id, "product-plan-", 32) {
            return Err("mutation plan ID is invalid".into());
        }
        if !is_lower_hex(
            &self.confirmation.confirmation_id,
            "product-confirmation-",
            32,
        ) {
            return Err("mutation confirmation ID is invalid".into());
        }
        let lifetime = self
            .confirmation
            .expires_at_unix_ms
            .checked_sub(self.confirmation.issued_at_unix_ms)
            .ok_or_else(|| "mutation confirmation lifetime is invalid".to_string())?;
        if !self.confirmation.one_use || lifetime == 0 || lifetime > 15 * 60 * 1000 {
            return Err("mutation confirmation lifetime is invalid".into());
        }
        if self.targets.is_empty() || self.targets.len() > 64 {
            return Err("mutation targets are invalid".into());
        }
        if self.effects.is_empty() || self.effects.len() > 64 {
            return Err("mutation effect summary is invalid".into());
        }
        if !matches!(
            self.effect_class,
            EffectClass::PlanningMutation
                | EffectClass::BuildMutation
                | EffectClass::GitMutation
                | EffectClass::ExternalShared
        ) {
            return Err("mutation plan requires a mutating effect class".into());
        }
        let mut targets = HashSet::new();
        for target in &self.targets {
            let path = Path::new(&target.path);
            if target.path.is_empty()
                || target.path.len() > 512
                || path.is_absolute()
                || path.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
                || !targets.insert(target.path.as_str())
            {
                return Err("mutation target is invalid".into());
            }
            if self.effect_class == EffectClass::PlanningMutation
                && matches!(
                    target.operation,
                    TargetOperation::Publish | TargetOperation::Repair
                )
            {
                return Err("planning mutation cannot include shared effects".into());
            }
        }
        let baseline_targets = self
            .baselines
            .iter()
            .map(|baseline| baseline.target.as_str())
            .collect::<HashSet<_>>();
        if self.baselines.len() != self.targets.len()
            || baseline_targets.len() != targets.len()
            || self
                .baselines
                .iter()
                .any(|baseline| !targets.contains(baseline.target.as_str()))
        {
            return Err("every mutation target requires one matching baseline".into());
        }
        let serialized = serde_json::to_value(self).map_err(|error| error.to_string())?;
        validate_safe_value(&serialized, "contract", 0)
    }

    pub(crate) fn validate_receipt(&self, receipt: &MutationReceipt) -> Result<(), String> {
        if receipt.plan_id != self.plan_id || receipt.remote_authority_advanced {
            return Err("mutation receipt does not match the local plan contract".into());
        }
        let planned = self
            .targets
            .iter()
            .map(|target| target.path.as_str())
            .collect::<HashSet<_>>();
        let reported = receipt
            .committed_targets
            .iter()
            .chain(&receipt.failed_targets)
            .map(String::as_str)
            .collect::<Vec<_>>();
        let unique = reported.iter().copied().collect::<HashSet<_>>();
        if unique.len() != reported.len() || reported.iter().any(|target| !planned.contains(target))
        {
            return Err("mutation receipt contains an unplanned or duplicate target".into());
        }
        let all_reported = reported.len() == planned.len();
        match receipt.status {
            MutationReceiptStatus::Applied
                if !receipt.failed_targets.is_empty()
                    || receipt.committed_targets.len() != planned.len() =>
            {
                return Err("applied receipt does not account for every target".into());
            }
            MutationReceiptStatus::Partial
                if !all_reported
                    || receipt.committed_targets.is_empty()
                    || receipt.failed_targets.is_empty() =>
            {
                return Err("partial receipt must report committed and failed targets".into());
            }
            MutationReceiptStatus::Failed | MutationReceiptStatus::Stale
                if !all_reported || !receipt.committed_targets.is_empty() =>
            {
                return Err(
                    "failed or stale receipt must account for every target without claiming a commit"
                        .into(),
                );
            }
            _ => {}
        }
        let serialized = serde_json::to_value(receipt).map_err(|error| error.to_string())?;
        validate_safe_value(&serialized, "receipt", 0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProductFailureClass {
    Repository,
    Contract,
    Helper,
    Runtime,
    Git,
    NetworkPolicy,
    Collaboration,
    Cancellation,
    StaleState,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FailureEvidence {
    source: String,
    code: String,
    summary: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProductFailure {
    failure_class: ProductFailureClass,
    code: String,
    message: String,
    evidence: Vec<FailureEvidence>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RepairAction {
    RefreshRepository,
    InspectContract,
    RepairHelper,
    ReauthenticateRuntime,
    InspectRuntimeDiagnostics,
    ResolveGitState,
    OpenLocalNetworkSettings,
    InspectNetworkPolicy,
    ReconnectCollaboration,
    RetryAfterCancellation,
    RefreshStaleState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RepairConfidence {
    Evidence,
    Hypothesis,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepairGuidance {
    action: RepairAction,
    confidence: RepairConfidence,
    message: &'static str,
}

pub(crate) fn select_repair_guidance(failure: &ProductFailure) -> RepairGuidance {
    let guidance = |action, confidence, message| RepairGuidance {
        action,
        confidence,
        message,
    };
    match failure.failure_class {
        ProductFailureClass::Repository => guidance(
            RepairAction::RefreshRepository,
            RepairConfidence::Evidence,
            "Refresh repository authority and inspect the reported path or verification failure.",
        ),
        ProductFailureClass::Contract => guidance(
            RepairAction::InspectContract,
            RepairConfidence::Evidence,
            "Inspect the rejected contract field and regenerate a valid bounded preview.",
        ),
        ProductFailureClass::Helper => guidance(
            RepairAction::RepairHelper,
            RepairConfidence::Evidence,
            "Inspect helper provenance, availability, and structured output before retrying.",
        ),
        ProductFailureClass::Runtime if failure.code == "authenticationRequired" => guidance(
            RepairAction::ReauthenticateRuntime,
            RepairConfidence::Evidence,
            "Restore runtime authentication, then create a fresh confirmation.",
        ),
        ProductFailureClass::Runtime => guidance(
            RepairAction::InspectRuntimeDiagnostics,
            RepairConfidence::Evidence,
            "Inspect the bounded runtime evidence without assuming a network-policy cause.",
        ),
        ProductFailureClass::Git => guidance(
            RepairAction::ResolveGitState,
            RepairConfidence::Evidence,
            "Refresh and resolve the reported Git baseline before creating a new mutation preview.",
        ),
        ProductFailureClass::NetworkPolicy
            if failure
                .evidence
                .iter()
                .any(|evidence| evidence.code == "localNetworkDenied") =>
        {
            guidance(
                RepairAction::OpenLocalNetworkSettings,
                RepairConfidence::Evidence,
                "Local Network access was denied by the operating system. Review the app permission before retrying.",
            )
        }
        ProductFailureClass::NetworkPolicy => guidance(
            RepairAction::InspectNetworkPolicy,
            RepairConfidence::Hypothesis,
            "Network policy may be involved; inspect system and runtime evidence before changing permissions.",
        ),
        ProductFailureClass::Collaboration => guidance(
            RepairAction::ReconnectCollaboration,
            RepairConfidence::Evidence,
            "Reconnect the matching collaboration session and follow the existing typed repair contract.",
        ),
        ProductFailureClass::Cancellation => guidance(
            RepairAction::RetryAfterCancellation,
            RepairConfidence::Evidence,
            "The operation was cancelled. Review partial evidence before creating a fresh preview.",
        ),
        ProductFailureClass::StaleState => guidance(
            RepairAction::RefreshStaleState,
            RepairConfidence::Evidence,
            "Refresh every bound baseline and create a new one-use confirmation.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection_input() -> ProductProjectionInput {
        ProductProjectionInput {
            project_selected: true,
            project_needs_setup: false,
            founder_input_required: false,
            planning_ready: false,
            selected_task_ready: true,
            operation_running: false,
            result_needs_review: false,
            goal_loop_state: Some(GoalLoopState::AwaitingConfirmation),
            recovery_state: None,
            collaboration_mode: Some(CollaborationMode::LocalOnly),
            collaboration_access: None,
            reconciliation: Some(ReconciliationState::LocalOnly),
        }
    }

    #[test]
    fn projection_composes_existing_local_and_shared_contracts() {
        let projection = derive_product_workflow_projection(projection_input());
        let serialized = serde_json::to_value(projection).unwrap();
        assert_eq!(serialized["state"], "awaitingConfirmation");
        assert_eq!(serialized["mode"], "localSolo");
        assert_eq!(serialized["projectionSource"], "goalReceipt");
        assert_eq!(serialized["primaryAction"], "confirmOperation");
        assert_eq!(
            serialized["allowedEffects"],
            serde_json::json!(["inspect", "buildMutation"])
        );
        assert_eq!(serialized["automaticExecutionStarted"], false);
        assert_eq!(serialized["local"]["loopState"], "awaitingConfirmation");
        assert_eq!(serialized["shared"]["reconciliation"], "localOnly");
    }

    #[test]
    fn viewer_projection_is_inspection_only() {
        let mut input = projection_input();
        input.collaboration_mode = Some(CollaborationMode::Viewer);
        input.collaboration_access = Some(CollaborationAccess::Viewer);
        input.reconciliation = Some(ReconciliationState::RepairRequired);
        let projection = derive_product_workflow_projection(input);
        assert_eq!(projection.state, ProductWorkspaceState::RepairRequired);
        assert_eq!(projection.mode, ProductMode::ViewerInspection);
        assert_eq!(projection.primary_action, ProductAction::InspectSharedState);
        assert_eq!(projection.allowed_effects, vec![EffectClass::Inspect]);
        assert!(!projection.mutation_allowed);
        assert!(!projection.explicit_confirmation_required);
    }

    #[test]
    fn founder_input_is_more_specific_than_generic_setup() {
        let mut input = projection_input();
        input.project_needs_setup = true;
        input.founder_input_required = true;
        input.goal_loop_state = None;
        input.selected_task_ready = false;
        let projection = derive_product_workflow_projection(input);
        assert_eq!(projection.state, ProductWorkspaceState::PreflightNeedsInput);
        assert_eq!(
            projection.primary_action,
            ProductAction::AnswerFounderQuestions
        );
        assert!(!projection.mutation_allowed);
    }

    #[test]
    fn active_operation_is_dominant_over_shared_repair_context() {
        let mut input = projection_input();
        input.operation_running = true;
        input.collaboration_mode = Some(CollaborationMode::SharedCollaborator);
        input.collaboration_access = Some(CollaborationAccess::Collaborator);
        input.reconciliation = Some(ReconciliationState::RepairRequired);
        let projection = derive_product_workflow_projection(input);
        assert_eq!(projection.state, ProductWorkspaceState::OperationRunning);
        assert_eq!(
            projection.primary_action,
            ProductAction::InspectRunningOperation
        );
        assert!(!projection.mutation_allowed);
        assert!(!projection.automatic_execution_started);
        assert_eq!(
            projection.shared.unwrap().reconciliation,
            ReconciliationState::RepairRequired
        );
    }

    #[test]
    fn typescript_compatible_plan_fixture_is_strict_and_secret_free() {
        let fixture = format!(
            r#"{{
              "planId":"product-plan-{plan}",
              "effectClass":"planningMutation",
              "targets":[{{"path":"tasks/issues/023-create.md","operation":"create","summary":"Create one task"}}],
              "baselines":[{{"target":"tasks/issues/023-create.md","kind":"absent","value":null}}],
              "effects":["Create one reviewed task file"],
              "confirmation":{{
                "confirmationId":"product-confirmation-{confirmation}",
                "issuedAtUnixMs":1000,
                "expiresAtUnixMs":61000,
                "oneUse":true
              }}
            }}"#,
            plan = "1".repeat(32),
            confirmation = "2".repeat(32),
        );
        let plan: MutationPlan = serde_json::from_str(&fixture).unwrap();
        plan.validate().unwrap();
        let value = serde_json::to_value(plan).unwrap();
        assert_eq!(value["effectClass"], "planningMutation");
        assert_eq!(value["confirmation"]["oneUse"], true);
    }

    #[test]
    fn receipt_accounts_for_every_planned_target_and_never_advances_remote_authority() {
        let fixture = format!(
            r#"{{
              "planId":"product-plan-{plan}",
              "effectClass":"planningMutation",
              "targets":[{{"path":"tasks/issues/023-create.md","operation":"create","summary":"Create one task"}}],
              "baselines":[{{"target":"tasks/issues/023-create.md","kind":"absent","value":null}}],
              "effects":["Create one reviewed task file"],
              "confirmation":{{
                "confirmationId":"product-confirmation-{confirmation}",
                "issuedAtUnixMs":1000,
                "expiresAtUnixMs":61000,
                "oneUse":true
              }}
            }}"#,
            plan = "1".repeat(32),
            confirmation = "2".repeat(32),
        );
        let plan: MutationPlan = serde_json::from_str(&fixture).unwrap();
        let receipt = MutationReceipt {
            plan_id: plan.plan_id.clone(),
            status: MutationReceiptStatus::Applied,
            committed_targets: vec!["tasks/issues/023-create.md".into()],
            failed_targets: Vec::new(),
            evidence: vec!["Repository readback matched the proposed content".into()],
            repository_verified: true,
            remote_authority_advanced: false,
        };
        plan.validate_receipt(&receipt).unwrap();

        let dishonest = MutationReceipt {
            committed_targets: Vec::new(),
            ..receipt
        };
        assert!(plan.validate_receipt(&dishonest).is_err());
    }

    #[test]
    fn planning_never_contains_shared_effects_or_unbound_baselines() {
        let fixture = format!(
            r#"{{
              "planId":"product-plan-{plan}",
              "effectClass":"planningMutation",
              "targets":[{{"path":"tasks/issues/023-create.md","operation":"publish","summary":"Publish task"}}],
              "baselines":[],
              "effects":["Publish task"],
              "confirmation":{{
                "confirmationId":"product-confirmation-{confirmation}",
                "issuedAtUnixMs":1000,
                "expiresAtUnixMs":61000,
                "oneUse":true
              }}
            }}"#,
            plan = "1".repeat(32),
            confirmation = "2".repeat(32),
        );
        let plan: MutationPlan = serde_json::from_str(&fixture).unwrap();
        assert!(plan.validate().is_err());
    }

    #[test]
    fn local_network_settings_require_matching_typed_evidence() {
        let unknown = ProductFailure {
            failure_class: ProductFailureClass::NetworkPolicy,
            code: "connectivityUnknown".into(),
            message: "Connection failed".into(),
            evidence: vec![FailureEvidence {
                source: "system".into(),
                code: "connectivityUnknown".into(),
                summary: "Cause is unknown".into(),
            }],
        };
        let guidance = select_repair_guidance(&unknown);
        assert_eq!(guidance.action, RepairAction::InspectNetworkPolicy);
        assert_eq!(guidance.confidence, RepairConfidence::Hypothesis);

        let denied = ProductFailure {
            evidence: vec![FailureEvidence {
                source: "system".into(),
                code: "localNetworkDenied".into(),
                summary: "OS permission result".into(),
            }],
            ..unknown
        };
        let guidance = select_repair_guidance(&denied);
        assert_eq!(guidance.action, RepairAction::OpenLocalNetworkSettings);
        assert_eq!(guidance.confidence, RepairConfidence::Evidence);
    }

    #[test]
    fn serialized_contract_rejects_capability_urls_and_provider_material() {
        for unsafe_value in [
            serde_json::json!({"authorizationHeader":"redacted"}),
            serde_json::json!({"note":"provider payload follows"}),
            serde_json::json!({"path":"https://host.invalid/workspace?edit=opaque"}),
            serde_json::json!({"capability":"opaque"}),
        ] {
            assert!(validate_safe_value(&unsafe_value, "contract", 0).is_err());
        }
    }
}
