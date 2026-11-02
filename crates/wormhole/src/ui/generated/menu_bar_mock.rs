//! `MenuBarMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look};

/// One item of `items`.
#[derive(Default)]
pub struct MenuItem {
    pub name: SharedString,
    pub key: SharedString,
    pub key_color: Hsla,
    pub selected: bool,
}

/// What `MenuBarMock` is drawn from.
pub struct MenuBarMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub shown: bool,
    pub dot: bool,
    pub count: bool,
    pub count_text: SharedString,
    pub clock: SharedString,
    pub header: SharedString,
    pub items: Vec<MenuItem>,
}

impl MenuBarMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_agent_attention = token(look.tokens, "agent.attention");
        let t_button_hover = token(look.tokens, "button.hover");
        let t_primary = token(look.tokens, "primary");
        let t_shadow = token(look.tokens, "shadow");
        let t_surface_elevated = token(look.tokens, "surface.elevated");
        let t_surface_panel = token(look.tokens, "surface.panel");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let t_titlebar_background = token(look.tokens, "titlebar.background");
        let Self { id, spoken, shown, dot, count, count_text, clock, header, items } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .pb(px(12.0))
            .bg(t_surface_panel)
            .child(div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(12.0))
                .h(px(24.0))
                .py(px(0.0)).px(px(10.0))
                .bg(t_titlebar_background)
                .text_color(t_text_primary)
                .text_size(rems(0.6875))
                .line_height(rems(0.8125))
                .when(shown, |this| {
                    this
                        .child(div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .h(px(18.0))
                            .py(px(0.0)).px(px(5.0))
                            .rounded(px(4.0))
                            .bg(t_button_hover)
                            .child(Icon::new(IconName::Terminal).with_size(px(12.0)))
                            .when(dot, |this| {
                                this
                                    .child(div()
                                        .w(px(6.0))
                                        .h(px(6.0))
                                        .rounded_full()
                                        .bg(t_agent_attention))
                            })
                            .when(count, |this| {
                                this
                                    .child(div()
                                        .child(count_text.clone()))
                            }))
                })
                .child(div()
                    .child(clock.clone())))
            .when(shown, |this| {
                this
                    .child(div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .w(px(240.0))
                        .self_end()
                        .mt(px(6.0))
                        .mr(px(10.0))
                        .p(px(6.0))
                        .rounded(px(10.0))
                        .bg(t_surface_elevated)
                        .shadow(vec![BoxShadow { color: t_shadow, offset: point(px(0.0), px(10.0)), blur_radius: px(28.0), spread_radius: px(0.0), inset: false }])
                        .text_size(rems(0.75))
                        .line_height(rems(0.875))
                        .child(div()
                            .py(px(2.0)).px(px(6.0))
                            .text_size(rems(0.625))
                            .line_height(rems(0.75))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t_text_muted)
                            .child(header.clone()))
                        .children(items.into_iter().flat_map(|i| {
                            let i_key_color = i.key_color;
                            let i_selected = i.selected;
                            let mut out: Vec<AnyElement> = Vec::new();
                            out.push(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .py(px(4.0)).px(px(6.0))
                                    .rounded(px(5.0))
                                    .when(i_selected, |this| {
                                        this
                                            .bg(t_primary)
                                            .text_color(t_text_on_primary)
                                    })
                                    .child(div()
                                        .child(i.name.clone()))
                                    .child(div()
                                        .text_color(i_key_color)
                                        .child(i.key.clone()))
                                    .into_any_element(),
                            );
                            out
                        })))
            })
    }
}
