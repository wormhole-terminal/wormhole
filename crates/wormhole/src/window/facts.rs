//! What a feature may read of the window: borrowed, built when a frame (or a hook) asks, and gone
//! when it is done. A feature gets these and never the `Workspace` (`feature` says why), so what it
//! can know of the window is what is listed here and nothing else.
//!
//! Nothing is copied but a few words: [`WindowFacts`] points into the `Workspace` and holds no `&App`,
//! so building one lends no allocation and leaves `cx` free for the hook that is handed the facts
//! (`view(&entity, &facts, window, cx)` needs `cx` mutably, which a facts value holding `&App` would
//! forbid). What reads an entity (a terminal's folder, whether an agent runs) takes the caller's `cx`.
//! The facts live through the build part of `render`; a listener never captures them (it captures the
//! feature's entity and ids), so the borrow ends before any listener runs.

#![cfg_attr(not(test), expect(dead_code, reason = "the contract is built before every feature uses it"))]

use std::path::Path;

use gpui_kit::*;
use wormhole_core::config::{FeatureMask, TabId};
use wormhole_core::storage::paths::ProfileId;

use crate::terminal_view::TerminalView;
use crate::window::Workspace;
// A feature's type that the window names. It moves to `window/repo.rs` in 5d, when Changes takes its
// state; until then `Session.git` is lent through the facts, not copied.
use crate::window::features::git::changes::GitState;
use crate::window::session::Session;

/// The window as a feature sees it.
#[derive(Clone, Copy)]
pub(in crate::window) struct WindowFacts<'a> {
    pub profile: &'a ProfileId,
    /// Which features are on. Copied, so a hook can ask about an off feature too (the menu hook runs
    /// whether the feature is on or off).
    pub features: FeatureMask,
    /// The panel tab on show.
    pub tab: TabId,
    /// The panel's width.
    pub panel_width: Pixels,
    /// The row of the shown tab the keys are on, and whether its ring shows (the keys, not a click,
    /// were last used, and the window is in front). Only for facts made for drawing a frame
    /// ([`Workspace::frame_facts`]); `None` otherwise.
    pub panel_cursor: Option<(usize, bool)>,
    /// Scrolls the panel's short tabs (a tab that is all there at once draws itself in a column
    /// that scrolls with this); the window sets it back to the top when another tab is shown.
    pub panel_scroll: &'a ScrollHandle,
    workspace: &'a Workspace,
}

/// One terminal of the window.
#[derive(Clone, Copy)]
pub(in crate::window) struct SessionFacts<'a> {
    pub id: i64,
    pub terminal: &'a Entity<TerminalView>,
    /// `Session.git` stays on the session (the sidebar's rows, the title row and the status bar read
    /// it too, so it is not a feature's); the facts lend it, so the Changes tab reads the status
    /// without the window naming its types and without a copy.
    pub git: &'a GitState,
}

impl<'a> SessionFacts<'a> {
    fn of(session: &'a Session) -> Self {
        Self { id: session.id, terminal: session.terminal(), git: &session.git }
    }

    /// The terminal's folder, if it reports one. Reads the terminal, so it takes the caller's `cx`.
    pub fn folder<'c>(&self, cx: &'c App) -> Option<&'c Path> {
        self.terminal.read(cx).cwd.as_deref()
    }
}

impl<'a> WindowFacts<'a> {
    /// The selected terminal; `None` only while the window has none.
    pub fn active(&self) -> Option<SessionFacts<'a>> {
        self.workspace.sessions.get(self.workspace.active).map(SessionFacts::of)
    }

    /// The terminals on screen, in the window's order: the selected one, or every pane of its split.
    pub fn on_screen(&self) -> impl Iterator<Item = SessionFacts<'a>> + use<'a> {
        let workspace = self.workspace;
        (0..workspace.sessions.len())
            .filter(move |&ix| workspace.on_screen(ix))
            .map(move |ix| SessionFacts::of(&workspace.sessions[ix]))
    }

    /// Whether a coding agent is detected in a terminal on screen.
    pub fn agent_on_screen(&self, cx: &App) -> bool {
        self.on_screen().any(|session| session.terminal.read(cx).agent.is_some())
    }
}

impl Workspace {
    /// The facts a feature may read of this window, as of now. `cx` is read for the settings' feature
    /// mask and not kept.
    pub(in crate::window) fn facts<'a>(&'a self, cx: &App) -> WindowFacts<'a> {
        WindowFacts {
            profile: &self.profile,
            features: crate::settings::store_in(&self.profile, cx).features(),
            tab: self.right_tab,
            panel_width: px(self.columns().panel),
            panel_cursor: None,
            panel_scroll: &self.panel_scroll,
            workspace: self,
        }
    }

    /// [`Self::facts`] for drawing a frame: with the row the keys are on in the shown tab, which
    /// needs the window to say whether the ring shows.
    pub(in crate::window) fn frame_facts<'a>(&'a self, window: &Window, cx: &App) -> WindowFacts<'a> {
        let mut facts = self.facts(cx);
        facts.panel_cursor = self
            .areas
            .panel_at
            .filter(|&(tab, _)| tab == self.right_tab)
            .map(|(_, row)| (row, self.ring(wormhole_core::view::focus::Area::PanelContent, window)));
        facts
    }
}
