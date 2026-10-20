//! Printing text: ASCII runs, wide characters, zero-width marks, grapheme clusters, REP.
//!
//! The delayed wrap (see the module docs of `term`) is `Cursor::pending`: writing in the last
//! column leaves the cursor there, and the next graphic character first wraps. Autowrap (DECAWM)
//! off keeps the cursor in the last column and overwrites it. A run of ASCII is written one row
//! segment at a time with one call into the grid; everything else goes character by character.
//!
//! Widths are `unicode::width`'s (UAX #11 plus Emoji_Presentation). A zero-width character joins
//! the cell before it (`Grid::push_mark`). Mode 2027 is on from the start: characters are grouped
//! by UAX #29 and a cluster is one cell group whose width `unicode::extend_width` follows character
//! by character: the first character's, 2 once a spacing mark, a conjunct's consonant or a VS16
//! after an emoji variation base joins (terminal-unicode-core, "Variation Selector 16"), and 1
//! again when a VS15 joins such a base (UTS #51 ED-8a; the terminal-unicode-core text leaves the
//! width alone, which is the one place this terminal follows the Unicode standard and wcwidth instead).
//! A cluster that no longer fits at the right margin moves to the next line whole.

use crate::grid::MAX_MARKS;
use crate::types::{Column, Line, Point};
use crate::unicode::{self, GraphemeState};

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

impl Terminal {
    /// A run of printable ASCII (0x20..=0x7E).
    #[inline]
    pub(in crate::terminal) fn print_ascii_run(&mut self, run: &[u8]) {
        self.history_just_cleared = false;
        if !self.charsets.plain() {
            self.print_mapped(run);
            return;
        }
        let Some(&last) = run.last() else { return };
        self.last_char = Some(char::from(last));
        let head = self.write_ascii(run);
        if self.modes.contains(Modes::GRAPHEME_CLUSTERS) {
            self.start_cluster_after_ascii(char::from(last), head);
        }
    }

    /// The bytes of `run` through the invoked character sets, one at a time until the sets are
    /// plain again (a single shift is used up by the first byte).
    fn print_mapped(&mut self, mut run: &[u8]) {
        self.cluster.open = false;
        while let Some((&byte, rest)) = run.split_first() {
            if self.charsets.plain() {
                self.print_ascii_run(run);
                return;
            }
            let c = self.charsets.take_active().map(byte);
            self.last_char = Some(c);
            if c.is_ascii() {
                self.write_ascii(&[c as u8]);
            } else {
                self.put_char(c, 1);
            }
            run = rest;
        }
    }

    /// Write ASCII at the cursor, wrapping at the right edge. Returns the column the last byte
    /// went to: the cursor alone cannot say (it also stands at the right edge after a write that
    /// ended one column short of it, and after one that filled it with autowrap off).
    fn write_ascii(&mut self, mut run: &[u8]) -> usize {
        let autowrap = self.modes.contains(Modes::AUTOWRAP);
        let insert = self.modes.contains(Modes::INSERT);
        while !run.is_empty() {
            if self.cursor.pending {
                if autowrap {
                    self.wrap();
                } else {
                    self.cursor.pending = false;
                }
            }
            let y = self.cursor.y;
            let right = self.right_limit();
            let x = self.cursor.x.min(right);
            let (head, rest) = run.split_at(run.len().min(right + 1 - x));
            if insert {
                let bg = self.erase_color();
                self.grid.insert_cells(y, x, right, head.len(), bg);
            }
            self.grid.write_ascii(y, x, head, &self.pen);
            let end = x + head.len();
            if end <= right {
                self.cursor.x = end;
                return end - 1;
            }
            self.cursor.x = right;
            if autowrap {
                self.cursor.pending = true;
            } else {
                // Without autowrap everything past the edge lands on the last cell.
                if let Some(&last) = rest.last() {
                    self.grid.write_ascii(y, right, &[last], &self.pen);
                }
                return right;
            }
            run = rest;
        }
        self.cursor.x
    }

    /// The delayed wrap happens: the row is marked as continuing, and the cursor goes to the start
    /// of the next line (the left margin), scrolling at the bottom margin.
    pub(in crate::terminal) fn wrap(&mut self) {
        let y = self.cursor.y;
        // A double-width row has no soft wrap: its text is not rejoined with the next row's
        // (reflow leaves it alone, and the half it hides would come between them).
        if !self.grid.live_line_size(y).is_double() {
            self.grid.set_wrapped(y, true);
        }
        let x = if self.cursor.x <= self.margins.right { self.margins.left } else { 0 };
        if y == self.margins.bottom {
            self.scroll_up(1);
        } else if y + 1 < self.rows {
            self.cursor.y += 1;
        }
        self.cursor.x = x;
        self.cursor.pending = false;
        self.fit_cursor();
    }

    /// A printable character from a multi-byte sequence.
    pub(in crate::terminal) fn print_char(&mut self, c: char) {
        self.history_just_cleared = false;
        // In UTF-8 the C1 range is not control characters, and nothing to draw either.
        if ('\u{80}'..='\u{9f}').contains(&c) {
            return;
        }
        if self.modes.contains(Modes::GRAPHEME_CLUSTERS) {
            self.print_clustered(c);
            return;
        }
        match unicode::width(c) {
            0 => self.attach_mark(c),
            width => {
                if self.put_char(c, width) {
                    self.last_char = Some(c);
                }
            }
        }
    }

    /// Put `c` of `width` 1 or 2 at the cursor and move past it. False when nothing was printed:
    /// a wide character with one cell left and autowrap off is dropped, and then it is not the
    /// last character printed nor a cluster to extend.
    fn put_char(&mut self, c: char, width: u8) -> bool {
        let autowrap = self.modes.contains(Modes::AUTOWRAP);
        if self.cursor.pending {
            if autowrap {
                self.wrap();
            } else {
                self.cursor.pending = false;
            }
        }
        let mut right = self.right_limit();
        if width == 2 && self.cursor.x >= right {
            // Only one cell is left: the wide character goes to the next line, leaving a blank.
            if !autowrap {
                return false;
            }
            if right == self.cols - 1 {
                self.grid.write_spacer_head(self.cursor.y, &self.pen);
            }
            self.wrap();
            right = self.right_limit();
        }
        let (x, y) = (self.cursor.x, self.cursor.y);
        if self.modes.contains(Modes::INSERT) {
            let bg = self.erase_color();
            self.grid.insert_cells(y, x, right, usize::from(width), bg);
        }
        self.grid.write_cluster(y, x, c, &[], width, &self.pen);
        self.cluster.head = x;
        self.advance_past(x + usize::from(width), right);
        true
    }

    /// The cursor after a write that ended just before column `end`.
    fn advance_past(&mut self, end: usize, right: usize) {
        if end <= right {
            self.cursor.x = end;
        } else {
            self.cursor.x = right;
            self.cursor.pending = self.modes.contains(Modes::AUTOWRAP);
        }
    }

    /// A zero-width character (a combining mark, a joiner, a variation selector) joins the
    /// character before the cursor, or the cell under it at the start of a line. On a blank cell
    /// it makes a blank with a mark: the mark is kept for whoever draws it, nothing is lost.
    fn attach_mark(&mut self, c: char) {
        let x = self.cursor.x;
        // With a pending wrap the character to join is under the cursor. So it is with autowrap
        // off after printing in the last column, which leaves the cursor there too; a blank
        // last cell means nothing was printed there (the cursor was moved to it).
        let on_cursor = self.cursor.pending
            || (!self.modes.contains(Modes::AUTOWRAP)
                && x == self.right_limit()
                && self.grid.live_cell(self.cursor.y, x).is_some_and(|cell| !cell.is_blank()));
        self.grid.push_mark(self.cursor.y, if on_cursor { x } else { x.saturating_sub(1) }, c);
    }

    // ---- mode 2027 ----

    /// After a run of ASCII the last byte is a cluster that later marks extend.
    #[inline]
    fn start_cluster_after_ascii(&mut self, last: char, head: usize) {
        self.cluster.state = GraphemeState::AFTER_ASCII;
        self.cluster.open = true;
        self.cluster.width = 1;
        self.cluster.last = last;
        self.cluster.head = head;
    }

    #[inline]
    fn print_clustered(&mut self, c: char) {
        let open = self.cluster.open;
        if !open {
            self.cluster.state = GraphemeState::new();
        }
        let step = self.cluster.state.step(c);
        if open && !step.breaks {
            self.extend_cluster(c);
        } else if step.width == 0 {
            self.cluster.open = false;
            self.attach_mark(c);
        } else {
            // `put_char` sets the head; the automaton already moved past `c`.
            let printed = self.put_char(c, step.width);
            if printed {
                self.last_char = Some(c);
            }
            self.cluster.open = printed;
            self.cluster.width = step.width;
            self.cluster.last = c;
        }
    }

    /// `c` belongs to the cluster just printed.
    fn extend_cluster(&mut self, c: char) {
        let (y, head) = (self.cursor.y, self.cluster.head);
        self.grid.push_mark(y, head, c);
        let before = self.cluster.last;
        self.cluster.last = c;
        match unicode::extend_width(self.cluster.width, before, c) {
            2 if self.cluster.width == 1 => self.widen_cluster(),
            1 if self.cluster.width == 2 => self.narrow_cluster(),
            _ => {}
        }
    }

    /// A VS15 made the two-cell cluster at `cluster.head` one cell wide: its second cell is blank
    /// again and the cursor steps back to it.
    fn narrow_cluster(&mut self) {
        let (y, head) = (self.cursor.y, self.cluster.head);
        let right = self.right_edge(head, y);
        if self.grid.set_width(y, head, 1) {
            self.cluster.width = 1;
            self.cursor.pending = false;
            self.advance_past(head + 1, right);
        }
    }

    /// A VS16, a spacing mark or a conjunct's consonant made the cluster at `cluster.head` two cells wide.
    fn widen_cluster(&mut self) {
        let (y, head) = (self.cursor.y, self.cluster.head);
        let right = self.right_edge(head, y);
        let autowrap = self.modes.contains(Modes::AUTOWRAP);
        if head < right {
            if self.grid.set_width(y, head, 2) {
                self.cluster.width = 2;
                self.advance_past(head + 2, right);
            }
        } else if autowrap && head == right {
            self.move_cluster_to_next_line();
        }
    }

    /// The cluster at the right edge no longer fits at its width: blank its cell, wrap, and write
    /// it again, whole, at the start of the next line.
    fn move_cluster_to_next_line(&mut self) {
        let (y, head) = (self.cursor.y, self.cluster.head);
        let at = Point::new(Line(y as i32), Column(head));
        let Some(base) = self.grid.cell(at).and_then(|cell| cell.character()) else { return };
        // The marks are copied out because erasing the cell drops them; they fit a stack buffer.
        let mut kept = ['\0'; MAX_MARKS];
        let marks = self.grid.marks_at(at);
        let count = marks.len().min(MAX_MARKS);
        kept[..count].copy_from_slice(&marks[..count]);
        let marks = &kept[..count];
        let bg = self.erase_color();
        self.grid.erase(y, head..head + 1, bg, false);
        if head == self.cols - 1 {
            self.grid.write_spacer_head(y, &self.pen);
        }
        self.wrap();
        let (x, y) = (self.cursor.x, self.cursor.y);
        let right = self.right_limit();
        self.grid.write_cluster(y, x, base, marks, 2, &self.pen);
        self.cluster.head = x;
        self.cluster.width = 2;
        self.advance_past(x + 2, right);
    }

    // ---- REP ----

    /// REP: print the last graphic character `n` more times, as if typed.
    pub(in crate::terminal) fn repeat_last(&mut self, n: usize) {
        let Some(c) = self.last_char else { return };
        if c.is_ascii() {
            let block = [c as u8; 256];
            let mut left = n;
            while left > 0 {
                let take = left.min(block.len());
                self.write_ascii(&block[..take]);
                left -= take;
            }
        } else {
            for _ in 0..n {
                self.print_char(c);
            }
        }
    }
}
