//! What a floating panel (the quick panel) needs beyond GPUI's window: no title-bar buttons,
//! growing and shrinking from its top edge, and which display the pointer is on, where it opens.

use gpui_kit::{App, Window};

use super::os::panel as os;

/// Take the title bar's buttons off `window`: GPUI gives a pop-up window a (transparent) title
/// bar with the traffic lights in it.
pub fn hide_window_buttons(window: &Window) {
    os::hide_window_buttons(window);
}

/// Make `window` `height` tall, keeping its top edge where it is (a window grows downwards from
/// the top as its rows come and go).
pub fn set_height_keeping_top(window: &mut Window, height: f32, cx: &mut App) {
    os::set_height_keeping_top(window, height, cx);
}

/// The system's id of the display the pointer is on, as GPUI's displays give it; `None` where
/// it can't be asked.
pub fn pointer_display_id() -> Option<u64> {
    os::pointer_display_id()
}
