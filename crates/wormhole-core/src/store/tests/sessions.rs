use super::*;

#[test]
fn saving_what_is_already_saved_writes_nothing() {
    let dir = temp_dir("no-op-save");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = session(1, "/a");
    first.notes = vec![note("a"), note("b")];
    first.tasks = vec![task("t")];
    let mut record = window(1, vec![first, session(2, "/b")]);
    record.layout = Some("{}".into());
    store.save_window(&record).expect("saves");
    let before = changes(&store);
    store.save_window(&record).expect("saves again");
    assert_eq!(changes(&store), before, "an unchanged window is not rewritten");

    // One edited note rewrites that session's notes (2 deleted, 2 inserted) and nothing else.
    record.sessions[0].notes[1].body = "b!".into();
    store.save_window(&record).expect("saves");
    assert_eq!(changes(&store), before + 4);
    let loaded = store.load().expect("loads");
    let bodies: Vec<&str> = loaded[0].sessions[0].notes.iter().map(|n| n.body.as_str()).collect();
    assert_eq!(bodies, ["a", "b!"]);
    assert_eq!(loaded[0].sessions[0].tasks.len(), 1);
}

#[test]
fn save_session_touches_only_that_session() {
    let dir = temp_dir("save-session");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut sessions: Vec<SessionRecord> = (1..=3).map(|id| session(id, "/w")).collect();
    for (i, c) in sessions.iter_mut().enumerate() {
        c.notes = vec![note(&format!("note {i}a")), note(&format!("note {i}b"))];
        c.tasks = vec![task(&format!("task {i}"))];
    }
    let mut record = window(1, sessions);
    store.save_window(&record).expect("saves");
    let rows_of = |store: &Store, id: i64| -> Vec<i64> {
        let mut stmt = store
            .conn
            .prepare("SELECT rowid FROM notes WHERE session_id = ? ORDER BY rowid")
            .expect("prepare");
        stmt.query_map([id], |r| r.get(0)).expect("query").collect::<Result<_, _>>().expect("rows")
    };
    let others = (rows_of(&store, 1), rows_of(&store, 3));

    // The window row and the other sessions are not part of this save, even if they changed in the
    // record: only session 2 is written.
    record.layout = Some("not saved".into());
    record.sessions[0].notes[0].body = "not saved".into();
    record.sessions[1].notes[0].body = "edited".into();
    let before = changes(&store);
    store.save_session(1, &record.sessions[1]).expect("saves");
    assert_eq!(changes(&store) - before, 4, "session 2's two notes deleted and inserted again");
    assert_eq!((rows_of(&store, 1), rows_of(&store, 3)), others);

    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].layout, None);
    assert_eq!(loaded[0].sessions[0].notes[0].body, "note 0a");
    assert_eq!(loaded[0].sessions[1].notes[0].body, "edited");
    assert_eq!(loaded[0].sessions.iter().map(|s| s.id).collect::<Vec<_>>(), [1, 2, 3]);
}

#[test]
fn save_session_places_new_sessions_last_and_keeps_old_places() {
    let dir = temp_dir("save-session-order");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    store.save_window(&window(2, vec![session(7, "/other")])).expect("saves");

    store.save_session(1, &session(3, "/c")).expect("saves");
    store.save_session(1, &session(1, "/a2")).expect("saves"); // an old session keeps its place
    let order = |store: &Store, w: usize| -> Vec<i64> {
        store.load().expect("loads")[w].sessions.iter().map(|s| s.id).collect()
    };
    assert_eq!(order(&store, 0), [1, 2, 3]);
    assert_eq!(order(&store, 1), [7]);

    store.order_sessions(1, &[3, 1, 2]).expect("orders");
    assert_eq!(order(&store, 0), [3, 1, 2]);
    // Another window's session is not moved by naming it.
    store.order_sessions(1, &[7, 3, 1, 2]).expect("orders");
    assert_eq!(order(&store, 1), [7]);
    assert_eq!(order(&store, 0), [3, 1, 2]);
}

#[test]
fn save_session_updates_the_terminal_and_keeps_its_scrollback() {
    let dir = temp_dir("save-session-terminal");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut first = session(1, "/a");
    first.shell = Some("fish".into());
    store.save_window(&window(1, vec![first.clone()])).expect("saves");
    store.save_scrollbacks(&[(1, "one")]).expect("saves");

    first.cwd = Some("/a/sub".into());
    first.shell = None;
    store.save_session(1, &first).expect("saves");
    let loaded = store.load().expect("loads");
    let saved = &loaded[0].sessions[0];
    assert_eq!(
        (saved.cwd.as_deref(), saved.shell.as_deref(), saved.scrollback.as_deref()),
        (Some("/a/sub"), None, Some("one"))
    );
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs WHERE session_id = 1"), 1);
}

#[test]
fn delete_sessions_except_forgets_closed_sessions_in_that_window_only() {
    let dir = temp_dir("delete-sessions");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut closing = session(2, "/b");
    closing.notes = vec![note("n")];
    closing.tasks = vec![task("t")];
    store.save_window(&window(1, vec![session(1, "/a"), closing, session(3, "/c")])).expect("saves");
    store.save_window(&window(2, vec![session(4, "/d")])).expect("saves");
    store.save_scrollbacks(&[(2, "gone"), (4, "stays")]).expect("saves");

    store.delete_sessions_except(1, &[1, 3, 4]).expect("deletes");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions.iter().map(|s| s.id).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(loaded[1].sessions.iter().map(|s| s.id).collect::<Vec<_>>(), [4], "4 is not window 1's");
    assert_eq!(loaded[1].sessions[0].scrollback.as_deref(), Some("stays"));
    for table in ["notes", "tasks"] {
        assert_eq!(
            count(&store, &format!("SELECT COUNT(*) FROM {table} WHERE session_id = 2")),
            0,
            "{table}"
        );
    }
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs WHERE session_id = 2"), 0);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback WHERE tab_id = 2"), 0);
}

#[test]
fn save_window_meta_writes_the_window_row_only() {
    let dir = temp_dir("window-meta");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    let mut record = window(1, vec![session(5, "/ignored")]);
    record.active = 2;
    record.layout = Some("{\"a\":1}".into());
    record.bounds = Some("b".into());
    store.save_window_meta(&record).expect("saves");
    let loaded = store.load().expect("loads");
    assert_eq!(
        (loaded[0].active, loaded[0].layout.as_deref(), loaded[0].bounds.as_deref()),
        (2, Some("{\"a\":1}"), Some("b"))
    );
    assert_eq!(loaded[0].sessions.iter().map(|s| s.id).collect::<Vec<_>>(), [1]);
    // A window the store has not seen is created (last), as `save_window` does.
    store.save_window_meta(&window(2, vec![])).expect("saves");
    assert_eq!(store.load().expect("loads").iter().map(|w| w.id).collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn a_terminals_host_id_is_saved_loaded_and_cleared() {
    let dir = temp_dir("host-id");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut hosted = session(1, "/a");
    hosted.host_id = Some("abcdefghij12".into());
    store.save_window(&window(1, vec![hosted.clone(), session(2, "/b")])).expect("saves");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions[0].host_id.as_deref(), Some("abcdefghij12"));
    assert_eq!(loaded[0].sessions[1].host_id, None);
    assert_eq!(store.host_ids().expect("ids"), [(1, "abcdefghij12".to_string())]);

    // Saving the same record writes nothing.
    let before = changes(&store);
    store.save_window(&window(1, vec![hosted.clone(), session(2, "/b")])).expect("saves");
    assert_eq!(changes(&store), before);

    // A session that ends its host is saved without one.
    hosted.host_id = None;
    store.save_session(1, &hosted).expect("saves");
    assert_eq!(store.load().expect("loads")[0].sessions[0].host_id, None);
    assert!(store.host_ids().expect("ids").is_empty());
}

#[test]
fn a_new_hosted_session_is_on_disk_as_soon_as_save_session_host_returns() {
    let dir = temp_dir("host-id-now");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    store.save_session_host(1, &session(2, "/b"), "zzzzzzzzzzzz").expect("saves");
    // A second connection (what the next launch sees) finds the row without any window save.
    let again = Store::open_at(&dir).expect("opens again");
    let loaded = again.load().expect("loads");
    assert_eq!(loaded[0].sessions.len(), 2);
    assert_eq!(loaded[0].sessions[1].id, 2, "goes last in the window");
    assert_eq!(loaded[0].sessions[1].host_id.as_deref(), Some("zzzzzzzzzzzz"));
}
