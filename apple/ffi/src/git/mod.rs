//! Git for the right panel: Changes, History, the diff and the worktree per terminal, over core's `git/`
//! domain (every command through its hardened `GitCmd`; nothing here spawns git). Calls are blocking and
//! meant for a background thread; changes arrive as one coalesced `GitListener.changed()` per debounced
//! burst of file events (`FFI-CONTRACT.md`, "Git").

mod error;
mod notes;
mod problem;
mod records;
mod service;
mod shown;
mod watch;
mod worktrees;

pub use error::GitBackendError;
pub use records::{
    GitChange, GitChangeKind, GitDiff, GitDiffFile, GitDiffLine, GitHunk, GitLineKind, GitLog, GitLogCommit,
    GitRef, GitRefKind, GitRev, GitRevKind, GitStatus,
};
pub use notes::{GitDiffNotes, GitSideNote};
pub use problem::{GitProblemWords, git_problem};
pub use service::Git;
pub use shown::{GitCommitDetails, GitCompact, GitFact, GitParent, GitSpan, GitTone};
pub use watch::{GitListener, GitWatch};
pub use worktrees::{GitEnvVar, GitWorktree, GitWorktreePlan, WorktreeService};
pub(crate) use worktrees::env_for as worktree_env;
