//! View tests of a terminal whose program reports progress with OSC 9;4: what its row says, that the
//! chrome follows it (the regions' oracle panics in a replayed frame that shows a stale fact), and how
//! it takes turns with an agent's hooks and with time. The reports are fed to the terminal's records as
//! its PTY thread would (`/bin/cat` prints none), at instants the test chooses.

use std::time::{Duration, Instant};

use gpui_kit::{Entity, TestAppContext};
use wormhole_term::program_status::PROGRESS_TIMEOUT;
use wormhole_term::{Progress, ProgressState};

use crate::terminal_view::{Status, TerminalView};
use crate::window::reported_tests::{now, terminal, words};
use crate::window::test_harness::{draw_replaying as draw, hook, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn bar(state: ProgressState, percent: u8) -> Option<Progress> {
    Some(Progress { state, percent })
}

fn send(cx: &mut TestAppContext, terminal: &Entity<TerminalView>, progress: Option<Progress>, at: Instant) {
    cx.update(|cx| terminal.update(cx, |t, cx| t.progress_for_test(progress, at, cx)));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_row_says_how_far_along_a_program_is_and_the_chrome_follows(cx: &mut TestAppContext) {
    if isolate("the_row_says_how_far_along_a_program_is_and_the_chrome_follows") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    for _ in 0..3 {
        draw(cx, &window);
    }
    let at = Instant::now();
    let steps = [
        (bar(ProgressState::Normal, 40), (Status::Busy, words("Working · 40%"))),
        (bar(ProgressState::Normal, 250), (Status::Busy, words("Working · 100%"))),
        (bar(ProgressState::Paused, 40), (Status::Busy, words("Paused · 40%"))),
        (bar(ProgressState::Indeterminate, 0), (Status::Busy, None)),
        (bar(ProgressState::Error, 50), (Status::Busy, words("Error · 50%"))),
        (None, (Status::Idle, None)),
    ];
    for (progress, expected) in steps {
        send(cx, &t, progress, at);
        assert_eq!(now(cx, &t), expected, "{progress:?}");
        // A replayed frame: the regions that show the status are checked against it.
        draw(cx, &window);
    }
}

#[gpui_kit::test]
fn an_error_bar_is_not_a_failure_and_looking_does_not_end_it(cx: &mut TestAppContext) {
    if isolate("an_error_bar_is_not_a_failure_and_looking_does_not_end_it") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    send(cx, &t, bar(ProgressState::Error, 50), Instant::now());
    assert_eq!(now(cx, &t).0, Status::Busy, "never Failed, which a notification would announce");
    let dropped = cx.update(|cx| t.update(cx, |t, _| t.acknowledge()));
    assert!(!dropped, "the program still holds it");
    assert_eq!(now(cx, &t), (Status::Busy, words("Error · 50%")));
}

#[gpui_kit::test]
fn an_agents_hook_keeps_the_status_a_later_progress_bar_would_take(cx: &mut TestAppContext) {
    if isolate("an_agents_hook_keeps_the_status_a_later_progress_bar_would_take") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    hook(cx, &window, 1, "Notification", Some("Allow Bash?"));
    assert_eq!(now(cx, &t), (Status::Attention, words("Allow Bash?")));
    let at = Instant::now();
    send(cx, &t, bar(ProgressState::Normal, 10), at);
    assert_eq!(now(cx, &t), (Status::Attention, words("Allow Bash?")), "a bar does not answer the question");
    send(cx, &t, None, at);
    assert_eq!(now(cx, &t), (Status::Attention, words("Allow Bash?")), "nor does its removal");
}

#[gpui_kit::test]
fn a_bar_nobody_renews_ends_by_its_own_timer_and_the_window_is_told(cx: &mut TestAppContext) {
    if isolate("a_bar_nobody_renews_ends_by_its_own_timer_and_the_window_is_told") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    let armed = |cx: &mut TestAppContext| t.read_with(cx, |t, _| t.progress_timer_armed_for_test());
    let fire = |cx: &mut TestAppContext, at: Instant| {
        cx.update(|cx| t.update(cx, |t, cx| t.fire_progress_timer(at, cx)));
        cx.run_until_parked();
    };
    assert!(!armed(cx));
    let start_at = Instant::now();
    send(cx, &t, bar(ProgressState::Normal, 40), start_at);
    draw(cx, &window);
    assert!(armed(cx), "a report arms the timer");
    // Early: nothing to drop, and it waits again.
    fire(cx, start_at + PROGRESS_TIMEOUT - Duration::from_millis(1));
    assert_eq!(now(cx, &t).0, Status::Busy);
    assert!(armed(cx));
    // A renewal moves the deadline without a word to the terminal; the timer finds it when it fires.
    let renewed = start_at + Duration::from_secs(10);
    send(cx, &t, bar(ProgressState::Normal, 40), renewed);
    fire(cx, start_at + PROGRESS_TIMEOUT);
    assert_eq!(now(cx, &t).0, Status::Busy, "renewed, so it stands");
    fire(cx, renewed + PROGRESS_TIMEOUT);
    assert_eq!(now(cx, &t), (Status::Idle, None));
    assert!(!armed(cx), "nothing left to wait for");
    // The summary was announced, so the chrome is redrawn from it: the oracle checks the replay.
    draw(cx, &window);
    // A removal cancels a timer that stands.
    send(cx, &t, bar(ProgressState::Normal, 1), Instant::now());
    assert!(armed(cx));
    send(cx, &t, None, Instant::now());
    assert!(!armed(cx));
}
