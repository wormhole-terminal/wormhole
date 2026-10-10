//! The settings window's nav: "Settings" under the traffic lights, the
//! search field with its count and clear button, then the sections under their group labels,
//! 28 high and full width, the current one on the sidebar's selected tint. While searching,
//! a section shows how many settings it holds that match, and dims when it holds none.

use std::collections::HashMap;

use wormhole_core::settings_index::{NavGroup, Query};

use super::*;
use style::NavMark;

/// The nav's width.
pub(super) const NAV_W: f32 = 220.0;

/// The icon of a section in the nav and in search results.
pub(super) fn section_icon(section: Section) -> IconName {
    match section {
        Section::Appearance => IconName::Palette,
        Section::Fonts => IconName::ALargeSmall,
        Section::Terminal => IconName::SquareTerminal,
        Section::Sessions => IconName::PanelLeft,
        Section::Git => IconName::GitCompare,
        Section::Agents => IconName::Sparkles,
        Section::Projects => IconName::Folder,
        Section::FeatureFlags => IconName::FlaskConical,
        Section::Notifications => IconName::Bell,
        Section::MenuBar => IconName::Command,
        Section::Updates => IconName::Download,
        Section::Advanced => IconName::SlidersHorizontal,
    }
}

impl SettingsWindow {
    pub(super) fn render_nav(&self, query: &Query, fill: Hsla, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let mut hits: HashMap<Section, usize> = HashMap::new();
        for hit in settings_index::search_visible(query, settings(cx)) {
            *hits.entry(hit.section).or_default() += 1;
        }
        let searching = !query.is_empty();
        let total: usize = hits.values().sum();
        let attention = self.agents.as_ref().is_some_and(|agents| agents.needs_repair() > 0);

        let mut nav = div()
            .id("settings-nav")
            .flex()
            .flex_col()
            .w(px(NAV_W))
            .flex_none()
            .h_full()
            .overflow_y_scroll()
            .pt(px(style::TITLE_ROW_H))
            .pb_4()
            .bg(fill)
            .rule_right(ui)
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(style::TITLE_ROW_H))
                    .px_4()
                    .child(style::panel_title(ui, "Settings")),
            )
            .child(
                div().px_4().pb_2().child(
                    Input::new(&self.search)
                        .small()
                        .cleanable(true)
                        .prefix(style::icon(IconName::Search).text_color(ui.muted))
                        .when(searching, |input| {
                            input.suffix(
                                div()
                                    .text_caption()
                                    .tabular_nums()
                                    .text_color(ui.muted)
                                    .child(total.to_string()),
                            )
                        }),
                ),
            );
        let mut group = None;
        // Only the sections shown for this build: the Preview group (feature flags, and a feature's
        // own section while its flag is on) is there only in a dev build, so a release never lists it.
        for section in Section::visible(settings(cx)) {
            let nav_group = section.info().nav;
            if group != Some(nav_group) {
                group = Some(nav_group);
                nav = nav.child(group_label(ui, nav_group));
            }
            let mark = match hits.get(&section) {
                _ if !searching && section == Section::Agents && attention => NavMark::Attention,
                // A preview section wears its tag when not searching; search marks override it.
                _ if !searching && section.is_preview() => NavMark::Preview,
                _ if !searching => NavMark::None,
                Some(n) => NavMark::Hits(*n),
                None => NavMark::NoHits,
            };
            let selected = section == self.section && !self.show_results;
            nav = nav.child(
                style::nav_row(
                    SharedString::from(format!("settings-nav-{section:?}")),
                    ui,
                    section_icon(section),
                    section.title(),
                    selected,
                    mark,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.show_section(section, cx))),
            );
        }
        nav.into_any_element()
    }
}

/// A group's label in the nav ("LOOK"): caption caps at x 16, at the bottom of a 28 row.
#[track_caller]
fn group_label(ui: UiColors, group: NavGroup) -> Div {
    div().flex().items_end().h(px(style::ROW_H)).px_4().pb_1().child(style::section_label(ui, group.title()))
}
