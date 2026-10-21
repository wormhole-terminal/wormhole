//! How much history the main screen keeps: the line count a host asked for, as the grid's byte cap.
//!
//! The grid caps its history in bytes and drops whole pages, so the host's "at least this many
//! lines" is turned into bytes at the current width, and again when the width changes.

use crate::terminal::Terminal;

/// Cells are 8 bytes and a row's record 16; a history of N lines is capped at about N rows of
/// the current width, plus this slack: the grid drops whole pages, so it keeps up to three pages
/// (128 KiB each) fewer lines than the cap allows, and the slack buys them back.
const HISTORY_SLACK: usize = 3 * 128 * 1024;

pub(in crate::terminal) fn history_bytes(lines: usize, cols: usize) -> usize {
    lines.saturating_mul(cols * 8 + 16).saturating_add(HISTORY_SLACK)
}

impl Terminal {
    /// The history the terminal was asked to keep (at least this many lines; see
    /// [`Builder::scrollback`](crate::Builder::scrollback)).
    pub fn scrollback_lines(&self) -> usize {
        self.scrollback_lines
    }

    /// Change how much history is kept; lines over it are dropped now.
    pub fn set_scrollback_lines(&mut self, lines: usize) {
        self.scrollback_lines = lines;
        self.apply_history_limit();
    }

    /// Give the main screen the history the configuration asks for at the current width. The
    /// alternate screen never has any, so the grid that is not showing is the main one while it shows.
    pub(in crate::terminal) fn apply_history_limit(&mut self) {
        let main = if self.on_alt_screen() { &mut self.other } else { &mut self.grid };
        main.set_scrollback_limit(history_bytes(self.scrollback_lines, self.cols));
    }
}
