//! A window being dragged to another size: its resize steps, and hearing when a drag begins and ends. The
//! rules (what the program in the terminal is told meanwhile, the strip a window off the cell grid leaves)
//! are core's (`view::motion::live_resize`). Tests never ask the real system: nothing is watched, and the
//! tests say whether a window is being dragged.

use gpui_kit::Window;
use wormhole_core::view::motion::live_resize::Event;

use super::os::live_resize as os;

/// Make the window resize in steps of `steps` (width and height, in points) when it is dragged, or freely
/// with `None`. Full screen, zoom and tiling ignore them, and so does a size set by a program or a script.
/// AppKit keeps steps in whole points (core's `live_resize::steps` takes that into account), and does not say
/// which size they count from, so a window that is off the grid may stay off it: the strip is taken off when the
/// drag is over (`window::Native::settle`).
pub fn set_increments(window: &Window, steps: Option<(f32, f32)>) {
    os::set_increments(window, steps);
}

/// Whether the person is dragging this window's edge or corner right now, as the notifications said.
pub fn in_progress(window: &Window) -> bool {
    #[cfg(test)]
    if let Some(live) = test_live::get() {
        return live;
    }
    os::in_progress(window)
}

/// Call `changed` each time a drag of a window's edge begins or ends, at once and in whatever the system is
/// doing: it only passes the event on to a later turn. Once, for the app's life.
pub fn watch(changed: impl Fn(Event) + 'static) {
    if !cfg!(test) {
        os::watch(changed);
    }
}

/// What the tests say about a live resize: no test window has an `NSWindow` to ask.
#[cfg(test)]
pub(crate) mod test_live {
    use std::cell::Cell;

    thread_local! {
        static LIVE: Cell<Option<bool>> = const { Cell::new(None) };
    }

    pub(crate) fn get() -> Option<bool> {
        LIVE.with(Cell::get)
    }

    /// Windows are in a live resize (or not) until `None` puts the real answer back.
    pub(crate) fn set(live: Option<bool>) {
        LIVE.with(|cell| cell.set(live));
    }
}
