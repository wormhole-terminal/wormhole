//! The rows of the settings nav, like a sidebar of System Settings: the profile card, a row for each
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

use super::lead;
use crate::settings::UiColors;
use crate::style::labels::count_badge;
use crate::style::scale::{
    Edges, MARK_DOT, NAV_GROUP_GAP, NAV_LEAD_W, NAV_ROW_GAP, NAV_STEP, ROW_H, SWATCH, TITLE_GLYPH, TextRole,
};

/// What a [`nav_row`] says beside its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMark {
    None,
    /// Something on the page wants attention (an agent's hooks are incomplete): an amber dot.
    Attention,
    /// A search found this many settings on the page.
    Hits(usize),
    /// A search found nothing on the page: the row dims.
    NoHits,
    /// A feature-flagged section still being built: an outlined "Preview" tag, so it's obvious
    /// the section isn't a shipped one.
    Preview,
}

/// A page of the nav: what [`nav_row`] draws, as data.
pub struct NavItem {
    pub icon: IconName,
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

/// The keyboard ring of a row, drawn inside the nav's inset.
fn focus_ring<E: StatefulInteractiveElement>(row: E, ui: UiColors) -> E {
    row.focus_visible(move |d| d.ringed(ui, ui.focus, ui.focus_ring_width))
}

/// A page of the settings nav: [`ROW_H`] tall, its 16 pt icon in the lead column and its name (13) a gap
/// after it. The current page is the opaque tint of the sidebar's selected row, its icon and name (600)
/// in the tint's text color. Add `.on_click(…)`.
#[track_caller]
pub fn nav_row(id: impl Into<ElementId>, ui: UiColors, item: NavItem) -> Stateful<Div> {
    let NavItem { icon, label, spoken, selected, mark, opened } = item;
    let dim = mark == NavMark::NoHits;
    let (icon_color, text_color) = match (selected, dim) {
        (true, _) => (ui.selected_text, ui.selected_text),
        (false, true) => (ui.faint, ui.faint),
        (false, false) => (ui.muted, ui.text),
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
        .when(selected, |d| d.bg(ui.selected_row))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(lead(Icon::new(icon).with_size(px(TITLE_GLYPH)).text_color(icon_color)))
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
                d.child(div().flex_none().size(px(MARK_DOT)).rounded_full().bg(ui.agent_attention))
            }
            NavMark::Hits(n) => d.child(count_badge(ui, crate::style::scale::count_label(n))),
            NavMark::Preview => d.child(crate::style::status_badge(ui, ui.info, "Preview")),
            NavMark::None | NavMark::NoHits => d,
        });
    focus_ring(row, ui)
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
    focus_ring(row, ui)
}

/// The profile card above the nav's rows: its avatar, its name and "Profile" under it, for the page of the
/// profile Wormhole is in. Selected, it takes the same tint as a row. `hits` is the count a search found on
/// its page. Add `.on_click(…)`.
#[track_caller]
pub fn nav_profile_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    avatar: impl IntoElement,
    name: impl Into<SharedString>,
    caption: impl Into<SharedString>,
    selected: bool,
    hits: Option<usize>,
) -> Stateful<Div> {
    let (name, caption): (SharedString, SharedString) = (name.into(), caption.into());
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
        .when(selected, |d| d.bg(ui.selected_row))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(avatar)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_label()
                        .text_color(if selected { ui.selected_text } else { ui.text })
                        .child(name),
                )
                .child(
                    div()
                        .truncate()
                        .text_caption()
                        .text_color(if selected { ui.selected_text } else { ui.muted })
                        .child(caption),
                ),
        )
        .children(hits.map(|n| count_badge(ui, crate::style::scale::count_label(n))));
    focus_ring(row, ui)
}
