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

/// The screen the global point (`x`, `y`, y up) is on, and where on it. A screen holds its left and
/// bottom edges and not its right and top ones, so a point on the line between two screens is on one
/// of them only. `None` in a gap between screens, where a pointer can't be.
#[must_use]
pub fn locate(x: f32, y: f32, screens: &[Screen]) -> Option<Spot> {
    screens.iter().find_map(|screen| {
        let f = screen.frame;
        let inside = x >= f.x && x < f.right() && y >= f.y && y < f.bottom();
        // The screen's top is its largest y: down from there is the display's y.
        inside.then(|| Spot { display: screen.id, x: x - f.x, y: f.bottom() - y })
    })
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
        assert_eq!(locate(0.0, 0.0, &screens), Some(Spot { display: 1, x: 0.0, y: 900.0 }));
    }

    #[test]
    fn a_screen_left_of_the_primary_has_negative_global_x() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(2, -1920.0, -100.0, 1920.0, 1080.0)];
        // 1820 from the left edge of the screen on the left, 100 down from its top (global y 980).
        assert_eq!(locate(-100.0, 880.0, &screens), Some(Spot { display: 2, x: 1820.0, y: 100.0 }));
        assert_eq!(locate(-1920.0, 979.0, &screens).map(|s| (s.display, s.x)), Some((2, 0.0)));
    }

    #[test]
    fn a_screen_above_the_primary_has_a_global_y_beyond_its_height() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(3, 0.0, 900.0, 1440.0, 900.0)];
        assert_eq!(locate(50.0, 1700.0, &screens), Some(Spot { display: 3, x: 50.0, y: 100.0 }));
        // The line between them belongs to the upper one (its bottom edge) only.
        assert_eq!(locate(50.0, 900.0, &screens).map(|s| s.display), Some(3));
        assert_eq!(locate(50.0, 899.5, &screens).map(|s| s.display), Some(1));
    }

    #[test]
    fn a_gap_is_no_screen() {
        let screens = [screen(1, 0.0, 0.0, 1440.0, 900.0), screen(2, 1600.0, 0.0, 1440.0, 900.0)];
        assert_eq!(locate(1500.0, 100.0, &screens), None);
        assert_eq!(locate(-1.0, 100.0, &screens), None);
        assert_eq!(locate(1440.0, 100.0, &screens), None, "a right edge is outside");
    }
}
