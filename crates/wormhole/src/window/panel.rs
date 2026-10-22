//! The right panel as the window draws it: the column with its tab bar and the shown tab's body,
//! and which tab is shown. The tabs' contents are their features' (`feature`); this is the frame
//! around them, and the one place `right_tab` changes.
use gpui_kit::*;
use wormhole_core::config::TabId;

use crate::settings::ui_in;
use crate::window::{Workspace, panel_list};

impl Workspace {
    /// The right panel, `width` points wide (its column's, or its width at rest while it slides).
    pub(in crate::window) fn render_panel(
        &self,
        width: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ui = ui_in(&self.profile, cx);
        // The tab actually on screen: if the chosen one isn't in the tab list any more (the
        // Project tab vanished with its feature flag), show Changes until the next refresh resets it.
        let tab =
            if self.panel_tab_list(cx).contains(&self.right_tab) { self.right_tab } else { TabId::Changes };
        div()
            .relative()
            .flex()
            .flex_col()
            // Narrower than chosen while the window can't spare it (see core `Panes::columns`).
            .w(px(width))
            .flex_none()
            .h_full()
            .bg(ui.panel_right)
            .child(self.render_panel_tab_bar(window, cx))
            .child(div().flex().flex_col().flex_1().min_h_0().child(match self.tab_entry(tab) {
                // A feature in the table draws its tab; a tab with a list gets the area that holds the
                // keyboard for it.
                Some((at, entry)) => match (entry.window_side, self.feature_entity(at)) {
                    // The temporary side door (Changes until 5d): the window draws the tab itself.
                    (Some(side), _) => {
                        let content = (side.view)(self, window, cx);
                        self.panel_content(content, cx)
                    }
                    (None, Some(entity)) => {
                        let content = (entry.view)(entity, &self.facts(cx), window, cx);
                        if entry.walk.is_some() { self.panel_content(content, cx) } else { content }
                    }
                    (None, None) => div().into_any_element(),
                },
                None => match tab {
                    // Virtualized lists fill the panel and scroll themselves; the list area holds the
                    // keyboard for them.
                    TabId::History => {
                        let list = self.render_history_tab(window, cx);
                        self.panel_content(list, cx)
                    }
                    TabId::Notes => {
                        let list = self.render_notes_tab(window, cx);
                        self.panel_content(list, cx)
                    }
                    // Short tabs, all there at once: a plain scrolling column.
                    TabId::Project => {
                        let content = self.render_project_tab(cx).into_any_element();
                        panel_list::scroll_column("project-scroll", &self.panel_scroll, ui, content, cx)
                    }
                    TabId::Usage => {
                        let content = self.render_usage_tab(cx).into_any_element();
                        panel_list::scroll_column("usage-scroll", &self.panel_scroll, ui, content, cx)
                    }
                    // Changes is always in the table; a window made without it has nothing to show.
                    TabId::Changes => div().into_any_element(),
                },
            }))
    }

    /// Makes `tab` the one the panel shows. A feature's text field only has the keys while its tab
    /// is on show, so a change of tab ends that, whoever asks for it; every change of `right_tab`
    /// after the window is made goes through here.
    pub(in crate::window) fn set_right_tab(&mut self, tab: TabId) {
        if tab != self.right_tab {
            self.panel_scroll.set_offset(Point::default());
            self.feature_editing = false;
        }
        self.right_tab = tab;
    }

    pub(crate) fn show_tab(&mut self, tab: TabId, cx: &mut Context<Self>) {
        // The Notes tab is gone while its flag is off.
        let tab = if tab == TabId::Notes && !crate::window::features::notes::notes_on(cx) {
            TabId::Changes
        } else {
            tab
        };
        let tab = if tab == TabId::Usage && !self.usage_tab_on(cx) { TabId::Changes } else { tab };
        if tab == TabId::Usage && self.right_tab != TabId::Usage {
            self.tab_before_usage = Some(self.right_tab);
        }
        self.set_right_tab(tab);
        self.reveal_notes();
        self.remember_panes(cx);
        self.picker = None;
        let session = self.active_session().id;
        match self.tab_entry(tab) {
            // The feature in the table says what showing its tab does.
            Some((at, entry)) => {
                if let Some(side) = entry.window_side {
                    (side.on_show)(self, session, cx);
                } else if let (Some(on_show), Some(entity)) =
                    (entry.on_show, self.feature_entity(at).cloned())
                {
                    on_show(&entity, session, cx);
                }
            }
            None => match tab {
                TabId::History => self.refresh_history(self.active, cx),
                TabId::Project => self.refresh_projects(cx),
                TabId::Changes | TabId::Notes | TabId::Usage => {}
            },
        }
        self.mark_dirty(cx);
    }
}
