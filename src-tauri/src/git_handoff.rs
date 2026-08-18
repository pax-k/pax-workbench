//! Explicit, local-only Git handoff boundary for reviewed post-run paths.
//! The surface deliberately supports only path-scoped staging followed by one
//! new local commit. Remote and destructive Git operations do not exist here.

use super::{
    git_fingerprint, inspect_project_path, operation_registry, repository_identity,
    validated_repository_root, GitFingerprint, OperationKind, ProjectError, ProjectSnapshot,
    RepositoryIdentity,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    process::{Command, Output},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_RECEIPT_PATHS: usize = 200;
const MAX_SELECTED_PATHS: usize = 200;
const MAX_MESSAGE_BYTES: usize = 512;
const MAX_SCAN_BYTES: u64 = 1024 * 1024;
const MAX_ACTIVE_PREVIEWS: usize = 64;
const PREVIEW_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHandoffStatus {
    path: String,
    status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHandoffCandidate {
    path: String,
    status: String,
    staged_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHandoffExclusion {
    path: String,
    status: String,
    code: String,
    reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalGitHandoffPreview {
    root: String,
    repository: RepositoryIdentity,
    baseline: GitFingerprint,
    current_status: Vec<GitHandoffStatus>,
    candidates: Vec<GitHandoffCandidate>,
    exclusions: Vec<GitHandoffExclusion>,
    selected_paths: Vec<String>,
    proposed_message: String,
    staged_effects: Vec<String>,
    preview_token: Option<String>,
    expires_at_ms: Option<u128>,
    explicit_confirmation_required: bool,
    pre_existing_index: bool,
    remote_effects: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHandoffRepair {
    code: String,
    message: String,
    next_action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalGitHandoffResult {
    success: bool,
    outcome: String,
    commit_created: bool,
    previous_head: String,
    new_head: Option<String>,
    selected_paths: Vec<String>,
    staged_paths: Vec<String>,
    committed_paths: Vec<String>,
    message: String,
    repair: Option<GitHandoffRepair>,
    project: ProjectSnapshot,
    remote_effects: Vec<String>,
}

#[derive(Clone)]
struct StoredPreview {
    root: PathBuf,
    repository: RepositoryIdentity,
    baseline: GitFingerprint,
    selected_paths: Vec<String>,
    selected_content: HashMap<String, String>,
    message: String,
    expires_at: SystemTime,
}

#[derive(Default)]
pub(crate) struct LocalGitHandoffStore {
    previews: Mutex<HashMap<String, StoredPreview>>,
}

impl LocalGitHandoffStore {
    fn issue(&self, token: String, preview: StoredPreview) -> Result<(), ProjectError> {
        let mut previews = self.previews.lock().map_err(|_| {
            ProjectError::new(
                "git_handoff_preview_lock_failed",
                "Local Git handoff preview registry is poisoned",
                Some(&preview.root),
            )
        })?;
        previews.retain(|_, stored| stored.expires_at > SystemTime::now());
        if previews.len() >= MAX_ACTIVE_PREVIEWS {
            return Err(ProjectError::new(
                "git_handoff_preview_capacity",
                "Too many local Git handoff previews are active",
                Some(&preview.root),
            ));
        }
        previews.insert(token, preview);
        Ok(())
    }

    fn consume(&self, root: &Path, token: &str) -> Result<StoredPreview, ProjectError> {
        let mut previews = self.previews.lock().map_err(|_| {
            ProjectError::new(
                "git_handoff_preview_lock_failed",
                "Local Git handoff preview registry is poisoned",
                Some(root),
            )
        })?;
        let preview = previews.remove(token).ok_or_else(|| {
            ProjectError::new(
                "git_handoff_confirmation_invalid",
                "Local Git handoff confirmation is missing, expired, or already consumed",
                Some(root),
            )
        })?;
        if preview.root != root {
            return Err(ProjectError::new(
                "git_handoff_confirmation_mismatch",
                "Local Git handoff confirmation belongs to another repository",
                Some(root),
            ));
        }
        if preview.expires_at <= SystemTime::now() {
            return Err(ProjectError::new(
                "git_handoff_confirmation_expired",
                "Local Git handoff confirmation expired; preview again",
                Some(root),
            ));
        }
        Ok(preview)
    }
}

#[derive(Clone, Debug)]
struct StatusEntry {
    path: String,
    status: String,
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.chars().any(char::is_control)
        && !Path::new(value).is_absolute()
        && !Path::new(value).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn git(root: &Path, args: &[&str]) -> Result<Output, ProjectError> {
    git_command(root).args(args).output().map_err(|error| {
        ProjectError::new(
            "git_handoff_git_unavailable",
            format!("Cannot start Git: {error}"),
            Some(root),
        )
    })
}

fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
    ] {
        command.env_remove(variable);
    }
    command
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "gc.auto=0",
            "-c",
            "maintenance.auto=false",
        ])
        .current_dir(root);
    command
}

fn successful_git(root: &Path, args: &[&str], code: &str) -> Result<Output, ProjectError> {
    let output = git(root, args)?;
    if output.status.success() {
        Ok(output)
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(ProjectError::new(
            code,
            if detail.is_empty() {
                "Git command failed".into()
            } else {
                detail
            },
            Some(root),
        ))
    }
}

fn status_entries(root: &Path) -> Result<Vec<StatusEntry>, ProjectError> {
    let output = successful_git(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        "git_handoff_status_failed",
    )?;
    let mut fields = output.stdout.split(|byte| *byte == 0);
    let mut entries = Vec::new();
    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        if field.len() < 4 || field[2] != b' ' {
            return Err(ProjectError::new(
                "git_handoff_status_invalid",
                "Git returned malformed porcelain status",
                Some(root),
            ));
        }
        let status = String::from_utf8_lossy(&field[..2]).to_string();
        let path = std::str::from_utf8(&field[3..]).map_err(|_| {
            ProjectError::new(
                "git_handoff_path_invalid",
                "Git returned a non-UTF-8 path",
                Some(root),
            )
        })?;
        if !safe_relative_path(path) {
            return Err(ProjectError::new(
                "git_handoff_path_invalid",
                "Git returned an unsafe repository-relative path",
                Some(root),
            ));
        }
        if status.contains('R') || status.contains('C') {
            let source = fields.next().ok_or_else(|| {
                ProjectError::new(
                    "git_handoff_status_invalid",
                    "Git omitted a rename/copy source path",
                    Some(root),
                )
            })?;
            let source = std::str::from_utf8(source).map_err(|_| {
                ProjectError::new(
                    "git_handoff_path_invalid",
                    "Git returned a non-UTF-8 rename/copy source",
                    Some(root),
                )
            })?;
            if !safe_relative_path(source) {
                return Err(ProjectError::new(
                    "git_handoff_path_invalid",
                    "Git returned an unsafe rename/copy source",
                    Some(root),
                ));
            }
        }
        entries.push(StatusEntry {
            path: path.into(),
            status,
        });
    }
    if entries.len() > MAX_RECEIPT_PATHS {
        return Err(ProjectError::new(
            "git_handoff_status_oversized",
            "Current Git status exceeds the bounded 200-path handoff surface",
            Some(root),
        ));
    }
    Ok(entries)
}

fn index_is_clean(root: &Path) -> Result<bool, ProjectError> {
    let output = git(root, &["diff", "--cached", "--quiet", "--exit-code"])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(ProjectError::new(
            "git_handoff_index_inspection_failed",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            Some(root),
        )),
    }
}

fn contains_capability_material(bytes: &[u8]) -> bool {
    fn has_secret_assignment(line: &str, marker: &str) -> bool {
        let Some((_, tail)) = line.split_once(marker) else {
            return false;
        };
        let value = tail.trim_start();
        let value = value
            .split(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | '&' | ';' | ')' | ']' | '}')
            })
            .next()
            .unwrap_or_default();
        value.len() >= 8
            && !value.starts_with('<')
            && !value.starts_with('[')
            && !value.contains("redacted")
            && !value.contains("example")
            && value
                .chars()
                .any(|character| character.is_ascii_alphanumeric())
    }
    if bytes.contains(&0) {
        return true;
    }
    String::from_utf8_lossy(bytes).lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("-----begin private key-----")
            || lower.contains("-----begin rsa private key-----")
            || (lower.contains("authorization:")
                && (lower.contains("bearer ") || lower.contains("basic ")))
            || [
                "api_key=",
                "apikey=",
                "access_token=",
                "refresh_token=",
                "client_secret=",
                "password=",
            ]
            .iter()
            .any(|marker| has_secret_assignment(&lower, marker))
            || ((lower.contains("http://") || lower.contains("https://"))
                && (lower.contains("?token=")
                    || lower.contains("?key=")
                    || lower.contains("?capability=")))
    })
}

fn is_submodule(root: &Path, path: &str) -> Result<bool, ProjectError> {
    let output = successful_git(
        root,
        &["ls-files", "--stage", "--", path],
        "git_handoff_submodule_inspection_failed",
    )?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.starts_with("160000 ")))
}

fn exclusion_for(
    root: &Path,
    entry: &StatusEntry,
    in_receipt: bool,
) -> Result<Option<GitHandoffExclusion>, ProjectError> {
    let excluded = |code: &str, reason: &str| GitHandoffExclusion {
        path: entry.path.clone(),
        status: entry.status.clone(),
        code: code.into(),
        reason: reason.into(),
    };
    if !in_receipt {
        return Ok(Some(excluded(
            "notInReviewReceipt",
            "Current dirty path was not present in the supplied review receipt",
        )));
    }
    let status = entry.status.as_bytes();
    if entry.status.contains('R') || entry.status.contains('C') {
        return Ok(Some(excluded(
            "renameOrCopy",
            "Rename/copy staging is outside this narrow handoff boundary",
        )));
    }
    if entry.status.contains('U')
        || matches!(
            entry.status.as_str(),
            "DD" | "AA" | "AU" | "UA" | "DU" | "UD"
        )
    {
        return Ok(Some(excluded(
            "conflict",
            "Conflict resolution is outside this handoff boundary",
        )));
    }
    if status
        .first()
        .is_some_and(|value| *value != b' ' && *value != b'?')
    {
        return Ok(Some(excluded(
            "preStaged",
            "Path is already present in the Git index",
        )));
    }
    if is_submodule(root, &entry.path)? {
        return Ok(Some(excluded(
            "submodule",
            "Submodule mutations are outside this handoff boundary",
        )));
    }
    let target = root.join(&entry.path);
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(excluded(
                "missing",
                "Only existing review-receipt files can be committed here",
            )));
        }
        Err(error) => {
            return Err(ProjectError::new(
                "git_handoff_path_inspection_failed",
                error.to_string(),
                Some(&target),
            ));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(Some(excluded(
            "untrustedFileType",
            "Selected path must be a non-symlink regular file",
        )));
    }
    let canonical = fs::canonicalize(&target).map_err(|error| {
        ProjectError::new(
            "git_handoff_path_inspection_failed",
            error.to_string(),
            Some(&target),
        )
    })?;
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        ProjectError::new(
            "git_handoff_path_inspection_failed",
            error.to_string(),
            Some(root),
        )
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Ok(Some(excluded(
            "outsideRepository",
            "Selected path resolves outside the repository",
        )));
    }
    if metadata.len() > MAX_SCAN_BYTES {
        return Ok(Some(excluded(
            "capabilityScanUnavailable",
            "File exceeds the bounded capability scan limit",
        )));
    }
    let bytes = fs::read(&target).map_err(|error| {
        ProjectError::new(
            "git_handoff_path_read_failed",
            error.to_string(),
            Some(&target),
        )
    })?;
    if contains_capability_material(&bytes) {
        return Ok(Some(excluded(
            "capabilityMaterial",
            "File is binary or contains capability-like material",
        )));
    }
    Ok(None)
}

fn validate_receipt_paths(paths: &[String]) -> Result<HashSet<&str>, ProjectError> {
    if paths.len() > MAX_RECEIPT_PATHS {
        return Err(ProjectError::new(
            "git_handoff_receipt_oversized",
            "Review receipt exceeds the bounded 200-path handoff surface",
            None,
        ));
    }
    let mut unique = HashSet::new();
    for path in paths {
        if !safe_relative_path(path) || !unique.insert(path.as_str()) {
            return Err(ProjectError::new(
                "git_handoff_receipt_invalid",
                "Review receipt paths must be unique safe repository-relative paths",
                Some(Path::new(path)),
            ));
        }
    }
    Ok(unique)
}

fn validate_message(message: &str) -> Result<(), ProjectError> {
    if message.is_empty()
        || message.len() > MAX_MESSAGE_BYTES
        || message.trim() != message
        || message.chars().any(|character| character.is_control())
    {
        return Err(ProjectError::new(
            "git_handoff_message_invalid",
            "Commit message must be one trimmed printable line of at most 512 bytes",
            None,
        ));
    }
    Ok(())
}

fn preview_token(
    root: &Path,
    repository: &RepositoryIdentity,
    baseline: &GitFingerprint,
    paths: &[String],
    message: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"pax-workbench-local-git-handoff-v1\0");
    digest.update(root.to_string_lossy().as_bytes());
    digest.update(b"\0");
    digest.update(repository.repository_id.as_bytes());
    digest.update(b"\0");
    digest.update(baseline.head.as_bytes());
    digest.update(b"\0");
    digest.update(baseline.index.as_bytes());
    digest.update(b"\0");
    digest.update(baseline.worktree.as_bytes());
    for path in paths {
        digest.update(b"\0");
        digest.update(path.as_bytes());
    }
    digest.update(b"\0");
    digest.update(message.as_bytes());
    digest.update(uuid::Uuid::new_v4().as_bytes());
    format!("git-handoff:{:x}", digest.finalize())
}

fn preview_with(
    root: &Path,
    receipt_paths: Vec<String>,
    mut selected_paths: Vec<String>,
    proposed_message: String,
    store: &LocalGitHandoffStore,
) -> Result<LocalGitHandoffPreview, ProjectError> {
    let receipt = validate_receipt_paths(&receipt_paths)?;
    if selected_paths.len() > MAX_SELECTED_PATHS {
        return Err(ProjectError::new(
            "git_handoff_selection_oversized",
            "At most 200 reviewed paths may be selected",
            Some(root),
        ));
    }
    selected_paths.sort();
    if selected_paths.windows(2).any(|pair| pair[0] == pair[1])
        || selected_paths.iter().any(|path| !safe_relative_path(path))
    {
        return Err(ProjectError::new(
            "git_handoff_selection_invalid",
            "Selected paths must be unique safe repository-relative paths",
            Some(root),
        ));
    }
    let entries = status_entries(root)?;
    let pre_existing_index = !index_is_clean(root)?;
    if pre_existing_index {
        return Err(ProjectError::new(
            "git_handoff_index_not_clean",
            "The Git index already contains staged changes; unstage or commit them outside Build Right Studio, then preview again",
            Some(root),
        ));
    }
    let repository = repository_identity(root)?;
    let baseline = git_fingerprint(root)?;
    let current_paths = entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<HashSet<_>>();
    let mut candidates = Vec::new();
    let mut exclusions = Vec::new();
    for entry in &entries {
        if let Some(exclusion) = exclusion_for(root, entry, receipt.contains(entry.path.as_str()))?
        {
            exclusions.push(exclusion);
        } else {
            candidates.push(GitHandoffCandidate {
                path: entry.path.clone(),
                status: entry.status.clone(),
                staged_effect: "Stage this existing path and include it in one new local commit"
                    .into(),
            });
        }
    }
    for path in receipt
        .iter()
        .filter(|path| !current_paths.contains(**path))
    {
        exclusions.push(GitHandoffExclusion {
            path: (*path).into(),
            status: "clean".into(),
            code: "notCurrentlyChanged".into(),
            reason: "Receipt path is no longer changed in the current worktree".into(),
        });
    }
    exclusions.sort_by(|left, right| left.path.cmp(&right.path));
    let candidate_paths = candidates
        .iter()
        .map(|candidate| candidate.path.as_str())
        .collect::<HashSet<_>>();
    if let Some(path) = selected_paths
        .iter()
        .find(|path| !receipt.contains(path.as_str()) || !candidate_paths.contains(path.as_str()))
    {
        let reason = exclusions
            .iter()
            .find(|item| item.path == **path)
            .map(|item| item.reason.as_str())
            .unwrap_or("path is not present in the current eligible candidate set");
        return Err(ProjectError::new(
            "git_handoff_selection_not_eligible",
            format!("Selected path is not eligible: {reason}"),
            Some(Path::new(path)),
        ));
    }
    let (token, expires_at_ms) = if selected_paths.is_empty() {
        if !proposed_message.is_empty() {
            validate_message(&proposed_message)?;
        }
        (None, None)
    } else {
        validate_message(&proposed_message)?;
        let selected_content = selected_paths
            .iter()
            .map(|path| {
                fs::read(root.join(path))
                    .map(|bytes| (path.clone(), format!("sha256:{:x}", Sha256::digest(bytes))))
                    .map_err(|error| {
                        ProjectError::new(
                            "git_handoff_path_read_failed",
                            error.to_string(),
                            Some(&root.join(path)),
                        )
                    })
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let expires_at = SystemTime::now() + PREVIEW_TTL;
        let token = preview_token(
            root,
            &repository,
            &baseline,
            &selected_paths,
            &proposed_message,
        );
        store.issue(
            token.clone(),
            StoredPreview {
                root: root.to_path_buf(),
                repository: repository.clone(),
                baseline: baseline.clone(),
                selected_paths: selected_paths.clone(),
                selected_content,
                message: proposed_message.clone(),
                expires_at,
            },
        )?;
        (
            Some(token),
            Some(
                expires_at
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
            ),
        )
    };
    let staged_effects = selected_paths
        .iter()
        .map(|path| format!("Stage `{path}`"))
        .chain((!selected_paths.is_empty()).then(|| {
            format!(
                "Create one local commit with message `{proposed_message}`; no push or remote effect"
            )
        }))
        .collect();
    Ok(LocalGitHandoffPreview {
        root: root.to_string_lossy().to_string(),
        repository,
        baseline,
        current_status: entries
            .into_iter()
            .map(|entry| GitHandoffStatus {
                path: entry.path,
                status: entry.status,
            })
            .collect(),
        candidates,
        exclusions,
        selected_paths,
        proposed_message,
        staged_effects,
        preview_token: token,
        expires_at_ms,
        explicit_confirmation_required: true,
        pre_existing_index,
        remote_effects: Vec::new(),
    })
}

fn staged_paths(root: &Path) -> Result<Vec<String>, ProjectError> {
    let output = successful_git(
        root,
        &["diff", "--cached", "--name-only", "-z"],
        "git_handoff_staged_readback_failed",
    )?;
    let mut paths = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path).map(str::to_string).map_err(|_| {
                ProjectError::new(
                    "git_handoff_path_invalid",
                    "Git returned a non-UTF-8 staged path",
                    Some(root),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    Ok(paths)
}

fn index_mode(root: &Path, path: &str) -> Result<String, ProjectError> {
    let output = successful_git(
        root,
        &["ls-files", "--stage", "--", path],
        "git_handoff_mode_inspection_failed",
    )?;
    if let Some(mode) = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
    {
        if matches!(mode, "100644" | "100755") {
            return Ok(mode.into());
        }
        return Err(ProjectError::new(
            "git_handoff_mode_not_allowed",
            "Only regular tracked file modes can be staged",
            Some(Path::new(path)),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(root.join(path))
            .map_err(|error| {
                ProjectError::new(
                    "git_handoff_mode_inspection_failed",
                    error.to_string(),
                    Some(&root.join(path)),
                )
            })?
            .permissions()
            .mode();
        Ok(if mode & 0o111 == 0 {
            "100644".into()
        } else {
            "100755".into()
        })
    }
    #[cfg(not(unix))]
    {
        Ok("100644".into())
    }
}

fn stage_without_filters(
    root: &Path,
    paths: &[String],
    modes: &HashMap<String, String>,
) -> Result<(), String> {
    for path in paths {
        let hash = git_command(root)
            .args(["hash-object", "-w", "--no-filters", "--", path])
            .output()
            .map_err(|error| format!("Cannot start Git hash-object for `{path}`: {error}"))?;
        if !hash.status.success() {
            return Err(format!(
                "Git hash-object failed for `{path}`: {}",
                String::from_utf8_lossy(&hash.stderr).trim()
            ));
        }
        let object = String::from_utf8_lossy(&hash.stdout).trim().to_string();
        let mode = modes
            .get(path)
            .ok_or_else(|| format!("No reviewed file mode was retained for `{path}`"))?;
        let cacheinfo = format!("{mode},{object},{path}");
        let update = git_command(root)
            .args(["update-index", "--add", "--cacheinfo", &cacheinfo])
            .output()
            .map_err(|error| format!("Cannot start Git update-index for `{path}`: {error}"))?;
        if !update.status.success() {
            return Err(format!(
                "Git update-index failed for `{path}`: {}",
                String::from_utf8_lossy(&update.stderr).trim()
            ));
        }
    }
    Ok(())
}

fn failure_result(
    root: &Path,
    preview: &StoredPreview,
    outcome: &str,
    commit_created: bool,
    new_head: Option<String>,
    staged: Vec<String>,
    committed: Vec<String>,
    code: &str,
    message: String,
    next_action: &str,
) -> LocalGitHandoffResult {
    LocalGitHandoffResult {
        success: false,
        outcome: outcome.into(),
        commit_created,
        previous_head: preview.baseline.head.clone(),
        new_head,
        selected_paths: preview.selected_paths.clone(),
        staged_paths: staged,
        committed_paths: committed,
        message: preview.message.clone(),
        repair: Some(GitHandoffRepair {
            code: code.into(),
            message,
            next_action: next_action.into(),
        }),
        project: inspect_project_path(root),
        remote_effects: Vec::new(),
    }
}

fn apply_with(
    root: &Path,
    preview_token: &str,
    confirmed: bool,
    store: &LocalGitHandoffStore,
) -> Result<LocalGitHandoffResult, ProjectError> {
    if !confirmed {
        return Err(ProjectError::new(
            "git_handoff_confirmation_required",
            "Creating a local commit requires explicit confirmation",
            Some(root),
        ));
    }
    let preview = store.consume(root, preview_token)?;
    if repository_identity(root)? != preview.repository
        || git_fingerprint(root)? != preview.baseline
        || !index_is_clean(root)?
    {
        return Err(ProjectError::new(
            "git_handoff_preview_stale",
            "Repository identity, HEAD, index, or worktree changed; inspect and preview again",
            Some(root),
        ));
    }
    let entries = status_entries(root)?;
    let by_path = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<HashMap<_, _>>();
    for path in &preview.selected_paths {
        let entry = by_path.get(path.as_str()).ok_or_else(|| {
            ProjectError::new(
                "git_handoff_preview_stale",
                "A selected path is no longer changed",
                Some(Path::new(path)),
            )
        })?;
        if exclusion_for(root, entry, true)?.is_some() {
            return Err(ProjectError::new(
                "git_handoff_selection_not_eligible",
                "A selected path is no longer eligible",
                Some(Path::new(path)),
            ));
        }
    }

    let modes = preview
        .selected_paths
        .iter()
        .map(|path| index_mode(root, path).map(|mode| (path.clone(), mode)))
        .collect::<Result<HashMap<_, _>, _>>()?;
    let hooks = tempfile::Builder::new()
        .prefix("pax-workbench-empty-hooks-")
        .tempdir()
        .map_err(|error| {
            ProjectError::new(
                "git_handoff_hook_isolation_failed",
                error.to_string(),
                Some(root),
            )
        })?;
    if let Err(message) = stage_without_filters(root, &preview.selected_paths, &modes) {
        let staged = staged_paths(root).unwrap_or_default();
        return Ok(failure_result(
            root,
            &preview,
            "stageFailed",
            false,
            None,
            staged,
            Vec::new(),
            "git_handoff_stage_failed",
            message,
            "Inspect the reported index state and repair it with Git outside Build Right Studio; then create a fresh preview.",
        ));
    }
    let staged = staged_paths(root)?;
    if staged != preview.selected_paths {
        return Ok(failure_result(
            root,
            &preview,
            "stageVerificationFailed",
            false,
            None,
            staged,
            Vec::new(),
            "git_handoff_stage_readback_mismatch",
            "The staged path readback did not exactly match the confirmed selection".into(),
            "Do not commit this index. Inspect and repair it with Git outside Build Right Studio, then preview again.",
        ));
    }
    for path in &preview.selected_paths {
        let spec = format!(":{path}");
        let output = successful_git(
            root,
            &["show", &spec],
            "git_handoff_staged_content_readback_failed",
        )?;
        let actual = format!("sha256:{:x}", Sha256::digest(&output.stdout));
        if preview.selected_content.get(path) != Some(&actual) {
            return Ok(failure_result(
                root,
                &preview,
                "stageVerificationFailed",
                false,
                None,
                staged,
                Vec::new(),
                "git_handoff_staged_content_mismatch",
                format!("Staged content for `{path}` differs from the confirmed preview"),
                "Do not commit this index. Inspect and repair it with Git outside Build Right Studio, then preview again.",
            ));
        }
    }

    let hooks_config = format!("core.hooksPath={}", hooks.path().to_string_lossy());
    let commit = git_command(root)
        .args([
            "-c",
            &hooks_config,
            "-c",
            "commit.gpgSign=false",
            "commit",
            "--no-verify",
            "-m",
            &preview.message,
        ])
        .output()
        .map_err(|error| {
            ProjectError::new(
                "git_handoff_git_unavailable",
                format!("Cannot start Git commit: {error}"),
                Some(root),
            )
        })?;
    if !commit.status.success() {
        let staged = staged_paths(root).unwrap_or_else(|_| preview.selected_paths.clone());
        return Ok(failure_result(
            root,
            &preview,
            "commitFailed",
            false,
            None,
            staged,
            Vec::new(),
            "git_handoff_commit_failed",
            String::from_utf8_lossy(&commit.stderr).trim().to_string(),
            "The selected paths remain staged. Inspect the Git error and either commit them outside Build Right Studio or repair the index before a fresh preview.",
        ));
    }

    let new_head = match successful_git(
        root,
        &["rev-parse", "HEAD"],
        "git_handoff_head_readback_failed",
    ) {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(error) => {
            return Ok(failure_result(
                root,
                &preview,
                "verificationFailed",
                true,
                None,
                Vec::new(),
                Vec::new(),
                "git_handoff_head_readback_failed",
                error.message,
                "A commit may exist. Inspect local HEAD and committed paths before taking any further action.",
            ));
        }
    };
    let committed_output = successful_git(
        root,
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-z",
            "HEAD",
        ],
        "git_handoff_commit_readback_failed",
    );
    let message_output = successful_git(
        root,
        &["log", "-1", "--format=%B"],
        "git_handoff_message_readback_failed",
    );
    let mut committed = match committed_output {
        Ok(output) => output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .filter_map(|path| std::str::from_utf8(path).ok().map(str::to_string))
            .collect::<Vec<_>>(),
        Err(error) => {
            return Ok(failure_result(
                root,
                &preview,
                "verificationFailed",
                true,
                Some(new_head),
                Vec::new(),
                Vec::new(),
                "git_handoff_commit_readback_failed",
                error.message,
                "The local commit exists, but path readback failed. Inspect HEAD before further action.",
            ));
        }
    };
    committed.sort();
    let read_message = match message_output {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\r', '\n'])
            .to_string(),
        Err(error) => {
            return Ok(failure_result(
                root,
                &preview,
                "verificationFailed",
                true,
                Some(new_head),
                Vec::new(),
                committed,
                "git_handoff_message_readback_failed",
                error.message,
                "The local commit exists, but message readback failed. Inspect HEAD before further action.",
            ));
        }
    };
    if new_head == preview.baseline.head
        || committed != preview.selected_paths
        || read_message != preview.message
    {
        return Ok(failure_result(
            root,
            &preview,
            "verificationFailed",
            true,
            Some(new_head),
            Vec::new(),
            committed,
            "git_handoff_commit_readback_mismatch",
            "New HEAD, committed paths, or commit message did not exactly match the confirmed preview"
                .into(),
            "The local commit exists. Inspect HEAD before any further handoff action.",
        ));
    }
    Ok(LocalGitHandoffResult {
        success: true,
        outcome: "completed".into(),
        commit_created: true,
        previous_head: preview.baseline.head,
        new_head: Some(new_head),
        selected_paths: preview.selected_paths.clone(),
        staged_paths: Vec::new(),
        committed_paths: committed,
        message: preview.message,
        repair: None,
        project: inspect_project_path(root),
        remote_effects: Vec::new(),
    })
}

#[tauri::command]
pub(crate) fn preview_local_git_handoff(
    root: String,
    receipt_paths: Vec<String>,
    selected_paths: Vec<String>,
    proposed_message: String,
    store: tauri::State<'_, LocalGitHandoffStore>,
) -> Result<LocalGitHandoffPreview, ProjectError> {
    let root = validated_repository_root(&root)?;
    let _lease = operation_registry().begin(&root, OperationKind::GitHandoff, None)?;
    preview_with(
        &root,
        receipt_paths,
        selected_paths,
        proposed_message,
        &store,
    )
}

#[tauri::command]
pub(crate) fn apply_local_git_handoff(
    root: String,
    preview_token: String,
    confirmed: bool,
    store: tauri::State<'_, LocalGitHandoffStore>,
) -> Result<LocalGitHandoffResult, ProjectError> {
    let root = validated_repository_root(&root)?;
    let _lease = operation_registry().begin(&root, OperationKind::GitHandoff, None)?;
    apply_with(&root, &preview_token, confirmed, &store)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        run(root.path(), &["init", "-q"]);
        run(root.path(), &["config", "user.name", "Build Right Test"]);
        run(
            root.path(),
            &["config", "user.email", "build-right@example.invalid"],
        );
        fs::write(root.path().join("selected.txt"), "before\n").unwrap();
        fs::write(root.path().join("unrelated.txt"), "before\n").unwrap();
        run(root.path(), &["add", "selected.txt", "unrelated.txt"]);
        run(root.path(), &["commit", "-q", "-m", "initial"]);
        root
    }

    #[test]
    fn commits_only_confirmed_receipt_paths_and_leaves_unrelated_dirty() {
        let root = repository();
        fs::write(root.path().join("selected.txt"), "after\n").unwrap();
        fs::write(root.path().join("unrelated.txt"), "also after\n").unwrap();
        let store = LocalGitHandoffStore::default();
        let preview = preview_with(
            root.path(),
            vec!["selected.txt".into(), "unrelated.txt".into()],
            vec!["selected.txt".into()],
            "Commit reviewed result".into(),
            &store,
        )
        .unwrap();
        assert_eq!(preview.selected_paths, ["selected.txt"]);
        assert!(preview.preview_token.is_some());
        let result = apply_with(
            root.path(),
            preview.preview_token.as_deref().unwrap(),
            true,
            &store,
        )
        .unwrap();
        assert!(result.success);
        assert_eq!(result.committed_paths, ["selected.txt"]);
        assert!(String::from_utf8_lossy(
            &successful_git(root.path(), &["status", "--porcelain"], "test")
                .unwrap()
                .stdout
        )
        .contains("unrelated.txt"));
        assert!(staged_paths(root.path()).unwrap().is_empty());
        assert!(apply_with(
            root.path(),
            preview.preview_token.as_deref().unwrap(),
            true,
            &store
        )
        .unwrap_err()
        .message
        .contains("already consumed"));
    }

    #[test]
    fn stale_baseline_and_pre_existing_index_stop_before_mutation() {
        let root = repository();
        fs::write(root.path().join("selected.txt"), "after\n").unwrap();
        let store = LocalGitHandoffStore::default();
        let preview = preview_with(
            root.path(),
            vec!["selected.txt".into()],
            vec!["selected.txt".into()],
            "Commit reviewed result".into(),
            &store,
        )
        .unwrap();
        fs::write(root.path().join("selected.txt"), "changed again\n").unwrap();
        let error = apply_with(
            root.path(),
            preview.preview_token.as_deref().unwrap(),
            true,
            &store,
        )
        .unwrap_err();
        assert_eq!(error.code, "git_handoff_preview_stale");
        assert!(staged_paths(root.path()).unwrap().is_empty());

        run(root.path(), &["add", "selected.txt"]);
        let error = preview_with(
            root.path(),
            vec!["selected.txt".into()],
            Vec::new(),
            String::new(),
            &store,
        )
        .unwrap_err();
        assert_eq!(error.code, "git_handoff_index_not_clean");
    }

    #[cfg(unix)]
    #[test]
    fn inspection_excludes_symlinks_capabilities_and_non_receipt_paths() {
        use std::os::unix::fs::symlink;
        let root = repository();
        fs::write(root.path().join("selected.txt"), "after\n").unwrap();
        fs::write(
            root.path().join("secret.txt"),
            "Authorization: Bearer opaque-value\n",
        )
        .unwrap();
        symlink("/tmp/outside", root.path().join("link.txt")).unwrap();
        let store = LocalGitHandoffStore::default();
        let preview = preview_with(
            root.path(),
            vec!["secret.txt".into(), "link.txt".into()],
            Vec::new(),
            String::new(),
            &store,
        )
        .unwrap();
        assert!(preview
            .exclusions
            .iter()
            .any(|item| item.path == "secret.txt" && item.code == "capabilityMaterial"));
        assert!(preview
            .exclusions
            .iter()
            .any(|item| item.path == "link.txt" && item.code == "untrustedFileType"));
        assert!(preview
            .exclusions
            .iter()
            .any(|item| item.path == "selected.txt" && item.code == "notInReviewReceipt"));
        assert!(preview.preview_token.is_none());
    }

    #[test]
    fn rejects_unconfirmed_apply_and_invalid_message_without_issuing_a_token() {
        let root = repository();
        fs::write(root.path().join("selected.txt"), "after\n").unwrap();
        let store = LocalGitHandoffStore::default();
        let preview = preview_with(
            root.path(),
            vec!["selected.txt".into()],
            vec!["selected.txt".into()],
            "Commit reviewed result".into(),
            &store,
        )
        .unwrap();
        assert_eq!(
            apply_with(
                root.path(),
                preview.preview_token.as_deref().unwrap(),
                false,
                &store
            )
            .unwrap_err()
            .code,
            "git_handoff_confirmation_required"
        );
        assert!(preview_with(
            root.path(),
            vec!["selected.txt".into()],
            vec!["selected.txt".into()],
            " bad\nmessage ".into(),
            &store,
        )
        .is_err());
    }

    #[test]
    fn commit_failure_is_truthful_and_retains_only_selected_staging() {
        let root = repository();
        fs::write(root.path().join("selected.txt"), "after\n").unwrap();
        let store = LocalGitHandoffStore::default();
        let preview = preview_with(
            root.path(),
            vec!["selected.txt".into()],
            vec!["selected.txt".into()],
            "Commit reviewed result".into(),
            &store,
        )
        .unwrap();
        let git_dir =
            successful_git(root.path(), &["rev-parse", "--absolute-git-dir"], "test").unwrap();
        let git_dir = PathBuf::from(String::from_utf8_lossy(&git_dir.stdout).trim());
        let head_ref = successful_git(root.path(), &["symbolic-ref", "HEAD"], "test").unwrap();
        let head_ref = String::from_utf8_lossy(&head_ref.stdout).trim().to_string();
        let lock = git_dir.join(format!("{head_ref}.lock"));
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(lock, b"held by test").unwrap();
        let result = apply_with(
            root.path(),
            preview.preview_token.as_deref().unwrap(),
            true,
            &store,
        )
        .unwrap();
        assert!(!result.success);
        assert_eq!(result.outcome, "commitFailed");
        assert!(!result.commit_created);
        assert_eq!(result.staged_paths, ["selected.txt"]);
        assert_eq!(
            result.repair.as_ref().unwrap().code,
            "git_handoff_commit_failed"
        );
    }

    #[test]
    fn staged_blob_mismatch_stops_before_commit_with_explicit_repair() {
        let root = repository();
        fs::write(root.path().join("selected.txt"), "after\n").unwrap();
        let store = LocalGitHandoffStore::default();
        let baseline = git_fingerprint(root.path()).unwrap();
        let repository = repository_identity(root.path()).unwrap();
        store
            .issue(
                "git-handoff:test-mismatch".into(),
                StoredPreview {
                    root: root.path().to_path_buf(),
                    repository,
                    baseline,
                    selected_paths: vec!["selected.txt".into()],
                    selected_content: HashMap::from([(
                        "selected.txt".into(),
                        "sha256:not-the-previewed-content".into(),
                    )]),
                    message: "Commit reviewed result".into(),
                    expires_at: SystemTime::now() + PREVIEW_TTL,
                },
            )
            .unwrap();
        let result = apply_with(root.path(), "git-handoff:test-mismatch", true, &store).unwrap();
        assert!(!result.success);
        assert_eq!(result.outcome, "stageVerificationFailed");
        assert_eq!(result.staged_paths, ["selected.txt"]);
        assert!(result.new_head.is_none());
        assert_eq!(
            result.repair.as_ref().unwrap().code,
            "git_handoff_staged_content_mismatch"
        );
    }

    #[test]
    fn missing_conflict_and_gitlink_paths_are_explicitly_excluded() {
        let root = repository();
        fs::remove_file(root.path().join("selected.txt")).unwrap();
        let missing = StatusEntry {
            path: "selected.txt".into(),
            status: " D".into(),
        };
        assert_eq!(
            exclusion_for(root.path(), &missing, true)
                .unwrap()
                .unwrap()
                .code,
            "missing"
        );
        let conflict = StatusEntry {
            path: "conflict.txt".into(),
            status: "UU".into(),
        };
        assert_eq!(
            exclusion_for(root.path(), &conflict, true)
                .unwrap()
                .unwrap()
                .code,
            "conflict"
        );

        let head = successful_git(root.path(), &["rev-parse", "HEAD"], "test").unwrap();
        let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
        run(
            root.path(),
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{head},module"),
            ],
        );
        assert!(is_submodule(root.path(), "module").unwrap());
    }
}
