//! The tab bar a window shows instead of the sidebar (Settings → Show terminals in → Tabs), as
//! RF-02 draws it: one 36 pt row with the group switcher (the group's colour dot, its name and
//! count, up-down chevrons, an amber dot while another group needs you) and that group's
//! terminals as tabs. A tab is 28 tall and 172 wide: a 16 pt status tile, the name in 12.5, and the
//! × on the open one; the open tab is a 10 % fill, the others sit apart by a hairline.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::controls::keycap;
use super::groups::PopoverTrigger;
use super::scale::{MARK_DOT, TAB_MIN_W, TAB_W, TextRole};
use crate::settings::UiColors;

/// The parts of a [`terminal_tab`].
pub struct TerminalTab {
    /// A `style::status_tile` at [`super::TILE_SMALL`].
    pub tile: AnyElement,
    pub name: SharedString,
    pub selected: bool,
    /// The ⌘N keycap while ⌘ is held.
    pub hint: Option<SharedString>,
    /// The close × (the app's, since closing asks first): shown on the open tab, and on another
    /// while the pointer is over it. Hidden while the ⌘ keycap shows.
    pub close: Option<AnyElement>,
}

/// One terminal's tab: its status tile, then its name; the open one in a 10 % fill of the text
/// colour with the name in 600, the others in 500 with a hover fill. The tab group named by
/// [`TAB_GROUP`] drives the × on hover. Add `.on_click(…)`.
#[track_caller]
pub fn terminal_tab(id: impl Into<ElementId>, ui: UiColors, tab: TerminalTab) -> Stateful<Div> {
    let TerminalTab { tile, name, selected, hint, close } = tab;
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .h(px(TAB_H))
        .px_2()
        .rounded(ui.radius)
        .overflow_hidden()
        .flex_shrink_1()
        .flex_basis(px(TAB_W))
        .max_w(px(TAB_W))
        .min_w(px(TAB_MIN_W))
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.text.opacity(SELECTED_FILL)))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(tile)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_label()
                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                .text_color(if selected { ui.text } else { ui.muted })
                .child(name),
        )
        .children(hint.map(|hint| keycap(ui, hint)))
        .children(close)
}

/// The hairline between two tabs that are not the open one, and after the switcher: 14 tall, in the
/// rule's colour. Not drawn beside the open tab, whose fill is its edge.
#[track_caller]
pub fn tab_divider(ui: UiColors, tall: f32) -> Div {
    div().flex_none().w(px(1.0)).h(px(tall)).bg(ui.border)
}

/// The parts of a [`group_switcher`].
pub struct GroupSwitch {
    /// The group's colour (the dot before its name); none for "In no group" or a group without one.
    pub color: Option<Hsla>,
    pub name: SharedString,
    pub count: usize,
    /// Another group has a terminal that needs you (or failed): the amber dot at the corner.
    pub alert: bool,
    pub spoken: SharedString,
}

/// The tab bar's group switcher: 26 tall, the group's dot, its name in 13 semibold, its count in the
/// muted colour and up-down chevrons. Wrap it in a `Popover` as the trigger (it is the one thing in
/// the row that is not a tab), or add `.on_click(…)`.
#[track_caller]
pub fn group_switcher(id: impl Into<ElementId>, ui: UiColors, parts: GroupSwitch) -> PopoverTrigger {
    let GroupSwitch { color, name, count, alert, spoken } = parts;
    let button = div()
        .id(id)
        .role(Role::Button)
        .aria_label(spoken)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .h(px(SWITCHER_H))
        .px_2()
        .rounded(ui.radius)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.band_hover))
        .child(div().flex_none().size(px(DOT)).rounded_full().bg(color.unwrap_or(ui.faint)))
        .child(div().max_w(px(SWITCHER_NAME_MAX)).truncate().text_label().text_color(ui.text).child(name))
        .child(
            div()
                .flex_none()
                .text_label()
                .font_weight(FontWeight::NORMAL)
                .text_color(ui.muted)
                .child(count.to_string()),
        )
        .child(Icon::new(IconName::ChevronsUpDown).xsmall().text_color(ui.muted))
        .when(alert, |d| {
            // Ringed by the bar's colour, so it reads as a dot on the corner.
            let mut dot = div()
                .absolute()
                .top(px(ALERT_INSET))
                .right(px(ALERT_INSET))
                .size(px(MARK_DOT + 2.0 * ALERT_RING))
                .rounded_full()
                .bg(ui.agent_attention)
                .border_color(ui.panel);
            let ring = Some(px(ALERT_RING).into());
            let b = &mut dot.style().border_widths;
            (b.top, b.right, b.bottom, b.left) = (ring, ring, ring, ring);
            d.child(dot)
        });
    PopoverTrigger::new(button)
}

/// A tab's height, the switcher's, and how far the open tab's fill is from invisible: the text
/// colour at this share.
pub const TAB_H: f32 = 28.0;
pub const SWITCHER_H: f32 = 26.0;
const SELECTED_FILL: f32 = 0.10;
const SWITCHER_NAME_MAX: f32 = 160.0;
/// The group dot before the switcher's name.
const DOT: f32 = 8.0;
/// How far the amber dot sits from the switcher's corner.
const ALERT_INSET: f32 = 0.0;
const ALERT_RING: f32 = 2.0;

/// The tab group a tab names itself with, so its × can show on hover.
pub const TAB_GROUP: &str = "terminal-tab";
