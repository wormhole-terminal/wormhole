//! The projects worked in lately, one list per profile in the profile's own database (the GPUI app's
//! `app_state/recent_projects.rs`: the `meta` row `recent_projects`, a JSON array of folders, the latest first, at most
//! eight), for Open Recent, the status item's menu, the palette and the welcome's places.

use wormhole_core::storage::store::Store;

/// The most a profile remembers.
const LIMIT: usize = 8;

/// The store's `meta` key the list is kept under, as in the GPUI app, so the list of one is the other's.
const STORE_KEY: &str = "recent_projects";

fn read(store: &Store) -> Vec<String> {
    store.meta(STORE_KEY).ok().flatten().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
}

/// What profile `profile` remembers (nothing for an unknown profile or outside an isolated instance).
#[uniffi::export]
pub fn recent_projects_load(profile: String) -> Vec<String> {
    crate::profile::dir_of(&profile)
        .and_then(|dir| Store::open_at(&dir).ok())
        .map(|store| read(&store))
        .unwrap_or_default()
}

/// `root` was just worked in: it leads profile `profile`'s list, which is kept. Returns the list as it is now.
#[uniffi::export]
pub fn recent_projects_remember(profile: String, root: String) -> Vec<String> {
    let Some(store) = crate::profile::dir_of(&profile).and_then(|dir| Store::open_at(&dir).ok()) else {
        return Vec::new();
    };
    let mut roots = read(&store);
    if roots.first() == Some(&root) {
        return roots;
    }
    roots.retain(|path| path != &root);
    roots.insert(0, root);
    roots.truncate(LIMIT);
    if let Err(err) = store.set_meta(STORE_KEY, &serde_json::to_string(&roots).unwrap_or_default()) {
        tracing::warn!("saving recent projects: {err:#}");
    }
    roots
}
