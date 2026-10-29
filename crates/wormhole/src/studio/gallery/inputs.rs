//! Form controls in their states.

use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::component::radio::Radio;
use gpui_kit::component::switch::Switch;
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::style::{self, TextRole};

pub(super) fn render(
    studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let inputs = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_2()
                .child(env.mark("input", div().w(px(220.0)).child(Input::new(&studio.sample_input).small())))
                .child(div().text_caption().text_color(ui.muted).child("← a real input")), // text: sample (the gallery's specimen)
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_4()
                .child(
                    env.mark(
                        "checkbox",
                        // The app's own checkbox (`checkbox.border`), not the Kit's.
                        div()
                            .flex()
                            .gap_4()
                            .child(style::checkbox("g-check-off", ui, false).in_form("Checkbox")) // text: sample (the gallery's specimen)
                            .child(style::checkbox("g-check-on", ui, true).in_form("Checked")) // text: sample (the gallery's specimen)
                            .child(
                                style::checkbox("g-check-ring", ui, false)
                                    .in_form("Focused") // text: sample (the gallery's specimen)
                                    .focus_ring(true),
                            ),
                    ),
                )
                .child(
                    env.mark(
                        "switch",
                        div()
                            .flex()
                            .gap_4()
                            .child(Switch::new("g-switch-off").label("Switch")) // text: sample (the gallery's specimen)
                            .child(Switch::new("g-switch-on").checked(true).label("On")), // text: sample (the gallery's specimen)
                    ),
                )
                .child(Radio::new("g-radio-off").label("Radio")) // text: sample (the gallery's specimen)
                .child(Radio::new("g-radio-on").checked(true).label("Selected")), // text: sample (the gallery's specimen)
        );
    inputs.into_any_element()
}
