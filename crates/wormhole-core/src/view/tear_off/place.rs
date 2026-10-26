//! Where the window of a torn-off terminal opens: under the pointer, on the screen it was let go on,
//! kept inside the part of it a window can use, and where in that window the chip it grows from is.

use crate::view::panes::Rect;

/// A new window's place, and the chip it grows from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// The window, in the space of the display it opens on.
    pub bounds: Rect,
    /// The chip that was dragged, in the window's own space: where its content starts to grow.
    pub chip: Rect,
}

/// The window of `size` that a terminal let go of at (`x`, `y`) opens as, on a screen whose usable part
/// (under the menu bar, clear of the Dock) is `visible`. The window is placed so that the pointer lies at
/// `anchor` in it (the new terminal's row, which was under the pointer a moment ago), then moved to lie
/// inside `visible`, and made smaller than `size` where the screen is. `chip` is the dragged chip's
/// rectangle in the display's space; it comes back in the window's.
#[must_use]
pub fn place(
    (x, y): (f32, f32),
    chip: Rect,
    size: (f32, f32),
    anchor: (f32, f32),
    visible: Rect,
) -> Placement {
    let (w, h) = (size.0.min(visible.w), size.1.min(visible.h));
    let clamp =
        |wanted: f32, low: f32, extent: f32, space: f32| wanted.clamp(low, low + (space - extent).max(0.0));
    let left = clamp(x - anchor.0, visible.x, w, visible.w);
    let top = clamp(y - anchor.1, visible.y, h, visible.h);
    Placement {
        bounds: Rect::new(left, top, w, h),
        chip: Rect::new(chip.x - left, chip.y - top, chip.w, chip.h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect { x: 0.0, y: 38.0, w: 1440.0, h: 800.0 };
    const CHIP: Rect = Rect { x: 508.0, y: 392.0, w: 160.0, h: 28.0 };

    #[test]
    fn the_pointer_lies_at_the_anchor() {
        let tall = Rect::new(0.0, 38.0, 1440.0, 1000.0);
        let placed = place((500.0, 400.0), CHIP, (900.0, 600.0), (120.0, 90.0), tall);
        assert_eq!(placed.bounds, Rect::new(380.0, 310.0, 900.0, 600.0));
        assert_eq!(placed.chip, Rect::new(128.0, 82.0, 160.0, 28.0), "the chip stays under the pointer");
    }

    #[test]
    fn a_window_is_kept_inside_the_usable_part() {
        // Let go at the bottom right: the window comes back up and left, the chip with it.
        let placed = place(
            (1400.0, 830.0),
            Rect::new(1408.0, 822.0, 160.0, 28.0),
            (900.0, 600.0),
            (120.0, 90.0),
            SCREEN,
        );
        assert_eq!(placed.bounds, Rect::new(540.0, 238.0, 900.0, 600.0));
        assert_eq!(placed.chip.x, 868.0);
        // Let go above the menu bar's edge: it stays under it.
        let high = place((600.0, 5.0), CHIP, (900.0, 600.0), (120.0, 90.0), SCREEN);
        assert_eq!(high.bounds.y, 38.0);
    }

    #[test]
    fn a_window_is_never_bigger_than_the_screen_it_opens_on() {
        let small = Rect::new(0.0, 0.0, 800.0, 500.0);
        let placed = place((400.0, 250.0), CHIP, (1320.0, 820.0), (120.0, 90.0), small);
        assert_eq!(placed.bounds, small);
    }
}
