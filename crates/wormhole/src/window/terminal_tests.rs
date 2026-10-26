//! The terminal in front in a real window: find in scrollback and answering an agent's question by key.
//! The harness is `test_harness`.

use gpui_kit::TestAppContext;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use crate::terminal_view::Status;
use crate::window::test_harness::{draw, hook, isolate_in, press, read, start, start_with, with_window};

/// ⌘F opens the bar with the keyboard in its field, typing finds every match and picks the
/// newest, ⌘G / ⇧⌘G go round, and Esc closes it and gives the terminal the keyboard back.
#[gpui_kit::test]
fn find_opens_counts_steps_and_closes(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "find_opens_counts_steps_and_closes") {
        return;
    }
    // A terminal restored with text in it: the test can't wait for a shell's output.
    let mut session = SessionRecord::new(1, None, None);
    session.scrollback = Some("one split, two split\nnothing\nthree split\nsplits".into());
    let window = start_with(cx, WindowRecord { id: 1, sessions: vec![session], ..Default::default() });
    draw(cx, &window);

    let find = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| ws.active_terminal()).read_with(cx, |t, _| t.find_summary())
    };
    press(cx, &window, "cmd-f");
    assert_eq!(find(cx), Some("\"\" ".to_string()), "open, nothing typed yet");
    let terminal_focused = |cx: &mut TestAppContext| {
        let terminal = read(cx, &window, |ws| ws.active_terminal());
        with_window(cx, &window, |window, cx| terminal.read(cx).focus_handle.is_focused(window))
    };
    assert!(!terminal_focused(cx), "the field has the keyboard");
    for key in ["s", "p", "l", "i", "t"] {
        press(cx, &window, key);
    }
    assert_eq!(find(cx), Some("\"split\" 4 of 4".to_string()), "the newest match first");
    press(cx, &window, "cmd-g");
    assert_eq!(find(cx), Some("\"split\" 1 of 4".to_string()), "round the end");
    press(cx, &window, "shift-cmd-g");
    press(cx, &window, "shift-cmd-g");
    assert_eq!(find(cx), Some("\"split\" 3 of 4".to_string()));
    draw(cx, &window);
    press(cx, &window, "escape");
    assert_eq!(find(cx), None, "closed");
    assert!(terminal_focused(cx), "the terminal has the keyboard again");
}

#[gpui_kit::test]
fn answering_the_agents_question_by_key_ends_needs_you(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "answering_the_agents_question_by_key_ends_needs_you") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let now = |cx: &mut TestAppContext| {
        let summary = read(cx, &window, |ws| ws.active_terminal()).read_with(cx, |t, _| t.summary());
        (summary.status, summary.agent_message)
    };
    // The key as the terminal reads it, without sending it to the shell (whose echo the test
    // scheduler can't wait for).
    let key = |cx: &mut TestAppContext, key: &str| {
        let terminal = read(cx, &window, |ws| ws.active_terminal());
        cx.update(|cx| terminal.update(cx, |t, _| t.answered(key, false)));
    };
    let asked = (Status::Attention, Some("Approve cargo test?".to_string()));

    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Approve cargo test?"));
    assert_eq!(now(cx), asked);
    // The notification that the same prompt still waits keeps the prompt's own words.
    hook(cx, &window, 1, "Notification:permission_prompt", Some("Claude needs your permission to use Bash"));
    assert_eq!(now(cx), asked);
    // Moving through the choices answers nothing; Return does, and the tool runs.
    key(cx, "down");
    assert_eq!(now(cx), asked);
    key(cx, "enter");
    assert_eq!(now(cx), (Status::Busy, None));

    // Esc turns it down: the agent stops at its prompt, still reporting through its hooks.
    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Approve cargo test?"));
    key(cx, "escape");
    assert_eq!(now(cx).0, Status::Idle);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    assert_eq!(now(cx).0, Status::Busy);
}

/// A program that prints while the test waits (the echo of what was typed into a terminal) reaches the
/// window from the PTY's reader thread; the test scheduler does not report that as a test that is not
/// deterministic (`test_harness::start`, issue #3).
#[gpui_kit::test]
fn a_program_printing_while_the_test_waits_does_not_fail_it(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_program_printing_while_the_test_waits_does_not_fail_it") {
        return;
    }
    let window = start(cx, 1);
    cx.run_until_parked();
    // The terminal's event task is waiting for the first event now: the echo wakes it from the reader.
    let terminal = read(cx, &window, |ws| ws.active_terminal());
    cx.update(|cx| terminal.update(cx, |t, _| t.write_input(b"hello\r".to_vec())));
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !terminal.read_with(cx, |t, _| t.screen_text()).contains("hello") {
        assert!(std::time::Instant::now() < until, "the echo arrived");
        std::thread::sleep(std::time::Duration::from_millis(10));
        cx.run_until_parked();
    }
}
