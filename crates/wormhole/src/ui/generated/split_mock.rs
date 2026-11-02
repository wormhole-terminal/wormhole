//! `SplitMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// One item of `left`.
#[derive(Default)]
pub struct LeftLine {
    pub text: SharedString,
    pub color: Hsla,
}

/// One item of `right`.
#[derive(Default)]
pub struct RightLine {
    pub text: SharedString,
    pub color: Hsla,
}

/// What `SplitMock` is drawn from.
pub struct SplitMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub gaps: bool,
    pub minimap: bool,
    pub split: bool,
    pub ring_fill: Hsla,
    pub bg: Hsla,
    pub sep: Hsla,
    pub rule: Hsla,
    pub bar: Hsla,
    pub left: Vec<LeftLine>,
    pub right: Vec<RightLine>,
}

impl SplitMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_surface_panel = token(look.tokens, "surface.panel");
        let Self { id, spoken, gaps, minimap, split, ring_fill, bg, sep, rule, bar, left, right } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_shrink_0()
            .h(px(200.0))
            .bg(t_surface_panel)
            .when(gaps, |this| {
                this
                    .gap(px(8.0))
                    .p(px(8.0))
                    .h(px(216.0))
            })
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .when(gaps, |this| {
                    this
                        .p(px(1.0))
                        .rounded(px(6.0))
                })
                .bg(ring_fill)
                .child(div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .when(gaps, |this| this.rounded(px(5.0)))
                    .bg(bg)
                    .child(div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .p(px(10.0))
                        .font_family(look.code_font.clone())
                        .text_size(rems(0.6875))
                        .line_height(rems(0.9375))
                        .overflow_hidden()
                        .children(left.into_iter().flat_map(|l| {
                            let l_color = l.color;
                            let mut out: Vec<AnyElement> = Vec::new();
                            out.push(
                                div()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .text_color(l_color)
                                    .child(l.text.clone())
                                    .into_any_element(),
                            );
                            out
                        })))
                    .when(minimap, |this| {
                        this
                            .child(div()
                                .w(px(1.0))
                                .flex_shrink_0()
                                .bg(rule))
                            .child(div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .w(px(18.0))
                                .flex_shrink_0()
                                .py(px(8.0)).px(px(3.0))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.6))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.85))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.4))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.7))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.9))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.55))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.3))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.75))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.65))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.45))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.8))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.35))
                                    .bg(bar)))
                    })
                    .when(split, |this| {
                        this
                            .child(div()
                                .w(px(1.0))
                                .flex_shrink_0()
                                .bg(sep))
                    })))
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .when(gaps, |this| {
                    this
                        .p(px(1.0))
                        .rounded(px(6.0))
                })
                .bg(ring_fill)
                .child(div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .when(gaps, |this| this.rounded(px(5.0)))
                    .bg(bg)
                    .child(div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .p(px(10.0))
                        .font_family(look.code_font.clone())
                        .text_size(rems(0.6875))
                        .line_height(rems(0.9375))
                        .overflow_hidden()
                        .children(right.into_iter().flat_map(|l| {
                            let l_color = l.color;
                            let mut out: Vec<AnyElement> = Vec::new();
                            out.push(
                                div()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .text_color(l_color)
                                    .child(l.text.clone())
                                    .into_any_element(),
                            );
                            out
                        })))
                    .when(minimap, |this| {
                        this
                            .child(div()
                                .w(px(1.0))
                                .flex_shrink_0()
                                .bg(rule))
                            .child(div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .w(px(18.0))
                                .flex_shrink_0()
                                .py(px(8.0)).px(px(3.0))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.7))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.5))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.85))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.4))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.6))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.9))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.45))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.3))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.75))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.55))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.65))
                                    .bg(bar))
                                .child(div()
                                    .h(px(3.0))
                                    .rounded(px(1.0))
                                    .w(relative(0.4))
                                    .bg(bar)))
                    })))
    }
}
