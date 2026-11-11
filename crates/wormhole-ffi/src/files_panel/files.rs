//! The `FilesService` object: the Files tab's disk reads, its one write (rename) and the pinned folders.
//!
//! A folder's content is hostile input (a cloned repository chooses its names and plants links), so
//! every call goes through core's hardened helpers: `listing` reads by `lstat` and never walks through a
//! link, `viewing::load` caps the read and refuses a path that resolves outside the root, `rename_within`
//! refuses what exists and anything through a link. Paths from Swift are relative to the root and plain
//! (no `..`, no leading `/`); anything else is refused before the disk is touched. The calls block on the
//! disk: Swift runs them off the main thread.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use wormhole_core::config::Config;
use wormhole_core::storage::files::{listing, load, rename_within};
use wormhole_core::view::file_tree::{Dir, ENTRY_CAP, Failure};

use crate::error::FfiError;
use crate::files_panel::files_records::{
    DirectoryRecord, FileContentRecord, FileProblem, FilesFailure, RenameFailure,
};
use crate::files_panel::listener::PanelListener;
use crate::files_panel::settings_edit::update_settings;
use crate::files_panel::watch::FolderWatch;
use crate::isolation;

/// Reads, renames and pins for the Files tab. Holds nothing between calls.
#[derive(uniffi::Object)]
pub struct FilesService;

#[uniffi::export]
impl FilesService {
    /// Fails unless this is an isolated instance of the experiment (the pins live in its `config.json`).
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, FfiError> {
        isolation::require()?;
        Ok(Arc::new(Self))
    }

    /// The folder `folder` below `root` (`""` is `root`), sorted, cleaned and capped as the tab lists it.
    pub fn list(&self, root: String, folder: String) -> Result<DirectoryRecord, FilesFailure> {
        let Some(relative) = plain(&folder) else { return Err(FilesFailure::Other) };
        let listing = listing::list_below(Path::new(&root), &relative).map_err(|err| Failure::of(&err))?;
        Ok(DirectoryRecord::from(&Dir::from_listing(listing, ENTRY_CAP)))
    }

    /// The file `path` below `root` for the viewer: capped, never followed out of `root`.
    pub fn read(&self, root: String, path: String) -> FileContentRecord {
        match plain(&path) {
            Some(relative) => load(Path::new(&root), &relative).into(),
            None => FileContentRecord::Unavailable { problem: FileProblem::Outside },
        }
    }

    /// Renames `path` to `name` in its own folder and returns its new path. A rename of a folder that
    /// is pinned (or has pinned folders below it) moves those pins with it.
    pub fn rename(&self, root: String, path: String, name: String) -> Result<String, RenameFailure> {
        let Some(from) = plain(&path) else { return Err(RenameFailure::Gone) };
        let root = PathBuf::from(root);
        let to = rename_within(&root, &from, name.as_ref()).map_err(|err| RenameFailure::from(&err))?;
        if to != from {
            move_pins(&root.join(&from), &root.join(&to));
        }
        Ok(to.to_string_lossy().into_owned())
    }

    /// The pinned folders (absolute paths) in the order they were pinned.
    pub fn pins(&self) -> Vec<String> {
        Config::load().settings.pinned_folders().iter().map(|p| p.to_string_lossy().into_owned()).collect()
    }

    /// Pins `folder` or takes it off the pins. A relative path, a full list and a no-change are no-ops.
    pub fn set_pinned(&self, folder: String, pinned: bool) {
        update_settings(|settings| settings.set_pinned(Path::new(&folder), pinned));
    }

    /// Follows `root` through the system's file events (FSEvents) so the tree is read when something
    /// changed instead of on a timer. `wake()` is called at most once between two `drain()`s of the
    /// returned watch. A root the system can't watch (a network volume, a failed stream) is still served:
    /// the watch then wakes every few seconds as a poll, and `is_live()` says which it is.
    pub fn watch(&self, root: String, listener: Arc<dyn PanelListener>) -> Arc<FolderWatch> {
        FolderWatch::start(PathBuf::from(root), listener, |_| true)
    }
}

/// `path` as a relative path made only of plain names (`""` is the root itself), else `None`.
fn plain(path: &str) -> Option<PathBuf> {
    let relative = Path::new(path);
    relative.components().all(|part| matches!(part, Component::Normal(_))).then(|| relative.to_path_buf())
}

fn move_pins(from: &Path, to: &Path) {
    if Config::load().settings.pinned_folders().iter().any(|pin| pin.starts_with(from)) {
        update_settings(|settings| settings.move_pinned(from, to));
    }
}
