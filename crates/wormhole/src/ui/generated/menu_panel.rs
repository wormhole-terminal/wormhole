//! `MenuPanel`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Rule, Sides};

/// What `MenuPanel` is drawn from.
pub struct MenuPanelProps {
    pub id: ElementId,
    pub label: SharedString,
    pub compact: bool,
    pub rows: AnyElement,
}

impl MenuPanelProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_shadow = token(look.tokens, "shadow");
        let t_surface_elevated = token(look.tokens, "surface.elevated");
        let borders = look.borders;
        let Self { id, label, compact, rows } = self;

        div()
            .id(id)
            .role(Role::ListBox)
            .aria_label(label.clone())
            .flex()
            .flex_col()
            .min_w(px(190.0))
            .p(px(4.0))
            .rule(borders, Sides::ALL, 1.0, t_border, false)
            .rounded(px(8.0))
            .bg(t_surface_elevated)
            .shadow(vec![BoxShadow { color: t_shadow, offset: point(px(0.0), px(10.0)), blur_radius: px(28.0), spread_radius: px(0.0), inset: false }])
            .when(compact, |this| this.min_w(px(170.0)))
            .child(rows)
    }
}
