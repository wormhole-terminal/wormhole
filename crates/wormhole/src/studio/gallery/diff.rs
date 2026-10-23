//! The diff card: its header (the path with the folder muted, what happened to
//! the file, the counts, wrap, Split | Unified and esc) over its own rows (split: hunk
//! header, a replacement padded with hatched fillers, context).

use gpui_kit::assets::IconName;
use gpui_kit::component::Selectable as _;
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::style::{self, Edges, TextRole};

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let tokens = env.tokens;
    let (code_family, code_size) = crate::fonts::code_font(cx);
    let colors = crate::features::git::diff_render::DiffColors::new(tokens);
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::TITLE_ROW_H))
        .px_3()
        .rule_bottom(ui)
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_1()
                .font_family(code_family.clone())
                .text_caption()
                .child(div().min_w_0().truncate().text_color(ui.muted).child("src/api/"))
                .child(div().flex_none().text_color(ui.text).child("orders.rs"))
                .child(style::status_badge(ui, ui.primary, "Renamed")),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .gap_1()
                .font_family(code_family.clone())
                .text_caption()
                .child(div().text_color(colors.added).child("+3"))
                .child(div().text_color(colors.removed).child("−1")),
        )
        .child(style::icon_button("g-diff-wrap", ui, IconName::TextWrap, "Wrap long lines").selected(true))
        .child(
            style::segmented(ui)
                .child(style::segment("g-diff-split", ui, "Split", true))
                .child(style::segment("g-diff-unified", ui, "Unified", false)),
        );
    let rows = crate::features::git::diff_render::sample(ui, colors, code_family, code_size.into());
    env.mark(
        "diff",
        div()
            .flex()
            .flex_col()
            .rounded(ui.radius_lg)
            .overflow_hidden()
            .outlined(ui, ui.border)
            .bg(env.tk("surface.window"))
            .child(header)
            .child(rows),
    )
}
