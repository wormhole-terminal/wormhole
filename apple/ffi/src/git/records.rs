//! The values that cross the seam for git: core's types cut down to what the panels show. Wording that
//! core owns (a submodule's label, a too-large diff's title) arrives as text, so Swift keeps no copy of it.

use wormhole_core::git::{
    ChangeKind, Commit, CommitDetails, CommitRef, FileChange, RefKind, Signature, Submodule,
};

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

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitSignature {
    pub name: String,
    pub email: String,
    pub time: i64,
    /// Seconds east of UTC where it was written.
    pub zone: i32,
}

impl From<&Signature> for GitSignature {
    fn from(signature: &Signature) -> Self {
        Self {
            name: signature.name.clone(),
            email: signature.email.clone(),
            time: signature.time,
            zone: signature.offset,
        }
    }
}

/// An opened commit: its message cleaned for the screen, who and when, and its files with their counts.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GitCommitDetails {
    pub sha: String,
    pub parents: Vec<String>,
    pub author: GitSignature,
    pub committer: GitSignature,
    pub refs: Vec<GitRef>,
    pub subject: String,
    /// The body's text and, after a blank line, its trailers; empty for a one-line message.
    pub body: String,
    /// The message was longer than core reads (or than the screen draws).
    pub cut: bool,
    pub files: Vec<GitChange>,
}

impl GitCommitDetails {
    pub(crate) fn of(details: &CommitDetails, files: &[FileChange]) -> Self {
        let body = details.body_view();
        let text = match (body.text.is_empty(), body.trailers.is_empty()) {
            (_, true) => body.text.clone(),
            (true, false) => body.trailers.clone(),
            (false, false) => format!("{}\n\n{}", body.text, body.trailers),
        };
        Self {
            sha: details.sha.clone(),
            parents: details.parents.clone(),
            author: (&details.author).into(),
            committer: (&details.committer).into(),
            refs: details.refs.iter().map(GitRef::from).collect(),
            subject: details.shown_subject(),
            body: text,
            cut: details.message_cut || body.cut,
            files: files.iter().map(GitChange::of).collect(),
        }
    }
}

/// What a file's diff is, for a person: lines, or why there are none.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum GitDiff {
    /// Unified diff text, git's own hunks (`patch`).
    Text { patch: String },
    /// A binary file: `patch` says so in git's words, `sizes` is "48.2 KB → 52.9 KB" when known.
    Binary { patch: String, sizes: Option<String> },
    /// Stored with Git LFS: what the pointers stand for ("12.3 MB → 12.8 MB · oid 4d7a21…").
    Lfs { summary: Option<String> },
    /// More than core loads unasked: its two lines of wording and the size. Ask again with `load_anyway`.
    TooLarge { bytes: Option<u64>, title: String, detail: String },
    /// A submodule: what moved, and the commits between (its own repository's, newest first).
    Submodule { label: String, note: Option<String>, ahead: Vec<GitLogCommit>, behind: Vec<GitLogCommit> },
}
