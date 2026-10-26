//! `platform::pointer` where there is nothing to ask.

use crate::platform::pointer::Pointer;

pub fn on_screen() -> Option<Pointer> {
    None
}

pub fn screens() -> Vec<wormhole_core::view::tear_off::Screen> {
    Vec::new()
}

pub fn window_number_at(_: (f32, f32)) -> Option<i64> {
    None
}
