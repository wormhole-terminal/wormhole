//! `FooterLine`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look};

/// One item of `links`.
#[derive(Default)]
pub struct FooterLink {
    pub key: usize,
    pub name: SharedString,
    pub pick: Option<Handler>,
}

/// What `FooterLine` is drawn from.
pub struct FooterLineProps {
    pub text: SharedString,
    pub links: Vec<FooterLink>,
}

impl FooterLineProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_border_focus = token(look.tokens, "border.focus");
        let t_primary = token(look.tokens, "primary");
        let t_text_muted = token(look.tokens, "text.muted");
        let Self { text, links } = self;

        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_y(px(4.0)).gap_x(px(12.0))
            .pl(px(4.0))
            .text_size(rems(0.75))
            .line_height(rems(1.0625))
            .text_color(t_text_muted)
            .child(div()
                .child(text.clone()))
            .children(links.into_iter().flat_map(|link| {
                let link_key = link.key;
                let mut out: Vec<AnyElement> = Vec::new();
                out.push(
                    div()
                        .id(("footer-link", link_key))
                        .role(Role::Button)
                        .focusable().tab_stop(true)
                        .flex_shrink_0()
                        .text_color(t_primary)
                        .text_size(rems(0.75))
                        .line_height(rems(1.0625))
                        .cursor_pointer()
                        .hover(move |style| {
                            let style = style.underline();
                            style
                        })
                        .focus_visible(move |style| {
                            let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                            style
                        })
                        .when_some(link.pick.clone(), |this, handler| {
                            this.on_click(move |event, window, cx| handler(event, window, cx))
                        })
                        .child(link.name.clone())
                        .into_any_element(),
                );
                out
            }))
    }
}
