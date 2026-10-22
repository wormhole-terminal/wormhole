//! The window's features: the side panels' contents and what they work on, each in its own folder until
//! phase 5 gives them their own state (they read and write `Workspace`'s fields today). `notes` holds the
//! notes and tasks of a terminal, `usage` the coding agents' plan limits, `project` the `.wormhole/`
//! folder's panel, trust and sync, and `git` the Changes, History and diff views and the worktrees.

pub(crate) mod git;
pub(crate) mod notes;
pub(crate) mod project;
pub(in crate::workspace) mod usage;
