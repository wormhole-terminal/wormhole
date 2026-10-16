//! The keys of the areas that are a row of buttons or tabs: the title row's buttons, the
//! terminal's title strip, the right panel's tabs and the status bar's items. ← → move along them,
//! wrapping; tabs open as the keys reach them (automatic activation); ↩ / Space press the button
//! or item the keys are on.

use super::session::step_wrapping;
use super::*;

/// The title row's buttons, in the focus order (region 1). The traffic lights are the system's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::workspace) enum TitleButton {
    /// Hide (or show) the sidebar.
    Sidebar,
    Settings,
    /// Group and filter: the funnel's popover.
    Filter,
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
    /// The title row's buttons on screen: all four with the sidebar open, its show button alone
    /// without it. The funnel is a stop only while its feature flag is on.
    pub(in crate::workspace) fn title_buttons(&self, cx: &App) -> Vec<TitleButton> {
        if self.sidebar.tabs.layout.is_some() {
            // Under tabs the window has no sidebar, nor a button for it.
            Vec::new()
        } else if self.panes.sidebar_open {
            let mut buttons = vec![TitleButton::Sidebar, TitleButton::Settings];
            if super::sidebar::sidebar_filter_on(cx) {
                buttons.push(TitleButton::Filter);
            }
            buttons.push(TitleButton::NewTerminal);
            buttons
        } else {
            vec![TitleButton::Sidebar]
        }
    }

    /// Whether `button` wears the focus ring.
    pub(in crate::workspace) fn title_ring(&self, button: TitleButton, window: &Window, cx: &App) -> bool {
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
            Some(TitleButton::Settings) => self.toggle_settings(window, cx),
            Some(TitleButton::Filter) => self.sidebar.filter_open = !self.sidebar.filter_open,
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

    /// The right panel's tabs as they are on screen. Project is there for any terminal that has a
    /// folder, with or without a `.wormhole/`: it is where a folder gets its colour and, without
    /// one, where the project folder is made. It stays while it is the tab on show, so a terminal
    /// whose folder isn't known for a moment doesn't pull it out from under the person.
    pub(in crate::workspace) fn panel_tab_list(&self, cx: &App) -> Vec<RightTab> {
        let mut tabs = vec![RightTab::Changes, RightTab::History, RightTab::Notes, RightTab::Usage];
        // The Project tab belongs to the project-folder feature, so it's gone while that flag is
        // off (see `features`). It comes last so that the other tabs keep their place, and their
        // keys, whether it is there or not.
        let project_folders = super::project::project_folders_on(cx);
        if project_folders && (self.active_folder(cx).is_some() || self.right_tab == RightTab::Project) {
            tabs.push(RightTab::Project);
        }
        tabs
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
