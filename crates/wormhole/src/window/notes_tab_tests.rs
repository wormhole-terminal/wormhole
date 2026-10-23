//! The Notes tab in a real window: ⌘E shows the tab before it focuses a note, and a new or clicked note is
//! shown and focused. The harness is `test_harness`.

use gpui_kit::{Focusable as _, TestAppContext};

use crate::features::notes::testing as notes;
use crate::window::test_harness::{drive, isolate_in, start};
use wormhole_core::config::TabId;

#[gpui_kit::test]
fn cmd_e_shows_the_notes_tab_before_focusing_a_note(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "cmd_e_shows_the_notes_tab_before_focusing_a_note") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| {
        notes::add_note(ws, 0, "remember this", false, window, cx);
        ws.show_tab(TabId::Changes, cx);
        notes::set_notes_collapsed(ws, 0, true, cx);
        ws.focus_terminal(window, cx);
        assert!(ws.active_terminal().focus_handle(cx).is_focused(window), "the terminal has the keyboard");
    });

    drive(cx, &window, |ws, window, cx| ws.toggle_focus(window, cx));
    drive(cx, &window, |ws, window, cx| {
        assert!(ws.right_tab == TabId::Notes, "the panel switched to the Notes tab");
        assert!(!notes::notes_collapsed(ws, 0, cx), "and unfolded the section");
        assert_eq!(notes::focused_note(ws, cx), Some(0));
        let focus = notes::note(ws, 0, 0, cx).0.read(cx).focus_handle(cx);
        assert!(focus.is_focused(window), "the note has the keyboard");
    });
}

#[gpui_kit::test]
fn a_new_note_and_a_clicked_note_are_shown_and_focused(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_new_note_and_a_clicked_note_are_shown_and_focused") {
        return;
    }
    let window = start(cx, 1);
    // ⌥⌘N with History showing and the Notes section folded away.
    drive(cx, &window, |ws, _, cx| {
        ws.show_tab(TabId::History, cx);
        notes::set_notes_collapsed(ws, 0, true, cx);
    });
    drive(cx, &window, |ws, window, cx| notes::new_note(ws, 0, Some("first"), window, cx));
    drive(cx, &window, |ws, window, cx| {
        assert!(ws.right_tab == TabId::Notes);
        assert!(!notes::notes_collapsed(ws, 0, cx));
        assert_eq!((notes::note_count(ws, 0, cx), notes::focused_note(ws, cx)), (1, Some(0)));
        assert!(notes::note(ws, 0, 0, cx).0.read(cx).focus_handle(cx).is_focused(window));
    });

    // Clicking the title of a folded note unfolds it and puts the cursor at the end.
    drive(cx, &window, |ws, window, cx| {
        notes::set_note_collapsed(ws, 0, 0, true, cx);
        ws.focus_terminal(window, cx);
        notes::open_note(ws, 0, 0, window, cx);
    });
    drive(cx, &window, |ws, window, cx| {
        let (note, collapsed) = notes::note(ws, 0, 0, cx);
        assert!(!collapsed);
        let state = note.read(cx);
        assert!(state.focus_handle(cx).is_focused(window));
        assert_eq!(state.cursor(), "first".len(), "at the end of the text");
    });
}
