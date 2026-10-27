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
const MIGRATIONS: &[&str] = &[
    steps::V7_EVERY_TERMINAL_A_SESSION,
    steps::V8_TERMINAL_HOST,
    steps::V9_TEXT_IN_ITS_OWN_FILE,
    steps::V10_ROW_CLAIMS,
    steps::V11_AGENT_ACCESS,
    steps::V12_AGENT_HIDDEN,
];

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
/// `tabs.host_id` names the session host that runs the terminal (`hosts`), NULL for a terminal in
/// the app's own process.
///
/// `agent_grants` are the programs the user approved to control Wormhole for good; each row is one
/// identity (`ipc::access`), whose key is the columns before `label`.
///
/// A terminal's saved text is not here but in a file of its own (`saved_text`), keyed by the
/// terminal's id: never in a `tabs` row, which a window save rewrites.
///
/// `row_claims` holds the files a terminal changed (the count on its row, `git::attribution`): a row
/// per file with the repository it is in, keyed by the terminal's id like its notes and tasks, and
/// forgotten with the session.
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
        split_id INTEGER,
        forget_text INTEGER NOT NULL DEFAULT 0,
        agent_hidden INTEGER NOT NULL DEFAULT 0
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
        layout TEXT,
        host_id TEXT
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
    CREATE TABLE IF NOT EXISTS row_claims (
        session_id INTEGER NOT NULL,
        root TEXT NOT NULL,
        path TEXT NOT NULL,
        PRIMARY KEY (session_id, root, path)
    );
    CREATE TABLE IF NOT EXISTS agent_grants (
        id INTEGER PRIMARY KEY,
        kind TEXT NOT NULL,
        agent TEXT NOT NULL DEFAULT '',
        identifier TEXT NOT NULL DEFAULT '',
        team_id TEXT NOT NULL DEFAULT '',
        pin TEXT NOT NULL DEFAULT '',
        label TEXT NOT NULL,
        location TEXT NOT NULL DEFAULT '',
        cdhash TEXT NOT NULL DEFAULT '',
        caps INTEGER NOT NULL,
        created INTEGER NOT NULL,
        last_used INTEGER,
        uses INTEGER NOT NULL DEFAULT 0
    );
    CREATE UNIQUE INDEX IF NOT EXISTS agent_grants_identity
        ON agent_grants (kind, agent, identifier, team_id, pin);
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
/// `after_steps` runs inside the upgrade's transaction, after the steps (only when some ran), for
/// what SQL can't do alone (`saved_text::take_moved`).
pub(super) fn init(conn: &mut Connection, after_steps: impl FnOnce(&Connection) -> Result<()>) -> Result<()> {
    init_with_then(conn, SCHEMA, MIGRATIONS, after_steps)
}

/// [`init`] with the layout and the steps as arguments, so tests can upgrade through steps of
/// their own.
#[cfg(test)]
pub(super) fn init_with(conn: &mut Connection, schema: &str, migrations: &[&str]) -> Result<()> {
    init_with_then(conn, schema, migrations, |_| Ok(()))
}

fn init_with_then(
    conn: &mut Connection,
    schema: &str,
    migrations: &[&str],
    after_steps: impl FnOnce(&Connection) -> Result<()>,
) -> Result<()> {
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
                after_steps(&tx)?;
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
/// in is nothing; the files of a terminal that is gone (`row_claims`) are of nobody. Cheap enough to
/// check on every open.
fn repair(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO windows (id, position, active)
             SELECT DISTINCT window_id, window_id, 0 FROM sessions
             WHERE window_id NOT IN (SELECT id FROM windows);
         INSERT INTO tabs (id, session_id, tab_id, position)
             SELECT id, id, id, 0 FROM sessions WHERE id NOT IN (SELECT session_id FROM tabs);
         UPDATE sessions SET split_id = NULL
             WHERE split_id IS NOT NULL AND split_id NOT IN (SELECT id FROM splits);
         DELETE FROM splits WHERE id NOT IN (SELECT split_id FROM sessions WHERE split_id IS NOT NULL);
         DELETE FROM row_claims WHERE session_id NOT IN (SELECT id FROM sessions);",
    )?;
    Ok(())
}
