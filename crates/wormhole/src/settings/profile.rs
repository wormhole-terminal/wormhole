//! Settings across a profile switch: the global holds the shown profile's store, and a switch
//! puts another profile's store in its place.

use gpui_kit::App;

use super::SettingsStore;

impl SettingsStore {
    /// Make `next` (another profile's settings) the store in force and hand back the one it
    /// replaces, to be parked. What the system said carries over, and every revision moves on
    /// from the outgoing store's, so no view mistakes the new profile's look for the one it drew.
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
