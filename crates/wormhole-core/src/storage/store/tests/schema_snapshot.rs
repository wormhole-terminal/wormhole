//! The schema of a fresh database, as a literal: a schema change (a column lost or added in the
//! `CREATE` block of `schema.rs`) shows up here as a diff to update on purpose.

use super::*;

/// What a database holds, normalized so the text is stable: every table and index in
/// `sqlite_master` with its SQL on one line, and each table's columns from `pragma_table_info`
/// as `name TYPE [not null] [default ..] [pk n]`, in column order.
pub(super) fn dump(conn: &Connection) -> String {
    let mut out = String::new();
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).expect("version");
    out.push_str(&format!("user_version {version}\n"));
    let mut objects = conn
        .prepare(
            "SELECT type, name, sql FROM sqlite_master
             WHERE name NOT LIKE 'sqlite_%' ORDER BY type DESC, name",
        )
        .expect("prepares");
    let objects: Vec<(String, String, String)> = objects
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("queries")
        .collect::<rusqlite::Result<_>>()
        .expect("rows");
    for (kind, name, sql) in objects {
        // SQLite stores the `CREATE` text as written: how it is indented and wrapped doesn't count.
        let sql = sql
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace(" ,", ",");
        out.push_str(&format!("{kind} {name}: {sql}\n"));
        if kind != "table" {
            continue;
        }
        let mut columns = conn
            .prepare("SELECT name, type, \"notnull\", dflt_value, pk FROM pragma_table_info(?1) ORDER BY cid")
            .expect("prepares");
        let columns: Vec<String> = columns
            .query_map([&name], |row| {
                let (name, ty, not_null, default, pk): (String, String, i64, Option<String>, i64) =
                    (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?);
                let mut column = format!("{name} {ty}");
                if not_null == 1 {
                    column.push_str(" not null");
                }
                if let Some(default) = default {
                    column.push_str(&format!(" default {default}"));
                }
                if pk > 0 {
                    column.push_str(&format!(" pk {pk}"));
                }
                Ok(column)
            })
            .expect("queries")
            .collect::<rusqlite::Result<_>>()
            .expect("rows");
        out.push_str(&format!("  columns: {}\n", columns.join(", ")));
    }
    out
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// What [`dump`] must give for a fresh database, compared ignoring how the lines are wrapped.
const EXPECTED: &str = r#"
user_version 11
table agent_grants: CREATE TABLE agent_grants (id INTEGER PRIMARY KEY, kind TEXT NOT NULL,
        agent TEXT NOT NULL DEFAULT '', identifier TEXT NOT NULL DEFAULT '',
        team_id TEXT NOT NULL DEFAULT '', pin TEXT NOT NULL DEFAULT '', label TEXT NOT NULL,
        location TEXT NOT NULL DEFAULT '', cdhash TEXT NOT NULL DEFAULT '', caps INTEGER NOT NULL,
        created INTEGER NOT NULL, last_used INTEGER, uses INTEGER NOT NULL DEFAULT 0)
    columns: id INTEGER pk 1, kind TEXT not null, agent TEXT not null default '',
        identifier TEXT not null default '', team_id TEXT not null default '',
        pin TEXT not null default '', label TEXT not null, location TEXT not null default '',
        cdhash TEXT not null default '', caps INTEGER not null, created INTEGER not null,
        last_used INTEGER, uses INTEGER not null default 0
table meta: CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)
    columns: key TEXT pk 1, value TEXT
table notes: CREATE TABLE notes (session_id INTEGER NOT NULL, position INTEGER NOT NULL,
        body TEXT NOT NULL, collapsed INTEGER NOT NULL DEFAULT 0)
    columns: session_id INTEGER not null, position INTEGER not null, body TEXT not null,
        collapsed INTEGER not null default 0
table row_claims: CREATE TABLE row_claims (session_id INTEGER NOT NULL, root TEXT NOT NULL,
        path TEXT NOT NULL, PRIMARY KEY (session_id, root, path))
    columns: session_id INTEGER not null pk 1, root TEXT not null pk 2, path TEXT not null pk 3
table sessions: CREATE TABLE sessions (id INTEGER PRIMARY KEY, window_id INTEGER NOT NULL,
        position INTEGER NOT NULL, name TEXT, active_tab INTEGER NOT NULL DEFAULT 0,
        tasks_collapsed INTEGER NOT NULL DEFAULT 0, notes_collapsed INTEGER NOT NULL DEFAULT 0,
        pinned INTEGER NOT NULL DEFAULT 0, split_id INTEGER, forget_text INTEGER NOT NULL DEFAULT 0)
    columns: id INTEGER pk 1, window_id INTEGER not null, position INTEGER not null, name TEXT,
        active_tab INTEGER not null default 0, tasks_collapsed INTEGER not null default 0,
        notes_collapsed INTEGER not null default 0, pinned INTEGER not null default 0, split_id INTEGER,
        forget_text INTEGER not null default 0
table splits: CREATE TABLE splits (id INTEGER PRIMARY KEY, window_id INTEGER NOT NULL, layout TEXT)
    columns: id INTEGER pk 1, window_id INTEGER not null, layout TEXT
table tabs: CREATE TABLE tabs (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL,
        tab_id INTEGER NOT NULL, position INTEGER NOT NULL, cwd TEXT, shell TEXT, layout TEXT,
        host_id TEXT)
    columns: id INTEGER pk 1, session_id INTEGER not null, tab_id INTEGER not null,
        position INTEGER not null, cwd TEXT, shell TEXT, layout TEXT, host_id TEXT
table tasks: CREATE TABLE tasks (session_id INTEGER NOT NULL, position INTEGER NOT NULL,
        text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)
    columns: session_id INTEGER not null, position INTEGER not null, text TEXT not null,
        done INTEGER not null default 0
table windows: CREATE TABLE windows (id INTEGER PRIMARY KEY, position INTEGER NOT NULL,
        active INTEGER NOT NULL DEFAULT 0, layout TEXT, bounds TEXT)
    columns: id INTEGER pk 1, position INTEGER not null, active INTEGER not null default 0,
        layout TEXT, bounds TEXT
index agent_grants_identity: CREATE UNIQUE INDEX agent_grants_identity
        ON agent_grants (kind, agent, identifier, team_id, pin)
index notes_session: CREATE INDEX notes_session ON notes (session_id)
index sessions_window: CREATE INDEX sessions_window ON sessions (window_id)
index splits_window: CREATE INDEX splits_window ON splits (window_id)
index tabs_session: CREATE INDEX tabs_session ON tabs (session_id)
index tasks_session: CREATE INDEX tasks_session ON tasks (session_id)
"#;

#[test]
fn a_fresh_database_has_exactly_this_schema() {
    let dir = temp_dir("schema-snapshot");
    let store = Store::open_at(&dir).expect("opens");
    let actual = dump(&store.conn);
    assert!(
        words(&actual) == words(EXPECTED),
        "the schema of a fresh database changed; if that is intended, update EXPECTED\n\
         --- actual ---\n{actual}--- expected ---\n{EXPECTED}"
    );
}
