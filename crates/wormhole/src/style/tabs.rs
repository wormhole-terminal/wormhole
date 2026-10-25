//! The tab bar a window shows instead of the sidebar (Settings → Show terminals in → Tabs), as
//! RF-02 draws it: one 38 pt row with the group switcher (the group's name, up-down chevrons, an amber dot while another group needs input) and that group's
//! terminals as tabs. A tab is 28 tall and 172 wide: a 16 pt status tile, the name in 12.5, and the
//! × while the pointer is over it (the open one too; it takes its room from the name then). Every
//! tab is a box on the terminal's colour: the open one in a 14 % fill, the others in a 5 % one.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::controls::keycap;
use super::groups::PopoverTrigger;
use super::scale::{MARK_DOT, TAB_W, TextRole};
use crate::settings::UiColors;

/// The parts of a [`terminal_tab`].
pub struct TerminalTab {
    /// A `style::status_tile` at [`super::TILE_SMALL`].
    pub tile: AnyElement,
    pub name: SharedString,
    pub selected: bool,
    /// The ⌘N keycap while ⌘ is held.
    pub hint: Option<SharedString>,
    /// The close × (the app's, since closing asks first): laid over the tab's right end, shown while
    /// the pointer is over the tab, the open one included. Hidden while the ⌘ keycap shows.
    pub close: Option<AnyElement>,
    /// A name too long for the tab: cut at [`TAB_NAME_W`] and faded out over its end.
    pub name_fade: Option<super::rows::NameFade>,
    /// The name under the pointer, with the × at the tab's end: cut shorter, faded out before it.
    pub name_hover: Option<super::rows::NameFade>,
}

/// What a tab leaves its name: its width less its padding, the tile and the gap after it.
pub const TAB_NAME_W: f32 = TAB_W - 2.0 * 8.0 - super::scale::TILE_SMALL - 8.0;

/// One terminal's tab: its status tile, then its name, in a box of the text colour; the open one
/// in an [`OPEN_FILL`] with the name in the text colour, the others in a [`CLOSED_FILL`] with the
/// name muted and a hover fill. The tab group named by
/// [`TAB_GROUP`] drives the × on hover. Add `.on_click(…)`.
#[track_caller]
pub fn terminal_tab(id: impl Into<ElementId>, ui: UiColors, tab: TerminalTab) -> Stateful<Div> {
    let TerminalTab { tile, name, selected, hint, close, name_fade, name_hover } = tab;
    let name_hover = name_hover.map(|fade| (fade, SharedString::from(TAB_GROUP)));
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .h(px(TAB_H))
        // The same 8 before the tile and after the end. Every tab is as wide, so its name's room is
        // known and a long one fades out over its end; too many to fit, and the row scrolls.
        .px_2()
        .rounded(ui.radius)
        .overflow_hidden()
        .flex_none()
        .w(px(TAB_W))
        .cursor_pointer()
        // Every tab is a box, so each reads on its own on the terminal's colour: the open one filled
        // most. No edge on either, so switching tabs moves nothing.
        .bg(ui.text.opacity(if selected { OPEN_FILL } else { CLOSED_FILL }))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(tile)
        .child(
            super::rows::name_text(name.clone(), name_fade.as_ref(), name_hover.as_ref())
                .flex_1()
                .ml_2()
                .text_label()
                // One weight for every tab: a bolder name on the open one would change how much of
                // it fits, and selecting another tab must move nothing.
                .font_weight(FontWeight::MEDIUM)
                .text_color(if selected { ui.text } else { ui.muted }),
        )
        .children(hint.map(|hint| div().flex_none().ml_2().child(keycap(ui, hint))))
        .children(close)
}

/// The hairline between two tabs that are not the open one, and after the switcher: 14 tall, in the
/// rule's colour. Beside the open tab, whose fill is its edge, the caller makes it transparent: it
/// always takes its 1 pt, so selecting another tab moves nothing.
#[track_caller]
pub fn tab_divider(ui: UiColors, tall: f32) -> Div {
    div().flex_none().w(px(1.0)).h(px(tall)).bg(ui.border)
}

/// The parts of a [`group_switcher`].
pub struct GroupSwitch {
    pub name: SharedString,
    /// Another group has a terminal that needs input (or failed): the amber dot at the corner.
    pub alert: bool,
    pub spoken: SharedString,
}

/// The tab bar's group switcher: 26 tall, the group's name in 13 semibold and up-down chevrons (no
/// dot or count: the popover's cards have them; VoiceOver still hears the count). Wrap it in a `Popover` as the trigger (it is the one thing in
/// the row that is not a tab), or add `.on_click(…)`.
#[track_caller]
pub fn group_switcher(id: impl Into<ElementId>, ui: UiColors, parts: GroupSwitch) -> PopoverTrigger {
    let GroupSwitch { name, alert, spoken } = parts;
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
        .child(div().max_w(px(SWITCHER_NAME_MAX)).truncate().text_label().text_color(ui.text).child(name))
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

/// A tab's height, the switcher's, and the tabs' fills: the text colour at these shares, the open
/// one's enough to set it apart from the closed ones, theirs enough to hold each one's box.
pub const TAB_H: f32 = 28.0;
pub const SWITCHER_H: f32 = 26.0;
const OPEN_FILL: f32 = 0.14;
const CLOSED_FILL: f32 = 0.05;
const SWITCHER_NAME_MAX: f32 = 160.0;
/// How far the amber dot sits from the switcher's corner.
const ALERT_INSET: f32 = 0.0;
const ALERT_RING: f32 = 2.0;

/// The tab group a tab names itself with, so its × can show on hover.
pub const TAB_GROUP: &str = "terminal-tab";
