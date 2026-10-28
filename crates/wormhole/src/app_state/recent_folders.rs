//! The folders the terminals were in lately, for the folder picker's "Recent" rows (core's
//! `folder_picker::RecentFolders` ranks them, by frecency).
//!
//! One list per profile, in the profile's own store under a key of its own (a `meta` row: the database's
//! layout doesn't change), so it stays on this Mac, goes with the profile and swaps with it. The list lives
//! in memory once, a global of its own (not a field of the shared state): read from the store the first
//! time a profile needs it, changed in place and written back when it changes. The picker's provider holds
//! the same list, so a visit while it is open and a forgotten folder are seen by both and nothing is ever
//! written over from an old copy.
//!
//! A terminal's folder counts as a visit when it *changes* (the window says so), not whenever the terminal
//! announces something about itself. Nothing inside the app's own data folder or a temporary folder is ever
//! kept; core's rules say the rest. It is kept only while the folder picker's flag is on, and the first time a
//! folder changes with it off the list stored for the profile is cleared: turning the feature off forgets what
//! it learned.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::{App, Global};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::folder_picker::{RecentFolders, keep_out};
use wormhole_core::platform::paths;
use wormhole_core::storage::paths::ProfileId;

use super::{AppState, app};
use crate::settings::settings_in;

/// The store's `meta` key the list is kept under.
const STORE_KEY: &str = "recent_folders";

#[derive(Default)]
struct Lists {
    /// Each profile's list, once it was needed.
    by_profile: HashMap<ProfileId, Arc<Mutex<RecentFolders>>>,
    /// The profiles whose stored list was cleared because the flag is off.
    cleared: HashSet<ProfileId>,
}

impl Global for Lists {}

/// Seconds since the epoch.
pub(crate) fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn load(state: &AppState, profile: &ProfileId) -> RecentFolders {
    let json = state.store_of(profile).and_then(|store| store.meta(STORE_KEY).ok().flatten());
    RecentFolders::from_json(json.as_deref())
}

/// What `profile`'s store keeps, for a test to see that it was written.
#[cfg(test)]
pub(crate) fn stored(state: &AppState, profile: &ProfileId) -> RecentFolders {
    load(state, profile)
}

fn save(state: &AppState, profile: &ProfileId, recents: &RecentFolders) {
    if let Some(store) = state.store_of(profile)
        && let Err(err) = store.set_meta(STORE_KEY, &recents.to_json())
    {
        tracing::warn!("saving recent folders: {err:#}");
    }
}

/// `profile`'s list, the one in memory: read from its store the first time.
pub(crate) fn shared(cx: &mut App, profile: &ProfileId) -> Arc<Mutex<RecentFolders>> {
    if let Some(list) = cx.try_global::<Lists>().and_then(|lists| lists.by_profile.get(profile)) {
        return list.clone();
    }
    let list = Arc::new(Mutex::new(load(app(cx), profile)));
    cx.default_global::<Lists>().by_profile.insert(profile.clone(), list.clone());
    list
}

/// A terminal of `profile` has moved to `folder`: a visit, unless it is a place that is never kept or the
/// folder picker is off for the profile (then nothing is kept, and what an earlier run kept is cleared, once).
pub(crate) fn visited(cx: &mut App, profile: &ProfileId, folder: &Path) {
    if !settings_in(profile, cx).feature_enabled(FeatureFlag::FolderPicker) {
        clear_once(cx, profile);
        return;
    }
    record(cx, profile, folder);
}

/// A visit to `folder`, whatever the flag says (the picker is open: it is on).
pub(crate) fn record(cx: &mut App, profile: &ProfileId, folder: &Path) {
    let list = shared(cx, profile);
    let out = keep_out(Some(app(cx).root.base()), &paths::temp());
    let Ok(mut recents) = list.lock() else { return };
    if recents.record(folder, now(), &out) {
        save(app(cx), profile, &recents);
    }
}

/// Forget the folder `key` (a path as the picker keys it). Whether it was there.
pub(crate) fn forget(cx: &mut App, profile: &ProfileId, key: &str) -> bool {
    let list = shared(cx, profile);
    let Ok(mut recents) = list.lock() else { return false };
    let was = recents.forget(key);
    if was {
        save(app(cx), profile, &recents);
    }
    was
}

/// The list stored for `profile` is emptied, the first time this is asked in a run. Reading it costs one read
/// and nothing is written when there is nothing to forget.
fn clear_once(cx: &mut App, profile: &ProfileId) {
    if !cx.default_global::<Lists>().cleared.insert(profile.clone()) {
        return;
    }
    let list = shared(cx, profile);
    let Ok(mut recents) = list.lock() else { return };
    if !recents.is_empty() {
        *recents = RecentFolders::default();
        save(app(cx), profile, &recents);
    }
}
