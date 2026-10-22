//! GPUI Kit's button variants and sizes.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let buttons = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(env.mark("button", Button::new("g-default").label("Button")))
                .child(env.mark("button.primary", Button::new("g-primary").primary().label("Primary")))
                .child(Button::new("g-ghost").ghost().label("Ghost"))
                .child(Button::new("g-outline").outline().label("Outline"))
                .child(Button::new("g-link").link().label("Link")),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(Button::new("g-success").success().label("Success"))
                .child(Button::new("g-warning").warning().label("Warning"))
                .child(Button::new("g-danger").danger().label("Danger"))
                .child(Button::new("g-info").info().label("Info"))
                .child(Button::new("g-loading").primary().loading(true).label("Loading")),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(Button::new("g-xs").primary().xsmall().label("xsmall"))
                .child(Button::new("g-sm").primary().small().label("small"))
                .child(Button::new("g-md").primary().label("medium"))
                .child(Button::new("g-lg").primary().large().label("large")),
        );
    buttons.into_any_element()
}
