//! The card a choice is drawn on when it is a picture rather than a word: a theme on the first run's Look
//! step, an answer to one of its questions. The chosen one is ringed in the primary, the others outlined
//! and darker on hover; to a screen reader it is a radio button, chosen or not.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::Edges;

/// The width of the chosen card's ring.
const RING: f32 = 2.0;

/// A choice card, a column for its picture and the strip under it. Add `.on_click(…)`, an accessible
/// name (`.aria_label(…)`) and the parts.
#[track_caller]
pub fn choice_card(id: impl Into<ElementId>, ui: UiColors, chosen: bool) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::RadioButton)
        .aria_selected(chosen)
        .flex()
        .flex_col()
        .min_w_0()
        .rounded(ui.radius)
        .overflow_hidden()
        .cursor_pointer()
        // The ring is a shadow drawn behind the card: without a fill of its own the card fills with it.
        .bg(ui.elevated)
        .map(|d| if chosen { d.ringed(ui, ui.primary, px(RING)) } else { d.outlined(ui, ui.border) })
        .when(!chosen, |d| d.hover(|d| d.border_color(ui.border_strong)))
}
