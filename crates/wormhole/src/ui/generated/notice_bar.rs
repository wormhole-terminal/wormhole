//! `NoticeBar`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// What `NoticeBar` is drawn from.
pub struct NoticeBarProps {
    pub id: ElementId,
    pub text: SharedString,
    pub warning: bool,
    pub danger: bool,
    pub alerting: bool,
    pub informing: bool,
    pub actions: Option<AnyElement>,
}

impl NoticeBarProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_banner_danger_solid = token(look.tokens, "banner.danger.solid");
        let t_banner_danger_solid_text = token(look.tokens, "banner.danger.solid.text");
        let t_banner_info_solid = token(look.tokens, "banner.info.solid");
        let t_banner_info_solid_text = token(look.tokens, "banner.info.solid.text");
        let t_banner_warning_solid = token(look.tokens, "banner.warning.solid");
        let t_banner_warning_solid_text = token(look.tokens, "banner.warning.solid.text");
        let Self { id, text, warning, danger, alerting, informing, actions } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(text.clone())
            .flex()
            .items_center()
            .gap(px(8.0))
            .py(px(10.0)).px(px(12.0))
            .rounded(px(10.0))
            .text_size(rems(0.75))
            .line_height(rems(1.0))
            .bg(t_banner_info_solid)
            .text_color(t_banner_info_solid_text)
            .when(warning, |this| {
                this
                    .bg(t_banner_warning_solid)
                    .text_color(t_banner_warning_solid_text)
            })
            .when(danger, |this| {
                this
                    .bg(t_banner_danger_solid)
                    .text_color(t_banner_danger_solid_text)
            })
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(14.0))
                .h(px(16.0))
                .when(alerting, |this| {
                    this
                        .child(Icon::new(IconName::TriangleAlert).with_size(px(14.0)))
                })
                .when(informing, |this| {
                    this
                        .child(Icon::new(IconName::Info).with_size(px(14.0)))
                }))
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .child(text.clone()))
            .when_some(actions, |this, actions| {
                this
                    .child(div()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .gap(px(8.0))
                        .child(actions))
            })
    }
}
