//! The right panel as the window draws it: the column with its tab bar and the shown tab's body,
//! and which tab is shown. The tabs' contents are their features' (`feature`); this is the frame
//! around them, and the one place `right_tab` changes.
use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::view::layout::Edge;
use wormhole_core::view::motion::tabs::{CONTENT_IN, content};

use crate::window::Workspace;

impl Workspace {
    /// The right panel, `width` points wide (its column's, or its width at rest while it slides).
    pub(in crate::window) fn render_panel(
        &self,
        width: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.panel_column(width, false, window, cx)
    }

    /// The panel's column. `bare` leaves its surface off, for a slide that fades the column's content over
    /// a surface of its own (`window::slide`).
    pub(in crate::window) fn panel_column(
        &self,
        width: f32,
        bare: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // The tab actually on screen: if the chosen one isn't in the tab list any more (the
        // Project tab vanished with its feature flag), show Changes until the next refresh resets it.
        let tab =
            if self.panel_tab_list(cx).contains(&self.right_tab) { self.right_tab } else { TabId::FALLBACK };
        let body = div().flex().flex_col().flex_1().min_h_0().child(match self.tab_entry(tab) {
            // A feature in the table draws its tab; a tab with a list gets the area that holds the
            // keyboard for it.
            Some((at, entry)) => match self.feature_entity(at) {
                Some(entity) => {
                    let content = (entry.view)(entity, &self.frame_facts(window, cx), window, cx);
                    if entry.walk.is_some() { self.panel_content(content, cx) } else { content }
                }
                None => div().into_any_element(),
            },
            // Every tab is a feature's; a window made without it has nothing to show.
            None => div().into_any_element(),
        });
        div()
            .relative()
            .flex()
            .flex_col()
            // Narrower than chosen while the window can't spare it (see core `Panes::columns`).
            .w(px(width))
            .flex_none()
            .h_full()
            .bg(if bare { transparent_black() } else { self.column_surface(Edge::Panel, cx) })
            .child(self.render_panel_tab_bar(window, cx))
            .child(self.slide_in_content(body, cx))
    }

    /// The shown tab's body, sliding in from the side of the tab it came from while a switch is playing
    /// (`panel_switch`), and as it is otherwise. Clipped to the panel only while it moves, so what a
    /// feature draws over its edges (a menu) is not cut at rest.
    fn slide_in_content(&self, body: Div, cx: &App) -> AnyElement {
        let Some((side, serial)) = self.regions.drawn.panel_switch.playing(cx.background_executor().now())
        else {
            return body.into_any_element();
        };
        body.with_animation(("panel-switch", serial), Animation::new(CONTENT_IN), move |body, t| {
            if t >= 1.0 {
                return body;
            }
            let pose = content(side, t);
            body.relative().left(px(pose.dx)).opacity(pose.opacity).overflow_hidden()
        })
        .into_any_element()
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

    /// The title row's branch chip was clicked: the feature that contributes it shows its tab and opens
    /// the branch picker it stands for.
    pub(in crate::window) fn pick_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((at, chip)) = self.features.iter().enumerate().find_map(|(at, def)| {
            let chip = def.branch_chip.as_ref()?;
            self.facts(cx).features.has(def.id).then_some((at, chip))
        }) else {
            return;
        };
        self.show_tab(chip.tab, cx);
        if let Some(entity) = self.feature_entity(at).cloned() {
            let facts = self.facts(cx);
            (chip.pick)(&entity, &facts, window, cx);
        }
    }

    /// The title row's branch status was clicked: shows the tab of the feature that contributes the
    /// branch chip, the one that has the changes the status counts.
    pub(in crate::window) fn show_branch_tab(&mut self, cx: &mut Context<Self>) {
        let tab = self.features.iter().find_map(|def| def.branch_chip.as_ref()).map(|chip| chip.tab);
        if let Some(tab) = tab {
            self.show_tab(tab, cx);
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
                    TabId::FALLBACK
                }
                _ => tab,
            }
        };
        if self.right_tab != tab && self.tab_entry(tab).is_some_and(|(_, entry)| entry.returns_to_previous) {
            self.tab_before_previous = Some(self.right_tab);
        }
        let (before, visible) = (self.right_tab, self.notes_visible());
        self.set_right_tab(tab);
        if before != tab {
            self.start_tab_switch(before, tab, visible, cx);
        }
        self.reveal_notes();
        self.remember_panes(cx);
        self.tabs_changed(tab, cx);
        let session = self.active_session().id;
        // The feature in the table says what showing its tab does.
        if let Some((at, entry)) = self.tab_entry(tab)
            && let (Some(on_show), Some(entity)) = (entry.on_show, self.feature_entity(at).cloned())
        {
            let facts = self.facts(cx);
            on_show(&entity, &facts, session, cx);
        }
        self.mark_dirty(cx);
    }
}
