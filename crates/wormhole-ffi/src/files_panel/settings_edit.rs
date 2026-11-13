//! The one way the panel's seams change the person's `config.json` (pinned folders, folder colours).

use std::sync::Mutex;

use wormhole_core::config::{Config, Settings};
use wormhole_core::storage::paths::{DataRoot, ProfilePaths};

/// One edit at a time within this process, so two seams changing `config.json` together (a pin and a folder
/// colour) cannot each write a copy that lacks the other's change.
static EDITING: Mutex<()> = Mutex::new(());

/// Edits the settings through a fresh read of `config.json`, so what another part of the app saved
/// meanwhile is not overwritten with a stale copy. The file is written only when `edit` changes something.
pub(super) fn update_settings(edit: impl Fn(&mut Settings) -> bool) {
    update_settings_of(&DataRoot::current().default_profile(), edit);
}

/// The same for the `config.json` of one profile (a window's own, which need not be the Default one).
pub(super) fn update_settings_of(paths: &ProfilePaths, edit: impl Fn(&mut Settings) -> bool) {
    let _one_at_a_time = EDITING.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut config = Config::load_for(paths);
    if edit(&mut config.settings.clone()) {
        config.update(|settings| {
            edit(settings);
        });
    }
}
