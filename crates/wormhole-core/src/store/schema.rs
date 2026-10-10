//! The tables. `PRAGMA user_version` names the layout a database has. A database from an older
//! build is upgraded in place, one [`MIGRATIONS`] step per version, so an update keeps the user's
//! windows and terminals; one from before migrations existed (older than [`FIRST_MIGRATABLE`]) is
//! [`Stale`] and gets moved aside for a fresh one; one from a newer build is refused.
//!
//! Changing the layout: change [`SCHEMA`] (what a fresh database gets) and append the step that
//! takes the previous version there to [`MIGRATIONS`] (the steps are in `steps.rs`). Never edit a
//! step that has shipped: a database out there may already have run it. `tests/migrations.rs`
//! checks that a migrated database ends up exactly like a fresh one, and `tests/upgrade_v7.rs` is
//! how to test what a step does to the data.

use anyhow::{Context as _, Result, bail};
use rusqlite::{Connection, TransactionBehavior};

use super::steps;

/// The oldest layout a database can be upgraded from: the one in use when migrations began.
const FIRST_MIGRATABLE: i64 = 6;

/// `MIGRATIONS[i]` takes a database from version `FIRST_MIGRATABLE + i` to the next one. All the
/// steps a database needs run in one transaction, so a failing step leaves it as it was.
const MIGRATIONS: &[&str] = &[steps::V7_EVERY_TERMINAL_A_SESSION];

/// The schema version this build creates and reads.
const VERSION: i64 = FIRST_MIGRATABLE + MIGRATIONS.len() as i64;

/// The whole layout.
///
/// A session is one terminal (a sidebar row), and a split groups several: `sessions.split_id` names
/// a row of `splits`, which holds the layout the members share (the app's pane tree as JSON, keyed
/// by the members' ids). A terminal's id is its session's, and its `tabs` row, which holds the
/// working folder and the shell, has that id too: `session_id` and `tab_id` repeat it, `position`
/// is 0 and `layout` NULL. Those three and `sessions.active_tab` are what a session had when it had
/// tabs and panes (schema 6); nothing reads them now, and a later step drops them.
/// `scrollback.tab_id` names the terminal's row.
///
/// Scrollback has a table of its own: SQLite rewrites whole records, and updating a `tabs` row
/// to change its cwd would have to read (and maybe rewrite) the scrollback's overflow
/// chain too, 4 to 50 ms per window save with 20 terminals of scrollback against 0.5 ms without.
/// `hash` comes before `text` on purpose: a column's bytes are reached by walking the record, so
/// reading the hash after a 300 KB text would walk the whole overflow chain.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS windows (
        id INTEGER PRIMARY KEY,
        position INTEGER NOT NULL,
        active INTEGER NOT NULL DEFAULT 0,
        layout TEXT,
        bounds TEXT
    );
    CREATE TABLE IF NOT EXISTS sessions (
        id INTEGER PRIMARY KEY,
        window_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        name TEXT,
        active_tab INTEGER NOT NULL DEFAULT 0,
        tasks_collapsed INTEGER NOT NULL DEFAULT 0,
        notes_collapsed INTEGER NOT NULL DEFAULT 0,
        pinned INTEGER NOT NULL DEFAULT 0,
        split_id INTEGER
    );
    CREATE TABLE IF NOT EXISTS splits (
        id INTEGER PRIMARY KEY,
        window_id INTEGER NOT NULL,
        layout TEXT
    );
    CREATE TABLE IF NOT EXISTS tabs (
        id INTEGER PRIMARY KEY,
        session_id INTEGER NOT NULL,
        tab_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        cwd TEXT,
        shell TEXT,
        layout TEXT
    );
    CREATE TABLE IF NOT EXISTS scrollback (
        tab_id INTEGER PRIMARY KEY REFERENCES tabs(id) ON DELETE CASCADE,
        hash INTEGER,
        text TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS notes (
        session_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        body TEXT NOT NULL,
        collapsed INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS tasks (
        session_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        text TEXT NOT NULL,
        done INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
    CREATE INDEX IF NOT EXISTS sessions_window ON sessions (window_id);
    CREATE INDEX IF NOT EXISTS splits_window ON splits (window_id);
    CREATE INDEX IF NOT EXISTS tabs_session ON tabs (session_id);
    CREATE INDEX IF NOT EXISTS notes_session ON notes (session_id);
    CREATE INDEX IF NOT EXISTS tasks_session ON tasks (session_id);
";

/// The database was written by a build from before migrations, in a layout this one can't upgrade.
#[derive(Debug)]
pub(super) struct Stale(i64);

impl std::fmt::Display for Stale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the database has schema version {}, older than this build can upgrade ({FIRST_MIGRATABLE})",
            self.0
        )
    }
}

impl std::error::Error for Stale {}

/// The version of a database this build would upgrade when it opens it, so the caller can keep a
/// copy first; `None` when it is fresh, current, too old to upgrade or newer.
pub(super) fn upgrade_from(conn: &Connection) -> Result<Option<i64>> {
    let version = user_version(conn)?;
    Ok((FIRST_MIGRATABLE..VERSION).contains(&version).then_some(version))
}

/// Create the tables in a new database; check the version of an existing one and upgrade it.
pub(super) fn init(conn: &mut Connection) -> Result<()> {
    init_with(conn, SCHEMA, MIGRATIONS)
}

/// [`init`] with the layout and the steps as arguments, so tests can upgrade through steps of
/// their own.
pub(super) fn init_with(conn: &mut Connection, schema: &str, migrations: &[&str]) -> Result<()> {
    let latest = FIRST_MIGRATABLE + migrations.len() as i64;
    if user_version(conn)? != latest {
        // Immediate: take the write lock up front, and look again under it (another process may
        // have created or upgraded the tables in the meantime).
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version = user_version(&tx)?;
        if version > latest {
            // Never guess at a layout we don't know: a newer build may have moved data around.
            bail!("the database has schema version {version}, newer than this build understands ({latest})");
        }
        if version < latest {
            let empty: bool = tx.query_row("SELECT COUNT(*) = 0 FROM sqlite_master", [], |row| row.get(0))?;
            if empty {
                tx.execute_batch(schema)?;
            } else if version < FIRST_MIGRATABLE {
                return Err(Stale(version).into());
            } else {
                for (from, step) in (version..).zip(&migrations[(version - FIRST_MIGRATABLE) as usize..]) {
                    tx.execute_batch(step)
                        .with_context(|| format!("upgrading the database from schema version {from}"))?;
                }
                tracing::info!("upgraded the database from schema version {version} to {latest}");
            }
            tx.pragma_update(None, "user_version", latest)?;
        }
        tx.commit()?;
    }
    repair(conn)
}

fn user_version(conn: &Connection) -> Result<i64> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// Sessions whose window row or tab row is missing would be invisible, or have no terminal, and
/// never be cleaned up; a session naming a split that is gone is not in one, and a split nobody is
/// in is nothing. Cheap enough to check on every open.
fn repair(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO windows (id, position, active)
             SELECT DISTINCT window_id, window_id, 0 FROM sessions
             WHERE window_id NOT IN (SELECT id FROM windows);
         INSERT INTO tabs (id, session_id, tab_id, position)
             SELECT id, id, id, 0 FROM sessions WHERE id NOT IN (SELECT session_id FROM tabs);
         UPDATE sessions SET split_id = NULL
             WHERE split_id IS NOT NULL AND split_id NOT IN (SELECT id FROM splits);
         DELETE FROM splits WHERE id NOT IN (SELECT split_id FROM sessions WHERE split_id IS NOT NULL);",
    )?;
    Ok(())
}
