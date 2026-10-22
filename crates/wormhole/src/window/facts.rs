//! What a feature may read of the window: borrowed, built when a frame (or a hook) asks, and gone
//! when it is done. A feature gets these and never the `Workspace` (`feature` says why), so what it
//! can know of the window is what is listed here and nothing else.
//!
//! Nothing is copied: [`WindowFacts`] points into the `Workspace` and the app, so building one is a
//! few words and lends no allocation. It lives through the build part of `render`; a listener never
//! captures it (it captures the feature's entity and ids), so the borrow ends before any listener runs.

#![allow(dead_code, reason = "the contract exists before the first feature uses it")]

use std::path::Path;

use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::storage::paths::ProfileId;

use crate::terminal_view::TerminalView;
use crate::window::Workspace;
use crate::window::features::git::changes::GitState;
use crate::window::session::Session;

/// The window as a feature sees it.
#[derive(Clone, Copy)]
pub(in crate::window) struct WindowFacts<'a> {
    pub profile: &'a ProfileId,
    /// The panel tab on show.
    pub tab: TabId,
    /// The panel's width in points.
    pub panel_width: f32,
    workspace: &'a Workspace,
    cx: &'a App,
}

/// One terminal of the window.
#[derive(Clone, Copy)]
pub(in crate::window) struct SessionFacts<'a> {
    pub id: i64,
    pub terminal: &'a Entity<TerminalView>,
    /// The terminal's folder, if it reports one.
    pub folder: Option<&'a Path>,
    /// `Session.git` stays on the session (the sidebar's rows, the title row and the status bar read
    /// it too, so it is not a feature's); the facts lend it, so the Changes tab reads the status
    /// without the window naming its types and without a copy.
    pub git: &'a GitState,
}

impl<'a> SessionFacts<'a> {
    fn of(session: &'a Session, cx: &'a App) -> Self {
        let terminal = session.terminal();
        Self { id: session.id, terminal, folder: terminal.read(cx).cwd.as_deref(), git: &session.git }
    }
}

impl<'a> WindowFacts<'a> {
    /// The selected terminal; `None` only while the window has none.
    pub fn active(&self) -> Option<SessionFacts<'a>> {
        self.workspace.sessions.get(self.workspace.active).map(|s| SessionFacts::of(s, self.cx))
    }

    /// The terminals on screen, in the window's order: the selected one, or every pane of its split.
    pub fn on_screen(&self) -> impl Iterator<Item = SessionFacts<'a>> + use<'a> {
        let (workspace, cx) = (self.workspace, self.cx);
        (0..workspace.sessions.len())
            .filter(move |&ix| workspace.on_screen(ix))
            .map(move |ix| SessionFacts::of(&workspace.sessions[ix], cx))
    }

    /// Whether a coding agent is detected in a terminal on screen.
    pub fn agent_on_screen(&self) -> bool {
        self.workspace.usage_tab_on(self.cx)
    }
}

impl Workspace {
    /// The facts a feature may read of this window, as of now.
    pub(in crate::window) fn facts<'a>(&'a self, cx: &'a App) -> WindowFacts<'a> {
        WindowFacts {
            profile: &self.profile,
            tab: self.right_tab,
            panel_width: self.columns().panel,
            workspace: self,
            cx,
        }
    }
}
