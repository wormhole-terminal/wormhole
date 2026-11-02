//! `MenuItem`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look};

/// What `MenuItem` is drawn from.
pub struct MenuItemProps {
    pub id: ElementId,
    pub name: SharedString,
    pub hint: Option<SharedString>,
    pub lead: Option<AnyElement>,
    pub chosen: bool,
    pub active: bool,
    pub muted: bool,
    pub destructive: bool,
    pub on_click: Option<Handler>,
}

impl MenuItemProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_primary = token(look.tokens, "primary");
        let t_status_danger_text = token(look.tokens, "status.danger.text");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let Self { id, name, hint, lead, chosen, active, muted, destructive, on_click } = self;

        div()
            .id(id)
            .role(Role::ListBoxOption)
            .aria_label(name.clone())
            .aria_selected(chosen)
            .focusable().tab_stop(true)
            .flex()
            .items_center()
            .gap(px(6.0))
            .w_full()
            .h(px(24.0))
            .flex_shrink_0()
            .py(px(0.0)).pl(px(6.0)).pr(px(10.0))
            .rounded(px(5.0))
            .bg(transparent_black())
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .whitespace_nowrap()
            .text_left()
            .cursor_pointer()
            .when(chosen, |this| {
                this
                    .bg(t_primary)
                    .text_color(t_text_on_primary)
            })
            .when(active, |this| {
                this
                    .bg(t_primary)
                    .text_color(t_text_on_primary)
            })
            .when(muted, |this| this.text_color(t_text_muted))
            .when(destructive, |this| this.text_color(t_status_danger_text))
            .when(muted && chosen, |this| this.text_color(t_text_on_primary))
            .when(muted && active, |this| this.text_color(t_text_on_primary))
            .when(destructive && active, |this| this.text_color(t_text_on_primary))
            .hover(move |style| {
                let style = style.bg(t_primary).text_color(t_text_on_primary);
                let style = if muted { style.text_color(t_text_on_primary) } else { style };
                let style = if destructive { style.text_color(t_text_on_primary) } else { style };
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
                .w(px(14.0))
                .h(px(14.0))
                .when(chosen, |this| {
                    this
                        .child(Icon::new(IconName::Check).with_size(px(12.0)))
                }))
            .when_some(lead, |this, lead| {
                this
                    .child(lead)
            })
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(name.clone()))
            .when_some(hint.clone(), |this, hint| {
                this
                    .child(div()
                        .flex_shrink_0()
                        .text_size(rems(0.75))
                        .child(hint.clone()))
            })
    }
}
