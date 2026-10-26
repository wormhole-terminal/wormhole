//! The copy a Probe runs from: `<data dir>/probes-approved/<id>/`.
//!
//! The approved fingerprint covers the files as they were when the sheet was shown, but a script is
//! read from disk later, so an edit between the approval and a spawn would run what nobody saw.
//! At approval the files are therefore copied, from the bytes already read and fingerprinted, into
//! a folder nothing else writes: the files are `0400` (`0500` when the original was executable) and
//! the folders `0500`. [`Snapshot::verify`] re-hashes that folder and compares it with the approved
//! fingerprint: a mismatch runs nothing. Either alone would do; the copy closes the gap the re-hash
//! leaves between hash and `exec`. Later steps do not hold a `Snapshot` of their own: they ask
//! [`runnable`](super::runnable), which also requires the Probe's live state to be `On`.
//!
//! A new copy is replaced in two steps, so that a failed approval leaves the old one as it was:
//! [`stage`] writes `.new-<id>`, [`Staged::commit`] moves the old copy to `.old-<id>` and the new one in,
//! and [`Committed::finish`] deletes the old one only once the approval is saved ([`Committed::rollback`]
//! puts it back otherwise). Everything that deletes works on one Probe's folder under a `probes-approved/`
//! that is a real folder of this user, never a link.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use super::store::{Approval, ApprovalStore};
use crate::platform::fs::{private_dir, private_file, set_dir_mode_no_follow, set_mode};
use crate::platform::user::UserId;
use crate::probe::fingerprint::fingerprint;
use crate::probe::manifest::is_probe_id;
use crate::probe::read::ProbeFolder;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// There is no copy.
    Missing,
    /// It cannot be read as a Probe's folder; what the read said, in English.
    Unreadable(String),
    /// It holds other bytes than were approved.
    Differs,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("the approved copy is missing"),
            Self::Unreadable(why) => write!(f, "the approved copy cannot be read: {why}"),
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

    /// Whether the copy's folder is there (its bytes are [`Self::verify`]'s).
    pub(super) fn exists(&self) -> bool {
        fs::symlink_metadata(&self.dir).is_ok_and(|meta| meta.is_dir())
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
        let folder = ProbeFolder::read(&self.dir).map_err(|err| VerifyError::Unreadable(err.to_string()))?;
        if fingerprint(&folder.files) == self.fingerprint { Ok(()) } else { Err(VerifyError::Differs) }
    }
}

/// `probes-approved/` when it is there, and only when it is a real folder of this user: a link at its
/// name (`ln -s ~/Documents …`) would make everything below delete what it points at.
fn checked_parent(root: &DataRoot) -> io::Result<Option<PathBuf>> {
    let parent = root.probes_approved_dir();
    match fs::symlink_metadata(&parent) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
        Ok(meta) if meta.is_dir() && UserId::owner_of(&meta) == UserId::current() => Ok(Some(parent)),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is not a folder of this user, so nothing is changed in it", parent.display()),
        )),
    }
}

/// A new copy written, not yet in place.
pub(super) struct Staged {
    parent: PathBuf,
    id: String,
}

/// A new copy in place, with the old one (if any) set aside as `.old-<id>` until the approval is saved.
pub(super) struct Committed {
    parent: PathBuf,
    id: String,
    had_old: bool,
}

/// Write the copy of `folder` as `.new-<id>`. Only the folder's own bytes are written.
pub(super) fn stage(root: &DataRoot, folder: &ProbeFolder) -> io::Result<Staged> {
    let id = &folder.manifest.probe.id;
    if !is_probe_id(id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a Probe id"));
    }
    let parent = root.probes_approved_dir();
    ensure_private_dir(&parent)?;
    let staging = parent.join(format!(".new-{id}"));
    remove_tree(&staging)?;
    if let Err(err) = write_copy(&staging, folder) {
        let _ = remove_tree(&staging);
        return Err(err);
    }
    Ok(Staged { parent, id: id.clone() })
}

impl Staged {
    /// Swap the staged copy in: `<id>` becomes `.old-<id>` and `.new-<id>` becomes `<id>`. On an error
    /// the old copy is where it was.
    pub(super) fn commit(self) -> io::Result<Committed> {
        let (staging, old, target) = names(&self.parent, &self.id);
        let had_old = fs::symlink_metadata(&target).is_ok();
        if had_old {
            // A leftover of an earlier approval that was interrupted.
            let _ = remove_tree(&old);
            if let Err(err) = fs::rename(&target, &old) {
                let _ = remove_tree(&staging);
                return Err(err);
            }
        }
        if let Err(err) = fs::rename(&staging, &target) {
            if had_old {
                let _ = fs::rename(&old, &target);
            }
            let _ = remove_tree(&staging);
            return Err(err);
        }
        Ok(Committed { parent: self.parent, id: self.id, had_old })
    }
}

impl Committed {
    /// The approval is saved: the old copy is not needed any more.
    pub(super) fn finish(self) {
        let (_, old, _) = names(&self.parent, &self.id);
        if let Err(err) = remove_tree(&old) {
            // The new copy is in place; the old one is a leftover `sweep` removes.
            tracing::warn!("removing the previous copy of Probe {}: {err}", self.id);
        }
    }

    /// The approval could not be saved: the old copy comes back, or there is none.
    pub(super) fn rollback(self) {
        let (_, old, target) = names(&self.parent, &self.id);
        let restored =
            remove_tree(&target).and_then(|()| if self.had_old { fs::rename(&old, &target) } else { Ok(()) });
        if let Err(err) = restored {
            tracing::warn!("putting back the copy of Probe {}: {err}", self.id);
        }
    }
}

fn names(parent: &Path, id: &str) -> (PathBuf, PathBuf, PathBuf) {
    (parent.join(format!(".new-{id}")), parent.join(format!(".old-{id}")), parent.join(id))
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
    let Some(parent) = checked_parent(root)? else { return Ok(()) };
    if !is_probe_id(id) {
        return Ok(());
    }
    let (staging, old, target) = names(&parent, id);
    for path in [target, staging, old] {
        remove_tree(&path)?;
    }
    Ok(())
}

/// Tidy `probes-approved/` against `store`: delete every entry `store` has nothing stored for (copies of
/// Probes that were removed, leftovers), except a `.old-<id>` of an approved Probe whose `<id>` is gone:
/// that is the copy an interrupted approval set aside, and it is put back. Whether an entry is sealed does not
/// matter here, so tidying never needs the key.
pub(super) fn tidy(root: &DataRoot, store: &ApprovalStore) -> io::Result<()> {
    let Some(parent) = checked_parent(root)? else { return Ok(()) };
    let mut first_error = None;
    for entry in fs::read_dir(&parent)?.filter_map(Result::ok) {
        let name = entry.file_name();
        let name = name.to_str();
        if name.is_some_and(|name| store.contains(name)) {
            continue;
        }
        let restore = name
            .and_then(|name| name.strip_prefix(".old-"))
            .filter(|id| store.contains(id) && fs::symlink_metadata(parent.join(id)).is_err());
        let done = match restore {
            Some(id) => fs::rename(entry.path(), parent.join(id)),
            None => remove_tree(&entry.path()),
        };
        if let Err(err) = done {
            first_error.get_or_insert(err);
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// Delete `path` and what is below it. Folders are made writable first, each through a descriptor opened
/// without following a link, and the deletion is `remove_dir_all`, which does not follow links either
/// (since the CVE-2022-21658 fix). That is enough here only because every caller passes a validated Probe
/// id (or a `.new-`/`.old-` of one) under a [`checked_parent`], so no component above the folder is a link.
/// A missing path is fine.
pub(super) fn remove_tree(path: &Path) -> io::Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if !meta.is_dir() {
        return fs::remove_file(path);
    }
    make_writable(path, 0)?;
    fs::remove_dir_all(path)
}

fn make_writable(dir: &Path, depth: usize) -> io::Result<()> {
    if depth > MAX_DEPTH {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "a copy is not this deep"));
    }
    set_dir_mode_no_follow(dir, WRITABLE_DIR)?;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            make_writable(&entry.path(), depth + 1)?;
        }
    }
    Ok(())
}
