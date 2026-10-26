//! What a terminal let go of outside its window means: which window of the app is under the pointer,
//! and so whether it opens a window of its own, joins another one or stays. AppKit sends the mouse
//! events of a drag to the window it began in only, so the other windows never hear it: this is
//! worked out from where the windows are, front to back, all in the global space (`screen`), so a window
//! that lies across two screens is one window. Another application's window on top of them is not in the
//! stack, so the system says whether one is (`covered`).

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

/// A window of the app and where it is, in the global space (`screen::to_global`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed<K> {
    pub occupant: Occupant<K>,
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

/// The window at the global point (`x`, `y`, y up), the front-most of `stack` (front to back) that holds it.
/// A window holds its left and its top edge, as a screen does (`screen::locate`).
#[must_use]
pub fn top_at<K: Copy>(stack: &[Placed<K>], x: f32, y: f32) -> Option<Placed<K>> {
    stack.iter().copied().find(|window| {
        let b = window.bounds;
        x >= b.x && x < b.right() && y > b.y && y <= b.bottom()
    })
}

/// What a release at the global point (`x`, `y`) does for a terminal dragged out of window `source`.
/// `covered`: a window of another application is the front-most there, which is outside Wormhole whatever
/// lies behind it, so the terminal gets a window of its own (never goes into a window it can't see).
#[must_use]
pub fn landing<K: Copy + PartialEq>(
    stack: &[Placed<K>],
    x: f32,
    y: f32,
    source: K,
    covered: bool,
) -> Landing<K> {
    if covered {
        return Landing::NewWindow;
    }
    match top_at(stack, x, y).map(|window| window.occupant) {
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

    /// A window at `x`, `y` (its top left, counted down from the top of a 1000-high screen at the origin) of
    /// the global space.
    fn at(key: u8, x: f32, top: f32, w: f32, h: f32) -> Placed<u8> {
        Placed { occupant: Occupant::Terminals(key), bounds: Rect::new(x, 1000.0 - top - h, w, h) }
    }

    fn other(x: f32, top: f32, w: f32, h: f32) -> Placed<u8> {
        Placed { occupant: Occupant::Other, ..at(0, x, top, w, h) }
    }

    /// The global point of (`x`, `top`) counted down from the top of that screen.
    fn p(x: f32, top: f32) -> (f32, f32) {
        (x, 1000.0 - top)
    }

    fn land(stack: &[Placed<u8>], at: (f32, f32), source: u8, covered: bool) -> Landing<u8> {
        landing(stack, at.0, at.1, source, covered)
    }

    #[test]
    fn the_front_most_window_under_the_point_takes_it() {
        // Window 2 lies over window 1 where they overlap.
        let stack = [at(2, 500.0, 100.0, 600.0, 500.0), at(1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(land(&stack, p(600.0, 200.0), 1, false), Landing::Into(2));
        assert_eq!(land(&stack, p(100.0, 200.0), 3, false), Landing::Into(1));
        // The same point with 2 behind 1 goes the other way.
        let behind = [stack[1], stack[0]];
        assert_eq!(land(&behind, p(600.0, 200.0), 3, false), Landing::Into(1));
    }

    #[test]
    fn letting_go_over_its_own_window_or_one_that_takes_nothing_stays() {
        let stack = [other(700.0, 100.0, 300.0, 300.0), at(1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(land(&stack, p(100.0, 100.0), 1, false), Landing::Stay);
        assert_eq!(land(&stack, p(750.0, 150.0), 1, false), Landing::Stay, "Settings lies over it");
        // Outside the settings window, under which a window lay, the window behind takes it.
        assert_eq!(land(&stack, p(650.0, 150.0), 7, false), Landing::Into(1));
    }

    #[test]
    fn the_desktop_opens_a_window() {
        let stack = [at(1, 0.0, 0.0, 800.0, 600.0)];
        assert_eq!(land(&stack, p(900.0, 100.0), 1, false), Landing::NewWindow);
        assert_eq!(land(&stack, p(800.0, 100.0), 1, false), Landing::NewWindow, "a right edge is outside");
        assert_eq!(land(&stack, p(10.0, 600.0), 1, false), Landing::NewWindow, "so is a bottom edge");
        assert_eq!(land(&[], p(10.0, 10.0), 1, false), Landing::NewWindow);
    }

    #[test]
    fn a_window_across_two_screens_is_one_window() {
        // A window from x 900 to 1500 on screens that meet at 1000 (the second one starting there).
        let stack = [at(2, 900.0, 100.0, 600.0, 400.0)];
        assert_eq!(land(&stack, p(950.0, 200.0), 1, false), Landing::Into(2), "on the first screen");
        assert_eq!(land(&stack, p(1200.0, 200.0), 1, false), Landing::Into(2), "and on the second");
    }

    #[test]
    fn another_applications_window_on_top_is_outside_wormhole() {
        let stack = [at(2, 500.0, 100.0, 600.0, 500.0), at(1, 0.0, 0.0, 800.0, 600.0)];
        // A browser lies over both: what is let go over it gets a window of its own, never one it can't see.
        assert_eq!(land(&stack, p(600.0, 200.0), 1, true), Landing::NewWindow);
        assert_eq!(land(&stack, p(100.0, 200.0), 1, true), Landing::NewWindow, "not even its own window's");
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
