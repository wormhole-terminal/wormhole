//! `TileRow`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Look, Rule, Sides};

/// What `TileRow` is drawn from.
pub struct TileRowProps {
    pub id: ElementId,
    pub name: SharedString,
    pub spoken: Option<SharedString>,
    pub name_color: Hsla,
    pub tile_fill: Hsla,
    pub brand: Option<AnyElement>,
    pub icon: Option<IconName>,
    pub info: Option<AnyElement>,
    pub chip: Option<SharedString>,
    pub good: bool,
    pub warn: bool,
    pub danger: bool,
    pub primary: bool,
    pub hint: Option<SharedString>,
    pub note: Option<SharedString>,
    pub note_color: Hsla,
    pub controls: AnyElement,
}

impl TileRowProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_chip_danger_fill = token(look.tokens, "chip.danger.fill");
        let t_chip_good_fill = token(look.tokens, "chip.good.fill");
        let t_chip_quiet_fill = token(look.tokens, "chip.quiet.fill");
        let t_chip_solid_text = token(look.tokens, "chip.solid.text");
        let t_chip_warn_fill = token(look.tokens, "chip.warn.fill");
        let t_primary = token(look.tokens, "primary");
        let t_text_faint = token(look.tokens, "text.faint");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_on_primary = token(look.tokens, "text.on_primary");
        let t_text_primary = token(look.tokens, "text.primary");
        let t_tile_mark_brand = token(look.tokens, "tile.mark.brand");
        let borders = look.borders;
        let Self { id, name, spoken, name_color, tile_fill, brand, icon, info, chip, good, warn, danger, primary, hint, note, note_color, controls } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(name.clone())
            .when_some(spoken.clone(), |this, text| this.aria_description(text))
            .flex()
            .items_center()
            .gap(px(12.0))
            .w_full()
            .min_h(px(56.0))
            .py(px(8.0)).px(px(14.0))
            .rule(borders, Sides::TOP, 1.0, t_border, false)
            .text_color(t_text_primary)
            .text_size(rems(0.8125))
            .line_height(rems(1.0))
            .child(div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(28.0))
                .h(px(28.0))
                .rounded(px(7.0))
                .text_color(t_tile_mark_brand)
                .bg(tile_fill)
                .when_some(brand, |this, brand| {
                    this
                        .child(brand)
                })
                .when_some(icon.clone(), |this, icon| {
                    this
                        .child(Icon::new(icon).with_size(px(14.0)))
                }))
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
                    .child(div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_color(name_color)
                        .child(name.clone()))
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
                    .when_some(chip.clone(), |this, chip| {
                        this
                            .child(div()
                                .flex()
                                .items_center()
                                .flex_shrink_0()
                                .h(px(18.0))
                                .py(px(0.0)).px(px(7.0))
                                .rounded(px(9.0))
                                .bg(t_chip_quiet_fill)
                                .text_color(t_chip_solid_text)
                                .text_size(rems(0.6875))
                                .line_height(rems(0.875))
                                .font_weight(FontWeight::SEMIBOLD)
                                .whitespace_nowrap()
                                .when(good, |this| this.bg(t_chip_good_fill))
                                .when(warn, |this| this.bg(t_chip_warn_fill))
                                .when(danger, |this| this.bg(t_chip_danger_fill))
                                .when(primary, |this| {
                                    this
                                        .bg(t_primary)
                                        .text_color(t_text_on_primary)
                                })
                                .child(chip.clone()))
                    }))
                .when_some(hint.clone(), |this, hint| {
                    this
                        .child(div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(rems(0.75))
                            .line_height(rems(0.875))
                            .text_color(t_text_muted)
                            .child(hint.clone()))
                })
                .when_some(note.clone(), |this, note| {
                    this
                        .child(div()
                            .text_size(rems(0.75))
                            .line_height(rems(0.875))
                            .text_color(note_color)
                            .child(note.clone()))
                }))
            .child(div()
                .flex()
                .items_center()
                .justify_end()
                .flex_shrink_0()
                .gap(px(8.0))
                .child(controls))
    }
}
