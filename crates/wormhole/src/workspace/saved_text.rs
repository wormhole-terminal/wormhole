//! Restore terminal text after restart, turned off: what the profile's store holds of its terminals'
//! text is erased when it is turned off, at launch while it stays off (it may have been changed in
//! config.json while the app was closed), and when Wormhole switches to a profile that has it off.
//! Saving stops by itself (`Workspace::save_scrollback`); a profile opened beside the shown one is
//! erased as it opens (`ProfileSlot::open`).

use gpui_kit::{App, Global};
use wormhole_core::profile::ProfileId;

use super::app::app;
use crate::settings::{SettingsStore, settings};

/// The shown profile and its setting, as last seen: the settings global changes on every settings
/// poll, and only a change of these two means anything here.
struct Seen {
    profile: ProfileId,
    on: bool,
}

impl Global for Seen {}

/// At launch, and from then on whenever the settings change (a switch of profile too).
pub(super) fn init(cx: &mut App) {
    erase_if_turned_off(cx);
    cx.observe_global::<SettingsStore>(erase_if_turned_off).detach();
}

fn erase_if_turned_off(cx: &mut App) {
    let profile = app(cx).active_profile().clone();
    let on = settings(cx).restore_terminal_text;
    let news = match cx.try_global::<Seen>() {
        None => true,
        Some(seen) => seen.profile != profile || seen.on != on,
    };
    if !news {
        return;
    }
    cx.set_global(Seen { profile, on });
    if on {
        return;
    }
    let state = app(cx);
    let Some(store) = state.store_of(state.active_profile()) else { return };
    match store.erase_saved_text() {
        Ok(0) => {}
        Ok(count) => tracing::info!("erased the saved text of {count} terminals"),
        Err(err) => tracing::warn!("erasing the saved terminal text failed: {err:#}"),
    }
}
