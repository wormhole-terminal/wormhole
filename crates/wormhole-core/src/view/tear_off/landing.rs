//! What a terminal let go of outside its window means: which window of the app is under the pointer,
//! and so whether it opens a window of its own, joins another one or stays. AppKit sends the mouse
//! events of a drag to the window it began in only, so the other windows never hear it: this is
//! worked out from where the windows are, front to back.

use crate::view::panes::Rect;

/// What a window of the app is, for a release over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occupant<K> {
    /// A window that shows terminals, which can take one.
    Terminals(K),
    /// Any other window of the app (Settings, the Theme Studio, the chip that follows the pointer): it
    /// hides what lies behind it and takes nothing.
    Other,
}

/// A window of the app and where it is: its display and its bounds in that display's space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed<K> {
    pub occupant: Occupant<K>,
    pub display: u64,
    pub bounds: Rect,
}

/// What letting go of a terminal does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landing<K> {
    /// Nothing: it was let go over its own window (or one that takes nothing).
    Stay,
    /// Over no window of the app: the terminal opens a window of its own there.
    NewWindow,
    /// Over another window that shows terminals: the terminal goes into it.
    Into(K),
}

/// The window at (`x`, `y`) of `display`, the front-most of `stack` (front to back) that holds it.
#[must_use]
pub fn top_at<K: Copy>(stack: &[Placed<K>], display: u64, x: f32, y: f32) -> Option<Placed<K>> {
    stack.iter().copied().find(|window| {
        let b = window.bounds;
        window.display == display && x >= b.x && x < b.right() && y >= b.y && y < b.bottom()
    })
}

/// What a release at (`x`, `y`) of `display` does for a terminal dragged out of window `source`.
#[must_use]
pub fn landing<K: Copy + PartialEq>(
    stack: &[Placed<K>],
    display: u64,
    x: f32,
    y: f32,
    source: K,
) -> Landing<K> {
    match top_at(stack, display, x, y).map(|window| window.occupant) {
        None => Landing::NewWindow,
        Some(Occupant::Terminals(key)) if key == source => Landing::Stay,
        Some(Occupant::Terminals(key)) => Landing::Into(key),
        Some(Occupant::Other) => Landing::Stay,
    }
}

/// Whether a pointer at (`x`, `y`) of a window's own space, as the window that has the drag reports it
/// (even outside its content), has left a window whose content is `width` by `height`.
#[must_use]
pub fn outside(x: f32, y: f32, width: f32, height: f32) -> bool {
    x < 0.0 || y < 0.0 || x >= width || y >= height
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(key: u8, display: u64, x: f32, y: f32, w: f32, h: f32) -> Placed<u8> {
        Placed { occupant: Occupant::Terminals(key), display, bounds: Rect::new(x, y, w, h) }
    }

    fn other(display: u64, x: f32, y: f32, w: f32, h: f32) -> Placed<u8> {
        Placed { occupant: Occupant::Other, display, bounds: Rect::new(x, y, w, h) }
    }

    #[test]
    fn the_front_most_window_under_the_point_takes_it() {
        // Window 2 lies over window 1 where they overlap.
        let stack = [at(2, 1, 500.0, 100.0, 600.0, 500.0), at(1, 1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(landing(&stack, 1, 600.0, 200.0, 1), Landing::Into(2));
        assert_eq!(landing(&stack, 1, 100.0, 200.0, 3), Landing::Into(1));
        // The same point with 2 behind 1 goes the other way.
        let behind = [stack[1], stack[0]];
        assert_eq!(landing(&behind, 1, 600.0, 200.0, 3), Landing::Into(1));
    }

    #[test]
    fn letting_go_over_its_own_window_or_one_that_takes_nothing_stays() {
        let stack = [other(1, 700.0, 100.0, 300.0, 300.0), at(1, 1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(landing(&stack, 1, 100.0, 100.0, 1), Landing::Stay);
        assert_eq!(landing(&stack, 1, 750.0, 150.0, 1), Landing::Stay, "Settings lies over it");
        // Outside the settings window, under which a window lay, the window behind takes it.
        assert_eq!(landing(&stack, 1, 650.0, 150.0, 7), Landing::Into(1));
    }

    #[test]
    fn the_desktop_opens_a_window() {
        let stack = [at(1, 1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(landing(&stack, 1, 900.0, 100.0, 1), Landing::NewWindow);
        assert_eq!(landing(&stack, 1, 800.0, 100.0, 1), Landing::NewWindow, "a right edge is outside");
        assert_eq!(landing(&[], 1, 10.0, 10.0, 1), Landing::NewWindow);
    }

    #[test]
    fn windows_are_looked_for_on_the_display_of_the_point() {
        // The same display-space point on another display is another place.
        let stack = [at(2, 2, 0.0, 0.0, 800.0, 600.0), at(1, 1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(landing(&stack, 1, 100.0, 100.0, 9), Landing::Into(1));
        assert_eq!(landing(&stack, 2, 100.0, 100.0, 9), Landing::Into(2));
        assert_eq!(landing(&stack, 3, 100.0, 100.0, 9), Landing::NewWindow);
    }

    #[test]
    fn the_pointer_is_outside_a_window_past_any_edge() {
        assert!(!outside(0.0, 0.0, 800.0, 600.0));
        assert!(!outside(799.9, 599.9, 800.0, 600.0));
        assert!(outside(800.0, 10.0, 800.0, 600.0));
        assert!(outside(-0.5, 10.0, 800.0, 600.0));
        assert!(outside(10.0, 600.0, 800.0, 600.0));
        assert!(outside(10.0, -3.0, 800.0, 600.0));
    }
}
