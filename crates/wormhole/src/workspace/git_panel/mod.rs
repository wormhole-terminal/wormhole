//! Git views in the right panel: working-tree changes (with "compare with"
//! a branch), commit history, and the diff view they open.

pub mod changes;
pub mod diff;
mod file_line;
pub mod history;
mod job;
pub(crate) mod open_file;
pub mod picker;
mod problem;
mod submodule_row;

// The diff view's old paths, which `workspace` and the Theme Studio still use.
pub(crate) use diff as diff_view;
pub(crate) use diff::render as diff_render;
