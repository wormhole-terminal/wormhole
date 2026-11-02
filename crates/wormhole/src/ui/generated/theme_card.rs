//! `ThemeCard`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;
use gpui_kit::accesskit::Toggled;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Force};

/// One item of `halves`.
#[derive(Default)]
pub struct Half {
    pub bg: Hsla,
    pub strip: Hsla,
    pub strip_line: Hsla,
    pub strip_rule: Hsla,
    pub c1: Hsla,
    pub c2: Hsla,
    pub c3: Hsla,
    pub c4: Hsla,
    pub c5: Hsla,
}

/// What `ThemeCard` is drawn from.
pub struct ThemeCardProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub name: SharedString,
    pub mark: SharedString,
    pub on: bool,
    pub on_pick: Option<Handler>,
    pub halves: Vec<Half>,
}

impl ThemeCardProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_border_focus = token(look.tokens, "border.focus");
        let t_primary = token(look.tokens, "primary");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_primary = token(look.tokens, "text.primary");
        let force = look.force;
        let Self { id, spoken, name, mark, on, on_pick, halves } = self;

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(spoken.clone())
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .focusable().tab_stop(true)
            .flex()
            .flex_col()
            .gap(px(6.0))
            .w_full()
            .min_w(px(0.0))
            .p(px(0.0))
            .text_size(rems(0.75))
            .line_height(rems(1.0))
            .text_left()
            .cursor_pointer()
            .text_color(t_text_primary)
            .when(force == Force::Focus, |this| {
                this
                    .shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }])
            })
            .focus_visible(move |style| {
                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                style
            })
            .when_some(on_pick.clone(), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(div()
                .flex()
                .h(px(60.0))
                .rounded(px(7.0))
                .overflow_hidden()
                .shadow(vec![BoxShadow { color: t_border, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(1.0), inset: false }])
                .when(on, |this| this.shadow(vec![BoxShadow { color: t_primary, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]))
                .children(halves.into_iter().flat_map(|h| {
                    let h_bg = h.bg;
                    let h_c1 = h.c1;
                    let h_c2 = h.c2;
                    let h_c3 = h.c3;
                    let h_c4 = h.c4;
                    let h_c5 = h.c5;
                    let h_strip = h.strip;
                    let h_strip_line = h.strip_line;
                    let h_strip_rule = h.strip_rule;
                    let mut out: Vec<AnyElement> = Vec::new();
                    out.push(
                        div()
                            .flex_1()
                            .flex()
                            .h_full()
                            .bg(h_bg)
                            .child(div()
                                .flex()
                                .w(relative(0.22))
                                .flex_shrink_0()
                                .bg(h_strip)
                                .child(div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .py(px(8.0)).px(px(4.0))
                                    .child(div()
                                        .h(px(3.0))
                                        .rounded(px(2.0))
                                        .bg(h_strip_line))
                                    .child(div()
                                        .h(px(3.0))
                                        .rounded(px(2.0))
                                        .bg(h_strip_line)))
                                .child(div()
                                    .w(px(1.0))
                                    .flex_shrink_0()
                                    .bg(h_strip_rule)))
                            .child(div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .py(px(8.0)).px(px(6.0))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(2.0))
                                    .w(relative(0.7))
                                    .bg(h_c1))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(2.0))
                                    .w(relative(0.85))
                                    .bg(h_c2))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(2.0))
                                    .w(relative(0.55))
                                    .bg(h_c3))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(2.0))
                                    .w(relative(0.75))
                                    .bg(h_c4))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(2.0))
                                    .w(relative(0.45))
                                    .bg(h_c5)))
                            .into_any_element(),
                    );
                    out
                })))
            .child(div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .py(px(0.0)).px(px(2.0))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_color(t_text_muted)
                .when(on, |this| {
                    this
                        .text_color(t_text_primary)
                        .font_weight(FontWeight::SEMIBOLD)
                })
                .child(div()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(name.clone()))
                .when(on, |this| {
                    this
                        .child(div()
                            .text_color(t_primary)
                            .child(mark.clone()))
                }))
    }
}
