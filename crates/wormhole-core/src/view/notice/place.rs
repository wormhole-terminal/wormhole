//! Where the chip goes: centred on the terminal the notice is about, dropping from that terminal's top edge.
//! Pure, so the small sizes are tested: the chip is as wide as its words, may reach over a neighbouring pane
//! (it floats, as a Mac HUD does) but stays inside the window's edges, and only a window narrower than the chip
//! makes it cut its words.

use crate::view::motion::notice::REST_GAP;
use crate::view::panes::Rect;

/// How far the chip keeps from the window's edges (points).
pub const WINDOW_MARGIN: f32 = 8.0;

/// What the chip is placed for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// The window's content, in points.
    pub window: (f32, f32),
    /// The terminal the notice is about: its pane's area, under its header, in the window's space.
    pub anchor: Rect,
    /// The window is in full screen.
    pub full_screen: bool,
    /// The window's title row's lower edge: where a terminal that fills the window starts.
    pub title_row: f32,
}

/// Where the chip is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// Where the chip comes out from: the top edge of the terminal (the screen's edge in full screen, for a
    /// terminal under the title row). Nothing of the chip shows above it.
    pub top: f32,
    /// The chip at rest, in the window's space.
    pub chip: Rect,
    /// The words are cut: the window is narrower than the chip.
    pub cut: bool,
}

/// The chip, `natural` wide and tall as its words are, for `spot`.
#[must_use]
pub fn place(spot: Spot, natural: (f32, f32)) -> Placed {
    let Spot { window: (window_w, window_h), anchor, full_screen, title_row } = spot;
    let room = (window_w - 2.0 * WINDOW_MARGIN).max(0.0);
    let (natural_w, h) = natural;
    let w = natural_w.min(room);
    // Centred on the terminal, then moved inward until it is inside the window.
    let wanted = anchor.x + anchor.w / 2.0 - w / 2.0;
    let x = wanted.min(window_w - WINDOW_MARGIN - w).max(WINDOW_MARGIN.min(window_w - w));
    // In full screen a terminal that starts under the title row is at the screen's edge.
    let top = if full_screen && anchor.y <= title_row { 0.0 } else { anchor.y }.max(0.0);
    // A window too short for the chip below the top moves it up, but it still comes out from the top.
    let y = (top + REST_GAP).min(window_h - WINDOW_MARGIN - h).max(top.min(window_h - h).max(0.0));
    Placed { top, chip: Rect::new(x, y, w, h), cut: w < natural_w }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TITLE: f32 = 38.0;
    const CHIP: (f32, f32) = (140.0, 34.0);

    fn spot(window: (f32, f32), anchor: Rect) -> Spot {
        Spot { window, anchor, full_screen: false, title_row: TITLE }
    }

    #[test]
    fn in_a_wide_window_it_is_centred_on_the_terminal_and_drops_from_its_top() {
        // A sidebar of 264 at the left: the terminal is the rest.
        let p = place(spot((1200.0, 800.0), Rect::new(264.0, TITLE, 936.0, 762.0)), CHIP);
        assert_eq!(p.top, TITLE);
        assert_eq!(p.chip, Rect::new(264.0 + 468.0 - 70.0, TITLE + REST_GAP, 140.0, 34.0));
        assert!(!p.cut);
    }

    #[test]
    fn a_narrow_pane_at_the_left_edge_gives_the_chip_the_window_edge_to_keep_off() {
        // A pane 60 wide at x = 0: centred it would hang out of the window.
        let p = place(spot((900.0, 600.0), Rect::new(0.0, TITLE, 60.0, 500.0)), CHIP);
        assert_eq!(p.chip.x, WINDOW_MARGIN);
        assert!(p.chip.right() > 60.0, "it floats over the pane beside");
    }

    #[test]
    fn a_narrow_pane_at_the_right_edge_does_the_same() {
        let p = place(spot((900.0, 600.0), Rect::new(850.0, TITLE, 50.0, 500.0)), CHIP);
        assert_eq!(p.chip.right(), 900.0 - WINDOW_MARGIN);
    }

    #[test]
    fn a_narrow_pane_inside_the_window_keeps_its_own_middle() {
        let p = place(spot((900.0, 600.0), Rect::new(400.0, TITLE, 60.0, 500.0)), CHIP);
        assert_eq!(p.chip.x + 70.0, 430.0);
    }

    #[test]
    fn a_bottom_pane_has_the_chip_come_out_of_its_own_top_edge() {
        let p = place(spot((900.0, 700.0), Rect::new(0.0, 380.0, 900.0, 320.0)), CHIP);
        assert_eq!(p.top, 380.0);
        assert_eq!(p.chip.y, 380.0 + REST_GAP);
        assert_eq!(p.chip.x, 380.0, "and centred on it");
    }

    #[test]
    fn a_window_narrower_than_the_chip_cuts_the_words_not_the_window() {
        let p = place(spot((120.0, 400.0), Rect::new(0.0, TITLE, 120.0, 360.0)), CHIP);
        assert!(p.cut);
        assert_eq!((p.chip.x, p.chip.w), (WINDOW_MARGIN, 120.0 - 2.0 * WINDOW_MARGIN));
        assert_eq!(p.chip.h, 34.0, "as tall as ever");
    }

    #[test]
    fn a_window_with_no_room_at_all_still_places_a_chip() {
        let p = place(spot((10.0, 400.0), Rect::new(0.0, TITLE, 10.0, 360.0)), CHIP);
        assert_eq!(p.chip.w, 0.0);
        assert!(p.chip.x >= 0.0 && p.cut);
    }

    #[test]
    fn full_screen_puts_the_top_at_the_screens_edge_for_a_terminal_under_the_title_row() {
        let mut s = spot((1440.0, 900.0), Rect::new(0.0, TITLE, 1440.0, 862.0));
        s.full_screen = true;
        let p = place(s, CHIP);
        assert_eq!((p.top, p.chip.y), (0.0, REST_GAP));
        // A pane further down still comes out from its own top.
        s.anchor = Rect::new(0.0, 450.0, 1440.0, 450.0);
        assert_eq!(place(s, CHIP).top, 450.0);
        // And outside full screen the terminal's top is used.
        s.full_screen = false;
        s.anchor = Rect::new(0.0, TITLE, 1440.0, 862.0);
        assert_eq!(place(s, CHIP).top, TITLE);
    }

    #[test]
    fn a_pane_too_short_for_the_chip_still_gets_it_over_its_top() {
        let p = place(spot((900.0, 600.0), Rect::new(0.0, 300.0, 900.0, 30.0)), CHIP);
        assert_eq!((p.top, p.chip.y), (300.0, 300.0 + REST_GAP));
        assert!(p.chip.bottom() > 330.0, "it floats over what is below");
    }

    #[test]
    fn a_window_too_short_keeps_the_chip_inside_it() {
        let p = place(spot((400.0, 60.0), Rect::new(0.0, TITLE, 400.0, 22.0)), CHIP);
        assert!(p.chip.bottom() <= 60.0 && p.chip.y >= 0.0, "{p:?}");
    }
}
