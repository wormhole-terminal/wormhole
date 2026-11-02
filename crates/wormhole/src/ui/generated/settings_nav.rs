//! `SettingsNav`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look};

/// One item of `rows`.
#[derive(Default)]
pub struct NavEntry {
    pub key: usize,
    pub is_gap: bool,
    pub is_sub_end: bool,
    pub is_label: bool,
    pub is_item: bool,
    pub is_sub: bool,
    pub name: SharedString,
    pub spoken: SharedString,
    pub icon: Option<IconName>,
    pub tile: Hsla,
    pub selected: bool,
    pub dim: bool,
    pub follows: bool,
    pub first: bool,
    pub current: bool,
    pub preview: bool,
    pub opened: bool,
    pub count: Option<SharedString>,
    pub has_dot: bool,
    pub pick: Option<Handler>,
}

/// What `SettingsNav` is drawn from.
pub struct SettingsNavProps {
    pub rows: Vec<NavEntry>,
}

impl SettingsNavProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Div {
        let t_agent_attention = token(look.tokens, "agent.attention");
        let t_border_focus = token(look.tokens, "border.focus");
        let t_primary = token(look.tokens, "primary");
        let t_settings_nav_selected = token(look.tokens, "settings.nav.selected");
        let t_sidebar_row_hover = token(look.tokens, "sidebar.row.hover");
        let t_status_info = token(look.tokens, "status.info");
        let t_text_faint = token(look.tokens, "text.faint");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let t_tile_mark_brand = token(look.tokens, "tile.mark.brand");
        let Self { rows } = self;

        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .py(px(0.0)).px(px(12.0))
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .children(rows.into_iter().flat_map(|row| {
                let row_current = row.current;
                let row_dim = row.dim;
                let row_first = row.first;
                let row_follows = row.follows;
                let row_has_dot = row.has_dot;
                let row_is_gap = row.is_gap;
                let row_is_item = row.is_item;
                let row_is_label = row.is_label;
                let row_is_sub = row.is_sub;
                let row_is_sub_end = row.is_sub_end;
                let row_key = row.key;
                let row_opened = row.opened;
                let row_preview = row.preview;
                let row_selected = row.selected;
                let row_tile = row.tile;
                let mut out: Vec<AnyElement> = Vec::new();
                if row_is_gap {
                    out.push(
                        div()
                            .h(px(20.0))
                            .flex_shrink_0()
                            .into_any_element(),
                    );
                }
                if row_is_sub_end {
                    out.push(
                        div()
                            .h(px(4.0))
                            .flex_shrink_0()
                            .into_any_element(),
                    );
                }
                if row_is_label {
                    out.push(
                        div()
                            .flex()
                            .items_center()
                            .h(px(22.0))
                            .flex_shrink_0()
                            .pt(px(0.0)).pb(px(2.0)).px(px(8.0))
                            .text_size(rems(0.6875))
                            .line_height(rems(0.875))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t_text_faint)
                            .child(row.name.clone())
                            .into_any_element(),
                    );
                }
                if row_is_item {
                    out.push(
                        div()
                            .id(("settings-nav", row_key))
                            .role(Role::Button)
                            .aria_label(row.spoken.clone())
                            .aria_selected(row_selected)
                            .when(row_opened, |this| this.aria_expanded(true))
                            .focusable().tab_stop(true)
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .w_full()
                            .h(px(28.0))
                            .flex_shrink_0()
                            .py(px(0.0)).px(px(8.0))
                            .rounded(px(7.0))
                            .bg(transparent_black())
                            .text_color(t_text_primary)
                            .text_size(rems(0.8125))
                            .line_height(rems(1.0))
                            .text_left()
                            .cursor_pointer()
                            .when(row_follows, |this| this.mt(px(2.0)))
                            .when(row_dim, |this| this.text_color(t_text_faint))
                            .when(row_selected, |this| {
                                this
                                    .bg(t_settings_nav_selected)
                                    .text_color(t_text_on_primary)
                                    .font_weight(FontWeight::SEMIBOLD)
                            })
                            .hover(move |style| {
                                let style = style.bg(t_sidebar_row_hover);
                                let style = if row_selected { style.bg(t_settings_nav_selected) } else { style };
                                style
                            })
                            .focus_visible(move |style| {
                                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                                let style = if row_selected { style.shadow(vec![BoxShadow { color: t_text_primary, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]) } else { style };
                                style
                            })
                            .when_some(row.pick.clone(), |this, handler| {
                                this.on_click(move |event, window, cx| handler(event, window, cx))
                            })
                            .child(div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .flex_shrink_0()
                                .w(px(20.0))
                                .h(px(20.0))
                                .rounded(px(6.0))
                                .when(row_dim, |this| this.opacity(0.5))
                                .bg(row_tile)
                                .when_some(row.icon.clone(), |this, row_icon| {
                                    this
                                        .child(Icon::new(row_icon).with_size(px(13.0))
                                            .text_color(t_tile_mark_brand))
                                }))
                            .child(div()
                                .flex_1()
                                .min_w(px(0.0))
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(row.name.clone()))
                            .when_some(row.count.clone(), |this, row_count| {
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
                                        .when(row_selected, |this| this.bg(t_text_on_primary.opacity(0.25)))
                                        .child(row_count.clone()))
                            })
                            .when(row_has_dot, |this| {
                                this
                                    .child(div()
                                        .w(px(6.0))
                                        .h(px(6.0))
                                        .flex_shrink_0()
                                        .rounded_full()
                                        .bg(t_agent_attention)
                                        .when(row_preview, |this| this.bg(t_status_info))
                                        .when(row_selected, |this| this.bg(t_text_on_primary)))
                            })
                            .into_any_element(),
                    );
                }
                if row_is_sub {
                    out.push(
                        div()
                            .id(("settings-nav-group", row_key))
                            .role(Role::Button)
                            .aria_label(row.spoken.clone())
                            .aria_selected(row_current)
                            .focusable().tab_stop(true)
                            .flex()
                            .items_center()
                            .w_full()
                            .h(px(24.0))
                            .flex_shrink_0()
                            .py(px(0.0)).pl(px(38.0)).pr(px(8.0))
                            .rounded(px(6.0))
                            .bg(transparent_black())
                            .text_color(t_text_muted)
                            .text_size(rems(0.75))
                            .line_height(rems(1.0))
                            .text_left()
                            .cursor_pointer()
                            .when(row_first, |this| this.mt(px(4.0)))
                            .when(row_current, |this| {
                                this
                                    .text_color(t_text_primary)
                                    .font_weight(FontWeight::SEMIBOLD)
                            })
                            .hover(move |style| {
                                let style = style.text_color(t_text_primary);
                                style
                            })
                            .focus_visible(move |style| {
                                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                                style
                            })
                            .when_some(row.pick.clone(), |this, handler| {
                                this.on_click(move |event, window, cx| handler(event, window, cx))
                            })
                            .child(div()
                                .flex_1()
                                .min_w(px(0.0))
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(row.name.clone()))
                            .into_any_element(),
                    );
                }
                out
            }))
    }
}
