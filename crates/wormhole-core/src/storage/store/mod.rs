//! Persistence of windows, sessions, splits, notes and tasks in SQLite.
//!
//! A window owns sessions, a session is one terminal, and a split groups several of a window's
//! sessions under one layout (see `schema`). Rows are upserted
//! by a stable id and only written when they differ from what is stored. The
//! (large) scrollback lives in a file of its own (`saved_text`) and is saved separately, in
//! batches, only when it changed. Every window saves only its own slice, so two
//! windows never overwrite each other's sessions; `save_session` narrows that to one session.

mod claims;
mod agent_grants;
mod copies;
mod records;
mod saved_text;
mod schema;
mod scrollback;
mod sessions;
mod steps;
#[cfg(test)]
mod tests;
mod windows;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use rusqlite::{Connection, OptionalExtension};

pub use claims::ClaimRecord;
pub use agent_grants::{AgentGrantKey, AgentGrantRow};
pub use records::{NoteRecord, SessionRecord, SplitRecord, TaskRecord, WindowRecord, is_blank};

use crate::platform::paths;
use crate::storage::paths::{data_dir, data_dir_name};
use saved_text::platform::{self, Platform};

/// Make Wormhole's files and folders in its data folder owner-only (folders `0700`, files `0600`),
/// for what an earlier version made with the umask's modes and what a library made since (the
/// logs). Run at launch, off the UI thread. A folder `WORMHOLE_DATA_DIR` names is only touched when
/// it already holds Wormhole's data, and then only the names Wormhole puts there.
pub fn make_data_dir_private(dir: &Path) {
    let chosen = std::env::var_os("WORMHOLE_DATA_DIR").is_some();
    if !is_data_dir(dir, chosen) {
        return;
    }
    match crate::storage::files::make_tree_private(dir, &is_wormhole_name) {
        Ok(0) => {}
        Ok(changed) => tracing::info!("made {changed} files and folders in the data folder owner-only"),
        Err(err) => tracing::warn!("couldn't make the data folder {} owner-only: {err}", dir.display()),
    }
}

/// Whether `dir` is Wormhole's data folder: the default one, or (`chosen` by `WORMHOLE_DATA_DIR`)
/// one that holds its database or settings already.
fn is_data_dir(dir: &Path, chosen: bool) -> bool {
    !chosen || dir.join(DB_FILE).is_file() || dir.join("config.json").is_file()
}

/// The names Wormhole puts at the top of its data folder (and a profile's): its database with its
/// journals and copies, its JSON files, the instance lock and its folders.
fn is_wormhole_name(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else { return false };
    const FOLDERS: [&str; 7] = ["logs", "profiles", "hosts", "bin", "fonts", "themes", "icon-themes"];
    FOLDERS.contains(&name)
        || name == crate::storage::paths::INSTANCE_LOCK_FILE
        || name.ends_with(".json")
        || ["wormhole.db", saved_text::FILE, "andromeda.db", "andromeda-before-wormhole.db"]
            .iter()
            .any(|db| name.starts_with(db))
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
    let Some(base) = paths::data_base() else { return Ok(None) };
    migrate_legacy_data_in(&base)
}

fn migrate_legacy_data_in(base: &Path) -> Result<Option<PathBuf>> {
    let (old, new) = (base.join(LEGACY_DIR), base.join(data_dir_name()));
    if new.exists() || !old.is_dir() {
        return Ok(None);
    }
    // An earlier version still running holds its instance lock there: leave its folder alone and
    // move it at a launch when that one has quit.
    if crate::storage::paths::in_use(&old) {
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

pub struct Store {
    conn: Connection,
    /// The saved text's key and what was last written of it.
    saved: saved_text::Keeper,
    /// Where a damaged or outdated database was moved to when this one had to be started fresh.
    recovered_from: Option<PathBuf>,
}

const DB_FILE: &str = "wormhole.db";
/// `meta` key: the database was rewritten once without the free pages older versions left.
const SCRUBBED_KEY: &str = "scrubbed";
/// How long a statement waits for another connection's lock.
const BUSY_WAIT: Duration = Duration::from_secs(5);
/// How long a checkpoint waits for one.
const CHECKPOINT_WAIT: Duration = Duration::from_millis(200);

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
    pub fn open_for(paths: &crate::storage::paths::ProfilePaths) -> Result<Self> {
        Self::open_at(paths.dir())
    }

    /// Whether the profile has a database yet (a new one has none until it is first opened).
    pub fn exists_for(paths: &crate::storage::paths::ProfilePaths) -> bool {
        paths.dir().join(DB_FILE).exists()
    }

    /// Open (or create) the database in `dir`. A file that is not a database, or that fails
    /// `PRAGMA quick_check`, is moved aside as `wormhole.db.corrupt-<unix time>` (with its WAL)
    /// and a fresh one is started, so a damaged file costs the saved sessions once instead of
    /// silently disabling persistence on every launch. A database from an older build is upgraded
    /// in place, after a copy of it is kept as `wormhole.db.v<version>-<unix time>`; one too old
    /// to upgrade is moved aside as `wormhole.db.old-<unix time>`. `recovered_from` tells the
    /// caller, who should say so. Any other failure (locked, unreadable, newer schema, a failed
    /// upgrade) is an error and leaves the file alone. Copies past keeping are deleted first (see
    /// `copies`): each one holds the terminals' saved text as it was then.
    pub fn open_at(dir: &Path) -> Result<Self> {
        Self::open_in(dir, Arc::new(platform::System))
    }

    /// [`open_at`](Self::open_at) with the operating system's part played by `platform`.
    fn open_in(dir: &Path, platform: Arc<dyn Platform>) -> Result<Self> {
        if let Err(err) = crate::storage::files::ensure_private_dir(dir) {
            // A data folder that is a link (to another disk, a synced folder) still works; it just
            // isn't made private here.
            tracing::warn!("couldn't make {} owner-only: {err}", dir.display());
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        copies::prune(dir, SystemTime::now());
        let path = dir.join(DB_FILE);
        match Self::open_file(&path, &platform) {
            Err(err) if is_damaged(&err) || is_stale(&err) => {
                let label = if is_stale(&err) { "old" } else { "corrupt" };
                let aside = move_aside(&path, label).with_context(|| format!("{err:#}"))?;
                tracing::error!(
                    "{err:#}; moved {} to {} and starting with an empty database",
                    path.display(),
                    aside.display()
                );
                let mut store = Self::open_file(&path, &platform).context("creating a new database")?;
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

    fn open_file(path: &Path, platform: &Arc<dyn Platform>) -> Result<Self> {
        // Made owner-only before SQLite opens it, which then gives the `-wal` and `-shm` it creates
        // the same mode; ones an earlier version made are tightened.
        // A failure here is only logged: a mode that can't be set must never cost the saved data.
        for file in [path.to_path_buf(), with_suffix(path, "-wal"), with_suffix(path, "-shm")] {
            let made = if file == path {
                crate::storage::files::create_private_file(&file)
            } else {
                crate::storage::files::make_private(&file)
            };
            if let Err(err) = made {
                tracing::warn!("couldn't make {} owner-only: {err}", file.display());
            }
        }
        let mut conn = Connection::open(path)?;
        configure(&conn)?;
        // Before anything is written: writing to a damaged file would only spread the damage.
        let verdict: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if verdict != "ok" {
            return Err(Damaged(verdict).into());
        }
        // Before the upgrade, which moves the text there (an attach can't happen in a transaction).
        let dir = path.parent().unwrap_or(Path::new("."));
        let saved = saved_text::Keeper::new(platform.clone(), dir);
        saved_text::attach(&conn, dir, saved.platform())?;
        if let Some(version) = schema::upgrade_from(&conn)? {
            // If an upgrade loses something it shouldn't, the windows and terminals as they were
            // are still in this copy.
            let copy = aside_path(path, &format!("v{version}"));
            conn.execute("VACUUM main INTO ?1", [copy.to_string_lossy()])
                .with_context(|| format!("keeping a copy of the database in {}", copy.display()))?;
            // SQLite makes the copy with its default mode, readable by everyone.
            if let Err(err) = crate::storage::files::make_private(&copy) {
                tracing::warn!("couldn't make {} owner-only: {err}", copy.display());
            }
            // Not the terminals' text: that moves to a file of its own, out of backups. A copy that
            // can't be cleaned isn't kept.
            match saved_text::strip_copy(&copy) {
                Ok(()) => tracing::info!(
                    "kept a copy of the schema version {version} database in {}",
                    copy.display()
                ),
                Err(err) => {
                    tracing::warn!("{err:#}; not keeping {}", copy.display());
                    for file in [copy.clone(), with_suffix(&copy, "-journal"), with_suffix(&copy, "-wal")] {
                        let _ = std::fs::remove_file(file);
                    }
                }
            }
        }
        schema::init(&mut conn, |tx| saved_text::take_moved(tx, &saved))?;
        saved.load_account(&conn)?;
        saved_text::forget_orphans(&conn);
        let store = Self { conn, saved, recovered_from: None };
        store.scrub_leftovers()?;
        Ok(store)
    }

    /// Text deleted before `secure_delete` was on (by 1.2.0 and earlier) still sits in free pages,
    /// and a WAL left by a crash or a kill holds old copies of pages: once per database a `VACUUM`
    /// rewrites the file without its free pages, and every open folds in and empties the WAL.
    fn scrub_leftovers(&self) -> Result<()> {
        if read_meta(&self.conn, SCRUBBED_KEY)?.is_none() {
            self.conn.execute_batch("VACUUM").context("rewriting the database without its free pages")?;
            self.set_meta(SCRUBBED_KEY, "1")?;
        }
        self.checkpoint()
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

    /// Fold the WAL into the database and cut it to nothing. `secure_delete` zeroes what a delete
    /// frees in the database's pages, but the WAL still holds the earlier copies of those pages
    /// (and a WAL under `journal_size_limit` is never shortened): this is what makes deleted text
    /// leave the disk. Called after terminals are forgotten or their text is cleared, and at quit;
    /// never per frame.
    pub fn checkpoint(&self) -> Result<()> {
        // A reader from outside (a backup tool, `sqlite3`) must not hold the UI up for the usual
        // 5 s: a checkpoint that can't finish now is tried again at the next one.
        self.conn.busy_timeout(CHECKPOINT_WAIT)?;
        let result = self.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get::<_, i64>(0));
        self.conn.busy_timeout(BUSY_WAIT)?;
        let busy = result.context("checkpointing the database")?;
        if busy != 0 {
            tracing::warn!("the database checkpoint could not finish: another connection is reading");
        }
        Ok(())
    }

    /// [`checkpoint`](Self::checkpoint) after a delete that is already committed: a failure is
    /// logged, not returned, since what the caller asked for was done.
    fn checkpoint_after_delete(&self) {
        if let Err(err) = self.checkpoint() {
            tracing::warn!("{err:#}");
        }
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
    conn.busy_timeout(BUSY_WAIT)?;
    // Scrollback makes the WAL grow in bursts; hand the space back after the checkpoint.
    conn.pragma_update(None, "journal_size_limit", 8 * 1024 * 1024)?;
    // Deleted rows are overwritten with zeros, not left in free pages: a closed terminal's text
    // (scrollback holds whatever was printed, tokens included) must not be readable from the file.
    conn.pragma_update(None, "secure_delete", true)?;
    // What a `VACUUM` and the upgrade's temporary tables hold (the terminals' text among it) stays
    // in memory, never in a temporary file.
    conn.pragma_update(None, "temp_store", "MEMORY")?;
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
