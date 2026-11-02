//! `PreviewFrame`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Rule, Sides};

/// What `PreviewFrame` is drawn from.
pub struct PreviewFrameProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub label: SharedString,
    pub what: SharedString,
    pub caption: SharedString,
    pub picture: AnyElement,
}

impl PreviewFrameProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_surface_panel = token(look.tokens, "surface.panel");
        let t_text_faint = token(look.tokens, "text.faint");
        let t_text_muted = token(look.tokens, "text.muted");
        let borders = look.borders;
        let Self { id, spoken, label, what, caption, picture } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w_full()
            .child(div()
                .flex()
                .items_baseline()
                .gap(px(6.0))
                .pl(px(4.0))
                .text_size(rems(0.6875))
                .line_height(rems(0.875))
                .child(div()
                    .font_weight(FontWeight::BOLD)
                    .text_color(t_text_muted)
                    .child(label.clone()))
                .child(div()
                    .text_color(t_text_faint)
                    .child(format!("· {}", what))))
            .child(div()
                .flex()
                .flex_col()
                .overflow_hidden()
                .rule(borders, Sides::ALL, 1.0, t_border, false)
                .rounded(px(10.0))
                .bg(t_surface_panel)
                .child(picture))
            .child(div()
                .m(px(0.0))
                .pl(px(4.0))
                .text_size(rems(0.75))
                .line_height(rems(1.0625))
                .text_color(t_text_muted)
                .child(caption.clone()))
    }
}
