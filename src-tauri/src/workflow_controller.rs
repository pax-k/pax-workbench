//! Pure bounded-controller policy. This module has no Tauri/WebView state and
//! performs no repository, runtime, goal-storage, or collaboration effect.

use serde_json::Value;

pub(crate) const EXPECTED_EFFECTS: [&str; 3] = [
    "Codex may edit files inside the selected repository",
    "Codex may run task-scoped verification commands",
    "Repository files, Git state, task evidence, and resolver state will be reread after exit",
];

pub(crate) const STOP_CONDITIONS: [&str; 9] = [
    "founder-owned decision required",
    "external state required",
    "open material conflict",
    "runtime or verification failure",
    "stale repository or task state",
    "user cancellation",
    "no ready AI-owned task",
    "invalid resolver state",
    "repository-affirmed goal completion",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolverStopKind {
    Founder,
    External,
    Conflict,
    NoReadyTask,
    InvalidState,
}

pub(crate) fn resolver_stop_kind(decision: &str, blocking_gates: &[String]) -> ResolverStopKind {
    if blocking_gates
        .iter()
        .any(|gate| gate.to_ascii_lowercase().contains("conflict"))
    {
        return ResolverStopKind::Conflict;
    }
    match decision {
        "ask-founder" => ResolverStopKind::Founder,
        "wait-external" => ResolverStopKind::External,
        "no-ready-task" => ResolverStopKind::NoReadyTask,
        _ => ResolverStopKind::InvalidState,
    }
}

pub(crate) fn resolver_selected_task(stdout: &str) -> Option<String> {
    let value: Value = serde_json::from_str(stdout).ok()?;
    let task = value.get("nextTask")?.as_object()?;
    let path = task.get("path")?.as_str()?;
    let status = task.get("status")?.as_str()?;
    let owner = task.get("owner")?.as_str()?;
    let missing = task.get("missingContractFields")?.as_array()?;
    if status != "ready" || !owner.eq_ignore_ascii_case("ai") || !missing.is_empty() {
        return None;
    }
    Some(path.into())
}

/// Effect ports required by a bounded controller implementation. Concrete
/// command adapters provide repository, helper, runtime, and persistence.
#[allow(dead_code)]
pub(crate) trait ControllerPorts {
    type Error;
    type Snapshot;
    type HelperResult;
    type RuntimeResult;

    fn refresh_repository(&mut self) -> Result<Self::Snapshot, Self::Error>;
    fn run_resolver(&mut self) -> Result<Self::HelperResult, Self::Error>;
    fn run_once(&mut self) -> Result<Self::RuntimeResult, Self::Error>;
    fn persist_checkpoint(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_gate_dominates_the_resolver_decision() {
        assert_eq!(
            resolver_stop_kind("ask-founder", &["Open conflict exists".into()]),
            ResolverStopKind::Conflict
        );
    }

    #[test]
    fn selected_task_requires_ready_ai_ownership_and_complete_contract() {
        let valid = r#"{"nextTask":{"path":"tasks/issues/023-safe.md","status":"ready","owner":"AI","missingContractFields":[]}}"#;
        assert_eq!(
            resolver_selected_task(valid).as_deref(),
            Some("tasks/issues/023-safe.md")
        );
        for invalid in [
            r#"{"nextTask":{"path":"tasks/issues/x.md","status":"planned","owner":"AI","missingContractFields":[]}}"#,
            r#"{"nextTask":{"path":"tasks/issues/x.md","status":"ready","owner":"founder","missingContractFields":[]}}"#,
            r#"{"nextTask":{"path":"tasks/issues/x.md","status":"ready","owner":"AI","missingContractFields":["Goal"]}}"#,
        ] {
            assert_eq!(resolver_selected_task(invalid), None);
        }
    }

    #[test]
    fn policy_declares_bounded_effect_and_terminal_stop_contracts() {
        assert_eq!(EXPECTED_EFFECTS.len(), 3);
        assert!(STOP_CONDITIONS.contains(&"runtime or verification failure"));
        assert!(STOP_CONDITIONS.contains(&"repository-affirmed goal completion"));
    }
}
