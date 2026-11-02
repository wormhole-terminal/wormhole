//! `SwitchControl`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;
use gpui_kit::accesskit::Toggled;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Force};

/// What `SwitchControl` is drawn from.
pub struct SwitchControlProps {
    pub id: ElementId,
    pub label: SharedString,
    pub on: bool,
    pub disabled: bool,
    pub on_click: Option<Handler>,
}

impl SwitchControlProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border_focus = token(look.tokens, "border.focus");
        let t_checkbox_border = token(look.tokens, "checkbox.border");
        let t_primary = token(look.tokens, "primary");
        let t_switch_knob = token(look.tokens, "switch.knob");
        let force = look.force;
        let Self { id, label, on, disabled, on_click } = self;

        div()
            .id(id)
            .role(Role::Switch)
            .aria_label(label.clone())
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .focusable().tab_stop(true)
            .flex()
            .items_center()
            .justify_start()
            .flex_shrink_0()
            .w(px(34.0))
            .h(px(20.0))
            .py(px(0.0)).px(px(2.0))
            .rounded(px(10.0))
            .bg(t_checkbox_border)
            .cursor_pointer()
            .when(on, |this| {
                this
                    .justify_end()
                    .bg(t_primary)
            })
            .when(disabled, |this| {
                this
                    .opacity(0.5)
                    .cursor_default()
            })
            .when(force == Force::Focus, |this| {
                this
                    .shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }])
            })
            .focus_visible(move |style| {
                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                style
            })
            .when_some(on_click.clone(), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(div()
                .w(px(16.0))
                .h(px(16.0))
                .rounded_full()
                .bg(t_switch_knob))
    }
}
