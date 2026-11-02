//! The app's side of a terminal's checkpoint (the format and the host's writing are `wormhole_term::checkpoint`
//! and `wormhole-host`): reading one back into a new terminal after a restart of the Mac, and looking after the
//! folder they live in. The host never deletes a checkpoint, so everything that removes one is here: a terminal
//! closed, its buffer cleared, a host forgotten, and a sweep at launch of what is old or too much.
//!
//! What a restore takes from a checkpoint is the screen, drawn above a fresh shell with a divider that says
//! when. The folder and the shell the new terminal runs are not taken from the file: they come from the
//! terminal's saved record, through the same path as any new terminal, so a trusted project's rules apply and
//! an untrusted one's do not. There is no environment on disk to take.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use wormhole_term::checkpoint::{self, Issue, Key, ReadError, divider, safe_replay};

use crate::platform::clock::local_date_time;

/// A checkpoint older than this goes at launch: what it holds is a week stale.
pub const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// All checkpoints together may take this much; past it the oldest go first.
pub const BUDGET_BYTES: u64 = 256 * 1024 * 1024;

/// `<data dir>/sessions.noindex`.
pub fn sessions_dir(data_dir: &Path) -> PathBuf {
    checkpoint::sessions_dir(data_dir)
}

/// A checkpoint ready to draw above a new shell.
#[derive(Debug, PartialEq, Eq)]
pub struct Seed {
    /// The screen as styled VT, ending with the divider that says when it was kept.
    pub text: String,
    /// When the checkpoint was written (Unix seconds).
    pub time: u64,
    /// How the program ended, if it did (the last generation a host writes).
    pub ended: Option<String>,
}

/// Why a terminal is restored from its text instead.
#[derive(Debug, PartialEq, Eq)]
pub enum Skipped {
    /// There is no checkpoint for it.
    Missing,
    /// Written by a newer Wormhole than this one; left alone.
    Newer,
    /// Nothing in it reads.
    Unusable,
    /// It has no screen that opens: no key was given to the host, or ours is another, or the file is damaged.
    NoScreen(Option<Issue>),
}

/// The screen of the checkpoint of host `host_id` in `sessions_dir`, opened with `key` and drawn with a divider
/// at the time it was written.
pub fn read(sessions_dir: &Path, host_id: &str, key: Option<&[u8; 32]>) -> Result<Seed, Skipped> {
    let key = key.map(|k| Key::new(*k));
    let read =
        checkpoint::read_latest(&checkpoint::store::dir_in(sessions_dir, host_id), host_id, key.as_ref())
            .map_err(|e| match e {
                ReadError::Missing => Skipped::Missing,
                ReadError::Newer(_) => Skipped::Newer,
                ReadError::Unusable => Skipped::Unusable,
            })?;
    let Some(screen) = read.screen else { return Err(Skipped::NoScreen(read.issue)) };
    // Whatever the file holds, only text and SGR get in; a restored terminal obeys nothing of its history.
    let mut text = String::from_utf8_lossy(&safe_replay(&screen)).into_owned();
    if !text.ends_with('\n') {
        text.push_str("\r\n");
    }
    let when = local_date_time(read.manifest.time).unwrap_or_default();
    text.push_str(&format!("\x1b[2m{}\x1b[0m\r\n", divider(&when)));
    Ok(Seed { text, time: read.manifest.time, ended: read.manifest.ended })
}

/// Make `options` restore the screen of the checkpoint of host `host_id` (the host the terminal ran in before
/// the restart) in place of its saved text: the text becomes the styled screen with its divider. Whether it did;
/// on any failure the options are as they were, and the terminal is restored from its text as before. Nothing
/// but the screen is taken: the folder, the shell and the environment stay what the terminal's record and its
/// project say.
pub fn seed_options(
    sessions_dir: &Path,
    host_id: &str,
    key: Option<&[u8; 32]>,
    options: &mut super::SpawnOptions,
) -> bool {
    match read(sessions_dir, host_id, key) {
        Ok(seed) => {
            tracing::info!(host_id, ended = ?seed.ended, "restoring a terminal from its checkpoint");
            options.restore = Some(seed.text);
            options.restore_styled = true;
            true
        }
        Err(why) => {
            tracing::debug!(host_id, ?why, "no checkpoint to restore from, the saved text is used");
            false
        }
    }
}

/// A terminal was restored from the checkpoint of host `old` and now runs in host `new`, which was asked to write
/// its own (with `key`). If that has landed (`new`'s newest generation reads), `old`'s is spent and removed:
/// true. Until then both stay, so that another restart or a crash restores from `old` again.
pub fn forget_when_confirmed(sessions_dir: &Path, old: &str, new: &str, key: Option<&[u8; 32]>) -> bool {
    let key = key.map(|k| Key::new(*k));
    let confirmed =
        checkpoint::store::is_readable(&checkpoint::store::dir_in(sessions_dir, new), new, key.as_ref());
    if confirmed {
        forget(sessions_dir, old);
    }
    confirmed
}

/// Make the folder the checkpoints live in: `0700`, and out of backups (`exclude`, which the caller gives:
/// the saved text's platform does the same for its file). A failure to exclude is only worth a warning; one to
/// make the folder is the error.
pub fn prepare_dir(sessions_dir: &Path, exclude: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<()> {
    use wormhole_term::sys::fs::private_dir;
    let mut builder = fs::DirBuilder::new();
    private_dir(builder.recursive(true)).create(sessions_dir)?;
    if let Err(err) = exclude(sessions_dir) {
        tracing::warn!("the checkpoint folder could not be kept out of backups: {err}");
    }
    Ok(())
}

/// Remove the checkpoint of host `host_id`. For a terminal that is closed, cleared or forgotten.
pub fn forget(sessions_dir: &Path, host_id: &str) {
    if wormhole_term::hosted::meta::valid_id(host_id) {
        checkpoint::forget(&checkpoint::store::dir_in(sessions_dir, host_id));
    }
}

/// What the sweep found about one checkpoint folder.
struct Found {
    id: String,
    modified: SystemTime,
    bytes: u64,
}

fn survey(sessions_dir: &Path) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(sessions_dir) else { return Vec::new() };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else { continue };
        if !wormhole_term::hosted::meta::valid_id(&id) || !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let (mut modified, mut bytes) = (SystemTime::UNIX_EPOCH, 0);
        for file in fs::read_dir(entry.path()).into_iter().flatten().flatten() {
            if let Ok(meta) = file.metadata() {
                bytes += meta.len();
                modified = modified.max(meta.modified().unwrap_or(SystemTime::UNIX_EPOCH));
            }
        }
        found.push(Found { id, modified, bytes });
    }
    found
}

/// Remove the checkpoints older than [`MAX_AGE`], then the oldest while all together take more than
/// [`BUDGET_BYTES`]. `running` are hosts that still run: their checkpoints are in use and stay. The ids removed.
pub fn sweep(sessions_dir: &Path, running: &HashSet<String>, now: SystemTime) -> Vec<String> {
    sweep_with(sessions_dir, running, now, MAX_AGE, BUDGET_BYTES)
}

fn sweep_with(
    sessions_dir: &Path,
    running: &HashSet<String>,
    now: SystemTime,
    max_age: Duration,
    budget: u64,
) -> Vec<String> {
    let mut found = survey(sessions_dir);
    found.sort_by_key(|f| f.modified);
    let mut removed = Vec::new();
    let mut total: u64 = found.iter().map(|f| f.bytes).sum();
    for f in &found {
        if running.contains(&f.id) {
            continue;
        }
        let old = now.duration_since(f.modified).is_ok_and(|age| age > max_age);
        if old || total > budget {
            forget(sessions_dir, &f.id);
            total = total.saturating_sub(f.bytes);
            removed.push(f.id.clone());
        }
    }
    removed
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
