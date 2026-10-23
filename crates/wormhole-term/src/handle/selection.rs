//! Selecting text on the screen and finding the link under the pointer.

use crate::Pty;
use wormhole_vt::{Column, Line, Point, SelectionType, Side};

use crate::{Link, images, links};

impl Pty {
    /// Start a mouse selection at a grid point.
    pub fn start_selection(&self, kind: SelectionType, point: Point, side: Side) {
        self.term.lock().start_selection(kind, point, side);
    }

    /// Extend the selection being dragged.
    pub fn update_selection(&self, point: Point, side: Side) {
        self.term.lock().update_selection(point, side);
    }

    /// A click without a drag selects nothing.
    pub fn finish_selection(&self) {
        self.term.lock().drop_empty_selection();
    }

    /// Select the history and the screen.
    pub fn select_all(&self) {
        let mut term = self.term.lock();
        let start = Point::new(term.topmost_line(), Column(0));
        let end = Point::new(term.bottommost_line(), term.last_column());
        term.start_selection(SelectionType::Simple, start, Side::Left);
        term.update_selection(end, Side::Right);
    }

    /// Select whole lines `first..=last` (a block's output).
    pub fn select_lines(&self, first: Line, last: Line) {
        let mut term = self.term.lock();
        let first = first.max(term.topmost_line());
        let last = last.min(term.bottommost_line());
        if first > last {
            return;
        }
        term.start_selection(SelectionType::Lines, Point::new(first, Column(0)), Side::Left);
        let end = Point::new(last, term.last_column());
        term.update_selection(end, Side::Right);
    }

    /// The selected text, if anything is selected (an image's placeholders aren't text).
    pub fn selection_text(&self) -> Option<String> {
        let text = self.term.lock().selection_text()?;
        let text = images::strip_placeholders(&text).into_owned();
        (!text.trim().is_empty()).then_some(text)
    }

    /// The link at a grid point: a program's hyperlink (OSC 8), else a URL in the text.
    pub fn link_at(&self, point: Point) -> Option<Link> {
        links::link_at(&self.term.lock(), point)
    }
}
