//! Restore terminal text after restart, turned off: what the profile's store holds of its terminals'
//! text is erased, at once and at every launch while it stays off (the setting may have been changed
//! in config.json while the app was closed). Saving stops by itself (`Workspace::save_scrollback`).

use super::app::app;
use super::*;

/// At launch, and from then on whenever the settings change (a switch of profile too).
pub(super) fn init(cx: &mut App) {
    erase_if_off(cx);
    cx.observe_global::<SettingsStore>(erase_if_off).detach();
}

fn erase_if_off(cx: &mut App) {
    if settings(cx).restore_terminal_text {
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
