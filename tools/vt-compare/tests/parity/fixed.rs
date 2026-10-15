//! alacritty with its delayed wrap fixed up, for the comparison.
//!
//! alacritty keeps the "wrap pending" flag across a line feed and across the sequences that edit
//! the line (`known-fail: alacritty` in the suite: a LF does not clear it), so the character
//! after a full row lands one row too low. That one difference would hide every other in a
//! stream. Before each of those controls this adapter asks where the cursor is and, if it is in
//! the last column, moves it right (`CSI C`), which clears the flag and has nowhere to go.

use vt_compare::engines::AlacrittyEngine;
use wormhole_vt_bench::{Cell, Cursor, Engine};

pub struct FixedAlacritty(AlacrittyEngine);

/// Whether a control that clears the delayed wrap in the new engine starts at `bytes[i]`.
fn clears_wrap(bytes: &[u8], i: usize) -> bool {
    match bytes[i] {
        0x09..=0x0c => true,
        0x1b => match bytes.get(i + 1) {
            Some(b'D' | b'E' | b'M') => true,
            Some(b'[') => {
                let rest = &bytes[i + 2..];
                let params = rest.iter().take_while(|&&b| (0x30..=0x3f).contains(&b)).count();
                // An intermediate makes it another function altogether.
                matches!(rest.get(params), Some(b'J' | b'K' | b'X' | b'@' | b'P' | b'L' | b'M' | b'Z' | b'I'))
            }
            _ => false,
        },
        _ => false,
    }
}

impl Engine for FixedAlacritty {
    const NAME: &'static str = "alacritty";

    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        FixedAlacritty(AlacrittyEngine::new(cols, rows, scrollback))
    }

    fn feed(&mut self, bytes: &[u8]) {
        let mut start = 0;
        for i in 0..bytes.len() {
            if clears_wrap(bytes, i) {
                self.0.feed(&bytes[start..i]);
                start = i;
                let (cols, _) = self.0.size();
                if self.0.cursor().col == cols - 1 {
                    self.0.feed(b"\x1b[C");
                }
            }
        }
        self.0.feed(&bytes[start..]);
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.0.resize(cols, rows);
    }
    fn size(&self) -> (usize, usize) {
        self.0.size()
    }
    fn row_text(&self, row: usize) -> String {
        // From the cells, as the new engine's, so that the half of a wide character that is left
        // over (see `cell`) is a blank.
        let (cols, _) = self.0.size();
        let text: String = (0..cols).map(|col| self.cell(row, col).text).collect();
        text.trim_end_matches(' ').to_string()
    }
    fn row_wrapped(&self, row: usize) -> bool {
        self.0.row_wrapped(row)
    }
    /// A wide character whose second half is gone (an erase or a shift cut it: `known-fail:
    /// alacritty` in the suite) is a blank, as in the new engine, and so is a second half
    /// without its first.
    fn cell(&self, row: usize, col: usize) -> Cell {
        let cell = self.0.cell(row, col);
        let (cols, _) = self.0.size();
        let orphan_head = cell.width == 2 && (col + 1 >= cols || self.0.cell(row, col + 1).width != 0);
        let orphan_tail = cell.width == 0 && (col == 0 || self.0.cell(row, col - 1).width != 2);
        if orphan_head || orphan_tail {
            // What was marked on it stays: the new engine puts a mark that follows an erase on
            // the blank it finds.
            let marks: String = cell.text.chars().skip(1).collect();
            Cell { text: format!(" {marks}"), width: 1, ..cell }
        } else {
            cell
        }
    }
    fn cursor(&self) -> Cursor {
        self.0.cursor()
    }
    fn scrollback_len(&self) -> usize {
        self.0.scrollback_len()
    }
    fn scrollback_text(&self, index: usize) -> String {
        self.0.scrollback_text(index)
    }
    fn scrollback_wrapped(&self, index: usize) -> bool {
        self.0.scrollback_wrapped(index)
    }
    fn take_replies(&mut self) -> Vec<u8> {
        self.0.take_replies()
    }
    fn title(&self) -> Option<String> {
        self.0.title()
    }
    fn alt_screen(&self) -> bool {
        self.0.alt_screen()
    }
}
