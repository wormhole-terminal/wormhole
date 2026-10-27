//! Settings across a profile switch: the global holds the shown profile's store, and a switch
//! puts another profile's store in its place. The stores of the other open profiles are kept here
//! too ([`ParkedSettings`]), so a window in another profile finds its own without settings knowing
//! the app state that opens and switches profiles (`app_state::profiles`, which keeps the rest of
//! each profile's data beside them).

use std::collections::HashMap;

use gpui_kit::{App, Global};
use wormhole_core::config::Settings;
use wormhole_core::storage::paths::ProfileId;

use super::{SettingsStore, UiColors};

/// The settings stores of the profiles that are open but not shown, by profile: what their odd
/// windows draw with. The shown profile's store is the [`SettingsStore`] global, never one of these.
pub struct ParkedSettings {
    /// The profile shown. Checked first, so a window of the shown profile pays one comparison
    /// and no lookup.
    shown: ProfileId,
    stores: HashMap<ProfileId, SettingsStore>,
}

impl Global for ParkedSettings {}

impl Default for ParkedSettings {
    /// Nothing parked, with Default shown (as a fresh profile list has it).
    fn default() -> Self {
        Self::new(ProfileId::default_profile())
    }
}

impl ParkedSettings {
    pub fn new(shown: ProfileId) -> Self {
        Self { shown, stores: HashMap::new() }
    }

    /// The store of `profile` when it is open and not shown.
    pub fn get(&self, profile: &ProfileId) -> Option<&SettingsStore> {
        if *profile == self.shown { None } else { self.stores.get(profile) }
    }

    pub(crate) fn get_mut(&mut self, profile: &ProfileId) -> Option<&mut SettingsStore> {
        if *profile == self.shown { None } else { self.stores.get_mut(profile) }
    }

    pub fn is_empty(&self) -> bool {
        self.stores.is_empty()
    }

    /// Keep `store` for `profile`, which is not the one shown.
    pub fn park(&mut self, profile: ProfileId, store: SettingsStore) {
        // `get` skips the shown profile, so a store parked under it would never be read.
        debug_assert!(profile != self.shown, "the shown profile's store is the global, never parked");
        self.stores.insert(profile, store);
    }

    /// `profile` was closed (deleted): its store goes.
    pub fn remove(&mut self, profile: &ProfileId) {
        self.stores.remove(profile);
    }

    /// `profile` is the shown one from now on: its parked store, if it had one, is handed back to
    /// be swapped in (`SettingsStore::swap_in`).
    pub fn show(&mut self, profile: &ProfileId) -> Option<SettingsStore> {
        self.shown = profile.clone();
        self.stores.remove(profile)
    }

    /// Every parked store keeps up with the system and its files (`SettingsStore::follow`).
    /// Returns whether any look changed.
    pub fn follow(&mut self, system: super::SystemLook) -> bool {
        let mut moved = false;
        for store in self.stores.values_mut() {
            moved |= store.follow(system);
        }
        moved
    }
}

impl SettingsStore {
    /// Make `next` (another profile's settings) the store in force and hand back the one it
    /// replaces, to be parked. What the system said carries over, and every revision moves on
    /// from the outgoing store's, so no view mistakes the new profile's look for the one it drew.
    /// Keep a parked store current (its odd windows draw with it): what the system says now, and
    /// its `config.json` if that changed on disk. Not published to GPUI Kit's theme, which is the
    /// shown profile's. Returns whether anything moved.
    pub fn follow(&mut self, system: super::SystemLook) -> bool {
        if self.system != system {
            self.take_system(system);
        } else if !self.config.poll() {
            return false;
        }
        self.derive().any()
    }

    /// Change a parked store's settings in a test, as a settings change would.
    #[cfg(test)]
    pub(crate) fn restyle(&mut self, f: impl FnOnce(&mut Settings)) {
        self.config.update(f);
        self.derive();
    }

    pub fn swap_in(cx: &mut App, mut next: SettingsStore) -> SettingsStore {
        let live = cx.global_mut::<SettingsStore>();
        next.system = live.system;
        next.revision = live.revision + 1;
        next.revisions = super::Revisions {
            settings: live.revisions.settings + 1,
            theme: live.revisions.theme + 1,
            fonts: live.revisions.fonts + 1,
            icons: live.revisions.icons + 1,
            features: live.revisions.features + 1,
        };
        let old = std::mem::replace(live, next);
        // Reduce Transparency and Increase Contrast shape the settings in force; the new store
        // has not seen them yet. Its own revisions only move on if that changes something.
        let changed = live.derive();
        Self::publish(changed, cx);
        old
    }
}

/// The settings store a window in `profile` draws with: the global (the shown profile's) unless
/// the window is in another profile whose store is parked (the odd window). A normal window pays
/// one string comparison. Views pass their own profile, never an ambient "current" one: a cached
/// or separately dirtied child view renders without its window's root, and event handlers have
/// no render at all.
pub fn store_in<'a>(profile: &ProfileId, cx: &'a App) -> &'a SettingsStore {
    if let Some(parked) = cx.try_global::<ParkedSettings>().and_then(|parked| parked.get(profile)) {
        return parked;
    }
    cx.global::<SettingsStore>()
}

/// Whether a window in `profile` draws with the global store, the one [`super::update_settings`]
/// changes: the shown profile's. A window in another profile has its own, parked.
pub fn edits_in(profile: &ProfileId, cx: &App) -> bool {
    cx.try_global::<ParkedSettings>().and_then(|parked| parked.get(profile)).is_none()
}

/// [`super::settings`] for a window in `profile`.
pub fn settings_in<'a>(profile: &ProfileId, cx: &'a App) -> &'a Settings {
    &store_in(profile, cx).settings
}

/// [`super::features`] for a window in `profile`: a hidden profile's window reads its own mask.
pub fn features_in(profile: &ProfileId, cx: &App) -> wormhole_core::config::FeatureMask {
    store_in(profile, cx).features()
}

/// [`super::ui`] (the translucent main window's colours) for a window in `profile`.
pub fn ui_in(profile: &ProfileId, cx: &App) -> UiColors {
    store_in(profile, cx).ui
}

/// Change the settings of the window's profile `profile` as the UI of its window would: the shown profile's
/// through the global, another open profile's in its parked store.
pub fn update_in(profile: &ProfileId, cx: &mut App, f: impl FnOnce(&mut Settings)) {
    if edits_in(profile, cx) {
        super::update_settings(cx, f);
    } else if cx.has_global::<ParkedSettings>()
        && let Some(store) = cx.global_mut::<ParkedSettings>().get_mut(profile)
    {
        store.update_here(f);
    }
}

impl SettingsStore {
    /// Change this store's settings as an edit from inside Wormhole.
    fn update_here(&mut self, f: impl FnOnce(&mut Settings)) {
        self.config.update(f);
        self.ui_edits += 1;
        self.derive();
    }
}
