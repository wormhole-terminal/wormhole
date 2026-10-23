//! Git views in the right panel: working-tree changes (with "compare with"
//! a branch), commit history, and the diff view they open.

pub mod changes;
pub mod diff;
mod file_line;
pub mod history;
pub(crate) mod open_file;
pub mod picker;
mod problem;
mod submodule_row;
pub(crate) mod worktree;

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod testing;

// The diff view's old paths, which `window` and the Theme Studio still use.
pub(crate) use diff::render as diff_render;
