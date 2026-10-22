//! Walks the welcome as a person does: keys into the model. What is asked of the app, and when, is the
//! point: nothing is installed or changed without its own press. (The driver, with its ticks, is `driving`.)

use std::time::Duration;

use super::fixtures;
use super::flow::Step;
use super::keys::Key;
use super::model::{Effect, End, Welcome};
use crate::agents::AgentId;
use crate::config::NotifySound;
use crate::intro::Handover;
use crate::ipc::control::{PermissionFact, WelcomeOp};

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

/// An op that installs hooks or changes what reaches the person: only a press on its own row sends one.
fn installs_or_notifies(op: &WelcomeOp) -> bool {
    matches!(op, WelcomeOp::InstallHooks { .. } | WelcomeOp::SetNotify { .. })
}

fn set_notify(
    needs_you: Option<bool>,
    agent_done: Option<bool>,
    sound: Option<NotifySound>,
    allow: bool,
) -> WelcomeOp {
    WelcomeOp::SetNotify { needs_you, agent_done, sound, allow }
}

// ---------------------------------------------------------------------------------- the model

/// Moving through the steps with every key but Return (and the click that presses it) never asks for
/// an Install or a notification choice, whatever the order. This is the property the pages rest on.
#[test]
fn no_way_of_moving_about_installs_or_changes_what_reaches_you() {
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
                assert!(!installs_or_notifies(op), "{key:?} at {:?} asked for {op:?}", w.step());
            }
            if next.ended().is_none() {
                walk(&next, depth - 1, moves, seen);
            }
        }
    }
    let mut seen = 0;
    for permission in [PermissionFact::NotAsked, PermissionFact::Allowed] {
        for found in [true, false] {
            let mut state = fixtures::state(Handover::Cursor);
            if let Some(notify) = state.notify.as_mut() {
                notify.permission = permission;
            }
            if !found {
                state.agents = Some(Default::default());
            }
            walk(&Welcome::new(state, true), 5, &moves, &mut seen);
        }
    }
    assert!(seen > 20_000, "the walk covered {seen} moves");
}

#[test]
fn returning_through_every_step_installs_and_changes_nothing_and_ends() {
    let mut w = fixtures::welcome(Handover::Cursor);
    let mut all = Vec::new();
    for _ in 0..5 {
        all.extend(w.key(Key::Enter, NOW));
    }
    // Welcome, look, notifications, sidebar (the cursor starts on Continue on each), what else.
    assert!(asks(&all).is_empty(), "{all:?}");
    assert_eq!(all.last(), Some(&Effect::End(End { skipped: false })));
}

#[test]
fn the_steps_come_in_the_order_the_welcome_promises() {
    let mut w = fixtures::welcome(Handover::Cursor);
    let mut seen = vec![w.step()];
    for _ in 0..4 {
        w.key(Key::Enter, NOW);
        seen.push(w.step());
    }
    assert_eq!(seen, [Step::Welcome, Step::Look, Step::Notify, Step::Sidebar, Step::Extras]);
    for _ in 0..4 {
        w.key(Key::Left, NOW);
    }
    assert_eq!(w.step(), Step::Welcome);
}

#[test]
fn an_install_comes_only_from_return_on_that_agents_own_row() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..3 {
        w.key(Key::Enter, NOW);
    }
    assert_eq!(w.step(), Step::Sidebar);
    // Up from Continue: Aider (nothing to install), Codex (installed), Claude (Install).
    w.key(Key::Up, NOW);
    assert!(asks(&w.key(Key::Enter, NOW)).is_empty(), "Aider has no hooks");
    w.key(Key::Up, NOW);
    assert!(asks(&w.key(Key::Enter, NOW)).is_empty(), "Codex's are in");
    w.key(Key::Up, NOW);
    let effects = w.key(Key::Enter, NOW);
    assert_eq!(asks(&effects), [&WelcomeOp::InstallHooks { agent: AgentId::Claude }]);
    assert_eq!(w.step(), Step::Sidebar, "an install doesn't move on");
    // Space does the same on the same row; no other row is installed by it.
    let again = w.key(Key::Char(' '), NOW);
    assert_eq!(asks(&again), [&WelcomeOp::InstallHooks { agent: AgentId::Claude }]);
    // → goes on and installs nothing, with the cursor on the row that would.
    assert!(asks(&w.key(Key::Right, NOW)).is_empty());
    assert_eq!(w.step(), Step::Extras);
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
    assert_eq!(w.step(), Step::Look);
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

/// Each row of the notifications step sends its own choice when pressed, and nothing else.
#[test]
fn a_notification_choice_comes_only_from_a_press_on_its_own_row() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..2 {
        w.key(Key::Enter, NOW);
    }
    assert_eq!(w.step(), Step::Notify);
    // The cursor starts on Continue: going up the list presses nothing.
    for _ in 0..4 {
        assert!(asks(&w.key(Key::Up, NOW)).is_empty());
    }
    // Needs you: on, so Return turns it off.
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(Some(false), None, None, false)]);
    assert_eq!(w.notify().map(|notify| notify.needs_you), Some(false), "the page shows it at once");
    // Again: back on.
    assert_eq!(asks(&w.key(Key::Char(' '), NOW)), [&set_notify(Some(true), None, None, false)]);
    // Finished.
    w.key(Key::Down, NOW);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, Some(false), None, false)]);
    // Sound goes round: all, none, and back to needs you only.
    w.key(Key::Down, NOW);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, None, Some(NotifySound::All), false)]);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, None, Some(NotifySound::None), false)]);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, None, Some(NotifySound::NeedsYou), false)]);
    // Allow notifications asks macOS, and says nothing else.
    w.key(Key::Down, NOW);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, None, None, true)]);
    assert_eq!(w.step(), Step::Notify, "none of them moves on");
}

#[test]
fn a_click_on_a_notification_row_presses_it() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..2 {
        w.key(Key::Enter, NOW);
    }
    assert_eq!(asks(&w.click_row(1, NOW)), [&set_notify(None, Some(false), None, false)]);
    assert!(asks(&w.click_row(9, NOW)).is_empty(), "a click past the list does nothing");
}

#[test]
fn once_macos_has_been_asked_there_is_nothing_to_allow() {
    let mut state = fixtures::state(Handover::Cursor);
    if let Some(notify) = state.notify.as_mut() {
        notify.permission = PermissionFact::Allowed;
    }
    let mut w = Welcome::new(state, true);
    for _ in 0..2 {
        w.key(Key::Enter, NOW);
    }
    // Three switches and the button: one Up from the button is the sound, not an Allow.
    assert_eq!(w.items().len(), 4);
    w.key(Key::Up, NOW);
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, None, Some(NotifySound::All), false)]);
}

#[test]
fn a_read_only_tour_asks_for_nothing_whatever_is_pressed() {
    let mut w = Welcome::new(fixtures::state(Handover::Cursor), false);
    let mut all = Vec::new();
    for key in [Key::Enter, Key::Char('p'), Key::Char('l'), Key::Char('+'), Key::Down, Key::Enter] {
        all.extend(w.key(key, NOW));
    }
    for _ in 0..4 {
        for _ in 0..4 {
            w.key(Key::Up, NOW);
            all.extend(w.key(Key::Enter, NOW));
        }
        all.extend(w.key(Key::Right, NOW));
    }
    assert!(asks(&all).is_empty(), "{all:?}");
    assert!(!w.wants_demo(), "and no example band is asked for");
}

#[test]
fn the_look_is_applied_as_the_cursor_moves_and_by_its_keys() {
    let mut w = fixtures::welcome(Handover::Cursor);
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
fn escape_skips_from_anywhere() {
    let mut w = fixtures::welcome(Handover::Cursor);
    w.key(Key::Enter, NOW);
    let effects = w.key(Key::Esc, NOW);
    assert_eq!(effects, vec![Effect::End(End { skipped: true })]);
    assert!(w.key(Key::Enter, NOW).is_empty(), "nothing after the end");
}

#[test]
fn a_replay_ends_on_the_first_return() {
    let mut state = fixtures::state(Handover::Cursor);
    state.first_run = false;
    let mut w = Welcome::new(state, true);
    assert_eq!(w.key(Key::Enter, NOW), vec![Effect::End(End { skipped: false })]);
}

#[test]
fn the_example_band_is_wanted_on_the_sidebar_step_alone_while_the_welcome_runs() {
    let mut w = fixtures::welcome(Handover::Cursor);
    let mut wanted = Vec::new();
    for _ in 0..4 {
        wanted.push(w.wants_demo());
        w.key(Key::Enter, NOW);
    }
    wanted.push(w.wants_demo());
    assert_eq!(wanted, [false, false, false, true, false], "welcome, look, notify, sidebar, extras");
    // Back into it, and out by Esc: the end wants no band.
    w.key(Key::Left, NOW);
    assert!(w.wants_demo());
    w.key(Key::Esc, NOW);
    assert!(!w.wants_demo(), "the end takes it away");
}

#[test]
fn a_state_asked_before_a_choice_does_not_undo_it() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..2 {
        w.key(Key::Enter, NOW);
    }
    for _ in 0..4 {
        w.key(Key::Up, NOW);
    }
    w.key(Key::Enter, NOW);
    assert_eq!(w.notify().map(|notify| notify.needs_you), Some(false));
    // The state on its way was asked for before: it still says on. macOS' answer is taken all the same.
    let mut old = fixtures::state(Handover::Cursor);
    if let Some(notify) = old.notify.as_mut() {
        notify.permission = PermissionFact::Allowed;
    }
    w.update(old.clone(), true);
    let notify = w.notify().copied().expect("notify facts");
    assert!(!notify.needs_you, "the press stays");
    assert_eq!(notify.permission, PermissionFact::Allowed);
    assert_eq!(w.items().len(), 4, "and the Allow row is gone");
    // The next state is the app's word.
    w.update(old, false);
    assert_eq!(w.notify().map(|notify| notify.needs_you), Some(true));
}

#[test]
fn the_app_updates_keep_the_cursor_and_the_run_s_own_facts() {
    let mut w = fixtures::welcome(Handover::Logo);
    w.key(Key::Enter, NOW);
    w.key(Key::Down, NOW);
    let mut state = fixtures::state(Handover::Cursor);
    state.first_run = false;
    w.update(state, false);
    assert_eq!(w.cursor(), 1, "the cursor stays");
    assert_eq!(w.handover(), Handover::Logo, "the hand-over was decided at the start");
    assert!(!w.is_replay(), "and so was whether this is the first run");
}

#[test]
fn the_cursor_stays_on_its_row_when_the_allow_row_goes() {
    let mut w = fixtures::welcome(Handover::Cursor);
    for _ in 0..2 {
        w.key(Key::Enter, NOW);
    }
    // On Finished (the second switch).
    for _ in 0..3 {
        w.key(Key::Up, NOW);
    }
    assert_eq!(w.cursor(), 1);
    let mut state = fixtures::state(Handover::Cursor);
    if let Some(notify) = state.notify.as_mut() {
        notify.permission = PermissionFact::Allowed;
    }
    w.update(state, false);
    assert_eq!(w.cursor(), 1, "a row above the one that went doesn't move");
    assert_eq!(asks(&w.key(Key::Enter, NOW)), [&set_notify(None, Some(false), None, false)]);
}
