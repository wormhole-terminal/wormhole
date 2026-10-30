//! Shared native slider presentation, including keyboard and accessibility actions.
use crate::platform::controls::{Slider, Sliders};
use gpui_kit::base::Button;
use gpui_kit::component::ThemeStyled as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;

pub(crate) type Change = Rc<dyn Fn(f64, &mut Window, &mut App)>;

/// Accessible value changes must obey the same bounds and increments as pointer input.
fn accessible_value(value: f64, min: f64, max: f64, step: f64) -> Option<f64> {
    value.is_finite().then(|| (min + ((value.clamp(min, max) - min) / step).round() * step).clamp(min, max))
}
#[derive(IntoElement)]
pub(crate) struct NativeSlider {
    pub native: Rc<Sliders>,
    pub id: String,
    pub label: SharedString,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub enabled: bool,
    pub focus: FocusHandle,
    pub appearance: wormhole_core::theme::tokens::Appearance,
    pub change: Change,
}
impl RenderOnce for NativeSlider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { native, id, label, value, min, max, step, enabled, focus, appearance, change } = self;
        let focused = focus.is_focused(window) && crate::style::native_focus::visible(window);
        let up = change.clone();
        let down = change.clone();
        let set = change.clone();
        let keyboard = change;
        let probe_id = id.clone();
        let probe = canvas(
            move |bounds, window, _| {
                native.place(&Slider {
                    id: &probe_id,
                    value,
                    min,
                    max,
                    enabled,
                    bounds,
                    clip: window.content_mask().bounds,
                    appearance,
                })
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        Button::new(SharedString::from(id))
            .role(Role::Slider)
            .accessibility_label(label)
            .aria_numeric_value(value)
            .aria_min_numeric_value(min)
            .aria_max_numeric_value(max)
            .aria_numeric_value_step(step)
            .track_focus(&focus)
            .disabled(!enabled)
            .relative()
            .w_full()
            .h(px(24.))
            .when(focused, |button| button.focus_ring_style(window, cx))
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                crate::style::native_focus::keyboard(window);
                if !enabled || event.keystroke.modifiers.modified() {
                    return;
                }
                let next = match event.keystroke.key.as_str() {
                    "up" | "right" => (value + step).min(max),
                    "down" | "left" => (value - step).max(min),
                    "home" => min,
                    "end" => max,
                    _ => return,
                };
                keyboard(next, window, cx);
                cx.stop_propagation();
            })
            .when(enabled, |button| {
                button
                    .on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                        let value = match data {
                            Some(gpui_kit::accesskit::ActionData::NumericValue(value)) => Some(*value),
                            Some(gpui_kit::accesskit::ActionData::Value(value)) => value.parse().ok(),
                            _ => None,
                        };
                        if let Some(value) = value.and_then(|value| accessible_value(value, min, max, step)) {
                            set(value, window, cx);
                        }
                    })
                    .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
                        up((value + step).min(max), window, cx)
                    })
                    .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
                        down((value - step).max(min), window, cx)
                    })
            })
            .child(probe)
    }
}

#[cfg(test)]
mod tests {
    use super::accessible_value;

    #[test]
    fn accessible_values_are_quantized_clamped_and_finite() {
        assert!((accessible_value(1.53, 1.0, 2.0, 0.05).unwrap() - 1.55).abs() < 0.0001);
        assert_eq!(accessible_value(4.0, 1.0, 2.0, 0.05), Some(2.0));
        assert_eq!(accessible_value(f64::NAN, 1.0, 2.0, 0.05), None);
    }
}
