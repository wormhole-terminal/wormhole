//! What the step from schema 6 to 7 does to the data (`steps::V7_EVERY_TERMINAL_A_SESSION`): every
//! pane becomes a session next to its parent, a tab with several panes becomes a split, and
//! nothing a session holds (name, flags, notes, tasks, scrollback) goes missing or to another one.
//!
//! The fixture is a version 6 database as the app wrote it, with the shapes closing and moving
//! panes leave behind: a session's first pane that closed (its id is no pane's any more) and one
//! that moved to a tab of its own (the session's id is a pane id, but not the first's).

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use super::migrations::{empty_v6_database, version};
use super::schema_snapshot::dump;
use super::*;

/// A tab's layout as the app wrote it: a row of panes (`Tree`'s JSON with `TabPanes`' focus and
/// zoom), the shares equal.
fn row_layout(ids: &[i64], focused: i64, zoomed: Option<i64>) -> String {
    let share = 1.0 / ids.len() as f64;
    let leaves: Vec<String> =
        ids.iter().map(|id| format!(r#"{{"ratio":{share},"node":{{"leaf":{id}}}}}"#)).collect();
    let zoomed = zoomed.map_or("null".to_string(), |id| id.to_string());
    format!(
        r#"{{"tree":{{"root":{{"split":{{"axis":"row","children":[{}]}}}}}},"focused":{focused},"zoomed":{zoomed}}}"#,
        leaves.join(",")
    )
}

/// Pane 10 on the left, 11 above 12 on the right; 11 has the keyboard.
const ALPHA_LAYOUT: &str = r#"{"tree":{"root":{"split":{"axis":"row","children":[{"ratio":0.5,"node":{"leaf":10}},{"ratio":0.5,"node":{"split":{"axis":"column","children":[{"ratio":0.5,"node":{"leaf":11}},{"ratio":0.5,"node":{"leaf":12}}]}}}]}}},"focused":11,"zoomed":null}"#;

/// The panes of a layout, in reading order.
fn leaves(layout: &str) -> Vec<i64> {
    fn walk(node: &Value, out: &mut Vec<i64>) {
        if let Some(id) = node.get("leaf") {
            out.extend(id.as_i64());
        } else if let Some(children) = node["split"]["children"].as_array() {
            for child in children {
                walk(&child["node"], out);
            }
        }
    }
    let json: Value = serde_json::from_str(layout).expect("a layout is JSON");
    let mut out = Vec::new();
    walk(&json["tree"]["root"], &mut out);
    out
}

fn focus(layout: &str, key: &str) -> Option<i64> {
    let json: Value = serde_json::from_str(layout).expect("a layout is JSON");
    json[key].as_i64()
}

fn add_window(conn: &Connection, id: i64, position: i64, active: i64) {
    conn.execute(
        "INSERT INTO windows (id, position, active, layout) VALUES (?1, ?2, ?3, ?4)",
        (id, position, active, format!("{{\"window\":{id}}}")),
    )
    .expect("window");
}

fn add_session(conn: &Connection, id: i64, window: i64, position: i64, name: &str, active_tab: i64) {
    conn.execute(
        "INSERT INTO sessions (id, window_id, position, name, active_tab) VALUES (?1, ?2, ?3, ?4, ?5)",
        (id, window, position, name, active_tab),
    )
    .expect("session");
}

/// A pane's row, with its scrollback.
fn add_pane(
    conn: &Connection,
    id: i64,
    session: i64,
    tab: i64,
    position: i64,
    cwd: &str,
    layout: Option<&str>,
) {
    conn.execute(
        "INSERT INTO tabs (id, session_id, tab_id, position, cwd, layout) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (id, session, tab, position, cwd, layout),
    )
    .expect("pane");
    conn.execute(
        "INSERT INTO scrollback (tab_id, hash, text) VALUES (?1, ?1, ?2)",
        (id, format!("out-{id}")),
    )
    .expect("scrollback");
}

/// Four windows (`W1` to `W4`, the sessions of each in the order of their positions, which have
/// gaps as they do after closing sessions):
///
/// - W1, selected session 0: *alpha* (10: a tab of three panes, 10 to 12, laid out as
///   `ALPHA_LAYOUT`, and a tab of one, 13; pinned, with notes, a task and both sections folded),
///   *beta* (20: two tabs of one pane each), *gamma* (30: its first tab, 31 and 32 side by side,
///   31 zoomed, was made with a pane that closed: no pane is 30; pinned, a note, a task).
/// - W2, selected session 0 (tab 1): *delta* (40: its first tab holds 41 and 42, its second pane 40,
///   which moved out of the first tab; a note and a task), *epsilon* (50).
/// - W3, selected session 1: *zeta* (60 and 61, two tabs), *eta* (70 and 71, whose layout says the
///   keyboard is in pane 99, which is gone).
/// - W4, selected session 0: *theta* (90: panes 91 and 90 in one tab, in that order, 90 focused,
///   91 zoomed).
///
/// Every pane has the scrollback `out-<id>`.
fn v6_fixture(dir: &Path) {
    let conn = empty_v6_database(dir);
    for (id, position, active) in [(1, 0, 0), (2, 1, 0), (3, 2, 1), (4, 3, 0)] {
        add_window(&conn, id, position, active);
    }
    add_session(&conn, 10, 1, 0, "alpha", 0);
    add_session(&conn, 20, 1, 3, "beta", 0);
    add_session(&conn, 30, 1, 7, "gamma", 0);
    add_session(&conn, 40, 2, 0, "delta", 1);
    add_session(&conn, 50, 2, 5, "epsilon", 0);
    add_session(&conn, 60, 3, 0, "zeta", 0);
    add_session(&conn, 70, 3, 1, "eta", 0);
    add_session(&conn, 90, 4, 0, "theta", 0);
    conn.execute_batch(
        "UPDATE sessions SET pinned = 1 WHERE id IN (10, 30);
         UPDATE sessions SET tasks_collapsed = 1, notes_collapsed = 1 WHERE id = 10;",
    )
    .expect("flags");

    add_pane(&conn, 10, 10, 10, 0, "/a", Some(ALPHA_LAYOUT));
    add_pane(&conn, 11, 10, 10, 1, "/a/l", None);
    add_pane(&conn, 12, 10, 10, 2, "/a/r", None);
    add_pane(&conn, 13, 10, 13, 3, "/a2", None);
    conn.execute("UPDATE tabs SET shell = 'fish' WHERE id = 11", []).expect("shell");
    add_pane(&conn, 20, 20, 20, 0, "/b", None);
    add_pane(&conn, 21, 20, 21, 1, "/b2", None);
    add_pane(&conn, 31, 30, 31, 0, "/g", Some(&row_layout(&[31, 32], 32, Some(31))));
    add_pane(&conn, 32, 30, 31, 1, "/g/r", None);
    add_pane(&conn, 41, 40, 40, 0, "/d", Some(&row_layout(&[41, 42], 42, None)));
    add_pane(&conn, 42, 40, 40, 1, "/d/r", None);
    add_pane(&conn, 40, 40, 43, 2, "/d/moved", None);
    add_pane(&conn, 50, 50, 50, 0, "/e", None);
    add_pane(&conn, 60, 60, 60, 0, "/z", None);
    add_pane(&conn, 61, 60, 61, 1, "/z2", None);
    add_pane(&conn, 70, 70, 70, 0, "/h", Some(&row_layout(&[70, 71], 99, None)));
    add_pane(&conn, 71, 70, 70, 1, "/h/r", None);
    add_pane(&conn, 91, 90, 90, 0, "/t", Some(&row_layout(&[91, 90], 90, Some(91))));
    add_pane(&conn, 90, 90, 90, 1, "/t/r", None);

    conn.execute_batch(
        "INSERT INTO notes (session_id, position, body, collapsed) VALUES
             (10, 0, 'a1', 1), (10, 1, 'a2', 0), (20, 0, 'beta note', 0), (30, 0, 'gamma note', 0),
             (40, 0, 'delta note', 0);
         INSERT INTO tasks (session_id, position, text, done) VALUES
             (10, 0, 't1', 1), (30, 0, 'gamma task', 0), (40, 0, 'delta task', 0);",
    )
    .expect("notes and tasks");
}

fn migrated(name: &str) -> Store {
    let dir = temp_dir(name);
    v6_fixture(&dir);
    let store = Store::open_at(&dir).expect("upgrades");
    assert_eq!(version(&store.conn), 13);
    assert!(store.recovered_from().is_none(), "upgraded, not moved aside");
    store
}

fn window_of(windows: &[WindowRecord], id: i64) -> &WindowRecord {
    windows.iter().find(|window| window.id == id).expect("the window")
}

fn ids(window: &WindowRecord) -> Vec<i64> {
    window.sessions.iter().map(|session| session.id).collect()
}

fn session_of(windows: &[WindowRecord], id: i64) -> &SessionRecord {
    windows.iter().flat_map(|window| &window.sessions).find(|session| session.id == id).expect("the session")
}

#[test]
fn every_pane_becomes_a_session_right_after_its_parent() {
    let store = migrated("v7-rows");
    let windows = store.load().expect("loads");
    assert_eq!(ids(window_of(&windows, 1)), [10, 11, 12, 13, 20, 21, 30, 32]);
    // The first pane of delta's first tab is session 40 now, and the pane that was 40 is 41.
    assert_eq!(ids(window_of(&windows, 2)), [40, 42, 41, 50]);
    assert_eq!(ids(window_of(&windows, 3)), [60, 61, 70, 71]);
    assert_eq!(ids(window_of(&windows, 4)), [90, 91]);

    let folder = |id| session_of(&windows, id).cwd.clone().expect("a folder");
    let by_id: Vec<String> = [10, 11, 12, 13, 20, 21, 30, 32, 40, 42, 41, 50, 90, 91].map(folder).to_vec();
    assert_eq!(
        by_id,
        [
            "/a", "/a/l", "/a/r", "/a2", "/b", "/b2", "/g", "/g/r", // as they were
            "/d", "/d/r", "/d/moved", "/e", // 41 is 40 now, and the other way round
            "/t", "/t/r", // 91 is 90 now
        ]
    );
    assert_eq!(session_of(&windows, 11).shell.as_deref(), Some("fish"), "a pane keeps its shell");

    // The names stay with the first pane; the others are unnamed and the app names them.
    let names: Vec<Option<&str>> =
        [10, 11, 12, 13, 20, 21, 30, 32].map(|id| session_of(&windows, id).name.as_deref()).to_vec();
    assert_eq!(
        names,
        [Some("alpha"), None, None, None, Some("beta"), None, Some("gamma"), None],
        "gamma is session 30 though its first pane was 31"
    );
    assert_eq!(session_of(&windows, 40).name.as_deref(), Some("delta"));
    assert_eq!(session_of(&windows, 41).name, None);

    // Positions run 0.. in each window with no gaps, and the loaded order is theirs.
    for window in &windows {
        let positions: Vec<i64> = store
            .conn
            .prepare("SELECT position FROM sessions WHERE window_id = ? ORDER BY position")
            .expect("prepares")
            .query_map([window.id], |row| row.get(0))
            .expect("queries")
            .collect::<rusqlite::Result<_>>()
            .expect("rows");
        assert_eq!(positions, (0..window.sessions.len() as i64).collect::<Vec<_>>(), "window {}", window.id);
    }

    // A session is one terminal: its row has its id, and nothing of the tab it was in.
    assert_eq!(count(&store, "SELECT COUNT(*) FROM sessions"), 18);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs"), 18);
    assert_eq!(
        count(
            &store,
            "SELECT COUNT(*) FROM tabs t JOIN sessions s ON s.id = t.id
             WHERE t.session_id = t.id AND t.tab_id = t.id AND t.position = 0 AND t.layout IS NULL
               AND s.active_tab = 0"
        ),
        18
    );
}

#[test]
fn a_tab_of_several_panes_becomes_a_split_and_several_tabs_stay_apart() {
    let store = migrated("v7-splits");
    let windows = store.load().expect("loads");
    let split = |id| session_of(&windows, id).split_id;

    let alpha = split(10).expect("alpha's tab is a split");
    assert_eq!([split(11), split(12)], [Some(alpha), Some(alpha)]);
    assert_eq!(split(13), None, "the session's other tab is not part of it");
    assert_eq!([split(20), split(21)], [None, None], "two tabs of one pane make two sessions, no split");
    let gamma = split(30).expect("gamma's");
    assert_eq!(split(32), Some(gamma));
    let delta = split(40).expect("delta's");
    assert_eq!(split(42), Some(delta));
    assert_eq!(split(41), None, "the pane that moved to a tab of its own");
    assert_eq!(split(50), None);
    let eta = split(70).expect("eta's");
    assert_eq!(split(71), Some(eta));
    let theta = split(90).expect("theta's");
    assert_eq!(split(91), Some(theta));

    // Each window holds its own splits; the ids are all different, and no session's.
    let of_window = |id| window_of(&windows, id).splits.iter().map(|s| s.id).collect::<Vec<_>>();
    assert_eq!(of_window(1), [alpha, gamma]);
    assert_eq!(of_window(2), [delta]);
    assert_eq!(of_window(3), [eta]);
    assert_eq!(of_window(4), [theta]);
    let all = [alpha, gamma, delta, eta, theta];
    assert_eq!(all.iter().collect::<std::collections::HashSet<_>>().len(), 5);
    assert!(all.iter().all(|id| *id > 91), "past the highest session and pane id: {all:?}");
    assert_eq!(store.next_ids().expect("ids"), (all.into_iter().max().expect("some") + 1, 5));
    assert_eq!(count(&store, "SELECT COUNT(*) FROM splits"), 5);
}

#[test]
fn a_splits_layout_is_the_tabs_with_the_panes_new_ids() {
    let store = migrated("v7-layouts");
    let windows = store.load().expect("loads");
    let layout = |window, member| {
        let id = session_of(&windows, member).split_id.expect("in a split");
        let split = window_of(&windows, window).splits.iter().find(|s| s.id == id).expect("the split");
        split.layout.clone().expect("a layout")
    };

    // Nothing changes id in alpha: the layout is copied as it was.
    assert_eq!(layout(1, 10), ALPHA_LAYOUT);
    // Gamma's first pane is 30 now; the focus and the zoom follow.
    let gamma = layout(1, 30);
    assert_eq!(leaves(&gamma), [30, 32]);
    assert_eq!((focus(&gamma, "focused"), focus(&gamma, "zoomed")), (Some(32), Some(30)));
    // Delta's first pane is 40 now (a pane that is 41 now is not in this tab).
    let delta = layout(2, 40);
    assert_eq!(leaves(&delta), [40, 42]);
    assert_eq!((focus(&delta, "focused"), focus(&delta, "zoomed")), (Some(42), None));
    // A focus on a pane that is gone is left for the app to repair.
    let eta = layout(3, 70);
    assert_eq!((leaves(&eta), focus(&eta, "focused")), (vec![70, 71], Some(99)));
    // Theta's panes swap ids in one layout: 91 (first, zoomed) is 90 now, 90 (focused) is 91.
    let theta = layout(4, 90);
    assert_eq!(leaves(&theta), [90, 91]);
    assert_eq!((focus(&theta, "focused"), focus(&theta, "zoomed")), (Some(91), Some(90)));
    // Only the ids were touched: the rest of the text is what it was.
    assert_eq!(theta, row_layout(&[90, 91], 91, Some(90)));
}

#[test]
fn notes_tasks_and_flags_stay_with_the_sessions_first_pane() {
    let store = migrated("v7-notes");
    let windows = store.load().expect("loads");
    let notes = |id| -> Vec<(String, bool)> {
        session_of(&windows, id).notes.iter().map(|n| (n.body.clone(), n.collapsed)).collect()
    };
    let tasks = |id| -> Vec<(String, bool)> {
        session_of(&windows, id).tasks.iter().map(|t| (t.text.clone(), t.done)).collect()
    };
    assert_eq!(notes(10), [("a1".to_string(), true), ("a2".to_string(), false)]);
    assert_eq!(tasks(10), [("t1".to_string(), true)]);
    let alpha = session_of(&windows, 10);
    assert!(alpha.tasks_collapsed && alpha.notes_collapsed);
    assert_eq!(notes(20), [("beta note".to_string(), false)]);
    // Gamma's notes are under 30, its id, though its first pane was 31.
    assert_eq!(notes(30), [("gamma note".to_string(), false)]);
    assert_eq!(tasks(30), [("gamma task".to_string(), false)]);
    // Delta's are on its first pane, and not on the pane that used to have its id.
    assert_eq!(notes(40), [("delta note".to_string(), false)]);
    assert_eq!(tasks(40), [("delta task".to_string(), false)]);

    for id in [11, 12, 13, 21, 32, 42, 41, 61, 71, 91] {
        let session = session_of(&windows, id);
        assert!(
            session.notes.is_empty() && session.tasks.is_empty(),
            "session {id} starts with nothing of its own"
        );
        assert!(!session.tasks_collapsed && !session.notes_collapsed, "session {id}");
    }
    // None were lost, and none went to a session that isn't there.
    assert_eq!(count(&store, "SELECT COUNT(*) FROM notes"), 5);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tasks"), 3);
    assert_eq!(
        count(&store, "SELECT COUNT(*) FROM notes WHERE session_id NOT IN (SELECT id FROM sessions)"),
        0
    );
    assert_eq!(
        count(&store, "SELECT COUNT(*) FROM tasks WHERE session_id NOT IN (SELECT id FROM sessions)"),
        0
    );
}

#[test]
fn a_pinned_session_pins_every_pane_that_came_out_of_it() {
    let store = migrated("v7-pinned");
    let windows = store.load().expect("loads");
    let pinned: HashMap<i64, bool> = windows
        .iter()
        .flat_map(|window| &window.sessions)
        .map(|session| (session.id, session.pinned))
        .collect();
    for id in [10, 11, 12, 13, 30, 32] {
        assert!(pinned[&id], "{id} came out of a pinned session");
    }
    for id in [20, 21, 40, 42, 41, 50, 60, 61, 70, 71, 90, 91] {
        assert!(!pinned[&id], "{id} came out of one that wasn't");
    }
}

#[test]
fn scrollback_follows_its_pane_to_the_pane_s_new_id() {
    let store = migrated("v7-scrollback");
    let windows = store.load().expect("loads");
    let text = |id| session_of(&windows, id).scrollback.clone().expect("scrollback");
    for id in [10, 11, 12, 13, 20, 21, 32, 42, 50, 60, 61, 70, 71] {
        assert_eq!(text(id), format!("out-{id}"));
    }
    assert_eq!(text(30), "out-31", "gamma's first pane was 31");
    assert_eq!((text(40).as_str(), text(41).as_str()), ("out-41", "out-40"), "delta's two panes swapped ids");
    assert_eq!((text(90).as_str(), text(91).as_str()), ("out-91", "out-90"));
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback"), 18);
    // The terminal that has the text is the one the session says: the cwd comes with it.
    assert_eq!(session_of(&windows, 41).cwd.as_deref(), Some("/d/moved"));

    let broken =
        count(&store, "SELECT COUNT(*) FROM saved.scrollback WHERE tab_id NOT IN (SELECT id FROM tabs)");
    assert_eq!(broken, 0, "scrollback rows without a pane");
}

#[test]
fn the_selected_session_is_the_row_of_the_old_selected_tabs_focused_pane() {
    let store = migrated("v7-active");
    let windows = store.load().expect("loads");
    let selected = |id| {
        let window = window_of(&windows, id);
        window.sessions[window.active].id
    };
    // W1: alpha's tab 0, whose layout focuses pane 11.
    assert_eq!(window_of(&windows, 1).active, 1);
    assert_eq!(selected(1), 11);
    // W2: delta's tab 1 holds the pane that was 40, which is session 41 now.
    assert_eq!(window_of(&windows, 2).active, 2);
    assert_eq!(selected(2), 41);
    // W3: eta's keyboard was in a pane that is gone: its tab's first pane.
    assert_eq!(window_of(&windows, 3).active, 2);
    assert_eq!(selected(3), 70);
    // W4: theta's focus was on the pane that is 91 now.
    assert_eq!(selected(4), 91);
    // The rest of the window row is as it was.
    assert_eq!(window_of(&windows, 3).layout.as_deref(), Some("{\"window\":3}"));
}

#[test]
fn the_upgraded_database_is_exactly_the_fresh_layout() {
    let store = migrated("v7-layout");
    let fresh = Store::open_at(&temp_dir("v7-fresh")).expect("opens");
    assert_eq!(dump(&store.conn), dump(&fresh.conn));
}

/// What the app does with an upgraded database: it loads each terminal as a session and saves it
/// back, and that writes no row.
#[test]
fn an_upgraded_database_saves_back_as_it_is() {
    let mut store = migrated("v7-save-back");
    let windows = store.load().expect("loads");
    let before = changes(&store);
    for window in &windows {
        store.save_window(window).expect("saves as loaded");
    }
    assert_eq!(changes(&store), before, "as loaded");
    let again = store.load().expect("loads");
    assert_eq!(again[0].splits.len(), 2, "splits are as they were");
    assert_eq!(ids(&again[1]), [40, 42, 41, 50]);
}

/// Odd rows a version 6 database can hold (`repair` made sure of the first on every open): a
/// session with no pane gets one, and a pane of a session that isn't there is left alone.
#[test]
fn odd_rows_are_repaired_or_left_alone() {
    let dir = temp_dir("v7-odd");
    let conn = empty_v6_database(&dir);
    add_window(&conn, 1, 0, 0);
    add_session(&conn, 5, 1, 0, "no pane", 0);
    add_session(&conn, 6, 1, 1, "plain", 0);
    add_pane(&conn, 6, 6, 6, 0, "/p", None);
    add_pane(&conn, 77, 99, 77, 0, "/orphan", None);
    // A window that names a selected session beyond the end.
    add_window(&conn, 2, 1, 7);
    add_session(&conn, 8, 2, 0, "alone", 4);
    add_pane(&conn, 8, 8, 8, 0, "/alone", None);
    // A layout that isn't JSON.
    add_session(&conn, 9, 2, 1, "broken", 0);
    add_pane(&conn, 9, 9, 9, 0, "/broken", Some("not json"));
    add_pane(&conn, 19, 9, 9, 1, "/broken/r", None);
    drop(conn);

    let store = Store::open_at(&dir).expect("upgrades");
    let windows = store.load().expect("loads");
    assert_eq!(ids(window_of(&windows, 1)), [5, 6]);
    assert_eq!(session_of(&windows, 5).name.as_deref(), Some("no pane"), "it has a terminal now");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs WHERE id = 5 AND session_id = 5"), 1);
    assert_eq!(
        count(&store, "SELECT COUNT(*) FROM tabs WHERE id = 77 AND session_id = 99"),
        1,
        "a pane nobody owns is not made into anything, and not deleted"
    );
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback WHERE tab_id = 77"), 1);
    // Beyond the end is the last session's selected pane; a layout that can't be read has no focus.
    let second = window_of(&windows, 2);
    assert_eq!(ids(second), [8, 9, 19]);
    assert_eq!(second.active, 1, "the last session's first pane");
    assert_eq!(second.splits.len(), 1);
    assert_eq!(second.splits[0].layout.as_deref(), Some("not json"), "kept as it was, the app repairs it");
}
