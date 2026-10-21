//! Placements when rows move: a part of the screen scrolls, or rows under a scroll region are
//! carried down past new history.

use crate::grid::region::Region;

use super::super::Grid;

impl Grid {
    /// Rows `from..=region.bottom` of the live screen move up (`up`) or down by `n`, and the
    /// rows that leave are lost; `n` blank rows enter at the other end. Called for the scrolls
    /// that are not appends to the history.
    ///
    /// A placement whose rows all lie inside the moving rows moves with them, even if that takes
    /// its upper rows off the top of the screen (there is no row above to show them on). One that
    /// crosses an edge of the moving rows, lands on lost rows or sits beside the margins is
    /// deleted: half a picture in a scrolled region would be a picture over other text.
    pub(in crate::grid) fn placements_shifted(&mut self, region: Region, from: usize, n: usize, up: bool) {
        if self.placements.is_empty() {
            return;
        }
        let top = self.top_serial() as i64;
        let (first, last) = (top + from as i64, top + region.bottom as i64);
        let full = region.left == 0 && region.right == self.cols - 1;
        let at_screen_top = from == 0;
        let n = n as i64;
        self.placements.list.retain_mut(|p| {
            let bottom = p.end() - 1;
            if bottom < first || p.top > last {
                return true;
            }
            if !full && (p.col + p.spec.cols <= region.left || p.col > region.right) {
                return true;
            }
            if !full || p.top < first || bottom > last {
                return false;
            }
            if up {
                // The rows that stay are the ones from `first + n`; the upper part of a placement
                // may be lost only where the screen ends, not inside it.
                if p.top >= first + n || (at_screen_top && bottom >= first + n) {
                    p.top -= n;
                    true
                } else {
                    false
                }
            } else if bottom + n <= last {
                p.top += n;
                true
            } else {
                false
            }
        });
        self.all_dirty = true;
    }

    /// The rows under a scroll region (a status line below it) are copied down by `n`, past the
    /// `n` rows appended to the history. `below` is the live row where they start. Call it before
    /// the append, while the rows have their old numbers: those below move, those above stay in
    /// the history, and one on both sides of the edge is deleted.
    pub(in crate::grid) fn placements_under_region(&mut self, below: usize, n: usize) {
        if self.placements.is_empty() {
            return;
        }
        let first_below = self.top_serial() as i64 + below as i64;
        self.placements.list.retain_mut(|p| {
            if p.end() <= first_below {
                true
            } else if p.top >= first_below {
                p.top += n as i64;
                true
            } else {
                false
            }
        });
        self.all_dirty = true;
    }
}
