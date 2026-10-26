//! Where a point of the global space is: which screen, and where on it.

use crate::view::panes::Rect;

/// One screen as AppKit reports it. `id` is the Core Graphics display id, which is also GPUI's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub id: u64,
    /// The whole screen in the global space (y up).
    pub frame: Rect,
}

/// A point on one screen, in that screen's own space (from its top left, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub display: u64,
    pub x: f32,
    pub y: f32,
}

/// The screen the global point (`x`, `y`, y up) is on, and where on it. A screen holds its left and its
/// top edge and not its right and bottom ones, as the display space it is counted in does: a pointer at the
/// very top of a screen is on its first row of points (the system reports 0 there, which this space has as
/// the screen's height), and the line between two screens is on one of them only. `None` in a gap between
/// screens, where a pointer can't be.
#[must_use]
pub fn locate(x: f32, y: f32, screens: &[Screen]) -> Option<Spot> {
    screens.iter().find_map(|screen| {
        let f = screen.frame;
        let inside = x >= f.x && x < f.right() && y > f.y && y <= f.bottom();
        // The screen's top is its largest y: down from there is the display's y.
        inside.then(|| Spot { display: screen.id, x: x - f.x, y: f.bottom() - y })
    })
}

/// A rectangle of display `display`'s own space (from its top left, y down: a window's bounds) in the global
/// space. `None` for a display that is not among `screens`.
#[must_use]
pub fn to_global(display: u64, local: Rect, screens: &[Screen]) -> Option<Rect> {
    let screen = screens.iter().find(|screen| screen.id == display)?;
    let f = screen.frame;
    Some(Rect::new(f.x + local.x, f.bottom() - local.y - local.h, local.w, local.h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(id: u64, x: f32, y: f32, w: f32, h: f32) -> Screen {
        Screen { id, frame: Rect::new(x, y, w, h) }
    }

    #[test]
    fn the_primary_screen_counts_down_from_its_top() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0)];
        assert_eq!(locate(100.0, 800.0, &screens), Some(Spot { display: 1, x: 100.0, y: 100.0 }));
        // The system's top row of points (y 0 from the top) is a height up in the global space.
        assert_eq!(locate(100.0, 900.0, &screens), Some(Spot { display: 1, x: 100.0, y: 0.0 }));
        assert_eq!(locate(0.0, 0.0, &screens), None, "the line under the bottom row is not on it");
        assert_eq!(locate(0.0, 0.5, &screens), Some(Spot { display: 1, x: 0.0, y: 899.5 }));
    }

    #[test]
    fn a_screen_left_of_the_primary_has_negative_global_x() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(2, -1920.0, -100.0, 1920.0, 1080.0)];
        // 1820 from the left edge of the screen on the left, 100 down from its top (global y 980).
        assert_eq!(locate(-100.0, 880.0, &screens), Some(Spot { display: 2, x: 1820.0, y: 100.0 }));
        assert_eq!(locate(-1920.0, 979.0, &screens).map(|s| (s.display, s.x)), Some((2, 0.0)));
        assert_eq!(locate(-1920.0, 980.0, &screens).map(|s| (s.display, s.y)), Some((2, 0.0)), "its top row");
    }

    #[test]
    fn a_screen_above_the_primary_has_a_global_y_beyond_its_height() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(3, 0.0, 900.0, 1440.0, 900.0)];
        assert_eq!(locate(50.0, 1700.0, &screens), Some(Spot { display: 3, x: 50.0, y: 100.0 }));
        // The line between them belongs to the lower one: it is its top row, the upper one's bottom edge is out.
        assert_eq!(locate(50.0, 900.0, &screens).map(|s| s.display), Some(1));
        assert_eq!(locate(50.0, 900.5, &screens).map(|s| s.display), Some(3));
    }

    #[test]
    fn a_windows_bounds_go_to_the_global_space_of_the_screen_it_is_on() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(2, -1920.0, -100.0, 1920.0, 1080.0)];
        let on_primary = to_global(1, Rect::new(100.0, 50.0, 800.0, 600.0), &screens);
        assert_eq!(on_primary, Some(Rect::new(100.0, 250.0, 800.0, 600.0)));
        // 10 points down and 20 in on the left screen, whose top is at global y 980.
        let on_left = to_global(2, Rect::new(20.0, 10.0, 500.0, 300.0), &screens);
        assert_eq!(on_left, Some(Rect::new(-1900.0, 670.0, 500.0, 300.0)));
        assert_eq!(to_global(9, Rect::new(0.0, 0.0, 1.0, 1.0), &screens), None);
        // Round trip: a point inside it is on the screen it came from.
        let inside = on_left.map(|r| (r.x + 5.0, r.bottom() - 5.0));
        let spot = inside.and_then(|(x, y)| locate(x, y, &screens));
        assert_eq!(spot, Some(Spot { display: 2, x: 25.0, y: 15.0 }));
    }

    #[test]
    fn a_gap_is_no_screen() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(2, 1600.0, 0.0, 1440.0, 900.0)];
        assert_eq!(locate(1500.0, 100.0, &screens), None);
        assert_eq!(locate(-1.0, 100.0, &screens), None);
        assert_eq!(locate(1440.0, 100.0, &screens), None, "a right edge is outside");
    }
}
