//! `MenuFace`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Force, Rule, Sides};

/// What `MenuFace` is drawn from.
pub struct MenuFaceProps {
    pub value: SharedString,
    pub lead: Option<AnyElement>,
    pub open: bool,
    pub disabled: bool,
}

impl MenuFaceProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_button_background = token(look.tokens, "button.background");
        let t_button_hover = token(look.tokens, "button.hover");
        let t_primary = token(look.tokens, "primary");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_primary = token(look.tokens, "text.primary");
        let force = look.force;
        let borders = look.borders;
        let Self { value, lead, open, disabled } = self;

        div()
            .id("e1")
            .flex()
            .items_center()
            .gap(px(8.0))
            .flex_shrink_0()
            .h(px(26.0))
            .py(px(0.0)).pl(px(10.0)).pr(px(7.0))
            .rule(borders, Sides::ALL, 1.0, t_border, false)
            .rounded(px(6.0))
            .bg(t_button_background)
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .whitespace_nowrap()
            .cursor_pointer()
            .when(open, |this| this.rule(borders, Sides::ALL, 1.0, t_primary, true))
            .when(disabled, |this| {
                this
                    .opacity(0.5)
                    .cursor_default()
            })
            .when(force == Force::Hover, |this| {
                this
                    .bg(t_button_hover)
            })
            .when(force == Force::Hover && disabled, |this| {
                this
                    .bg(t_button_background)
            })
            .hover(move |style| {
                let style = style.bg(t_button_hover);
                let style = if disabled { style.bg(t_button_background) } else { style };
                style
            })
            .when_some(lead, |this, lead| {
                this
                    .child(lead)
            })
            .child(div()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(value.clone()))
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(10.0))
                .h(px(12.0))
                .text_color(t_text_muted)
                .child(Icon::new(IconName::ChevronsUpDown).with_size(px(12.0))))
    }
}
