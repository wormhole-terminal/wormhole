//! The settings window's nav: "Settings" under the traffic lights, the search field with its count and
//! clear button, then the profile card and the sections in runs set apart by a clear step, the open one
//! with a sub-row for each group of its page (`groups`). The rows are the `SettingsNav` component
//! (`ui/settings_nav.dc.html`, which is the design: 28 high, 2 between them, 20 between runs, the current
//! one on the primary); this gives it its entries (`entries`). While searching, a section shows how many
//! settings it holds that match, and dims when it holds none, and the sub-rows give way to that.
//!
//! What to list and in which order is core's (`SectionId::visible`, `NavGroup`); this draws it.

mod entries;
mod groups;

pub(super) use groups::GroupPick;

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use std::collections::HashMap;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::Query;
use wormhole_core::config::settings_index::{self, SectionId as Section};

use super::{SettingsWindow, ui};
use crate::app_state::app;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::token;
use crate::ui::{Look, SettingsNavProps};

/// The nav's width.
pub(super) const NAV_W: f32 = 240.0;

/// The space between the profile card and the first run of rows.
const PROFILE_GAP: f32 = 16.0;

/// The nav's inset: the profile card lines up with the component's rows.
const LIST_INSET: f32 = 12.0;

/// The icon of a section in the nav and in search results.
pub(super) fn section_icon(section: Section) -> IconName {
    match section {
        Section::Profile => IconName::CircleUser,
        Section::Changed => IconName::RotateCcw,
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
    /// `changed` is how many settings differ from their defaults: the Changed page's badge, and whether the
    /// page is listed at all (it is while one does, and while it is the page that is open).
    pub(super) fn render_nav(
        &self,
        query: &Query,
        changed: usize,
        fill: Hsla,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let mut hits: HashMap<Section, usize> = HashMap::new();
        for hit in settings_index::search_visible(query, settings(cx)) {
            *hits.entry(hit.section).or_default() += 1;
        }
        let searching = !query.is_empty();
        let total: usize = hits.values().sum();
        let rows = self.nav_entries(ui, &hits, searching, changed, cx);
        let profile = self.render_profile_card(ui, searching, hits.get(&Section::Profile), cx);
        let list = div()
            .flex_none()
            .px(px(LIST_INSET))
            .pb(px(PROFILE_GAP))
            .child(profile)
            .child(div().h(px(PROFILE_GAP)))
            .into_any_element();
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
                    .child(list)
                    .child(SettingsNavProps { rows }.render(&Look::of(cx))),
            )
            .into_any_element()
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
        // A neutral grey, the Advanced page's: the page is about all the others at once.
        Section::Changed => "settings.nav.tile.advanced",
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

/// The colour of a page's tile in the nav, which a list on another page may borrow for its own tiles (the
/// Probes under Extensions wear Extensions' colour).
pub(super) fn page_tile(section: Section, cx: &App) -> Hsla {
    let tokens = &cx.global::<SettingsStore>().tokens;
    tile_token(section).map_or_else(|| token(tokens, "tile.background"), |name| token(tokens, name))
}
