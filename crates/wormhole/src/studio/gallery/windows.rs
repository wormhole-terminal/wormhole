//! The app's own small windows and sheets, drawn small: the in-app dialog (here the Studio's
//! close question), the first-run sheet, About and Licenses. Each is the real
//! window's parts on the live tokens; the windows themselves need their own state.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::about;
use crate::style::{self, Edges, TextRole};

/// The About window's width: its content is drawn on a card that wide.
const ABOUT_W: f32 = 340.0;
/// The Licenses window's package list, drawn small.
const LICENSE_LIST_W: f32 = 120.0;
/// A scrollbar's thumb as the lists draw it.
const THUMB_W: f32 = 4.0;
const THUMB_H: f32 = 40.0;
/// The first-run sheet's footer, drawn small.
const SHEET_FOOTER_H: f32 = 48.0;
/// The first-run sheet's step dots: the current one a bar.
const DOT: f32 = 4.0;
const DOT_BAR: f32 = 16.0;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let surface = |d: Div| d.rounded(ui.radius_lg).outlined(ui, ui.border_strong).bg(ui.elevated);

    // The dialog on its scrim, as the close question puts it up.
    let dialog = div().p_4().rounded(ui.radius_lg).bg(env.tk("surface.overlay")).child(
        surface(div())
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(
                div()
                    .text_body()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Save \u{201c}Dusk\u{201d} before closing?"), // text: sample (the gallery's specimen)
            )
            .child(
                div()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("2 changes. If you don\u{2019}t save, the app goes back to Tokyo Night."), // text: sample (the gallery's specimen)
            )
            .child(
                div()
                    .pt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("g-dialog-discard").ghost().small().label("Don\u{2019}t save")) // text: sample (the gallery's specimen)
                    .child(Button::new("g-dialog-cancel").small().label("Cancel")) // text: sample (the gallery's specimen)
                    .child(Button::new("g-dialog-save").primary().small().label("Save")), // text: sample (the gallery's specimen)
            ),
    );

    // The first-run sheet's card: where it is, the step, and its footer.
    let dots = div().flex().items_center().gap_1().children((1..=3).map(|n| {
        let current = n == 1;
        div().h(px(DOT)).w(px(if current { DOT_BAR } else { DOT })).rounded_full().bg(if current {
            ui.primary
        } else {
            ui.faint
        })
    }));
    let sheet = surface(div())
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("Step 1 of 3") // text: sample (the gallery's specimen)
                        .child(
                            style::link_button("g-sheet-skip", ui, "Skip setup", false).text_color(ui.muted), // text: sample (the gallery's specimen)
                        ),
                )
                .child(div().text_title().font_weight(FontWeight::BOLD).child("Where do you work?")) // text: sample (the gallery's specimen)
                .child(
                    div()
                        .text_body()
                        .text_color(ui.muted)
                        .child("Pick a folder to start in; every terminal remembers its own."), // text: sample (the gallery's specimen)
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .h(px(SHEET_FOOTER_H))
                .px_4()
                .rule_top(ui)
                .child(div().flex_1().child(
                    Button::new("g-sheet-back").ghost().small().icon(IconName::ChevronLeft).label("Back"), // text: sample (the gallery's specimen)
                ))
                .child(dots)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .justify_end()
                        .child(Button::new("g-sheet-next").primary().small().label("Continue")), // text: sample (the gallery's specimen)
                ),
        );

    // About, as the system's panel shows it: the icon, the name, the version and the copyright,
    // nothing to press (updates are in the app menu, the licences in Help).
    let about = surface(div())
        .w_full()
        .max_w(px(ABOUT_W))
        .flex()
        .flex_col()
        .items_center()
        .px_4()
        .py_4()
        .child(img(crate::icons::app_icon_for(env.tokens.appearance)).size(px(about::ICON)).flex_none())
        .child(div().mt_2().text_heading().font_weight(FontWeight::BOLD).child("Wormhole")) // text: sample (the gallery's specimen)
        .child(div().mt_1().text_caption().text_color(ui.muted).child(about::version_line()))
        .child(div().mt_1().text_caption().text_color(ui.muted).text_center().child(about::copyright()));

    // Licenses: the packages (one selected, the list scrolled) and the selected one's text.
    let package = |id: &'static str, name: &'static str, selected: bool| {
        style::flush_row(id, ui, selected).px_2().text_caption().child(name)
    };
    let thumb = div()
        .absolute()
        .top_1()
        .right_1()
        .w(px(THUMB_W))
        .h(px(THUMB_H))
        .rounded_full()
        .bg(env.tk("scrollbar.thumb"));
    let licenses = surface(div())
        .flex()
        .overflow_hidden()
        .h(px(4.0 * style::ROW_H))
        .child(
            div()
                .relative()
                .w(px(LICENSE_LIST_W))
                .flex_none()
                .flex()
                .flex_col()
                .rule_right(ui)
                .child(package("g-lic-a", "ahash", false))
                .child(package("g-lic-b", "anyhow", true))
                .child(package("g-lic-c", "gpui-kit", false))
                .child(package("g-lic-d", "rusqlite", false))
                .child(env.mark("scrollbar", thumb)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .p_2()
                .flex()
                .flex_col()
                .gap_1()
                .text_caption()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("anyhow 1.0 · MIT OR Apache-2.0")) // text: sample (the gallery's specimen)
                .child(div().text_color(ui.muted).child(
                    "Permission is hereby granted, free of charge, to any person obtaining a copy…", // text: sample (the gallery's specimen)
                )),
        );

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(env.mark("dialog", dialog))
        .child(sheet)
        .child(about)
        .child(licenses)
        .into_any_element()
}
