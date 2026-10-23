//! A terminal in a new git worktree (Shell → New Terminal in New Worktree…, the palette, and the
//! command line's `new-terminal --worktree <branch>`): the dialog that asks for the branch and names the
//! folder, what happens once the person confirms, the question before closing such a terminal (keep the
//! worktree, or remove it) and the removal. The rules and the git calls are core's `git::worktree`;
//! here is only what needs the window and the app's state. An always-on feature (`FeatureId::Worktrees`)
//! with no tab: it must never hide behind another git switch.
//!
//! Safety rules, kept as they were and pinned by `tests`:
//! - Nothing here runs on its own: the dialog is the one way a worktree gets made, so a script asking
//!   through the command line only gets the dialog opened ([`api::control_ask`]).
//! - A repository with an `.wormhole/` project needs that project trusted and its settings on, as a
//!   terminal opened there would take its env and shell; the check is the project feature's
//!   (`project::api::worktree_allowed`), and so is the project's setup command
//!   (`project::api::worktree_setup`). Neither is named from here: the project hands them to the window in
//!   its `WorktreeHooks`, and this feature asks through `FeatureTable::worktree_allowed` and
//!   `worktree_setup`. A repository without a project needs nothing.
//! - A worktree is removed only for a terminal the person closed and chose "Close and Remove Worktree"
//!   for, never one with uncommitted changes or another terminal in it, and only a worktree Wormhole
//!   made; the branch stays. Git itself refuses again if changes came meanwhile.
//! - Git is written to only through core's `git::worktree`, which hardens every call.
//!
//! The window knows this feature by its hooks only: the action it binds, the extra it adds to the close
//! question, the environment it gives a new terminal in a worktree (its ports) and the events it emits
//! to open the terminal.

use gpui_kit::*;

use crate::window::feature::FeatureEvent;

pub(crate) mod api;
mod ask;
mod close;
mod def;
#[cfg(test)]
mod tests;

use ask::Asking;
pub(crate) use def::DEF;

/// The window's worktree feature: the dialog while it is up.
pub(crate) struct Worktrees {
    asking: Option<Asking>,
}

impl EventEmitter<FeatureEvent> for Worktrees {}

impl Worktrees {
    pub(crate) fn new() -> Self {
        Self { asking: None }
    }

    /// Whether the dialog is up.
    #[cfg(test)]
    pub(crate) fn is_asking(&self) -> bool {
        self.asking.is_some()
    }
}
