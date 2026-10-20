//! Picture placements: where an image sits in the grid, kept by row so that it moves with the
//! text.
//!
//! A placement is a rectangle of cells whose top row is named by its **serial number** (the same
//! number pins use), so it stays on its text while the screen scrolls into the history, while the
//! history above it is dropped and, through [`Grid::resize`], while a reflow moves the row. The
//! cells under a placement are ordinary cells: text printed there is kept and drawn over or under
//! the picture (its `z`), which is what the protocols ask for. The table is the grid's, not a
//! page's: a placement spans rows, and so pages, and is rare enough that a short list is the
//! right shape (there is one comparison against an empty list in the paths below).
//!
//! # Rules
//!
//! - **Scrolling** the whole screen moves nothing (rows keep their serials); a part of the screen
//!   moves the placements whose rows all lie in the part, and deletes the ones that cross its
//!   edge or are pushed out of it (`shift.rs`).
//! - **Erasing** cells (ED, EL, ECH, DCH, ICH, the rows of IL and DL) deletes the placements that
//!   cover any erased cell. Selective erase (DECSED, DECSEL) spares them, as it spares protected
//!   text. Printing over a picture changes nothing.
//! - **Reflow** carries a placement's top row with its text; the placement keeps its size in cells
//!   and is cut at the screen's right edge when it no longer fits (`resize.rs`).
//! - **History**: a placement lives while any of its rows does.
//! - At most [`MAX_PLACEMENTS`] are kept; the oldest go first.

use std::ops::Range;

use crate::types::{Extent, ImageHandle, Line, SourceRect};

use super::Grid;

mod shift;
mod view;

pub(crate) use view::Hit;

/// Most placements a grid keeps. A program that places thousands loses the oldest, which are
/// long out of sight.
pub const MAX_PLACEMENTS: usize = 4096;

/// A placement to add; see [`Grid::place_image`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NewPlacement {
    /// The picture.
    pub image: ImageHandle,
    /// The id the program gave the picture, 0 for none.
    pub image_id: u32,
    /// The id the program gave the placement, 0 for none. A placement of the same picture with
    /// the same nonzero id is replaced.
    pub id: u32,
    /// Cells across (at least 1).
    pub cols: usize,
    /// Cells down (at least 1).
    pub rows: usize,
    /// The part of the picture shown.
    pub source: SourceRect,
    /// Pixels from the first cell's left edge to the picture.
    pub offset_x: u32,
    /// Pixels from the first cell's top edge to the picture.
    pub offset_y: u32,
    /// How it is sized.
    pub extent: Extent,
    /// Stacking order.
    pub z: i32,
}

/// A placement as the grid keeps it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Placement {
    pub(super) spec: NewPlacement,
    /// Serial number of the top row. Signed: a placement scrolled off the top of a screen with no
    /// history has its upper rows above the first row that exists.
    pub(super) top: i64,
    pub(super) col: usize,
}

impl Placement {
    /// Serial number after the last row.
    fn end(&self) -> i64 {
        self.top.saturating_add(self.spec.rows as i64)
    }
}

/// The table, oldest first (which is also the order of drawing within one `z`).
#[derive(Clone, Debug, Default)]
pub(super) struct Placements {
    list: Vec<Placement>,
}

impl Placements {
    #[inline]
    pub(super) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The anchors of the placements: index, top row, column. For a reflow to carry.
    pub(super) fn anchors(&self) -> impl Iterator<Item = (usize, i64, usize)> + '_ {
        self.list.iter().enumerate().map(|(i, p)| (i, p.top, p.col))
    }

    /// A reflow to `cols` columns gives rows new numbers: a placement whose top row is already
    /// gone can't be carried along, and one that starts right of the new edge would only be
    /// squeezed against it. Both go.
    pub(super) fn drop_headless(&mut self, first: i64, cols: usize) {
        self.list.retain(|p| p.top >= first && p.col < cols);
    }

    pub(super) fn set_anchor(&mut self, index: usize, top: i64, col: usize) {
        if let Some(p) = self.list.get_mut(index) {
            p.top = top;
            p.col = col;
        }
    }
}

impl Grid {
    /// Whether any picture is placed (a frame without one skips the rest of the API).
    #[inline]
    pub fn has_placements(&self) -> bool {
        !self.placements.is_empty()
    }

    /// How many placements there are, history included.
    pub fn placement_count(&self) -> usize {
        self.placements.list.len()
    }

    /// Place a picture with its top-left cell at `column` of `line` (0 is the live screen's top
    /// row; a negative line is history, for a picture that scrolled the screen to make room).
    /// False, and nothing placed, when the cell is outside the grid.
    pub(crate) fn place_image(&mut self, line: Line, column: usize, new: NewPlacement) -> bool {
        let top = self.top_serial() as i64 + i64::from(line.0);
        if column >= self.cols
            || top >= self.end as i64
            || top.saturating_add(new.rows as i64) <= self.first_serial() as i64
        {
            return false;
        }
        let list = &mut self.placements.list;
        if new.id != 0 {
            list.retain(|p| !(p.spec.image == new.image && p.spec.id == new.id));
        }
        if list.len() >= MAX_PLACEMENTS {
            list.remove(0);
        }
        list.push(Placement {
            spec: NewPlacement { cols: new.cols.max(1), rows: new.rows.max(1), ..new },
            top,
            col: column,
        });
        self.all_dirty = true;
        true
    }

    /// Every placement, history included, one line each with its top row counted from the live
    /// screen's (negative in the history): the state dump compares these as text.
    #[cfg(any(test, feature = "internals"))]
    pub fn describe_placements(&self) -> Vec<String> {
        let top = self.top_serial() as i64;
        self.placements
            .list
            .iter()
            .map(|p| format!("line {} col {} {:?}", p.top - top, p.col, p.spec))
            .collect()
    }

    /// Whether a placement of `image` is anywhere in the grid.
    pub fn is_placed(&self, image: ImageHandle) -> bool {
        self.placements.list.iter().any(|p| p.spec.image == image)
    }

    /// Delete the placements `pick` chooses, appending the picture of each to `removed` (once
    /// per placement). Returns how many went.
    pub(crate) fn delete_placements(
        &mut self,
        mut pick: impl FnMut(&Hit) -> bool,
        removed: &mut Vec<ImageHandle>,
    ) -> usize {
        let (top, rows, cols) = (self.top_serial() as i64, self.rows, self.cols);
        let before = self.placements.list.len();
        self.placements.list.retain(|p| {
            let gone = pick(&view::hit(p, top, rows, cols));
            if gone {
                removed.push(p.spec.image);
            }
            !gone
        });
        let gone = before - self.placements.list.len();
        if gone > 0 {
            self.all_dirty = true;
        }
        gone
    }

    /// Delete every placement of `image`.
    pub(crate) fn delete_image_placements(&mut self, image: ImageHandle) {
        let before = self.placements.list.len();
        self.placements.list.retain(|p| p.spec.image != image);
        self.all_dirty |= self.placements.list.len() != before;
    }

    /// Delete every placement.
    pub(crate) fn clear_placements(&mut self) {
        if !self.placements.is_empty() {
            self.placements.list.clear();
            self.all_dirty = true;
        }
    }

    /// The live screen's rows down to the lowest one a picture covers: what a clear that keeps
    /// the scrollback has to move up, so the pictures go with the text.
    pub(super) fn placements_used_rows(&self) -> usize {
        let top = self.top_serial() as i64;
        self.placements
            .list
            .iter()
            .map(|p| (p.end() - top).clamp(0, self.rows as i64) as usize)
            .max()
            .unwrap_or(0)
    }

    /// Cells `xs` of live rows `ys` were erased: the pictures covering any of them go.
    pub(super) fn placements_erased(&mut self, ys: Range<usize>, xs: Range<usize>) {
        if self.placements.is_empty() || ys.is_empty() || xs.is_empty() {
            return;
        }
        let top = self.top_serial() as i64;
        let (first, last) = (top + ys.start as i64, top + ys.end as i64);
        let before = self.placements.list.len();
        self.placements.list.retain(|p| {
            !(p.top < last && p.end() > first && p.col < xs.end && p.col + p.spec.cols > xs.start)
        });
        self.all_dirty |= self.placements.list.len() != before;
    }

    /// Drop the placements whose rows have all left the history.
    pub(super) fn prune_placements(&mut self) {
        if !self.placements.is_empty() {
            let first = self.first_serial() as i64;
            self.placements.list.retain(|p| p.end() > first);
        }
    }

    /// The rows from `end` on were cut off the grid: placements below go, the others are cut.
    pub(super) fn cut_placements_at(&mut self, end: u64) {
        let end = end as i64;
        self.placements.list.retain_mut(|p| {
            if p.top >= end {
                return false;
            }
            if p.end() > end {
                let rows = (end - p.top).max(1) as usize;
                if p.spec.extent == Extent::Fill {
                    p.spec.source.height =
                        (u64::from(p.spec.source.height) * rows as u64 / p.spec.rows as u64).max(1) as u32;
                }
                p.spec.rows = rows;
            }
            true
        });
    }

    /// After a reflow carried the anchors: placements whose left edge no longer fits are gone.
    pub(super) fn drop_placements_off_screen(&mut self) {
        let cols = self.cols;
        self.placements.list.retain(|p| p.col < cols);
    }
}

impl Grid {
    /// The placements' share of [`Grid::check_invariants`].
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn check_placements(&self) -> Result<(), String> {
        let (first, end) = (self.first_serial() as i64, self.end as i64);
        for (i, p) in self.placements.list.iter().enumerate() {
            if p.spec.cols == 0 || p.spec.rows == 0 {
                return Err(format!("placement {i} covers no cell"));
            }
            if p.col >= self.cols {
                return Err(format!("placement {i} starts at column {} of {}", p.col, self.cols));
            }
            if p.end() <= first || p.top >= end {
                return Err(format!(
                    "placement {i} has no row left ({}..{} of {first}..{end})",
                    p.top,
                    p.end()
                ));
            }
        }
        Ok(())
    }
}
