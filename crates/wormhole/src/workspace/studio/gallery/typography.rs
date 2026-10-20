//! The type roles (and the label role of band titles) and the text colors.

use super::*;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let tk = |name: &str| env.tk(name);
    let typography = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_title().text_color(tk("text.primary")).child("Title — the top of a page"))
        .child(div().text_heading().text_color(tk("text.primary")).child("Heading — a subsection"))
        .child(div().text_body().text_color(tk("text.primary")).child("Body — content, rows and forms"))
        .child(
            div()
                .text_body()
                .font_weight(FontWeight::MEDIUM)
                .text_color(tk("text.primary"))
                .child("Body 500 — a terminal's name in the sidebar"),
        )
        .child(div().text_label().text_color(tk("text.primary")).child("Label 13 / 600 — a band's title"))
        .child(div().text_caption().text_color(tk("text.primary")).child("Caption — metadata and labels"))
        .child(
            div()
                .text_caption()
                .tabular_nums()
                .text_color(tk("text.primary"))
                .child("Tabular digits — 1:12 · 11:48 · 999+"),
        )
        .child(
            div().text_body().text_color(tk("text.muted")).child("Muted text — secondary labels, subtitles"),
        )
        .child(div().text_body().text_color(tk("text.faint")).child("Faint text — hints, decoration"))
        .child(div().text_body().text_color(tk("text.disabled")).child("Disabled text"))
        .child(div().text_body().text_color(tk("text.link")).underline().child("A link"));
    typography.into_any_element()
}
