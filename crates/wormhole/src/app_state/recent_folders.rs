//! The folders the terminals were in lately, for the folder picker's "Recent" rows (core's
//! `folder_picker::RecentFolders` ranks them, by frecency).
//!
//! One list per profile, in the profile's own store under a key of its own (a `meta` row: the database's
//! layout doesn't change), so it stays on this Mac, goes with the profile and swaps with it. It is not a field of
//! the shared state: it is read when it is needed (the picker opens, a terminal's folder changes) and written
//! when it changes, which is a few times a minute at most.
//!
//! Nothing inside the app's own data folder or a temporary folder is ever kept; core's rules say the rest.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use wormhole_core::folder_picker::{RecentFolders, keep_out};
use wormhole_core::platform::paths;
use wormhole_core::storage::paths::ProfileId;

use super::AppState;

/// The store's `meta` key the list is kept under.
const STORE_KEY: &str = "recent_folders";

/// Seconds since the epoch.
pub(crate) fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// What `profile`'s store keeps.
pub(crate) fn load(state: &AppState, profile: &ProfileId) -> RecentFolders {
    let json = state.store_of(profile).and_then(|store| store.meta(STORE_KEY).ok().flatten());
    RecentFolders::from_json(json.as_deref())
}

/// Keep `recents` in `profile`'s store.
pub(crate) fn save(state: &AppState, profile: &ProfileId, recents: &RecentFolders) {
    if let Some(store) = state.store_of(profile)
        && let Err(err) = store.set_meta(STORE_KEY, &recents.to_json())
    {
        tracing::warn!("saving recent folders: {err:#}");
    }
}

/// A terminal of `profile` is in `folder`: it counts as a visit, unless it is a place that is never kept.
pub(crate) fn record(state: &AppState, profile: &ProfileId, folder: &Path) {
    let mut recents = load(state, profile);
    let out = keep_out(Some(state.root.base()), &paths::temp());
    if recents.record(folder, now(), &out) {
        save(state, profile, &recents);
    }
}
