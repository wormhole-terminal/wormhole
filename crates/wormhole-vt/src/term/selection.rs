//! Selecting text: the cells a mouse drag, a double click or a triple click covers.
//!
//! A selection is two anchors, each a [`PinId`](crate::grid::PinId) of the grid and the half of
//! its cell the pointer was on. Pins follow the text through output that scrolls the screen into
//! the history and through a reflow, so the selection stays on what was selected; what it covers
//! is worked out from the anchors each time it is asked (drawn, copied), never stored, so it also
//! follows text that is rewritten under it.
//!
//! # What each kind covers
//!
//! - **Simple**: from the first anchor's edge to the second's in reading order. A pointer on a
//!   cell's left half puts the edge before the cell, on its right half after it, so a drag from
//!   the middle of one cell to the middle of another includes both and a click (one edge) selects
//!   nothing. An edge at the end of a row is the start of the next.
//! - **Block**: the rectangle between the anchors' edges, the same columns on every row.
//! - **Semantic** (a word): from the start of the word around the earlier anchor's cell to the end
//!   of the word around the later one's. A word is the run of cells that no *separator* character
//!   (see [`set_word_separators`](Terminal::set_word_separators)) interrupts; the cell clicked is
//!   always in it, so a click on a separator takes it together with the word before it. Words
//!   continue over a soft wrap. A click on a bracket (`()`, `[]`, `{}`, `<>`) selects up to its
//!   partner instead.
//! - **Lines**: whole program lines (every soft-wrapped row of them) from the earlier anchor's
//!   to the later's.
//!
//! A wide character is never cut in the text: one whose second cell starts the range, or whose first
//! ends it, is copied whole. The range itself is the cells the pointer chose.
//!
//! # When a selection ends
//!
//! A pointer click ([`drop_empty_selection`](Terminal::drop_empty_selection)) leaves nothing
//! selected; switching to or from the alternate screen drops the selection (its anchors are rows of
//! the other screen); so does losing an anchor's row, which happens when the history above it is
//! dropped for good or cleared.

use crate::grid::{PinId, TextShape};
use crate::types::{Column, Line, Point, SelectionRange, SelectionType, Side};

use super::Terminal;

/// The characters that end a word until the host says otherwise: the ones that separate words in
/// shell output and paths (and a space and a tab).
pub const DEFAULT_WORD_SEPARATORS: &str = ",│`|:\"' ()[]{}<>\t";

/// One end of a selection.
#[derive(Clone, Copy, Debug)]
struct Anchor {
    pin: PinId,
    side: Side,
}

#[derive(Clone, Copy, Debug)]
struct Selection {
    kind: SelectionType,
    start: Anchor,
    end: Anchor,
}

/// The terminal's selection state: the selection, if any, and which characters end a word.
#[derive(Debug)]
pub(super) struct Select {
    current: Option<Selection>,
    separators: String,
}

impl Default for Select {
    fn default() -> Select {
        Select { current: None, separators: DEFAULT_WORD_SEPARATORS.to_string() }
    }
}

/// The cells a selection covers now, in reading order (for a block, `start` is the top left and
/// `end` the bottom right).
struct Covered {
    start: Point,
    end: Point,
    kind: SelectionType,
}

impl Terminal {
    /// Select from `point`, replacing any selection. `side` is the half of the cell the pointer
    /// is on. Extend it with [`update_selection`](Self::update_selection).
    pub fn start_selection(&mut self, kind: SelectionType, point: Point, side: Side) {
        self.clear_selection();
        let (start, end) = (self.grid.pin(point), self.grid.pin(point));
        self.select.current =
            Some(Selection { kind, start: Anchor { pin: start, side }, end: Anchor { pin: end, side } });
    }

    /// Move the end being dragged to `point` (nothing without a selection).
    pub fn update_selection(&mut self, point: Point, side: Side) {
        let Some(selection) = self.select.current.as_mut() else { return };
        self.grid.move_pin(selection.end.pin, point);
        selection.end.side = side;
    }

    /// Drop the selection; whether there was one.
    pub fn take_selection(&mut self) -> bool {
        let had = self.select.current.is_some();
        self.clear_selection();
        had
    }

    /// Drop the selection when it covers nothing: a click without a drag.
    pub fn drop_empty_selection(&mut self) {
        if self.select.current.is_some() && self.covered().is_none() {
            self.clear_selection();
        }
    }

    /// The cells selected, in grid coordinates (line 0 is the live screen's top row, the history
    /// is negative; the view's offset is for the caller to add). `None` without a selection or
    /// one that covers nothing.
    pub fn selection_range(&self) -> Option<SelectionRange> {
        let covered = self.covered()?;
        Some(SelectionRange {
            start: covered.start,
            end: covered.end,
            is_block: covered.kind == SelectionType::Block,
        })
    }

    /// The selected text as copying it gives it: soft-wrapped rows joined, a newline at every other
    /// line end inside the selection, the blanks at the end of lines cut. A selection of lines
    /// ends with a newline, any other does not.
    pub fn selection_text(&self) -> Option<String> {
        let covered = self.covered()?;
        Some(match covered.kind {
            SelectionType::Block => self.grid.text_between(covered.start, covered.end, TextShape::Block),
            SelectionType::Lines => {
                let mut text = self.grid.text_between(covered.start, covered.end, TextShape::Stream);
                text.push('\n');
                text
            }
            SelectionType::Simple | SelectionType::Semantic => {
                self.grid.text_between(covered.start, covered.end, TextShape::Stream)
            }
        })
    }

    /// The characters that end a word for a double click (and for a drag started by one).
    pub fn word_separators(&self) -> &str {
        &self.select.separators
    }

    /// Choose which characters end a word; [`DEFAULT_WORD_SEPARATORS`] to begin with. An empty
    /// cell always counts as a space.
    pub fn set_word_separators(&mut self, separators: &str) {
        self.select.separators.clear();
        self.select.separators.push_str(separators);
    }

    /// Forget the selection and free its pins.
    pub(super) fn clear_selection(&mut self) {
        if let Some(selection) = self.select.current.take() {
            self.grid.unpin(selection.start.pin);
            self.grid.unpin(selection.end.pin);
        }
    }

    /// Drop the selection if the text it was on is gone from the history; cheap when there is
    /// none. Called after output, which is what scrolls rows away.
    pub(super) fn prune_selection(&mut self) {
        let Some(selection) = self.select.current else { return };
        let alive = |anchor: Anchor| self.grid.pin_point(anchor.pin).is_some();
        if !(alive(selection.start) && alive(selection.end)) {
            self.clear_selection();
        }
    }

    /// What the selection covers now.
    fn covered(&self) -> Option<Covered> {
        let selection = self.select.current?;
        let (a, b) = (self.grid.pin_point(selection.start.pin)?, self.grid.pin_point(selection.end.pin)?);
        let cols = self.grid.cols();
        let last = Column(cols - 1);
        let (start, end) = match selection.kind {
            SelectionType::Simple => {
                // Edges numbered along the reading order: the edge before a cell is its number, the
                // one after it the next cell's.
                let edge = |p: Point, side: Side| {
                    (p.line.0 as i64) * cols as i64 + p.column.0 as i64 + i64::from(side == Side::Right)
                };
                let (x, y) = (edge(a, selection.start.side), edge(b, selection.end.side));
                let (lo, hi) = (x.min(y), x.max(y));
                if lo == hi {
                    return None;
                }
                let cell = |n: i64| {
                    Point::new(
                        Line(n.div_euclid(cols as i64) as i32),
                        Column(n.rem_euclid(cols as i64) as usize),
                    )
                };
                (cell(lo), cell(hi - 1))
            }
            SelectionType::Block => {
                let edge = |p: Point, side: Side| p.column.0 + usize::from(side == Side::Right);
                let (x, y) = (edge(a, selection.start.side), edge(b, selection.end.side));
                let (lo, hi) = (x.min(y), x.max(y));
                if lo == hi {
                    return None;
                }
                let (top, bottom) = (a.line.min(b.line), a.line.max(b.line));
                return Some(Covered {
                    start: Point::new(top, Column(lo.min(cols - 1))),
                    end: Point::new(bottom, Column((hi - 1).min(cols - 1))),
                    kind: SelectionType::Block,
                });
            }
            SelectionType::Semantic => {
                let (first, second) = if a <= b { (a, b) } else { (b, a) };
                // A click on a bracket takes everything up to its partner.
                if a == b
                    && let Some(partner) = self.grid.bracket_match(a)
                {
                    let (open, close) = if a <= partner { (a, partner) } else { (partner, a) };
                    return Some(Covered { start: open, end: close, kind: selection.kind });
                }
                let separator = |c: char| self.select.separators.contains(c);
                (self.grid.word_start(first, separator), self.grid.word_end(second, separator))
            }
            SelectionType::Lines => {
                let (first, _) = self.grid.line_extent(a.line.min(b.line));
                let (_, final_line) = self.grid.line_extent(a.line.max(b.line));
                (Point::new(first, Column(0)), Point::new(final_line, last))
            }
        };
        let (top, bottom) = (self.grid.topmost(), self.grid.bottommost());
        if end.line < top || start.line > bottom {
            return None;
        }
        Some(Covered { start, end, kind: selection.kind })
    }
}
