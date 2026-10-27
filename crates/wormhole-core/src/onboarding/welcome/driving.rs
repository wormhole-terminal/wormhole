//! The driver on a script of ticks: what it asks of the app and when, how the start animation's wait
//! goes, the end, a refused request, and the example band's going up and down with the sidebar step.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;

use super::driver::{Answer, Driver};
use super::fixtures;
use super::flow::Step;
use super::model::Welcome;
use super::render::Size;
use crate::ipc::control::WelcomeOp;
use crate::onboarding::intro::Handover;

fn set_notify(
    needs_you: Option<bool>,
    agent_done: Option<bool>,
    sound: Option<crate::config::NotifySound>,
    allow: bool,
) -> WelcomeOp {
    WelcomeOp::SetNotify { needs_you, agent_done, sound, allow }
}

const SIZE: Size = Size { cols: 80, rows: 24 };

fn state_answer(wait_ms: u64) -> Answer {
    let mut state = fixtures::state(Handover::Cursor);
    state.wait_ms = wait_ms;
    Answer { op: "state", result: Ok(Some(serde_json::to_value(state).expect("serializes"))) }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn text(draw: &[u8]) -> String {
    String::from_utf8_lossy(draw).into_owned()
}

/// A driver that has its state and is on the first page, the app asked at 0 ms and answered at 10.
fn started() -> Driver {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer(0)]);
    driver
}

/// The ops in a tick's asks that are not a poll.
fn but_polls(ask: &[WelcomeOp]) -> Vec<&WelcomeOp> {
    ask.iter().filter(|op| **op != WelcomeOp::State).collect()
}

#[test]
fn it_asks_for_the_state_first_and_draws_nothing_until_the_wait_is_over() {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    let first = driver.tick(ms(0), SIZE, &[], vec![]);
    assert_eq!(first.ask, [WelcomeOp::State]);
    assert!(first.draw.is_empty());
    // While the request is out it isn't asked again.
    assert!(driver.tick(ms(300), SIZE, &[], vec![]).ask.is_empty());
    // The answer: wait 5.2 s from when it arrives. Nothing is drawn meanwhile.
    let answered = driver.tick(ms(400), SIZE, &[], vec![state_answer(5200)]);
    assert!(answered.draw.is_empty() && !driver.started(ms(400)));
    assert!(driver.tick(ms(5500), SIZE, &[], vec![]).draw.is_empty(), "not yet");
    let started = driver.tick(ms(5600), SIZE, &[], vec![]);
    assert!(driver.started(ms(5600)));
    assert!(text(&started.draw).contains('>'), "the mark comes up the moment the text starts");
}

#[test]
fn a_key_ends_the_wait_and_is_not_acted_on() {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(100), SIZE, &[], vec![state_answer(5000)]);
    let tick = driver.tick(ms(200), SIZE, b"\r", vec![]);
    assert!(driver.started(ms(200)));
    assert!(tick.end.is_none(), "Return only said 'now'");
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Welcome));
}

#[test]
fn the_app_may_shorten_the_wait_when_the_animation_is_skipped() {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(100), SIZE, &[], vec![state_answer(5000)]);
    // The next poll says no wait: the text starts at once, never later than first said.
    driver.tick(ms(400), SIZE, &[], vec![]);
    driver.tick(ms(600), SIZE, &[], vec![state_answer(0)]);
    assert!(driver.started(ms(600)));
    driver.tick(ms(900), SIZE, &[], vec![]);
    driver.tick(ms(1000), SIZE, &[], vec![state_answer(9000)]);
    assert!(driver.started(ms(1000)), "a longer wait later doesn't put it off again");
}

#[test]
fn without_a_link_it_is_a_tour_that_asks_for_nothing() {
    let mut driver = Driver::new(false, PathBuf::from("/nonexistent"));
    let tick = driver.tick(ms(0), SIZE, &[], vec![]);
    assert!(tick.ask.is_empty() && driver.started(ms(0)));
    assert!(text(&tick.draw).contains("Read-only tour") || driver.model().is_some_and(|m| !m.live()));
    // Return all the way through: it ends without a Finish to send, and no example band was asked for.
    let mut end = None;
    for n in 1..=7 {
        let tick = driver.tick(ms(n * 50), SIZE, b"\r", vec![]);
        assert!(tick.ask.is_empty(), "{tick:?}");
        end = tick.end.or(end);
    }
    assert_eq!(end.map(|e| e.skipped), Some(false));
}

#[test]
fn an_app_that_cannot_be_reached_turns_the_welcome_into_a_tour() {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    let failed = Answer { op: "state", result: Err("Wormhole isn't running".to_string()) };
    let tick = driver.tick(ms(100), SIZE, &[], vec![failed]);
    assert!(tick.ask.is_empty(), "no asking again");
    assert!(driver.started(ms(100)));
    assert!(driver.model().is_some_and(|model| !model.live()));
}

#[test]
fn the_end_sends_the_finish_and_then_stops() {
    let mut driver = started();
    let mut last = None;
    for n in 0..6 {
        let tick = driver.tick(ms(100 + n * 50), SIZE, b"\r", vec![]);
        if tick.end.is_some() {
            last = Some(tick);
        }
    }
    let tick = last.expect("it ended");
    assert_eq!(tick.ask.last(), Some(&WelcomeOp::Finish { skipped: false }), "the Finish goes last");
    assert!(driver.tick(ms(500), SIZE, b"\r", vec![]).ask.is_empty(), "nothing once it is over");
}

#[test]
fn esc_skips_after_a_pause_and_an_arrow_is_not_an_esc() {
    let mut driver = started();
    driver.tick(ms(100), SIZE, b"\x1b[B", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Welcome), "Down is not Esc");
    driver.tick(ms(150), SIZE, b"\x1b", vec![]);
    assert!(driver.tick(ms(170), SIZE, &[], vec![]).end.is_none(), "it waits to see if more comes");
    let tick = driver.tick(ms(220), SIZE, &[], vec![]);
    assert_eq!(tick.end.map(|e| e.skipped), Some(true));
    assert_eq!(tick.ask, [WelcomeOp::Finish { skipped: true }]);
}

#[test]
fn state_is_polled_every_so_often() {
    let mut driver = started();
    // Not again until a quarter of a second has gone by.
    assert!(driver.tick(ms(100), SIZE, &[], vec![]).ask.is_empty());
    assert_eq!(driver.tick(ms(400), SIZE, &[], vec![]).ask, [WelcomeOp::State]);
    driver.tick(ms(420), SIZE, &[], vec![state_answer(0)]);
    assert!(driver.tick(ms(500), SIZE, &[], vec![]).ask.is_empty());
    assert_eq!(driver.tick(ms(700), SIZE, &[], vec![]).ask, [WelcomeOp::State]);
}

#[test]
fn a_refused_request_is_shown_until_one_works() {
    let mut driver = started();
    let refused = Answer { op: "install_hooks", result: Err("too many requests".to_string()) };
    driver.tick(ms(100), SIZE, &[], vec![refused]);
    assert_eq!(driver.model().and_then(Welcome::problem), Some("too many requests"));
    let fine = Answer { op: "install_hooks", result: Ok(Some(json!({}))) };
    driver.tick(ms(200), SIZE, &[], vec![fine]);
    assert_eq!(driver.model().and_then(Welcome::problem), None);
}

#[test]
fn a_click_on_a_row_presses_it() {
    let mut driver = started();
    // The welcome page's "Get started" is at row 8; a click there is Return.
    driver.tick(ms(100), SIZE, b"?", vec![]);
    driver.tick(ms(150), SIZE, &[], vec![]);
    driver.tick(ms(200), SIZE, b"\x1b[<0;6;9M", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Look));
    // A click on nothing does nothing.
    driver.tick(ms(250), SIZE, b"\x1b[<0;70;20M", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Look));
}

/// Entering the sidebar step asks for the example band; leaving it, by any way, takes it away.
#[test]
fn the_example_band_is_asked_for_on_entering_the_sidebar_step_and_dropped_on_leaving_it() {
    let mut driver = started();
    let demos = |driver: &mut Driver, at: u64, input: &[u8]| -> Vec<WelcomeOp> {
        but_polls(&driver.tick(ms(at), SIZE, input, vec![]).ask).into_iter().cloned().collect()
    };
    assert!(demos(&mut driver, 100, b"\r").is_empty(), "the look step");
    assert!(demos(&mut driver, 110, b"\r").is_empty(), "the step for how much is shown");
    assert!(demos(&mut driver, 120, b"\r").is_empty(), "the notifications step");
    assert_eq!(demos(&mut driver, 140, b"\r"), [WelcomeOp::Demo { show: true }], "the sidebar step");
    assert!(demos(&mut driver, 160, &[]).is_empty(), "once, not every frame");
    assert_eq!(demos(&mut driver, 180, b"\x1b[D"), [WelcomeOp::Demo { show: false }], "back");
    assert_eq!(demos(&mut driver, 200, b"\x1b[C"), [WelcomeOp::Demo { show: true }], "forward again");
    assert_eq!(demos(&mut driver, 220, b"\r"), [WelcomeOp::Demo { show: false }], "on to the last step");
    // Back in, and out by Esc: the Finish is what takes it away, in the same breath.
    assert_eq!(demos(&mut driver, 240, b"\x1b[D"), [WelcomeOp::Demo { show: true }]);
    driver.tick(ms(260), SIZE, b"\x1b", vec![]);
    let end = driver.tick(ms(320), SIZE, &[], vec![]);
    assert_eq!(but_polls(&end.ask), [&WelcomeOp::Finish { skipped: true }]);
}

#[test]
fn a_refused_example_band_is_asked_for_again_but_not_every_frame() {
    let mut driver = started();
    for (at, input) in [(100, &b"\r"[..]), (110, b"\r"), (120, b"\r"), (140, b"\r")] {
        driver.tick(ms(at), SIZE, input, vec![]);
    }
    let refused = Answer { op: "demo", result: Err("too many requests".to_string()) };
    driver.tick(ms(160), SIZE, &[], vec![refused]);
    assert!(but_polls(&driver.tick(ms(200), SIZE, &[], vec![]).ask).is_empty(), "too soon to try again");
    let again = driver.tick(ms(420), SIZE, &[], vec![]);
    assert_eq!(but_polls(&again.ask), [&WelcomeOp::Demo { show: true }]);
    // It took: nothing more.
    let fine = Answer { op: "demo", result: Ok(Some(json!({}))) };
    driver.tick(ms(440), SIZE, &[], vec![fine]);
    assert!(but_polls(&driver.tick(ms(900), SIZE, &[], vec![]).ask).is_empty());
}

#[test]
fn a_state_on_its_way_when_a_choice_is_made_does_not_undo_it() {
    let mut driver = started();
    driver.tick(ms(100), SIZE, b"\r", vec![]);
    driver.tick(ms(110), SIZE, b"\r", vec![]);
    driver.tick(ms(120), SIZE, b"\r", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Notify));
    // The poll is due, and in the same breath the person turns "Needs input" off (four Ups, then Return).
    let tick = driver.tick(ms(400), SIZE, b"\x1b[A\x1b[A\x1b[A\x1b[A\r", vec![]);
    assert_eq!(
        tick.ask,
        [WelcomeOp::State, set_notify(Some(false), None, None, false)],
        "the state is asked first, the choice after it"
    );
    // The state that answers is the old one, which still says on.
    driver.tick(ms(420), SIZE, &[], vec![state_answer(0)]);
    assert_eq!(driver.model().and_then(|m| m.notify().map(|n| n.needs_you)), Some(false));
    // The next one was asked after the choice and is believed.
    driver.tick(ms(700), SIZE, &[], vec![]);
    driver.tick(ms(720), SIZE, &[], vec![state_answer(0)]);
    assert_eq!(driver.model().and_then(|m| m.notify().map(|n| n.needs_you)), Some(true));
}
