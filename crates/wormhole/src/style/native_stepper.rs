//! Shared presentation for AppKit steppers, with GPUI keyboard and accessibility focus.
use crate::platform::controls::{Stepper, Steppers};
use crate::settings::UiColors;
use crate::style::TextRole as _;
use gpui_kit::base::Button;
use gpui_kit::component::ThemeStyled as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;

pub(crate) type Change = Rc<dyn Fn(i32, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct NativeStepper {
    pub native: Rc<Steppers>,
    pub id: String,
    pub label: SharedString,
    pub value: String,
    pub enabled: bool,
    pub focus: FocusHandle,
    pub appearance: wormhole_core::theme::tokens::Appearance,
    pub ui: UiColors,
    pub change: Change,
}
impl RenderOnce for NativeStepper {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { native, id, label, value, enabled, focus, appearance, ui, change } = self;
        let size = native.size();
        let focused = focus.is_focused(window) && crate::style::native_focus::visible(window);
        let keyboard = change.clone();
        let up = change.clone();
        let down = change.clone();
        let probe_id = id.clone();
        let probe = canvas(
            move |bounds, window, _| {
                native.place(&Stepper {
                    id: &probe_id,
                    enabled,
                    bounds,
                    clip: window.content_mask().bounds,
                    appearance,
                })
            },
            |_, _, _, _| {},
        )
        .w(size.width)
        .h(size.height);
        Button::new(SharedString::from(id))
            .role(Role::SpinButton)
            .accessibility_label(label)
            .aria_value(value.clone())
            .track_focus(&focus)
            .disabled(!enabled)
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .p_1()
            .when(focused, |button| button.focus_ring_style(window, cx))
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                crate::style::native_focus::keyboard(window);
                if !enabled || event.keystroke.modifiers.modified() {
                    return;
                }
                let delta = match event.keystroke.key.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => return,
                };
                keyboard(delta, window, cx);
                cx.stop_propagation();
            })
            .when(enabled, |button| {
                button
                    .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| up(1, window, cx))
                    .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| down(-1, window, cx))
            })
            .child(div().w(px(56.)).text_label().tabular_nums().text_color(ui.text).child(value))
            .child(probe)
    }
}
