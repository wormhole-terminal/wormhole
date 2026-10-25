//! The copy a Probe runs from: `<data dir>/probes-approved/<id>/`.
//!
//! The approved fingerprint covers the files as they were when the sheet was shown, but a script is
//! read from disk later, so an edit between the approval and a spawn would run what nobody saw.
//! At approval the files are therefore copied, from the bytes already read and fingerprinted, into
//! a folder nothing else writes: the files are `0400` (`0500` when the original was executable) and
//! the folders `0500`. [`Snapshot::verify`] re-hashes that folder and compares it with the approved
//! fingerprint, and a later step calls it right before every spawn: a mismatch runs nothing. Either
//! alone would do; the copy closes the gap the re-hash leaves between hash and `exec`.
//!
//! A new copy is written next to the old one and swapped in by rename, so a half-written copy is
//! never the one in use. Everything that deletes works on one Probe's folder, entry by entry.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use super::store::{Approval, ApprovalStore};
use crate::platform::fs::{private_dir, private_file, set_mode};
use crate::probe::fingerprint::fingerprint;
use crate::probe::manifest::is_probe_id;
use crate::probe::read::{ProbeFolder, ReadError};
use crate::storage::files::ensure_private_dir;
use crate::storage::paths::DataRoot;

const FILE_MODE: u32 = 0o400;
const EXECUTABLE_MODE: u32 = 0o500;
const DIR_MODE: u32 = 0o500;
const WRITABLE_DIR: u32 = 0o700;
/// A copy is at most as deep as `read` allows (8) plus the Probe's folder.
const MAX_DEPTH: usize = 12;

/// Where the copy of an approved Probe is, and the fingerprint it must still have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    dir: PathBuf,
    fingerprint: String,
}

/// Why a copy cannot be used.
#[derive(Debug)]
pub enum VerifyError {
    /// There is no copy.
    Missing,
    /// It cannot be read as a Probe's folder.
    Unreadable(ReadError),
    /// It holds other bytes than were approved.
    Differs,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("the approved copy is missing"),
            Self::Unreadable(err) => write!(f, "the approved copy cannot be read: {err}"),
            Self::Differs => f.write_str("the approved copy is not what was approved"),
        }
    }
}

impl std::error::Error for VerifyError {}

impl Snapshot {
    /// The copy of `id` approved as `approval`; `None` when `id` is no Probe id, which never names a
    /// folder here.
    pub(super) fn of(root: &DataRoot, id: &str, approval: &Approval) -> Option<Self> {
        is_probe_id(id).then(|| Self {
            dir: root.probes_approved_dir().join(id),
            fingerprint: approval.fingerprint.clone(),
        })
    }

    /// The folder to run from and to use as the Probe's own folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Re-hash the copy and compare it with the approved fingerprint. At most 2 MiB are read; call
    /// it off the main thread. `Err` means the Probe runs nothing.
    pub fn verify(&self) -> Result<(), VerifyError> {
        match fs::symlink_metadata(&self.dir) {
            Ok(meta) if meta.is_dir() => {}
            _ => return Err(VerifyError::Missing),
        }
        let folder = ProbeFolder::read(&self.dir).map_err(VerifyError::Unreadable)?;
        if fingerprint(&folder.files) == self.fingerprint { Ok(()) } else { Err(VerifyError::Differs) }
    }
}

/// Make the copy of `folder`, replacing the one there. Only the folder's own bytes are written.
pub(super) fn create(root: &DataRoot, folder: &ProbeFolder) -> io::Result<()> {
    let id = &folder.manifest.probe.id;
    if !is_probe_id(id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a Probe id"));
    }
    let parent = root.probes_approved_dir();
    ensure_private_dir(&parent)?;
    let staging = parent.join(format!(".new-{id}"));
    let old = parent.join(format!(".old-{id}"));
    let target = parent.join(id);
    remove_tree(&staging)?;
    remove_tree(&old)?;

    if let Err(err) = write_copy(&staging, folder) {
        let _ = remove_tree(&staging);
        return Err(err);
    }
    let had_one = fs::symlink_metadata(&target).is_ok();
    if had_one {
        fs::rename(&target, &old).inspect_err(|_| {
            let _ = remove_tree(&staging);
        })?;
    }
    if let Err(err) = fs::rename(&staging, &target) {
        if had_one {
            let _ = fs::rename(&old, &target);
        }
        let _ = remove_tree(&staging);
        return Err(err);
    }
    if let Err(err) = remove_tree(&old) {
        // The new copy is in place; the old one is a leftover `sweep` removes.
        tracing::warn!("removing the previous copy of Probe {id}: {err}");
    }
    Ok(())
}

fn write_copy(dir: &Path, folder: &ProbeFolder) -> io::Result<()> {
    private_dir(&mut DirBuilder::new()).create(dir)?;
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for file in &folder.files {
        let relative = clean_relative(&file.path)?;
        let dest = dir.join(&relative);
        if let Some(parent) = dest.parent() {
            private_dir(DirBuilder::new().recursive(true)).create(parent)?;
            let mut up = parent;
            while up != dir && dirs.insert(up.to_path_buf()) {
                up = up.parent().unwrap_or(dir);
            }
        }
        let mut out = private_file(OpenOptions::new().write(true).create_new(true)).open(&dest)?;
        out.write_all(file.bytes())?;
        out.sync_all()?;
        set_mode(&dest, if file.is_executable() { EXECUTABLE_MODE } else { FILE_MODE })?;
    }
    // Deepest first: a folder is made read-only after what is in it was written.
    let mut dirs: Vec<PathBuf> = dirs.into_iter().collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for sub in dirs {
        set_mode(&sub, DIR_MODE)?;
    }
    set_mode(dir, DIR_MODE)
}

/// `path` as a relative path of plain names; a Probe's files already are, and the copy checks again
/// because it is the one place that writes them.
fn clean_relative(path: &str) -> io::Result<PathBuf> {
    let relative = Path::new(path);
    let plain = !path.is_empty() && relative.components().all(|c| matches!(c, Component::Normal(_)));
    if plain {
        Ok(relative.to_path_buf())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{path:?} is not a path inside the folder")))
    }
}

/// Delete `id`'s copy and anything left from an interrupted approval of it. A Probe without a copy is
/// fine; a name that is no id deletes nothing.
pub(super) fn remove(root: &DataRoot, id: &str) -> io::Result<()> {
    if !is_probe_id(id) {
        return Ok(());
    }
    let parent = root.probes_approved_dir();
    for name in [id.to_string(), format!(".new-{id}"), format!(".old-{id}")] {
        remove_tree(&parent.join(name))?;
    }
    Ok(())
}

/// Delete every entry of `probes-approved/` that `store` has no approval for: copies of Probes that
/// were removed, and leftovers.
pub(super) fn remove_unowned(root: &DataRoot, store: &ApprovalStore) -> io::Result<()> {
    let parent = root.probes_approved_dir();
    let entries = match fs::read_dir(&parent) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    let mut first_error = None;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        if name.to_str().is_some_and(|name| store.get(name).is_some()) {
            continue;
        }
        if let Err(err) = remove_tree(&entry.path()) {
            first_error.get_or_insert(err);
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// Delete `path` and what is below it, entry by entry: folders are made writable first, a link is
/// removed and never followed, and a missing path is fine.
pub(super) fn remove_tree(path: &Path) -> io::Result<()> {
    remove_below(path, 0)
}

fn remove_below(path: &Path, depth: usize) -> io::Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if !meta.is_dir() {
        return fs::remove_file(path);
    }
    if depth > MAX_DEPTH {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "a copy is not this deep"));
    }
    set_mode(path, WRITABLE_DIR)?;
    for entry in fs::read_dir(path)? {
        remove_below(&entry?.path(), depth + 1)?;
    }
    fs::remove_dir(path)
}
