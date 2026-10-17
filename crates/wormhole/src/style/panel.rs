//! The right panel's cards. Every tab of the panel is a stack of cards like the sidebar's
//! (`card_segment`: [`CARD_MARGIN`] in, [`CARD_PAD`] of padding, [`CARD_RADIUS`] round, 8
//! apart), each led by a head (`section_card`, [`BAND_H`] tall: chevron, title, count, +). A
//! long list draws its cards line by line (`panel_list::Rows::element_carded`); the short
//! ones, whose content is all there at once, are built with [`panel_card`] and [`Cards`].

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::tooltip_delay;
use super::rows::card_segment;
use super::scale::Edges as _;
use crate::settings::UiColors;
use wormhole_core::layout::PANEL_STRIP;

/// One whole card round `content`: a note where a list would be, a block of the Usage tab.
#[track_caller]
pub fn panel_card(ui: UiColors, content: impl IntoElement) -> Div {
    card_segment(ui, true, true, true, false, content)
}

/// A column of cards built from lines: [`Cards::head`] starts a card and every
/// [`Cards::child`] after it goes into it, until the next head. For the tabs whose content is
/// short and not virtual (Project).
pub struct Cards {
    ui: UiColors,
    cards: Vec<Vec<AnyElement>>,
}

impl Cards {
    pub fn new(ui: UiColors) -> Self {
        Self { ui, cards: Vec::new() }
    }

    /// Start a new card with `head` as its first line.
    pub fn head(mut self, head: impl IntoElement) -> Self {
        self.cards.push(vec![head.into_any_element()]);
        self
    }

    /// Add a line to the card being built (the first one, if no head came before it).
    pub fn child(mut self, line: impl IntoElement) -> Self {
        if self.cards.is_empty() {
            self.cards.push(Vec::new());
        }
        if let Some(card) = self.cards.last_mut() {
            card.push(line.into_any_element());
        }
        self
    }
}

impl IntoElement for Cards {
    type Element = Div;

    fn into_element(self) -> Div {
        let ui = self.ui;
        div().flex().flex_col().children(
            self.cards.into_iter().map(|lines| panel_card(ui, div().flex().flex_col().children(lines))),
        )
    }
}

// ---------------------------------------------------------------- the strip

/// The strip's buttons: their side, the gap between them, their icon, and how far under the
/// window's top the first one is, so its centre is level with the first card's head (the panel's
/// top 6, the card's hairline and padding 5, half the head 16: 27).
pub const STRIP_BUTTON: f32 = 22.0;
pub const STRIP_GAP: f32 = 2.0;
pub const STRIP_ICON: f32 = 13.0;
pub const STRIP_TOP: f32 = 16.0;
/// The room under the last button at the strip's foot.
pub const STRIP_FOOT: f32 = 4.0;
/// The amber dot on a tab where something is new, and the ring that sets it off the button.
const STRIP_DOT: f32 = 5.0;
const STRIP_DOT_INSET: f32 = 1.0;

/// The right panel's tabs in a strip down the window's right edge: [`PANEL_STRIP`] wide, on the
/// panel's surface, a hairline on its left. Put the tabs in with `.child(…)`, then a
/// `div().flex_1()` and the foot's button.
#[track_caller]
pub fn panel_strip(ui: UiColors) -> Div {
    div()
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(STRIP_GAP))
        .w(px(PANEL_STRIP))
        .h_full()
        .pt(px(STRIP_TOP))
        .pb(px(STRIP_FOOT))
        .bg(ui.panel_right)
        .rule_left(ui)
}

/// A button of the strip: [`STRIP_BUTTON`] square with the chip corners and a [`STRIP_ICON`]
/// icon. The open tab is the accent at 16 % (`state.selected`) with its icon in `selected_text`;
/// the others are muted and take the hover fill. `new` is the amber dot where something has
/// changed that the person hasn't seen. The tooltip names the tab, since nothing else does.
#[track_caller]
pub fn strip_tab(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: IconName,
    tooltip: impl Into<SharedString>,
    open: bool,
    new: bool,
) -> Stateful<Div> {
    let tooltip: SharedString = tooltip.into();
    tooltip_delay(
        div()
            .id(id)
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(STRIP_BUTTON))
            .rounded(ui.radius_chip)
            .cursor_pointer()
            .text_color(if open { ui.selected_text } else { ui.muted })
            .when(open, |d| d.bg(ui.selected))
            .hover(move |d| if open { d } else { d.bg(ui.band_hover).text_color(ui.text) })
            .active(move |d| if open { d } else { d.bg(ui.active) })
            .tooltip(move |window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
            }),
    )
    .child(Icon::new(icon).with_size(px(STRIP_ICON)))
    .when(new, |d| {
        d.child(
            div()
                .absolute()
                .top(px(STRIP_DOT_INSET))
                .right(px(STRIP_DOT_INSET))
                .size(px(STRIP_DOT))
                .rounded_full()
                .bg(ui.agent_attention)
                .border_2()
                .border_color(ui.panel_right),
        )
    })
}
