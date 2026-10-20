//! The screen and its history: what the terminal state writes into and a host copies from.
//!
//! No escape sequences are read here. The terminal state (`terminal`) decides what a sequence means
//! and calls the operations below; the snapshot copy turns rows into what a host draws.
//!
//! # Shape
//!
//! - A **cell** ([`Cell`]) is eight bytes: code point, style id, flags. All zero bits is a blank.
//! - Rows live in **pages** (`page/`). A page has its own style table (styles interned with use
//!   counts) and side tables for the characters after a cluster's first and for hyperlinks. Pages
//!   are a list, oldest first; the **live screen is the last
//!   `rows` rows** of the list and everything before it is the history. A page holds a few hundred
//!   rows (at most 128 KiB of cells).
//! - Every row has a **serial number**, counting up from the first row ever made, so a row keeps its
//!   number while the history above it is dropped. Pins and the view are tied to serial numbers.
//! - **Scrolling the whole screen up is appending**: the top row becomes history where it is, a new
//!   blank row is made at the bottom, and no cell moves. Scrolling a part of the screen rotates row
//!   records inside a page, copying a row only where the part crosses a page. See `scroll.rs`.
//! - **Pictures** (`images`) are placements kept by the serial number of their top row, in a table
//!   of the grid's: they scroll into the history with their text, are cut by a reflow and are
//!   deleted by the erases that cover them. The cells under one are ordinary cells.
//! - A **resize** that changes the width rewraps soft-wrapped lines (`resize.rs`; the rebuild itself is
//!   `rewrap.rs`). The history's cap and clearing are in `history.rs`, the page list in `pages.rs`, the
//!   scrolled-back view in `view.rs`.
//! - The snapshot copy (`copy.rs`) turns rows into [`crate::types::Cell`]s and [`crate::types::RowExtras`].
//! - Text out of the grid (`text.rs`: a range to copy, the lines to restore; `words.rs`: the word,
//!   bracket and line around a cell; `links.rs`: how far a hyperlink reaches; `search.rs`: a
//!   window of rows as text with a byte-to-cell map, for a search that runs off the lock).
//!
//! # Choices, for whoever calls this
//!
//! - **The history is capped in bytes** (cells and row records of the rows above the screen). Pages
//!   are dropped whole, oldest first, so the cap is a ceiling and one to two pages (at most 256 KiB)
//!   less than it is kept. The memory of a dropped page is reused for the next.
//! - **Rows that scroll into the history stay as they are**, wide characters and wrap flags
//!   included; only a resize changes them.
//! - **The view** (scrolled back) keeps showing the same text while output arrives below it.
//! - **Pins** follow text through scrolling that appends to the history and through reflow. A
//!   scroll of a *part* of the screen moves text between rows, and a pin stays on its row.
//! - **Erasing up to the last column ends a row's soft wrap**; erasing a whole row forgets its
//!   block. Shifting cells through the last column turns a spacer head into a plain blank.
//! - **Operations outside the screen do nothing**; sizes are clamped to [`MIN_COLS`]..=[`MAX_COLS`]
//!   columns and 1..=[`MAX_ROWS`] rows.
//!
//! # Lines
//!
//! Operations that edit the live screen take `y` and `x`, from 0 at its top left. What reads the
//! grid, including its history, takes a [`Line`](crate::Line): 0 is the live screen's top row and the history is
//! negative, as everywhere in the crate's API.

mod cell;
#[cfg(any(test, feature = "internals"))]
mod check;
mod copy;
mod edit;
mod history;
mod images;
mod links;
mod page;
mod pages;
mod pins;
mod read;
mod rect;
mod resize;
mod rewrap;
mod row;
mod rowview;
mod scroll;
mod search;
mod side_tables;
mod style;
mod style_table;
#[cfg(test)]
mod tests;
mod text;
mod view;
mod words;

use std::collections::VecDeque;

#[cfg(any(test, feature = "internals"))]
pub use cell::flag_names;
pub use cell::{BlockPart, Cell, Spare, StyleId};
pub use copy::ViewRow;
#[cfg(test)]
pub use images::MAX_PLACEMENTS;
pub(crate) use images::NewPlacement;
pub use links::LinkSpan;
pub use pins::PinId;
pub use row::{Pen, Region, RowMark};
pub use rowview::RowView;
pub use search::{CopiedLine, TextCopy};
#[cfg(test)]
pub use side_tables::MAX_MARKS;
pub use style::Style;
pub use text::TextShape;

use crate::types::{Color, NamedColor};
use page::Page;
use pins::Pins;

/// The narrowest and widest, shortest and tallest grid. Two columns so that a wide character
/// always fits a row; 16,384 so that one row fits a page (65,534 cells, the most a page holds).
pub const MIN_COLS: usize = 2;
pub const MAX_COLS: usize = 16_384;
pub const MAX_ROWS: usize = 16_384;

/// The default background, for the calls that erase without a pen.
const BG_DEFAULT: Color = Color::Named(NamedColor::Background);

/// The history a grid keeps by default: 10 MiB, about 15,000 lines of 80 columns.
#[cfg(any(test, feature = "internals"))]
pub const DEFAULT_SCROLLBACK_BYTES: usize = 10 << 20;

/// The screen and its history; see the module docs.
pub struct Grid {
    cols: usize,
    rows: usize,
    pages: VecDeque<Page>,
    /// Serial number of the row after the last one (the next to be made).
    end: u64,
    /// Bytes of history to keep; 0 for a screen without any (the alternate screen).
    limit: usize,
    /// How many lines the view is scrolled up into the history.
    offset: usize,
    next_uid: u64,
    /// A dropped page kept for the next one, so that a full history reuses its memory.
    spare: Option<Page>,
    pins: Pins,
    /// Where pictures sit; see `images`.
    placements: images::Placements,
    /// Everything on the screen changed (a resize, a clear, the view moved) and the snapshot must
    /// copy all of it, whatever the rows' own marks say.
    all_dirty: bool,
    /// Rows per page, when a test wants pages small enough for the live rows to span several;
    /// 0 for the size `page_capacity` picks.
    page_rows: usize,
    /// Scratch for scrolling a region across pages, kept so that doing so allocates nothing.
    segments: Vec<(usize, usize, usize)>,
    /// Scratch for copying a rectangle (DECCRA): one row's cells and their marks, kept so that
    /// copying allocates nothing once they have grown to a row.
    copied: Vec<page::Copied>,
    copied_marks: Vec<char>,
}

impl Grid {
    /// A blank grid of `cols` by `rows` keeping up to `scrollback_bytes` of history (0 for none).
    pub fn new(cols: usize, rows: usize, scrollback_bytes: usize) -> Grid {
        Grid::build(cols, rows, scrollback_bytes, 0)
    }

    /// A grid whose pages hold `page_rows` rows, so that tests and benchmarks can put page
    /// boundaries inside the live screen.
    #[doc(hidden)]
    #[cfg(any(test, feature = "internals"))]
    pub fn with_page_rows(cols: usize, rows: usize, scrollback_bytes: usize, page_rows: usize) -> Grid {
        Grid::build(cols, rows, scrollback_bytes, page_rows.max(1))
    }

    fn build(cols: usize, rows: usize, limit: usize, page_rows: usize) -> Grid {
        let (cols, rows) = (cols.clamp(MIN_COLS, MAX_COLS), rows.clamp(1, MAX_ROWS));
        let mut grid = Grid::blank(cols, rows, limit, page_rows);
        for _ in 0..rows {
            grid.push_row();
        }
        grid
    }

    /// A grid of the given size (already clamped) with no rows made yet: the caller pushes the
    /// screen's rows, or sets `end` and `next_uid` to carry on from another grid.
    fn blank(cols: usize, rows: usize, limit: usize, page_rows: usize) -> Grid {
        Grid {
            cols,
            rows,
            pages: VecDeque::new(),
            end: 0,
            limit,
            offset: 0,
            next_uid: 1,
            spare: None,
            pins: Pins::default(),
            placements: images::Placements::default(),
            all_dirty: true,
            page_rows,
            segments: Vec::new(),
            copied: Vec::new(),
            copied_marks: Vec::new(),
        }
    }

    // ---- geometry ----

    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Rows of the live screen.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Lines of history above the live screen.
    pub fn history_len(&self) -> usize {
        (self.end - self.first_serial()) as usize - self.rows
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("cols", &self.cols)
            .field("rows", &self.rows)
            .field("history", &self.history_len())
            .field("pages", &self.pages.len())
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}
