//! `MoreRow`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Force, Rule, Sides};

/// What `MoreRow` is drawn from.
pub struct MoreRowProps {
    pub id: ElementId,
    pub label: SharedString,
    pub value: Option<SharedString>,
    pub open: bool,
    pub closed: bool,
    pub on_click: Option<Handler>,
}

impl MoreRowProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_border_focus = token(look.tokens, "border.focus");
        let t_sidebar_row_hover = token(look.tokens, "sidebar.row.hover");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_primary = token(look.tokens, "text.primary");
        let force = look.force;
        let borders = look.borders;
        let Self { id, label, value, open, closed, on_click } = self;

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label.clone())
            .when_some(value.clone(), |this, text| this.aria_description(text))
            .when(open, |this| this.aria_expanded(true))
            .focusable().tab_stop(true)
            .flex()
            .items_center()
            .gap(px(12.0))
            .w_full()
            .min_h(px(40.0))
            .py(px(8.0)).px(px(14.0))
            .rule(borders, Sides::TOP, 1.0, t_border, false)
            .bg(transparent_black())
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .text_left()
            .cursor_pointer()
            .when(force == Force::Hover, |this| {
                this
                    .bg(t_sidebar_row_hover)
            })
            .hover(move |style| {
                let style = style.bg(t_sidebar_row_hover);
                style
            })
            .when(force == Force::Focus, |this| {
                this
                    .shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }])
            })
            .focus_visible(move |style| {
                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                style
            })
            .when_some(on_click.clone(), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(label.clone()))
            .when_some(value.clone(), |this, value| {
                this
                    .child(div()
                        .flex_shrink_0()
                        .text_color(t_text_muted)
                        .child(value.clone()))
            })
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(14.0))
                .h(px(14.0))
                .text_color(t_text_muted)
                .when(open, |this| {
                    this
                        .child(Icon::new(IconName::ChevronDown).with_size(px(12.0)))
                })
                .when(closed, |this| {
                    this
                        .child(Icon::new(IconName::ChevronRight).with_size(px(12.0)))
                }))
    }
}
