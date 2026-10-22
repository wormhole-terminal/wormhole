//! Whole-window tests of asking before closing: a terminal, a window, the last window (which
//! quits), and the close button's dot that says a question would come. The harness is
//! `view_tests`'.
//!
//! Tests only: they drive the window through the harness and must not touch the person's data.

use gpui_kit::TestAppContext;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use crate::app_state::app;
use crate::workspace::notes::TaskItem;
use crate::workspace::view_tests::{drive, hook, isolate_in, read, start};
use crate::workspace::window_lifecycle::open_workspace_window;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn closing_a_session_asks_only_while_an_agent_is_running(cx: &mut TestAppContext) {
    if isolate("closing_a_session_asks_only_while_an_agent_is_running") {
        return;
    }
    let window = start(cx, 2);
    drive(cx, &window, |ws, _, _| {
        ws.sessions[0].tasks.push(TaskItem { text: "ship it".into(), done: false });
    });
    // A task not done is not a reason to ask: the first session goes at once.
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.sessions[0].id), (1, 2));
        assert!(!ws.closing_session_prompt);
    });
    // An agent at work: a question first, and the session stays until it is answered.
    hook(cx, &window, 2, "UserPromptSubmit", None);
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.sessions[0].id), (1, 2));
        assert!(ws.closing_session_prompt, "the question is on screen");
    });
}

/// The question is the platform's sheet: Cancel keeps the terminal and frees the next question,
/// the closing button (named "Close") closes it.
#[gpui_kit::test]
fn the_close_question_is_a_native_sheet_with_a_cancel_and_a_close(cx: &mut TestAppContext) {
    if isolate("the_close_question_is_a_native_sheet_with_a_cancel_and_a_close") {
        return;
    }
    let window = start(cx, 2);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    assert!(cx.has_pending_prompt(), "a sheet, not a card in the window");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 2, "cancelled: nothing closes");
        assert!(!ws.closing_session_prompt, "and the next question may be asked");
    });
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    cx.simulate_prompt_answer("Close");
    cx.run_until_parked();
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.sessions[0].id), (1, 2), "the first terminal is gone");
        assert!(!ws.closing_session_prompt);
    });
}

#[gpui_kit::test]
fn cmd_w_asks_like_the_x_does(cx: &mut TestAppContext) {
    if isolate("cmd_w_asks_like_the_x_does") {
        return;
    }
    let window = start(cx, 1);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    drive(cx, &window, |ws, window, cx| ws.close_selected(window, cx));
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.sessions[0].id), (1, 1));
        assert!(ws.closing_session_prompt);
    });
}

#[gpui_kit::test]
fn closing_a_window_asks_only_while_an_agent_or_a_command_runs(cx: &mut TestAppContext) {
    if isolate("closing_a_window_asks_only_while_an_agent_or_a_command_runs") {
        return;
    }
    let first = start(cx, 2);
    // A second window, so closing the first doesn't quit.
    cx.update(|cx| {
        let record =
            WindowRecord { id: 2, sessions: vec![SessionRecord::new(9, None, None)], ..Default::default() };
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    // Notes and tasks alone don't ask: the window may go.
    drive(cx, &first, |ws, _, _| ws.sessions[0].tasks.push(TaskItem { text: "ship it".into(), done: false }));
    let quiet = drive(cx, &first, |ws, window, cx| ws.should_close(window, cx));
    assert!(quiet, "nothing running: it closes without a question");

    // Two windows again (closing the last one would quit instead of asking).
    cx.update(|cx| {
        let record =
            WindowRecord { id: 3, sessions: vec![SessionRecord::new(12, None, None)], ..Default::default() };
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    let second =
        cx.update(|cx| app(cx).windows.iter().find(|w| w.id == 2).cloned()).expect("the second window");
    // An agent at work in the second window: a question first.
    hook(cx, &second, 9, "UserPromptSubmit", None);
    let asked = drive(cx, &second, |ws, window, cx| ws.should_close(window, cx));
    assert!(!asked, "it waits for the answer");
    read(cx, &second, |ws| assert!(ws.closing_prompt, "the question is on screen"));
}

/// Closing the last window quits, and asks first while something runs, as ⌘Q does; a second ⌘Q
/// while the question is up doesn't ask again.
#[gpui_kit::test]
fn closing_the_last_window_asks_before_quitting_while_an_agent_works(cx: &mut TestAppContext) {
    if isolate("closing_the_last_window_asks_before_quitting_while_an_agent_works") {
        return;
    }
    let ws = start(cx, 1);
    let (window_id, tab) = read(cx, &ws, |ws| (ws.window_id, ws.sessions[0].id));
    hook(cx, &ws, tab, "UserPromptSubmit", None);
    let closes = drive(cx, &ws, |ws, window, cx| ws.should_close(window, cx));
    assert!(!closes, "the window stays while the question is up");
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(app(cx).quit_prompt, Some(window_id), "the question is on screen");
        assert!(!app(cx).quitting);
        crate::workspace::request_quit(cx);
        assert_eq!(app(cx).quit_prompt, Some(window_id), "one question at a time");
    });
}

/// The close button's dot is on exactly while closing the window would ask first, as Terminal's is
/// while processes run in a window.
#[gpui_kit::test]
fn the_close_button_shows_a_dot_while_closing_would_ask(cx: &mut TestAppContext) {
    if isolate("the_close_button_shows_a_dot_while_closing_would_ask") {
        return;
    }
    let ws = start(cx, 1);
    let id = read(cx, &ws, |ws| ws.sessions[0].id);
    drive(cx, &ws, |ws, window, cx| ws.sync_close_dot(window, cx));
    read(cx, &ws, |ws| assert!(!ws.close_dot, "a shell at its prompt: no dot"));
    hook(cx, &ws, id, "UserPromptSubmit", None);
    drive(cx, &ws, |ws, window, cx| ws.sync_close_dot(window, cx));
    read(cx, &ws, |ws| assert!(ws.close_dot, "an agent at work: the dot"));
    hook(cx, &ws, id, "Stop", None);
    drive(cx, &ws, |ws, window, cx| ws.sync_close_dot(window, cx));
    read(cx, &ws, |ws| assert!(!ws.close_dot, "the agent is done: the dot goes"));
}
