//! Read-only, bounded Git evidence for the founder-facing post-run receipt.
//! This is a current-worktree inspection, not a claim that every change was
//! produced by the just-finished controller invocation.

use crate::repository_service::{GitReadFailure, GitReadPort};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path},
};

const MAX_CHANGED_FILES: usize = 200;
const MAX_DIFF_BYTES_PER_FILE: usize = 64 * 1024;
const MAX_DIFF_BYTES_TOTAL: usize = 256 * 1024;
const MAX_UNTRACKED_FILE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewChangedFile {
    pub(crate) path: String,
    pub(crate) status: String,
    pub(crate) diff: Option<String>,
    pub(crate) diff_unavailable_reason: Option<String>,
    pub(crate) truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PostRunReviewEvidence {
    pub(crate) scope_note: String,
    pub(crate) changed_files: Vec<ReviewChangedFile>,
    pub(crate) truncated: bool,
}

#[derive(Debug)]
pub(crate) enum ReviewEvidenceFailure {
    Git(GitReadFailure),
    InvalidStatus(String),
    Filesystem(String),
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

fn sanitized_text(bytes: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    text = text
        .chars()
        .filter(|character| *character == '\n' || *character == '\t' || !character.is_control())
        .collect();
    text.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            let sensitive = [
                "authorization:",
                "bearer ",
                "password",
                "api_key",
                "apikey",
                "access_token",
                "refresh_token",
                "client_secret",
                "capability",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
                || ((lower.contains("http://") || lower.contains("https://"))
                    && lower.contains('?'));
            if sensitive {
                let diff_prefix = line
                    .chars()
                    .next()
                    .filter(|character| matches!(character, '+' | '-' | ' '));
                format!(
                    "{}[REDACTED sensitive line]",
                    diff_prefix.map_or("", |character| match character {
                        '+' => "+",
                        '-' => "-",
                        ' ' => " ",
                        _ => "",
                    })
                )
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_status(bytes: &[u8]) -> Result<Vec<(String, String, bool)>, ReviewEvidenceFailure> {
    let mut fields = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    let mut result = Vec::new();
    while let Some(field) = fields.next() {
        if field.len() < 4 || field[2] != b' ' {
            return Err(ReviewEvidenceFailure::InvalidStatus(
                "Git returned malformed porcelain status".into(),
            ));
        }
        let code = String::from_utf8_lossy(&field[..2]).to_string();
        let path = String::from_utf8_lossy(&field[3..]).to_string();
        if !safe_relative_path(&path) {
            return Err(ReviewEvidenceFailure::InvalidStatus(
                "Git returned an unsafe changed path".into(),
            ));
        }
        let renamed = code.contains('R') || code.contains('C');
        if renamed {
            let source = fields.next().ok_or_else(|| {
                ReviewEvidenceFailure::InvalidStatus(
                    "Git omitted the source path for a rename or copy".into(),
                )
            })?;
            if !safe_relative_path(&String::from_utf8_lossy(source)) {
                return Err(ReviewEvidenceFailure::InvalidStatus(
                    "Git returned an unsafe rename source".into(),
                ));
            }
        }
        result.push((code.clone(), path, code == "??"));
    }
    Ok(result)
}

fn tracked_diff<P: GitReadPort>(
    port: &P,
    root: &Path,
    path: &str,
) -> Result<Vec<u8>, ReviewEvidenceFailure> {
    port.output(
        root,
        &[
            "diff",
            "HEAD",
            "--no-ext-diff",
            "--no-color",
            "--binary",
            "--",
            path,
        ],
    )
    .map(|output| output.stdout)
    .map_err(ReviewEvidenceFailure::Git)
}

fn untracked_diff(root: &Path, path: &str) -> Result<Vec<u8>, ReviewEvidenceFailure> {
    let target = root.join(path);
    let metadata = fs::symlink_metadata(&target)
        .map_err(|error| ReviewEvidenceFailure::Filesystem(error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ReviewEvidenceFailure::Filesystem(
            "Untracked path is not a regular file".into(),
        ));
    }
    if metadata.len() > MAX_UNTRACKED_FILE_BYTES {
        return Err(ReviewEvidenceFailure::Filesystem(
            "Untracked file exceeds the bounded textual diff limit".into(),
        ));
    }
    let bytes =
        fs::read(&target).map_err(|error| ReviewEvidenceFailure::Filesystem(error.to_string()))?;
    if bytes.contains(&0) {
        return Err(ReviewEvidenceFailure::Filesystem(
            "Binary diff is unavailable".into(),
        ));
    }
    let mut diff = format!("--- /dev/null\n+++ b/{path}\n").into_bytes();
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        diff.push(b'+');
        diff.extend_from_slice(line);
    }
    Ok(diff)
}

pub(crate) fn inspect_post_run_review_with<P: GitReadPort>(
    port: &P,
    root: &Path,
) -> Result<PostRunReviewEvidence, ReviewEvidenceFailure> {
    let status = port
        .output(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )
        .map_err(ReviewEvidenceFailure::Git)?;
    let entries = parse_status(&status.stdout)?;
    let mut remaining = MAX_DIFF_BYTES_TOTAL;
    let truncated = entries.len() > MAX_CHANGED_FILES;
    let mut changed_files = Vec::new();
    for (status, path, untracked) in entries.into_iter().take(MAX_CHANGED_FILES) {
        let raw = if untracked {
            untracked_diff(root, &path)
        } else {
            tracked_diff(port, root, &path)
        };
        let (diff, reason, file_truncated) = match raw {
            Ok(raw)
                if raw
                    .windows(b"GIT binary patch".len())
                    .any(|window| window == b"GIT binary patch")
                    || raw
                        .windows(b"Binary files".len())
                        .any(|window| window == b"Binary files") =>
            {
                (None, Some("Binary diff is unavailable".into()), false)
            }
            Ok(raw) => {
                let allowed = raw.len().min(MAX_DIFF_BYTES_PER_FILE).min(remaining);
                if allowed == 0 {
                    (
                        None,
                        Some("Aggregate textual diff limit reached".into()),
                        true,
                    )
                } else {
                    remaining -= allowed;
                    (
                        Some(sanitized_text(&raw[..allowed])),
                        None,
                        allowed < raw.len(),
                    )
                }
            }
            Err(ReviewEvidenceFailure::Filesystem(reason)) => (None, Some(reason), false),
            Err(ReviewEvidenceFailure::Git(error)) => (
                None,
                Some(format!("Git diff unavailable: {}", error.detail)),
                false,
            ),
            Err(ReviewEvidenceFailure::InvalidStatus(reason)) => (None, Some(reason), false),
        };
        changed_files.push(ReviewChangedFile {
            path,
            status,
            diff,
            diff_unavailable_reason: reason,
            truncated: file_truncated,
        });
    }
    Ok(PostRunReviewEvidence {
        scope_note: "Current Git working tree after the run; it may include pre-existing changes and does not attribute authorship to Codex.".into(),
        changed_files,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_service::GitReadFailureKind;
    use std::{
        collections::VecDeque,
        os::unix::process::ExitStatusExt,
        process::{ExitStatus, Output},
        sync::Mutex,
    };
    use tempfile::tempdir;

    struct FakeGit {
        outputs: Mutex<VecDeque<Result<Output, GitReadFailure>>>,
    }

    impl GitReadPort for FakeGit {
        fn output(&self, _root: &Path, _args: &[&str]) -> Result<Output, GitReadFailure> {
            self.outputs.lock().unwrap().pop_front().unwrap()
        }
    }

    fn output(stdout: &[u8]) -> Result<Output, GitReadFailure> {
        Ok(Output {
            status: ExitStatus::from_raw(0),
            stdout: stdout.to_vec(),
            stderr: Vec::new(),
        })
    }

    #[test]
    fn returns_bounded_redacted_text_and_explicit_binary_unavailability() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("new.txt"),
            "safe\nAuthorization: Bearer opaque\n",
        )
        .unwrap();
        let port = FakeGit {
            outputs: Mutex::new(VecDeque::from([
                output(b" M tracked.txt\0?? new.txt\0"),
                output(b"diff --git a/track b/track\n+safe\n+password=opaque\n"),
            ])),
        };
        let review = inspect_post_run_review_with(&port, root.path()).unwrap();
        assert_eq!(review.changed_files.len(), 2);
        assert!(review.changed_files[0]
            .diff
            .as_deref()
            .unwrap()
            .contains("[REDACTED sensitive line]"));
        assert!(review.changed_files[1]
            .diff
            .as_deref()
            .unwrap()
            .contains("+[REDACTED sensitive line]"));
        assert!(!format!("{review:?}").contains("opaque"));
    }

    #[test]
    fn rejects_unsafe_status_paths_and_preserves_typed_git_failure() {
        let root = tempdir().unwrap();
        let unsafe_port = FakeGit {
            outputs: Mutex::new(VecDeque::from([output(b"?? ../escape\0")])),
        };
        assert!(matches!(
            inspect_post_run_review_with(&unsafe_port, root.path()),
            Err(ReviewEvidenceFailure::InvalidStatus(_))
        ));
        let failed_port = FakeGit {
            outputs: Mutex::new(VecDeque::from([Err(GitReadFailure {
                kind: GitReadFailureKind::Failed,
                detail: "typed".into(),
            })])),
        };
        assert!(matches!(
            inspect_post_run_review_with(&failed_port, root.path()),
            Err(ReviewEvidenceFailure::Git(_))
        ));
    }
}
