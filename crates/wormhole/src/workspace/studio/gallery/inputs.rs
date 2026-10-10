//! Form controls in their states.

use super::*;
use gpui_kit::component::radio::Radio;
use gpui_kit::component::switch::Switch;

pub(super) fn render(studio: &StudioWindow, env: &Env, _cx: &mut Context<StudioWindow>) -> AnyElement {
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
                        // The app's own checkbox (`checkbox.border`), not the Kit's.
                        div()
                            .flex()
                            .gap_4()
                            .child(style::checkbox("g-check-off", ui, false).in_form("Checkbox"))
                            .child(style::checkbox("g-check-on", ui, true).in_form("Checked"))
                            .child(
                                style::checkbox("g-check-ring", ui, false)
                                    .in_form("Focused")
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
                            .child(Switch::new("g-switch-off").label("Switch"))
                            .child(Switch::new("g-switch-on").checked(true).label("On")),
                    ),
                )
                .child(Radio::new("g-radio-off").label("Radio"))
                .child(Radio::new("g-radio-on").checked(true).label("Selected")),
        );
    inputs.into_any_element()
}
