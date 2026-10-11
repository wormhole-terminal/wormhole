//! Walks the welcome as a person does: keys into the model, and ticks into the driver. What is
//! asked of the app, and when, is the point: nothing is installed or trusted without its own press.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;

use super::driver::{Answer, Driver};
use super::fixtures;
use super::keys::Key;
use super::model::{Effect, End, Welcome};
use super::render::Size;
use crate::agents::AgentId;
use crate::intro::Handover;
use crate::ipc::control::WelcomeOp;
use crate::onboarding::Step;

const NOW: Duration = Duration::ZERO;

fn asks(effects: &[Effect]) -> Vec<&WelcomeOp> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Ask(op) => Some(op),
            Effect::End(_) => None,
        })
        .collect()
}

fn installs_or_trusts(op: &WelcomeOp) -> bool {
    matches!(op, WelcomeOp::InstallHooks { .. } | WelcomeOp::Trust { .. })
}

// ---------------------------------------------------------------------------------- the model

/// Moving through the steps with every key but Return (and the click that presses it) never asks for
/// an Install or a Trust, whatever the order. This is the property the whole page rests on.
#[test]
fn no_way_of_moving_about_installs_or_trusts() {
    let moves = [
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
        Key::Tab,
        Key::BackTab,
        Key::Esc,
        Key::Backspace,
        Key::Char('j'),
        Key::Char('k'),
        Key::Char('p'),
        Key::Char('l'),
        Key::Char('+'),
        Key::Char('-'),
    ];
    fn walk(w: &Welcome, depth: usize, moves: &[Key], seen: &mut usize) {
        if depth == 0 {
            return;
        }
        for key in moves {
            let mut next = w.clone();
            let effects = next.key(*key, NOW);
            *seen += 1;
            for op in asks(&effects) {
                assert!(!installs_or_trusts(op), "{key:?} at {:?} asked for {op:?}", w.step());
            }
            if next.ended().is_none() {
                walk(&next, depth - 1, moves, seen);
            }
        }
    }
    let mut seen = 0;
    for with_trust in [false, true] {
        for found in [true, false] {
            let mut state = fixtures::state(Handover::Cursor);
            state.trust = with_trust.then(fixtures::trust);
            if !found {
                state.agents = Some(Default::default());
            }
            walk(&Welcome::new(state, true, true), 5, &moves, &mut seen);
        }
    }
    assert!(seen > 20_000, "the walk covered {seen} moves");
}

#[test]
fn returning_through_every_step_installs_and_trusts_nothing_and_ends() {
    let mut state = fixtures::state(Handover::Cursor);
    state.trust = Some(fixtures::trust());
    let mut w = Welcome::new(state, true, true);
    let mut all = Vec::new();
    for _ in 0..4 {
        all.extend(w.key(Key::Enter, NOW));
    }
    // Welcome, folder (the first place is picked), look, agents (the cursor starts on Start).
    assert!(asks(&all).is_empty(), "{all:?}");
    assert_eq!(
        all.last(),
        Some(&Effect::End(End {
            folder: Some(PathBuf::from("/Users/me/Projects/terminal")),
            skipped: false
        }))
    );
}

#[test]
fn an_install_comes_only_from_return_on_that_agents_own_row() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..3 {
        w.key(Key::Enter, NOW);
    }
    assert_eq!(w.step(), Step::Agents);
    // Up from Start: Aider (nothing to install), Codex (installed), Claude (Install).
    w.key(Key::Up, NOW);
    assert!(asks(&w.key(Key::Enter, NOW)).is_empty(), "Aider has no hooks");
    w.key(Key::Up, NOW);
    assert!(asks(&w.key(Key::Enter, NOW)).is_empty(), "Codex's are in");
    w.key(Key::Up, NOW);
    let effects = w.key(Key::Enter, NOW);
    assert_eq!(asks(&effects), [&WelcomeOp::InstallHooks { agent: AgentId::Claude }]);
    assert_eq!(w.step(), Step::Agents, "an install doesn't move on");
    // Space does the same on the same row; no other row is installed by it.
    let again = w.key(Key::Char(' '), NOW);
    assert_eq!(asks(&again), [&WelcomeOp::InstallHooks { agent: AgentId::Claude }]);
}

#[test]
fn a_click_on_an_agents_row_installs_that_one_only() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..3 {
        w.key(Key::Enter, NOW);
    }
    assert!(asks(&w.click_row(1, NOW)).is_empty(), "Codex's hooks are in");
    let effects = w.click_row(0, NOW);
    assert_eq!(asks(&effects), [&WelcomeOp::InstallHooks { agent: AgentId::Claude }]);
}

#[test]
fn a_click_on_a_theme_chooses_it_and_stays_on_the_page() {
    let mut w = fixtures::welcome(Handover::Cursor);
    w.key(Key::Enter, NOW);
    w.key(Key::Enter, NOW);
    let effects = w.click_row(3, NOW);
    assert_eq!(w.step(), Step::Look, "a click doesn't go on");
    assert_eq!(
        asks(&effects),
        [&WelcomeOp::SetLook {
            theme: Some("Rose Pine".to_string()),
            primary: None,
            ligatures: None,
            font_step: None
        }]
    );
    assert_eq!(w.cursor(), 3);
}

#[test]
fn trust_asks_for_exactly_the_folder_the_page_names() {
    let mut state = fixtures::state(Handover::Cursor);
    state.trust = Some(fixtures::trust());
    let mut w = Welcome::new(state, true, true);
    for _ in 0..3 {
        w.key(Key::Enter, NOW);
    }
    w.key(Key::Up, NOW);
    let effects = w.key(Key::Enter, NOW);
    assert_eq!(asks(&effects), [&WelcomeOp::Trust { folder: "/Users/me/Projects/terminal".to_string() }]);
}

#[test]
fn a_read_only_tour_asks_for_nothing_whatever_is_pressed() {
    let mut state = fixtures::state(Handover::Cursor);
    state.trust = Some(fixtures::trust());
    let mut w = Welcome::new(state, false, true);
    let mut all = Vec::new();
    for key in [Key::Enter, Key::Enter, Key::Char('p'), Key::Char('l'), Key::Char('+'), Key::Down, Key::Enter]
    {
        all.extend(w.key(key, NOW));
    }
    for _ in 0..3 {
        w.key(Key::Up, NOW);
        all.extend(w.key(Key::Enter, NOW));
    }
    assert!(asks(&all).is_empty(), "{all:?}");
}

#[test]
fn the_look_is_applied_as_the_cursor_moves_and_by_its_keys() {
    let mut w = fixtures::welcome(Handover::Cursor);
    w.key(Key::Enter, NOW);
    w.key(Key::Enter, NOW);
    assert_eq!(w.step(), Step::Look);
    let look = |theme: Option<&str>, primary: Option<&str>, ligatures: Option<bool>, step: Option<i8>| {
        WelcomeOp::SetLook {
            theme: theme.map(str::to_string),
            primary: primary.map(str::to_string),
            ligatures,
            font_step: step,
        }
    };
    assert_eq!(asks(&w.key(Key::Down, NOW)), [&look(Some("Catppuccin Mocha"), None, None, None)]);
    assert_eq!(asks(&w.key(Key::Down, NOW)), [&look(Some("Nord"), None, None, None)]);
    assert_eq!(asks(&w.key(Key::Up, NOW)), [&look(Some("Catppuccin Mocha"), None, None, None)]);
    assert_eq!(asks(&w.key(Key::Char('p'), NOW)), [&look(None, Some("#9ece6a"), None, None)]);
    assert_eq!(asks(&w.key(Key::Char('l'), NOW)), [&look(None, None, Some(false), None)]);
    assert_eq!(asks(&w.key(Key::Char('+'), NOW)), [&look(None, None, None, Some(1))]);
    assert_eq!(asks(&w.key(Key::Char('-'), NOW)), [&look(None, None, None, Some(-1))]);
    // At the top and bottom the cursor stays, and nothing is asked.
    for _ in 0..10 {
        w.key(Key::Down, NOW);
    }
    assert!(asks(&w.key(Key::Down, NOW)).is_empty());
    assert_eq!(w.look().theme, "GitHub Light");
    // The colour cycles round to the theme's own.
    for _ in 0..5 {
        w.key(Key::Char('p'), NOW);
    }
    assert_eq!(w.look().primary, "");
}

#[test]
fn without_a_shell_there_is_no_folder_step_either_way() {
    let mut w = Welcome::new(fixtures::state(Handover::Cursor), true, false);
    w.key(Key::Enter, NOW);
    assert_eq!(w.step(), Step::Look, "the folder step is skipped going on");
    w.key(Key::Left, NOW);
    assert_eq!(w.step(), Step::Welcome, "and going back");
    for _ in 0..2 {
        w.key(Key::Enter, NOW);
    }
    assert_eq!(w.step(), Step::Agents);
    let end = w.key(Key::Enter, NOW);
    assert_eq!(end, vec![Effect::End(End { folder: None, skipped: false })]);
}

#[test]
fn escape_skips_from_anywhere_without_a_folder() {
    let mut w = fixtures::welcome(Handover::Cursor);
    w.key(Key::Enter, NOW);
    let effects = w.key(Key::Esc, NOW);
    assert_eq!(effects, vec![Effect::End(End { folder: None, skipped: true })]);
    assert!(w.key(Key::Enter, NOW).is_empty(), "nothing after the end");
}

#[test]
fn a_replay_ends_on_the_first_return() {
    let mut state = fixtures::state(Handover::Cursor);
    state.first_run = false;
    let mut w = Welcome::new(state, true, true);
    assert_eq!(w.key(Key::Enter, NOW), vec![Effect::End(End { folder: None, skipped: false })]);
}

#[test]
fn the_agents_turning_up_late_moves_the_third_step() {
    let mut state = fixtures::state(Handover::Cursor);
    state.agents = None;
    let mut w = Welcome::new(state.clone(), true, true);
    for _ in 0..3 {
        w.key(Key::Enter, NOW);
    }
    assert_eq!(w.step(), Step::Agents, "assumed to be there while they are looked for");
    state.agents = Some(Default::default());
    w.update(state);
    assert_eq!(w.step(), Step::Primer, "none found after all: the primer");
}

#[test]
fn the_app_updates_keep_the_cursor_and_the_run_s_own_facts() {
    let mut w = fixtures::welcome(Handover::Logo);
    w.key(Key::Enter, NOW);
    w.key(Key::Down, NOW);
    let mut state = fixtures::state(Handover::Cursor);
    state.first_run = false;
    w.update(state);
    assert_eq!(w.cursor(), 1, "the cursor stays");
    assert_eq!(w.handover(), Handover::Logo, "the hand-over was decided at the start");
    assert!(!w.is_replay(), "and so was whether this is the first run");
}

// --------------------------------------------------------------------------------- the driver

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

#[test]
fn it_asks_for_the_state_first_and_draws_nothing_until_the_wait_is_over() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    let first = driver.tick(ms(0), SIZE, &[], vec![]);
    assert_eq!(first.ask, [WelcomeOp::State { folder: None }]);
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
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(100), SIZE, &[], vec![state_answer(5000)]);
    let tick = driver.tick(ms(200), SIZE, b"\r", vec![]);
    assert!(driver.started(ms(200)));
    assert!(tick.end.is_none(), "Return only said 'now'");
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Welcome));
}

#[test]
fn the_app_may_shorten_the_wait_when_the_animation_is_skipped() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
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
    let mut driver = Driver::new(true, false, PathBuf::from("/nonexistent"));
    let tick = driver.tick(ms(0), SIZE, &[], vec![]);
    assert!(tick.ask.is_empty() && driver.started(ms(0)));
    assert!(text(&tick.draw).contains("Read-only tour") || driver.model().is_some_and(|m| !m.live()));
    // Return all the way through: it ends without a Finish to send.
    let mut end = None;
    for n in 1..=4 {
        end = driver.tick(ms(n * 50), SIZE, b"\r", vec![]).end.or(end);
    }
    assert_eq!(end.map(|e| e.skipped), Some(false));
}

#[test]
fn an_app_that_cannot_be_reached_turns_the_welcome_into_a_tour() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    let failed = Answer { op: "state", result: Err("Wormhole isn't running".to_string()) };
    let tick = driver.tick(ms(100), SIZE, &[], vec![failed]);
    assert!(tick.ask.is_empty(), "no asking again");
    assert!(driver.started(ms(100)));
    assert!(driver.model().is_some_and(|model| !model.live()));
}

#[test]
fn the_end_sends_the_finish_with_the_folder_and_then_stops() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer(0)]);
    let mut last = None;
    for n in 0..4 {
        let tick = driver.tick(ms(100 + n * 50), SIZE, b"\r", vec![]);
        if tick.end.is_some() {
            last = Some(tick);
        }
    }
    let tick = last.expect("it ended");
    let finish =
        WelcomeOp::Finish { folder: Some("/Users/me/Projects/terminal".to_string()), skipped: false };
    assert_eq!(tick.ask.last(), Some(&finish), "the Finish goes last, after any poll");
    assert!(driver.tick(ms(500), SIZE, b"\r", vec![]).ask.is_empty(), "nothing once it is over");
}

#[test]
fn esc_skips_after_a_pause_and_an_arrow_is_not_an_esc() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer(0)]);
    driver.tick(ms(100), SIZE, b"\x1b[B", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Welcome), "Down is not Esc");
    driver.tick(ms(150), SIZE, b"\x1b", vec![]);
    assert!(driver.tick(ms(170), SIZE, &[], vec![]).end.is_none(), "it waits to see if more comes");
    let tick = driver.tick(ms(220), SIZE, &[], vec![]);
    assert_eq!(tick.end.map(|e| e.skipped), Some(true));
    assert_eq!(tick.ask, [WelcomeOp::Finish { folder: None, skipped: true }]);
}

#[test]
fn state_is_polled_and_carries_the_folder_chosen_on_the_agents_page() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer(0)]);
    driver.tick(ms(100), SIZE, b"\r", vec![]);
    driver.tick(ms(150), SIZE, b"\r", vec![]);
    // On the look page nothing is asked about a folder's trust.
    let ask = driver.tick(ms(400), SIZE, &[], vec![]).ask;
    assert_eq!(ask, [WelcomeOp::State { folder: None }]);
    driver.tick(ms(420), SIZE, &[], vec![state_answer(0)]);
    driver.tick(ms(450), SIZE, b"\r", vec![]);
    let ask = driver.tick(ms(700), SIZE, &[], vec![]).ask;
    assert_eq!(
        ask,
        [WelcomeOp::State { folder: Some("/Users/me/Projects/terminal".to_string()) }],
        "the agents page's trust is about the folder picked"
    );
}

#[test]
fn a_refused_request_is_shown_until_one_works() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer(0)]);
    let refused = Answer { op: "install_hooks", result: Err("too many requests".to_string()) };
    driver.tick(ms(100), SIZE, &[], vec![refused]);
    assert_eq!(driver.model().and_then(Welcome::problem), Some("too many requests"));
    let fine = Answer { op: "install_hooks", result: Ok(Some(json!({}))) };
    driver.tick(ms(200), SIZE, &[], vec![fine]);
    assert_eq!(driver.model().and_then(Welcome::problem), None);
}

#[test]
fn a_click_on_a_row_presses_it() {
    let mut driver = Driver::new(true, true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer(0)]);
    // The welcome page's "Get started" is at row 8; a click there is Return.
    driver.tick(ms(100), SIZE, b"?", vec![]);
    driver.tick(ms(150), SIZE, &[], vec![]);
    driver.tick(ms(200), SIZE, b"\x1b[<0;6;9M", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Folder));
    // A click on nothing does nothing.
    driver.tick(ms(250), SIZE, b"\x1b[<0;70;20M", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Folder));
}
