//! What a terminal keeps for the notifications and the next launch beyond its row: a program's notification in
//! its own words (and `wh notify`'s), a failed command's facts, the shell it was opened with, and a result
//! nobody saw that comes back with a reattached host.

use std::time::Duration;

use wormhole_term::{Ended, Mark};

use crate::view::sidebar::Status;
use crate::workspace::{Action, Event, ModelConfig, ProgramNotice, StatusEvent, WorkspaceModel};

use super::model;

fn notice(title: &str, body: &str) -> StatusEvent {
    StatusEvent::Notice { title: title.into(), body: body.into() }
}

#[test]
fn a_programs_notification_keeps_its_words_numbered_and_rings_as_the_bell() {
    let mut m = model(2);
    let events = m.feed(2, notice("Build", "done in 3m"), 1_000);
    assert!(events.contains(&Event::Listing));
    let facts = m.facts(2).expect("terminal");
    assert_eq!(
        facts.notice,
        Some(ProgramNotice { serial: 1, title: "Build".into(), body: "done in 3m".into() })
    );
    // A plain terminal's notification is the bell with words: it asks for the person.
    assert_eq!(facts.status, Status::Attention);
    m.feed(2, notice("Build", "done in 3m"), 2_000);
    assert_eq!(
        m.facts(2).and_then(|f| f.notice.as_ref()).map(|n| n.serial),
        Some(2),
        "the same words again are news"
    );
    let summary = m.terminals_snapshot().terminals.into_iter().find(|t| t.id == 2).expect("listed");
    assert_eq!(summary.notice, 2, "the all-terminals list carries the serial");
}

#[test]
fn wh_notify_is_told_in_its_words_without_touching_the_status() {
    let mut m = model(2);
    m.feed(2, StatusEvent::ControlNotice { title: "Deploy".into(), body: "staging is up".into() }, 1_000);
    let facts = m.facts(2).expect("terminal");
    assert_eq!(facts.notice.as_ref().map(|n| (n.serial, n.body.as_str())), Some((1, "staging is up")));
    assert_eq!(facts.status, Status::Idle, "the caller said something; the terminal did not ring");
    assert!(!facts.bell);
}

#[test]
fn a_failed_command_keeps_how_long_it_ran_and_its_last_line_and_counts() {
    let mut m = model(2);
    let ended = |command: &str| {
        Some(Ended {
            command: command.into(),
            took: Some(Duration::from_secs(42)),
            last_line: Some("error[E0308]".into()),
        })
    };
    m.feed(2, StatusEvent::Mark { mark: Mark::OutputStart, ended: None }, 1_000);
    m.feed(2, StatusEvent::Mark { mark: Mark::Finished(Some(101)), ended: ended("cargo build") }, 43_000);
    let failure = m.facts(2).and_then(|f| f.failure.clone()).expect("a failure");
    assert_eq!((failure.serial, failure.code, failure.command.as_deref()), (1, 101, Some("cargo build")));
    assert_eq!((failure.took, failure.last_line.as_deref()), (Some(42_000), Some("error[E0308]")));
    // The same command failing the same way again is a new failure.
    m.feed(2, StatusEvent::Mark { mark: Mark::OutputStart, ended: None }, 50_000);
    m.feed(2, StatusEvent::Mark { mark: Mark::Finished(Some(101)), ended: ended("cargo build") }, 92_000);
    assert_eq!(m.facts(2).and_then(|f| f.failure.as_ref()).map(|f| f.serial), Some(2));
}

#[test]
fn a_terminal_opened_with_a_shell_keeps_it_through_a_save_and_a_restore() {
    let mut m = model(1);
    let events = m.apply(Action::NewTerminalWithShell("fish".into()));
    let id = events.iter().find_map(|e| match e {
        Event::Opened { id, .. } => Some(*id),
        _ => None,
    });
    let id = id.expect("opened");
    assert_eq!(m.selected(), Some(id));
    assert_eq!((m.shell_key(id), m.shell_key(1)), (Some("fish"), None));
    let record = m.window_record(1, None);
    assert_eq!(record.sessions.iter().find(|s| s.id == id).and_then(|s| s.shell.as_deref()), Some("fish"));
    let mut again = WorkspaceModel::new(ModelConfig::default());
    again.restore(&record);
    assert_eq!(again.shell_key(id), Some("fish"));
}

#[test]
fn a_result_nobody_saw_comes_back_only_with_its_reattached_program() {
    let mut m = model(2);
    m.feed(2, StatusEvent::Mark { mark: Mark::OutputStart, ended: None }, 1_000);
    m.feed(2, StatusEvent::Mark { mark: Mark::Finished(Some(0)), ended: None }, 2_000);
    assert_eq!(m.facts(2).map(|f| f.status), Some(Status::Done));
    let record = m.window_record(1, None);
    assert!(record.sessions.iter().find(|s| s.id == 2).is_some_and(|s| s.done_unseen));
    assert!(record.sessions.iter().find(|s| s.id == 1).is_some_and(|s| !s.done_unseen));

    let mut again = WorkspaceModel::new(ModelConfig::default());
    again.restore(&record);
    again.apply(Action::Select(1));
    assert_eq!(again.facts(2).map(|f| f.status), Some(Status::Idle), "a restored terminal starts idle");
    assert!(again.attached_again(2, 10).contains(&Event::Listing));
    assert_eq!(again.facts(2).map(|f| f.status), Some(Status::Done));
    // Asked once: a later attach (a reconnect) brings nothing back.
    again.feed(2, StatusEvent::Seen, 20);
    assert!(again.attached_again(2, 30).is_empty());
    assert!(again.attached_again(1, 30).is_empty(), "a terminal whose record said nothing gets nothing");
}
