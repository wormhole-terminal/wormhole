//! Read-only git access for the Changes panel and diff view, and the one exception.
//!
//! Shells out to the `git` CLI so results match what the user sees in their
//! own terminal (config, attributes, ignore rules). Every command is built by
//! [`GitCmd`] (`cmd`), which keeps a repository's own configuration from
//! running code, keeps names from the outside from being read as options, takes
//! no locks (a background refresh must not take the index lock away from an
//! agent that is committing) and bounds how long and how much a command may run.
//!
//! The exception is `worktree`: `git worktree add` and `git worktree remove`, through `write`'s
//! builder with the same hardening (and hooks off), only on the user's action, confirmed in the
//! app, never forced. Nothing else writes. (`pr` is not git at all: it asks the GitHub CLI for a
//! branch's pull request, read-only, for a trusted project, with a scrubbed environment.)

mod cmd;
mod counts;
mod diff;
mod diff_text;
mod head;
mod lfs;
mod log;
pub mod pr;
mod split;
mod status;
#[cfg(test)]
pub(crate) mod testing;
pub mod worktree;
mod write;

use std::path::{Path, PathBuf};

use anyhow::Result;

pub(crate) use cmd::GitCmd;
pub use cmd::GitError;
#[cfg(test)]
pub(crate) use diff::parse_diff;
pub use diff::{
    DIFF_ASK_BYTES, DIFF_HIGHLIGHT_MAX_LINES, DIFF_MEASURE_BYTES, DiffLimits, DiffLine, FileDiff, Hunk,
    LineKind, diff, diff_with_limits,
};
pub use diff_text::{byte_size, lfs_side, lfs_summary, sizes_label, too_large_detail, too_large_title};
pub use head::{HeadCommit, head_commit, worktree_of};
pub use lfs::{LfsChange, LfsPointer, parse_pointer};
pub use log::{LOG_LIMIT, branches, commit_files, commit_spec, is_ancestor, log, log_failure};
pub use split::{Cell, Side, SplitRow, sides, split_rows};
pub use status::{changed_files, status, status_summary};

/// Largest file we read into memory to count its lines or to show as highlighting source.
pub(crate) const MAX_TEXT_BYTES: u64 = 4 << 20;

/// Hash of git's empty tree, used as the base when a repo has no commits yet.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

impl ChangeKind {
    pub fn letter(self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Untracked => "U",
            Self::Conflicted => "!",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileChange {
    /// Path relative to the repo root.
    pub path: String,
    pub original_path: Option<String>,
    pub kind: ChangeKind,
    /// Line counts vs HEAD; `None` for binary files.
    pub added: Option<u32>,
    pub removed: Option<u32>,
    /// Stored with Git LFS: the counts are the pointer's, not the file's.
    pub lfs: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepoStatus {
    pub root: PathBuf,
    pub branch: Option<String>,
    pub files: Vec<FileChange>,
    /// The revision tracked files are compared against: HEAD, the empty tree
    /// (no commits yet), or the merge-base with a chosen branch.
    pub base_rev: String,
    /// The branch picked in "Compare with", if not HEAD.
    pub compare: Option<String>,
    /// The branch picked in "Compare with" when it no longer exists. The status is then against
    /// HEAD (and `compare` is `None`) until another branch is picked, and the panel says so.
    pub compare_missing: Option<String>,
    /// Whether the working tree has uncommitted changes: a tracked file edited or staged, or an
    /// untracked one. It is the tree's, whatever "Compare with" puts in `files`.
    pub dirty: bool,
    /// Where the branch stands against its upstream, from the same `git status` call, so the
    /// title row's ahead and behind cost no process. `None` for a detached HEAD, a branch with no
    /// upstream, or one whose upstream is gone.
    pub upstream: Option<Upstream>,
}

/// A branch's upstream and how far apart they are (what `git status` calls `branch.ab`). Only what
/// the local refs say: nothing is fetched, so it is as old as the last fetch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upstream {
    /// `origin/main`.
    pub name: String,
    /// Commits the branch has that its upstream hasn't.
    pub ahead: u32,
    /// Commits its upstream has that the branch hasn't.
    pub behind: u32,
}

/// Which two sides a file diff compares.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffSpec {
    pub old: String,
    /// `None` means the working tree.
    pub new: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub parent: Option<String>,
    pub author: String,
    /// Unix timestamp (seconds).
    pub time: i64,
    pub subject: String,
}

pub(crate) fn has_head(root: &Path) -> bool {
    GitCmd::new(root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The repository `path` is in, found by looking for `.git` in it and its
/// parents. No git process, so it is instant (the sidebar groups by it on every
/// render); it agrees with `git rev-parse --show-toplevel` for ordinary repos,
/// worktrees and submodules, whose `.git` is a file.
pub fn find_root(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|dir| dir.join(".git").exists()).map(Path::to_path_buf)
}

/// The top of the repository `cwd` is in, as git sees it. `Ok(None)` only when git says there is
/// none; a git that is missing or fails (dubious ownership, a corrupt index) is an error, so it is
/// never shown as "not a repository".
pub fn repo_root(cwd: &Path) -> Result<Option<PathBuf>> {
    match GitCmd::new(cwd).args(["rev-parse", "--show-toplevel"]).run() {
        Ok(root) => Ok(Some(PathBuf::from(String::from_utf8_lossy(&root).trim()))),
        Err(err) if GitError::of(&err) == GitError::NotARepo => Ok(None),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::{GitError, find_root, repo_root};

    #[test]
    #[cfg(unix)]
    fn repo_root_is_none_only_when_git_says_there_is_no_repository() {
        use super::testing::{DUBIOUS, fake_git, init, with_git};
        let plain = tempfile::tempdir().expect("tempdir");
        assert_eq!(repo_root(plain.path()).expect("git answers"), None);
        let repo = init();
        let root = repo_root(repo.path()).expect("git answers").expect("a repository");
        assert_eq!(root.canonicalize().ok(), repo.path().canonicalize().ok());

        // Everything else used to read "not a repository" too, and now says what it is.
        let tools = tempfile::tempdir().expect("tempdir");
        let dubious = fake_git(tools.path(), "dubious", DUBIOUS, 128);
        let err = with_git(&dubious, || repo_root(repo.path())).expect_err("git failed");
        assert_eq!(GitError::of(&err), GitError::Failed(DUBIOUS.into()));
        let err = with_git(tools.path().join("absent"), || repo_root(repo.path())).expect_err("no git");
        assert_eq!(GitError::of(&err), GitError::GitMissing);
        let stub = fake_git(tools.path(), "stub", "xcrun: error: invalid active developer path (x)", 1);
        let err = with_git(&stub, || repo_root(repo.path())).expect_err("no git");
        assert_eq!(GitError::of(&err), GitError::GitMissing);
        let none = fake_git(
            tools.path(),
            "none",
            "fatal: not a git repository (or any of the parent directories): .git",
            128,
        );
        assert_eq!(with_git(&none, || repo_root(repo.path())).expect("git answers"), None);
    }

    #[test]
    fn find_root_walks_up_to_the_nearest_git() {
        let base = std::env::temp_dir().join(format!("hd-find-root-{}", std::process::id()));
        let repo = base.join("repo");
        let nested = repo.join("src/deep");
        let worktree = base.join("worktree");
        let plain = base.join("plain");
        for dir in [&nested, &repo.join(".git"), &worktree, &plain] {
            std::fs::create_dir_all(dir).expect("create dir");
        }
        // A worktree's (or submodule's) `.git` is a file pointing elsewhere.
        std::fs::write(worktree.join(".git"), "gitdir: elsewhere").expect("git file");
        assert_eq!(find_root(&nested), Some(repo.clone()));
        assert_eq!(find_root(&repo), Some(repo.clone()));
        assert_eq!(find_root(&worktree), Some(worktree.clone()));
        assert_eq!(find_root(&plain), None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
