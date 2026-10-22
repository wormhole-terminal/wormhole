//! Where the welcome draws its mark and its logo, and what the logo is made of. The app reads the
//! places (`MARK_CELL`, `LOGO`, [`dock`]) to aim the ship of the start animation at the real cell,
//! so they are fixed: the page puts them there whatever the size of the window, from the top left.
//!
//! The logo is the app icon's `>_` on its dark tile as block art, about five rows: a grid of
//! half-block pixels (two to a row, so they are about square), drawn with the terminal's palette
//! (bright black for the tile, bright white for the prompt) so the theme colours it.

use super::ansi::{Grid, Style, color};
use crate::onboarding::intro::Handover;

/// The page's left margin, in cells, and the blank rows above everything.
pub const MARGIN: usize = 2;
pub const TOP: usize = 1;

/// The cell of the `>` of the `>_` mark (row, column), 0-based from the top left of the terminal. The
/// Cursor hand-over docks the ship here; the Portal one draws the same mark with no ship.
pub const MARK_CELL: (usize, usize) = (TOP, MARGIN);

/// How tall the ship's chevron is when it docks on the mark, in rows: a `>` is about half the height
/// of its cell, and the ship dissolves into it.
pub const MARK_CHEVRON_ROWS: f32 = 0.55;

/// A rectangle of cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}

/// Where the Logo hand-over's block-art logo is: its top left, and how many rows and columns.
pub const LOGO: Rect = Rect { row: TOP, col: MARGIN, rows: 5, cols: 12 };

/// The cell the logo's chevron is centred in (row, column): the one the ship lands on.
pub const LOGO_CHEVRON_CELL: (usize, usize) = (LOGO.row + 2, LOGO.col + 4);

/// Where the ship docks: the middle of the chevron it lands on and how tall that chevron is, in cells
/// from the top left of the terminal's grid (fractions of a cell, so the middle of a cell is `n + 0.5`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockCell {
    pub row: f32,
    pub col: f32,
    pub rows: f32,
}

// A `>` is smaller than the cell it sits in.
const _: () = assert!(MARK_CHEVRON_ROWS < 1.0);

/// The mark's `>`, in the middle of its cell.
pub const MARK_DOCK: DockCell =
    DockCell { row: MARK_CELL.0 as f32 + 0.5, col: MARK_CELL.1 as f32 + 0.5, rows: MARK_CHEVRON_ROWS };

/// The logo's chevron (a test derives it from the pixels): 4 pixels wide at the third, 5 half-rows
/// tall from the third.
pub const LOGO_DOCK: DockCell =
    DockCell { row: LOGO.row as f32 + 2.25, col: LOGO.col as f32 + 4.0, rows: 2.5 };

/// Where the ship of `handover` docks, or `None` when it has no ship.
pub fn dock(handover: Handover) -> Option<DockCell> {
    match handover {
        Handover::Cursor => Some(MARK_DOCK),
        Handover::Logo => Some(LOGO_DOCK),
        Handover::Portal => None,
    }
}

/// `.` is nothing (the tile's rounded corners), `#` the tile, `>` and `_` the prompt.
const LOGO_PIXELS: [&str; 10] = [
    ".##########.",
    "############",
    "##>>########",
    "###>>#######",
    "####>>######",
    "###>>#######",
    "##>>###___##",
    "############",
    "############",
    ".##########.",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pixel {
    Empty,
    Tile,
    Prompt,
}

fn pixel(x: usize, y: usize) -> Pixel {
    match LOGO_PIXELS.get(y).and_then(|row| row.as_bytes().get(x)) {
        Some(b'#') => Pixel::Tile,
        Some(b'>' | b'_') => Pixel::Prompt,
        _ => Pixel::Empty,
    }
}

fn paint(pixel: Pixel) -> Option<u8> {
    match pixel {
        Pixel::Empty => None,
        Pixel::Tile => Some(color::BRIGHT_BLACK),
        Pixel::Prompt => Some(color::BRIGHT_WHITE),
    }
}

/// The logo's cell at (`row`, `col`) within it: a character and its style, from the two pixels it
/// stacks.
pub fn logo_cell(row: usize, col: usize) -> (char, Style) {
    let (top, bottom) = (paint(pixel(col, 2 * row)), paint(pixel(col, 2 * row + 1)));
    match (top, bottom) {
        (None, None) => (' ', Style::PLAIN),
        (Some(a), Some(b)) if a == b => (' ', Style::PLAIN.bg(a)),
        (Some(a), None) => ('▀', Style::PLAIN.fg(a)),
        (None, Some(b)) => ('▄', Style::PLAIN.fg(b)),
        (Some(a), Some(b)) => ('▀', Style::PLAIN.fg(a).bg(b)),
    }
}

/// Draw the logo with its top left at (`row`, `col`), showing only the first `rows` of its rows (the
/// logo appears whole when the ship has docked, so this is for a clipped window).
pub fn draw_logo(grid: &mut Grid, row: usize, col: usize) {
    for r in 0..LOGO.rows {
        for c in 0..LOGO.cols {
            let (ch, style) = logo_cell(r, c);
            if ch != ' ' || style != Style::PLAIN {
                grid.put(row + r, col + c, &ch.to_string(), style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_is_five_rows_of_twelve_pixels() {
        assert_eq!(LOGO_PIXELS.len(), 2 * LOGO.rows);
        for row in LOGO_PIXELS {
            assert_eq!(row.len(), LOGO.cols, "{row:?}");
        }
    }

    /// The ship is aimed at the chevron the pixels draw: derive its middle and height from them, so
    /// changing the art without changing `LOGO_DOCK` fails here.
    #[test]
    fn the_dock_is_the_chevron_of_the_pixels() {
        let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0, usize::MAX, 0);
        for (y, row) in LOGO_PIXELS.iter().enumerate() {
            for (x, c) in row.bytes().enumerate() {
                if c == b'>' {
                    (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
                }
            }
        }
        // Pixels are one cell wide and half a row tall; the box includes the last pixel.
        let (width, height) = ((x1 + 1 - x0) as f32, (y1 + 1 - y0) as f32 / 2.0);
        assert_eq!((width, height), (4.0, 2.5));
        let middle = ((x0 as f32 + width / 2.0), (y0 as f32 / 2.0 + height / 2.0));
        assert_eq!(LOGO_DOCK.col, LOGO.col as f32 + middle.0);
        assert_eq!(LOGO_DOCK.row, LOGO.row as f32 + middle.1);
        assert_eq!(LOGO_DOCK.rows, height);
    }

    #[test]
    fn the_docks_are_inside_what_they_dock_on() {
        let (row, col) = MARK_CELL;
        assert_eq!((MARK_DOCK.row, MARK_DOCK.col), (row as f32 + 0.5, col as f32 + 0.5));
        let inside = |dock: DockCell, rect: Rect| {
            (rect.row as f32..(rect.row + rect.rows) as f32).contains(&dock.row)
                && (rect.col as f32..(rect.col + rect.cols) as f32).contains(&dock.col)
        };
        assert!(inside(LOGO_DOCK, LOGO));
        let (row, col) = LOGO_CHEVRON_CELL;
        assert_eq!((LOGO_DOCK.row as usize, LOGO_DOCK.col as usize), (row, col), "the cell the dock is in");
        assert_eq!(dock(Handover::Cursor), Some(MARK_DOCK));
        assert_eq!(dock(Handover::Logo), Some(LOGO_DOCK));
        assert_eq!(dock(Handover::Portal), None, "no ship, no dock");
    }

    #[test]
    fn two_pixels_make_a_cell_with_half_blocks() {
        let tile = color::BRIGHT_BLACK;
        let prompt = color::BRIGHT_WHITE;
        // The tile's rounded corner: only the lower pixel of the first cell is the tile.
        assert_eq!(logo_cell(0, 0), ('▄', Style::PLAIN.fg(tile)));
        assert_eq!(logo_cell(4, 0), ('▀', Style::PLAIN.fg(tile)));
        // Tile over tile is a filled blank; the prompt over the tile is an upper half on its fill.
        assert_eq!(logo_cell(0, 5), (' ', Style::PLAIN.bg(tile)));
        assert_eq!(logo_cell(1, 2), ('▀', Style::PLAIN.fg(prompt).bg(tile)));
        // Outside the logo is nothing.
        assert_eq!(logo_cell(9, 40), (' ', Style::PLAIN));
    }

    #[test]
    fn the_logo_draws_five_rows_and_only_palette_colours() {
        let mut grid = Grid::new(30, 8);
        draw_logo(&mut grid, LOGO.row, LOGO.col);
        let text = grid.text();
        assert!(text[0].is_empty(), "the blank row above");
        for (row, line) in text.iter().enumerate().skip(LOGO.row).take(LOGO.rows) {
            assert!(!line.is_empty(), "row {row} is drawn");
        }
        assert!(text[LOGO.row + LOGO.rows].is_empty(), "and nothing below");
        assert!(grid.style_at(LOGO.row + 1, LOGO.col + 2).fg.is_some());
    }
}
