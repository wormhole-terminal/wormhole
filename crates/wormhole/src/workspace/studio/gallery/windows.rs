//! The app's own small windows and sheets, drawn small: the in-app dialog (here the Studio's
//! close question), the first-run sheet, About and Licenses. Each is the real
//! window's parts on the live tokens; the windows themselves need their own state.

use super::*;

/// The About window's icon, drawn small.
const ABOUT_ICON: f32 = 48.0;
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

pub(super) fn render(_studio: &StudioWindow, env: &Env, _cx: &mut Context<StudioWindow>) -> AnyElement {
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
                    .child("Save \u{201c}Dusk\u{201d} before closing?"),
            )
            .child(
                div()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("2 changes. If you don\u{2019}t save, the app goes back to Tokyo Night."),
            )
            .child(
                div()
                    .pt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("g-dialog-discard").ghost().small().label("Don\u{2019}t save"))
                    .child(Button::new("g-dialog-cancel").small().label("Cancel"))
                    .child(Button::new("g-dialog-save").primary().small().label("Save")),
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
                        .child("Step 1 of 3")
                        .child(
                            style::link_button("g-sheet-skip", ui, "Skip setup", false).text_color(ui.muted),
                        ),
                )
                .child(div().text_title().font_weight(FontWeight::BOLD).child("Where do you work?"))
                .child(
                    div()
                        .text_body()
                        .text_color(ui.muted)
                        .child("Pick a folder to start in; every terminal remembers its own."),
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
                    Button::new("g-sheet-back").ghost().small().icon(IconName::ChevronLeft).label("Back"),
                ))
                .child(dots)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .justify_end()
                        .child(Button::new("g-sheet-next").primary().small().label("Continue")),
                ),
        );

    // About: the icon, the name and version, and the way to the licenses.
    let about = surface(div())
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .p_4()
        .child(img(crate::icons::app_icon_for(env.tokens.appearance)).size(px(ABOUT_ICON)).flex_none())
        .child(div().text_heading().font_weight(FontWeight::BOLD).child("Wormhole"))
        .child(div().text_caption().text_color(ui.muted).child("Version 0.1.0"))
        .child(
            div()
                .pt_2()
                .flex()
                .gap_2()
                .child(Button::new("g-about-updates").small().label("Check for Updates…"))
                .child(Button::new("g-about-licenses").small().label("Licenses…")),
        );

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
    let licenses =
        surface(div())
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
                    .child(package("g-lic-a", "alacritty_terminal", false))
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
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("anyhow 1.0 · MIT OR Apache-2.0"))
                    .child(div().text_color(ui.muted).child(
                        "Permission is hereby granted, free of charge, to any person obtaining a copy…",
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
