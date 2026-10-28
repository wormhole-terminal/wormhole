//! Where the picker's card goes: against the terminal's cursor, above its line, and inside the pane. Pure,
//! so the tight places (a cursor on the first line, a small pane) are tested.

use crate::view::panes::Rect;

/// The room kept between the card and the cursor's cell, and between the card and the pane's edges.
pub const GAP: f32 = 4.0;
pub const MARGIN: f32 = 8.0;

/// What the card is made of: its width, what it has besides rows (the search field and the card's own
/// padding), a row's height and the most rows it shows before its list scrolls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Card {
    pub width: f32,
    pub chrome: f32,
    pub row: f32,
    pub max_rows: usize,
}

impl Card {
    /// The card's height with `rows` rows (the "No folders" line is one).
    pub fn height(&self, rows: usize) -> f32 {
        self.chrome + self.row * rows as f32
    }
}

/// The card as placed, in the window's space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub rect: Rect,
    /// How many rows the card has room for: at most what was wanted, fewer in a pane too short for all.
    pub rows: usize,
    /// The card is above the cursor's line; below it when there was no room above.
    pub above: bool,
}

/// Place a card that wants `wanted` rows against the cursor's cell, `cursor`, in the pane `pane`.
///
/// The side is decided by whether the card at its largest fits, not by how many rows there are now, so
/// typing, which makes the list shorter, never moves the card from one side of the line to the other. Above,
/// the card's bottom edge stays at the line and the list grows upward; below, its top edge stays. A pane with
/// room for neither side takes the roomier one and shows the rows that fit. The card's left edge is the
/// cursor's, moved in until the card is inside the pane; a pane narrower than the card narrows it.
pub fn place(pane: Rect, cursor: Rect, card: Card, wanted: usize) -> Placed {
    let top = pane.y + MARGIN;
    let bottom = pane.bottom() - MARGIN;
    let room_above = (cursor.y - GAP - top).max(0.0);
    let room_below = (bottom - (cursor.bottom() + GAP)).max(0.0);
    let full = card.height(card.max_rows);
    let above = room_above >= full || (room_below < full && room_above >= room_below);
    let room = if above { room_above } else { room_below };
    let fits = (((room - card.chrome) / card.row).floor().max(1.0)) as usize;
    let rows = wanted.clamp(1, card.max_rows).min(fits);
    let height = card.height(rows);
    let y = if above { cursor.y - GAP - height } else { cursor.bottom() + GAP };
    // Inside the pane when it can be; a pane shorter than the card keeps the card's top in view.
    let y = y.min(bottom - height).max(top);
    let width = card.width.min((pane.w - 2.0 * MARGIN).max(0.0));
    let x = cursor.x.min(pane.right() - MARGIN - width).max(pane.x + MARGIN);
    Placed { rect: Rect::new(x, y, width, height), rows, above }
}

#[cfg(test)]
mod tests {
    use super::{Card, GAP, MARGIN, place};
    use crate::view::panes::Rect;

    const CARD: Card = Card { width: 300.0, chrome: 52.0, row: 28.0, max_rows: 8 };
    const CELL: (f32, f32) = (8.0, 18.0);

    fn cursor_at(x: f32, y: f32) -> Rect {
        Rect::new(x, y, CELL.0, CELL.1)
    }

    #[test]
    fn it_sits_above_the_cursors_line_with_its_bottom_at_the_line() {
        let pane = Rect::new(0.0, 0.0, 800.0, 600.0);
        let at = place(pane, cursor_at(100.0, 400.0), CARD, 5);
        assert!(at.above);
        assert_eq!(at.rows, 5);
        assert_eq!(at.rect.bottom(), 400.0 - GAP);
        assert_eq!(at.rect.x, 100.0);
        assert_eq!(at.rect.w, 300.0);
    }

    #[test]
    fn fewer_rows_keep_the_bottom_where_it_was_and_the_top_comes_down() {
        let pane = Rect::new(0.0, 0.0, 800.0, 600.0);
        let many = place(pane, cursor_at(100.0, 400.0), CARD, 8);
        let few = place(pane, cursor_at(100.0, 400.0), CARD, 2);
        assert_eq!(many.rect.bottom(), few.rect.bottom());
        assert!(few.rect.y > many.rect.y);
    }

    #[test]
    fn more_rows_than_it_shows_are_cut_to_the_most() {
        let pane = Rect::new(0.0, 0.0, 800.0, 600.0);
        assert_eq!(place(pane, cursor_at(10.0, 500.0), CARD, 500).rows, 8);
        assert_eq!(place(pane, cursor_at(10.0, 500.0), CARD, 0).rows, 1, "the empty line is a row");
    }

    #[test]
    fn a_cursor_on_the_first_lines_puts_it_below() {
        let pane = Rect::new(0.0, 38.0, 800.0, 600.0);
        let at = place(pane, cursor_at(100.0, 60.0), CARD, 8);
        assert!(!at.above);
        assert_eq!(at.rect.y, 60.0 + CELL.1 + GAP);
        assert_eq!(at.rows, 8);
    }

    #[test]
    fn the_side_does_not_depend_on_how_many_rows_there_are() {
        // Room above for three rows but not eight: it goes below however few rows there are, so the card
        // doesn't jump across the line as the list is narrowed.
        let pane = Rect::new(0.0, 0.0, 800.0, 600.0);
        let cursor = cursor_at(100.0, MARGIN + GAP + 52.0 + 3.0 * 28.0 + 1.0);
        for wanted in [1, 3, 8] {
            assert!(!place(pane, cursor, CARD, wanted).above, "{wanted}");
        }
    }

    #[test]
    fn a_pane_with_room_for_neither_side_takes_the_roomier_and_shows_what_fits() {
        let pane = Rect::new(0.0, 0.0, 800.0, 300.0);
        let at = place(pane, cursor_at(100.0, 200.0), CARD, 8);
        assert!(at.above, "186 above, 70 below");
        assert!(at.rows < 8 && at.rows >= 1);
        assert!(at.rect.y >= MARGIN && at.rect.bottom() <= 200.0 - GAP + 0.001);
        let low = place(pane, cursor_at(100.0, 20.0), CARD, 8);
        assert!(!low.above, "the cursor is at the top: 8 above, 258 below");
        assert!(low.rect.bottom() <= 300.0 - MARGIN + 0.001);
    }

    #[test]
    fn it_stays_inside_the_panes_sides() {
        let pane = Rect::new(200.0, 0.0, 500.0, 600.0);
        let right = place(pane, cursor_at(680.0, 400.0), CARD, 3);
        assert_eq!(right.rect.right(), pane.right() - MARGIN);
        let left = place(pane, cursor_at(150.0, 400.0), CARD, 3);
        assert_eq!(left.rect.x, pane.x + MARGIN);
    }

    #[test]
    fn a_pane_narrower_than_the_card_narrows_it() {
        let pane = Rect::new(0.0, 0.0, 200.0, 600.0);
        let at = place(pane, cursor_at(150.0, 400.0), CARD, 3);
        assert_eq!((at.rect.x, at.rect.w), (MARGIN, 200.0 - 2.0 * MARGIN));
    }

    #[test]
    fn a_pane_shorter_than_the_card_keeps_its_top_in_view() {
        let pane = Rect::new(0.0, 0.0, 800.0, 90.0);
        let at = place(pane, cursor_at(10.0, 40.0), CARD, 8);
        assert_eq!(at.rows, 1);
        assert!(at.rect.y >= MARGIN);
    }
}
