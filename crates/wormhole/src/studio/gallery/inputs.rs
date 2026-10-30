//! Form controls in their states.

use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::component::radio::Radio;
use gpui_kit::prelude::*;
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
                .child(div().text_caption().text_color(ui.muted).child("← a real input")),
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
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(studio.checkbox("g-check-off", "Checkbox", false, env))
                                    .child("Checkbox"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(studio.checkbox("g-check-on", "Checked", true, env))
                                    .child("Checked"),
                            )
                            .when(!studio.has_native_controls(), |row| {
                                row.child(
                                    style::checkbox("g-check-ring", ui, false)
                                        .in_form("Focused")
                                        .focus_ring(true),
                                )
                            }),
                    ),
                )
                .child(
                    env.mark(
                        "switch",
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(studio.switch("g-switch-off", "Switch", false, true, env))
                                    .child("Switch"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(studio.switch("g-switch-on", "On", true, true, env))
                                    .child("On"),
                            ),
                    ),
                )
                .child(Radio::new("g-radio-off").label("Radio"))
                .child(Radio::new("g-radio-on").checked(true).label("Selected")),
        )
        .when(studio.has_native_controls(), |inputs| {
            inputs.child(
                div()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("macOS controls use the system accent colour."),
            )
        });
    inputs.into_any_element()
}
