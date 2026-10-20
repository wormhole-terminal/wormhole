//! Find in scrollback: the bar, and matches in terminal text, the current one solid.

use std::rc::Rc;

use wormhole_core::terminal::find::FindOptions;

use super::*;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let term = &env.tokens.terminal;
    let found = |text: &'static str, current: bool| {
        let (bg, fg) = if current {
            (env.tk("find.match.current"), env.tk("find.match.current.text"))
        } else {
            (env.tk("find.match"), hsla(term.foreground))
        };
        div().bg(bg).text_color(fg).when(current, |d| d.font_weight(FontWeight::BOLD)).child(text)
    };
    let sample = div()
        .p_3()
        .rounded(ui.radius_lg)
        .bg(hsla(term.background))
        .text_color(hsla(term.foreground))
        .font_family(SharedString::from(settings(cx).font_family.clone()))
        .text_body()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div().flex().child("test git::").child(found("split", false)).child("::tests::pairs ... FAILED"),
        )
        .child(div().flex().child("panicked at src/git/").child(found("split", true)).child(".rs:212:9"));
    // Pressed toggles show the primary's tint, as the panel's do.
    let options = FindOptions { match_case: true, ..FindOptions::default() };
    let bar = style::find_bar(
        "g-find",
        ui,
        style::FindBarParts {
            field: div().text_body().text_color(ui.text).child("split").into_any_element(),
            focused: true,
            count: "2 of 11".into(),
            invalid: false,
            options,
            on_option: Rc::new(|_, _, _| {}),
            on_step: Rc::new(|_, _, _| {}),
            on_close: Rc::new(|_, _, _| {}),
        },
    );
    let invalid = style::find_bar(
        "g-find-invalid",
        ui,
        style::FindBarParts {
            field: div().text_body().text_color(ui.text).child("error[").into_any_element(),
            focused: false,
            count: "Not a valid pattern".into(),
            invalid: true,
            options: FindOptions { regex: true, ..FindOptions::default() },
            on_option: Rc::new(|_, _, _| {}),
            on_step: Rc::new(|_, _, _| {}),
            on_close: Rc::new(|_, _, _| {}),
        },
    );
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(env.mark("find", sample))
        .child(env.mark("find", bar))
        .child(invalid)
        .into_any_element()
}
