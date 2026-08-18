//! Repository/Git read boundary shared by project inspection, recovery, and
//! future guided artifact planning. Callers own product policy and serialized
//! command error mapping.

use std::{
    path::Path,
    process::{Command, Output},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GitReadFailureKind {
    Unavailable,
    Failed,
}

#[derive(Debug)]
pub(crate) struct GitReadFailure {
    pub(crate) kind: GitReadFailureKind,
    pub(crate) detail: String,
}

pub(crate) trait GitReadPort {
    fn output(&self, root: &Path, args: &[&str]) -> Result<Output, GitReadFailure>;
}

/// Filesystem shape required by artifact planning and result review. Existing
/// command adapters retain their proved no-follow/versioned implementation;
/// new workflow services depend on this port instead of Tauri state.
#[allow(dead_code)]
pub(crate) trait RepositoryFilePort {
    type Error;
    type Snapshot;
    type VersionedFile;
    type WriteReceipt;

    fn inspect(&self, root: &Path) -> Result<Self::Snapshot, Self::Error>;
    fn read(&self, root: &Path, relative: &str) -> Result<Self::VersionedFile, Self::Error>;
    fn write_existing(
        &self,
        root: &Path,
        relative: &str,
        content: &[u8],
        expected_version: &str,
    ) -> Result<Self::WriteReceipt, Self::Error>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NativeGitRead;

impl GitReadPort for NativeGitRead {
    fn output(&self, root: &Path, args: &[&str]) -> Result<Output, GitReadFailure> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|error| GitReadFailure {
                kind: GitReadFailureKind::Unavailable,
                detail: error.to_string(),
            })?;
        if !output.status.success() {
            return Err(GitReadFailure {
                kind: GitReadFailureKind::Failed,
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(output)
    }
}

pub(crate) fn read_text_with<P: GitReadPort>(
    port: &P,
    root: &Path,
    args: &[&str],
) -> Result<String, GitReadFailure> {
    port.output(root, args)
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn read_bytes_with<P: GitReadPort>(
    port: &P,
    root: &Path,
    args: &[&str],
) -> Result<Vec<u8>, GitReadFailure> {
    port.output(root, args).map(|output| output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::process::ExitStatusExt,
        process::{ExitStatus, Output},
    };

    struct FakeGit {
        output: Result<Output, GitReadFailureKind>,
    }

    impl GitReadPort for FakeGit {
        fn output(&self, _root: &Path, _args: &[&str]) -> Result<Output, GitReadFailure> {
            self.output
                .as_ref()
                .map(Clone::clone)
                .map_err(|kind| GitReadFailure {
                    kind: *kind,
                    detail: "typed failure".into(),
                })
        }
    }

    #[test]
    fn text_and_bytes_share_one_injected_git_read_port() {
        let port = FakeGit {
            output: Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: b"main\n".to_vec(),
                stderr: Vec::new(),
            }),
        };
        assert_eq!(
            read_text_with(&port, Path::new("/unused"), &["branch"]).unwrap(),
            "main"
        );
        assert_eq!(
            read_bytes_with(&port, Path::new("/unused"), &["branch"]).unwrap(),
            b"main\n"
        );
    }

    #[test]
    fn typed_failures_are_webview_and_serialization_independent() {
        let port = FakeGit {
            output: Err(GitReadFailureKind::Unavailable),
        };
        let failure = read_text_with(&port, Path::new("/unused"), &[]).unwrap_err();
        assert_eq!(failure.kind, GitReadFailureKind::Unavailable);
        assert_eq!(failure.detail, "typed failure");
    }
}
