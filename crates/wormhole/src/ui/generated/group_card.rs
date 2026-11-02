//! `GroupCard`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// What `GroupCard` is drawn from.
pub struct GroupCardProps {
    pub rows: AnyElement,
}

impl GroupCardProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_band_hover = token(look.tokens, "band.hover");
        let Self { rows } = self;

        div()
            .flex()
            .flex_col()
            .w_full()
            .overflow_hidden()
            .rounded(px(10.0))
            .bg(t_band_hover)
            .child(div()
                .flex()
                .flex_col()
                .mt(px(-1.0))
                .child(rows))
    }
}
