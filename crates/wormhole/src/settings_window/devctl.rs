//! devctl's hold on the Settings window: open a page, search, change a row or press a button the way the window
//! does, and have the window draw itself to a PNG. devctl calls these from a window's tick.

use std::path::Path;

use gpui_kit::App;
use wormhole_core::config::settings_index::SectionId as Section;

use super::native::{self, Show};
use super::open_settings;

/// devctl `settings` / `settings-section <page>`: the window, on that page (empty: where it was).
pub(crate) fn section(name: &str, cx: &mut App) {
    let section = Section::ALL.into_iter().find(|s| format!("{s:?}").eq_ignore_ascii_case(name));
    open_settings(section, cx);
}

/// devctl `settings-search <text>`: type into the window's search (empty clears it).
pub(crate) fn search(text: &str, cx: &mut App) {
    native::show(Show { search: Some(text.to_string()), ..Show::default() }, cx);
}

/// devctl `settings-reveal <id>`: the window at that setting's row, as the palette opens it.
pub(crate) fn reveal(id: &str, cx: &mut App) {
    super::reveal_row(id, cx);
}

/// devctl `profile-sheet`: the "New profile" sheet, as the File menu opens it.
pub(crate) fn profile_sheet(cx: &mut App) {
    super::open_new_profile_sheet(cx);
}

/// devctl `native-set <row key> <value>`: a change in the window, through the path its events take.
pub(crate) fn native_set(arg: &str, cx: &mut App) {
    // The row key has no spaces; a value may.
    let Some((key, value)) = arg.trim().split_once(' ') else { return };
    let event = serde_json::json!({ "t": "set", "k": key, "v": value }).to_string();
    native::event(&event, cx);
}

/// devctl `native-event <json>`: an event of the window as the window would send it.
pub(crate) fn native_event(arg: &str, cx: &mut App) {
    native::event(arg.trim(), cx);
}

/// devctl `settings-shot` / `native-shot`: the window's content as a PNG at `path`; whether it was written.
pub(crate) fn shot(path: &Path) -> bool {
    crate::platform::native_settings::shot(&path.to_string_lossy())
}

/// The window gets its words again, after the interface language changed.
pub(crate) fn words_changed(cx: &mut App) {
    native::push(cx);
}
