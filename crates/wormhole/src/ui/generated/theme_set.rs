//! `ThemeSet`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// What `ThemeSet` is drawn from.
pub struct ThemeSetProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub caption: Option<SharedString>,
    pub cards: Vec<AnyElement>,
}

impl ThemeSetProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_text_muted = token(look.tokens, "text.muted");
        let Self { id, spoken, caption, cards } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .gap(px(8.0))
            .when_some(caption.clone(), |this, caption| {
                this
                    .child(div()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(t_text_muted)
                        .child(caption.clone()))
            })
            .child(div()
                .grid()
                .grid_cols(4)
                .gap_y(px(12.0)).gap_x(px(10.0))
                .children(cards))
    }
}
