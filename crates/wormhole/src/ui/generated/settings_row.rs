//! `SettingsRow`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Rule, Sides};

/// What `SettingsRow` is drawn from.
pub struct SettingsRowProps {
    pub id: ElementId,
    pub name: SharedString,
    pub description: Option<SharedString>,
    pub label: AnyElement,
    pub tall: bool,
    pub disabled: bool,
    pub changed: bool,
    pub project: Option<SharedString>,
    pub chip: Option<SharedString>,
    pub info: Option<AnyElement>,
    pub key_text: Option<SharedString>,
    pub key_name: SharedString,
    pub copy_label: SharedString,
    pub on_copy: Option<Handler>,
    pub dormant: bool,
    pub copied: bool,
    pub hint: Option<SharedString>,
    pub on_reset: Option<Handler>,
    pub reset_label: SharedString,
    pub extra: Option<AnyElement>,
    pub control: AnyElement,
}

impl SettingsRowProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_border_focus = token(look.tokens, "border.focus");
        let t_button_background = token(look.tokens, "button.background");
        let t_primary = token(look.tokens, "primary");
        let t_sidebar_row_hover = token(look.tokens, "sidebar.row.hover");
        let t_text_faint = token(look.tokens, "text.faint");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let borders = look.borders;
        let Self { id, name, description, label, tall, disabled, changed, project, chip, info, key_text, key_name, copy_label, on_copy, dormant, copied, hint, on_reset, reset_label, extra, control } = self;

        div()
            .id(id)
            .group("settings-row-sr")
            .role(Role::Group)
            .aria_label(name.clone())
            .when_some(description.clone(), |this, text| this.aria_description(text))
            .relative()
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_h(px(40.0))
            .py(px(8.0)).px(px(14.0))
            .rule(borders, Sides::TOP, 1.0, t_border, false)
            .rounded(px(10.0))
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .when(tall, |this| this.items_start())
            .hover(move |style| {
                let style = style.bg(t_sidebar_row_hover);
                let style = if disabled { style.bg(transparent_black()) } else { style };
                style
            })
            .child(div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .min_w(px(0.0))
                    .when(tall, |this| this.min_h(px(24.0)))
                    .when(changed, |this| {
                        this
                            .child(div()
                                .w(px(7.0))
                                .h(px(7.0))
                                .rounded_full()
                                .flex_shrink_0()
                                .bg(t_primary))
                    })
                    .child(div()
                        .flex_shrink_0()
                        .text_color(t_text_primary)
                        .when(disabled, |this| this.text_color(t_text_faint))
                        .child(label))
                    .when_some(project.clone(), |this, project| {
                        this
                            .child(div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .flex_shrink_0()
                                .h(px(16.0))
                                .py(px(0.0)).px(px(6.0))
                                .rounded(px(8.0))
                                .bg(t_primary)
                                .text_color(t_text_on_primary)
                                .text_size(rems(0.6875))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(rems(0.875))
                                .whitespace_nowrap()
                                .child(Icon::new(IconName::Folder).with_size(px(11.0)))
                                .child(project.clone()))
                    })
                    .when_some(chip.clone(), |this, chip| {
                        this
                            .child(div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .flex_shrink_0()
                                .h(px(16.0))
                                .py(px(0.0)).px(px(6.0))
                                .rounded(px(8.0))
                                .bg(t_primary)
                                .text_color(t_text_on_primary)
                                .text_size(rems(0.6875))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(rems(0.875))
                                .whitespace_nowrap()
                                .child(chip.clone()))
                    })
                    .when_some(info, |this, info| {
                        this
                            .child(div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .flex_shrink_0()
                                .w(px(16.0))
                                .h(px(16.0))
                                .text_color(t_text_faint)
                                .child(info))
                    })
                    .when_some(key_text.clone(), |this, key_text| {
                        this
                            .child(div()
                                .id("e1")
                                .role(Role::Button)
                                .aria_label(copy_label.clone())
                                .aria_description(key_name.clone())
                                .focusable().tab_stop(true)
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .items_center()
                                .h(px(18.0))
                                .text_color(t_text_faint)
                                .font_family(look.code_font.clone())
                                .text_size(rems(0.6875))
                                .line_height(rems(0.875))
                                .text_left()
                                .cursor(CursorStyle::DragCopy)
                                .when(copied, |this| this.text_color(t_primary))
                                .when(dormant, |this| this.text_color(transparent_black()))
                                .group_hover("settings-row-sr", move |style| {
                                    let style = if dormant { style.text_color(t_text_faint) } else { style };
                                    style
                                })
                                .focus_visible(move |style| {
                                    let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                                    style
                                })
                                .when_some(on_copy.clone(), |this, handler| {
                                    this.on_click(move |event, window, cx| handler(event, window, cx))
                                })
                                .child(div()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(key_text.clone())))
                    }))
                .when_some(hint.clone(), |this, hint| {
                    this
                        .child(div()
                            .text_size(rems(0.75))
                            .line_height(rems(0.875))
                            .text_color(t_text_muted)
                            .when(disabled, |this| this.text_color(t_text_faint))
                            .child(hint.clone()))
                }))
            .child(div()
                .relative()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.0))
                .when(on_reset.is_some(), |this| {
                    this
                        .child(div()
                            .id("e2")
                            .role(Role::Button)
                            .aria_label(reset_label.clone())
                            .focusable().tab_stop(true)
                            .w(px(22.0))
                            .h(px(22.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .rounded(px(6.0))
                            .bg(transparent_black())
                            .text_color(t_text_muted)
                            .cursor_pointer()
                            .when(dormant, |this| this.text_color(transparent_black()))
                            .group_hover("settings-row-sr", move |style| {
                                let style = if dormant { style.text_color(t_text_muted) } else { style };
                                style
                            })
                            .hover(move |style| {
                                let style = style.bg(t_button_background);
                                style
                            })
                            .focus_visible(move |style| {
                                let style = style.shadow(vec![BoxShadow { color: t_border_focus, offset: point(px(0.0), px(0.0)), blur_radius: px(0.0), spread_radius: px(2.0), inset: true }]);
                                style
                            })
                            .when_some(on_reset.clone(), |this, handler| {
                                this.on_click(move |event, window, cx| handler(event, window, cx))
                            })
                            .child(Icon::new(IconName::RotateCcw).with_size(px(13.0))))
                })
                .when_some(extra, |this, extra| {
                    this
                        .child(extra)
                })
                .child(control))
    }
}
