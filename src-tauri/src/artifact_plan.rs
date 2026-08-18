use super::{
    git_fingerprint, inspect_project_path, native_runtime_run_id, operation_registry, sha256_bytes,
    validated_repository_root, write_project_file_serialized, GitFingerprint, OperationKind,
    ProjectError, ProjectSnapshot,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};

const MAX_TARGETS: usize = 24;
const MAX_CONTENT_BYTES: usize = 128 * 1024;
const MAX_PLAN_BYTES: usize = 512 * 1024;
const MAX_ACTIVE_PLANS: usize = 64;
const PLAN_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ArtifactDraft {
    path: String,
    content: String,
    #[serde(default)]
    expected_version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactPlanTarget {
    path: String,
    content: String,
    content_version: String,
    diff: String,
    effect: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactPlanPreview {
    root: String,
    targets: Vec<ArtifactPlanTarget>,
    baseline: GitFingerprint,
    preview_token: String,
    expires_at_ms: u128,
    explicit_confirmation_required: bool,
    effect_class: String,
    collaboration_effects: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactApplyResult {
    success: bool,
    committed_paths: Vec<String>,
    already_committed_paths: Vec<String>,
    unapplied_paths: Vec<String>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    project: ProjectSnapshot,
    collaboration_effects: Vec<String>,
}

#[derive(Clone)]
struct StoredPlan {
    root: PathBuf,
    targets: Vec<ArtifactDraft>,
    baseline: GitFingerprint,
    expires_at: SystemTime,
}

#[derive(Default)]
pub(crate) struct ArtifactPlanStore {
    plans: Mutex<HashMap<String, StoredPlan>>,
}

impl ArtifactPlanStore {
    fn issue(&self, token: String, plan: StoredPlan) -> Result<(), ProjectError> {
        let mut plans = self.plans.lock().map_err(|_| {
            ProjectError::new(
                "artifact_plan_lock_failed",
                "Artifact plan registry is poisoned",
                Some(&plan.root),
            )
        })?;
        plans.retain(|_, stored| stored.expires_at > SystemTime::now());
        if plans.len() >= MAX_ACTIVE_PLANS {
            return Err(ProjectError::new(
                "artifact_plan_capacity",
                "Too many artifact previews are active; wait for one to expire",
                Some(&plan.root),
            ));
        }
        plans.insert(token, plan);
        Ok(())
    }

    fn consume(&self, root: &Path, token: &str) -> Result<StoredPlan, ProjectError> {
        let mut plans = self.plans.lock().map_err(|_| {
            ProjectError::new(
                "artifact_plan_lock_failed",
                "Artifact plan registry is poisoned",
                Some(root),
            )
        })?;
        let plan = plans.remove(token).ok_or_else(|| {
            ProjectError::new(
                "artifact_confirmation_invalid",
                "Artifact confirmation is missing, expired, or already consumed",
                Some(root),
            )
        })?;
        if plan.root != root {
            return Err(ProjectError::new(
                "artifact_confirmation_mismatch",
                "Artifact confirmation belongs to another repository",
                Some(root),
            ));
        }
        if plan.expires_at <= SystemTime::now() {
            return Err(ProjectError::new(
                "artifact_confirmation_expired",
                "Artifact confirmation expired; preview the plan again",
                Some(root),
            ));
        }
        Ok(plan)
    }
}

fn allowlisted_path(path: &str) -> bool {
    matches!(
        path,
        "AGENTS.md"
            | "docs/source-index.md"
            | "docs/mvp-scope.md"
            | "docs/blueprint-status.md"
            | "docs/decision-log.md"
            | "docs/conflicts.md"
            | "docs/execution-rules.md"
            | "docs/release-gates.md"
            | "docs/raw/founder-interview.md"
            | "docs/evidence/preflight.md"
            | "tasks/post-release-backlog.md"
            | "tasks/sprint-0.md"
            | "tasks/sprint-1.md"
            | "tasks/sprint-2.md"
            | "tasks/sprint-3.md"
    ) || (path.starts_with("tasks/issues/")
        && path.ends_with(".md")
        && path.strip_prefix("tasks/issues/").is_some_and(|name| {
            let Some((id, slug)) = name.trim_end_matches(".md").split_once('-') else {
                return false;
            };
            id.len() == 3
                && id.bytes().all(|byte| byte.is_ascii_digit())
                && !slug.is_empty()
                && slug
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        }))
}

fn validate_target(root: &Path, draft: &ArtifactDraft) -> Result<(), ProjectError> {
    if draft.path.is_empty()
        || Path::new(&draft.path).is_absolute()
        || Path::new(&draft.path).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || !allowlisted_path(&draft.path)
    {
        return Err(ProjectError::new(
            "artifact_path_not_allowed",
            "Artifact path is not an allowlisted repository Markdown target",
            Some(Path::new(&draft.path)),
        ));
    }
    if draft.content.is_empty()
        || draft.content.len() > MAX_CONTENT_BYTES
        || draft.content.contains('\0')
    {
        return Err(ProjectError::new(
            "artifact_content_invalid",
            "Artifact content must be non-empty and within the per-file bound",
            Some(Path::new(&draft.path)),
        ));
    }
    let mut cursor = root.to_path_buf();
    let components = Path::new(&draft.path).components().collect::<Vec<_>>();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(ProjectError::new(
                    "artifact_path_untrusted",
                    "Artifact parent must be a real directory inside the repository",
                    Some(&cursor),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ProjectError::new(
                    "artifact_path_inspection_failed",
                    error.to_string(),
                    Some(&cursor),
                ));
            }
        }
    }
    Ok(())
}

fn validate_drafts(root: &Path, drafts: &[ArtifactDraft]) -> Result<(), ProjectError> {
    if drafts.is_empty() || drafts.len() > MAX_TARGETS {
        return Err(ProjectError::new(
            "artifact_plan_size_invalid",
            "Artifact plan must contain between 1 and 24 targets",
            Some(root),
        ));
    }
    let total = drafts
        .iter()
        .map(|draft| draft.content.len())
        .sum::<usize>();
    if total > MAX_PLAN_BYTES {
        return Err(ProjectError::new(
            "artifact_plan_oversized",
            "Artifact plan exceeds the aggregate content bound",
            Some(root),
        ));
    }
    let mut paths = HashSet::new();
    for draft in drafts {
        validate_target(root, draft)?;
        if !paths.insert(draft.path.as_str()) {
            return Err(ProjectError::new(
                "artifact_target_duplicate",
                "Artifact plan contains a duplicate target",
                Some(Path::new(&draft.path)),
            ));
        }
        let target = root.join(&draft.path);
        match fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(ProjectError::new(
                    "artifact_target_untrusted",
                    "Existing artifact target must be a regular file",
                    Some(&target),
                ));
            }
            Ok(_) => {
                let existing = fs::read(&target).map_err(|error| {
                    ProjectError::new(
                        "artifact_target_read_failed",
                        error.to_string(),
                        Some(&target),
                    )
                })?;
                if existing == draft.content.as_bytes() {
                    continue;
                }
                let Some(expected) = draft.expected_version.as_deref() else {
                    return Err(ProjectError::new(
                        "artifact_existing_file_conflict",
                        "Artifact plan cannot overwrite an existing file",
                        Some(&target),
                    ));
                };
                if sha256_bytes(&existing) != expected {
                    return Err(ProjectError::new(
                        "artifact_expected_version_stale",
                        "Existing planning artifact no longer matches its expected version",
                        Some(&target),
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if draft.expected_version.is_some() {
                    return Err(ProjectError::new(
                        "artifact_expected_target_missing",
                        "Expected planning artifact is missing",
                        Some(&target),
                    ));
                }
            }
            Err(error) => {
                return Err(ProjectError::new(
                    "artifact_target_inspection_failed",
                    error.to_string(),
                    Some(&target),
                ));
            }
        }
    }
    Ok(())
}

fn preview_token(
    root: &Path,
    baseline: &GitFingerprint,
    drafts: &[ArtifactDraft],
) -> Result<String, ProjectError> {
    let mut digest = Sha256::new();
    digest.update(b"pax-workbench-artifact-plan-v1\0");
    digest.update(root.to_string_lossy().as_bytes());
    digest.update(serde_json::to_vec(baseline).map_err(|error| {
        ProjectError::new("artifact_plan_encode_failed", error.to_string(), Some(root))
    })?);
    digest.update(serde_json::to_vec(drafts).map_err(|error| {
        ProjectError::new("artifact_plan_encode_failed", error.to_string(), Some(root))
    })?);
    digest.update(native_runtime_run_id()?.as_bytes());
    Ok(format!("artifact-plan:{:x}", digest.finalize()))
}

#[tauri::command]
pub(crate) fn preview_artifact_plan(
    root: String,
    targets: Vec<ArtifactDraft>,
    store: tauri::State<'_, ArtifactPlanStore>,
) -> Result<ArtifactPlanPreview, ProjectError> {
    let root = validated_repository_root(&root)?;
    let _lease = operation_registry().begin(&root, OperationKind::ArtifactPlan, None)?;
    validate_drafts(&root, &targets)?;
    let baseline = git_fingerprint(&root)?;
    let token = preview_token(&root, &baseline, &targets)?;
    let expires_at = SystemTime::now() + PLAN_TTL;
    store.issue(
        token.clone(),
        StoredPlan {
            root: root.clone(),
            targets: targets.clone(),
            baseline: baseline.clone(),
            expires_at,
        },
    )?;
    let targets = targets
        .into_iter()
        .map(|draft| {
            let existing = fs::read(root.join(&draft.path)).ok();
            let effect = match existing.as_deref() {
                Some(bytes) if bytes == draft.content.as_bytes() => "alreadyCommitted",
                Some(_) => "update",
                None => "create",
            };
            let diff = match existing {
                Some(bytes) if bytes != draft.content.as_bytes() => {
                    let before = String::from_utf8_lossy(&bytes);
                    before
                        .lines()
                        .map(|line| format!("-{line}"))
                        .chain(draft.content.lines().map(|line| format!("+{line}")))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                Some(_) => String::new(),
                None => draft
                    .content
                    .lines()
                    .map(|line| format!("+{line}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            };
            ArtifactPlanTarget {
                path: draft.path,
                content_version: sha256_bytes(draft.content.as_bytes()),
                diff,
                content: draft.content,
                effect: effect.into(),
            }
        })
        .collect();
    Ok(ArtifactPlanPreview {
        root: root.to_string_lossy().to_string(),
        targets,
        baseline,
        preview_token: token,
        expires_at_ms: expires_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        explicit_confirmation_required: true,
        effect_class: "planMutation".into(),
        collaboration_effects: Vec::new(),
    })
}

fn ensure_parent_directories(root: &Path, relative: &str) -> std::io::Result<()> {
    let mut cursor = root.to_path_buf();
    if let Some(parent) = Path::new(relative).parent() {
        for component in parent.components() {
            cursor.push(component.as_os_str());
            match fs::create_dir(&cursor) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let metadata = fs::symlink_metadata(&cursor)?;
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "artifact parent is not a trusted directory",
                        ));
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}

fn create_artifact(root: &Path, draft: &ArtifactDraft) -> std::io::Result<()> {
    ensure_parent_directories(root, &draft.path)?;
    let target = root.join(&draft.path);
    let temp = target.with_extension(format!("md.pax-artifact-{}", uuid::Uuid::new_v4().simple()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(draft.content.as_bytes())?;
    file.sync_all()?;
    match fs::hard_link(&temp, &target) {
        Ok(()) => {
            fs::remove_file(&temp)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

fn update_artifact(root: &Path, draft: &ArtifactDraft) -> std::io::Result<()> {
    let expected = draft.expected_version.as_deref().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "artifact update requires an expected version",
        )
    })?;
    write_project_file_serialized(root, &draft.path, &draft.content, expected, |_| {})
        .map(|_| ())
        .map_err(|error| std::io::Error::other(format!("{}: {}", error.code, error.message)))
}

fn apply_plan_with<F>(
    root: &Path,
    plan: &StoredPlan,
    mut create: F,
) -> Result<ArtifactApplyResult, ProjectError>
where
    F: FnMut(&Path, &ArtifactDraft) -> std::io::Result<()>,
{
    validate_drafts(root, &plan.targets)?;
    let current = git_fingerprint(root)?;
    if current != plan.baseline {
        return Err(ProjectError::new(
            "artifact_plan_stale",
            "Repository baseline changed; preview the artifact plan again",
            Some(root),
        ));
    }
    let mut committed = Vec::new();
    let mut already_committed = Vec::new();
    for (index, draft) in plan.targets.iter().enumerate() {
        let target = root.join(&draft.path);
        if target.exists() && fs::read(&target).ok().as_deref() == Some(draft.content.as_bytes()) {
            already_committed.push(draft.path.clone());
            continue;
        }
        if let Err(error) = create(root, draft) {
            return Ok(ArtifactApplyResult {
                success: false,
                committed_paths: committed,
                already_committed_paths: already_committed,
                unapplied_paths: plan.targets[index..]
                    .iter()
                    .map(|target| target.path.clone())
                    .collect(),
                failure_code: Some("artifact_partial_apply".into()),
                failure_message: Some(error.to_string()),
                project: inspect_project_path(root),
                collaboration_effects: Vec::new(),
            });
        }
        committed.push(draft.path.clone());
    }
    Ok(ArtifactApplyResult {
        success: true,
        committed_paths: committed,
        already_committed_paths: already_committed,
        unapplied_paths: Vec::new(),
        failure_code: None,
        failure_message: None,
        project: inspect_project_path(root),
        collaboration_effects: Vec::new(),
    })
}

#[tauri::command]
pub(crate) fn apply_artifact_plan(
    root: String,
    preview_token: String,
    confirmed: bool,
    store: tauri::State<'_, ArtifactPlanStore>,
) -> Result<ArtifactApplyResult, ProjectError> {
    if !confirmed {
        return Err(ProjectError::new(
            "artifact_confirmation_required",
            "Artifact creation requires explicit confirmation",
            Some(Path::new(&root)),
        ));
    }
    let root = validated_repository_root(&root)?;
    let _lease = operation_registry().begin(&root, OperationKind::ArtifactPlan, None)?;
    let plan = store.consume(&root, &preview_token)?;
    apply_plan_with(&root, &plan, |root, draft| {
        if draft.expected_version.is_some() {
            update_artifact(root, draft)
        } else {
            create_artifact(root, draft)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn init_repo(root: &Path) {
        assert!(std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .unwrap()
            .success());
    }

    fn draft(path: &str, content: &str) -> ArtifactDraft {
        ArtifactDraft {
            path: path.into(),
            content: content.into(),
            expected_version: None,
        }
    }

    #[test]
    fn rejects_traversal_duplicates_oversize_and_existing_overwrite() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        assert!(validate_drafts(project.path(), &[draft("../AGENTS.md", "x")]).is_err());
        assert!(validate_drafts(
            project.path(),
            &[draft("AGENTS.md", "x"), draft("AGENTS.md", "x")]
        )
        .is_err());
        assert!(validate_drafts(
            project.path(),
            &[draft("AGENTS.md", &"x".repeat(MAX_CONTENT_BYTES + 1))]
        )
        .is_err());
        fs::write(project.path().join("AGENTS.md"), "old").unwrap();
        assert!(validate_drafts(project.path(), &[draft("AGENTS.md", "new")]).is_err());
    }

    #[test]
    fn planning_update_requires_exact_version_and_preserves_create_only_default() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let root = fs::canonicalize(project.path()).unwrap();
        fs::create_dir_all(root.join("tasks")).unwrap();
        let target = root.join("tasks/sprint-3.md");
        fs::write(&target, "# Sprint 3\n").unwrap();
        let expected = sha256_bytes(b"# Sprint 3\n");
        let update = ArtifactDraft {
            path: "tasks/sprint-3.md".into(),
            content: "# Sprint 3\n\n| 026 | Planned | ready |\n".into(),
            expected_version: Some(expected),
        };
        assert!(validate_drafts(&root, std::slice::from_ref(&update)).is_ok());
        let plan = StoredPlan {
            root: root.clone(),
            targets: vec![update.clone()],
            baseline: git_fingerprint(&root).unwrap(),
            expires_at: SystemTime::now() + PLAN_TTL,
        };
        let result =
            apply_plan_with(&root, &plan, |root, draft| update_artifact(root, draft)).unwrap();
        assert!(
            result.success,
            "planning update failed: {:?}",
            result.failure_message
        );
        assert!(fs::read_to_string(&target).unwrap().contains("Planned"));

        let mut stale = update;
        stale.content.push_str("changed\n");
        assert_eq!(
            validate_drafts(&root, &[stale]).unwrap_err().code,
            "artifact_expected_version_stale"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_parent_before_any_write() {
        use std::os::unix::fs::symlink;
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        init_repo(project.path());
        symlink(outside.path(), project.path().join("docs")).unwrap();
        assert!(validate_drafts(project.path(), &[draft("docs/mvp-scope.md", "# Scope")]).is_err());
        assert!(!outside.path().join("mvp-scope.md").exists());
    }

    #[test]
    fn partial_failure_reports_exact_committed_and_unapplied_paths() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let targets = vec![
            draft("AGENTS.md", "# Rules"),
            draft("docs/mvp-scope.md", "# Scope"),
        ];
        let plan = StoredPlan {
            root: project.path().to_path_buf(),
            targets,
            baseline: git_fingerprint(project.path()).unwrap(),
            expires_at: SystemTime::now() + PLAN_TTL,
        };
        let calls = AtomicUsize::new(0);
        let result = apply_plan_with(project.path(), &plan, |root, draft| {
            if calls.fetch_add(1, Ordering::AcqRel) == 1 {
                return Err(std::io::Error::other("injected failure"));
            }
            create_artifact(root, draft)
        })
        .unwrap();
        assert!(!result.success);
        assert_eq!(result.committed_paths, ["AGENTS.md"]);
        assert_eq!(result.unapplied_paths, ["docs/mvp-scope.md"]);
        assert_eq!(
            result.failure_code.as_deref(),
            Some("artifact_partial_apply")
        );
    }

    #[test]
    fn successful_apply_creates_every_target_and_stale_baseline_writes_nothing() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let targets = vec![
            draft("AGENTS.md", "# Rules"),
            draft("docs/mvp-scope.md", "# Scope"),
        ];
        let plan = StoredPlan {
            root: project.path().to_path_buf(),
            targets: targets.clone(),
            baseline: git_fingerprint(project.path()).unwrap(),
            expires_at: SystemTime::now() + PLAN_TTL,
        };
        let result = apply_plan_with(project.path(), &plan, create_artifact).unwrap();
        assert!(result.success);
        assert_eq!(result.committed_paths, ["AGENTS.md", "docs/mvp-scope.md"]);
        assert_eq!(
            fs::read_to_string(project.path().join("docs/mvp-scope.md")).unwrap(),
            "# Scope"
        );

        let stale = StoredPlan {
            root: project.path().to_path_buf(),
            targets: vec![draft("tasks/sprint-0.md", "# Sprint")],
            baseline: git_fingerprint(project.path()).unwrap(),
            expires_at: SystemTime::now() + PLAN_TTL,
        };
        fs::write(project.path().join("untracked.txt"), "drift").unwrap();
        assert!(apply_plan_with(project.path(), &stale, create_artifact).is_err());
        assert!(!project.path().join("tasks/sprint-0.md").exists());
    }

    #[test]
    fn exact_existing_content_is_an_idempotent_retry_input() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        fs::write(project.path().join("AGENTS.md"), "# Rules").unwrap();
        validate_drafts(project.path(), &[draft("AGENTS.md", "# Rules")]).unwrap();
    }

    #[test]
    fn confirmation_is_repository_bound_expiring_and_one_use() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        init_repo(first.path());
        init_repo(second.path());
        let store = ArtifactPlanStore::default();
        let plan = StoredPlan {
            root: first.path().to_path_buf(),
            targets: vec![draft("AGENTS.md", "# Rules")],
            baseline: git_fingerprint(first.path()).unwrap(),
            expires_at: SystemTime::now() + PLAN_TTL,
        };
        store.issue("one-use".into(), plan.clone()).unwrap();
        assert!(store.consume(second.path(), "one-use").is_err());
        store.issue("one-use".into(), plan).unwrap();
        assert!(store.consume(first.path(), "one-use").is_ok());
        assert!(store.consume(first.path(), "one-use").is_err());

        let expired = StoredPlan {
            root: first.path().to_path_buf(),
            targets: vec![draft("AGENTS.md", "# Rules")],
            baseline: git_fingerprint(first.path()).unwrap(),
            expires_at: SystemTime::UNIX_EPOCH,
        };
        store.issue("expired".into(), expired).unwrap();
        assert!(store.consume(first.path(), "expired").is_err());
    }

    #[test]
    fn artifact_plan_uses_the_shared_operation_linearization_point() {
        let project = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let registry = operation_registry();
        let lease = registry
            .begin(project.path(), OperationKind::ArtifactPlan, None)
            .unwrap();
        assert!(registry
            .begin(project.path(), OperationKind::Runtime, Some("run".into()))
            .is_err());
        drop(lease);
        assert!(registry
            .begin(project.path(), OperationKind::Helper, None)
            .is_ok());
    }
}
