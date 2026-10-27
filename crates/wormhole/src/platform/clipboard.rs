//! The app's clipboard, the one way anything of the app copies or pastes. A test instance must not make its
//! copies for real: in background mode
//! (`WORMHOLE_BACKGROUND`, devctl builds) a copy goes to a pasteboard of the app's own, so a test
//! instance driven from a script never writes over what the person has copied. Otherwise it is the
//! system's, through GPUI.

use gpui_kit::*;

use super::background_mode;

/// What a background instance has "copied": its own pasteboard.
#[derive(Default)]
struct Held(Option<String>);

impl Global for Held {}

/// Puts `text` on the clipboard (the app's own, in background mode).
pub fn write(text: String, cx: &mut App) {
    if background_mode::on() {
        cx.set_global(Held(Some(text)));
    } else {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

/// The text on the clipboard that [`write`] puts it on.
pub fn read(cx: &App) -> Option<String> {
    if background_mode::on() {
        cx.try_global::<Held>().and_then(|held| held.0.clone())
    } else {
        cx.read_from_clipboard().and_then(|item| item.text())
    }
}
