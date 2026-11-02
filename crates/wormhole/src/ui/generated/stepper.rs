//! `Stepper`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Rule, Sides};

/// What `Stepper` is drawn from.
pub struct StepperProps {
    pub id: ElementId,
    pub label: SharedString,
    pub smaller: SharedString,
    pub larger: SharedString,
    pub value: SharedString,
    pub on_smaller: Option<Handler>,
    pub on_larger: Option<Handler>,
}

impl StepperProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_border_focus = token(look.tokens, "border.focus");
        let t_button_background = token(look.tokens, "button.background");
        let t_button_hover = token(look.tokens, "button.hover");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_primary = token(look.tokens, "text.primary");
        let borders = look.borders;
        let Self { id, label, smaller, larger, value, on_smaller, on_larger } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(label.clone())
            .flex()
            .items_center()
            .flex_shrink_0()
            .h(px(24.0))
            .overflow_hidden()
            .rule(borders, Sides::ALL, 1.0, t_border, false)
            .rounded(px(6.0))
            .bg(t_button_background)
            .text_color(t_text_primary)
            .child(div()
                .id("stepper-smaller")
                .role(Role::Button)
                .aria_label(smaller.clone())
                .focusable().tab_stop(true)
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(22.0))
                .h(px(22.0))
                .text_color(t_text_muted)
                .text_size(rems(0.8125))
                .line_height(rems(1.0))
                .cursor_pointer()
                .hover(move |style| {
                    let style = style.bg(t_button_hover).text_color(t_text_primary);
                    style
                })
                .focus_visible(move |style| {
                    let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                    style
                })
                .when_some(on_smaller.clone(), |this, handler| {
                    this.on_click(move |event, window, cx| handler(event, window, cx))
                })
                .child("−"))
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .min_w(px(42.0))
                .h(px(22.0))
                .rule(borders, Sides::LEFT, 1.0, t_border, false)
                .rule(borders, Sides::RIGHT, 1.0, t_border, false)
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .child(value.clone()))
            .child(div()
                .id("stepper-larger")
                .role(Role::Button)
                .aria_label(larger.clone())
                .focusable().tab_stop(true)
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(22.0))
                .h(px(22.0))
                .text_color(t_text_muted)
                .text_size(rems(0.8125))
                .line_height(rems(1.0))
                .cursor_pointer()
                .hover(move |style| {
                    let style = style.bg(t_button_hover).text_color(t_text_primary);
                    style
                })
                .focus_visible(move |style| {
                    let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                    style
                })
                .when_some(on_larger.clone(), |this, handler| {
                    this.on_click(move |event, window, cx| handler(event, window, cx))
                })
                .child("+"))
    }
}
