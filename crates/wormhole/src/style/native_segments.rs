//! Shared AppKit segmented control presentation with GPUI focus and accessibility.
use crate::platform::controls::{SegmentOption, Segmented, Segments};
use gpui_kit::base::Button;
use gpui_kit::component::ThemeStyled as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;

pub(crate) type Change = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub(crate) type Hover = Rc<dyn Fn(usize, bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct NativeSegments {
    pub native: Rc<Segments>,
    pub id: SharedString,
    pub options: Vec<SegmentOption>,
    pub accessibility_labels: Option<Vec<String>>,
    pub selected: usize,
    pub enabled: bool,
    pub hover: Option<Hover>,
    pub focus: Vec<FocusHandle>,
    pub appearance: wormhole_core::theme::tokens::Appearance,
    pub metrics: crate::platform::controls::SegmentMetrics,
    pub change: Change,
}

impl RenderOnce for NativeSegments {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            native,
            id,
            options,
            accessibility_labels,
            selected,
            enabled,
            hover,
            focus,
            appearance,
            metrics,
            change,
        } = self;
        let buttons = options
            .iter()
            .enumerate()
            .map(|(ix, option)| {
                let change = change.clone();
                let focused = focus[ix].is_focused(window) && crate::style::native_focus::visible(window);
                Button::new((id.clone(), ix))
                    .role(Role::RadioButton)
                    .aria_selected(ix == selected)
                    .accessibility_label(
                        accessibility_labels
                            .as_ref()
                            .and_then(|labels| labels.get(ix))
                            .cloned()
                            .unwrap_or_else(|| option.label.clone()),
                    )
                    .track_focus(&focus[ix])
                    .disabled(!enabled)
                    .w(metrics.widths[ix])
                    .h(metrics.height)
                    .when(focused, |button| button.focus_ring_style(window, cx))
                    .when_some(hover.clone(), |button, hover| {
                        button.on_hover(move |over, window, cx| hover(ix, *over, window, cx))
                    })
                    .on_click(move |_, window, cx| change(ix, window, cx))
            })
            .collect::<Vec<_>>();
        let keyboard_focus = focus.clone();
        let keyboard_change = change.clone();
        let probe_id = id.clone();
        let probe = canvas(
            move |bounds, window, _| {
                native.place(&Segmented {
                    id: &probe_id,
                    options: &options,
                    selected,
                    enabled,
                    bounds,
                    clip: window.content_mask().bounds,
                    appearance,
                })
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        div()
            .id((id, usize::MAX))
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                crate::style::native_focus::keyboard(window);
                if !enabled || event.keystroke.modifiers.modified() {
                    return;
                }
                let Some(current) = keyboard_focus.iter().position(|focus| focus.is_focused(window)) else {
                    return;
                };
                let next = match event.keystroke.key.as_str() {
                    "left" => current.saturating_sub(1),
                    "right" => (current + 1).min(keyboard_focus.len() - 1),
                    _ => return,
                };
                if next != current {
                    window.focus(&keyboard_focus[next], cx);
                    keyboard_change(next, window, cx);
                }
                cx.stop_propagation();
            })
            .relative()
            .flex()
            .flex_none()
            .w(metrics.width)
            .h(metrics.height)
            .children(buttons)
            .child(probe)
    }
}
