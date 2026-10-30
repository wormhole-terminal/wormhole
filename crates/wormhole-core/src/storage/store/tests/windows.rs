use super::*;
use crate::storage::store::front_last;

#[test]
fn round_trips_windows_sessions_terminals_notes_and_tasks() {
    let dir = temp_dir("round-trip");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = session(1, "/a");
    first.shell = Some("fish".into());
    first.notes_collapsed = true;
    first.notes.push(NoteRecord { body: "remember".into(), collapsed: true });
    first.tasks.push(TaskRecord { text: "ship".into(), done: true });
    let record = WindowRecord {
        id: 3,
        active: 1,
        layout: Some("{\"sidebar_open\":false}".into()),
        bounds: Some("b".into()),
        sessions: vec![first, session(2, "/b")],
        splits: Vec::new(),
    };
    store.save_window(&record).expect("saves");

    let loaded = store.load().expect("loads");
    assert_eq!(loaded.len(), 1);
    let window = &loaded[0];
    assert_eq!((window.id, window.active), (3, 1));
    assert_eq!(window.layout.as_deref(), Some("{\"sidebar_open\":false}"));
    assert_eq!(window.bounds.as_deref(), Some("b"));
    let ids: Vec<i64> = window.sessions.iter().map(|s| s.id).collect();
    assert_eq!(ids, [1, 2]);
    let first = &window.sessions[0];
    assert_eq!(first.name.as_deref(), Some("session 1"));
    assert!(first.notes_collapsed && !window.sessions[1].notes_collapsed);
    assert_eq!(
        (first.cwd.as_deref(), first.shell.as_deref()),
        (Some("/a"), Some("fish")),
        "it keeps its shell"
    );
    assert_eq!((window.sessions[1].cwd.as_deref(), window.sessions[1].shell.as_deref()), (Some("/b"), None));
    assert_eq!((first.notes.len(), first.notes[0].collapsed), (1, true));
    assert_eq!((first.tasks.len(), first.tasks[0].done), (1, true));
}

#[test]
fn saving_keeps_scrollback_and_drops_closed_sessions() {
    let dir = temp_dir("scrollback");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = session(1, "/a");
    store.save_window(&window(1, vec![first.clone(), session(3, "/c")])).expect("saves");
    store.save_scrollback(1, "hello").expect("scrollback");
    store.save_scrollback(3, "closed").expect("scrollback");

    // Saving again must not wipe what was written separately; closing a session removes its rows.
    first.notes.push(NoteRecord { body: "n".into(), collapsed: false });
    store.save_window(&window(1, vec![first])).expect("saves");

    let loaded = store.load().expect("loads");
    let sessions = &loaded[0].sessions;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].scrollback.as_deref(), Some("hello"));
    assert_eq!(sessions[0].notes.len(), 1);
    let orphans: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM tabs WHERE session_id NOT IN (SELECT id FROM sessions)", [], |r| {
            r.get(0)
        })
        .expect("counts");
    assert_eq!(orphans, 0);
}

#[test]
fn windows_save_independently_and_can_be_deleted() {
    let dir = temp_dir("windows");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut a = session(1, "/a");
    a.notes.push(NoteRecord { body: "a".into(), collapsed: false });
    let mut b = session(2, "/b");
    b.tasks.push(TaskRecord { text: "b".into(), done: false });
    store.save_window(&window(1, vec![a])).expect("saves");
    store.save_window(&window(2, vec![b, session(3, "/c")])).expect("saves");

    // Window 1 closing all of its sessions must leave window 2 untouched.
    store.save_window(&window(1, vec![])).expect("saves");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded.iter().map(|w| w.id).collect::<Vec<_>>(), [1, 2]);
    assert!(loaded[0].sessions.is_empty());
    assert_eq!(loaded[1].sessions.len(), 2);
    assert_eq!(loaded[1].sessions[0].tasks.len(), 1);

    store.delete_window(2).expect("deletes");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded.iter().map(|w| w.id).collect::<Vec<_>>(), [1]);
    for table in ["sessions", "tabs", "notes", "tasks"] {
        let rows: i64 =
            store.conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).expect("counts");
        assert_eq!(rows, 0, "{table} should be empty");
    }
}

#[test]
fn window_order_survives_saving_out_of_order() {
    let dir = temp_dir("order");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(5, vec![session(1, "/a")])).expect("saves");
    store.save_window(&window(2, vec![session(2, "/b")])).expect("saves");
    // Saving the first one again keeps its place.
    store.save_window(&window(5, vec![session(1, "/a")])).expect("saves");
    let ids: Vec<i64> = store.load().expect("loads").iter().map(|w| w.id).collect();
    assert_eq!(ids, [5, 2]);
}

#[test]
fn next_ids_clear_every_saved_session_and_window() {
    let dir = temp_dir("ids");
    let mut store = Store::open_at(&dir).expect("opens");
    assert_eq!(store.next_ids().expect("ids"), (1, 1));
    store.save_window(&window(4, vec![session(2, "/a"), session(9, "/b")])).expect("saves");
    assert_eq!(store.next_ids().expect("ids"), (10, 5));
}

#[test]
fn deleting_a_window_leaves_the_others_whole() {
    let dir = temp_dir("delete-keeps");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut keep = session(1, "/keep");
    keep.notes.push(NoteRecord { body: "note".into(), collapsed: false });
    keep.tasks.push(TaskRecord { text: "task".into(), done: false });
    store.save_window(&window(1, vec![keep])).expect("saves");
    store.save_window(&window(2, vec![session(3, "/gone")])).expect("saves");
    store.save_scrollback(1, "first").expect("scrollback");
    store.save_scrollback(3, "gone").expect("scrollback");

    store.delete_window(2).expect("deletes");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded.len(), 1);
    let kept = &loaded[0].sessions[0];
    assert_eq!((kept.notes.len(), kept.tasks.len()), (1, 1));
    assert_eq!(kept.scrollback.as_deref(), Some("first"));
}

#[test]
fn notes_and_tasks_keep_the_order_they_were_put_in() {
    let dir = temp_dir("item-order");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = session(1, "/a");
    first.tasks = ["one", "two", "three"].map(|t| TaskRecord { text: t.into(), done: false }).into();
    first.notes = ["a", "b"].map(|b| NoteRecord { body: b.into(), collapsed: false }).into();
    store.save_window(&window(1, vec![first.clone()])).expect("saves");
    // Reordered by hand (drag and drop), then saved again.
    crate::view::reorder::move_item(&mut first.tasks, 2, 0);
    crate::view::reorder::move_item(&mut first.notes, 0, 1);
    store.save_window(&window(1, vec![first])).expect("saves");

    let loaded = store.load().expect("loads");
    let session = &loaded[0].sessions[0];
    let tasks: Vec<&str> = session.tasks.iter().map(|t| t.text.as_str()).collect();
    let notes: Vec<&str> = session.notes.iter().map(|n| n.body.as_str()).collect();
    assert_eq!(tasks, ["three", "one", "two"]);
    assert_eq!(notes, ["b", "a"]);
}

#[test]
fn blank_notes_and_tasks_are_never_saved_or_loaded() {
    let dir = temp_dir("blank-items");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = session(1, "/a");
    first.tasks =
        vec![TaskRecord { text: "  ".into(), done: false }, TaskRecord { text: "real".into(), done: true }];
    first.notes = vec![
        NoteRecord { body: "kept".into(), collapsed: false },
        NoteRecord { body: "\n \n".into(), collapsed: false },
    ];
    store.save_window(&window(1, vec![first])).expect("saves");
    // One left behind by an older version, which saved them.
    store.conn.execute("INSERT INTO notes VALUES (1, 5, '', 0)", []).expect("inserts");

    let loaded = store.load().expect("loads");
    let session = &loaded[0].sessions[0];
    assert_eq!(session.tasks.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["real"]);
    assert_eq!(session.notes.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(), ["kept"]);
}

#[test]
fn a_session_id_is_unique_across_windows() {
    // Ids come from one counter, so a collision can only be a bug; this pins
    // what happens if one slips through: the row moves, it is never duplicated.
    let dir = temp_dir("collision");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(5, "/a")])).expect("saves");
    store.save_window(&window(2, vec![session(5, "/b")])).expect("saves");
    let loaded = store.load().expect("loads");
    let total: usize = loaded.iter().map(|w| w.sessions.len()).sum();
    assert_eq!(total, 1);
}

#[test]
fn a_pin_is_saved_and_unpinning_is_saved_too() {
    let dir = temp_dir("pinned");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut pinned = session(2, "/b");
    pinned.pinned = true;
    store.save_window(&window(1, vec![session(1, "/a"), pinned])).expect("saves");
    let loaded = store.load().expect("loads");
    let pins: Vec<(i64, bool)> = loaded[0].sessions.iter().map(|s| (s.id, s.pinned)).collect();
    assert_eq!(pins, [(1, false), (2, true)]);

    // Unpinned through the granular save as well.
    let mut unpinned = session(2, "/b");
    unpinned.pinned = false;
    store.save_session(1, &unpinned).expect("saves");
    assert!(store.load().expect("loads")[0].sessions.iter().all(|s| !s.pinned));
}

#[test]
fn each_window_keeps_its_own_groups_and_order() {
    use crate::view::sidebar::WindowSidebar;
    let dir = temp_dir("sidebar-state");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = WindowSidebar::default();
    let docs = first.new_group(&[1, 2], &[2]);
    first.groups.rename(docs, "Docs");
    first.groups.set_pinned(docs, true);
    first.toggle_group_fold(docs);
    let second = WindowSidebar::default();
    // The app saves the sidebar's state inside the window's layout JSON.
    let layout = |state: &WindowSidebar| serde_json::to_string(state).ok();
    store
        .save_window(&WindowRecord { layout: layout(&first), ..window(1, vec![session(1, "/a")]) })
        .expect("saves");
    store
        .save_window(&WindowRecord { layout: layout(&second), ..window(2, vec![session(2, "/b")]) })
        .expect("saves");
    drop(store);

    let store = Store::open_at(&dir).expect("opens again");
    let restored: Vec<WindowSidebar> = store
        .load()
        .expect("loads")
        .iter()
        .map(|w| serde_json::from_str(w.layout.as_deref().unwrap_or("{}")).expect("parses"))
        .collect();
    assert_eq!(restored, [first, second]);
}

#[test]
fn the_front_window_is_remembered_and_opens_last() {
    let dir = temp_dir("front-window");
    let store = Store::open_at(&dir).expect("opens");
    assert_eq!(store.front_window().expect("reads"), None);
    store.save_front_window(2).expect("saves");
    assert_eq!(store.front_window().expect("reads"), Some(2));

    let mut windows = vec![window(1, vec![]), window(2, vec![]), window(3, vec![])];
    front_last(&mut windows, Some(2));
    assert_eq!(windows.iter().map(|w| w.id).collect::<Vec<_>>(), [1, 3, 2]);
    // One that is gone, or none saved, leaves the order as it was.
    front_last(&mut windows, Some(9));
    front_last(&mut windows, None);
    assert_eq!(windows.iter().map(|w| w.id).collect::<Vec<_>>(), [1, 3, 2]);
}
