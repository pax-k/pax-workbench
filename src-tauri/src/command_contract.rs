//! Stable serialized command surface. The Tauri registration in `lib.rs` is a
//! thin adapter over these names; changing this list requires an explicit
//! compatibility decision and bridge update.

pub(crate) const NATIVE_COMMAND_NAMES: [&str; 31] = [
    "inspect_project",
    "inspect_post_run_review",
    "preview_local_git_handoff",
    "apply_local_git_handoff",
    "read_project_file",
    "write_project_file",
    "preview_skill_setup",
    "execute_skill_setup",
    "cancel_skill_setup",
    "execute_helper",
    "cancel_helper",
    "preview_bounded_task",
    "preview_shared_bounded_task",
    "recover_goal_state",
    "clear_goal_state",
    "execute_bounded_task",
    "execute_shared_bounded_task",
    "repair_collaboration_completion",
    "cancel_bounded_task",
    "execute_runtime",
    "cancel_runtime",
    "connect_mdsync_session",
    "disconnect_mdsync_session",
    "list_mdsync_files",
    "read_mdsync_file",
    "write_mdsync_file",
    "preview_ha2ha_publish",
    "apply_ha2ha_publish",
    "join_ha2ha_workspace",
    "preview_artifact_plan",
    "apply_artifact_plan",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_contract_is_unique_and_keeps_compatibility_critical_surfaces() {
        let mut names = NATIVE_COMMAND_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), NATIVE_COMMAND_NAMES.len());
        for required in [
            "inspect_project",
            "write_project_file",
            "execute_bounded_task",
            "execute_shared_bounded_task",
            "repair_collaboration_completion",
        ] {
            assert!(NATIVE_COMMAND_NAMES.contains(&required));
        }
    }

    #[test]
    fn tauri_registration_matches_the_closed_command_contract() {
        let source = include_str!("lib.rs");
        let registration = source
            .split(".invoke_handler(tauri::generate_handler![")
            .nth(1)
            .and_then(|tail| tail.split("])").next())
            .expect("Tauri command registration must remain explicit");
        let registered = registration
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| {
                if name == "inspect_project_command" {
                    "inspect_project"
                } else {
                    name
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(registered, NATIVE_COMMAND_NAMES);
    }
}
