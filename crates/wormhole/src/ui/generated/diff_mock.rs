//! `DiffMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Rule, Sides};

/// One item of `files`.
#[derive(Default)]
pub struct File {
    pub name: SharedString,
    pub badge: SharedString,
    pub badge_color: Hsla,
    pub deep: bool,
    pub dir: bool,
}

/// One item of `cells`.
#[derive(Default)]
pub struct Cell {
    pub text: SharedString,
    pub wrap: bool,
    pub left: bool,
    pub del: bool,
    pub add: bool,
    pub none: bool,
}

/// One item of `lines`.
#[derive(Default)]
pub struct DiffLine {
    pub text: SharedString,
    pub wrap: bool,
    pub del: bool,
    pub add: bool,
}

/// What `DiffMock` is drawn from.
pub struct DiffMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub heading: SharedString,
    pub path: SharedString,
    pub split: bool,
    pub unified: bool,
    pub files: Vec<File>,
    pub cells: Vec<Cell>,
    pub lines: Vec<DiffLine>,
}

impl DiffMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_diff_added_background = token(look.tokens, "diff.added.background");
        let t_diff_filler = token(look.tokens, "diff.filler");
        let t_diff_removed_background = token(look.tokens, "diff.removed.background");
        let t_surface_window = token(look.tokens, "surface.window");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_primary = token(look.tokens, "text.primary");
        let borders = look.borders;
        let Self { id, spoken, heading, path, split, unified, files, cells, lines } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_shrink_0()
            .h(px(200.0))
            .bg(t_surface_window)
            .child(div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .gap(px(1.0))
                .w(px(160.0))
                .py(px(8.0)).px(px(6.0))
                .rule(borders, Sides::RIGHT, 1.0, t_border, false)
                .text_size(rems(0.6875))
                .line_height(rems(0.875))
                .child(div()
                    .pt(px(0.0)).pb(px(6.0)).px(px(4.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t_text_muted)
                    .child(heading.clone()))
                .children(files.into_iter().flat_map(|f| {
                    let f_badge_color = f.badge_color;
                    let f_deep = f.deep;
                    let f_dir = f.dir;
                    let mut out: Vec<AnyElement> = Vec::new();
                    out.push(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .h(px(18.0))
                            .py(px(0.0)).px(px(4.0))
                            .when(f_deep, |this| this.pl(px(16.0)))
                            .when(f_dir, |this| {
                                this
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(t_text_muted)
                            })
                            .child(div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .when(f_dir, |this| this.text_size(rems(0.625)))
                                .child(f.name.clone()))
                            .child(div()
                                .font_family(look.code_font.clone())
                                .text_size(rems(0.625))
                                .line_height(rems(0.875))
                                .text_color(f_badge_color)
                                .child(f.badge.clone()))
                            .into_any_element(),
                    );
                    out
                })))
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .child(div()
                    .py(px(7.0)).px(px(10.0))
                    .rule(borders, Sides::BOTTOM, 1.0, t_border, false)
                    .text_size(rems(0.6875))
                    .line_height(rems(0.875))
                    .text_color(t_text_muted)
                    .child(path.clone()))
                .when(split, |this| {
                    this
                        .child(div()
                            .grid()
                            .grid_cols(2)
                            .font_family(look.code_font.clone())
                            .text_size(rems(0.625))
                            .line_height(rems(0.9375))
                            .children(cells.into_iter().flat_map(|c| {
                                let c_add = c.add;
                                let c_del = c.del;
                                let c_left = c.left;
                                let c_none = c.none;
                                let c_wrap = c.wrap;
                                let mut out: Vec<AnyElement> = Vec::new();
                                out.push(
                                    div()
                                        .py(px(0.0)).px(px(6.0))
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .text_color(t_text_muted)
                                        .when(c_wrap, |this| this.whitespace_normal())
                                        .when(c_left, |this| this.rule(borders, Sides::RIGHT, 1.0, t_border, false))
                                        .when(c_del, |this| {
                                            this
                                                .bg(t_diff_removed_background)
                                                .text_color(t_text_primary)
                                        })
                                        .when(c_add, |this| {
                                            this
                                                .bg(t_diff_added_background)
                                                .text_color(t_text_primary)
                                        })
                                        .when(c_none, |this| this.bg(t_diff_filler))
                                        .child(c.text.clone())
                                        .into_any_element(),
                                );
                                out
                            })))
                })
                .when(unified, |this| {
                    this
                        .child(div()
                            .flex()
                            .flex_col()
                            .font_family(look.code_font.clone())
                            .text_size(rems(0.625))
                            .line_height(rems(0.9375))
                            .children(lines.into_iter().flat_map(|c| {
                                let c_add = c.add;
                                let c_del = c.del;
                                let c_wrap = c.wrap;
                                let mut out: Vec<AnyElement> = Vec::new();
                                out.push(
                                    div()
                                        .py(px(0.0)).px(px(6.0))
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .text_color(t_text_muted)
                                        .when(c_wrap, |this| this.whitespace_normal())
                                        .when(c_del, |this| {
                                            this
                                                .bg(t_diff_removed_background)
                                                .text_color(t_text_primary)
                                        })
                                        .when(c_add, |this| {
                                            this
                                                .bg(t_diff_added_background)
                                                .text_color(t_text_primary)
                                        })
                                        .child(c.text.clone())
                                        .into_any_element(),
                                );
                                out
                            })))
                }))
    }
}
