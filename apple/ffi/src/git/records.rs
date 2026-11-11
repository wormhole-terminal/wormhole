//! The values that cross the seam for git: core's types cut down to what the panels show. Wording that
//! core owns (a submodule's label, a too-large diff's title) arrives as text, so Swift keeps no copy of it.

use wormhole_core::config::SizeUnits;
use wormhole_core::git::{
    ChangeKind, Commit, CommitRef, DIFF_HIGHLIGHT_MAX_LINES, DiffLine, FileChange, FileDiff, Hunk, LineKind,
    RefKind, Submodule, lines_in_new_file,
};

use super::notes::GitDiffNotes;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GitChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

impl From<ChangeKind> for GitChangeKind {
    fn from(kind: ChangeKind) -> Self {
        match kind {
            ChangeKind::Modified => Self::Modified,
            ChangeKind::Added => Self::Added,
            ChangeKind::Deleted => Self::Deleted,
            ChangeKind::Renamed => Self::Renamed,
            ChangeKind::Untracked => Self::Untracked,
            ChangeKind::Conflicted => Self::Conflicted,
        }
    }
}

/// One entry of the status, or of a commit's files.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GitChange {
    /// Relative to the repository's root.
    pub path: String,
    pub original_path: Option<String>,
    pub kind: GitChangeKind,
    /// `None` for a binary file and for a submodule.
    pub added: Option<u32>,
    pub removed: Option<u32>,
    /// Stored with Git LFS: the counts are not shown.
    pub lfs: bool,
    /// A submodule, not a file: core's words for what the parent sees of it ("2 new commits").
    pub submodule: Option<String>,
    /// A submodule that is checked out and still there: the row offers "Open a terminal in" it.
    pub submodule_enterable: bool,
}

impl GitChange {
    pub(crate) fn of(file: &FileChange) -> Self {
        Self {
            path: file.path.clone(),
            original_path: file.original_path.clone(),
            kind: file.kind.into(),
            added: file.added,
            removed: file.removed,
            lfs: file.lfs,
            submodule: file.submodule.as_ref().map(|sub: &Submodule| sub.label(file.kind)),
            submodule_enterable: file.submodule.as_ref().is_some_and(Submodule::checked_out)
                && file.kind != ChangeKind::Deleted,
        }
    }
}

/// The working tree's status, or the files against a compared revision.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GitStatus {
    pub files: Vec<GitChange>,
    pub branch: Option<String>,
    /// The tree has uncommitted changes, whatever "Compare with" lists.
    pub dirty: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// The compared branch no longer exists: the files are against HEAD and the panel says so.
    pub compare_missing: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GitRevKind {
    Branch,
    Tag,
    Commit,
    Stashes,
}

/// What a branch picker lists. `name` is what git is given back.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitRev {
    pub name: String,
    pub kind: GitRevKind,
    /// A commit's subject.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GitRefKind {
    Head,
    Branch,
    Remote,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitRef {
    pub kind: GitRefKind,
    pub name: String,
}

impl From<&CommitRef> for GitRef {
    fn from(found: &CommitRef) -> Self {
        let kind = match found.kind {
            RefKind::Head => GitRefKind::Head,
            RefKind::Branch => GitRefKind::Branch,
            RefKind::Remote => GitRefKind::Remote,
            RefKind::Tag => GitRefKind::Tag,
        };
        Self { kind, name: found.name.clone() }
    }
}

/// A row of the log.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitLogCommit {
    pub sha: String,
    pub short: String,
    pub parent: Option<String>,
    pub merge: bool,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    pub subject: String,
    pub refs: Vec<GitRef>,
}

impl From<&Commit> for GitLogCommit {
    fn from(commit: &Commit) -> Self {
        Self {
            sha: commit.sha.clone(),
            short: commit.short.clone(),
            parent: commit.parent.clone(),
            merge: commit.merge,
            author: commit.author.clone(),
            time: commit.time,
            subject: commit.subject.clone(),
            refs: commit.refs.iter().map(GitRef::from).collect(),
        }
    }
}

/// The log of a revision (or the stashes).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GitLog {
    /// The branch the log is of: the revision's name, or what HEAD is on.
    pub branch: Option<String>,
    pub commits: Vec<GitLogCommit>,
    /// More commits exist than `limit` (the tab says "Latest 300 commits").
    pub limited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GitLineKind {
    Context,
    Added,
    Removed,
}

/// One line of a hunk, numbered on the side(s) it is on.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitDiffLine {
    pub kind: GitLineKind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: String,
    /// Git's "\ No newline at end of file" followed this line.
    pub no_newline: bool,
    /// The line of the new file the line's open icon opens the editor at: its own, or for a removed line the nearest one
    /// (`git::lines_in_new_file`); none when the hunk leaves no line.
    pub open_line: Option<u32>,
}

impl GitDiffLine {
    fn of(line: &DiffLine, open_line: Option<u32>) -> Self {
        let kind = match line.kind {
            LineKind::Context => GitLineKind::Context,
            LineKind::Added => GitLineKind::Added,
            LineKind::Removed => GitLineKind::Removed,
        };
        Self {
            kind,
            old: line.old,
            new: line.new,
            text: line.text.clone(),
            no_newline: line.no_newline,
            open_line,
        }
    }
}

/// A hunk: git's `@@ -41,8 +41,10 @@ section` line as it is drawn, and its lines.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitHunk {
    pub header: String,
    pub lines: Vec<GitDiffLine>,
}

impl From<&Hunk> for GitHunk {
    fn from(hunk: &Hunk) -> Self {
        let open = lines_in_new_file(&hunk.lines);
        let lines = hunk.lines.iter().zip(open).map(|(line, open)| GitDiffLine::of(line, open)).collect();
        Self { header: hunk.header.clone(), lines }
    }
}

/// A file's parsed diff. `old_path` is `None` for a new file and `new_path` for a deleted one; they differ for a
/// rename. A binary file has no hunks.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitDiffFile {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub hunks: Vec<GitHunk>,
    /// "Mode 100644 → 100755" when the file's mode changed.
    pub mode_change: Option<String>,
    /// More lines than core colours (`DIFF_HIGHLIGHT_MAX_LINES`): the badge "No highlighting", and Swift colours none.
    pub highlight_skipped: bool,
    /// What the card says over a diff with no hunks (empty when it has some).
    pub notes: GitDiffNotes,
}

impl GitDiffFile {
    /// `diff` for `file`: the paths as the header of `git diff` names them (a rename keeps both), hunks as core
    /// parsed them.
    pub(crate) fn of(file: &FileChange, diff: &FileDiff, units: SizeUnits) -> Self {
        let old = diff.renamed_from.as_deref().or(file.original_path.as_deref()).unwrap_or(&file.path);
        Self {
            old_path: (!diff.new_file).then(|| old.to_string()),
            new_path: (!diff.deleted_file).then(|| file.path.clone()),
            hunks: diff.hunks.iter().map(GitHunk::from).collect(),
            mode_change: diff.mode_change.as_ref().map(|(old, new)| format!("Mode {old} \u{2192} {new}")),
            highlight_skipped: diff.hunks.iter().map(|hunk| hunk.lines.len()).sum::<usize>()
                > DIFF_HIGHLIGHT_MAX_LINES,
            notes: super::notes::notes(diff, file.kind, units),
        }
    }
}

/// What a file's diff is, for a person: lines, or why there are none.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum GitDiff {
    /// Core's parsed hunks (`git::diff`): the panel draws them as they are.
    Text { file: GitDiffFile },
    /// A binary file: `sizes` is "48.2 KB → 52.9 KB" when known.
    Binary { file: GitDiffFile, sizes: Option<String> },
    /// Stored with Git LFS: what the pointers stand for ("12.3 MB → 12.8 MB · oid 4d7a21…").
    Lfs { file: GitDiffFile, summary: Option<String> },
    /// More than core loads unasked: its two lines of wording and the size. Ask again with `load_anyway`.
    TooLarge { bytes: Option<u64>, title: String, detail: String },
    /// A submodule: what moved, and the commits between (its own repository's, newest first).
    Submodule { label: String, note: Option<String>, ahead: Vec<GitLogCommit>, behind: Vec<GitLogCommit> },
}
