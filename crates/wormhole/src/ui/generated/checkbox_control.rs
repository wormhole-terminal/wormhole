//! `CheckboxControl`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use gpui_kit::accesskit::Toggled;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Rule, Sides};

/// What `CheckboxControl` is drawn from.
pub struct CheckboxControlProps {
    pub id: ElementId,
    pub label: SharedString,
    pub on: bool,
    pub on_click: Option<Handler>,
}

impl CheckboxControlProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border_focus = token(look.tokens, "border.focus");
        let t_checkbox_background = token(look.tokens, "checkbox.background");
        let t_checkbox_border = token(look.tokens, "checkbox.border");
        let t_checkbox_checked = token(look.tokens, "checkbox.checked");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let borders = look.borders;
        let Self { id, label, on, on_click } = self;

        div()
            .id(id)
            .role(Role::CheckBox)
            .aria_label(label.clone())
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .focusable().tab_stop(true)
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .w(px(16.0))
            .h(px(16.0))
            .rule(borders, Sides::ALL, 1.0, t_checkbox_border, true)
            .rounded(px(4.0))
            .bg(t_checkbox_background)
            .text_color(t_text_on_primary)
            .cursor_pointer()
            .when(on, |this| {
                this
                    .rule(borders, Sides::ALL, 1.0, t_checkbox_checked, true)
                    .bg(t_checkbox_checked)
            })
            .focus_visible(move |style| {
                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                style
            })
            .when_some(on_click.clone(), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(on, |this| {
                this
                    .child(Icon::new(IconName::Check).with_size(px(10.0)))
            })
    }
}
