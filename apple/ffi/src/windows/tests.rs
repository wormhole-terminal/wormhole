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

/// Two profiles, Default (shown) and Work; `earlier` is what Work saved on its own, before the profiles shared one
/// counter (its windows and terminals numbered from 1).
struct Pair {
    _dir: tempfile::TempDir,
    root: wormhole_core::storage::paths::DataRoot,
    work: String,
}

fn pair() -> Pair {
    use wormhole_core::config::profile::{NewProfile, ProfileList, StartFrom};
    let dir = tempfile::tempdir().expect("temp dir");
    let root = wormhole_core::storage::paths::DataRoot::new(dir.path());
    let mut list = ProfileList::default();
    let work = list
        .create(
            &root,
            NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
        )
        .expect("makes Work");
    list.save(&root).expect("saves the list");
    Pair { _dir: dir, root, work: work.as_str().to_owned() }
}

/// A window Work saved on its own: terminals `ids` in one split, in one group, with notes, a task, a pin, hidden from
/// agents, and a file claimed.
fn earlier_window(pair: &Pair, window: i64, ids: [i64; 2]) {
    use wormhole_core::storage::store::{
        ClaimRecord, NoteRecord, SessionRecord, SplitRecord, TaskRecord, WindowRecord,
    };
    use wormhole_core::view::panes::{Axis, TabPanes};
    use wormhole_core::workspace::SavedLayout;

    let earlier = Windows::in_dir(
        pair.root.profile(&wormhole_core::storage::paths::ProfileId::parse(&pair.work).expect("id")).dir(),
    )
    .expect("opens Work alone");
    let mut panes = TabPanes::single(ids[0]);
    assert!(panes.split(Axis::Row, ids[1]));
    let split = ids[1] + 100;
    let mut layout = SavedLayout::default();
    layout.sidebar.groups.create(&ids);
    let session = |id: i64| SessionRecord {
        split_id: Some(split),
        notes: vec![NoteRecord { body: format!("note of {id}"), collapsed: false }],
        tasks: vec![TaskRecord { text: format!("task of {id}"), done: true }],
        pinned: true,
        agent_hidden: true,
        ..SessionRecord::new(id, Some(format!("t{id}")), Some("/tmp".into()))
    };
    let mut store = earlier.store_locked();
    store
        .save_window(&WindowRecord {
            id: window,
            layout: serde_json::to_string(&layout).ok(),
            bounds: Some(wormhole_core::view::layout::frame::WindowFrame::from(frame(10.0)).to_json()),
            sessions: ids.iter().map(|id| session(*id)).collect(),
            splits: vec![SplitRecord { id: split, layout: Some(panes.to_json()) }],
            ..Default::default()
        })
        .expect("saved");
    let claims: Vec<ClaimRecord> = ids
        .iter()
        .map(|id| ClaimRecord { session_id: *id, root: "/repo".into(), path: format!("f{id}.rs") })
        .collect();
    store.save_claims(&ids, &claims).expect("claimed");
}

/// What `windows` holds of one saved window, as the database has it.
fn saved_record(windows: &Windows, id: i64) -> Option<wormhole_core::storage::store::WindowRecord> {
    windows.store_locked().load().expect("loads").into_iter().find(|w| w.id == id)
}

/// Work's window as it was saved, found again after a move renumbered it: whatever window of `windows` has `terminals`
/// terminals and the frame at x 10.
fn renumbered(windows: &Windows) -> wormhole_core::storage::store::WindowRecord {
    let all = windows.store_locked().load().expect("loads");
    let mut found = all.into_iter().filter(|w| {
        w.bounds
            .as_deref()
            .and_then(wormhole_core::view::layout::frame::WindowFrame::from_json)
            .is_some_and(|f| f.x == 10.0)
    });
    let record = found.next().expect("Work's window is still there");
    assert!(found.next().is_none());
    record
}

fn assert_whole(record: &wormhole_core::storage::store::WindowRecord, windows: &Windows) {
    use wormhole_core::view::panes::TabPanes;
    assert_eq!(record.sessions.len(), 2);
    for session in &record.sessions {
        // Named after the id it was saved with, which is not its id any more.
        let old = &session.name.as_deref().expect("name")[1..];
        assert_eq!(session.notes[0].body, format!("note of {old}"), "notes");
        assert_eq!(
            (session.tasks[0].text.as_str(), session.tasks[0].done),
            (format!("task of {old}").as_str(), true)
        );
        assert!(session.pinned && session.agent_hidden, "flags");
        assert_eq!(windows.store_locked().claims(&[session.id]).expect("claims").len(), 1, "claims follow");
    }
    let split = &record.splits[0];
    assert!(record.sessions.iter().all(|s| s.split_id == Some(split.id)));
    let panes = TabPanes::from_json(split.layout.as_deref().expect("layout")).expect("reads");
    let mut leaves = panes.tree().panes();
    leaves.sort_unstable();
    let mut ids: Vec<i64> = record.sessions.iter().map(|s| s.id).collect();
    ids.sort_unstable();
    assert_eq!(leaves, ids, "the split's panes are the renumbered terminals");
    let layout: wormhole_core::workspace::SavedLayout =
        serde_json::from_str(record.layout.as_deref().expect("layout")).expect("reads");
    let mut members = layout.sidebar.groups.0[0].members.clone();
    members.sort_unstable();
    assert_eq!(members, ids, "and so are the group's members");
}

/// Moving a window into a profile whose saved window has the same window id: the profile's own window is given new ids
/// (it is not running), nothing of it is lost, and the moving window keeps its ids and its terminals run on.
#[test]
fn a_window_moved_over_a_saved_window_of_the_same_id_renumbers_the_saved_one() {
    let pair = pair();
    earlier_window(&pair, 1, [1, 2]);
    let in_default = open(pair.root.base());
    let in_work = in_default.beside_in(&pair.root, &pair.work).expect("opens Work beside Default");
    let ws = window(&in_default, 1);
    let mine = opened(&ws.apply(Action::NewTerminal))[0];
    assert!(in_default.save(1, ws.clone(), Some(frame(40.0))));

    assert!(in_default.rehome(1, ws.clone(), None, in_work.clone()));
    let work_windows = in_work.store_locked().load().expect("loads");
    assert_eq!(work_windows.len(), 2, "Work has both");
    assert!(
        saved_record(&in_work, 1).is_some_and(|w| w.sessions.len() == 1 && w.sessions[0].id == mine),
        "the mover keeps window 1"
    );
    let theirs = renumbered(&in_work);
    assert!(theirs.id != 1 && theirs.sessions.iter().all(|s| s.id != 1 && s.id != 2 && s.id != mine));
    assert_whole(&theirs, &in_work);
    assert!(in_default.saved().is_empty(), "and it left Default");
    let seat = ws.shared().windows().expect("among windows").0;
    assert_eq!(seat.profile().id().as_str(), pair.work, "its requests are judged by Work from now on");

    // And back: Default has nothing under its ids now.
    assert!(in_work.rehome(1, ws.clone(), None, in_default.clone()));
    assert_eq!(in_default.saved().len(), 1);
    assert_eq!(in_work.saved().len(), 1, "Work's own window stays");
    assert_whole(&renumbered(&in_work), &in_work);
    assert_eq!(ws.shared().windows().expect("among windows").0.profile().id().as_str(), "default");
}

/// A terminal of the moving window with the id of a terminal in another saved window (the window ids differ): that
/// window gets new ids, the moving terminal keeps its own.
#[test]
fn a_colliding_terminal_alone_renumbers_the_saved_window_that_has_it() {
    let pair = pair();
    let in_default = open(pair.root.base());
    let ws = window(&in_default, 1);
    let mine = opened(&ws.apply(Action::NewTerminal))[0];
    assert_eq!(mine, 1);
    assert!(in_default.save(1, ws.clone(), None));
    earlier_window(&pair, 7, [1, 2]);
    let in_work = in_default.beside_in(&pair.root, &pair.work).expect("opens Work beside Default");

    assert!(in_default.rehome(1, ws, None, in_work.clone()));
    assert!(saved_record(&in_work, 7).is_none(), "window 7 was renumbered");
    assert!(saved_record(&in_work, 1).is_some_and(|w| w.sessions[0].id == 1));
    let theirs = renumbered(&in_work);
    assert!(theirs.id != 7 && theirs.sessions.iter().all(|s| s.id > 2));
    assert_whole(&theirs, &in_work);
}

/// A window of the target that is open is not dormant: its ids are its own, so the move is refused and nothing changes.
#[test]
fn a_window_is_not_moved_over_an_open_window_of_the_same_id() {
    let pair = pair();
    let in_default = open(pair.root.base());
    let in_work = in_default.beside_in(&pair.root, &pair.work).expect("opens Work beside Default");
    let theirs = window(&in_work, 1);
    theirs.apply(Action::NewTerminal);
    assert!(in_work.save(1, theirs.clone(), Some(frame(10.0))));
    let ws = window(&in_default, 1);
    ws.apply(Action::NewTerminal);
    assert!(in_default.save(1, ws.clone(), Some(frame(40.0))));

    assert!(!in_default.rehome(1, ws.clone(), None, in_work.clone()), "refused: Work's window 1 is open");
    assert_eq!(in_work.saved().len(), 1);
    assert_eq!(in_work.saved()[0].frame.map(|f| f.x), Some(10.0));
    assert_eq!(in_default.saved().len(), 1, "and the moving window did not leave");
    assert_eq!(ws.shared().windows().expect("among windows").0.profile().id().as_str(), "default");
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
