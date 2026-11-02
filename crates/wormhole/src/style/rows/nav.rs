//! The rows of the settings nav, like a sidebar of System Settings: the profile card, a tile and name for each
//! page, and under the open page a sub-row for each of its groups. All are [`ROW_H`] tall with
//! [`NAV_ROW_GAP`] between them and a [`NAV_GROUP_GAP`] between runs, in a container inset by `_2`
//! (`nav_list`), so their rounded hover and selection float in from the nav's edges.
//!
//! Each is a tab stop and a button for VoiceOver, named by what it says (`spoken`): the keys reach it
//! through the window's own focus order. The icon is decoration, so a name is not read twice.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::labels::{badge, count_badge};
use crate::style::scale::{
    Edges, MARK_DOT, NAV_GROUP_GAP, NAV_LEAD_W, NAV_ROW_GAP, NAV_STEP, NAV_TILE, NAV_TILE_ICON, ROW_H,
    SWATCH, TextRole,
};
use crate::theme_bridge;
use wormhole_core::theme::tokens::Color;

/// How alike two colours may be before a tile on the selection would vanish into it (their contrast ratio).
const TILE_SAME: f32 = 1.5;
/// The white wash a tile takes on the selection when its own colour is the selection's.
const TILE_WASH: f32 = 0.25;

/// What a [`nav_row`] says beside its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMark {
    None,
    /// Something on the page wants attention (an agent's hooks are incomplete): an amber dot.
    Attention,
    /// A search found this many settings on the page.
    Hits(usize),
    /// This many settings differ from their defaults: the Changed page's count.
    Changed(usize),
    /// A search found nothing on the page: the row dims.
    NoHits,
    /// A feature-flagged section still being built: a dot in the info colour, since the nav's Preview
    /// heading says what it is.
    Preview,
}

/// A page of the nav: what [`nav_row`] draws, as data.
pub struct NavItem {
    pub icon: IconName,
    /// The tile's colour under the white icon (`settings.nav.tile.*`).
    pub tile: Hsla,
    pub label: SharedString,
    /// What VoiceOver reads: the label, and what the row marks (a count, "needs attention").
    pub spoken: SharedString,
    pub selected: bool,
    pub mark: NavMark,
    /// The sub-rows of its groups are shown under it.
    pub opened: bool,
}

/// The container of a run of rows: inset by `_2` from the nav's edges, [`NAV_ROW_GAP`] between rows.
#[track_caller]
pub fn nav_list() -> Div {
    div().flex_none().flex().flex_col().gap(px(NAV_ROW_GAP)).px_2()
}

/// The step between two runs of rows. The list's own gaps are on both sides of it.
#[track_caller]
pub fn nav_group_gap() -> Div {
    div().flex_none().h(px(NAV_GROUP_GAP - 2.0 * NAV_ROW_GAP))
}

/// The sub-rows of the open page, a step of `_1` below its row and above the next.
#[track_caller]
pub fn nav_sub_list() -> Div {
    div().flex_none().flex().flex_col().gap(px(NAV_ROW_GAP)).py_1()
}

/// The keyboard ring of a row, drawn inside the nav's inset. On the selection, which is the primary and so
/// the ring's own colour, it is the text colour instead.
fn focus_ring<E: StatefulInteractiveElement>(row: E, ui: UiColors, selected: bool) -> E {
    let color = if selected { ui.text } else { ui.focus };
    row.focus_visible(move |d| d.ringed(ui, color, ui.focus_ring_width))
}

/// What a tile is filled with: its colour, unless the row is the selection and the tile is nearly the
/// selection's own colour (the Appearance tile on the default primary), when it is a wash of the icon's
/// white so it still reads as a tile.
fn nav_tile_fill(ui: UiColors, tile: Hsla, selected: bool) -> Hsla {
    let alike =
        Color::contrast_ratio(theme_bridge::color(tile), theme_bridge::color(ui.nav_selected)) < TILE_SAME;
    if selected && alike { ui.tile_mark_brand.opacity(TILE_WASH) } else { tile }
}

/// The count of a search on the selected row: a wash of the text colour, so it reads on the primary.
fn selected_count(ui: UiColors, text: impl Into<SharedString>) -> Div {
    badge(ui)
        .px_1()
        .bg(ui.on_primary.opacity(TILE_WASH))
        .text_color(ui.on_primary)
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

/// A page of the settings nav: [`ROW_H`] tall, its coloured tile in the lead column with the page's line
/// icon in white, and its name (13) a gap after it. The current page is the primary itself, its name (600)
/// in `text.on_primary`. Add `.on_click(…)`.
#[track_caller]
pub fn nav_row(id: impl Into<ElementId>, ui: UiColors, item: NavItem) -> Stateful<Div> {
    let NavItem { icon, tile, label, spoken, selected, mark, opened } = item;
    let dim = mark == NavMark::NoHits;
    let text_color = match (selected, dim) {
        (true, _) => ui.on_primary,
        (false, true) => ui.faint,
        (false, false) => ui.text,
    };
    let row = div()
        .id(id)
        .role(Role::Button)
        .aria_label(spoken)
        .aria_selected(selected)
        .when(opened, |d| d.aria_expanded(true))
        .focusable()
        .tab_stop(true)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(ROW_H))
        .px_2()
        .rounded(ui.radius_sm)
        .text_label()
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.nav_selected))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        // Decoration: the row's name says what the icon would.
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(px(NAV_TILE))
                .rounded(ui.radius)
                .bg(nav_tile_fill(ui, tile, selected))
                .when(dim, |d| d.opacity(0.5))
                .child(Icon::new(icon).with_size(px(NAV_TILE_ICON)).text_color(ui.tile_mark_brand)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .text_color(text_color)
                .child(label),
        )
        .map(|d| match mark {
            NavMark::Attention => {
                d.child(div().flex_none().size(px(MARK_DOT)).rounded_full().bg(if selected {
                    ui.on_primary
                } else {
                    ui.agent_attention
                }))
            }
            NavMark::Hits(n) | NavMark::Changed(n) if selected => {
                d.child(selected_count(ui, crate::style::scale::count_label(n)))
            }
            NavMark::Hits(n) | NavMark::Changed(n) => {
                d.child(count_badge(ui, crate::style::scale::count_label(n)))
            }
            // A dot, not the word: the word took the room the page's own name needs ("Project fold…"), and
            // the nav's Preview heading and the page's badge already say it. VoiceOver still hears it.
            NavMark::Preview => {
                d.child(div().flex_none().size(px(MARK_DOT)).rounded_full().bg(if selected {
                    ui.on_primary
                } else {
                    ui.info
                }))
            }
            NavMark::None | NavMark::NoHits => d,
        });
    focus_ring(row, ui, selected)
}

/// A group of the open page, under its row: [`ROW_H`] tall, its name starting where the row's does. The
/// group last picked is in the text color and 600; the others are muted. Add `.on_click(…)`.
#[track_caller]
pub fn nav_sub_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    current: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let row = div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .aria_selected(current)
        .focusable()
        .tab_stop(true)
        .flex()
        .items_center()
        .w_full()
        .h(px(ROW_H))
        // Under the name of the row above: its padding, its lead and the gap after it.
        .pl(px(NAV_STEP + NAV_LEAD_W + NAV_STEP))
        .pr_2()
        .rounded(ui.radius_sm)
        .text_label()
        .cursor_pointer()
        .text_color(if current { ui.text } else { ui.muted })
        .font_weight(if current { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .hover(move |d| d.bg(ui.hover))
        .child(div().min_w_0().truncate().child(label));
    focus_ring(row, ui, false)
}

/// The profile of the nav's card: its colour and letter, its name and the caption under it.
pub struct NavProfile {
    pub color: Hsla,
    pub letter: char,
    pub name: SharedString,
    pub caption: SharedString,
    pub selected: bool,
    /// The count a search found on its page.
    pub hits: Option<usize>,
}

/// The profile card above the nav's rows: its avatar, its name and "Profile" under it, for the page of the
/// profile Wormhole is in. Selected, it is the primary like a row, its avatar a wash of white. Add
/// `.on_click(…)`.
#[track_caller]
pub fn nav_profile_row(id: impl Into<ElementId>, ui: UiColors, profile: NavProfile) -> Stateful<Div> {
    let NavProfile { color, letter, name, caption, selected, hits } = profile;
    let (title, quiet) = if selected { (ui.on_primary, ui.on_primary) } else { (ui.text, ui.muted) };
    let avatar = if selected {
        div()
            .flex_none()
            .size(px(SWATCH))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(ui.tile_mark_brand.opacity(TILE_WASH))
            .text_color(ui.on_primary)
            .text_body()
            .font_weight(FontWeight::BOLD)
            .child(letter.to_uppercase().to_string())
            .into_any_element()
    } else {
        crate::style::profile_avatar(ui, color, letter, SWATCH).into_any_element()
    };
    let row = div()
        .id(id)
        .role(Role::Button)
        .aria_label(SharedString::from(format!("{caption}: {name}")))
        .aria_selected(selected)
        .focusable()
        .tab_stop(true)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(SWATCH + NAV_STEP * 2.0))
        .px_2()
        .rounded(ui.radius_sm)
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.nav_selected))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(avatar)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().text_label().text_color(title).child(name))
                .child(div().truncate().text_caption().text_color(quiet).child(caption)),
        )
        .children(hits.map(|n| {
            let text = crate::style::scale::count_label(n);
            if selected { selected_count(ui, text) } else { count_badge(ui, text) }
        }));
    focus_ring(row, ui, selected)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Hsla, rgb};

    use super::nav_tile_fill;
    use crate::settings::UiColors;

    fn colors() -> UiColors {
        UiColors {
            nav_selected: Hsla::from(rgb(0x0096ff)),
            tile_mark_brand: Hsla::from(rgb(0xffffff)),
            ..UiColors::default()
        }
    }

    #[test]
    fn a_tile_keeps_its_colour_on_the_selection_unless_that_is_the_selections_own() {
        let ui = colors();
        let appearance = Hsla::from(rgb(0x0a84ff));
        let git = Hsla::from(rgb(0xff9f0a));
        assert_eq!(nav_tile_fill(ui, git, true), git, "a different colour stays");
        assert_eq!(nav_tile_fill(ui, appearance, false), appearance, "off the selection it stays");
        let washed = nav_tile_fill(ui, appearance, true);
        assert_ne!(washed, appearance, "the blue tile on the blue selection would vanish");
        assert!((washed.a - 0.25).abs() < 1e-6 && washed.l > 0.99, "a wash of the icon's white: {washed:?}");
    }
}
