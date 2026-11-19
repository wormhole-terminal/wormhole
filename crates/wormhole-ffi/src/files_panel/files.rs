//! The `FilesService` object: the Files tab's disk reads, its one write (rename) and the pinned folders.
//!
//! A folder's content is hostile input (a cloned repository chooses its names and plants links), so
//! every call goes through core's hardened helpers: `listing` reads by `lstat` and never walks through a
//! link, `viewing::load` caps the read and refuses a path that resolves outside the root, `rename_within`
//! refuses what exists and anything through a link. Paths from Swift are relative to the root and plain
//! (no `..`, no leading `/`); anything else is refused before the disk is touched. The calls block on the
//! disk: Swift runs them off the main thread. A name that is not UTF-8 crosses as a key (`names`), so such a
//! file lists, opens, renames and pins like any other.

use std::path::{Component, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::config::Config;
use wormhole_core::storage::files::{listing, load, rename_within, resolve_path_within};
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
use wormhole_core::view::file_tree::{Dir, ENTRY_CAP, Failure};

use crate::error::FfiError;
use crate::files_panel::files_records::{
    DirectoryRecord, FileContentRecord, FileProblem, FilesFailure, RenameFailure,
};
use crate::files_panel::listener::PanelListener;
use crate::files_panel::names;
use crate::files_panel::settings_edit::update_settings_of;
use crate::files_panel::watch::FolderWatch;
use crate::{isolation, profile};

/// Reads, renames and pins for the Files tab. Holds nothing between calls but whose pins it shows.
#[derive(Default, uniffi::Object)]
pub struct FilesService {
    /// The window's own profile, whose `config.json` holds its pins; `None` is the Default profile.
    profile: Mutex<Option<ProfilePaths>>,
}

#[uniffi::export]
impl FilesService {
    /// Fails unless this is an isolated instance of the experiment (the pins live in its `config.json`).
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, FfiError> {
        isolation::require()?;
        Ok(Arc::new(Self::default()))
    }

    /// The window this belongs to is of profile `profile`: the pins are read and written in that profile's
    /// settings from here on (the GPUI app's `pins.rs` uses the window's own). False (nothing changes) for a
    /// profile that is not in `profiles.json`.
    pub fn follow_profile(&self, profile: String) -> bool {
        let Ok(id) = ProfileId::parse(&profile) else { return false };
        if isolation::require().is_err() {
            return false;
        }
        let root = DataRoot::current();
        if profile::list_in(&root).get(&id).is_none() {
            return false;
        }
        *self.profile.lock().unwrap_or_else(PoisonError::into_inner) = Some(root.profile(&id));
        true
    }

    /// The folder `folder` below `root` (`""` is `root`), sorted, cleaned and capped as the tab lists it.
    pub fn list(&self, root: String, folder: String) -> Result<DirectoryRecord, FilesFailure> {
        let Some(relative) = plain(&folder) else { return Err(FilesFailure::Other) };
        let Some(root) = names::path(&root) else { return Err(FilesFailure::Gone) };
        let listing = listing::list_below(&root, &relative).map_err(|err| Failure::of(&err))?;
        Ok(DirectoryRecord::from(&Dir::from_listing(listing, ENTRY_CAP)))
    }

    /// What the folder or file `path` below `root` looks like from outside (core's `stamp_below`: one `lstat`, never
    /// through a link below the root), as a word that is the same until it changes: a folder's until an entry is
    /// added, removed or renamed in it, a file's until it is written. `None` when nothing is there. Taken before a
    /// read, an unchanged word means the read can be skipped.
    pub fn stamp(&self, root: String, path: String) -> Option<String> {
        let stamp = listing::stamp_below(&names::path(&root)?, &plain(&path)?).ok()?;
        Some(format!("{stamp:?}"))
    }

    /// The file `path` below `root` for the viewer: capped, never followed out of `root`.
    pub fn read(&self, root: String, path: String) -> FileContentRecord {
        match (names::path(&root), plain(&path)) {
            (Some(root), Some(relative)) => load(&root, &relative).into(),
            _ => FileContentRecord::Unavailable { problem: FileProblem::Outside },
        }
    }

    /// Renames `path` to `name` in its own folder and returns its new path. A rename of a folder that
    /// is pinned (or has pinned folders below it) moves those pins with it.
    pub fn rename(&self, root: String, path: String, name: String) -> Result<String, RenameFailure> {
        let Some(from) = plain(&path) else { return Err(RenameFailure::Gone) };
        let Some(root) = names::path(&root) else { return Err(RenameFailure::Gone) };
        let to = rename_within(&root, &from, name.as_ref()).map_err(|err| RenameFailure::from(&err))?;
        if to != from {
            let (from, to) = (root.join(&from), root.join(&to));
            let paths = self.paths();
            if Config::load_for(&paths).settings.pinned_folders().iter().any(|pin| pin.starts_with(&from)) {
                update_settings_of(&paths, |settings| settings.move_pinned(&from, &to));
            }
        }
        Ok(names::path_key(&to))
    }

    /// The real path of the file (`folder`: the folder) `path` names below `root`, for handing it to another app:
    /// `None` when it is gone, is not of that kind any more, or leads out of `root` by a link planted since it
    /// was listed (the GPUI app's `open::real_path`). The result is what was checked, as a key (`names`) where it
    /// is not text: `open_file_run` and `DiskName.url` read it back.
    pub fn resolve(&self, root: String, path: String, folder: bool) -> Option<String> {
        let relative = plain(&path)?;
        let real = resolve_path_within(&names::path(&root)?, &relative).ok()?;
        let meta = std::fs::metadata(&real).ok()?;
        let of_kind = if folder { meta.is_dir() } else { meta.is_file() };
        of_kind.then(|| names::path_key(&real))
    }

    /// The pinned folders (absolute paths) in the order they were pinned.
    pub fn pins(&self) -> Vec<String> {
        let config = Config::load_for(&self.paths());
        config.settings.pinned_folders().iter().map(|pin| names::path_key(pin)).collect()
    }

    /// Pins `folder` or takes it off the pins. A relative path, a full list and a no-change are no-ops.
    pub fn set_pinned(&self, folder: String, pinned: bool) {
        let Some(folder) = names::path(&folder) else { return };
        update_settings_of(&self.paths(), |settings| settings.set_pinned(&folder, pinned));
    }

    /// Follows `root` through the system's file events (FSEvents) so the tree is read when something
    /// changed instead of on a timer. `wake()` is called at most once between two `drain()`s of the
    /// returned watch. A root the system can't watch (a network volume, a failed stream) is still served:
    /// the watch then wakes every few seconds as a poll, and `is_live()` says which it is.
    pub fn watch(&self, root: String, listener: Arc<dyn PanelListener>) -> Arc<FolderWatch> {
        // A key that is not one is watched as written: nothing is there, so the watch only polls.
        FolderWatch::start(names::path(&root).unwrap_or_else(|| PathBuf::from(root)), listener, |_| true)
    }
}

/// `path` (text or a key, `names`) as a relative path made only of plain names (`""` is the root itself), else `None`.
fn plain(path: &str) -> Option<PathBuf> {
    let relative = names::path(path)?;
    relative.components().all(|part| matches!(part, Component::Normal(_))).then_some(relative)
}

impl FilesService {
    /// The profile whose settings hold the pins.
    fn paths(&self) -> ProfilePaths {
        let followed = self.profile.lock().unwrap_or_else(PoisonError::into_inner).clone();
        followed.unwrap_or_else(|| DataRoot::current().default_profile())
    }
}
