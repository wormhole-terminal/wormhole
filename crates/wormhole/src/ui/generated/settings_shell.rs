//! `SettingsShell`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Rule, Sides};

/// What `SettingsShell` is drawn from.
pub struct SettingsShellProps {
    pub nav: AnyElement,
    pub page: AnyElement,
}

impl SettingsShellProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_border = token(look.tokens, "border");
        let borders = look.borders;
        let Self { nav, page } = self;

        div()
            .flex()
            .items_stretch()
            .w_full()
            .h_full()
            .child(div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .w(px(240.0))
                .rule(borders, Sides::RIGHT, 1.0, t_border, false)
                .child(nav))
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .child(page))
    }
}
