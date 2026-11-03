//! The buttons of the native window.

use gpui_kit::App;

use crate::settings::SettingsStore;

/// `key` is the row (or `"<page>:"` for a page's own buttons), `id` the button's id: snake_case, never its words.
pub(super) fn perform(key: &str, id: &str, cx: &mut App) {
    if super::lists::perform(key, id, cx) {
        return;
    }
    if super::system::perform(key, id, cx) {
        return;
    }
    match (key, id) {
        ("Advanced:0:0", _) => super::super::sections::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        (_, "edit_config_json") => super::super::sections::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        ("Appearance:0:2", "theme_studio") => crate::studio::open_studio(cx), // text: not shown (the native window's own button and row names)
        ("Extensions:", _) => super::lists::extensions_footer(id, cx),
        ("Appearance:", "show_in_finder") => super::super::open_folder(&crate::icons::user_dir(), cx), // text: not shown (the native window's own button and row names)
        _ => tracing::debug!("native settings: no action for {key} / {id}"),
    }
}
