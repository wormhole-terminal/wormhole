//! Saving a window: its record (terminals, splits, layout) when something changed, and every
//! terminal's text once a minute.
//!
//! Part of the window: it uses the shared state's stores and the settings, nothing above the window.

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::storage::store::WindowRecord;

use crate::app_state::{app, app_mut};
use crate::settings::settings_in;
use crate::window::{Layout, Workspace};

/// How many lines of each terminal's scrollback are saved and restored. Fixed, whatever the
/// `scrollback_lines` setting (that one is how much a running terminal keeps).
pub(super) const SCROLLBACK_LINES: usize = 2000;
pub(super) const SCROLLBACK_SAVE_INTERVAL: Duration = Duration::from_secs(60);

impl Workspace {
    pub(crate) fn window_record(&self, cx: &App) -> WindowRecord {
        // The groups keep only the terminals that are still here.
        let mut view = self.sidebar_view(cx).clone();
        view.groups.retain_terminals(|id| self.sessions.iter().any(|session| session.id == id));
        let layout = Layout::new(self.panes, self.right_tab, view);
        WindowRecord {
            id: self.window_id,
            active: self.active,
            layout: serde_json::to_string(&layout).ok(),
            bounds: self.bounds.clone(),
            sessions: (0..self.sessions.len()).map(|ix| self.session_record(ix, cx)).collect(),
            splits: self.split_records(),
        }
    }

    pub(crate) fn save(&mut self, cx: &mut App) {
        self.dirty = false;
        if self.discarded {
            return;
        }
        let record = self.window_record(cx);
        app_mut(cx).save_window(&self.profile, &record);
    }

    /// Every terminal's text whose text is kept, in one transaction (sealed: `store::saved_text`).
    /// Nothing when Restore terminal text after restart is off.
    pub(crate) fn save_scrollback(&mut self, cx: &mut App) {
        self.last_scrollback_save = Instant::now();
        if self.discarded || !settings_in(&self.profile, cx).restore_terminal_text {
            return;
        }
        let texts: Vec<(i64, String)> = self
            .sessions
            .iter()
            .filter(|session| !session.forget_text)
            .map(|session| {
                (session.pane.id, session.pane.terminal.read(cx).scrollback_text(SCROLLBACK_LINES))
            })
            .collect();
        let Some(store) = app_mut(cx).store_of_mut(&self.profile) else { return };
        if let Err(err) = store.save_scrollbacks(&texts) {
            tracing::warn!("scrollback save failed: {err:#}");
        }
    }

    /// "Don't keep this terminal's text" in its row menu: from now on nothing of it is saved, and
    /// what was is erased at once; chosen again, it is saved from the next minute on.
    pub(crate) fn toggle_forget_text(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get_mut(ix) else { return };
        session.forget_text = !session.forget_text;
        let (id, forget) = (session.id, session.forget_text);
        if forget
            && let Some(store) = app(cx).store_of(&self.profile)
            && let Err(err) = store.clear_scrollback(id)
        {
            tracing::warn!("erasing terminal {id}'s saved text failed: {err:#}");
        }
        // At once: a restart before the next save must not bring the choice back undone.
        self.save(cx);
        cx.notify();
    }

    /// "Hide from Agents": the terminal can no longer be listed, read or typed into by a program that
    /// controls Wormhole (one that runs inside it still sees itself); chosen again, it can. Saved at once,
    /// so a restart does not bring the choice back undone. The one place the flag changes, for the row
    /// menu, the terminal's menu, the palette and devctl.
    pub(crate) fn set_agent_hidden(&mut self, ix: usize, hidden: bool, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(ix) else { return };
        session.terminal().clone().update(cx, |terminal, cx| terminal.set_agent_hidden(hidden, cx));
        // The row, the title row and the bus hear of it through the summary.
        self.sync_summary_at(ix, cx);
        self.save(cx);
        cx.notify();
    }

    /// [`set_agent_hidden`](Self::set_agent_hidden) the other way round than the terminal is now.
    pub(crate) fn toggle_agent_hidden(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.get(ix) {
            let hidden = !session.terminal().read(cx).agent_hidden();
            self.set_agent_hidden(ix, hidden, cx);
        }
    }
}
