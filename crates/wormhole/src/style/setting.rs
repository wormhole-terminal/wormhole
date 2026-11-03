//! The settings form: the [`setting`] row with its states, a group's head, and the
//! controls that more than one page needs (a stepper, a slider with its value, a link, the
//! shortcut recorder). The settings window builds every row of its pages from these.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use std::rc::Rc;

use super::scale::{CONTROL_H, TextRole};
use crate::settings::UiColors;
use crate::ui::{Handler, Look, StepperProps, SwitchControlProps};

/// A small whole number with a minus and a plus: one outlined control as tall as a
/// select, the value between hairlines in the middle. Add nothing: `minus` and `plus` are
/// the click handlers. `id` names the two buttons.
#[track_caller]
pub fn stepper(
    _: UiColors,
    id: &'static str,
    value: String,
    minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> StepperView {
    StepperView { id, value, minus: Rc::new(minus), plus: Rc::new(plus) }
}

/// A [`stepper`], drawn by the design's `Stepper` component.
#[derive(IntoElement)]
pub struct StepperView {
    id: &'static str,
    value: String,
    minus: Handler,
    plus: Handler,
}

impl RenderOnce for StepperView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        StepperProps {
            id: self.id.into(),
            label: wormhole_text::text(wormhole_text::keys::SETTINGS_WINDOW_STEPPER_LABEL).into(),
            smaller: wormhole_text::text(wormhole_text::keys::SETTINGS_WINDOW_STEPPER_SMALLER).into(),
            larger: wormhole_text::text(wormhole_text::keys::SETTINGS_WINDOW_STEPPER_LARGER).into(),
            value: self.value.into(),
            on_smaller: Some(self.minus),
            on_larger: Some(self.plus),
        }
        .render(&Look::of(cx))
    }
}

/// What a switch does with the state it is flipped to.
type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A switch. `on_change` gets the new state. Drawn by the `SwitchControl` component (the design's 34 by 20
/// switch); a click flips it, and so do Space and Return, since GPUI sends a focused element those as clicks.
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    checked: bool,
    on_change: OnToggle,
    label: Option<SharedString>,
    disabled: bool,
}

pub fn toggle(
    id: impl Into<ElementId>,
    checked: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Toggle {
    Toggle { id: id.into(), checked, on_change: Rc::new(on_change), label: None, disabled: false }
}

impl Toggle {}

impl gpui_kit::component::Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { id, checked, on_change, label, disabled } = self;
        let click: Handler = Rc::new(move |_, window, cx| on_change(!checked, window, cx));
        SwitchControlProps {
            id,
            label: label.unwrap_or_default(),
            on: checked,
            disabled,
            on_click: (!disabled).then_some(click),
        }
        .render(&Look::of(cx))
    }
}

/// A text button in the link color ("Edit in Fonts", "Open Notifications"), with an arrow
/// after it when it takes you to another page. Its text lines up with the edge it is placed
/// at, like any other text: the hover fill reaches [`LINK_PAD`] past it on both sides. Add
/// `.on_click(…)`.
#[track_caller]
pub fn link_button(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    arrow: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .group(LINK_GROUP)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(CONTROL_H))
        .text_label()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.link)
        .cursor_pointer()
        // The fill is laid over nothing: out of the layout, so the text keeps its place.
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(-LINK_PAD))
                .right(px(-LINK_PAD))
                .rounded(ui.radius)
                .group_hover(LINK_GROUP, move |d| d.bg(ui.hover)),
        )
        .child(label.into())
        .when(arrow, |d| d.child(super::buttons::icon(IconName::ArrowRight)))
}

/// The group a [`link_button`]'s hover fill answers to.
const LINK_GROUP: &str = "link-button";

/// The room between a [`link_button`]'s text and the edge of its hover fill.
const LINK_PAD: f32 = 4.0;
