//! Native push button with a GPUI focus and accessibility proxy.
use crate::platform::controls::{Buttons, PushButton};
use gpui_kit::base::Button;
use gpui_kit::component::ThemeStyled as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;

#[derive(IntoElement)]
pub(crate) struct NativeButton {
    pub native: Rc<Buttons>,
    pub id: String,
    pub label: SharedString,
    pub tooltip: Option<SharedString>,
    pub enabled: bool,
    pub primary: bool,
    pub destructive: bool,
    pub focus: FocusHandle,
    pub appearance: wormhole_core::theme::tokens::Appearance,
    pub press: Rc<dyn Fn(&mut Window, &mut App)>,
}
impl RenderOnce for NativeButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { native, id, label, tooltip, enabled, primary, destructive, focus, appearance, press } =
            self;
        let size = native.size(&label);
        let focused = focus.is_focused(window) && crate::style::native_focus::visible(window);
        let probe_id = id.clone();
        let probe_label = label.clone();
        let probe = canvas(
            move |bounds, window, _| {
                native.place(&PushButton {
                    id: &probe_id,
                    label: &probe_label,
                    tooltip: tooltip.as_deref(),
                    enabled,
                    primary,
                    destructive,
                    bounds,
                    clip: window.content_mask().bounds,
                    appearance,
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        Button::new(SharedString::from(id))
            .accessibility_label(label)
            .track_focus(&focus)
            .disabled(!enabled)
            .relative()
            .w(size.width)
            .h(size.height)
            .when(focused, |button| button.focus_ring_style(window, cx))
            .on_click(move |_, window, cx| press(window, cx))
            .child(probe)
    }
}
