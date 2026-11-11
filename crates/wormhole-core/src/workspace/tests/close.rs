//! Closing: who is selected afterwards, what happens to a split and to a group.

use crate::view::panes::Axis;
use crate::workspace::{Action, Event, session_after_close};

use super::{listed, model, refused};

#[test]
fn closing_sessions_keeps_the_selection_sensible() {
    // The app's own table (`window/session.rs`), unchanged.
    assert_eq!(session_after_close(0, 0, 1), None);
    assert_eq!(session_after_close(1, 1, 3), Some((1, true)));
    assert_eq!(session_after_close(2, 2, 3), Some((1, true)));
    assert_eq!(session_after_close(0, 0, 2), Some((0, true)));
    assert_eq!(session_after_close(2, 0, 4), Some((1, false)));
    assert_eq!(session_after_close(1, 3, 4), Some((1, false)));
    assert_eq!(session_after_close(0, 1, 2), Some((0, false)));
}

#[test]
fn the_selected_terminal_closing_selects_the_one_that_slid_into_its_place() {
    let mut m = model(3);
    m.apply(Action::Select(2));
    let events = m.apply(Action::Close(2));
    assert!(events.contains(&Event::Closed { id: 2 }));
    assert!(events.contains(&Event::Selected { id: Some(3) }));
    assert_eq!(m.selected(), Some(3));
    assert_eq!(m.ids(), [1, 3]);
}

#[test]
fn closing_another_terminal_leaves_the_selection_alone() {
    let mut m = model(3);
    m.apply(Action::Select(3));
    let events = m.apply(Action::Close(1));
    assert_eq!(m.selected(), Some(3));
    assert!(!events.iter().any(|e| matches!(e, Event::Selected { .. })));
}

#[test]
fn closing_the_only_terminal_starts_a_new_one() {
    let mut m = model(1);
    let events = m.apply(Action::Close(1));
    assert!(events.contains(&Event::Closed { id: 1 }));
    assert!(events.iter().any(|e| matches!(e, Event::Opened { .. })));
    assert_eq!(m.ids().len(), 1);
    assert_ne!(m.ids(), [1], "a terminal never reuses an id");
    assert_eq!(m.selected(), m.ids().first().copied());
}

#[test]
fn a_group_goes_with_its_last_terminal_unless_it_is_pinned() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::Close(1));
    assert!(m.sidebar_snapshot(0).sections.iter().all(|s| s.head.is_none()));
    m.apply(Action::NewGroup { members: vec![2] });
    m.apply(Action::SetGroupPinned { group: 1, pinned: true });
    m.apply(Action::Close(2));
    assert_eq!(listed(&m), ["[Group 1]", "3:1"], "a pinned group stays as a line");
}

#[test]
fn closing_a_split_member_keeps_the_keyboard_in_the_split() {
    let mut m = model(1);
    m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    let selected = m.selected().expect("a new pane is selected");
    let events = m.apply(Action::Close(selected));
    assert_ne!(m.selected(), Some(selected));
    let snapshot = m.panes_snapshot(1000.0, 600.0);
    assert_eq!(snapshot.panes.len(), 2, "the others took its room");
    assert_eq!(snapshot.focused, m.selected(), "the selection is the pane the layout sent the keyboard to");
    assert!(!refused(&events));
}

#[test]
fn closing_a_terminal_that_is_not_here_is_refused() {
    let mut m = model(1);
    assert!(refused(&m.apply(Action::Close(99))));
    assert_eq!(m.ids(), [1]);
}

/// Data loss if wrong: a terminal nobody is asked about ends with its program.
#[test]
fn a_program_in_a_folded_group_is_asked_about_and_a_failed_command_is_not() {
    use crate::view::sidebar::Status;
    use crate::workspace::Failure;
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    // An agent sitting idle at its prompt: its program holds the terminal.
    m.update_facts(2, |f| {
        f.agent = Some(crate::agents::AgentId::Claude);
        f.status = Status::Idle;
        f.command = Some("claude".into());
    });
    // A plain terminal whose last command failed: nothing runs.
    m.update_facts(4, |f| {
        f.status = Status::Failed;
        f.failure = Some(Failure { code: 101, command: Some("cargo build".into()), at: 0 });
    });
    m.apply(Action::ToggleGroup(1));
    m.apply(Action::Search("no such row".into()));
    assert!(m.sidebar_snapshot(0).sections.iter().all(|s| s.rows.is_empty()), "the list shows none of them");
    let asking: Vec<(i64, Option<String>)> = m
        .terminals_snapshot()
        .terminals
        .into_iter()
        .filter(|t| t.asks_to_close)
        .map(|t| (t.id, t.command))
        .collect();
    assert_eq!(asking, [(2, Some("claude".to_string()))]);
}
