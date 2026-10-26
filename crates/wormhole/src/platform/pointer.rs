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
///
/// In a test, only where [`pretend`] put it, never where the person's own pointer is; in a devctl instance,
/// where it was put if it was.
pub fn on_screen() -> Option<Pointer> {
    #[cfg(any(test, feature = "devctl"))]
    {
        let pretended = *pretended().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if cfg!(test) || pretended.is_some() {
            return pretended;
        }
    }
    os::on_screen()
}

#[cfg(any(test, feature = "devctl"))]
fn pretended() -> &'static std::sync::Mutex<Option<Pointer>> {
    static POINTER: std::sync::Mutex<Option<Pointer>> = std::sync::Mutex::new(None);
    &POINTER
}

/// Say where the pointer is (`None`: wherever it really is, or nowhere in a test), for a drag that a script
/// or a test makes without a mouse.
#[cfg(any(test, feature = "devctl"))]
pub fn pretend(at: Option<Pointer>) {
    *pretended().lock().unwrap_or_else(std::sync::PoisonError::into_inner) = at;
}
