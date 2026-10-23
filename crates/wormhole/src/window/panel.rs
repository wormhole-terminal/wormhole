//! The right panel as the window draws it: the column with its tab bar and the shown tab's body,
//! and which tab is shown. The tabs' contents are their features' (`feature`); this is the frame
//! around them, and the one place `right_tab` changes.
use gpui_kit::*;
use wormhole_core::config::TabId;

use crate::settings::ui_in;
use crate::window::Workspace;

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
                Some((at, entry)) => match self.feature_entity(at) {
                    Some(entity) => {
                        let content = (entry.view)(entity, &self.frame_facts(window, cx), window, cx);
                        if entry.walk.is_some() { self.panel_content(content, cx) } else { content }
                    }
                    None => div().into_any_element(),
                },
                None => match tab {
                    // Virtualized lists fill the panel and scroll themselves; the list area holds the
                    // keyboard for them.
                    TabId::History => {
                        let list = self.render_history_tab(window, cx);
                        self.panel_content(list, cx)
                    }
                    // Notes is in the table.
                    TabId::Notes => div().into_any_element(),
                    // In the table.
                    TabId::Project | TabId::Usage => div().into_any_element(),
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

    /// Whether the tab on show wants the window redrawn on this tick (its words change with time).
    pub(in crate::window) fn tab_refresh_due(&self) -> bool {
        self.notes_visible()
            && self
                .tab_entry(self.right_tab)
                .and_then(|(_, entry)| entry.refresh_ticks)
                .is_some_and(|every| self.ticks.is_multiple_of(every))
    }

    /// Opens the branch picker of `tab`, asked from outside the tab (the title row's branch chip): the
    /// tab is shown first.
    pub(in crate::window) fn pick_branch_in(
        &mut self,
        tab: TabId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_tab(tab, cx);
        if let Some((at, entry)) = self.tab_entry(tab)
            && let (Some(pick), Some(entity)) = (entry.pick_branch, self.feature_entity(at).cloned())
        {
            let facts = self.facts(cx);
            pick(&entity, &facts, window, cx);
        }
    }

    pub(crate) fn show_tab(&mut self, tab: TabId, cx: &mut Context<Self>) {
        // A tab of a feature that is off, or that is not offered now (Usage without an agent), is gone.
        let tab = {
            let facts = self.facts(cx);
            match self.tab_entry(tab) {
                Some((at, entry))
                    if !facts.features.has(self.features[at].id) || !(entry.shown)(&facts, cx) =>
                {
                    TabId::Changes
                }
                _ => tab,
            }
        };
        if self.right_tab != tab && self.tab_entry(tab).is_some_and(|(_, entry)| entry.returns_to_previous) {
            self.tab_before_previous = Some(self.right_tab);
        }
        self.set_right_tab(tab);
        self.reveal_notes();
        self.remember_panes(cx);
        self.history_picker.close();
        self.tabs_changed(tab, cx);
        let session = self.active_session().id;
        match self.tab_entry(tab) {
            // The feature in the table says what showing its tab does.
            Some((at, entry)) => {
                if let (Some(on_show), Some(entity)) = (entry.on_show, self.feature_entity(at).cloned()) {
                    let facts = self.facts(cx);
                    on_show(&entity, &facts, session, cx);
                }
            }
            None => match tab {
                TabId::History => self.refresh_history(self.active, cx),
                TabId::Changes | TabId::Notes | TabId::Usage | TabId::Project => {}
            },
        }
        self.mark_dirty(cx);
    }
}
