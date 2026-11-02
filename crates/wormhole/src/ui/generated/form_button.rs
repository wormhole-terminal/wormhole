//! `FormButton`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Rule, Sides};

/// What `FormButton` is drawn from.
pub struct FormButtonProps {
    pub id: ElementId,
    pub label: SharedString,
    pub primary: bool,
    pub destructive: bool,
    pub disabled: bool,
    pub on_click: Option<Handler>,
}

impl FormButtonProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_border_focus = token(look.tokens, "border.focus");
        let t_button_background = token(look.tokens, "button.background");
        let t_button_hover = token(look.tokens, "button.hover");
        let t_primary = token(look.tokens, "primary");
        let t_primary_hover = token(look.tokens, "primary.hover");
        let t_status_danger_text = token(look.tokens, "status.danger.text");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let borders = look.borders;
        let Self { id, label, primary, destructive, disabled, on_click } = self;

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label.clone())
            .focusable().tab_stop(true)
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .h(px(24.0))
            .py(px(0.0)).px(px(12.0))
            .rule(borders, Sides::ALL, 1.0, t_border, false)
            .rounded(px(6.0))
            .bg(t_button_background)
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .whitespace_nowrap()
            .cursor_pointer()
            .when(primary, |this| {
                this
                    .rule(borders, Sides::ALL, 0.0, transparent_black(), false)
                    .bg(t_primary)
                    .text_color(t_text_on_primary)
            })
            .when(destructive, |this| this.text_color(t_status_danger_text))
            .when(disabled, |this| {
                this
                    .opacity(0.45)
                    .cursor_default()
            })
            .hover(move |style| {
                let style = style.bg(t_button_hover);
                let style = if primary { style.bg(t_primary_hover) } else { style };
                let style = if disabled { style.bg(t_button_background) } else { style };
                let style = if primary && disabled { style.bg(t_primary) } else { style };
                style
            })
            .focus_visible(move |style| {
                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                style
            })
            .when_some(on_click.clone(), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(label.clone())
    }
}
