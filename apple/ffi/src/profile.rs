//! The profile Wormhole is in, as `profiles.json` says: the one place the objects that keep a profile's data
//! (windows, notes, usage, trust, session hosts, settings) ask which folder that is, so a switch moves them all
//! at once. What the GPUI app keeps in `active_profile`; the list and the switch are core's `ProfileList`.

use wormhole_core::config::profile::ProfileList;
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
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

/// The folder of profile `id`, for an object that follows a window to another profile; `None` outside an isolated
/// instance and for an id that is not in `profiles.json`.
pub(crate) fn dir_of(id: &str) -> Option<std::path::PathBuf> {
    isolation::require().ok()?;
    dir_in(&DataRoot::current(), id)
}

fn dir_in(root: &DataRoot, id: &str) -> Option<std::path::PathBuf> {
    let id = ProfileId::parse(id).ok()?;
    let (list, _) = ProfileList::load_or_recover(root);
    list.get(&id)?;
    Some(root.profile(&id).dir().to_path_buf())
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

#[cfg(test)]
mod tests {
    use wormhole_core::config::profile::{NewProfile, StartFrom};

    use super::*;

    #[test]
    fn a_window_follows_only_a_profile_the_list_has() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = DataRoot::new(dir.path());
        let mut list = ProfileList::default();
        let work = list
            .create(
                &root,
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
            )
            .expect("makes Work");
        list.save(&root).expect("saves the list");
        assert_eq!(dir_in(&root, work.as_str()), Some(root.profile(&work).dir().to_path_buf()));
        assert_eq!(dir_in(&root, "default"), Some(root.default_profile().dir().to_path_buf()));
        assert_eq!(dir_in(&root, "nothing-by-that-name"), None);
        assert_eq!(dir_in(&root, "../escape"), None);
    }
}
