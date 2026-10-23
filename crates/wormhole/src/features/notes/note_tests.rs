//! View tests of the Notes tab's editors: the hint an empty note shows (see `view_tests` for how
//! each test runs in a child process with scratch data).
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{TestAppContext, TextRun, font, px, rems, size};
use wormhole_core::view::layout::{PANEL_MIN, SIDEBAR_DEFAULT, TERMINAL_MIN};

use crate::app_state::OpenWindow;
use crate::features::notes::testing as notes;
use crate::window::view_tests::{draw, drive, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// The window at which the panel has its narrowest width, next to the sidebar at its default width
/// and the least terminal.
const NARROWEST_PANEL_WINDOW: f32 = SIDEBAR_DEFAULT + TERMINAL_MIN + PANEL_MIN;

fn resize(cx: &mut TestAppContext, window: &OpenWindow, width: f32) {
    cx.simulate_window_resize(window.handle, size(px(width), px(900.0)));
    draw(cx, window);
}

/// The first note's hint as its editor draws it: one entry per line, each with its width.
fn hint(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<(String, f32)> {
    drive(cx, window, |ws, window, cx| {
        let hint = notes::note(ws, 0, 0, cx).0.read(cx).presentation().placeholder().to_string();
        let family = cx.theme().font_family.clone();
        let size = rems(0.875).to_pixels(window.rem_size());
        hint.split('\n')
            .map(|line| {
                let run = TextRun { len: line.len(), font: font(family.clone()), ..Default::default() };
                let shaped = window.text_system().layout_line(line, size, &[run], None);
                (line.to_string(), f32::from(shaped.width))
            })
            .collect()
    })
}

/// The editor draws a hint one line per newline and never wraps it, so a line longer than the
/// panel's text column was cut at its edge ("⌘↵ sends the paragraph under" and no more). The hint
/// is broken into lines that fit, at the narrowest panel too.
#[gpui_kit::test]
fn an_empty_notes_hint_fits_the_narrowest_panel(cx: &mut TestAppContext) {
    if isolate("an_empty_notes_hint_fits_the_narrowest_panel") {
        return;
    }
    let window = start(cx, 1);
    resize(cx, &window, NARROWEST_PANEL_WINDOW);
    drive(cx, &window, |ws, window, cx| {
        assert_eq!(ws.columns().panel, PANEL_MIN);
        notes::new_note(ws, 0, None, window, cx);
    });
    draw(cx, &window);
    // Where the editor's text starts, the panel's inset, and the margin the editor keeps.
    let room = PANEL_MIN - 40.0 - 16.0 - 10.0;
    let lines = hint(cx, &window);
    for (line, width) in &lines {
        assert!(*width <= room, "{line:?} is {width} wide, the text column {room}");
    }
    assert!(lines.len() > 2, "the long line is broken: {lines:?}");
}

/// A note made when the panel was narrow, and written in once it is wider (its text cleared, say),
/// shows its hint for the width it has by then: the same words in fewer lines.
#[gpui_kit::test]
fn a_notes_hint_follows_the_panel_when_the_note_is_taken_up(cx: &mut TestAppContext) {
    if isolate("a_notes_hint_follows_the_panel_when_the_note_is_taken_up") {
        return;
    }
    let window = start(cx, 1);
    resize(cx, &window, NARROWEST_PANEL_WINDOW);
    drive(cx, &window, |ws, window, cx| {
        notes::add_note(ws, 0, "kept", false, window, cx);
    });
    let narrow = hint(cx, &window);

    resize(cx, &window, 1600.0);
    drive(cx, &window, |ws, window, cx| notes::show_note(ws, 0, 0, false, window, cx));
    // The editor says it has the keyboard when a frame finds it so.
    draw(cx, &window);
    cx.run_until_parked();
    let wide = hint(cx, &window);
    assert!(wide.len() < narrow.len(), "{wide:?} against {narrow:?}");
    let words = |lines: &[(String, f32)]| {
        lines.iter().flat_map(|(line, _)| line.split_whitespace().map(str::to_string)).collect::<Vec<_>>()
    };
    assert_eq!(words(&wide), words(&narrow), "nothing is lost in the breaking");
}
