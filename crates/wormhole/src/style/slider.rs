//! The slider of a settings row: a thin track, a round knob.
//!
//! GPUI Kit's `Slider` is drawn in the primary with a 6 pt track, a hover halo and no thumb when it
//! is disabled, and none of that is ours to change from outside, so this one is put together from the
//! Kit's unstyled parts (`gpui_kit::base`): the same state, the same pointer, drag and VoiceOver
//! behaviour, a different face. The track is the flat control's fill under the pointer (a step stronger than a resting control's, as a track is
//! thin and would be lost), the part up to the value the
//! primary, the knob the switch's, lifted by a soft shadow.

use gpui_kit::base::{Slider as BaseSlider, SliderIndicator, SliderThumb, SliderTrack};
use gpui_kit::component::slider::SliderState;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{CONTROL_H, Edges};
use crate::settings::UiColors;

/// The track's thickness, and the knob's diameter: the knob stands out of the track on both sides.
const TRACK_H: f32 = 4.0;
const KNOB: f32 = 18.0;
/// A disabled slider is drawn at this opacity, knob included, so it reads as held and not missing.
const DISABLED_OPACITY: f32 = 0.5;

/// The slider of a settings row: horizontal, one value, as wide as what holds it. Build it with
/// [`slider`], and give it the [`SliderState`] the page keeps.
#[derive(IntoElement)]
pub struct SettingSlider {
    ui: UiColors,
    state: Entity<SliderState>,
    disabled: bool,
}

/// A thin-track slider over `state`; see [`SettingSlider`].
#[track_caller]
pub fn slider(ui: UiColors, state: &Entity<SliderState>) -> SettingSlider {
    SettingSlider { ui, state: state.clone(), disabled: false }
}

impl SettingSlider {
    /// Takes no value from the pointer or from VoiceOver, and is drawn faint.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for SettingSlider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (ui, state, disabled) = (self.ui, self.state, self.disabled);
        let value = state.read(cx).percentage().end.clamp(0.0, 1.0);
        let knob_lift = px(-(KNOB - TRACK_H) / 2.0);
        let knob = SliderThumb::new(&state)
            .disabled(disabled)
            .absolute()
            .top(knob_lift)
            .left(relative(value))
            .ml(px(-KNOB / 2.0))
            .size(px(KNOB))
            .rounded_full()
            .bg(ui.knob)
            .shadow(vec![
                BoxShadow {
                    color: Hsla { a: 0.28, ..ui.shadow },
                    offset: point(px(0.0), px(1.0)),
                    blur_radius: px(3.0),
                    spread_radius: px(0.0),
                    inset: false,
                },
                BoxShadow {
                    color: Hsla { a: 0.12, ..ui.shadow },
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: px(0.5),
                    inset: false,
                },
            ])
            .when(!disabled, |d| d.cursor_pointer())
            // Increase Contrast: a knob white on white would otherwise lose its edge.
            .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.border_strong));
        let fill = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right(relative(1.0 - value))
            .rounded_full()
            .bg(ui.primary);
        let bar = SliderIndicator::new(&state)
            .relative()
            .w_full()
            .h(px(TRACK_H))
            .rounded_full()
            .bg(ui.control_hover)
            // Increase Contrast: the empty part of the track needs an edge to be seen at all.
            .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.border_strong))
            .child(fill)
            .child(knob);
        let track = SliderTrack::new(&state)
            .disabled(disabled)
            .flex()
            .items_center()
            .w_full()
            .h(px(CONTROL_H))
            .child(bar);
        BaseSlider::new(&state)
            .disabled(disabled)
            .flex()
            .flex_1()
            .items_center()
            .w_full()
            .when(disabled, |s| s.opacity(DISABLED_OPACITY))
            .child(track)
    }
}
