//! The look page's offer to bring over another terminal's settings: what it shows, the one key and
//! click that send `WelcomeOp::Import`, and what the page does with the app's answer. (The app's side
//! is `wormhole`'s `onboarding::welcome::answers`.)

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use super::driver::{Answer, Driver};
use super::facts;
use super::fixtures;
use super::flow::Step;
use super::keys::Key;
use super::model::{Effect, Import, Welcome};
use super::render::{Size, Target, render};
use crate::ipc::control::{ImportDone, WelcomeOp};
use crate::onboarding::intro::Handover;

const END: Duration = Duration::MAX;
const NOW: Duration = Duration::ZERO;

fn ghostty() -> WelcomeOp {
    WelcomeOp::Import { source: "ghostty".into() }
}

fn look(w: Welcome) -> Welcome {
    let mut w = w;
    w.key(Key::Enter, NOW);
    assert_eq!(w.step(), Step::Look);
    w
}

fn page(w: &Welcome, cols: usize, rows: usize) -> Vec<String> {
    render(w, Size { cols, rows }, END).grid.text()
}

fn has(rows: &[String], needle: &str) -> bool {
    rows.iter().any(|row| row.contains(needle))
}

fn asks(effects: &[Effect]) -> Vec<&WelcomeOp> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Ask(op) => Some(op),
            Effect::End(_) => None,
        })
        .collect()
}

// ------------------------------------------------------------------------------------ the page

#[test]
fn the_look_page_says_what_was_found_and_offers_it_on_a_key() {
    let w = look(fixtures::welcome_with_offer());
    let rows = page(&w, 80, 24);
    assert!(has(&rows, "  Ligatures  on   => != >="), "{rows:#?}");
    let at = rows.iter().position(|row| row.contains("Your Ghostty settings were found")).expect("found");
    assert_eq!(rows[at], "  Your Ghostty settings were found.");
    assert_eq!(rows[at + 1], "  i  Use my Ghostty settings");
    // The footer is as it was: the key is on the line itself.
    assert_eq!(rows[23], "  ↑↓ choose  p colour  l ligatures  + - size  ⏎ continue  ← back  esc skip",);
}

#[test]
fn nothing_found_adds_nothing_to_the_page() {
    let plain = look(fixtures::welcome(Handover::Cursor));
    let mut none = plain.clone();
    none.offer(None);
    assert_eq!(page(&plain, 80, 24), page(&none, 80, 24));
    assert!(!has(&page(&plain, 80, 24), "found"));
}

#[test]
fn the_offer_is_on_the_look_page_alone() {
    let mut w = fixtures::welcome_with_offer();
    for step in [Step::Welcome, Step::Look, Step::Notify, Step::Sidebar, Step::Extras] {
        while w.step() != step {
            w.key(Key::Enter, NOW);
        }
        let rows = page(&w, 80, 24);
        assert_eq!(has(&rows, "Your Ghostty settings"), step == Step::Look, "{step:?}");
    }
}

#[test]
fn a_short_window_gives_the_offer_the_room_and_shrinks_the_list() {
    let w = look(fixtures::welcome_with_offer());
    for height in [14, 18, 24, 40] {
        let rows = page(&w, 80, height);
        assert!(has(&rows, "i  Use my Ghostty settings"), "{height}: {rows:#?}");
        assert!(rows[height - 1].contains("esc skip"), "{height}");
    }
}

#[test]
fn the_offer_draws_at_every_size_in_every_state() {
    let mut asked = look(fixtures::welcome_with_offer());
    asked.key(Key::Char('i'), NOW);
    let mut done = asked.clone();
    done.imported(ImportDone { brought: 3, left: 0 });
    let mut failed = asked.clone();
    failed.import_failed("x".repeat(300));
    let mut tour = Welcome::new(facts::tour_state(Path::new("/nonexistent")), false);
    tour.offer(Some(fixtures::offer()));
    let tour = look(tour);
    for w in [look(fixtures::welcome_with_offer()), asked, done, failed, tour] {
        for cols in (1..=100).step_by(3) {
            for rows in (1..=40).step_by(3) {
                // Neither panics nor draws outside the screen (the grid clips); the footer stays last.
                let drawn = page(&w, cols, rows);
                assert_eq!(drawn.len(), rows);
            }
        }
    }
}

// ------------------------------------------------------------------------------------- the press

#[test]
fn i_on_the_look_page_asks_the_app_once() {
    let mut w = look(fixtures::welcome_with_offer());
    let effects = w.key(Key::Char('i'), NOW);
    assert_eq!(asks(&effects), [&ghostty()]);
    assert!(matches!(w.import(), Some(Import::Asked(_))));
    assert!(w.key(Key::Char('i'), NOW).is_empty(), "not again while it is asked");
    // The line no longer offers it.
    assert!(!has(&page(&w, 80, 24), "i  Use my"));
}

#[test]
fn a_click_on_the_offer_is_the_same_press() {
    let mut w = look(fixtures::welcome_with_offer());
    let frame = render(&w, Size { cols: 80, rows: 24 }, END);
    let row = frame.grid.text().iter().position(|row| row.contains("Use my Ghostty")).expect("the offer");
    let hit = frame.hits.iter().find(|hit| hit.contains(row, 6)).expect("clickable");
    assert_eq!(hit.target, Target::Key(Key::Char('i')));
    assert_eq!(asks(&w.key(Key::Char('i'), NOW)), [&ghostty()]);
}

#[test]
fn only_i_on_the_look_page_sends_an_import() {
    let keys = [
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
        Key::Enter,
        Key::Tab,
        Key::BackTab,
        Key::Backspace,
        Key::Char(' '),
        Key::Char('j'),
        Key::Char('k'),
        Key::Char('p'),
        Key::Char('l'),
        Key::Char('+'),
        Key::Char('-'),
        Key::Char('I'),
    ];
    // Every step, every key but `i`: nothing is imported.
    let mut w = fixtures::welcome_with_offer();
    for _ in 0..5 {
        for key in keys {
            let mut copy = w.clone();
            let effects = copy.key(key, NOW);
            assert!(asks(&effects).iter().all(|op| !matches!(op, WelcomeOp::Import { .. })), "{key:?}");
        }
        // And `i` elsewhere than the look page does nothing.
        if w.step() != Step::Look {
            assert!(w.clone().key(Key::Char('i'), NOW).is_empty(), "{:?}", w.step());
        }
        w.key(Key::Enter, NOW);
    }
}

#[test]
fn a_tour_shows_where_to_find_it_and_asks_for_nothing() {
    let mut w = Welcome::new(facts::tour_state(Path::new("/nonexistent")), false);
    w.offer(Some(fixtures::offer()));
    let w = look(w);
    let rows = page(&w, 80, 24);
    assert!(has(&rows, "Your Ghostty settings were found."), "{rows:#?}");
    assert!(has(&rows, "Settings → Import brings them over."));
    assert!(!has(&rows, "Use my"));
    let mut w = w;
    assert!(w.key(Key::Char('i'), NOW).is_empty());
    assert!(matches!(w.import(), Some(Import::Offered(_))), "nothing was asked");
}

// ----------------------------------------------------------------------------- the app's answer

#[test]
fn what_came_over_replaces_the_offer() {
    let mut w = look(fixtures::welcome_with_offer());
    w.key(Key::Char('i'), NOW);
    w.imported(ImportDone { brought: 11, left: 7 });
    let rows = page(&w, 80, 24);
    assert!(has(&rows, "✓ 11 settings brought over, 7 didn't: Settings → Import lists them"), "{rows:#?}");
    assert!(!has(&rows, "were found"));
    assert!(w.key(Key::Char('i'), NOW).is_empty(), "once");

    let mut w = look(fixtures::welcome_with_offer());
    w.key(Key::Char('i'), NOW);
    w.imported(ImportDone { brought: 1, left: 0 });
    assert!(has(&page(&w, 80, 24), "✓ 1 setting brought over"));
}

#[test]
fn a_refused_import_is_said_and_can_be_tried_again() {
    let mut w = look(fixtures::welcome_with_offer());
    w.key(Key::Char('i'), NOW);
    w.import_failed("the `welcome` request could not be read".to_string());
    let rows = page(&w, 80, 24);
    assert!(has(&rows, "could not be read"), "{rows:#?}");
    assert!(has(&rows, "i  Use my Ghostty settings"), "the offer stands");
    assert_eq!(asks(&w.key(Key::Char('i'), NOW)), [&ghostty()]);
}

#[test]
fn the_cursor_follows_the_theme_the_import_chose() {
    let mut w = look(fixtures::welcome_with_offer());
    w.key(Key::Char('i'), NOW);
    w.imported(ImportDone { brought: 11, left: 7 });
    // The app's next state lists the imported theme (the page offers the theme in use) and has it chosen.
    let mut state = fixtures::state(Handover::Cursor);
    state.look.themes.pop();
    state.look.themes.push(crate::ipc::control::ThemeFact { name: "Ghostty colours".into(), light: false });
    state.look.theme = "Ghostty colours".into();
    w.update(state, false);
    assert_eq!(w.cursor(), 5);
    let rows = page(&w, 80, 24);
    assert!(has(&rows, "  ❯ Ghostty colours"), "{rows:#?}");
    // Moving the cursor afterwards is the person's own again.
    w.key(Key::Up, NOW);
    assert_eq!(w.cursor(), 4);
}

// -------------------------------------------------------------------------------------- the driver

const SIZE: Size = Size { cols: 80, rows: 24 };

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn started() -> Driver {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    let state = serde_json::to_value(fixtures::state(Handover::Cursor)).expect("serializes");
    driver.tick(ms(10), SIZE, &[], vec![Answer { op: "state", result: Ok(Some(state)) }]);
    // To the look page.
    driver.tick(ms(100), SIZE, b"\r", vec![]);
    driver
}

#[test]
fn an_offer_that_comes_after_the_page_is_up_shows_and_a_press_goes_to_the_app() {
    let mut driver = started();
    driver.offer(Some(fixtures::offer()));
    driver.tick(ms(200), SIZE, &[], vec![]);
    let tick = driver.tick(ms(5000), SIZE, &[], vec![]);
    assert!(String::from_utf8_lossy(&tick.draw).contains("Ghostty"), "drawn once its line is out");
    let tick = driver.tick(ms(300), SIZE, b"i", vec![]);
    assert!(tick.ask.contains(&ghostty()), "{:?}", tick.ask);
    // The app's answer, in the form it is sent.
    driver.tick(
        ms(400),
        SIZE,
        &[],
        vec![Answer { op: "import", result: Ok(Some(json!({"brought": 9, "left": 2}))) }],
    );
    let done = driver.model().and_then(Welcome::import);
    assert!(matches!(done, Some(Import::Done(_, ImportDone { brought: 9, left: 2 }))), "{done:?}");
}

#[test]
fn an_offer_found_before_the_state_arrives_is_there_when_the_page_is() {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.offer(Some(fixtures::offer()));
    driver.tick(ms(0), SIZE, &[], vec![]);
    let state = serde_json::to_value(fixtures::state(Handover::Cursor)).expect("serializes");
    driver.tick(ms(10), SIZE, &[], vec![Answer { op: "state", result: Ok(Some(state)) }]);
    assert!(matches!(driver.model().and_then(Welcome::import), Some(Import::Offered(_))));
}

#[test]
fn an_app_that_cannot_read_the_op_is_a_note_not_the_end_of_the_welcome() {
    let mut driver = started();
    driver.offer(Some(fixtures::offer()));
    driver.tick(ms(200), SIZE, b"i", vec![]);
    let older = Answer { op: "import", result: Err("the `welcome` request could not be read".into()) };
    let tick = driver.tick(ms(300), SIZE, &[], vec![older]);
    assert!(tick.end.is_none());
    assert!(String::from_utf8_lossy(&tick.draw).contains("could not be read"));
    assert!(matches!(driver.model().and_then(Welcome::import), Some(Import::Offered(_))));
    // An answer that is not what was promised is the same.
    driver.tick(ms(400), SIZE, b"i", vec![]);
    driver.tick(ms(500), SIZE, &[], vec![Answer { op: "import", result: Ok(Some(json!({"nope": 1}))) }]);
    assert!(matches!(driver.model().and_then(Welcome::import), Some(Import::Offered(_))));
}

#[test]
fn a_tour_with_an_offer_asks_for_nothing() {
    let mut driver = Driver::new(false, PathBuf::from("/nonexistent"));
    driver.offer(Some(fixtures::offer()));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(50), SIZE, b"\r", vec![]);
    let tick = driver.tick(ms(100), SIZE, b"i", vec![]);
    assert!(tick.ask.is_empty(), "{:?}", tick.ask);
}
