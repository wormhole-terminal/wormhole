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

/// A switch is Rust's own: `profiles.json` says which profile is active, and the windows and the settings read
/// are that profile's; the other one's stay as they were, with their ids never repeated.
#[test]
fn a_switch_makes_the_other_profiles_windows_and_settings_the_ones_read_and_back_again() {
    use wormhole_core::config::Config;
    use wormhole_core::config::profile::{NewProfile, ProfileList, StartFrom};
    use wormhole_core::storage::paths::DataRoot;

    let dir = tempfile::tempdir().expect("temp dir");
    let root = DataRoot::new(dir.path());
    let mut list = ProfileList::default();
    let work = list
        .create(
            &root,
            NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
        )
        .expect("makes Work");
    list.save(&root).expect("saves the list");
    crate::settings::tests::write_file(&root.default_profile().config_file(), r#"{"font_size": 20}"#);

    // Wormhole is in Default: its window is the one saved there.
    let in_default = open(dir.path());
    let a = window(&in_default, 1);
    a.apply(Action::NewTerminal);
    assert!(in_default.save(1, a.clone(), None));
    assert_eq!(crate::profile::active_in(&root).id().as_str(), "default");
    assert_eq!(Config::load_for(&crate::profile::active_in(&root)).settings.font_size, 20.0);

    // Switch to Work: the profile read is Work's, with its own (default) settings and no windows saved yet.
    list.set_active(&work).expect("known");
    list.save(&root).expect("saves");
    let active = crate::profile::active_in(&root);
    assert_eq!(active.id(), &work);
    assert_ne!(Config::load_for(&active).settings.font_size, 20.0);
    let in_work = in_default.beside_in(&root, work.as_str()).expect("opens Work beside Default");
    assert!(in_work.saved().is_empty());
    let b = window(&in_work, in_work.alloc_window_id());
    let work_terminal = opened(&b.apply(Action::NewTerminal))[0];
    assert!(in_work.save(2, b.clone(), None));
    assert_eq!(in_work.saved().iter().map(|w| w.id).collect::<Vec<_>>(), [2]);
    assert_eq!(in_default.saved().iter().map(|w| w.id).collect::<Vec<_>>(), [1], "Default's is untouched");
    assert_eq!(in_default.shares().len(), 2, "the control line reaches the windows of both");
    assert!(work_terminal > 1, "an id is never taken twice: {work_terminal}");

    // And back: Default's settings and window are what is read again.
    list.set_active(&wormhole_core::storage::paths::ProfileId::default_profile()).expect("known");
    list.save(&root).expect("saves");
    let active = crate::profile::active_in(&root);
    assert_eq!(Config::load_for(&active).settings.font_size, 20.0);
    assert_eq!(open(active.dir()).saved().iter().map(|w| w.id).collect::<Vec<_>>(), [1]);
    assert!(in_default.beside_in(&root, "nobody").is_err(), "a profile that is not in the list");
}

/// "Move this window to...": the record leaves one profile's database for the other's, its notes with it, and the
/// window stays among the live ones (its terminals run on where they are).
#[test]
fn a_window_moved_to_another_profile_is_saved_there_and_gone_from_here() {
    use wormhole_core::config::profile::{NewProfile, ProfileList, StartFrom};
    use wormhole_core::storage::paths::DataRoot;

    let dir = tempfile::tempdir().expect("temp dir");
    let root = DataRoot::new(dir.path());
    let mut list = ProfileList::default();
    let work = list
        .create(
            &root,
            NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
        )
        .expect("makes Work");
    list.save(&root).expect("saves the list");
    let in_default = open(dir.path());
    let in_work = in_default.beside_in(&root, work.as_str()).expect("opens Work beside Default");
    let ws = window(&in_default, 1);
    ws.apply(Action::NewTerminal);
    assert!(in_default.save(1, ws.clone(), Some(frame(40.0))));

    assert!(in_default.rehome(1, ws.clone(), Some(frame(50.0)), in_work.clone()));
    assert!(in_default.saved().is_empty(), "gone from the profile it left");
    let saved = in_work.saved();
    assert_eq!((saved.len(), saved[0].id, saved[0].terminals), (1, 1, 1));
    assert_eq!(saved[0].frame.map(|f| f.x), Some(50.0));
    assert_eq!(in_default.shares().len(), 1, "still live: its terminal runs on");
    // To the profile it is in already is nothing to do.
    assert!(in_work.rehome(1, ws, None, in_work.clone()));
    assert_eq!(in_work.saved().len(), 1);
}

/// The databases number their windows apart, so a window moved into a profile that has a saved window of the same id
/// (or a terminal of the same id) stays where it is: nothing of the other profile is replaced or deleted.
#[test]
fn a_window_is_not_moved_over_a_saved_window_or_terminal_of_the_same_id() {
    use wormhole_core::config::profile::{NewProfile, ProfileList, StartFrom};
    use wormhole_core::storage::paths::DataRoot;

    let dir = tempfile::tempdir().expect("temp dir");
    let root = DataRoot::new(dir.path());
    let mut list = ProfileList::default();
    let work = list
        .create(
            &root,
            NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
        )
        .expect("makes Work");
    list.save(&root).expect("saves the list");
    // Work already has window 1 with terminals 1 and 2, saved by an earlier run.
    {
        let earlier = Windows::in_dir(root.profile(&work).dir()).expect("opens Work alone");
        let ws = window(&earlier, 1);
        ws.apply(Action::NewTerminal);
        ws.apply(Action::NewTerminal);
        assert!(earlier.save(1, ws, Some(frame(10.0))));
    }
    let in_default = open(dir.path());
    let in_work = in_default.beside_in(&root, work.as_str()).expect("opens Work beside Default");
    let ws = window(&in_default, 1);
    ws.apply(Action::NewTerminal);
    assert!(in_default.save(1, ws.clone(), Some(frame(40.0))));

    assert!(!in_default.rehome(1, ws, None, in_work.clone()), "refused: window 1 is Work's");
    let saved = in_work.saved();
    assert_eq!((saved.len(), saved[0].id, saved[0].terminals), (1, 1, 2), "Work's window is as it was");
    assert_eq!(saved[0].frame.map(|f| f.x), Some(10.0));
    assert_eq!(in_default.saved().len(), 1, "and the moving window did not leave");
}

/// A window of another profile keeps its notes in that profile's database (and not in the shown one's), so what is
/// typed there is still there after a relaunch into it.
#[test]
fn the_notes_of_a_window_in_another_profile_are_kept_in_that_profiles_database() {
    use wormhole_core::config::profile::{NewProfile, ProfileList, StartFrom};
    use wormhole_core::storage::paths::DataRoot;

    let dir = tempfile::tempdir().expect("temp dir");
    let root = DataRoot::new(dir.path());
    let mut list = ProfileList::default();
    let work = list
        .create(
            &root,
            NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
        )
        .expect("makes Work");
    list.save(&root).expect("saves the list");
    let id = {
        let in_default = open(dir.path());
        let in_work = in_default.beside_in(&root, work.as_str()).expect("opens Work beside Default");
        let number = in_work.alloc_window_id();
        let ws = window(&in_work, number);
        let id = opened(&ws.apply(Action::NewTerminal))[0];
        assert!(in_work.save(number, ws, None));
        let notes = crate::SessionNotesStore::in_profile(&root, work.as_str()).expect("Work's notes");
        let typed = crate::NotesSaved {
            tasks: vec![crate::TaskSaved { text: "ship".into(), done: false }],
            ..Default::default()
        };
        assert!(notes.save(id, typed));
        id
    };
    let again = crate::SessionNotesStore::in_profile(&root, work.as_str()).expect("Work's notes");
    assert_eq!(again.load(id).tasks.len(), 1, "kept where the window's profile is");
    let shown = crate::SessionNotesStore::in_dir(dir.path()).expect("Default's notes");
    assert!(shown.load(id).tasks.is_empty(), "and not in the profile Wormhole is in");
    assert!(crate::SessionNotesStore::in_profile(&root, "not a profile!").is_err());
}

/// The files a terminal changed come back with it after a restart (`sidebar_changes_remember`): the row counts them
/// again, saving the window does not lose them, and with the setting off they are deleted and not read.
#[test]
fn the_files_a_terminal_changed_are_kept_with_its_window() {
    use wormhole_core::storage::store::{ClaimRecord, Store};

    let dir = tempfile::tempdir().expect("temp dir");
    let claim = |path: &str| ClaimRecord { session_id: 1, root: "/repo".into(), path: path.into() };
    {
        let windows = open(dir.path());
        let ws = window(&windows, windows.alloc_window_id());
        ws.apply(Action::NewTerminal);
        assert!(windows.save(1, ws, None));
    }
    // What the last run kept (the attribution that fills it is core's, with its own test).
    Store::open_at(dir.path())
        .expect("store")
        .save_claims(&[1], &[claim("a.rs"), claim("b.rs")])
        .expect("kept");

    let windows = open(dir.path());
    let ws = window(&windows, 1);
    ws.restore();
    let changes = |ws: &Workspace| ws.sidebar().sections[0].rows[0].changes;
    assert_eq!(changes(&ws), Some(2), "the row counts them again");
    assert!(windows.save(1, ws.clone(), None));
    let kept = |dir: &Path| Store::open_at(dir).expect("store").claims(&[1]).expect("rows").len();
    assert_eq!(kept(dir.path()), 2, "a save that changes nothing keeps what was kept");

    windows.set_keeps_changes(false);
    assert!(windows.save(1, ws, None));
    assert_eq!(kept(dir.path()), 0, "with the setting off they are deleted");
    let off = open(dir.path());
    off.set_keeps_changes(false);
    Store::open_at(dir.path()).expect("store").save_claims(&[1], &[claim("c.rs")]).expect("kept");
    let ws = window(&off, 1);
    ws.restore();
    assert_eq!(changes(&ws), None, "and not read back");
}
