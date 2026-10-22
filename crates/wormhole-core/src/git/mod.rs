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
pub mod discover;
mod head;
mod lfs;
mod log;
pub mod pr;
pub mod review;
mod split;
mod status;
mod status_parse;
mod submodule;
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
    DIFF_ASK_BYTES, DIFF_HIGHLIGHT_MAX_LINES, DIFF_MEASURE_BYTES, DiffLimits, diff, diff_with_limits,
    lines_in_new_file,
};
pub use diff_text::{byte_size, lfs_side, lfs_summary, sizes_label, too_large_detail, too_large_title};
pub use head::{HeadCommit, head_commit, worktree_of};
pub use lfs::{LfsChange, LfsPointer, parse_pointer};
pub use log::{LOG_LIMIT, branches, commit_files, commit_spec, is_ancestor, log, log_failure};
pub use split::{Cell, Side, SplitRow, sides, split_rows};
pub use status::{changed_files, status, status_summary};
pub use submodule::{
    COMMITS_SHOWN, Checkout, Range, Submodule, SubmoduleLog, Superproject, annotate, annotate_files, short,
    submodule_log, superproject,
};

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
    /// A submodule, not a file: what the parent sees of it (`added`/`removed` say nothing then).
    pub submodule: Option<Submodule>,
}

impl FileChange {
    /// A change to `path` that nothing is known about yet: no counts, no old name.
    pub fn new(path: impl Into<String>, kind: ChangeKind) -> Self {
        Self {
            path: path.into(),
            original_path: None,
            kind,
            added: None,
            removed: None,
            lfs: false,
            submodule: None,
        }
    }
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
    /// The repository this one is a submodule of, read from its `.git` file: no process.
    pub superproject: Option<Superproject>,
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

// A parsed patch. It sits here beside `FileChange` rather than in `diff`: `diff` builds it, and `lfs`
// and `split` read it, while `diff` calls both of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: String,
    /// Git's "\ No newline at end of file" followed this line.
    pub no_newline: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileDiff {
    pub hunks: Vec<Hunk>,
    pub binary: bool,
    /// The file doesn't exist on the old side (`new file mode`, `--- /dev/null`).
    pub new_file: bool,
    /// The file doesn't exist on the new side (`deleted file mode`, `+++ /dev/null`).
    pub deleted_file: bool,
    /// `rename from <path>`: where the file was before.
    pub renamed_from: Option<String>,
    /// `old mode` / `new mode`, e.g. `("100644", "100755")`.
    pub mode_change: Option<(String, String)>,
    /// Full file contents for syntax highlighting (None if missing/binary/over the limit).
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    /// The patch is bigger than [`DiffLimits::max_patch_bytes`], so `hunks` and the texts are
    /// left out (everything else is at its default but [`FileDiff::patch_bytes`]). Ask before
    /// loading it, then ask again with [`diff_with_limits`] and [`DiffLimits::LOAD_ANYWAY`].
    pub too_large: bool,
    /// With [`FileDiff::too_large`]: how big the patch is, unless it is bigger than
    /// [`DIFF_MEASURE_BYTES`] (`None`).
    pub patch_bytes: Option<u64>,
    /// A binary file's size on its old and new side, in bytes (`None` for a side the file
    /// doesn't have, or whose size could not be read).
    pub sizes: (Option<u64>, Option<u64>),
    /// Each side of the file is a Git LFS pointer: what they point at. Their lines are left out
    /// of `hunks`, which is empty.
    pub lfs: Option<LfsChange>,
    /// The patch has more lines than [`DiffLimits::max_highlight_lines`]: the hunks are all
    /// there, but the texts are left out, so it shows without syntax colors.
    pub highlight_skipped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub parent: Option<String>,
    pub author: String,
    /// Unix timestamp (seconds).
    pub time: i64,
    pub subject: String,
}

impl Commit {
    /// One commit of `--format=%H%x1f%h%x1f%P%x1f%an%x1f%at%x1f%s`. Beside the type rather than in
    /// `log`, because a submodule's log reads the same records and `log` itself uses `submodule`.
    pub(crate) fn from_record(record: &str) -> Option<Self> {
        let fields: Vec<&str> = record.split('\x1f').collect();
        if fields.len() < 6 {
            return None;
        }
        Some(Self {
            sha: fields[0].to_string(),
            short: fields[1].to_string(),
            parent: fields[2].split(' ').next().filter(|p| !p.is_empty()).map(str::to_string),
            author: fields[3].to_string(),
            time: fields[4].parse().unwrap_or(0),
            subject: fields[5].to_string(),
        })
    }
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
