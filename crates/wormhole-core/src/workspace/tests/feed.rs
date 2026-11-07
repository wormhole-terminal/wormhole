//! What the shell tells of a terminal becomes the row: names, state, and a change that shows is
//! announced once (the app's `summary_tests` and `notification_tests` chain, without a shell).

use crate::agents::AgentId;
use crate::view::sidebar::{RowState, Status};
use crate::workspace::{Action, Event, StatusEvent, TerminalFacts};

use super::model;

fn row_of(m: &crate::workspace::WorkspaceModel, id: i64) -> crate::workspace::SidebarRow {
    m.sidebar_snapshot(0).sections.iter().flat_map(|s| s.rows.clone()).find(|r| r.id == id).expect("row")
}

#[test]
fn a_terminal_is_named_by_its_title_then_its_agent_then_its_program_then_its_shell() {
    let mut m = model(1);
    assert_eq!(row_of(&m, 1).name, "Terminal 1", "nothing is known yet");
    m.update_facts(1, |f| f.shell = "zsh".into());
    assert_eq!(row_of(&m, 1).name, "zsh");
    m.update_facts(1, |f| f.command = Some("cargo watch".into()));
    assert_eq!(row_of(&m, 1).name, "cargo watch");
    m.update_facts(1, |f| f.agent = Some(AgentId::Claude));
    assert_eq!(row_of(&m, 1).name, "claude");
    m.update_facts(1, |f| f.title = Some("Fix the login bug".into()));
    assert_eq!(row_of(&m, 1).name, "Fix the login bug");
    m.apply(Action::Rename { id: 1, name: "  Mine ".into() });
    assert_eq!(row_of(&m, 1).name, "Mine", "a name the person gave wins");
    m.apply(Action::Rename { id: 1, name: "".into() });
    assert_eq!(row_of(&m, 1).name, "Fix the login bug", "a blank one gives it back");
}

#[test]
fn an_agents_status_is_its_rows_state_and_a_bell_is_a_plain_terminals() {
    let mut m = model(2);
    m.update_facts(1, |f| {
        f.agent = Some(AgentId::Codex);
        f.status = Status::Busy;
    });
    assert_eq!(row_of(&m, 1).state, RowState::Working);
    m.update_facts(1, |f| f.status = Status::Attention);
    assert_eq!(row_of(&m, 1).state, RowState::NeedsYou);
    m.update_facts(2, |f| f.status = Status::Busy);
    assert_eq!(row_of(&m, 2).state, RowState::Idle, "output alone is unread, not work");
    m.update_facts(2, |f| f.command = Some("make".into()));
    assert_eq!(row_of(&m, 2).state, RowState::Working);
    let agent = row_of(&m, 1).agent.expect("agent");
    assert_eq!((agent.key.as_str(), agent.short_name.as_str()), ("codex", "Codex"));
}

#[test]
fn a_change_that_shows_is_one_listing_event_and_a_change_that_does_not_is_none() {
    let mut m = model(1);
    let facts = TerminalFacts { shell: "zsh".into(), ..TerminalFacts::default() };
    assert_eq!(m.set_facts(1, facts.clone()), [Event::Listing]);
    assert!(m.set_facts(1, facts).is_empty(), "the same facts again say nothing");
    assert!(m.set_facts(99, TerminalFacts::default()).is_empty(), "a terminal that closed is ignored");
}

#[test]
fn the_row_carries_what_the_shell_knows_of_its_place() {
    let mut m = model(1);
    m.update_facts(1, |f| {
        f.branch = Some("main".into());
        f.changes = Some(3);
        f.notes = 2;
        f.bell = true;
        f.unread = true;
    });
    let row = row_of(&m, 1);
    assert_eq!(
        (row.branch.as_deref(), row.changes, row.notes, row.bell, row.unread),
        (Some("main"), Some(3), 2, true, true)
    );
    assert_eq!(row.keycap.as_deref(), Some("⌘1"));
}

#[test]
fn the_shell_is_told_to_start_and_end_programs_and_nothing_else_does() {
    let mut m = model(0);
    let events = m.apply(Action::NewTerminal);
    assert!(matches!(events[0], Event::Opened { id: 1, .. }));
    assert!(events.contains(&Event::Selected { id: Some(1) }));
    let events = m.apply(Action::Close(1));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Closed { .. })).count(), 1);
}

#[test]
fn what_is_in_front_of_the_person_counts_as_seen_and_a_question_waits_for_its_answer() {
    let mut m = model(3);
    m.update_facts(2, |f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Done;
        f.unread = true;
    });
    // Its hooks say it asks: a question the person looking does not answer (`status.rs`).
    let asks = StatusEvent::Hook { agent: AgentId::Claude, event: "PermissionRequest".into(), message: None };
    m.feed(3, asks, 0);
    let events = m.apply(Action::Select(2));
    assert!(events.contains(&Event::Listing));
    assert_eq!(row_of(&m, 2).state, RowState::Idle, "finished, then looked at");
    assert!(!row_of(&m, 2).unread);
    m.apply(Action::Select(3));
    assert_eq!(row_of(&m, 3).state, RowState::NeedsYou, "an agent's question stays until it is answered");
    m.update_facts(1, |f| {
        f.bell = true;
        f.status = Status::Attention;
    });
    assert_eq!(row_of(&m, 1).state, RowState::NeedsYou);
    m.apply(Action::Select(1));
    assert_eq!((row_of(&m, 1).state, row_of(&m, 1).bell), (RowState::Idle, false), "a bell is heard");
}
