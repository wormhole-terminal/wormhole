//! The buttons of the native window.

use gpui_kit::App;

use crate::settings::SettingsStore;

/// `key` is the row (or `"<page>:"` for a page's own buttons), `name` the button's words.
pub(super) fn perform(key: &str, name: &str, cx: &mut App) {
    if super::lists::perform(key, name, cx) {
        return;
    }
    match (key, name) {
        ("Advanced:0:0", _) => super::super::sections::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        (_, "Edit config.json") => super::super::sections::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        ("Appearance:0:2", "Theme Studio") => crate::studio::open_studio(cx), // text: not shown (the native window's own button and row names)
        ("Extensions:", _) => super::lists::extensions_footer(name, cx),
        ("Appearance:", "Show in Finder") => super::super::open_folder(&crate::icons::user_dir(), cx), // text: not shown (the native window's own button and row names)
        _ => tracing::debug!("native settings: no action for {key} / {name}"),
    }
}
