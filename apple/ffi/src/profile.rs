//! The profile Wormhole is in, as `profiles.json` says: the one place the objects that keep a profile's data
//! (windows, notes, usage, trust, session hosts, settings) ask which folder that is, so a switch moves them all
//! at once. What the GPUI app keeps in `active_profile`; the list and the switch are core's `ProfileList`.

use wormhole_core::config::profile::ProfileList;
use wormhole_core::storage::paths::{DataRoot, ProfilePaths};
use wormhole_core::storage::store::Store;

use crate::error::FfiError;
use crate::isolation;

/// The active profile's files under `root`: Default's (the data folder itself) when the list does not read.
pub(crate) fn active_in(root: &DataRoot) -> ProfilePaths {
    let (list, _) = ProfileList::load_or_recover(root);
    root.profile(list.active_id())
}

/// The folder an object keeps the shown profile's data in. Isolated instances only.
pub(crate) fn active_dir() -> Result<std::path::PathBuf, FfiError> {
    isolation::require()?;
    Ok(active_in(&DataRoot::current()).dir().to_path_buf())
}

/// The first terminal and window ids free in every profile's database, so an id is never taken twice by two
/// profiles that are both open (a terminal's id is what hooks report).
pub(crate) fn first_free_ids(root: &DataRoot) -> (i64, i64) {
    let (list, _) = ProfileList::load_or_recover(root);
    let mut ids = (1, 1);
    for profile in list.profiles() {
        let paths = root.profile(&profile.id);
        if !Store::exists_for(&paths) {
            continue;
        }
        match Store::open_for(&paths).and_then(|store| store.next_ids()) {
            Ok((terminal, window)) => ids = (ids.0.max(terminal), ids.1.max(window)),
            Err(err) => tracing::warn!("reading the ids of profile {}: {err:#}", profile.id),
        }
    }
    ids
}

/// The profile Wormhole is in: its id and the folder that holds its `config.json`, `themes/`, `fonts/` and database.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ActiveProfile {
    pub id: String,
    pub dir: String,
}

#[uniffi::export]
pub fn active_profile() -> Result<ActiveProfile, FfiError> {
    isolation::require()?;
    let paths = active_in(&DataRoot::current());
    Ok(ActiveProfile { id: paths.id().to_string(), dir: paths.dir().to_string_lossy().into_owned() })
}
