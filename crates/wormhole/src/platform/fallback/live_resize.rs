//! `platform::live_resize` where there is nothing to ask: windows resize freely and no drag is heard.

use gpui_kit::Window;
use wormhole_core::view::motion::live_resize::Event;

pub fn set_increments(_: &Window, _: Option<(f32, f32)>) {}

pub fn in_progress(_: &Window) -> bool {
    false
}

pub fn watch(_: impl Fn(Event) + 'static) {}
