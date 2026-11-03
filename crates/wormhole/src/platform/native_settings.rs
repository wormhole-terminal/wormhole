//! The Settings window drawn by the system's own UI toolkit (SwiftUI, from `macos/settings/`): the window, its
//! controls and its previews are native; what the settings are is not. The app keeps the settings and sends the
//! window a snapshot of them as JSON; the window answers each change of the person's as one JSON event. It reads
//! and writes no file.
//!
//! One window at a time. Off macOS, or in a test, there is no such window and the app draws its own.

use super::os::native_settings as os;

/// Whether this build can show the native window.
pub fn available() -> bool {
    !cfg!(test) && os::available()
}

/// Show the window on `page` (a page id of the window's catalog), opening it if it is not open. `behind`: do not
/// bring the app forward or take the keys (a test instance in background mode). `on_event` gets each event as JSON
/// text, on the main thread, outside any call of the app's: it must not do the app's work itself, only queue it.
pub fn open(page: &str, behind: bool, on_event: fn(&str)) {
    if available() {
        os::open(page, behind, on_event);
    }
}

/// Replace what the window shows with this snapshot (JSON). Nothing happens while the window is closed.
pub fn update(snapshot: &str) {
    if available() {
        os::update(snapshot);
    }
}

pub fn is_open() -> bool {
    available() && os::is_open()
}

/// The window's content as a PNG at `path` (devctl's `native-shot`); whether it was written.
#[cfg(feature = "devctl")]
pub fn shot(path: &str) -> bool {
    available() && os::shot(path)
}
