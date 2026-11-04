//! The settings window's nav: "Settings" under the traffic lights, the
//! search field with its count and clear button, the profile select (the Profile section's entry, so it
//! has no row of its own), then the sections under their group labels,
//! 28 high and inset 8 from the nav's edges (rounded, like System Settings), the current one on
//! the sidebar's selected tint. Each group starts a clear step below the one before. While searching,
//! a section shows how many settings it holds that match, and dims when it holds none.

use std::collections::HashMap;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::NavMark;
use wormhole_core::config::settings_index::Query;
use wormhole_core::config::settings_index::{self, SectionId as Section};

use super::{SettingsWindow, ui};
use crate::app_state::app;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};

/// The nav's width.
pub(super) const NAV_W: f32 = 220.0;

/// The icon of a section in the nav and in search results.
pub(super) fn section_icon(section: Section) -> IconName {
    match section {
        Section::Profile => IconName::CircleUser,
        Section::Appearance => IconName::Palette,
        Section::Fonts => IconName::ALargeSmall,
        Section::Motion => IconName::Play,
        Section::Terminal => IconName::SquareTerminal,
        Section::Sessions => IconName::PanelLeft,
        Section::Git => IconName::GitCompare,
        Section::Agents => IconName::Sparkles,
        Section::Import => IconName::ArrowDownToLine,
        Section::Projects => IconName::Folder,
        Section::Extensions => IconName::Puzzle,
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

        // A 4 pt gap between the items, so a row's selected fill doesn't touch the next one's.
        let mut rows = div().flex_none().flex().flex_col().gap_1().px_2();
        let mut group = None;
        // Only the sections shown for this build: the Preview group (feature flags, and a feature's
        // own section while its flag is on) is there only in a dev build, so a release never lists it.
        for section in Section::visible(settings(cx)) {
            // The profile select above the list is this section's entry; a row would repeat it.
            if section == Section::Profile {
                continue;
            }
            let nav_group = section.info().nav;
            if group != Some(nav_group) {
                group = Some(nav_group);
                if !nav_group.title().is_empty() {
                    rows = rows.child(style::nav_group_label(ui, nav_group.title()));
                }
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
            rows = rows.child(
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
        div()
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
                // Inset 8 like the rows below (`px_2`), so the field's edges line up with their fill.
                // 12 under it keeps the select from crowding the field.
                div().flex_none().px_2().pt_2().pb_3().child(
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
            )
            .child(self.render_profile_select(ui, self.section == Section::Profile && !self.show_results, cx))
            .child(rows)
            .into_any_element()
    }
}

impl SettingsWindow {
    /// Whose settings these are: the profile Wormhole is in, drawn as a pop-up button that is quiet (muted,
    /// nothing to choose from here) and still clickable: it opens the Profile page, and it is the nav's
    /// only entry for it, so on that page it wears a nav row's selected look.
    fn render_profile_select(&self, ui: UiColors, selected: bool, cx: &mut Context<Self>) -> AnyElement {
        let state = app(cx);
        let profile = state.profiles.active();
        let color = crate::theme_bridge::hsla(profile.color.color(&cx.global::<SettingsStore>().tokens));
        div()
            .flex_none()
            // The rows' 8 pt inset, and 12 below to the first group label.
            .px_2()
            .pb_3()
            .child(
                div()
                    .id("settings-profile-select")
                    .role(Role::Button)
                    .aria_label(Section::Profile.title())
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(style::CONTROL_H))
                    .px_2()
                    .rounded(ui.radius)
                    // The ring takes the fill's colour when selected, so the box keeps its size.
                    .outlined(ui, if selected { ui.selected_row } else { ui.border })
                    .cursor_pointer()
                    .when(selected, |d| d.bg(ui.selected_row))
                    .hover(move |d| if selected { d } else { d.bg(ui.band_hover) })
                    .child(style::profile_avatar(ui, color, profile.avatar, style::TITLE_GLYPH))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_label()
                            .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                            .text_color(if selected { ui.selected_text } else { ui.muted })
                            .child(profile.name.clone()),
                    )
                    .child(style::icon(IconName::ChevronsUpDown).text_color(if selected {
                        ui.selected_text
                    } else {
                        ui.muted
                    }))
                    .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::Profile, cx))),
            )
            .into_any_element()
    }
}
