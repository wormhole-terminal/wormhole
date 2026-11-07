//! The workspace through its public surface, as Swift drives it: actions in, events and snapshots
//! out, and a real terminal (`/bin/cat`) feeding a row.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use wormhole_core::view::panes::Axis;
use wormhole_core::view::sidebar::{Part, RowState};
use wormhole_core::workspace::{Action, Over, Refusal};

use crate::{
    TerminalFactsRecord, TerminalListener, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener,
};

#[derive(Default)]
struct Wakes(AtomicUsize);

impl WorkspaceListener for Wakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl TerminalListener for Wakes {
    fn wake(&self) {}
}

fn workspace() -> (Arc<Workspace>, Arc<Wakes>) {
    let wakes = Arc::new(Wakes::default());
    let config = WorkspaceConfig {
        home: Some("/Users/me".into()),
        numbers_follow_list: true,
        start_folder: Some("/Users/me/Projects/app".into()),
        pane_header_height: 0.0,
    };
    (Workspace::new(config, wakes.clone()), wakes)
}

fn ids(workspace: &Workspace) -> Vec<i64> {
    workspace.sidebar_at(0).sections.iter().flat_map(|s| &s.rows).map(|r| r.id).collect()
}

/// Wait (bounded) until `done` holds: a terminal's events arrive on a thread of their own.
fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn actions_in_events_and_snapshots_out() {
    let (ws, _) = workspace();
    let opened = ws.apply(Action::NewTerminal);
    assert_eq!(
        opened[0],
        WorkspaceEvent::Opened { id: 1, cwd: Some("/Users/me/Projects/app".into()) },
        "the first terminal starts in the configured folder"
    );
    ws.apply(Action::NewTerminal);
    ws.apply(Action::NewTerminal);
    assert_eq!(ws.selected(), Some(3));
    assert_eq!(ids(&ws), [1, 2, 3]);

    // A group, a rename, a pin: the snapshot is what the sidebar draws.
    ws.apply(Action::NewGroup { members: vec![2, 3] });
    ws.apply(Action::RenameGroup { group: 1, name: "Release".into() });
    ws.apply(Action::Rename { id: 1, name: "notes".into() });
    ws.apply(Action::TogglePin(1));
    let snapshot = ws.sidebar_at(0);
    let head = snapshot.sections[1].head.as_ref().expect("the group's section");
    assert_eq!((head.name.as_str(), head.folded, head.position, head.of), ("Release", false, 2, 2));
    let first = &snapshot.sections[0].rows[0];
    assert_eq!(
        (first.name.as_str(), first.pinned, first.number, first.keycap.as_deref()),
        ("notes", true, Some(1), Some("⌘1"))
    );
    assert_eq!(first.second_line.text.as_deref(), Some("Shell"), "no folder is known for it yet");
    assert_eq!(snapshot.sections[1].rows.iter().map(|r| r.number).collect::<Vec<_>>(), [Some(2), Some(3)]);
    assert!(first.spoken.starts_with("notes, shell, nothing running, 1 of 2"), "{}", first.spoken);

    // A drag: ask first (a pure query), then drop.
    let over = Over::Row { id: 1, part: Part::Bottom };
    assert!(ws.drop_plan(3, over).is_some());
    let before = ws.revisions();
    let events = ws.apply(Action::DropTerminal { dragged: 3, over });
    assert!(events.contains(&WorkspaceEvent::Moved { id: 3 }));
    assert!(ws.revisions().sidebar > before.sidebar);

    // A split: the new pane is a terminal for Swift to start.
    let events = ws.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    let WorkspaceEvent::Opened { id: pane, .. } = events[0].clone() else { panic!("{events:?}") };
    assert_eq!(ws.selected(), Some(pane));
    let panes = ws.panes(1000.0, 600.0);
    assert_eq!(panes.panes.iter().map(|p| p.id).collect::<Vec<_>>(), [1, pane]);
    assert_eq!(panes.focused, Some(pane));

    // A refusal is data, not an error.
    assert!(matches!(ws.apply(Action::Close(99))[0], WorkspaceEvent::Refused { reason: Refusal::Unknown }));
    ws.apply(Action::Close(pane));
    assert_eq!(ws.panes(1000.0, 600.0).panes.len(), 1);
}

#[test]
fn facts_set_from_outside_show_on_the_row_and_the_snapshot_says_when_its_words_move() {
    let (ws, wakes) = workspace();
    ws.apply(Action::NewTerminal);
    let events = ws.set_git(1, Some("main".into()), Some(3));
    assert_eq!(events, [WorkspaceEvent::Listing]);
    let facts = TerminalFactsRecord {
        title: None,
        shell: "zsh".into(),
        agent: Some("claude".into()),
        reports: false,
        status: wormhole_core::view::sidebar::Status::Busy,
        message: None,
        since_ms: 0,
        cwd: Some("/Users/me/Projects/app".into()),
        branch: Some("main".into()),
        command: None,
        running_since_ms: None,
        bell: false,
        failure_code: None,
        failure_command: None,
        failure_at_ms: 0,
        unread: false,
        changes: Some(3),
        notes: 0,
    };
    ws.set_facts(1, facts);
    let snapshot = ws.sidebar_at(3 * 60_000 + 1_000);
    let row = &snapshot.sections[0].rows[0];
    assert_eq!((row.state, row.name.as_str()), (RowState::Working, "claude"));
    assert_eq!(row.agent.as_ref().map(|a| a.short_name.as_str()), Some("Claude"));
    assert_eq!(row.second_line.text.as_deref(), Some("Working · 3m"));
    assert_eq!((row.branch.as_deref(), row.changes), (Some("main"), Some(3)));
    assert_eq!(snapshot.refresh_in_ms, Some(59_000), "the next minute of the working time");
    // Calls Swift made answer in their own return: nothing was queued, nothing woke.
    assert!(ws.drain().is_empty());
    assert_eq!(wakes.0.load(Ordering::SeqCst), 0);
}

#[test]
fn a_registered_terminal_feeds_its_row_and_wakes_once_for_what_it_told() {
    crate::isolate_for_tests();
    let (ws, wakes) = workspace();
    for _ in 0..2 {
        ws.apply(Action::NewTerminal);
    }
    let start = |id: i64| {
        let terminal = crate::Terminal::start("/bin/cat".into(), vec![], 80, 24, 8.0, 16.0, wakes.clone())
            .expect("cat starts in an isolated process");
        ws.register_terminal(id, terminal.clone());
        terminal
    };
    let (first, _second) = (start(1), start(2));
    assert_eq!(ws.selected(), Some(2), "the new terminal took the selection");
    let name_of = |id: i64| {
        ws.sidebar_at(0).sections.iter().flat_map(|s| s.rows.clone()).find(|r| r.id == id).map(|r| r.name)
    };
    assert_eq!(name_of(1).as_deref(), Some("cat"), "named for its program until its title says more");

    // The program sets a title, and rings the bell while terminal 2 is the one in front.
    first.write(b"\x1b]0;My title\x07\n".to_vec());
    eventually("the title", || name_of(1).as_deref() == Some("My title"));
    let row = |id: i64| ws.sidebar_at(0).sections.iter().flat_map(|s| s.rows.clone()).find(|r| r.id == id);
    eventually("the bell", || {
        first.write(b"\x07\n".to_vec());
        row(1).is_some_and(|r| r.bell && r.state == RowState::NeedsYou)
    });
    let events = ws.drain();
    assert!(events.contains(&WorkspaceEvent::Listing));
    assert_eq!(events.iter().filter(|e| **e == WorkspaceEvent::Listing).count(), 1, "told once");
    assert!(wakes.0.load(Ordering::SeqCst) >= 1);

    // Selecting the terminal hears the bell; closing it ends its program.
    ws.apply(Action::Select(1));
    assert!(row(1).is_some_and(|r| !r.bell && r.state == RowState::Idle));
    let events = ws.apply(Action::Close(1));
    assert!(events.contains(&WorkspaceEvent::Closed { id: 1 }));
    assert!(ws.terminal(1).is_none());
    assert!(first.frame().is_empty(), "the Terminal was closed with its row");
}
