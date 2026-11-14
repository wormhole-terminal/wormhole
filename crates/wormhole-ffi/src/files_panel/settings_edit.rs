//! The one way the panel's seams change the person's `config.json` (pinned folders, folder colours).

use wormhole_core::config::Settings;
use wormhole_core::storage::paths::ProfilePaths;

/// Edits the settings of one profile (a window's own, which need not be the Default one) through the settings the app
/// holds of it (`settings::live`), as the GPUI app's pins and folder colours go through its settings store
/// (`features/files/pins.rs:23,32`, `features/project/folder_color.rs:209`): one edit at a time, on the file as it is
/// now, and an open Settings window holds the change at once, so its next write does not put the old value back. The
/// file is written only when `edit` changes something.
pub(super) fn update_settings_of(paths: &ProfilePaths, edit: impl Fn(&mut Settings) -> bool) {
    crate::settings::live::update(paths, |settings| {
        edit(settings);
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wormhole_core::config::Config;
    use wormhole_core::storage::paths::{DataRoot, ProfileId};

    use super::update_settings_of;
    use crate::settings::{SettingsHandle, SettingsListener};

    struct Quiet;

    impl SettingsListener for Quiet {
        fn wake(&self) {}
    }

    /// Order: the Settings window holds the profile's settings; a folder is pinned through the panel's seam; before
    /// that window has read the file again (its follower waits for the write to settle) the person changes another
    /// setting in it. Written around the window, the pin was gone again with that write.
    #[test]
    fn a_pin_set_beside_an_open_settings_window_is_kept_by_its_next_write() {
        let dir = tempfile::tempdir().expect("data dir");
        let root = DataRoot::new(dir.path());
        let handle = SettingsHandle::open_at(&root, &ProfileId::default_profile(), Arc::new(Quiet));
        let paths = root.default_profile();
        let folder = dir.path().join("pinned");
        update_settings_of(&paths, |settings| settings.set_pinned(&folder, true));
        assert!(handle.shared.lock().effective().is_pinned(&folder), "the open window holds the pin at once");
        handle.set("diff_wrap".into(), "true".into()).expect("sets");
        let saved = Config::load_for(&paths).settings;
        assert!(saved.is_pinned(&folder), "the window's next write kept the pin");
        assert!(saved.diff_wrap, "and its own change is in the file");
    }
}
