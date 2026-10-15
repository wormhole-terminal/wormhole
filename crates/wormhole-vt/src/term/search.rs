//! Reading text and what is under a point: what the host needs to copy, restore, search and link.
//!
//! Thin over the grid (`grid/text.rs`, `search.rs`, `links.rs`): the terminal adds what only it
//! knows, the alternate screen (which has no history, so there is nothing to save or search above
//! it), and pinned spans that follow a found match through output and reflow.
//!
//! None of these allocate beyond the `String`s and buffers they hand back.

use crate::grid::{Content, LinkSpan, PinId, TextCopy, TextShape};
use crate::types::{Line, Point};

use super::Terminal;

/// A found range of cells that moves with its text: two pins of the grid. Keep it while a match
/// is shown; [`Terminal::unpin_span`] it when it is not, because the grid tracks every pin
/// through every reflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinnedSpan {
    start: PinId,
    end: PinId,
}

impl Terminal {
    /// The text of the cells from `start` to `end`; see [`Grid::text_between`](crate::grid::Grid::text_between).
    pub fn text_between(&self, start: Point, end: Point, shape: TextShape) -> String {
        self.grid.text_between(start, end, shape)
    }

    /// The visible screen's rows, trailing blanks cut, each ended by a newline.
    pub fn screen_text(&self) -> String {
        let mut out = String::new();
        for y in 0..self.rows as i32 {
            self.grid.push_row_text(Line(y), true, &mut out);
            out.push('\n');
        }
        out
    }

    /// The last `max_lines` rows of the history and the screen as the program's lines, for saving
    /// the terminal to restore it later: soft-wrapped rows joined, blanks at the ends cut, blank
    /// lines at the end dropped. `None` on the alternate screen, which has no history: what a
    /// program there draws is not what to bring back.
    pub fn restore_lines(&self, max_lines: usize) -> Option<Vec<String>> {
        if self.on_alt_screen() {
            return None;
        }
        let (top, bottom) = (self.grid.topmost().0, self.grid.bottommost().0);
        let first = (bottom - max_lines.min(i32::MAX as usize) as i32 + 1).max(top);
        let mut lines = self.grid.logical_lines(Line(first), Line(bottom));
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        Some(lines)
    }

    /// Copy about `rows` rows from `from` as text for a search that runs elsewhere; see
    /// [`Grid::copy_text`](crate::grid::Grid::copy_text). Returns where the next window starts.
    pub fn copy_text(&self, from: Line, rows: usize, out: &mut TextCopy) -> Option<Line> {
        self.grid.copy_text(from, rows, out)
    }

    /// Whether the cells from `start` to `end` still spell `text` (what a search saw there).
    pub fn span_is(&self, start: Point, end: Point, text: &str) -> bool {
        self.grid.span_is(start, end, text)
    }

    /// Pin a found range so it follows its text.
    pub fn pin_span(&mut self, start: Point, end: Point) -> PinnedSpan {
        PinnedSpan { start: self.grid.pin(start), end: self.grid.pin(end) }
    }

    /// Where the span is now, `None` once its text left the history.
    pub fn span_points(&self, span: PinnedSpan) -> Option<(Point, Point)> {
        Some((self.grid.pin_point(span.start)?, self.grid.pin_point(span.end)?))
    }

    /// Whether the span's text is still in the grid and the cells still spell `text`: a stored
    /// match that fails this is stale (the cells were overwritten, or scrolled away).
    pub fn span_holds(&self, span: PinnedSpan, text: &str) -> bool {
        self.span_points(span).is_some_and(|(start, end)| self.grid.span_is(start, end, text))
    }

    /// Forget a pinned span.
    pub fn unpin_span(&mut self, span: PinnedSpan) {
        self.grid.unpin(span.start);
        self.grid.unpin(span.end);
    }

    /// The OSC 8 hyperlink at `point` and the cells it covers (over soft wraps).
    pub fn link_span(&self, point: Point) -> Option<LinkSpan> {
        self.grid.link_span(point)
    }

    /// The text of `line` that is `content` of command block `block`; see
    /// [`Grid::content_text`](crate::grid::Grid::content_text).
    pub fn content_text(&self, line: Line, block: u32, content: Content) -> String {
        self.grid.content_text(line, block, content)
    }
}
