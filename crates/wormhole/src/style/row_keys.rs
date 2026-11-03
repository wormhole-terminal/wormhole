//! A setting row's config keys: shown under the pointer, copied by a click on them, and offered by the
//! row's menu (Copy Key, Copy as JSON, Reset to Default).
//!
//! Hover shows a key and a pointer can click it, but neither reaches a keyboard or VoiceOver, so the
//! menu is also opened by ⌃↩ on any control of the row and by VoiceOver's "Show menu" action
//! (`Action::ShowContextMenu`), and the key is a labelled button of its own.
//!
//! The row hands over data, not behaviour: its keys, a function that reads their current values as
//! JSON (the app's own settings, which this layer does not know) and, while changed, a way to put them
//! back. That is all, so a row of another source (a Probe's, later) is drawn the same. What a row
//! costs a frame is three listeners on the row and one on the key (`frame_tests` says how much); the
//! menu and the JSON exist only once the menu is open.
//!
//! Two small facts live in one global, read once per row: whether ⌥ is held (every key of the page is
//! shown) and which row has just copied (it says Copied for a moment).

use gpui_kit::prelude::*;
use gpui_kit::*;

// What the last frame drew of the keys, for the tests: the keys shown without the pointer.
#[cfg(test)]
thread_local! {
    pub(crate) static SHOWN_KEYS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A control with a context menu of its own (a text field's cut, copy and paste): a right press on it
/// goes no further up, so the row's menu does not open beside it, and the row's menu stays everywhere
/// else on the row. `Input` handles that press on its way up and leaves it to go on, and shows its menu
/// on the release, which this does not stop. Wrap every text field that sits in a row with keys.
#[track_caller]
pub fn own_menu<E: InteractiveElement>(control: E) -> E {
    control.on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
}
