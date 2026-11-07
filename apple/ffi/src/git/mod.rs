//! Git for the right panel: Changes, History, the diff and the worktree per terminal, over core's `git/`
//! domain (every command through its hardened `GitCmd`; nothing here spawns git). Calls are blocking and
//! meant for a background thread; changes arrive as one coalesced `GitListener.changed()` per debounced
//! burst of file events (`FFI-CONTRACT.md`, "Git").

mod error;
mod patch;
mod records;
mod service;
mod watch;
mod worktrees;

pub use error::GitBackendError;
pub use records::{
    GitChange, GitChangeKind, GitCommitDetails, GitDiff, GitLog, GitLogCommit, GitRef, GitRefKind, GitRev,
    GitRevKind, GitSignature, GitStatus,
};
pub use service::Git;
pub use watch::{GitListener, GitWatch};
pub use worktrees::{GitEnvVar, GitWorktree, GitWorktreePlan, WorktreeService};
