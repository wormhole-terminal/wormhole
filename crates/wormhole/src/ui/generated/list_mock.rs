//! `ListMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, DesignIcon, Rule, Sides};

/// One item of `rows`.
#[derive(Default)]
pub struct Row {
    pub name: SharedString,
    pub status: SharedString,
    pub has_status: bool,
    pub selected: bool,
    pub compact: bool,
    pub dense: bool,
    pub tile: Hsla,
    pub icon: Option<DesignIcon>,
    pub dot: bool,
    pub dot_color: Hsla,
    pub status_color: Hsla,
    pub changes: bool,
    pub added: SharedString,
    pub removed: SharedString,
    pub add_color: Hsla,
    pub remove_color: Hsla,
    pub notes: bool,
}

/// One item of `pane`.
#[derive(Default)]
pub struct PaneLine {
    pub text: SharedString,
    pub color: Hsla,
}

/// What `ListMock` is drawn from.
pub struct ListMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub cards: bool,
    pub flat: bool,
    pub group_fill: Hsla,
    pub group_line: Hsla,
    pub group: SharedString,
    pub folder: Option<SharedString>,
    pub pane_bg: Hsla,
    pub rows: Vec<Row>,
    pub pane: Vec<PaneLine>,
}

impl ListMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_primary = token(look.tokens, "primary");
        let t_sidebar_background = token(look.tokens, "sidebar.background");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let t_tile_mark_brand = token(look.tokens, "tile.mark.brand");
        let borders = look.borders;
        let Self { id, spoken, cards, flat, group_fill, group_line, group, folder, pane_bg, rows, pane } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_shrink_0()
            .h(px(210.0))
            .child(div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .w(px(220.0))
                .py(px(10.0)).px(px(8.0))
                .bg(t_sidebar_background)
                .child(div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .p(px(0.0))
                    .when(cards, |this| {
                        this
                            .p(px(4.0))
                            .rounded(px(10.0))
                    })
                    .bg(group_fill)
                    .when(flat, |this| {
                        this
                            .child(div()
                                .h(px(2.0))
                                .rounded(px(1.0))
                                .bg(group_line))
                    })
                    .child(div()
                        .flex()
                        .flex_col()
                        .pt(px(2.0)).pb(px(6.0)).px(px(6.0))
                        .child(div()
                            .text_size(rems(0.75))
                            .line_height(rems(1.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(group.clone()))
                        .when_some(folder.clone(), |this, folder| {
                            this
                                .child(div()
                                    .text_size(rems(0.6875))
                                    .line_height(rems(0.875))
                                    .text_color(t_text_muted)
                                    .child(folder.clone()))
                        }))
                    .children(rows.into_iter().flat_map(|r| {
                        let r_add_color = r.add_color;
                        let r_changes = r.changes;
                        let r_compact = r.compact;
                        let r_dense = r.dense;
                        let r_dot = r.dot;
                        let r_dot_color = r.dot_color;
                        let r_has_status = r.has_status;
                        let r_notes = r.notes;
                        let r_remove_color = r.remove_color;
                        let r_selected = r.selected;
                        let r_status_color = r.status_color;
                        let r_tile = r.tile;
                        let mut out: Vec<AnyElement> = Vec::new();
                        out.push(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .h(px(42.0))
                                .py(px(0.0)).px(px(8.0))
                                .rounded(px(7.0))
                                .when(r_compact, |this| this.h(px(34.0)))
                                .when(r_dense, |this| this.h(px(26.0)))
                                .when(r_selected, |this| this.bg(t_primary))
                                .child(div()
                                    .relative()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .flex_shrink_0()
                                    .w(px(22.0))
                                    .h(px(22.0))
                                    .rounded(px(6.0))
                                    .text_color(t_tile_mark_brand)
                                    .when(r_dense, |this| {
                                        this
                                            .w(px(18.0))
                                            .h(px(18.0))
                                    })
                                    .bg(r_tile)
                                    .when_some(r.icon.clone(), |this, r_icon| {
                                        this
                                            .child(Icon::new(r_icon).with_size(px(12.0)))
                                    })
                                    .when(r_dot, |this| {
                                        this
                                            .child(div()
                                                .absolute()
                                                .right(px(-3.0))
                                                .bottom(px(-3.0))
                                                .w(px(12.0))
                                                .h(px(12.0))
                                                .rounded_full()
                                                .rule(borders, Sides::ALL, 2.0, t_sidebar_background, true)
                                                .bg(r_dot_color))
                                    }))
                                .child(div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .flex()
                                    .flex_col()
                                    .child(div()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .text_size(rems(0.75))
                                        .line_height(rems(1.0))
                                        .text_color(t_text_primary)
                                        .when(r_selected, |this| {
                                            this
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(t_text_on_primary)
                                        })
                                        .child(r.name.clone()))
                                    .when(r_has_status, |this| {
                                        this
                                            .child(div()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .whitespace_nowrap()
                                                .text_size(rems(0.6875))
                                                .line_height(rems(0.875))
                                                .text_color(r_status_color)
                                                .child(r.status.clone()))
                                    }))
                                .when(r_changes, |this| {
                                    this
                                        .child(div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .flex_shrink_0()
                                            .font_family(look.code_font.clone())
                                            .text_size(rems(0.625))
                                            .line_height(rems(0.875))
                                            .whitespace_nowrap()
                                            .child(div()
                                                .text_color(r_add_color)
                                                .child(r.added.clone()))
                                            .child(div()
                                                .text_color(r_remove_color)
                                                .child(r.removed.clone())))
                                })
                                .when(r_notes, |this| {
                                    this
                                        .child(Icon::new(IconName::FileText).with_size(px(11.0)))
                                })
                                .into_any_element(),
                        );
                        out
                    }))))
            .child(div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(3.0))
                .py(px(10.0)).px(px(12.0))
                .font_family(look.code_font.clone())
                .text_size(rems(0.6875))
                .line_height(rems(0.9375))
                .overflow_hidden()
                .bg(pane_bg)
                .children(pane.into_iter().flat_map(|l| {
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
    }
}
