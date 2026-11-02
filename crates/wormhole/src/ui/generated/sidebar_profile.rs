//! `SidebarProfile`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Force};

/// What `SidebarProfile` is drawn from.
pub struct SidebarProfileProps {
    pub id: ElementId,
    pub label: SharedString,
    pub letter: SharedString,
    pub name: SharedString,
    pub caption: SharedString,
    pub selected: bool,
    pub hits: Option<SharedString>,
    pub on_click: Option<Handler>,
}

impl SidebarProfileProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border_focus = token(look.tokens, "border.focus");
        let t_primary = token(look.tokens, "primary");
        let t_settings_nav_selected = token(look.tokens, "settings.nav.selected");
        let t_sidebar_row_hover = token(look.tokens, "sidebar.row.hover");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let force = look.force;
        let Self { id, label, letter, name, caption, selected, hits, on_click } = self;

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_selected(selected)
            .focusable().tab_stop(true)
            .flex()
            .items_center()
            .gap(px(10.0))
            .w_full()
            .h(px(40.0))
            .flex_shrink_0()
            .py(px(0.0)).px(px(6.0))
            .rounded(px(7.0))
            .bg(transparent_black())
            .text_color(t_text_primary)
            .text_left()
            .cursor_pointer()
            .when(selected, |this| {
                this
                    .bg(t_settings_nav_selected)
                    .text_color(t_text_on_primary)
            })
            .when(force == Force::Hover, |this| {
                this
                    .bg(t_sidebar_row_hover)
            })
            .when(force == Force::Hover && selected, |this| {
                this
                    .bg(t_settings_nav_selected)
            })
            .hover(move |style| {
                let style = style.bg(t_sidebar_row_hover);
                let style = if selected { style.bg(t_settings_nav_selected) } else { style };
                style
            })
            .when(force == Force::Focus, |this| {
                this
                    .shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }])
            })
            .when(force == Force::Focus && selected, |this| {
                this
                    .shadow(vec![BoxShadow { color: t_text_primary, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }])
            })
            .focus_visible(move |style| {
                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                let style = if selected { style.shadow(vec![BoxShadow { color: t_text_primary, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]) } else { style };
                style
            })
            .when_some(on_click.clone(), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(28.0))
                .h(px(28.0))
                .rounded_full()
                .bg(t_primary)
                .text_color(t_text_on_primary)
                .text_size(rems(0.8125))
                .line_height(rems(1.0))
                .font_weight(FontWeight::SEMIBOLD)
                .when(selected, |this| {
                    this
                        .bg(t_text_on_primary)
                        .text_color(t_settings_nav_selected)
                })
                .child(letter.clone()))
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(div()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(rems(0.8125))
                    .line_height(rems(1.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(name.clone()))
                .child(div()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(rems(0.6875))
                    .line_height(rems(0.875))
                    .text_color(t_text_muted)
                    .when(selected, |this| this.text_color(t_text_on_primary))
                    .child(caption.clone())))
            .when_some(hits.clone(), |this, hits| {
                this
                    .child(div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_shrink_0()
                        .min_w(px(18.0))
                        .h(px(18.0))
                        .py(px(0.0)).px(px(5.0))
                        .rounded(px(9.0))
                        .bg(t_primary)
                        .text_color(t_text_on_primary)
                        .text_size(rems(0.6875))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(rems(0.875))
                        .when(selected, |this| {
                            this
                                .bg(t_text_on_primary)
                                .text_color(t_settings_nav_selected)
                        })
                        .child(hits.clone()))
            })
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(10.0))
                .h(px(12.0))
                .text_color(t_text_muted)
                .when(selected, |this| this.text_color(t_text_on_primary))
                .child(Icon::new(IconName::ChevronsUpDown).with_size(px(12.0))))
    }
}
