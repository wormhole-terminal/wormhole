//! The type roles (and the label role of band titles) and the text colors.

use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::style::TextRole;

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
        .child(div().text_title().text_color(tk("text.primary")).child("Title — the top of a page")) // text: sample (the gallery's specimen)
        .child(div().text_heading().text_color(tk("text.primary")).child("Heading — a subsection")) // text: sample (the gallery's specimen)
        .child(div().text_body().text_color(tk("text.primary")).child("Body — content, rows and forms")) // text: sample (the gallery's specimen)
        .child(
            div()
                .text_body()
                .font_weight(FontWeight::MEDIUM)
                .text_color(tk("text.primary"))
                .child("Body 500 — a terminal's name in the sidebar"), // text: sample (the gallery's specimen)
        )
        .child(div().text_label().text_color(tk("text.primary")).child("Label 13 / 600 — a band's title")) // text: sample (the gallery's specimen)
        .child(div().text_caption().text_color(tk("text.primary")).child("Caption — metadata and labels")) // text: sample (the gallery's specimen)
        .child(
            div()
                .text_caption()
                .tabular_nums()
                .text_color(tk("text.primary"))
                .child("Tabular digits — 1:12 · 11:48 · 999+"), // text: sample (the gallery's specimen)
        )
        .child(
            div().text_body().text_color(tk("text.muted")).child("Muted text — secondary labels, subtitles"), // text: sample (the gallery's specimen)
        )
        .child(div().text_body().text_color(tk("text.faint")).child("Faint text — hints, decoration")) // text: sample (the gallery's specimen)
        .child(div().text_body().text_color(tk("text.disabled")).child("Disabled text")) // text: sample (the gallery's specimen)
        .child(div().text_body().text_color(tk("text.link")).underline().child("A link")); // text: sample (the gallery's specimen)
    typography.into_any_element()
}
