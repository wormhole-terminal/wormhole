//! Where the pointer is on the screens, for a drag that has left its window: GPUI reports the pointer to the
//! window the drag began in, in that window's own space, which says nothing about which screen it is over once
//! it is far outside.

use wormhole_core::view::tear_off::Spot;

use super::os::pointer as os;

/// The pointer, now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    /// In the system's global space: points from the bottom left of the primary screen, y up. What a window
    /// is moved by (`window::set_global_top_left`).
    pub global: (f32, f32),
    /// Which screen it is on and where on it, which is how GPUI places a window (the display's id is GPUI's).
    pub spot: Spot,
}

/// Where the pointer is, `None` where the system can't say or it is in a gap between screens.
pub fn on_screen() -> Option<Pointer> {
    os::on_screen()
}
