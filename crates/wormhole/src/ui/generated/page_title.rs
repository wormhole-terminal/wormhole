//! `PageTitle`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// What `PageTitle` is drawn from.
pub struct PageTitleProps {
    pub title: SharedString,
    pub badge: Option<SharedString>,
}

impl PageTitleProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_primary = token(look.tokens, "primary");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let Self { title, badge } = self;

        div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(div()
                .m(px(0.0))
                .text_size(rems(1.375))
                .line_height(rems(1.75))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t_text_primary)
                .child(title.clone()))
            .when_some(badge.clone(), |this, badge| {
                this
                    .child(div()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .h(px(18.0))
                        .py(px(0.0)).px(px(7.0))
                        .rounded(px(9.0))
                        .bg(t_primary)
                        .text_color(t_text_on_primary)
                        .text_size(rems(0.6875))
                        .line_height(rems(0.875))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(badge.clone()))
            })
    }
}
