//! Shared presentation for AppKit switches with GPUI focus and accessibility.
use crate::platform::controls::{Switch, Switches};
use gpui_kit::base::Checkbox as BaseCheckbox;
use gpui_kit::base::Switch as BaseSwitch;
use gpui_kit::component::ThemeStyled as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;
use wormhole_core::theme::tokens::Appearance;

pub(crate) type Change = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct NativeSwitch {
    pub native: Rc<Switches>,
    pub id: String,
    pub label: SharedString,
    pub on: bool,
    pub enabled: bool,
    pub focus: Option<FocusHandle>,
    pub appearance: Appearance,
    pub change: Change,
}
impl RenderOnce for NativeSwitch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { native, id, label, on, enabled, focus, appearance, change } = self;
        let focused = focus.as_ref().is_some_and(|focus| focus.is_focused(window))
            && crate::style::native_focus::visible(window);
        let size = native.size();
        let probe_id = id.clone();
        let probe_label = label.clone();
        let probe = canvas(
            move |bounds, window, _| {
                native.place(&Switch {
                    id: &probe_id,
                    label: &probe_label,
                    on,
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
        BaseSwitch::new(SharedString::from(id))
            .checked(on)
            .disabled(!enabled)
            .accessibility_label(label)
            .tab_stop(focus.is_some())
            .when_some(focus, |base, focus| base.track_focus(&focus))
            .on_change(move |next, _, window, cx| change(next, window, cx))
            .flex()
            .items_center()
            .justify_center()
            .w(size.width + px(8.))
            .h(size.height + px(8.))
            .rounded_full()
            .when(focused, |this| this.focus_ring_style(window, cx))
            .child(probe)
    }
}

/// A native checkbox with the same GPUI focus and accessibility bridge as a switch.
#[derive(IntoElement)]
pub(crate) struct NativeCheckbox {
    pub native: Rc<Switches>,
    pub id: String,
    pub label: SharedString,
    pub on: bool,
    pub enabled: bool,
    pub show_label: bool,
    pub focus: Option<FocusHandle>,
    pub change: Change,
    pub appearance: Appearance,
    pub ui: crate::settings::UiColors,
}

impl RenderOnce for NativeCheckbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { native, id, label, on, enabled, show_label, focus, change, appearance, ui } = self;
        let size = native.checkbox_size();
        let focused = focus.as_ref().is_some_and(|focus| focus.is_focused(window))
            && crate::style::native_focus::visible(window);
        let probe_id = id.clone();
        let probe_label = label.clone();
        let probe = canvas(
            move |bounds, window, _| {
                native.place_checkbox(&Switch {
                    id: &probe_id,
                    label: &probe_label,
                    on,
                    enabled,
                    bounds,
                    clip: window.content_mask().bounds,
                    appearance,
                });
            },
            |_, _, _, _| {},
        )
        .w(size.width)
        .h(size.height);
        use crate::style::TextRole as _;
        BaseCheckbox::new(SharedString::from(id))
            .checked(on)
            .disabled(!enabled)
            .tab_stop(focus.is_some())
            .accessibility_label(label.clone())
            .when_some(focus, |base, focus| base.track_focus(&focus))
            .on_change(move |_, _, window, cx| change(!on, window, cx))
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .when(focused, |base| base.focus_ring_style(window, cx))
            .child(probe)
            .when(show_label, |base| base.child(div().text_body().text_color(ui.text).child(label)))
    }
}
