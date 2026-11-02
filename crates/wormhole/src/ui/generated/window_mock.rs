//! `WindowMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Rule, Sides};

/// One item of `left`.
#[derive(Default)]
pub struct Part {
    pub text: SharedString,
    pub color: Hsla,
    pub bold: bool,
}

/// One item of `right`.
#[derive(Default)]
pub struct RightPart {
    pub text: SharedString,
    pub color: Hsla,
    pub bold: bool,
}

/// One item of `side`.
#[derive(Default)]
pub struct Bar {
    pub fill: Hsla,
}

/// One item of `lines`.
#[derive(Default)]
pub struct Line {
    pub text: SharedString,
    pub color: Hsla,
}

/// What `WindowMock` is drawn from.
pub struct WindowMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub fill: Hsla,
    pub fg: Hsla,
    pub rule: Hsla,
    pub has_rule: bool,
    pub status_color: Hsla,
    pub has_status: bool,
    pub branch: SharedString,
    pub changed: SharedString,
    pub shell: SharedString,
    pub left: Vec<Part>,
    pub right: Vec<RightPart>,
    pub side: Vec<Bar>,
    pub lines: Vec<Line>,
}

impl WindowMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_shadow = token(look.tokens, "shadow");
        let t_surface_panel = token(look.tokens, "surface.panel");
        let borders = look.borders;
        let Self { id, spoken, fill, fg, rule, has_rule, status_color, has_status, branch, changed, shell, left, right, side, lines } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .p(px(12.0))
            .bg(t_surface_panel)
            .child(div()
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(px(8.0))
                .rule(borders, Sides::ALL, 1.0, t_border, false)
                .shadow(vec![BoxShadow { color: t_shadow, offset: point(px(0.0), px(10.0)), blur_radius: px(26.0), spread_radius: px(0.0), inset: false }])
                .bg(fill)
                .child(div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .h(px(28.0))
                    .py(px(0.0)).px(px(10.0))
                    .overflow_hidden()
                    .rule(borders, Sides::BOTTOM, 1.0, t_border, false)
                    .child(div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .mr(px(4.0))
                        .child(div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded_full()
                            .bg(Hsla::from(rgba(0xff5f57ff))))
                        .child(div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded_full()
                            .bg(Hsla::from(rgba(0xfebc2eff))))
                        .child(div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded_full()
                            .bg(Hsla::from(rgba(0x28c840ff)))))
                    .children(left.into_iter().flat_map(|p| {
                        let p_bold = p.bold;
                        let p_color = p.color;
                        let mut out: Vec<AnyElement> = Vec::new();
                        out.push(
                            div()
                                .whitespace_nowrap()
                                .text_size(rems(0.6875))
                                .line_height(rems(0.875))
                                .when(p_bold, |this| this.font_weight(FontWeight::SEMIBOLD))
                                .text_color(p_color)
                                .child(p.text.clone())
                                .into_any_element(),
                        );
                        out
                    }))
                    .child(div()
                        .flex_grow_1())
                    .children(right.into_iter().flat_map(|p| {
                        let p_bold = p.bold;
                        let p_color = p.color;
                        let mut out: Vec<AnyElement> = Vec::new();
                        out.push(
                            div()
                                .whitespace_nowrap()
                                .text_size(rems(0.6875))
                                .line_height(rems(0.875))
                                .when(p_bold, |this| this.font_weight(FontWeight::SEMIBOLD))
                                .text_color(p_color)
                                .child(p.text.clone())
                                .into_any_element(),
                        );
                        out
                    })))
                .child(div()
                    .flex()
                    .h(px(150.0))
                    .child(div()
                        .flex()
                        .w(px(120.0))
                        .flex_shrink_0()
                        .child(div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .py(px(10.0)).px(px(8.0))
                            .children(side.into_iter().flat_map(|s| {
                                let s_fill = s.fill;
                                let mut out: Vec<AnyElement> = Vec::new();
                                out.push(
                                    div()
                                        .h(px(16.0))
                                        .rounded(px(5.0))
                                        .bg(s_fill)
                                        .into_any_element(),
                                );
                                out
                            })))
                        .when(has_rule, |this| {
                            this
                                .child(div()
                                    .w(px(1.0))
                                    .flex_shrink_0()
                                    .bg(rule))
                        }))
                    .child(div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .p(px(10.0))
                        .font_family(look.code_font.clone())
                        .text_size(rems(0.6875))
                        .line_height(rems(0.8125))
                        .text_color(fg)
                        .children(lines.into_iter().flat_map(|l| {
                            let l_color = l.color;
                            let mut out: Vec<AnyElement> = Vec::new();
                            out.push(
                                div()
                                    .whitespace_nowrap()
                                    .text_color(l_color)
                                    .child(l.text.clone())
                                    .into_any_element(),
                            );
                            out
                        }))))
                .when(has_status, |this| {
                    this
                        .child(div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .h(px(20.0))
                            .py(px(0.0)).px(px(10.0))
                            .text_size(rems(0.625))
                            .line_height(rems(0.75))
                            .rule(borders, Sides::TOP, 1.0, t_border, false)
                            .text_color(status_color)
                            .child(div()
                                .child(branch.clone()))
                            .child(div()
                                .child(changed.clone()))
                            .child(div()
                                .flex_grow_1())
                            .child(div()
                                .child(shell.clone())))
                }))
    }
}
