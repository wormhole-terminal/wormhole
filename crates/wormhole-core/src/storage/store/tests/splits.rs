//! A terminal is a session, and a split groups several of a window's sessions under one layout.

use super::*;
use crate::store::SplitRecord;

/// A session of one terminal, as the app keeps them: no tabs.
fn terminal(id: i64, cwd: &str, split_id: Option<i64>) -> SessionRecord {
    SessionRecord {
        id,
        name: Some(format!("terminal {id}")),
        cwd: Some(cwd.into()),
        split_id,
        ..Default::default()
    }
}

fn split(id: i64, layout: &str) -> SplitRecord {
    SplitRecord { id, layout: Some(layout.into()) }
}

fn split_window(id: i64, sessions: Vec<SessionRecord>, splits: Vec<SplitRecord>) -> WindowRecord {
    WindowRecord { id, sessions, splits, ..Default::default() }
}

#[test]
fn a_split_and_its_members_round_trip_with_their_terminals() {
    let dir = temp_dir("split-round-trip");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut second = terminal(2, "/b", Some(9));
    second.shell = Some("fish".into());
    second.pinned = true;
    let window = split_window(
        1,
        vec![terminal(1, "/a", Some(9)), second, terminal(3, "/c", None)],
        vec![split(9, "{\"focused\":2}")],
    );
    store.save_window(&window).expect("saves");
    store.save_scrollbacks(&[(1, "one"), (2, "two"), (3, "three")]).expect("scrollback");

    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].splits.len(), 1);
    assert_eq!((loaded[0].splits[0].id, loaded[0].splits[0].layout.as_deref()), (9, Some("{\"focused\":2}")));
    let sessions = &loaded[0].sessions;
    assert_eq!(sessions.iter().map(|s| s.id).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(sessions.iter().map(|s| s.split_id).collect::<Vec<_>>(), [Some(9), Some(9), None]);
    assert_eq!(sessions[1].shell.as_deref(), Some("fish"));
    assert!(sessions[1].pinned);
    assert_eq!(
        sessions.iter().map(|s| (s.cwd.as_deref(), s.scrollback.as_deref())).collect::<Vec<_>>(),
        [(Some("/a"), Some("one")), (Some("/b"), Some("two")), (Some("/c"), Some("three"))]
    );
    // One row per terminal, with the session's id.
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs WHERE id = session_id AND id = tab_id"), 3);

    // Saved as loaded, nothing is written, a layout that changed is the only write.
    let before = changes(&store);
    let mut again = loaded[0].clone();
    store.save_window(&again).expect("saves");
    assert_eq!(changes(&store), before);
    again.splits[0].layout = Some("{\"focused\":1}".into());
    store.save_window(&again).expect("saves");
    assert_eq!(changes(&store), before + 1);
    assert_eq!(store.load().expect("loads")[0].splits[0].layout.as_deref(), Some("{\"focused\":1}"));
}

#[test]
fn a_split_without_members_is_not_kept() {
    let dir = temp_dir("split-empty");
    let mut store = Store::open_at(&dir).expect("opens");
    // Listed, but nobody names it.
    store.save_window(&split_window(1, vec![terminal(1, "/a", None)], vec![split(9, "{}")])).expect("saves");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 0);

    // Its members leave one by one: with the last the split goes.
    let both =
        split_window(1, vec![terminal(1, "/a", Some(9)), terminal(2, "/b", Some(9))], vec![split(9, "{}")]);
    store.save_window(&both).expect("saves");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 1);
    store
        .save_window(&split_window(1, vec![terminal(1, "/a", None), terminal(2, "/b", None)], vec![]))
        .expect("saves");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 0);
    assert_eq!(store.load().expect("loads")[0].sessions[0].split_id, None);

    // Closing the sessions does the same.
    store.save_window(&both).expect("saves");
    store.delete_sessions_except(1, &[1]).expect("closes");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 1, "one member is left");
    store.delete_sessions_except(1, &[]).expect("closes");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 0);
}

#[test]
fn windows_keep_their_splits_apart_and_deleting_one_forgets_its_own() {
    let dir = temp_dir("split-windows");
    let mut store = Store::open_at(&dir).expect("opens");
    let one = split_window(
        1,
        vec![terminal(1, "/a", Some(7)), terminal(2, "/b", Some(7))],
        vec![split(7, "{\"w\":1}")],
    );
    let two = split_window(
        2,
        vec![terminal(3, "/c", Some(8)), terminal(4, "/d", Some(8))],
        vec![split(8, "{\"w\":2}")],
    );
    store.save_window(&one).expect("saves");
    store.save_window(&two).expect("saves");
    // Saving a window whose splits are empty leaves the other's alone.
    store.save_window(&split_window(1, vec![], vec![])).expect("saves");
    let loaded = store.load().expect("loads");
    assert!(loaded[0].splits.is_empty());
    assert_eq!(loaded[1].splits.iter().map(|s| s.id).collect::<Vec<_>>(), [8]);

    store.save_window(&one).expect("saves");
    store.delete_window(2).expect("deletes");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].splits.iter().map(|s| s.id).collect::<Vec<_>>(), [7]);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 1);
}

#[test]
fn saving_one_terminal_names_its_split_and_drops_the_rows_of_its_old_tabs() {
    let dir = temp_dir("split-save-session");
    let mut store = Store::open_at(&dir).expect("opens");
    // A session as it was with tabs: a row of its own besides the terminal's.
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    store
        .conn
        .execute("INSERT INTO tabs (id, session_id, tab_id, position, cwd) VALUES (2, 1, 2, 1, '/old')", [])
        .expect("an old tab row");
    store.save_scrollbacks(&[(1, "one"), (2, "old")]).expect("scrollback");

    store.save_session(1, &terminal(1, "/a", Some(5))).expect("saves");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs WHERE session_id = 1"), 1);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback"), 1, "the other pane's text goes with it");
    assert_eq!(count(&store, "SELECT split_id FROM sessions WHERE id = 1"), 5);
    let kept = &store.load().expect("loads")[0].sessions[0];
    assert_eq!(kept.scrollback.as_deref(), Some("one"));
}

#[test]
fn ids_for_new_sessions_clear_the_splits_too() {
    let dir = temp_dir("split-ids");
    let mut store = Store::open_at(&dir).expect("opens");
    store
        .save_window(&split_window(
            4,
            vec![terminal(1, "/a", Some(50)), terminal(2, "/b", Some(50))],
            vec![split(50, "{}")],
        ))
        .expect("saves");
    assert_eq!(store.next_ids().expect("ids"), (51, 5));
}

#[test]
fn a_database_with_a_split_nobody_is_in_or_a_member_of_a_split_that_is_gone_is_repaired_on_open() {
    let dir = temp_dir("split-repair");
    {
        let store = Store::open_at(&dir).expect("opens");
        store
            .conn
            .execute_batch(
                "INSERT INTO windows (id, position) VALUES (1, 0);
                 INSERT INTO sessions (id, window_id, position, split_id) VALUES (1, 1, 0, 8), (2, 1, 1, 9);
                 INSERT INTO splits (id, window_id, layout) VALUES (9, 1, '{}'), (10, 1, '{}');",
            )
            .expect("inserts");
    }
    let store = Store::open_at(&dir).expect("repairs");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions.iter().map(|s| s.split_id).collect::<Vec<_>>(), [None, Some(9)]);
    assert_eq!(loaded[0].splits.iter().map(|s| s.id).collect::<Vec<_>>(), [9]);
}
