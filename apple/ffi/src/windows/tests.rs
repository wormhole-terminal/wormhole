//! Several windows over real things: a database in a temp folder, terminals that are real programs
//! (`/bin/cat`), and the hand-over of a running terminal from one workspace to another.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use wormhole_core::workspace::Action;

use super::{FrameKind, FrameRecord, Windows};
use crate::{Terminal, TerminalListener, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};

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

fn config() -> WorkspaceConfig {
    WorkspaceConfig { home: None, numbers_follow_list: true, start_folder: None, pane_header_height: 0.0 }
}

fn window(windows: &Arc<Windows>, id: i64) -> Arc<Workspace> {
    Workspace::in_window(windows.clone(), id, config(), Arc::new(Wakes::default()))
}

fn opened(events: &[WorkspaceEvent]) -> Vec<i64> {
    events
        .iter()
        .filter_map(|event| match event {
            WorkspaceEvent::Opened { id, .. } => Some(*id),
            _ => None,
        })
        .collect()
}

fn frame(x: f32) -> FrameRecord {
    FrameRecord {
        kind: FrameKind::Windowed,
        x,
        y: 80.0,
        width: 1000.0,
        height: 700.0,
        display: Some(1),
        minimised: false,
    }
}

fn open(dir: &Path) -> Arc<Windows> {
    Windows::in_dir(dir).expect("opens")
}

fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn windows_of_one_app_never_share_an_id() {
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = open(dir.path());
    let (a, b) = (window(&windows, 1), window(&windows, 2));
    let mut ids = opened(&a.apply(Action::NewTerminal));
    ids.extend(opened(&b.apply(Action::NewTerminal)));
    ids.extend(opened(&a.apply(Action::NewTerminal)));
    assert_eq!(ids.len(), 3);
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3);
    assert_eq!(windows.shares().len(), 2);
}

#[test]
fn what_is_saved_comes_back_in_the_next_run_with_the_frame_and_the_notes_untouched() {
    let dir = tempfile::tempdir().expect("temp dir");
    {
        let windows = open(dir.path());
        let ws = window(&windows, windows.alloc_window_id());
        ws.apply(Action::NewTerminal);
        ws.apply(Action::NewTerminal);
        ws.apply(Action::Rename { id: 2, name: "api".into() });
        ws.apply(Action::Select(2));
        let id = windows.saved().len() as i64; // none saved yet
        assert_eq!(id, 0);
        // The Notes tab's rows are its own: a save of the window must not wipe them.
        assert!(windows.save(1, ws.clone(), Some(frame(120.0))));
        let notes = crate::SessionNotesStore::in_dir(dir.path()).expect("notes");
        assert!(notes.save(
            2,
            crate::NotesSaved {
                tasks: vec![crate::TaskSaved { text: "ship".into(), done: false }],
                ..Default::default()
            }
        ));
        assert!(windows.save(1, ws, Some(frame(130.0))));
        windows.save_front(1);
    }
    let windows = open(dir.path());
    let saved = windows.saved();
    assert_eq!((saved.len(), saved[0].id, saved[0].terminals), (1, 1, 2));
    assert_eq!(saved[0].frame.map(|f| f.x), Some(130.0));
    let ws = window(&windows, 1);
    let events = ws.restore();
    assert_eq!(opened(&events), [1, 2]);
    assert_eq!(ws.selected(), Some(2));
    assert_eq!(ws.sidebar().sections[0].rows[1].name, "api");
    let notes = crate::SessionNotesStore::in_dir(dir.path()).expect("notes");
    assert_eq!(notes.load(2).tasks.len(), 1);
    // New ids and windows start past what was saved.
    assert_eq!(opened(&ws.apply(Action::NewTerminal)), [3]);
    assert_eq!(windows.alloc_window_id(), 2);
}

#[test]
fn a_planned_window_starts_with_its_one_terminal_laid_out_like_the_one_it_came_from() {
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = open(dir.path());
    let first = window(&windows, 1);
    first.apply(Action::NewTerminal);
    first.apply(Action::NewGroup { members: vec![1] });
    first.apply(Action::ToggleSidebar);
    let planned = windows.plan(Some(first), Some("/work".into()));
    let second = window(&windows, planned.window);
    let events = second.restore();
    assert_eq!(
        events.iter().find_map(|e| match e {
            WorkspaceEvent::Opened { id, cwd } => Some((*id, cwd.clone())),
            _ => None,
        }),
        Some((planned.terminal, Some("/work".into())))
    );
    let chrome = second.chrome();
    assert!(!chrome.sidebar_visible, "the columns are the first window's");
    assert!(second.sidebar().sections.iter().all(|s| s.head.is_none()), "its groups are not");
}

#[test]
fn a_planned_window_without_a_folder_starts_where_the_selected_terminal_is() {
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = open(dir.path());
    let first = window(&windows, 1);
    let id = opened(&first.apply(Action::NewTerminal))[0];
    first.shared().with_model(|model| model.update_facts(id, |facts| facts.cwd = Some("/where".into())));
    assert_eq!(windows.plan(Some(first.clone()), None).folder.as_deref(), Some("/where"));
    assert_eq!(windows.plan(Some(first), Some("/given".into())).folder.as_deref(), Some("/given"));
    assert_eq!(windows.plan(None, None).folder, None, "nothing to take a folder from");
}

#[test]
fn a_forgotten_window_is_not_restored_and_a_quit_keeps_the_rest() {
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = open(dir.path());
    let (a, b) = (window(&windows, 1), window(&windows, 2));
    a.apply(Action::NewTerminal);
    b.apply(Action::NewTerminal);
    assert!(windows.save(1, a, None) && windows.save(2, b, None));
    windows.save_front(1);
    windows.forget(2);
    let saved = windows.saved();
    assert_eq!(saved.iter().map(|w| w.id).collect::<Vec<_>>(), [1]);
}

#[test]
fn a_running_terminal_moves_to_another_window_and_goes_on_running() {
    crate::isolate_for_tests();
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = open(dir.path());
    let (a, b) = (window(&windows, 1), window(&windows, 2));
    let wakes = Arc::new(Wakes::default());
    for _ in 0..2 {
        let id = opened(&a.apply(Action::NewTerminal))[0];
        let terminal =
            Terminal::start("/bin/cat".into(), vec![], 80, 24, 8.0, 16.0, wakes.clone()).expect("starts");
        a.register_terminal(id, terminal);
    }
    let moved = a.take_out(vec![2]);
    let departure = moved.departure.expect("it was here");
    assert_eq!(departure.ids(), [2]);
    assert!(moved.events.contains(&WorkspaceEvent::Closed { id: 2 }));
    assert!(a.terminal(2).is_none() && a.terminal(1).is_some());

    let events = b.receive(departure.clone());
    assert!(events.contains(&WorkspaceEvent::Adopted { id: 2 }));
    assert!(b.receive(departure).is_empty(), "taken in once");
    let terminal = b.terminal(2).expect("registered in the new window");
    assert!(terminal.is_alive(), "the program was not ended by the move");
    terminal.write(b"moved\n".to_vec());
    eventually("what it echoes", || terminal.screen_text().contains("moved"));
    // Its events feed the new window's row now.
    eventually("its row to be told its shell", || {
        b.sidebar().sections.iter().flat_map(|s| &s.rows).any(|r| r.id == 2)
    });
    a.apply(Action::Close(1));
    assert!(terminal.is_alive(), "closing the old window's terminal leaves the moved one running");
    b.apply(Action::Close(2));
    eventually("a closed terminal to end", || !terminal.is_alive());
}

#[test]
fn taking_out_what_is_not_here_takes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = open(dir.path());
    let a = window(&windows, 1);
    a.apply(Action::NewTerminal);
    let none = a.take_out(vec![99]);
    assert!(none.departure.is_none() && none.events.is_empty());
}
