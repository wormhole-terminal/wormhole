//! The Shown step end to end: the walk through how much the window shows by keys and by clicks, what the
//! driver sends the app and when, and the pages as snapshots at several sizes. (The walk's own rules are
//! `onboarding::tailor`'s tests; the sketches are `render::sketch`'s.)

use std::path::PathBuf;
use std::time::Duration;

use super::driver::{Answer, Driver};
use super::facts;
use super::fixtures;
use super::flow::Step;
use super::keys::Key;
use super::model::{Effect, End, Welcome};
use super::render::{Size, Target, render};
use crate::config::presets::{Preset, Shown, TitleShown};
use crate::ipc::control::WelcomeOp;
use crate::onboarding::intro::Handover;
use crate::onboarding::tailor::Question;

const NOW: Duration = Duration::ZERO;
const END: Duration = Duration::MAX;

fn asks(effects: &[Effect]) -> Vec<&WelcomeOp> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Ask(op) => Some(op),
            Effect::End(_) => None,
        })
        .collect()
}

fn set_shown(from: Shown, shown: Shown) -> WelcomeOp {
    WelcomeOp::SetShown { from, shown }
}

fn of(preset: Preset) -> Shown {
    Shown::of(preset)
}

/// A live welcome (the window shows what Balanced makes) on the Shown step.
fn on_the_step() -> Welcome {
    at_the_step(fixtures::welcome(Handover::Cursor))
}

fn at_the_step(mut w: Welcome) -> Welcome {
    w.key(Key::Enter, NOW);
    w.key(Key::Enter, NOW);
    assert_eq!(w.step(), Step::Shown);
    w
}

/// The question asked.
fn asking(w: &Welcome) -> Option<Question> {
    w.tailoring().and_then(|tailoring| tailoring.asking()).map(|step| step.question)
}

fn press(w: &mut Welcome, c: char) -> Vec<Effect> {
    w.key(Key::Char(c), NOW)
}

// -------------------------------------------------------------------------------------- the model

#[test]
fn a_number_gives_that_answer_goes_on_and_tells_the_app_only_when_it_changes_the_window() {
    let mut w = on_the_step();
    assert_eq!(asking(&w), Some(Question::Start));
    assert_eq!(asks(&press(&mut w, '1')), [&set_shown(of(Preset::Balanced), of(Preset::Zen))]);
    assert_eq!(asking(&w), Some(Question::TitleRow));
    // The title row is already just the name in Zen: nothing to tell, but it goes on.
    assert!(asks(&press(&mut w, '1')).is_empty());
    assert_eq!(asking(&w), Some(Question::StatusBar));
    let with_status = Shown { status_bar: true, ..Shown::of(Preset::Zen) };
    assert_eq!(asks(&press(&mut w, '2')), [&set_shown(of(Preset::Zen), with_status)]);
    // A number the question has no answer for does nothing; so do the other letters.
    for c in ['3', '9', '0', 'x'] {
        assert!(press(&mut w, c).is_empty(), "{c}");
        assert_eq!(asking(&w), Some(Question::Minimap), "{c}");
    }
}

#[test]
fn return_on_a_line_and_a_click_answer_as_the_number_does_and_return_on_the_button_goes_on() {
    let mut w = on_the_step();
    // The cursor starts on the button; up from it is the last answer.
    w.key(Key::Up, NOW);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_shown(of(Preset::Balanced), of(Preset::Everything))]);
    assert_eq!(asking(&w), Some(Question::TitleRow));
    assert_eq!(w.cursor(), 3, "the next question's cursor is on its button again");
    // A click on a line is a press on it.
    let effects = w.click_row(0, NOW);
    assert_eq!(
        asks(&effects),
        [&set_shown(of(Preset::Everything), Shown { title: TitleShown::Name, ..of(Preset::Everything) })]
    );
    // Return on the button keeps things as they are: on to the next step, nothing asked, and the
    // walk's other questions are left unanswered.
    assert!(w.key(Key::Enter, NOW).is_empty());
    assert_eq!(w.step(), Step::Notify);
}

#[test]
fn back_goes_to_the_question_before_and_from_the_first_to_the_step_before() {
    let mut w = on_the_step();
    press(&mut w, '1');
    press(&mut w, '2');
    assert_eq!(asking(&w), Some(Question::StatusBar));
    for key in [Key::Left, Key::Backspace, Key::Char('b')] {
        let before = asking(&w);
        assert!(w.key(key, NOW).is_empty(), "going back asks for nothing");
        assert_eq!(w.step(), Step::Shown);
        assert_ne!(asking(&w), before, "{key:?}");
        press(&mut w, '2');
    }
    // Back to the first question, and then past it, to the look.
    while asking(&w) != Some(Question::Start) {
        w.key(Key::Left, NOW);
    }
    assert_eq!(w.step(), Step::Shown);
    w.key(Key::Left, NOW);
    assert_eq!(w.step(), Step::Look);
    // The walk is where it was left, with the answers given.
    w.key(Key::Right, NOW);
    assert_eq!(w.step(), Step::Shown);
    assert_eq!(asking(&w), Some(Question::Start));
    let walk = w.tailoring().expect("a walk").walk();
    assert_ne!(&walk.shown, walk.before());
}

#[test]
fn start_over_puts_the_window_back_as_it_was_and_asks_only_when_that_changes_it() {
    let mut w = on_the_step();
    assert!(press(&mut w, 'r').is_empty(), "nothing was changed yet");
    press(&mut w, '1');
    press(&mut w, '2');
    assert_eq!(
        asks(&press(&mut w, 'r')),
        [&set_shown(Shown { title: TitleShown::Path, ..of(Preset::Zen) }, of(Preset::Balanced))]
    );
    assert_eq!(asking(&w), Some(Question::Start));
    assert!(press(&mut w, 'r').is_empty(), "and again: nothing to put back");
    // The answers after it are given from the first question again.
    assert_eq!(asks(&press(&mut w, '3')), [&set_shown(of(Preset::Balanced), of(Preset::Everything))]);
}

#[test]
fn after_the_last_question_a_summary_and_return_goes_on() {
    let mut w = on_the_step();
    for _ in 0..7 {
        press(&mut w, '1');
    }
    assert!(w.tailoring().is_some_and(|tailoring| tailoring.asking().is_none()), "the summary");
    assert_eq!(w.items().len(), 1, "one line, the button");
    assert!(press(&mut w, '1').is_empty(), "and no answer to give");
    // Back from the summary is the last question; Return on the summary goes on to Notify.
    w.key(Key::Left, NOW);
    assert_eq!(asking(&w), Some(Question::Tools));
    press(&mut w, '2');
    assert!(w.key(Key::Enter, NOW).is_empty());
    assert_eq!(w.step(), Step::Notify);
    w.key(Key::Left, NOW);
    assert_eq!(w.step(), Step::Shown, "Back from Notify is the walk");
}

#[test]
fn esc_skips_from_the_walk_as_from_any_page() {
    let mut w = on_the_step();
    press(&mut w, '1');
    assert_eq!(w.key(Key::Esc, NOW), vec![Effect::End(End { skipped: true })]);
}

/// Without the app (a tour) or with one that can't say what its window shows, the walk can be tried
/// but reaches nothing.
#[test]
fn a_tour_and_an_app_that_does_not_know_the_walk_are_read_only() {
    let tour = at_the_step(Welcome::new(facts::tour_state(&PathBuf::from("/nonexistent")), false));
    let mut state = fixtures::state(Handover::Cursor);
    state.shown = None;
    let old_app = at_the_step(Welcome::new(state, true));
    for (name, mut w) in [("tour", tour), ("old app", old_app)] {
        assert!(!w.tailor_live(), "{name}");
        let mut all = Vec::new();
        for c in ['1', '2', '1', 'b', 'r', '3'] {
            all.extend(press(&mut w, c));
        }
        w.key(Key::Up, NOW);
        all.extend(w.key(Key::Enter, NOW));
        assert!(asks(&all).is_empty(), "{name}: {all:?}");
        assert_eq!(w.step(), Step::Shown, "{name}: a line answers; it does not go on");
        assert_ne!(asking(&w), Some(Question::Start), "{name}: but the walk moved");
    }
}

// ------------------------------------------------------------------------------------ the driver

const SIZE: Size = Size { cols: 80, rows: 24 };

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn state_answer() -> Answer {
    let state = fixtures::state(Handover::Cursor);
    Answer { op: "state", result: Ok(Some(serde_json::to_value(state).expect("serializes"))) }
}

/// A driver on the Shown step.
fn driver_on_the_step() -> Driver {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer()]);
    driver.tick(ms(100), SIZE, b"\r", vec![]);
    driver.tick(ms(110), SIZE, b"\r", vec![]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Shown));
    driver
}

fn but_polls(ask: &[WelcomeOp]) -> Vec<&WelcomeOp> {
    ask.iter().filter(|op| **op != WelcomeOp::State).collect()
}

#[test]
fn moving_through_the_steps_alone_never_sends_an_answer_and_a_press_sends_one() {
    let mut driver = Driver::new(true, PathBuf::from("/nonexistent"));
    driver.tick(ms(0), SIZE, &[], vec![]);
    driver.tick(ms(10), SIZE, &[], vec![state_answer()]);
    let mut sent = Vec::new();
    // Return and the arrows through every step and back: the walk's pages are passed, not answered.
    for (n, input) in [&b"\r"[..], b"\r", b"\r", b"\x1b[D", b"\x1b[D", b"\x1b[C", b"\r", b"\r", b"\r"]
        .into_iter()
        .enumerate()
    {
        sent.extend(
            but_polls(&driver.tick(ms(100 + 10 * n as u64), SIZE, input, vec![]).ask).into_iter().cloned(),
        );
    }
    assert!(!sent.iter().any(|op| matches!(op, WelcomeOp::SetShown { .. })), "{sent:?}");
    // On the step, one number is one request.
    let mut driver = driver_on_the_step();
    let tick = driver.tick(ms(200), SIZE, b"1", vec![]);
    assert_eq!(but_polls(&tick.ask), [&set_shown(of(Preset::Balanced), of(Preset::Zen))]);
    // Back and Start over put it back.
    let tick = driver.tick(ms(210), SIZE, b"r", vec![]);
    assert_eq!(but_polls(&tick.ask), [&set_shown(of(Preset::Zen), of(Preset::Balanced))]);
    assert!(but_polls(&driver.tick(ms(220), SIZE, b"\x1b[D", vec![]).ask).is_empty());
}

/// A request the app refuses (it is asked too often, say): the page says so, takes the answer back
/// and the walk goes on.
#[test]
fn a_refused_answer_is_said_and_taken_back_and_the_walk_goes_on() {
    let mut driver = driver_on_the_step();
    let tick = driver.tick(ms(200), SIZE, b"1", vec![]);
    assert_eq!(but_polls(&tick.ask), [&set_shown(of(Preset::Balanced), of(Preset::Zen))]);
    let refused = Answer { op: "set_shown", result: Err("too many requests".to_string()) };
    let tick = driver.tick(ms(210), SIZE, &[], vec![refused]);
    assert_eq!(driver.model().and_then(Welcome::problem), Some("too many requests"));
    assert!(String::from_utf8_lossy(&tick.draw).contains("too many requests"));
    let shown = |driver: &Driver| driver.model().and_then(Welcome::tailoring).map(|t| t.walk().shown.clone());
    assert_eq!(shown(&driver), Some(of(Preset::Balanced)), "Zen never was");
    // Back, and Zen again: it is sent again, from what the app has.
    driver.tick(ms(220), SIZE, b"\x1b[D", vec![]);
    let tick = driver.tick(ms(230), SIZE, b"1", vec![]);
    assert_eq!(but_polls(&tick.ask), [&set_shown(of(Preset::Balanced), of(Preset::Zen))]);
    assert_eq!(driver.model().map(Welcome::step), Some(Step::Shown));
    // This one is taken: the note goes, and a refusal later keeps it.
    let taken = Answer { op: "set_shown", result: Ok(Some(serde_json::json!({"panel_open": false}))) };
    driver.tick(ms(240), SIZE, &[], vec![taken]);
    assert_eq!(driver.model().and_then(Welcome::problem), None);
    assert_eq!(shown(&driver), Some(of(Preset::Zen)));
}

#[test]
fn a_click_on_a_card_and_on_the_keys_along_the_bottom_presses_them() {
    let mut driver = driver_on_the_step();
    driver.tick(ms(200), SIZE, &[], vec![]);
    // The third card: SGR mouse coordinates are 1-based, columns 55.. on rows 7..14.
    let tick = driver.tick(ms(210), SIZE, b"\x1b[<0;60;10M", vec![]);
    assert_eq!(but_polls(&tick.ask), [&set_shown(of(Preset::Balanced), of(Preset::Everything))]);
    // The footer's "r start over" is the key.
    let frame = render(driver.model().expect("a model"), SIZE, END);
    let row = frame.grid.text_row(SIZE.rows - 1);
    let col = row.chars().position(|c| c == 'r').expect("an r hint");
    assert_eq!(
        frame.hits.iter().find(|hit| hit.contains(SIZE.rows - 1, col)).map(|hit| hit.target),
        Some(Target::Key(Key::Char('r')))
    );
}

// ------------------------------------------------------------------------------------ the pages

fn rows(w: &Welcome, cols: usize, rows: usize) -> Vec<String> {
    render(w, Size { cols, rows }, END).grid.text()
}

fn lines(text: &[&str]) -> Vec<String> {
    text.iter().map(ToString::to_string).collect()
}

/// The page up to the last row that has anything on it, without the footer.
fn page(w: &Welcome, cols: usize, height: usize) -> Vec<String> {
    let mut all = rows(w, cols, height);
    all.pop();
    while all.last().is_some_and(String::is_empty) {
        all.pop();
    }
    all
}

#[test]
fn the_first_question_offers_three_windows_side_by_side_with_the_one_in_use_named() {
    let w = on_the_step();
    assert_eq!(
        page(&w, 80, 24),
        lines(&[
            "",
            "  How much to show",
            "  Each answer changes the window around this terminal as you give it.",
            "",
            "  Where do you want to start?                                           1 of 7",
            "",
            "  ╭──────────────────────╮  ╭──────────────────────╮  ╭──────────────────────╮",
            "  │     ━━━            □ │  │   │ ━━·━·      ▫▫▫□□ │  │   │ ━━━ ▫▫▫□□ │ ▪▪▪▪ │",
            "  │ ▪━  $ ━━━━━━━━━━     │  │ ▪━│ $ ━━━━━━━━━━     │  │ ▪━│ $ ━━━━   ░│ ━━━  │",
            "  │ ▪━  ━━━━━━━━         │  │ ▪━│ ━━━━━━━━         │  │ ▪━│ ━━━━     ░│ ━━   │",
            "  │ ▪━  ━━━━━━━━━━━      │  │ ▪━│ ━━━━━━━━━━━      │  │ ▪━│ ━━━━━    ░│      │",
            "  │     ━━━━             │  │   │ ━━━━             │  │   │ ━━       ░│      │",
            "  │                      │  │   │                  │  │ ━━━               ━━ │",
            "  ╰──────────────────────╯  ╰──────────────────────╯  ╰──────────────────────╯",
            "   1 Zen                     2 Balanced                3 Everything",
            "  The terminal list and     Wormhole as it comes      Every part shown: the",
            "  the terminal: its name…                             whole title row, the st…",
            "",
            "  ❯ Continue  the rest stays as it is",
        ])
    );
    assert_eq!(
        rows(&w, 80, 24).pop().as_deref(),
        Some("  1-3 pick  ↑↓ move  ⏎ continue  ← back  r start over  esc skip")
    );
}

#[test]
fn the_cursor_marks_a_card_and_each_card_says_what_it_is_under_its_own_label() {
    let mut w = on_the_step();
    w.key(Key::Up, NOW);
    w.key(Key::Up, NOW);
    let text = page(&w, 80, 24);
    assert_eq!(text[14], "   1 Zen                    ❯2 Balanced                3 Everything");
    // The details are the cards' own, wherever the cursor is.
    let details = text[15].clone();
    assert_eq!(details, "  The terminal list and     Wormhole as it comes      Every part shown: the");
    w.key(Key::Up, NOW);
    let text = page(&w, 80, 24);
    assert_eq!(text[14], "  ❯1 Zen                     2 Balanced                3 Everything");
    assert_eq!(text[15], details);
    assert_eq!(rows(&w, 80, 24).pop().as_deref().map(|row| row.contains("⏎ pick")), Some(true));
}

#[test]
fn a_two_answer_question_has_two_cards_and_the_title_question_tells_its_rows_apart() {
    let mut w = on_the_step();
    press(&mut w, '2');
    let text = page(&w, 80, 24);
    assert_eq!(text[4], "  What should the title row show?                                       2 of 7");
    assert_eq!(text[7], "  │   │ ━━━            □ │  │   │ ━━ ━━· ━━·   □ □ │  │   │ ━━·━·      ▫▫▫□□ │");
    assert_eq!(text[14], "   1 Just the name           2 Where you are           3 Everything");
    // Down to the last question: two answers, each a wider card.
    for _ in 0..5 {
        press(&mut w, '2');
    }
    let text = page(&w, 80, 24);
    assert_eq!(text[4], "  Which tools should the right panel have?                              7 of 7");
    assert_eq!(text[7], "  │   │ ━━ ━━· ━━·  □ □ │ ▪▪   │  │   │ ━━ ━━· ━━·  □ □ │ ▪▪▪▪ │");
    assert_eq!(text[14], "   1 Changes and usage             2 All of them");
    assert_eq!(
        rows(&w, 80, 24).pop().as_deref(),
        Some("  1-2 pick  ↑↓ move  ⏎ continue  ← back  r start over  esc skip")
    );
}

#[test]
fn the_summary_lists_the_answers_and_draws_the_window_under_them() {
    let mut w = on_the_step();
    press(&mut w, '3');
    for _ in 0..6 {
        press(&mut w, '2');
    }
    assert_eq!(
        page(&w, 80, 24),
        lines(&[
            "",
            "  That's your window",
            "",
            "  What should the title row show?           Where you are",
            "  A status bar along the bottom?            On",
            "  A minimap beside each terminal?           On",
            "  Lines between the columns?                On",
            "  Should the right panel be open?           Open",
            "  Which tools should the right panel have?  All of them",
            "",
            "  ╭──────────────────────╮",
            "  │   │ ━━·━·  □□ │ ▪▪▪▪ │",
            "  │ ▪━│ $ ━━━━   ░│ ━━━  │",
            "  │ ▪━│ ━━━━     ░│ ━━   │",
            "  │ ▪━│ ━━━━━    ░│      │",
            "  │   │ ━━       ░│      │",
            "  │ ━━━               ━━ │",
            "  ╰──────────────────────╯",
            "",
            "  ❯ Looks good",
        ])
    );
    assert_eq!(rows(&w, 80, 24).pop().as_deref(), Some("  ⏎ continue  ← back  r start over  esc skip"));
}

#[test]
fn a_narrow_terminal_lists_the_answers_with_the_window_of_the_one_under_the_cursor() {
    let mut w = on_the_step();
    assert_eq!(
        page(&w, 50, 24),
        lines(&[
            "",
            "  How much to show",
            "  Each answer changes the window around this",
            "  terminal as you give it.",
            "",
            "  Where do you want to start?             1 of 7",
            "",
            "    1  Zen",
            "    2  Balanced ●",
            "    3  Everything",
            "  Wormhole as it comes",
            "",
            "",
            "  ╭────────────────────────────╮",
            "  │   │ ━━ ━━· ━━·   ▫ ▫ ▫ □ □ │",
            "  │ ▪━│ $ ━━━━━━━━━━━━━━━      │",
            "  │ ▪━│ ━━━━━━━━━━━            │",
            "  │ ▪━│ ━━━━━━━━━━━━━━━        │",
            "  ╰────────────────────────────╯",
            "",
            "  ❯ Continue  the rest stays as it is",
        ])
    );
    // The window follows the cursor.
    w.key(Key::Up, NOW);
    w.key(Key::Up, NOW);
    w.key(Key::Up, NOW);
    assert_eq!(page(&w, 50, 24)[14], "  │     ━━━                  □ │");
}

#[test]
fn a_window_with_no_room_for_sketches_has_the_list_and_the_button() {
    let w = on_the_step();
    assert_eq!(
        page(&w, 40, 14),
        lines(&[
            "",
            "  How much to show",
            "  Each answer changes the window",
            "  around this terminal as you give it.",
            "",
            "  Where do you want to start?   1 of 7",
            "",
            "    1  Zen",
            "    2  Balanced ●",
            "    3  Everything",
            "  Wormhole as it comes",
            "",
            "  ❯ Continue",
        ])
    );
}

#[test]
fn rows_of_a_card_and_the_button_can_be_clicked() {
    let w = on_the_step();
    let frame = render(&w, Size { cols: 80, rows: 24 }, END);
    let hit =
        |row: usize, col: usize| frame.hits.iter().find(|hit| hit.contains(row, col)).map(|hit| hit.target);
    // Each card is 24 wide with two between: its rows and its label.
    for row in 6..=14 {
        assert_eq!(hit(row, 5), Some(Target::Row(0)), "row {row}");
        assert_eq!(hit(row, 30), Some(Target::Row(1)), "row {row}");
        assert_eq!(hit(row, 60), Some(Target::Row(2)), "row {row}");
    }
    assert_eq!(hit(6, 27), None, "between the cards");
    assert_eq!(hit(18, 6), Some(Target::Row(3)), "the button");
}

#[test]
fn a_walk_without_the_app_says_so_where_the_tour_does() {
    let mut state = fixtures::state(Handover::Cursor);
    state.shown = None;
    let old_app = at_the_step(Welcome::new(state, true));
    let all = rows(&old_app, 80, 24);
    assert!(all[22].starts_with("  The window's settings can't be changed from here"), "{:?}", all[22]);
    let live = on_the_step();
    assert!(rows(&live, 80, 24).iter().all(|row| !row.contains("can't be changed from here")));
    let tour = at_the_step(Welcome::new(facts::tour_state(&PathBuf::from("/nonexistent")), false));
    let all = rows(&tour, 80, 24);
    assert!(all[22].starts_with("  Read-only tour"), "{:?}", all[22]);
}

/// Every question and the summary at many sizes, down to nothing: no panic, the keys along the bottom
/// when there is room, the button kept on a window too short for the page, and the palette only.
#[test]
fn every_question_and_the_summary_draw_at_every_size_with_palette_colours_only() {
    for answer in ['1', '2', '3'] {
        let mut w = on_the_step();
        for _ in 0..8 {
            for cols in [0, 1, 7, 14, 20, 30, 40, 50, 60, 68, 80, 120, 200] {
                for height in [0, 1, 2, 3, 6, 10, 14, 19, 20, 21, 24, 30, 50] {
                    let frame = render(&w, Size { cols, rows: height }, END);
                    assert_eq!(frame.grid.rows(), height);
                    for r in 0..height {
                        for c in 0..cols {
                            let style = frame.grid.style_at(r, c);
                            assert!(style.fg.is_none_or(|n| n < 16) && style.bg.is_none_or(|n| n < 16));
                        }
                    }
                    if cols >= 40 && height >= 10 {
                        let all = frame.grid.text();
                        assert!(all[height - 1].contains('⏎'), "{cols}x{height}: {:?}", all[height - 1]);
                        assert!(
                            all.iter().any(|row| row.contains("Continue") || row.contains("Looks good")),
                            "the button is kept at {cols}x{height}: {all:#?}"
                        );
                    }
                    for clock in [NOW, Duration::from_millis(37)] {
                        let _ = render(&w, Size { cols, rows: height }, clock);
                    }
                }
            }
            press(&mut w, answer);
        }
    }
}
