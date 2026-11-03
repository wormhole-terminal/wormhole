//! `SolidChip`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// What `SolidChip` is drawn from.
pub struct SolidChipProps {
    pub text: SharedString,
    pub good: bool,
    pub warn: bool,
    pub danger: bool,
    pub primary: bool,
}

impl SolidChipProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_chip_danger_fill = token(look.tokens, "chip.danger.fill");
        let t_chip_good_fill = token(look.tokens, "chip.good.fill");
        let t_chip_quiet_fill = token(look.tokens, "chip.quiet.fill");
        let t_chip_solid_text = token(look.tokens, "chip.solid.text");
        let t_chip_warn_fill = token(look.tokens, "chip.warn.fill");
        let t_primary = token(look.tokens, "primary");
        let Self { text, good, warn, danger, primary } = self;

        div()
            .flex()
            .items_center()
            .flex_shrink_0()
            .h(px(18.0))
            .py(px(0.0)).px(px(7.0))
            .rounded(px(9.0))
            .bg(t_chip_quiet_fill)
            .text_color(t_chip_solid_text)
            .text_size(rems(0.6875))
            .line_height(rems(0.875))
            .font_weight(FontWeight::SEMIBOLD)
            .whitespace_nowrap()
            .when(good, |this| this.bg(t_chip_good_fill))
            .when(warn, |this| this.bg(t_chip_warn_fill))
            .when(danger, |this| this.bg(t_chip_danger_fill))
            .when(primary, |this| this.bg(t_primary))
            .child(text.clone())
    }
}
