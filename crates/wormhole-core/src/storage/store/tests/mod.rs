//! Tests of the store, by what they exercise. `use super::*` below gets the shared fixtures.

mod copies;
mod erasure;
mod migrations;
mod private;
mod recovery;
mod rename;
mod saved_text;
mod schema;
mod schema_snapshot;
mod scrollback;
mod sealed;
mod sessions;
mod splits;
mod upgrade_v7;
mod upgrade_v7_random;
mod windows;

use std::path::PathBuf;

use rusqlite::Connection;

use super::Store;
use super::records::{NoteRecord, SessionRecord, TaskRecord, WindowRecord};

pub(super) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wormhole-store-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

pub(super) fn session(id: i64, cwd: &str) -> SessionRecord {
    SessionRecord::new(id, Some(format!("session {id}")), Some(cwd.into()))
}

pub(super) fn window(id: i64, sessions: Vec<SessionRecord>) -> WindowRecord {
    WindowRecord { id, sessions, ..Default::default() }
}

pub(super) fn changes(store: &Store) -> u64 {
    store.conn.total_changes()
}

pub(super) fn count(store: &Store, sql: &str) -> i64 {
    store.conn.query_row(sql, [], |r| r.get(0)).expect("counts")
}

pub(super) fn note(body: &str) -> NoteRecord {
    NoteRecord { body: body.into(), collapsed: false }
}

pub(super) fn task(text: &str) -> TaskRecord {
    TaskRecord { text: text.into(), done: false }
}

/// A layout an older development build wrote, before `user_version` was set: scrollback in `tabs`,
/// no indexes.
pub(super) fn old_database(dir: &std::path::Path) -> Connection {
    std::fs::create_dir_all(dir).expect("dir");
    let conn = Connection::open(dir.join("wormhole.db")).expect("opens");
    conn.execute_batch(
        "CREATE TABLE sessions (id INTEGER PRIMARY KEY, position INTEGER NOT NULL, name TEXT, cwd TEXT,
            tasks_collapsed INTEGER NOT NULL DEFAULT 0, scrollback TEXT,
            window_id INTEGER NOT NULL DEFAULT 1, active_tab INTEGER NOT NULL DEFAULT 0,
            notes_collapsed INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE notes (session_id INTEGER NOT NULL, position INTEGER NOT NULL, body TEXT NOT NULL,
            collapsed INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE tasks (session_id INTEGER NOT NULL, position INTEGER NOT NULL, text TEXT NOT NULL,
            done INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
         CREATE TABLE windows (id INTEGER PRIMARY KEY, position INTEGER NOT NULL,
            active INTEGER NOT NULL DEFAULT 0, layout TEXT, bounds TEXT);
         CREATE TABLE tabs (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL, position INTEGER NOT NULL,
            cwd TEXT, scrollback TEXT);",
    )
    .expect("old schema");
    conn
}

pub(super) fn corrupt_files(dir: &std::path::Path) -> Vec<String> {
    aside_files(dir, ".corrupt-")
}

/// The files in `dir` that were moved aside with `label` in their name.
pub(super) fn aside_files(dir: &std::path::Path, label: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("reads")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.contains(label))
        .collect();
    names.sort();
    names
}
