//! The settings window's nav: "Settings" under the traffic lights, the search field with its count and
//! clear button, then the profile card and the sections in runs set apart by a clear step, the open one
//! with a sub-row for each group of its page (`groups`). The rows are 28 high with 2 between them and 20
//! between runs, inset 8 from the nav's edges (rounded, like System Settings), the current one on the
//! sidebar's selected tint. While searching, a section shows how many settings it holds that match, and
//! dims when it holds none, and the sub-rows give way to that.
//!
//! What to list and in which order is core's (`SectionId::visible`, `NavGroup`); this draws it.

mod groups;

pub(super) use groups::GroupPick;

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use std::collections::HashMap;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::{NavItem, NavMark};
use wormhole_core::config::settings_index::Query;
use wormhole_core::config::settings_index::{self, SectionId as Section};

use super::{SettingsWindow, ui};
use crate::app_state::app;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::token;

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

        // The Changed page (a later step of the redesign) is a run of its own between the profile card and
        // the first section, shown only while something differs: it joins the list here, ahead of the loop.
        let mut list =
            style::nav_list().child(self.render_profile_card(ui, searching, hits.get(&Section::Profile), cx));
        let mut run = None;
        // Only the sections shown for this build: the Preview group (feature flags, and a feature's
        // own section while its flag is on) is there only in a dev build, so a release never lists it.
        let tokens = &cx.global::<SettingsStore>().tokens;
        for section in Section::visible(settings(cx)) {
            // The profile is the card above, with an avatar of its own.
            let Some(tile) = tile_token(section) else { continue };
            let nav_group = section.info().nav;
            if run != Some(nav_group) {
                run = Some(nav_group);
                list = list.child(style::nav_group_gap());
                if !nav_group.title().is_empty() {
                    list = list.child(style::nav_group_label(ui, nav_group.title()));
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
            // A search shows what it found, not the page's groups.
            let subs = selected && !searching;
            list = list.child(
                style::nav_row(
                    ("settings-nav", section as usize),
                    ui,
                    NavItem {
                        icon: section_icon(section),
                        tile: token(tokens, tile),
                        label: section.title().into(),
                        spoken: spoken(section.title(), mark),
                        selected,
                        mark,
                        opened: subs && self.nav_groups(section, settings(cx)).next().is_some(),
                    },
                )
                .on_click(cx.listener(move |this, _, _, cx| this.show_section(section, cx))),
            );
            if subs {
                list = list.child(self.render_sub_rows(section, ui, cx));
            }
        }
        div()
            .id("settings-nav")
            .flex()
            .flex_col()
            .w(px(NAV_W))
            .flex_none()
            .h_full()
            .pt(px(style::TITLE_ROW_H))
            .bg(fill)
            .rule_right(ui)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(px(style::TITLE_ROW_H))
                    .px_4()
                    .child(style::panel_title(ui, text(SETTINGS_WINDOW_SETTINGS))),
            )
            .child(
                div().flex_none().px_4().pt_2().pb_2().child(
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
            // The list scrolls under the search when the open page's sub-rows run long.
            .child(
                div()
                    .id("settings-nav-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb_4()
                    .on_key_down(cx.listener(Self::walk_nav))
                    .child(list),
            )
            .into_any_element()
    }

    /// The sub-rows of the open page: a row for each group, in a step of `_1` above and below.
    fn render_sub_rows(&self, section: Section, ui: UiColors, cx: &Context<Self>) -> Div {
        let mut subs = style::nav_sub_list();
        let first = self.nav_groups(section, settings(cx)).next().map(|(at, _)| at);
        let current = self.current_group(first);
        for (at, group) in self.nav_groups(section, settings(cx)) {
            subs = subs.child(
                style::nav_sub_row(("settings-nav-group", at), ui, group.title(), current == Some(at))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_group(at, cx))),
            );
        }
        subs
    }

    /// ↑ and ↓ walk the nav's rows, the tab stops one after another, as ⇥ and ⇧⇥ do.
    fn walk_nav(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        if keys.modifiers.modified() {
            return;
        }
        match keys.key.as_str() {
            "down" => window.focus_next(cx),
            "up" => window.focus_prev(cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// Whose settings these are, and the way to their page: the avatar and name of the profile Wormhole is in.
    fn render_profile_card(
        &self,
        ui: UiColors,
        searching: bool,
        hits: Option<&usize>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let state = app(cx);
        let profile = state.profiles.active();
        let color = crate::theme_bridge::hsla(profile.color.color(&cx.global::<SettingsStore>().tokens));
        style::nav_profile_row(
            "settings-nav-profile",
            ui,
            style::NavProfile {
                color,
                letter: profile.avatar,
                name: profile.name.clone().into(),
                caption: Section::Profile.title().into(),
                selected: self.section == Section::Profile && !self.show_results,
                hits: hits.copied().filter(|_| searching),
            },
        )
        .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::Profile, cx)))
    }
}

/// The token of a section's tile, which the theme sets: a colour of its own for each page of the list.
fn tile_token(section: Section) -> Option<&'static str> {
    Some(match section {
        Section::Profile => return None,
        Section::Terminal => "settings.nav.tile.terminal",
        Section::Appearance => "settings.nav.tile.appearance",
        Section::Fonts => "settings.nav.tile.fonts",
        Section::Sessions => "settings.nav.tile.sidebar",
        Section::Git => "settings.nav.tile.git",
        Section::Notifications => "settings.nav.tile.notifications",
        Section::Agents => "settings.nav.tile.agents",
        Section::MenuBar => "settings.nav.tile.menubar",
        Section::Import => "settings.nav.tile.import",
        Section::Updates => "settings.nav.tile.updates",
        Section::Advanced => "settings.nav.tile.advanced",
        Section::Motion => "settings.nav.tile.motion",
        Section::Projects => "settings.nav.tile.projects",
        Section::Extensions => "settings.nav.tile.extensions",
        Section::FeatureFlags => "settings.nav.tile.flags",
    })
}

/// What VoiceOver reads for a row: its name, and what the row marks. A name with no mark is the
/// catalog's own text, which costs no allocation.
fn spoken(name: &'static str, mark: NavMark) -> SharedString {
    match mark {
        NavMark::None => SharedString::new_static(name),
        NavMark::Attention => settings_nav_spoken_attention(name).into(),
        NavMark::Preview => settings_nav_spoken_preview(name).into(),
        NavMark::NoHits => settings_nav_spoken_no_matches(name).into(),
        NavMark::Hits(n) => settings_nav_spoken_matches(name, whole(n)).into(),
    }
}
