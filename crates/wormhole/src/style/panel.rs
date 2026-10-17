//! The right panel's cards. Every tab of the panel is a stack of cards like the sidebar's
//! (`card_segment`: [`CARD_MARGIN`] in, [`CARD_PAD`] of padding, [`CARD_RADIUS`] round, 8
//! apart), each led by a head (`section_card`, [`BAND_H`] tall: chevron, title, count, +). A
//! long list draws its cards line by line (`panel_list::Rows::element_carded`); the short
//! ones, whose content is all there at once, are built with [`panel_card`] and [`Cards`].

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::rows::card_segment;
use crate::settings::UiColors;

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
