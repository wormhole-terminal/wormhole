//! A folder's colour, to pick (a row of choices in the Project tab) and to see (the dot in front of
//! a colour's name in the band's menu). The colours are the theme's: callers pass them in, from
//! core's `FolderColor::color`.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{COLOR_DOT, Edges, HIT_MIN};
use crate::settings::UiColors;

/// The mark of a colour: a `size` square in the small corners filled with `color` and outlined with
/// the hairline, so a pale colour on a pale surface still has an edge. In a menu it is
/// [`COLOR_DOT_MENU`], in a [`color_choice`] [`COLOR_DOT`].
#[track_caller]
pub fn color_dot(ui: UiColors, color: Hsla, size: f32) -> Div {
    div().flex_none().size(px(size)).rounded(ui.radius_sm).bg(color).outlined(ui, ui.border)
}

/// One choice in a row of folder colours: a [`HIT_MIN`] square holding the colour's [`color_dot`],
/// with a `text.primary` ring round it when it is the chosen one (a gap inside, so the ring reads
/// on any colour, as for [`swatch`](super::swatch)). `None` is "no colour": an empty outline with
/// a slash. `name` is what VoiceOver says. Add `.on_click(…)` and a tooltip. Where the colour can't
/// be changed (`enabled` false: the project sets it) the choices neither light up nor show a pointer
/// (don't give them a click) and are dimmed, all but the chosen one, which keeps its colour so it
/// still reads.
#[track_caller]
pub fn color_choice(
    id: impl Into<ElementId>,
    ui: UiColors,
    name: impl Into<SharedString>,
    color: Option<Hsla>,
    chosen: bool,
    enabled: bool,
) -> Stateful<Div> {
    let ring = UiColors { border_width: px(RING), ..ui };
    let inner = COLOR_DOT + 2.0 * (RING + RING_GAP);
    let dot = match color {
        Some(color) => color_dot(ui, color, COLOR_DOT),
        None => div()
            .flex_none()
            .size(px(COLOR_DOT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.radius_sm)
            .outlined(ui, ui.border_strong)
            .text_color(ui.muted)
            .child(Icon::new(IconName::Ban).xsmall()),
    };
    div()
        .id(id)
        .role(Role::RadioButton)
        .aria_label(name.into())
        .aria_selected(chosen)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(HIT_MIN))
        .rounded(ui.radius)
        .when(enabled, |d| d.cursor_pointer().hover(move |d| d.bg(ui.band_hover)))
        .when(!enabled && !chosen, |d| d.opacity(DISABLED_OPACITY))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(inner))
                .rounded(ui.radius_sm + px(RING + RING_GAP))
                .when(chosen, |d| d.outlined(ring, ui.text))
                .child(dot),
        )
}

/// How much of itself a [`color_choice`] keeps where it can't be used.
const DISABLED_OPACITY: f32 = 0.5;
/// The ring round a chosen [`color_choice`] and the gap inside it.
const RING: f32 = 2.0;
const RING_GAP: f32 = 2.0;
