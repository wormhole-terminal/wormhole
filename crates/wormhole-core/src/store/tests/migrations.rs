//! Upgrading a database from an older build: its windows and terminals stay, and it ends up with
//! exactly the layout of a fresh one. What the real steps do to the data has tests of its own
//! (`upgrade_v7`).

use super::schema_snapshot::dump;
use super::*;
use crate::store::schema::{self, SCHEMA};
use crate::store::steps;

/// The layout of schema version 6, the first one databases are upgraded from, frozen as it
/// shipped. Never edit it: the databases out there have it.
pub(super) const V6_SCHEMA: &str = "
    CREATE TABLE windows (
        id INTEGER PRIMARY KEY,
        position INTEGER NOT NULL,
        active INTEGER NOT NULL DEFAULT 0,
        layout TEXT,
        bounds TEXT
    );
    CREATE TABLE sessions (
        id INTEGER PRIMARY KEY,
        window_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        name TEXT,
        active_tab INTEGER NOT NULL DEFAULT 0,
        tasks_collapsed INTEGER NOT NULL DEFAULT 0,
        notes_collapsed INTEGER NOT NULL DEFAULT 0,
        pinned INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE tabs (
        id INTEGER PRIMARY KEY,
        session_id INTEGER NOT NULL,
        tab_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        cwd TEXT,
        shell TEXT,
        layout TEXT
    );
    CREATE TABLE scrollback (
        tab_id INTEGER PRIMARY KEY REFERENCES tabs(id) ON DELETE CASCADE,
        hash INTEGER,
        text TEXT NOT NULL
    );
    CREATE TABLE notes (
        session_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        body TEXT NOT NULL,
        collapsed INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE tasks (
        session_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        text TEXT NOT NULL,
        done INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
    CREATE INDEX sessions_window ON sessions (window_id);
    CREATE INDEX tabs_session ON tabs (session_id);
    CREATE INDEX notes_session ON notes (session_id);
    CREATE INDEX tasks_session ON tasks (session_id);
";

/// What a user of a version 6 build has: one window, a terminal with a note, a task and scrollback.
const V6_DATA: &str = "
    INSERT INTO windows (id, position, active) VALUES (1, 0, 0);
    INSERT INTO sessions (id, window_id, position, name, pinned) VALUES (1, 1, 0, 'kept', 1);
    INSERT INTO tabs (id, session_id, tab_id, position, cwd, shell) VALUES (1, 1, 1, 0, '/a', 'fish');
    INSERT INTO scrollback (tab_id, hash, text) VALUES (1, 7, 'output');
    INSERT INTO notes (session_id, position, body) VALUES (1, 0, 'a note');
    INSERT INTO tasks (session_id, position, text) VALUES (1, 0, 'a task');
    PRAGMA user_version = 6;
";

/// A version 6 database with nothing in it but the layout.
pub(super) fn empty_v6_database(dir: &std::path::Path) -> Connection {
    std::fs::create_dir_all(dir).expect("dir");
    let conn = Connection::open(dir.join("wormhole.db")).expect("opens");
    conn.execute_batch(V6_SCHEMA).expect("schema");
    conn.pragma_update(None, "user_version", 6).expect("version");
    conn
}

fn v6_database(dir: &std::path::Path) -> Connection {
    let conn = empty_v6_database(dir);
    conn.execute_batch(V6_DATA).expect("data");
    conn
}

pub(super) fn columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut names = conn.prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid").expect("prepares");
    names
        .query_map([table], |row| row.get(0))
        .expect("queries")
        .collect::<rusqlite::Result<_>>()
        .expect("rows")
}

pub(super) fn version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0)).expect("version")
}

/// The real steps: a version 6 database comes out of them with what it held and exactly the
/// layout a fresh database gets. When this fails after a change to `SCHEMA`, a step in
/// `MIGRATIONS` is missing or doesn't produce the same layout.
#[test]
fn a_version_6_database_is_upgraded_to_exactly_the_fresh_layout() {
    let dir = temp_dir("migrate-v6");
    drop(v6_database(&dir));
    let store = Store::open_at(&dir).expect("opens");
    assert!(store.recovered_from().is_none(), "upgraded, not moved aside");
    assert_eq!(version(&store.conn), 8);
    let fresh_dir = temp_dir("migrate-fresh");
    let fresh = Store::open_at(&fresh_dir).expect("opens");
    assert_eq!(dump(&store.conn), dump(&fresh.conn));

    let windows = store.load().expect("loads");
    let session = &windows[0].sessions[0];
    assert_eq!(session.name.as_deref(), Some("kept"));
    assert!(session.pinned);
    assert_eq!(session.notes[0].body, "a note");
    assert_eq!(session.tasks[0].text, "a task");
    assert_eq!(session.cwd.as_deref(), Some("/a"));
    assert_eq!(session.shell.as_deref(), Some("fish"));
    assert_eq!(session.scrollback.as_deref(), Some("output"));
    assert_eq!(session.split_id, None);
    assert_eq!(session.host_id, None, "a terminal from before hosts runs in the app");
    assert!(windows[0].splits.is_empty());

    // A copy of the database as it was is kept first, with everything in it.
    let copies = aside_files(&dir, ".v6-");
    let copies: Vec<&String> =
        copies.iter().filter(|n| !n.ends_with("-wal") && !n.ends_with("-shm")).collect();
    assert_eq!(copies.len(), 1, "{copies:?}");
    let copy = Connection::open(dir.join(copies[0])).expect("opens the copy");
    assert_eq!(version(&copy), 6);
    let kept = |sql: &str| -> String { copy.query_row(sql, [], |r| r.get(0)).expect("kept") };
    assert_eq!(kept("SELECT name FROM sessions"), "kept");
    assert_eq!(kept("SELECT text FROM scrollback"), "output");
    assert_eq!(kept("SELECT body FROM notes"), "a note");
    assert_eq!(kept("SELECT text FROM tasks"), "a task");
    assert!(!columns(&copy, "sessions").contains(&"split_id".to_string()), "as it was");
}

/// How the steps run, with steps of the test's own after the real one: from whatever version the
/// database has, each remaining one in order, and the version set to the last.
#[test]
fn the_remaining_steps_run_in_order_and_keep_the_data() {
    let dir = temp_dir("migrate-steps");
    let mut conn = v6_database(&dir);
    let steps = [
        steps::V7_EVERY_TERMINAL_A_SESSION,
        "ALTER TABLE sessions ADD COLUMN color TEXT;",
        "UPDATE sessions SET color = 'teal' WHERE name = 'kept';",
        "ALTER TABLE tabs ADD COLUMN title TEXT;",
    ];
    schema::init_with(&mut conn, SCHEMA, &steps[..3]).expect("upgrades to 9");
    assert_eq!(version(&conn), 9);
    let color: String = conn.query_row("SELECT color FROM sessions", [], |r| r.get(0)).expect("color");
    assert_eq!(color, "teal");

    // A later build adds one more step; only that one runs.
    conn.execute("UPDATE sessions SET color = 'red'", []).expect("updates");
    schema::init_with(&mut conn, SCHEMA, &steps).expect("upgrades to 10");
    assert_eq!(version(&conn), 10);
    let color: String = conn.query_row("SELECT color FROM sessions", [], |r| r.get(0)).expect("color");
    assert_eq!(color, "red", "the steps that already ran don't run again");
    assert!(columns(&conn, "tabs").contains(&"title".to_string()));
    let name: String = conn.query_row("SELECT name FROM sessions", [], |r| r.get(0)).expect("kept");
    assert_eq!(name, "kept");
}

#[test]
fn a_failing_step_leaves_the_database_as_it_was() {
    let dir = temp_dir("migrate-fails");
    let mut conn = v6_database(&dir);
    let steps = [
        steps::V7_EVERY_TERMINAL_A_SESSION,
        "ALTER TABLE sessions ADD COLUMN color TEXT;",
        "ALTER TABLE nowhere ADD COLUMN x TEXT;",
    ];
    let err = schema::init_with(&mut conn, SCHEMA, &steps).expect_err("fails");
    assert!(format!("{err:#}").contains("from schema version 8"), "{err:#}");
    assert_eq!(version(&conn), 6);
    // Neither the real step nor the one after it is left half done.
    assert!(!columns(&conn, "sessions").contains(&"color".to_string()));
    assert!(!columns(&conn, "sessions").contains(&"split_id".to_string()));
    let name: String = conn.query_row("SELECT name FROM sessions", [], |r| r.get(0)).expect("kept");
    assert_eq!(name, "kept");
    let text: String = conn.query_row("SELECT text FROM scrollback", [], |r| r.get(0)).expect("kept");
    assert_eq!(text, "output");
}

#[test]
fn a_fresh_database_gets_the_layout_without_running_the_steps() {
    let dir = temp_dir("migrate-new");
    std::fs::create_dir_all(&dir).expect("dir");
    let mut conn = Connection::open(dir.join("wormhole.db")).expect("opens");
    // A step that would fail on the fresh layout: it must not run.
    schema::init_with(&mut conn, SCHEMA, &["ALTER TABLE nowhere ADD COLUMN x TEXT;"]).expect("creates");
    assert_eq!(version(&conn), 7);
    assert!(columns(&conn, "sessions").contains(&"pinned".to_string()));
}
