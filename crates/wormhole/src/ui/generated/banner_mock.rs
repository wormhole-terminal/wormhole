//! `BannerMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// One item of `banners`.
#[derive(Default)]
pub struct Banner {
    pub title: SharedString,
    pub body: SharedString,
}

/// What `BannerMock` is drawn from.
pub struct BannerMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub when: SharedString,
    pub none: bool,
    pub empty: SharedString,
    pub banners: Vec<Banner>,
}

impl BannerMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_primary = token(look.tokens, "primary");
        let t_shadow = token(look.tokens, "shadow");
        let t_surface_elevated = token(look.tokens, "surface.elevated");
        let t_surface_panel = token(look.tokens, "surface.panel");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_tile_mark_brand = token(look.tokens, "tile.mark.brand");
        let Self { id, spoken, when, none, empty, banners } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .items_end()
            .gap(px(8.0))
            .min_h(px(64.0))
            .p(px(12.0))
            .bg(t_surface_panel)
            .children(banners.into_iter().flat_map(|b| {
                let mut out: Vec<AnyElement> = Vec::new();
                out.push(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .w(px(340.0))
                        .py(px(10.0)).px(px(12.0))
                        .rounded(px(14.0))
                        .bg(t_surface_elevated)
                        .shadow(vec![BoxShadow { color: t_shadow, offset: point(px(0.0), px(6.0)), blur_radius: px(18.0), spread_radius: px(0.0), inset: false }])
                        .child(div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .w(px(28.0))
                            .h(px(28.0))
                            .rounded(px(7.0))
                            .bg(t_primary)
                            .text_color(t_tile_mark_brand)
                            .child(Icon::new(IconName::Terminal).with_size(px(14.0))))
                        .child(div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .text_size(rems(0.75))
                            .line_height(rems(1.0))
                            .child(div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(b.title.clone()))
                            .child(div()
                                .text_color(t_text_muted)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(b.body.clone())))
                        .child(div()
                            .self_start()
                            .text_size(rems(0.625))
                            .line_height(rems(0.875))
                            .text_color(t_text_muted)
                            .child(when.clone()))
                        .into_any_element(),
                );
                out
            }))
            .when(none, |this| {
                this
                    .child(div()
                        .self_center()
                        .py(px(12.0)).px(px(0.0))
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(t_text_muted)
                        .child(empty.clone()))
            })
    }
}
