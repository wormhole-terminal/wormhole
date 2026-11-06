//! The card a choice is drawn on when it is a picture rather than a word: a theme on the first run's Look
//! step or in Settings, an answer to one of the first run's questions. It is flat, as System Settings draws
//! a choice: a filled rounded card with no outline, a little lighter under the pointer, the chosen one
//! wearing a soft ring of the primary round it (and its label in the primary, which the caller sets). To a
//! screen reader it is a radio button, chosen or not.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::text_field::{FLAT_RING_ALPHA, flat_ring_round};
use crate::settings::UiColors;
use crate::style::Edges;

/// A choice card, a column for its picture and the strip under it. Add `.on_click(…)`, an accessible
/// name (`.aria_label(…)`) and the parts.
///
/// The card is `settings.group` with `control.background` under the pointer. The ring is an overlay of its
/// own just outside the card's edge, not a shadow, because the fill is translucent and a shadow shows
/// through it as a solid block; so the card does not clip its children, and one that sits flush against
/// its edge rounds its own outer corners to `radius_lg`. Increase Contrast draws the card's outline at rest
/// and the ring solid; Borders off changes nothing, there is no line at rest to take away.
#[track_caller]
pub fn choice_card(id: impl Into<ElementId>, ui: UiColors, chosen: bool) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::RadioButton)
        .aria_selected(chosen)
        .relative()
        .flex()
        .flex_col()
        .min_w_0()
        .rounded(ui.radius_lg)
        .cursor_pointer()
        .bg(ui.group)
        .hover(move |d| d.bg(ui.control))
        .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.card_edge))
        .when(chosen, |d| {
            let alpha = if ui.increase_contrast { 1.0 } else { FLAT_RING_ALPHA };
            d.child(flat_ring_round(ui, ui.primary, alpha, ui.radius_lg))
        })
}
