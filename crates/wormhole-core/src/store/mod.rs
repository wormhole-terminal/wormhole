//! Persistence of windows, sessions, splits, notes and tasks in SQLite.
//!
//! A window owns sessions, a session is one terminal, and a split groups several of a window's
//! sessions under one layout (see `schema`). Rows are upserted
//! by a stable id and only written when they differ from what is stored. The
//! (large) scrollback lives in a table of its own and is saved separately, in
//! batches, only when it changed. Every window saves only its own slice, so two
//! windows never overwrite each other's sessions; `save_session` narrows that to one session.

mod records;
mod schema;
mod scrollback;
mod sessions;
mod steps;
#[cfg(test)]
mod tests;
mod windows;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use rusqlite::{Connection, OptionalExtension};

pub use records::{NoteRecord, SessionRecord, SplitRecord, TaskRecord, WindowRecord, is_blank};

/// `WORMHOLE_DATA_DIR` overrides the location (used for isolated test runs).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("WORMHOLE_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join(data_dir_name())
}

/// The data folder's name, and its database's, in 0.2 and before, when the app went by its codename.
const LEGACY_DIR: &str = "Andromeda";
const LEGACY_DB_FILE: &str = "andromeda.db";

/// Bring the data of a version from before the rename to where this one looks, once: the
/// `Andromeda` folder becomes `Wormhole` and its database `wormhole.db`, with a copy of the old
/// database kept beside it. Only when there is no `Wormhole` folder yet, and never with
/// `WORMHOLE_DATA_DIR` set. Returns the folder it moved. Run it before anything opens a file in
/// the data folder (the log, the database, the instance lock).
pub fn migrate_legacy_data() -> Result<Option<PathBuf>> {
    if std::env::var_os("WORMHOLE_DATA_DIR").is_some() {
        return Ok(None);
    }
    let Some(base) = dirs::data_dir() else { return Ok(None) };
    migrate_legacy_data_in(&base)
}

fn migrate_legacy_data_in(base: &Path) -> Result<Option<PathBuf>> {
    let (old, new) = (base.join(LEGACY_DIR), base.join(data_dir_name()));
    if new.exists() || !old.is_dir() {
        return Ok(None);
    }
    // An earlier version still running holds its instance lock there: leave its folder alone and
    // move it at a launch when that one has quit.
    if crate::ipc::is_locked_instance_dir(&old) {
        anyhow::bail!("{} is in use by an earlier version that is still running", old.display());
    }
    // One rename on the same disk: the folder is either all here or all there.
    std::fs::rename(&old, &new).with_context(|| format!("moving {} to {}", old.display(), new.display()))?;
    // The database and its journal files move together, so SQLite finds them as one database.
    let suffixes = ["", "-wal", "-shm"];
    if new.join(LEGACY_DB_FILE).exists() && !new.join(DB_FILE).exists() {
        for suffix in suffixes {
            let from = new.join(format!("{LEGACY_DB_FILE}{suffix}"));
            if from.exists() {
                let backup = new.join(format!("andromeda-before-wormhole.db{suffix}"));
                std::fs::copy(&from, &backup).with_context(|| format!("keeping {}", backup.display()))?;
            }
        }
        for suffix in suffixes {
            let from = new.join(format!("{LEGACY_DB_FILE}{suffix}"));
            if from.exists() {
                std::fs::rename(&from, new.join(format!("{DB_FILE}{suffix}")))
                    .with_context(|| format!("renaming {}", from.display()))?;
            }
        }
    }
    Ok(Some(new))
}

/// The data folder's name in Application Support.
fn data_dir_name() -> &'static str {
    "Wormhole"
}

pub struct Store {
    conn: Connection,
    /// Where a damaged or outdated database was moved to when this one had to be started fresh.
    recovered_from: Option<PathBuf>,
}

const DB_FILE: &str = "wormhole.db";

/// `PRAGMA quick_check` found something wrong; the message is its first complaint.
#[derive(Debug)]
struct Damaged(String);

impl std::fmt::Display for Damaged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the database is damaged: {}", self.0)
    }
}

impl std::error::Error for Damaged {}

impl Store {
    pub fn open() -> Result<Self> {
        Self::open_at(&data_dir())
    }

    /// A profile's database (Default's is the data folder's own, so this is [`open`](Self::open)
    /// for it).
    pub fn open_for(paths: &crate::profile::ProfilePaths) -> Result<Self> {
        Self::open_at(paths.dir())
    }

    /// Whether the profile has a database yet (a new one has none until it is first opened).
    pub fn exists_for(paths: &crate::profile::ProfilePaths) -> bool {
        paths.dir().join(DB_FILE).exists()
    }

    /// Open (or create) the database in `dir`. A file that is not a database, or that fails
    /// `PRAGMA quick_check`, is moved aside as `wormhole.db.corrupt-<unix time>` (with its WAL)
    /// and a fresh one is started, so a damaged file costs the saved sessions once instead of
    /// silently disabling persistence on every launch. A database from an older build is upgraded
    /// in place, after a copy of it is kept as `wormhole.db.v<version>-<unix time>`; one too old
    /// to upgrade is moved aside as `wormhole.db.old-<unix time>`. `recovered_from` tells the
    /// caller, who should say so. Any other failure (locked, unreadable, newer schema, a failed
    /// upgrade) is an error and leaves the file alone.
    pub fn open_at(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(DB_FILE);
        match Self::open_file(&path) {
            Err(err) if is_damaged(&err) || is_stale(&err) => {
                let label = if is_stale(&err) { "old" } else { "corrupt" };
                let aside = move_aside(&path, label).with_context(|| format!("{err:#}"))?;
                tracing::error!(
                    "{err:#}; moved {} to {} and starting with an empty database",
                    path.display(),
                    aside.display()
                );
                let mut store = Self::open_file(&path).context("creating a new database")?;
                store.recovered_from = Some(aside);
                Ok(store)
            }
            result => result,
        }
    }

    /// Set when `open_at` found the database damaged or outdated: where the old file went.
    pub fn recovered_from(&self) -> Option<&Path> {
        self.recovered_from.as_deref()
    }

    fn open_file(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        configure(&conn)?;
        // Before anything is written: writing to a damaged file would only spread the damage.
        let verdict: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if verdict != "ok" {
            return Err(Damaged(verdict).into());
        }
        if let Some(version) = schema::upgrade_from(&conn)? {
            // If an upgrade loses something it shouldn't, the windows and terminals as they were
            // are still in this copy.
            let copy = aside_path(path, &format!("v{version}"));
            conn.execute("VACUUM INTO ?1", [copy.to_string_lossy()])
                .with_context(|| format!("keeping a copy of the database in {}", copy.display()))?;
            tracing::info!("kept a copy of the schema version {version} database in {}", copy.display());
        }
        schema::init(&mut conn)?;
        Ok(Self { conn, recovered_from: None })
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        read_meta(&self.conn, key)
    }
}

fn is_stale(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.is::<schema::Stale>())
}

fn is_damaged(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause.is::<Damaged>()
            || matches!(
                cause.downcast_ref::<rusqlite::Error>(),
                Some(rusqlite::Error::SqliteFailure(e, _))
                    if matches!(e.code, rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt)
            )
    })
}

/// Rename the database and its `-wal`/`-shm` files out of the way (a stale WAL next to a new
/// main file would be replayed into it), as `<name>.<label>-<unix time>`. Returns the new name of
/// the main file.
fn move_aside(path: &Path, label: &str) -> Result<PathBuf> {
    let target = aside_path(path, label);
    std::fs::rename(path, &target).with_context(|| format!("moving {} aside", path.display()))?;
    for sidecar in ["-wal", "-shm"] {
        let from = with_suffix(path, sidecar);
        if from.exists() {
            std::fs::rename(&from, with_suffix(&target, sidecar))
                .with_context(|| format!("moving {} aside", from.display()))?;
        }
    }
    Ok(target)
}

/// A name next to the database that no file has yet: `<name>.<label>-<unix time>`.
fn aside_path(path: &Path, label: &str) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let mut target = with_suffix(path, &format!(".{label}-{stamp}"));
    for n in 2.. {
        if !target.exists() {
            break;
        }
        target = with_suffix(path, &format!(".{label}-{stamp}-{n}"));
    }
    target
}

fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = base.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Connection settings, applied before the schema is touched.
fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // The WAL setting's usual partner: a commit no longer waits for an fsync of its own (only
    // checkpoints do). A crash of the app loses nothing; a power cut can lose the last
    // transactions but cannot corrupt the database. We save on every edit and every minute,
    // so that trade is the right one.
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", true)?;
    // A second connection (a writer thread, another tool) waits for the lock instead of failing.
    conn.busy_timeout(Duration::from_secs(5))?;
    // Scrollback makes the WAL grow in bursts; hand the space back after the checkpoint.
    conn.pragma_update(None, "journal_size_limit", 8 * 1024 * 1024)?;
    Ok(())
}

/// Run a statement through the connection's statement cache: the SQL is compiled once per
/// connection, not once per save.
fn exec(conn: &Connection, sql: &str, params: impl rusqlite::Params) -> Result<usize> {
    Ok(conn.prepare_cached(sql)?.execute(params)?)
}

fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT value FROM meta WHERE key = ?", [key], |row| row.get(0)).optional()?)
}
