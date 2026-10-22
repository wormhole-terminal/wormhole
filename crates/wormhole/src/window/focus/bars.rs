//! The keys of the areas that are a row of buttons or tabs: the title row's buttons, the
//! terminal's title strip, the right panel's tabs and the status bar's items. ← → move along them,
//! wrapping; tabs open as the keys reach them (automatic activation); ↩ / Space press the button
//! or item the keys are on.

use gpui_kit::*;
use wormhole_core::view::focus::list::Key;
use wormhole_core::view::focus::{Area, Stop};

use super::{activation, list_key};
use crate::window::Workspace;
use crate::window::session::step_wrapping;
use wormhole_core::config::TabId;

/// The title row's buttons, in the focus order (region 1). The traffic lights are the system's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::window) enum TitleButton {
    /// Hide (or show) the sidebar.
    Sidebar,
    NewTerminal,
}

/// ← (-1) or → (1), with nothing held.
fn along(keystroke: &Keystroke) -> Option<isize> {
    match list_key(keystroke)? {
        Key::Left => Some(-1),
        Key::Right => Some(1),
        _ => None,
    }
}

impl Workspace {
    /// The title row's buttons on screen: both with the sidebar open, its show button alone
    /// without it.
    pub(in crate::window) fn title_buttons(&self, _cx: &App) -> Vec<TitleButton> {
        if self.sidebar.tabs.on {
            // Under tabs the window has no sidebar, nor a button for it.
            Vec::new()
        } else if self.panes.sidebar_open {
            vec![TitleButton::Sidebar, TitleButton::NewTerminal]
        } else {
            vec![TitleButton::Sidebar]
        }
    }

    /// Whether `button` wears the focus ring.
    pub(in crate::window) fn title_ring(&self, button: TitleButton, window: &Window, cx: &App) -> bool {
        self.ring(Area::TitleRow, window) && self.title_buttons(cx).get(self.areas.title) == Some(&button)
    }

    pub(super) fn title_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let buttons = self.title_buttons(cx);
        if let Some(delta) = along(keystroke) {
            self.areas.title = step_wrapping(self.areas.title.min(buttons.len() - 1), delta, buttons.len());
            return true;
        }
        if activation(keystroke).is_none() {
            return false;
        }
        match buttons.get(self.areas.title) {
            Some(TitleButton::Sidebar) => self.toggle_sidebar(cx),
            Some(TitleButton::NewTerminal) => self.new_session(window, cx),
            None => {}
        }
        true
    }

    /// The terminal's title strip: ← → step to the previous or next terminal and keep the keyboard
    /// in the strip, ↩ / Space go into its terminal.
    pub(super) fn tabs_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(delta) = along(keystroke) {
            self.step_session(delta, window, cx);
            self.refocus(Area::Tabs, window, cx);
            return true;
        }
        if activation(keystroke).is_some() {
            self.focus_terminal(window, cx);
            return true;
        }
        false
    }

    /// Whether the Usage tab is offered: a coding agent is detected in a terminal on screen (the
    /// selected one, or any pane of its split). It reads the same `TerminalView::agent` the title
    /// row's context status is keyed on (`agent_gauge`).
    pub(in crate::window) fn usage_tab_on(&self, cx: &App) -> bool {
        (0..self.sessions.len())
            .any(|ix| self.on_screen(ix) && self.sessions[ix].terminal().read(cx).agent.is_some())
    }

    /// Leave a tab that is no longer in the panel's list (Usage once the agent is gone, Notes
    /// with its flag off): back to the tab before Usage, else Changes.
    pub(in crate::window) fn leave_missing_tab(&mut self, cx: &mut Context<Self>) {
        if self.panel_tab_list(cx).contains(&self.right_tab) {
            return;
        }
        let back = self
            .tab_before_usage
            .take()
            .filter(|tab| *tab != self.right_tab && self.panel_tab_list(cx).contains(tab))
            .unwrap_or(TabId::Changes);
        self.show_tab(back, cx);
    }

    /// The right panel's tabs as they are on screen, in the panel's order: a tab of a feature in the
    /// table is offered when its feature is on and the tab says it is `shown`; the rest follow
    /// [`Self::legacy_tab_offered`] until their feature moves into the table.
    pub(in crate::window) fn panel_tab_list(&self, cx: &App) -> Vec<TabId> {
        let facts = self.facts(cx);
        TabId::ALL.into_iter().filter(|&tab| self.tab_offered(tab, &facts, cx)).collect()
    }

    /// The window's own rule for a tab whose feature has not moved into the table. Changes is always
    /// in the table; a window made without it has no such tab. Project is there for any terminal that
    /// has a folder, with or without a `.wormhole/`: it is where a folder gets its colour and, without
    /// one, where the project folder is made. It stays while it is the tab on show, so a terminal
    /// whose folder isn't known for a moment doesn't pull it out from under the person.
    pub(in crate::window) fn legacy_tab_offered(&self, tab: TabId, cx: &App) -> bool {
        match tab {
            TabId::Changes => false,
            TabId::History => true,
            // In the table.
            TabId::Notes => false,
            TabId::Usage => self.usage_tab_on(cx),
            // The Project tab belongs to the project-folder feature, so it's gone while that flag is
            // off (see `features`).
            TabId::Project => {
                crate::window::features::project::project_folders_on(cx)
                    && (self.active_folder(cx).is_some() || self.right_tab == TabId::Project)
            }
        }
    }

    /// The panel's tabs: ← → switch at once, ↩ / Space / ↓ go into the tab's list.
    pub(super) fn panel_tabs_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(delta) = along(keystroke) {
            let tabs = self.panel_tab_list(cx);
            let at = tabs.iter().position(|&tab| tab == self.right_tab).unwrap_or(0);
            self.show_tab(tabs[step_wrapping(at, delta, tabs.len())], cx);
            return true;
        }
        if activation(keystroke).is_some() || list_key(keystroke) == Some(Key::Down) {
            self.focus_stop(Stop { area: Area::PanelContent, index: 0 }, window, cx);
            return true;
        }
        false
    }

    /// The status bar, a toolbar: ← → between its items, ↩ / Space open what one counts.
    pub(super) fn status_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let items = self.status_items();
        if items.is_empty() {
            return false;
        }
        if let Some(delta) = along(keystroke) {
            self.areas.status = step_wrapping(self.areas.status.min(items.len() - 1), delta, items.len());
            return true;
        }
        if activation(keystroke).is_none() {
            return false;
        }
        if let Some(&item) = items.get(self.areas.status) {
            self.press_status_item(item, window, cx);
        }
        true
    }
}
