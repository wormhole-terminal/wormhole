//! The projects worked in lately, most recently used first: Open Recent, the status item's menu, the
//! welcome's places and Settings' project list read them.
//!
//! Part of the shared state (it goes to `app_state/recent.rs`): one list per profile, kept in the
//! profile's store, so a profile switch swaps it like the rest of the profile's data. It knows
//! nothing of the menus that show it.

use std::path::PathBuf;

use wormhole_core::storage::store::Store;

/// The most a profile remembers.
const LIMIT: usize = 8;

/// The store's `meta` key the list is kept under.
const STORE_KEY: &str = "recent_projects";

/// Project roots, most recently used first.
#[derive(Debug, Default)]
pub(crate) struct RecentProjects {
    roots: Vec<PathBuf>,
}

impl RecentProjects {
    /// What a store keeps.
    pub(crate) fn load(store: Option<&Store>) -> Self {
        let roots = store
            .and_then(|s| s.meta(STORE_KEY).ok().flatten())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Self { roots }
    }

    pub(crate) fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// `root` was just worked in: it leads the list, and the list is kept in `store`.
    pub(crate) fn remember(&mut self, root: PathBuf, store: Option<&Store>) {
        if self.roots.first() == Some(&root) {
            return;
        }
        self.roots.retain(|p| p != &root);
        self.roots.insert(0, root);
        self.roots.truncate(LIMIT);
        if let Some(store) = store
            && let Err(err) =
                store.set_meta(STORE_KEY, &serde_json::to_string(&self.roots).unwrap_or_default())
        {
            tracing::warn!("saving recent projects: {err:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{LIMIT, RecentProjects};

    fn roots(recent: &RecentProjects) -> Vec<&str> {
        recent.roots().iter().map(|p| p.to_str().unwrap_or_default()).collect()
    }

    #[test]
    fn the_latest_leads_and_a_repeat_moves_up() {
        let mut recent = RecentProjects::default();
        for root in ["/a", "/b", "/c", "/a"] {
            recent.remember(PathBuf::from(root), None);
        }
        assert_eq!(roots(&recent), ["/a", "/c", "/b"]);
    }

    #[test]
    fn only_the_latest_few_are_kept() {
        let mut recent = RecentProjects::default();
        for n in 0..LIMIT + 3 {
            recent.remember(PathBuf::from(format!("/p{n}")), None);
        }
        assert_eq!(recent.roots().len(), LIMIT);
        assert_eq!(roots(&recent)[0], format!("/p{}", LIMIT + 2));
    }
}
