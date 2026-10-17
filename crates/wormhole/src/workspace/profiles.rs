//! Profiles in the app (RF-21): which one Wormhole is in, what each one keeps while it is not the
//! shown one, and where a window's data goes.
//!
//! One profile is shown at a time and its state is the app's own (`AppState.store`, `hub`, the
//! `SettingsStore` global, …), so everything written for a single profile keeps working. Every
//! other profile that has been opened is parked in a [`ProfileSlot`]; switching swaps the two
//! (`switch_profile`), so no call site has to know which profile it is in. A window knows its
//! profile (`Workspace.profile`) and saves into that profile's database, shown or parked.

use std::collections::HashMap;

use wormhole_core::git::worktree::Worktrees;
use wormhole_core::layout::RememberedPanes;
use wormhole_core::onboarding::DismissedTips;
use wormhole_core::profile::{DataRoot, ProfileId, ProfileList};
use wormhole_core::store::Store;

use super::app::{AppState, OpenWindow};
use super::*;

impl AppState {
    /// The profile Wormhole is in.
    pub(crate) fn active_profile(&self) -> &ProfileId {
        self.profiles.active_id()
    }

    /// The database of `profile`: the live one when it is the shown profile, else the parked one.
    pub(crate) fn store_of(&self, profile: &ProfileId) -> Option<&Store> {
        if profile == self.active_profile() {
            self.store.as_ref()
        } else {
            self.parked.get(profile).and_then(|slot| slot.store.as_ref())
        }
    }

    fn store_of_mut(&mut self, profile: &ProfileId) -> Option<&mut Store> {
        if profile == self.profiles.active_id() {
            self.store.as_mut()
        } else {
            self.parked.get_mut(profile).and_then(|slot| slot.store.as_mut())
        }
    }

    /// The project folders (preferences, trust, notes sync) of `profile`.
    pub(super) fn hub_of_mut(&mut self, profile: &ProfileId) -> Option<&mut project::ProjectHub> {
        if profile == self.profiles.active_id() {
            Some(&mut self.hub)
        } else {
            self.parked.get_mut(profile).map(|slot| &mut slot.hub)
        }
    }

    pub(crate) fn save_window(&mut self, profile: &ProfileId, record: &WindowRecord) {
        if let Some(store) = self.store_of_mut(profile)
            && let Err(err) = store.save_window(record)
        {
            tracing::error!("saving window {} failed: {err:#}", record.id);
        }
    }

    /// A window was closed on purpose: its sessions go with it, from its own profile's database.
    pub(crate) fn forget_window(&mut self, profile: &ProfileId, id: i64) {
        if let Some(store) = self.store_of_mut(profile)
            && let Err(err) = store.delete_window(id)
        {
            tracing::error!("forgetting window {id} failed: {err:#}");
        }
    }

    /// Ids for sessions (terminals) and splits. One counter for every profile, so a terminal's id
    /// (what hooks report, what `find_session` searches for across windows) never repeats.
    pub(crate) fn alloc_id_in(&mut self, profile: &ProfileId) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        // Belt and braces with `ids_start_at`: a fresh id never comes with a notes base.
        if let Some(hub) = self.hub_of_mut(profile) {
            hub.id_allocated(id);
        }
        id
    }

    /// The windows of `profile`, shown or not.
    pub(crate) fn windows_of<'a>(&'a self, profile: &'a ProfileId) -> impl Iterator<Item = &'a OpenWindow> {
        self.windows.iter().filter(move |w| &w.profile == profile)
    }
}

/// What a profile keeps while another is shown: everything `AppState` holds per profile, and
/// its settings (the `SettingsStore` global is the shown profile's).
pub(super) struct ProfileSlot {
    pub store: Option<Store>,
    pub tips: DismissedTips,
    pub hub: project::ProjectHub,
    pub recent: Vec<PathBuf>,
    pub usage: Vec<wormhole_core::usage::Usage>,
    pub worktrees: Worktrees,
    pub last_panes: RememberedPanes,
    pub settings: SettingsStore,
}

pub(super) type Parked = HashMap<ProfileId, ProfileSlot>;

/// The first ids free in every profile's database: one counter serves them all, so a terminal
/// or window id never repeats across profiles. `live` is the shown profile's open database.
pub(super) fn first_free_ids(root: &DataRoot, list: &ProfileList, live: Option<&Store>) -> (i64, i64) {
    let mut ids = live.and_then(|s| s.next_ids().ok()).unwrap_or((1, 1));
    for profile in list.profiles().iter().filter(|p| &p.id != list.active_id()) {
        let paths = root.profile(&profile.id);
        if !Store::exists_for(&paths) {
            continue;
        }
        // Opened for its numbers only and closed again: it is opened for good when switched to.
        match Store::open_for(&paths).and_then(|s| s.next_ids()) {
            Ok((terminal, window)) => ids = (ids.0.max(terminal), ids.1.max(window)),
            Err(err) => tracing::warn!("reading the ids of profile {}: {err:#}", profile.id),
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use wormhole_core::profile::{NewProfile, StartFrom};
    use wormhole_core::store::{SessionRecord, WindowRecord};

    use super::{DataRoot, ProfileList, Store, first_free_ids};

    fn window(id: i64, session: i64) -> WindowRecord {
        WindowRecord { id, sessions: vec![SessionRecord::new(session, None, None)], ..Default::default() }
    }

    fn scratch(name: &str) -> DataRoot {
        let dir = std::env::temp_dir().join(format!("wormhole-profiles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        DataRoot::new(dir)
    }

    #[test]
    fn ids_never_collide_across_profiles_and_a_window_stays_in_its_profile() {
        let root = scratch("ids");
        let mut list = ProfileList::default();
        let work = list
            .create(
                &root,
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
            )
            .expect("creates the profile");
        let mut default_store = Store::open_for(&root.default_profile()).expect("opens Default's database");
        let mut work_store = Store::open_for(&root.profile(&work)).expect("opens Work's database");
        default_store.save_window(&window(1, 5)).expect("saves in Default");
        work_store.save_window(&window(7, 40)).expect("saves in Work");
        drop(work_store);

        // Default is shown: the counters start past Work's ids as well.
        let (terminal, window_id) = first_free_ids(&root, &list, Some(&default_store));
        assert!(terminal > 40 && window_id > 7, "got {terminal}, {window_id}");

        // A window saved in Work is not loaded when Default opens.
        let loaded = default_store.load().expect("loads Default");
        assert_eq!(loaded.iter().map(|w| w.id).collect::<Vec<_>>(), vec![1]);
        let _ = std::fs::remove_dir_all(root.base());
    }
}
