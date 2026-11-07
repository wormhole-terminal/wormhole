//! The card a choice is drawn on when it is a picture rather than a word: a theme on the first run's Look
//! step or in Settings, an answer to one of the first run's questions. It is flat, as System Settings draws
//! a choice: a filled rounded card with no outline, a little lighter under the pointer, the chosen one
//! using the selection tint. Keyboard focus gets
//! its own ring. To a screen reader it is a radio button, chosen or not.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::motion::SelectionFill;
use crate::settings::UiColors;
use crate::style::Edges;

/// A choice card, a column for its picture and the strip under it. Add `.on_click(…)`, an accessible
/// name (`.aria_label(…)`) and the parts.
///
/// The card uses the group's fill, the selection tint while chosen, and the control fill under the
/// pointer. Increase Contrast adds an outline, with the primary for the chosen card. Keyboard focus
/// uses the focus ring independently of selection.
#[track_caller]
pub fn choice_card(id: impl Into<ElementId>, ui: UiColors, chosen: bool) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::RadioButton)
        .aria_selected(chosen)
        .aria_toggled(if chosen { Toggled::True } else { Toggled::False })
        .focusable()
        .tab_stop(true)
        .focus_visible(move |d| d.focus_outline(ui))
        .relative()
        .flex()
        .flex_col()
        .min_w_0()
        .rounded(ui.radius_lg)
        .cursor_pointer()
        .bg(ui.group)
        .text_color(ui.text)
        .hover(move |d| if chosen { d } else { d.bg(ui.control) })
        .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.card_edge))
        .when(chosen && ui.increase_contrast, |d| d.outlined_always(ui, ui.primary))
        .child(SelectionFill { selected: chosen, color: ui.selected, radius: ui.radius_lg })
}
