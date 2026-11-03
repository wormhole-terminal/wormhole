//! The buttons of the native window.

use std::path::Path;

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
    // What a file panel of the window picked: the id carries the path.
    if let Some(path) = id.strip_prefix("app:") {
        super::open_in::chosen(Path::new(path), cx);
        return;
    }
    if let Some(path) = id.strip_prefix("import:") {
        super::themes::import(Path::new(path), cx);
        return;
    }
    match (key, id) {
        ("Advanced:0:0", _) => super::paths::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        (_, "edit_config_json") => super::paths::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        ("Appearance:0:2", "theme_studio") => crate::studio::open_studio(cx), // text: not shown (the native window's own button and row names)
        ("Extensions:", _) => super::lists::extensions_footer(id, cx), // text: not shown (a key or page id of the native window, or a log line)
        (_, "edit_shells") => super::paths::open_in_editor(&SettingsStore::config_path()), // text: not shown (the native window's own button and row names)
        ("Notifications:", "open_system_settings") => crate::notifications::open_system_settings(cx), // text: not shown (the native window's own button and row names)
        ("Notifications:", "send_a_test") => crate::notifications::send_test(cx), // text: not shown (the native window's own button and row names)
        ("Fonts:", "show_in_finder") => show_fonts_folder(cx), // text: not shown (the native window's own button and row names)
        ("Appearance:", "show_in_finder") => super::paths::open_folder(&crate::icons::user_dir(), cx), // text: not shown (the native window's own button and row names)
        _ => tracing::debug!("native settings: no action for {key} / {id}"),
    }
}

/// The Fonts page's Show in Finder.
fn show_fonts_folder(cx: &mut App) {
    super::paths::open_folder(&crate::active_profile::paths().fonts_dir(), cx);
}
