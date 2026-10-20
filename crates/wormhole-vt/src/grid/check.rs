//! The invariants of a grid, checked.
//!
//! [`Grid::check_invariants`] is for tests and fuzzing: it walks every page and reports the first
//! thing that is not as the module docs promise. It is exact and slow (it visits every cell), so
//! nothing calls it outside a test.

use std::collections::HashMap;

use super::Grid;
use super::cell::CellFlags;
use super::page::Page;
use super::row::DIRTY;
use super::style::LAYOUT_FLAGS;

impl Grid {
    /// Check everything a grid promises; the first violation found, described, as the error.
    ///
    /// - Pages are in order with consecutive serial numbers, the live screen is within them, every
    ///   page has the grid's width.
    /// - A page's rows use distinct slots, and dead slots hold blank cells.
    /// - A wide character is a `WIDE` cell followed by a `SPACER_TAIL` of its style; a tail follows
    ///   a wide cell; a spacer head is the last cell of a row.
    /// - Each style's count equals the cells that use it, and the table's index finds every entry.
    /// - A cell has a grapheme or link entry exactly when its flag says, and no entry is orphaned;
    ///   each link's count equals its cells.
    /// - Every pin is inside the grid or dropped, the view's offset is within the history.
    /// - Every placement has at least one cell, starts inside the width, and still has a row.
    pub fn check_invariants(&self) -> Result<(), String> {
        if self.pages.is_empty() {
            return Err("no pages".into());
        }
        let mut serial = self.first_serial();
        for (ix, page) in self.pages.iter().enumerate() {
            if page.base != serial {
                return Err(format!(
                    "page {ix} starts at serial {} but {serial} follows the last",
                    page.base
                ));
            }
            if page.cols() != self.cols {
                return Err(format!("page {ix} is {} columns wide, the grid {}", page.cols(), self.cols));
            }
            if page.rows.is_empty() && self.pages.len() > 1 {
                return Err(format!("page {ix} has no rows"));
            }
            check_page(page).map_err(|e| format!("page {ix}: {e}"))?;
            serial += page.rows.len() as u64;
        }
        if serial != self.end {
            return Err(format!("the pages end at {serial}, the grid says {}", self.end));
        }
        if (self.end - self.first_serial()) < self.rows as u64 {
            return Err("fewer rows than the screen".into());
        }
        if self.offset > self.history_len() {
            return Err(format!("view offset {} beyond the history of {}", self.offset, self.history_len()));
        }
        self.check_placements()?;
        for (_, serial, col) in self.pins.live() {
            // A pin whose row is gone (before the history's start, or cut off the end) keeps
            // whatever column it had; only a pin that points into the grid has to be inside it.
            let dropped = serial < self.first_serial() || serial == super::pins::DROPPED;
            if serial >= self.end && serial != super::pins::DROPPED {
                return Err(format!("a pin at serial {serial} beyond the end {}", self.end));
            }
            if !dropped && col >= self.cols {
                return Err(format!("a pin at column {col}"));
            }
        }
        Ok(())
    }
}
fn check_page(page: &Page) -> Result<(), String> {
    let cols = page.cols();
    if page.all_cells().len() != page.slots_used() * cols {
        return Err("cell buffer isn't the slots in use".into());
    }
    let mut used = vec![false; page.slots_used()];
    for (ix, row) in page.rows.iter().enumerate() {
        let slot = row.slot as usize;
        if slot >= used.len() {
            return Err(format!("row {ix} names slot {slot} of {}", used.len()));
        }
        if std::mem::replace(&mut used[slot], true) {
            return Err(format!("slot {slot} is two rows"));
        }
        if row.flags & !(DIRTY | super::row::WRAPPED) != 0 {
            return Err(format!("row {ix} has unknown flags {:#x}", row.flags));
        }
    }
    for (slot, in_use) in used.iter().enumerate() {
        if !in_use && page.all_cells()[slot * cols..(slot + 1) * cols].iter().any(|c| !c.is_blank()) {
            return Err(format!("dead slot {slot} holds cells"));
        }
    }

    page.styles.verify()?;
    page.links.verify()?;
    let mut style_refs: HashMap<u16, u32> = HashMap::new();
    let mut link_refs: HashMap<u32, u32> = HashMap::new();
    let (mut marked, mut linked) = (0, 0);
    for ix in 0..page.rows.len() {
        let cells = page.cells(ix);
        for (x, cell) in cells.iter().enumerate() {
            let at = || format!("row {ix} column {x}");
            if cell.codepoint() != 0 && char::from_u32(cell.codepoint()).is_none() {
                return Err(format!("{}: code point {:#x}", at(), cell.codepoint()));
            }
            let id = cell.style_id();
            if id != 0 {
                if id as usize >= page.styles.styles().len() || page.styles.refs(id) == 0 {
                    return Err(format!("{}: style {id} isn't in use", at()));
                }
                *style_refs.entry(id).or_default() += 1;
            }
            if cell.flag_set().contains(CellFlags::IMAGE) {
                return Err(format!("{}: an image flag nothing sets", at()));
            }
            if cell.is_wide() {
                let tail = cells.get(x + 1);
                if !tail.is_some_and(|t| t.is_spacer_tail() && t.style_id() == id && t.codepoint() == 0) {
                    return Err(format!("{}: a wide character without its tail", at()));
                }
                if cell.is_spacer_tail() || cell.is_spacer_head() {
                    return Err(format!("{}: a wide cell that is a spacer", at()));
                }
            }
            if cell.is_spacer_tail() {
                if x == 0 || !cells[x - 1].is_wide() {
                    return Err(format!("{}: a tail that follows no wide character", at()));
                }
                if cell.codepoint() != 0 {
                    return Err(format!("{}: a tail with a character", at()));
                }
            }
            if cell.is_spacer_head() && (x + 1 != cols || cell.codepoint() != 0) {
                return Err(format!("{}: a spacer head that isn't the last blank cell", at()));
            }
            let offset = page.offset(ix, x);
            if cell.has_grapheme() != !page.graphemes.get(offset).is_empty() {
                return Err(format!(
                    "{}: grapheme flag {} but marks {:?}",
                    at(),
                    cell.has_grapheme(),
                    page.graphemes.get(offset)
                ));
            }
            if cell.has_grapheme() {
                marked += 1;
            }
            match (cell.has_hyperlink(), page.links.id_at(offset)) {
                (true, Some(id)) => {
                    *link_refs.entry(id).or_default() += 1;
                    linked += 1;
                }
                (false, None) => {}
                (flag, entry) => return Err(format!("{}: hyperlink flag {flag} but entry {entry:?}", at())),
            }
        }
    }
    if marked != page.graphemes.len() {
        return Err(format!("{} cells with marks, {} entries", marked, page.graphemes.len()));
    }
    if linked != page.links.len() {
        return Err(format!("{} cells under links, {} entries", linked, page.links.len()));
    }
    // Dead slots hold no cells with flags, so entries can only belong to live rows; the counts
    // below catch the rest.
    if style_refs.len() != page.styles.live() {
        return Err(format!(
            "{} styles used by cells, {} in the table",
            style_refs.len(),
            page.styles.live()
        ));
    }
    for (id, refs) in page.styles.used() {
        if style_refs.get(&id).copied().unwrap_or(0) != refs {
            return Err(format!(
                "style {id} counts {refs}, cells use it {}",
                style_refs.get(&id).copied().unwrap_or(0)
            ));
        }
        let style = page.styles.get(id);
        if style.is_default() || style.attrs.intersects(LAYOUT_FLAGS) {
            return Err(format!("style {id} is {style:?}"));
        }
    }
    for (id, refs) in page.links.used() {
        if link_refs.get(&id).copied().unwrap_or(0) != refs || !page.links.is_indexed(id) {
            return Err(format!(
                "link {id} counts {refs}, cells use it {}",
                link_refs.get(&id).copied().unwrap_or(0)
            ));
        }
    }
    Ok(())
}
