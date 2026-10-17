//! Settings across a profile switch: the global holds the shown profile's store, and a switch
//! puts another profile's store in its place.

use gpui_kit::App;
use wormhole_core::config::Settings;
use wormhole_core::profile::ProfileId;

use super::{SettingsStore, UiColors};
use crate::workspace::AppState;

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
    if let Some(state) = cx.try_global::<AppState>()
        && profile != state.active_profile()
        && let Some(parked) = state.parked_settings(profile)
    {
        return parked;
    }
    cx.global::<SettingsStore>()
}

/// [`super::settings`] for a window in `profile`.
pub fn settings_in<'a>(profile: &ProfileId, cx: &'a App) -> &'a Settings {
    &store_in(profile, cx).settings
}

/// [`super::ui`] (the translucent main window's colours) for a window in `profile`.
pub fn ui_in(profile: &ProfileId, cx: &App) -> UiColors {
    store_in(profile, cx).ui
}
