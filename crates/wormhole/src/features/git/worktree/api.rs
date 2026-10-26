//! What the surfaces above the window (the command line) may ask of Worktrees about a window: the
//! dialog for `new-terminal --worktree`. It only opens the dialog; a worktree is made when the person
//! confirms it.

use gpui_kit::*;
use wormhole_core::ipc::control::{Asked, Error, ErrorCode};

use super::Worktrees;
use crate::window::Workspace;

/// The command line's `new-terminal --worktree <branch>`: the dialog, filled in, in this window.
pub(crate) fn control_ask(
    ws: &Workspace,
    folder: Option<String>,
    branch: &str,
    command: Option<String>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Result<Asked, Error> {
    let Some(worktrees) = ws.feature::<Worktrees>() else {
        return Err(Error::new(ErrorCode::Failed, "this window has no worktrees"));
    };
    let facts = ws.facts(cx);
    let Some(terminal) = facts.active().map(|active| active.terminal.clone()) else {
        return Err(Error::new(ErrorCode::Failed, "this window has no terminal"));
    };
    let (window_id, profile) = (facts.window_id, facts.profile.clone());
    worktrees.update(cx, |worktrees, wcx| {
        worktrees.control_ask_worktree(&profile, &terminal, window_id, folder, branch, command, window, wcx)
    })
}
