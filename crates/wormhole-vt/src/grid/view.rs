//! The view: the part of the history on show, scrolled back from the live screen.

use crate::types::Scroll;

use super::Grid;

impl Grid {
    /// How many lines the view is scrolled up into the history (0 for the live screen).
    pub fn display_offset(&self) -> usize {
        self.offset
    }

    /// Move the view; it stays inside the history. Everything shown changes, so the next snapshot
    /// copies all of it.
    pub fn scroll_view(&mut self, scroll: Scroll) {
        let history = self.history_len();
        let before = self.offset;
        self.offset = match scroll {
            Scroll::Delta(lines) => (self.offset as i64 + lines as i64).clamp(0, history as i64) as usize,
            Scroll::Top => history,
            Scroll::Bottom => 0,
        };
        self.all_dirty |= self.offset != before;
    }

    /// Mark everything as changed: the next snapshot copies every row.
    pub fn damage_all(&mut self) {
        self.all_dirty = true;
    }
}
