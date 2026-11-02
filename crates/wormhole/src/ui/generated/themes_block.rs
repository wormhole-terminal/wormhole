//! `ThemesBlock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Force};

/// What `ThemesBlock` is drawn from.
pub struct ThemesBlockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub show_all: SharedString,
    pub on_show_all: Option<Handler>,
    pub hidden: Option<SharedString>,
    pub on_hidden: Option<Handler>,
    pub buttons: AnyElement,
    pub sets: AnyElement,
}

impl ThemesBlockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border_focus = token(look.tokens, "border.focus");
        let t_primary = token(look.tokens, "primary");
        let force = look.force;
        let Self { id, spoken, show_all, on_show_all, hidden, on_hidden, buttons, sets } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .gap(px(12.0))
            .pt(px(12.0)).pb(px(14.0)).px(px(14.0))
            .child(sets)
            .child(div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pt(px(2.0))
                .child(div()
                    .flex_1()
                    .flex()
                    .gap(px(12.0))
                    .child(div()
                        .id("themes-all")
                        .role(Role::Button)
                        .focusable().tab_stop(true)
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(t_primary)
                        .cursor_pointer()
                        .when(force == Force::Hover, |this| {
                            this
                                .underline()
                        })
                        .hover(move |style| {
                            let style = style.underline();
                            style
                        })
                        .when(force == Force::Focus, |this| {
                            this
                                .shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }])
                        })
                        .focus_visible(move |style| {
                            let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                            style
                        })
                        .when_some(on_show_all.clone(), |this, handler| {
                            this.on_click(move |event, window, cx| handler(event, window, cx))
                        })
                        .child(show_all.clone()))
                    .when_some(hidden.clone(), |this, hidden| {
                        this
                            .child(div()
                                .id("themes-hidden")
                                .role(Role::Button)
                                .focusable().tab_stop(true)
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(t_primary)
                                .cursor_pointer()
                                .when(force == Force::Hover, |this| {
                                    this
                                        .underline()
                                })
                                .hover(move |style| {
                                    let style = style.underline();
                                    style
                                })
                                .when(force == Force::Focus, |this| {
                                    this
                                        .shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }])
                                })
                                .focus_visible(move |style| {
                                    let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: false }]);
                                    style
                                })
                                .when_some(on_hidden.clone(), |this, handler| {
                                    this.on_click(move |event, window, cx| handler(event, window, cx))
                                })
                                .child(hidden.clone()))
                    }))
                .child(buttons))
    }
}
