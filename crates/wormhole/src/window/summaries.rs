//! The window's copy of what each terminal shows of itself (`TerminalSummary`, kept on its `Pane`).
//!
//! The rows of the sidebar, the status bar, the pane headers and the menus are drawn from the copy,
//! never from the `TerminalView`, so a terminal's output (which notifies the view for every frame it
//! draws, and changes none of these facts) does not make the window draw again; the window is told
//! only when a fact changed, and at once. The copy is taken again when the terminal notifies
//! (`Pane::new` observes it), after what the window itself changes in a terminal (hooks, a seen
//! result, the tick) and, as a backstop, on every tick.

use gpui_kit::*;

use crate::window::Workspace;

impl Workspace {
    /// Take terminal `id`'s summary again if it moved, and tell the window (which draws it, and
    /// tells the sidebar) if it did. Nothing is cloned while it hasn't. Returns whether it moved.
    pub(super) fn sync_summary(&mut self, id: i64, cx: &mut Context<Self>) -> bool {
        self.find_session(id).is_some_and(|ix| self.sync_summary_at(ix, cx))
    }

    /// [`sync_summary`](Self::sync_summary) for the session at `ix`.
    pub(super) fn sync_summary_at(&mut self, ix: usize, cx: &mut Context<Self>) -> bool {
        let pane = &self.sessions[ix].pane;
        let terminal = pane.terminal.read(cx);
        if pane.summary.matches(terminal) {
            return false;
        }
        let fresh = terminal.summary();
        let moved = fresh.cwd != pane.summary.cwd;
        let old = std::mem::replace(&mut self.sessions[ix].pane.summary, fresh);
        self.publish_summary_change(ix, &old, cx);
        if moved {
            // A group that keeps only its folder's terminals lets this one go if it left the folder.
            self.folder_changed(self.sessions[ix].id, cx);
            // Where terminals go is what the folder picker's Recent rows are made of: a visit, once, when the
            // folder changed (not whenever the terminal says something about itself).
            if let Some(folder) = self.sessions[ix].pane.summary.cwd.clone() {
                crate::app_state::recent_folders::visited(cx, &self.profile, &folder);
            }
        }
        cx.notify();
        true
    }

    /// [`sync_summary`](Self::sync_summary) for every session.
    pub(crate) fn sync_summaries(&mut self, cx: &mut Context<Self>) -> bool {
        (0..self.sessions.len()).fold(false, |moved, ix| self.sync_summary_at(ix, cx) | moved)
    }
}
