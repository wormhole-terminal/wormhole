//! `platform::panel` with GPUI's own calls: its window keeps whatever title bar it has, resizing
//! is GPUI's (the system decides which edge stays), and the pointer's display isn't known.

use gpui_kit::{App, Window, px, size};

pub fn hide_window_buttons(_: &Window) {}

pub fn set_height_keeping_top(window: &mut Window, height: f32, _cx: &mut App) {
    window.resize(size(window.viewport_size().width, px(height)));
}

pub fn pointer_display_id() -> Option<u64> {
    None
}
