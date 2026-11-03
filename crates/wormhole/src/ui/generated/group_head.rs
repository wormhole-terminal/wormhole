//! `GroupHead`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// What `GroupHead` is drawn from.
pub struct GroupHeadProps {
    pub title: SharedString,
}

impl GroupHeadProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_text_muted = token(look.tokens, "text.muted");
        let Self { title } = self;

        div()
            .m(px(0.0))
            .py(px(0.0)).pl(px(4.0)).pr(px(0.0))
            .text_size(rems(0.8125))
            .line_height(rems(0.9375))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(t_text_muted)
            .child(title.clone())
    }
}
