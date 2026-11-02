//! View tests of busy → done as the terminal's own timer settles it, with no tick of the window.

use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_term::{Mark, PtyEvent};

use crate::terminal_view::Status;
use crate::window::reported_tests::{event, now, terminal};
use crate::window::test_harness::{isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn a_command_that_finishes_settles_at_once_without_a_tick(cx: &mut TestAppContext) {
    if isolate("a_command_that_finishes_settles_at_once_without_a_tick") {
        return;
    }
    let window = start(cx, 2);
    let t = terminal(cx, &window, 1);
    event(cx, &t, PtyEvent::Mark(Mark::OutputStart, 0));
    assert_eq!(now(cx, &t).0, Status::Busy);
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(0)), 0));
    cx.run_until_parked();
    assert_eq!(now(cx, &t).0, Status::Done, "nobody was looking");
}

#[gpui_kit::test]
fn a_busy_terminal_settles_when_its_output_has_been_quiet_long_enough(cx: &mut TestAppContext) {
    if isolate("a_busy_terminal_settles_when_its_output_has_been_quiet_long_enough") {
        return;
    }
    let window = start(cx, 2);
    let t = terminal(cx, &window, 1);
    event(cx, &t, PtyEvent::Mark(Mark::OutputStart, 0));
    cx.run_until_parked();
    assert_eq!(now(cx, &t).0, Status::Busy, "output may still come");
    // The deadline is on the real clock (it is `Instant`s), the timer on the test executor's.
    std::thread::sleep(Duration::from_millis(1900));
    cx.executor().advance_clock(Duration::from_millis(1900));
    cx.run_until_parked();
    assert_eq!(now(cx, &t).0, Status::Done);
}
