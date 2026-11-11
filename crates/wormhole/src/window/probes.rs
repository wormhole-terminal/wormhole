//! What a window does when the Probes that are on, or what their hooks set, changed
//! (`probe_runtime`): its terminals draw their rows again and its status bar is built again.
//!
//! Both are cached views, which replay what they drew until told otherwise, so a change that comes from
//! outside a draw has to reach them as a notify (`window::cached`).

use std::path::PathBuf;

use gpui_kit::*;

use super::Workspace;
use crate::app_state::{AppState, OpenWindow};

impl Workspace {
    /// Rules, a status item or a marked block changed: say so to everything that draws them.
    pub(crate) fn probes_changed(&mut self, cx: &mut Context<Self>) {
        for session in &self.sessions {
            session.terminal().update(cx, |_, cx| cx.notify());
        }
        self.redraw_status_bar_if_stale(cx);
        cx.notify();
    }

    /// The Probes that may fold are `probes`: every other Probe's folds in this window's terminals go at once
    /// (one turned off, removed or no longer holding the permission).
    pub(crate) fn retain_probe_folds(&mut self, probes: &[String], cx: &mut Context<Self>) {
        for session in &self.sessions {
            session.terminal().update(cx, |terminal, cx| terminal.retain_probe_folds(probes, cx));
        }
    }

    /// The window's own switch for "never hide lines" and the flag: every terminal shows what they say.
    pub(crate) fn sync_folds(&mut self, cx: &mut Context<Self>) {
        for session in &self.sessions {
            session.terminal().update(cx, |terminal, cx| terminal.sync_folds(cx));
        }
    }
}

/// The window that holds terminal `id`.
pub(crate) fn window_of(id: i64, cx: &App) -> Option<OpenWindow> {
    AppState::find_window(cx, |ws, _| ws.find_session(id).is_some())
}

/// Terminal `id`'s folder as the git chips have it: the one its shell confirmed, which a program's escape
/// sequence alone cannot set. What a hook's `run` starts in.
pub(crate) fn terminal_folder(id: i64, cx: &App) -> Option<PathBuf> {
    let window = window_of(id, cx)?;
    let folder =
        window.workspace.read_with(cx, |ws, cx| ws.pane_terminal(id).and_then(|t| t.read(cx).cwd.clone()));
    folder.ok().flatten()
}

/// A Probe's hook folds the output of block `block` in terminal `id`, less the lines `keep` leaves shown. The
/// terminal says no to what it may not fold (see `wormhole_term::folds`).
pub(crate) fn fold_probe_output(
    id: i64,
    block: u64,
    owner: wormhole_term::folds::FoldOwner,
    keep: (u32, u32),
    cx: &mut App,
) {
    let Some(terminal) = window_of(id, cx)
        .and_then(|window| window.workspace.read_with(cx, |ws, _| ws.pane_terminal(id)).ok().flatten())
    else {
        return;
    };
    terminal.update(cx, |terminal, cx| terminal.fold_for_probe(block, owner, keep, cx));
}

/// A Probe offered a file in terminal `id` and the user said yes: look for it in that terminal's folder and
/// open it, or say it was not found.
pub(crate) fn open_offered_file(id: i64, text: String, cx: &mut App) {
    let Some(terminal) = window_of(id, cx)
        .and_then(|window| window.workspace.read_with(cx, |ws, _| ws.pane_terminal(id)).ok().flatten())
    else {
        return;
    };
    terminal.update(cx, |terminal, cx| terminal.open_path(text, cx));
}
