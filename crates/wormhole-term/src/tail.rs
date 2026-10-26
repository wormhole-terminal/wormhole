//! What a program printed since the last look: the lines that scrolled past and the line the cursor
//! is on, for the output events (`wh events --output`).
//!
//! A [`Tail`] is a pin (`wormhole_vt::Pin`) on the first row not yet reported, which follows its
//! text while the screen scrolls and the width is reflowed. Each look reads the rows from the pin
//! to the cursor's line (the line the cursor is on is not finished, so it is reported as
//! `partial`, whole, each time it changes), joins soft-wrapped rows back into the lines the
//! program printed and moves the pin to the cursor's line. Rows are copied under the screen's lock
//! in chunks, like [`ScreenReader::capture`], and the pin is moved in the same hold, so the history
//! dropping rows between two holds loses only what it dropped (`lost`).
//!
//! It follows a program that prints lines. One that redraws the screen (a full-screen program on
//! the alternate screen) is reported as "the screen changed", nothing more: reading it is
//! [`ScreenReader::capture`]'s job. A program that moves the cursor back above what was reported
//! (a progress display, `clear`) has what it draws there reported again, as new output.

use wormhole_vt::{Column, Line, Modes, Pin, Point};

use crate::ScreenReader;
use crate::capture::{CHUNK, ReadScratch, RowBuf, copy_rows};

/// What a look may return: more is cut from the oldest, and counted as skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailLimits {
    pub lines: usize,
    pub bytes: usize,
}

/// A look at what was printed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TailRead {
    /// A full-screen program has the alternate screen: no lines, only `changed`.
    pub alt_screen: bool,
    /// On the alternate screen: the program printed since the last look.
    pub changed: bool,
    /// The lines that were finished, oldest first.
    pub lines: Vec<String>,
    /// The line the cursor is on, when it differs from the last one told (empty: it was cleared).
    pub partial: Option<String>,
    /// Lines printed and left out of `lines` (over the limits).
    pub skipped: usize,
    /// The history dropped rows that were not read yet: `lines` start at the oldest kept.
    pub lost: bool,
}

impl TailRead {
    /// Nothing to tell.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.partial.is_none() && self.skipped == 0 && !self.lost && !self.changed
    }
}

/// Where a terminal's output was last read to.
#[derive(Default)]
pub struct Tail {
    /// On the main screen's first unread row. The alternate screen has pins of its own, so this one
    /// is left alone while a program has that screen.
    pin: Option<Pin>,
    in_alt: bool,
    /// The cursor's line as last told.
    partial: String,
    /// The output revision of the last look, to tell whether the alternate screen was printed to.
    revision: u64,
}

/// Rows of finished lines one look reads at most: what is further back would be cut by the limits
/// anyway, and a flood can have written as many as the history holds.
const ROWS_PER_LINE_BUDGET: usize = 4;

/// The rows of the cursor's own line that are read for `partial`, at most.
const PARTIAL_ROWS: i32 = 64;

impl ScreenReader {
    /// What was printed since `tail` was last looked at, read in chunks under the lock. The first
    /// look only places the pin at the cursor's line: nothing printed before it is reported.
    pub fn tail(&self, tail: &mut Tail, limits: TailLimits, scratch: &mut ReadScratch) -> TailRead {
        let mut term = self.term.lock();
        let revision = self.shared.revision();
        if term.mode().contains(Modes::ALT_SCREEN) {
            tail.in_alt = true;
            let changed = revision != std::mem::replace(&mut tail.revision, revision);
            return TailRead { alt_screen: true, changed, ..TailRead::default() };
        }
        tail.revision = revision;
        if std::mem::take(&mut tail.in_alt) {
            // The pin is on the main screen's grid, which shows again: let it go and start afresh.
            if let Some(pin) = tail.pin.take() {
                term.unpin(pin);
            }
            tail.partial.clear();
        }
        let cursor = term.grid_cursor();
        let (cursor_line, cursor_col) = (cursor.line.0, cursor.column.0);
        let mut cursor_start = cursor_line;
        while cursor_start > term.topmost_line().0 && term.wraps(Line(cursor_start - 1)) {
            cursor_start -= 1;
        }
        let at = |line: i32| Point::new(Line(line), Column(0));
        let Some(pin) = tail.pin else {
            tail.pin = Some(term.pin(at(cursor_start)));
            let from = cursor_start.max(cursor_line - PARTIAL_ROWS);
            let n = copy_rows(&term, from, cursor_line, scratch);
            drop(term);
            tail.partial = partial_text(scratch.rows(n), cursor_col);
            // What the cursor's line shows now is told; nothing printed before is.
            return TailRead {
                partial: Some(tail.partial.clone()).filter(|text| !text.is_empty()),
                ..TailRead::default()
            };
        };
        let mut lost = false;
        let mut next = match term.pin_point(pin) {
            Some(point) => point.line.0,
            None => {
                lost = true;
                term.topmost_line().0
            }
        };
        if cursor_start < next {
            // The program went back above what was read: what it draws there is new.
            next = cursor_start;
        }
        let mut skipped_rows = 0;
        let budget = i32::try_from(limits.lines * ROWS_PER_LINE_BUDGET).unwrap_or(i32::MAX);
        if cursor_start - next > budget {
            skipped_rows += (cursor_start - next - budget) as usize;
            next = cursor_start - budget;
        }
        // A line cut across by the skip starts in the row before: begin at the next whole one.
        while next < cursor_start && next > term.topmost_line().0 && term.wraps(Line(next - 1)) {
            next += 1;
            skipped_rows += 1;
        }
        let (mut cursor_start, mut cursor_line) = (cursor_start, cursor_line);
        // More than one chunk: the cursor's own rows are pinned too, since output scrolling between
        // two holds renumbers every line.
        let mut ends = None;
        let mut lines = Joined::default();
        let mut partial = None;
        loop {
            let chunk_end = (next + CHUNK as i32 - 1).min(cursor_line);
            let n = copy_rows(&term, next, chunk_end, scratch);
            // Everything before the cursor's line is finished: the pin follows what has been read.
            term.move_pin(pin, at((chunk_end + 1).min(cursor_start)));
            let done = chunk_end >= cursor_line;
            match (done, ends) {
                (false, None) => ends = Some((term.pin(at(cursor_start)), term.pin(at(cursor_line)))),
                (true, Some((start, end))) => {
                    term.unpin(start);
                    term.unpin(end);
                }
                _ => {}
            }
            drop(term);
            let finished = usize::try_from(cursor_start.min(chunk_end + 1) - next).unwrap_or(0).min(n);
            lines.add(&scratch.rows(n)[..finished]);
            if done {
                let from = usize::try_from(cursor_start.max(cursor_line - PARTIAL_ROWS) - next).unwrap_or(0);
                partial = Some(partial_text(&scratch.rows(n)[from.min(n)..], cursor_col));
                break;
            }
            term = self.term.lock();
            // A program that took the screen between two holds ends this look, and the pins with
            // it: they are out of reach of the screen that shows now.
            let (Some((start, end)), false) = (ends, term.mode().contains(Modes::ALT_SCREEN)) else { break };
            // Behind the pins: the history may have dropped what they held meanwhile.
            next = match term.pin_point(pin) {
                Some(point) => point.line.0,
                None => {
                    lost = true;
                    term.topmost_line().0
                }
            };
            match (term.pin_point(start), term.pin_point(end)) {
                (Some(start_at), Some(end_at)) => {
                    (cursor_start, cursor_line) = (start_at.line.0, end_at.line.0)
                }
                _ => break,
            }
        }
        let (finished, cut) = lines.finish(limits);
        let partial = partial.filter(|text| *text != tail.partial);
        if let Some(text) = &partial {
            tail.partial.clone_from(text);
        }
        TailRead {
            alt_screen: false,
            changed: false,
            lines: finished,
            partial,
            skipped: skipped_rows + cut,
            lost,
        }
    }

    /// Let go of `tail`'s pin, so the grid stops tracking it through reflows. Not while a program
    /// has the alternate screen: the pin belongs to the main one, out of reach until it returns
    /// (and then the next look of a tail that is still used lets it go itself).
    pub fn release(&self, tail: &mut Tail) {
        let Some(pin) = tail.pin.take() else { return };
        let mut term = self.term.lock();
        if !term.mode().contains(Modes::ALT_SCREEN) {
            term.unpin(pin);
        }
    }
}

/// The text of the cursor's line from `rows`, which end at its row: up to the cursor, or to the
/// last character when that is further (a prompt's trailing space is the cursor's column).
fn partial_text(rows: &[RowBuf], cursor_col: usize) -> String {
    let mut text = String::new();
    for (at, row) in rows.iter().enumerate() {
        let upto = if at + 1 == rows.len() { cursor_col.max(row.text_end()) } else { row.len() };
        row.push_text(&mut text, upto);
    }
    text
}

/// Finished rows put together into the lines the program printed.
#[derive(Default)]
struct Joined {
    lines: Vec<String>,
    open: String,
    open_rows: u32,
}

impl Joined {
    fn add(&mut self, rows: &[RowBuf]) {
        for row in rows {
            row.push_text(&mut self.open, row.len());
            self.open_rows += 1;
            if !row.wrapped {
                self.end_line();
            }
        }
    }

    fn end_line(&mut self) {
        let kept = self.open.trim_end().len();
        self.open.truncate(kept);
        self.lines.push(std::mem::take(&mut self.open));
        self.open_rows = 0;
    }

    /// The lines within `limits`, the newest; how many were left out.
    fn finish(mut self, limits: TailLimits) -> (Vec<String>, usize) {
        if self.open_rows > 0 {
            self.end_line();
        }
        let mut cut = self.lines.len().saturating_sub(limits.lines);
        let mut bytes: usize = self.lines[cut..].iter().map(String::len).sum();
        while bytes > limits.bytes && cut + 1 < self.lines.len() {
            bytes -= self.lines[cut].len();
            cut += 1;
        }
        self.lines.drain(..cut);
        if let Some(only) = self.lines.first_mut().filter(|line| line.len() > limits.bytes) {
            let mut start = only.len() - limits.bytes;
            while !only.is_char_boundary(start) {
                start += 1;
            }
            only.drain(..start);
        }
        (self.lines, cut)
    }
}

#[cfg(test)]
mod tests {
    use super::{Tail, TailLimits, TailRead};
    use crate::capture::ReadScratch;
    use crate::screen::Screen;
    use crate::{GridSize, ScreenReader};

    const LIMITS: TailLimits = TailLimits { lines: 500, bytes: 64 * 1024 };

    fn rig(cols: u16, rows: u16, history: usize) -> (ScreenReader, Tail, ReadScratch) {
        let size = GridSize { cols, rows, cell_width: 8.0, cell_height: 16.0 };
        (ScreenReader::of(Screen::new(size, history)), Tail::default(), ReadScratch::default())
    }

    fn look(rig: &mut (ScreenReader, Tail, ReadScratch)) -> TailRead {
        rig.0.tail(&mut rig.1, LIMITS, &mut rig.2)
    }

    fn lines(read: &TailRead) -> Vec<&str> {
        read.lines.iter().map(String::as_str).collect()
    }

    #[test]
    fn the_first_look_reports_nothing_and_later_ones_what_was_printed_since() {
        let mut rig = rig(30, 5, 100);
        rig.0.feed(b"before\r\n$ ");
        let first = look(&mut rig);
        assert_eq!(
            (lines(&first), first.partial.as_deref()),
            (vec![], Some("$ ")),
            "what was there before is not output"
        );
        rig.0.feed(b"ls\r\na.rs\r\nb.rs\r\n$ ");
        let read = look(&mut rig);
        assert_eq!(
            (lines(&read), read.partial.as_deref()),
            (vec!["$ ls", "a.rs", "b.rs"], None),
            "the cursor's line is as told"
        );
        assert!(look(&mut rig).is_empty(), "nothing new, nothing to tell");
        rig.0.feed(b"x");
        let read = look(&mut rig);
        assert_eq!(
            (lines(&read), read.partial.as_deref()),
            (vec![], Some("$ x")),
            "only the cursor's line grew"
        );
    }

    #[test]
    fn lines_that_scrolled_into_the_history_are_still_reported_once() {
        let mut rig = rig(30, 4, 1000);
        look(&mut rig);
        for n in 0..50 {
            rig.0.feed(format!("line {n}\r\n").as_bytes());
        }
        let read = look(&mut rig);
        assert_eq!(read.lines.len(), 50);
        assert_eq!((read.lines[0].as_str(), read.lines[49].as_str()), ("line 0", "line 49"));
        assert_eq!((read.skipped, read.lost), (0, false));
        assert!(look(&mut rig).is_empty());
    }

    #[test]
    fn wrapped_rows_are_one_line_also_after_a_reflow() {
        let mut rig = rig(10, 6, 100);
        look(&mut rig);
        rig.0.feed(b"0123456789abcdef\r\nnext\r\n");
        // The window narrows before the lines are looked at: the rows wrap again.
        let _ = rig.0.term.lock().resize(GridSize { cols: 6, rows: 6, cell_width: 8.0, cell_height: 16.0 });
        let read = look(&mut rig);
        assert_eq!(lines(&read), ["0123456789abcdef", "next"]);
    }

    #[test]
    fn output_over_the_limits_keeps_the_newest_and_counts_the_rest() {
        let mut rig = rig(30, 4, 5000);
        look(&mut rig);
        for n in 0..2500 {
            rig.0.feed(format!("line {n}\r\n").as_bytes());
        }
        let read = look(&mut rig);
        assert_eq!(read.lines.len(), 500);
        assert_eq!(read.lines[499], "line 2499");
        assert_eq!(read.skipped, 2000, "{read:?}");
        let few = rig.0.tail(&mut rig.1, TailLimits { lines: 3, bytes: 64 * 1024 }, &mut rig.2);
        assert!(few.is_empty());
        rig.0.feed(b"a\r\nb\r\nc\r\nd\r\ne\r\n");
        let few = rig.0.tail(&mut rig.1, TailLimits { lines: 3, bytes: 64 * 1024 }, &mut rig.2);
        assert_eq!((lines(&few), few.skipped), (vec!["c", "d", "e"], 2));
        rig.0.feed(b"0123456789\r\n0123456789\r\n");
        let bytes = rig.0.tail(&mut rig.1, TailLimits { lines: 100, bytes: 12 }, &mut rig.2);
        assert_eq!((lines(&bytes), bytes.skipped), (vec!["0123456789"], 1));
    }

    #[test]
    fn rows_the_history_dropped_before_they_were_read_are_said_to_be_lost() {
        let mut rig = rig(30, 4, 100);
        look(&mut rig);
        for n in 0..30_000 {
            rig.0.feed(format!("line {n}\r\n").as_bytes());
        }
        let read = look(&mut rig);
        assert!(read.lost, "{read:?}");
        assert_eq!(read.lines.last().map(String::as_str), Some("line 29999"));
        assert!(!look(&mut rig).lost);
    }

    #[test]
    fn a_full_screen_program_is_only_told_as_the_screen_changing() {
        let mut rig = rig(30, 4, 100);
        look(&mut rig);
        rig.0.feed(b"one\r\n\x1b[?1049h\x1b[2JVIM");
        // The revision moves with the PTY's wake-ups; stand in for one.
        let alt = look(&mut rig);
        assert!(alt.alt_screen && alt.lines.is_empty() && alt.partial.is_none());
        rig.0.feed(b"\x1b[?1049lback\r\n");
        let read = look(&mut rig);
        assert!(!read.alt_screen);
        assert_eq!(lines(&read), Vec::<&str>::new(), "the main screen starts afresh at the cursor: {read:?}");
        rig.0.feed(b"after\r\n");
        assert_eq!(lines(&look(&mut rig)), ["after"]);
    }

    #[test]
    fn a_program_that_clears_the_screen_has_what_it_prints_next_reported() {
        let mut rig = rig(30, 6, 100);
        rig.0.feed(b"a\r\nb\r\nc\r\nd\r\n");
        look(&mut rig);
        rig.0.feed(b"\x1b[2J\x1b[Hfresh 1\r\nfresh 2\r\n");
        let read = look(&mut rig);
        assert_eq!(lines(&read), ["fresh 1", "fresh 2"]);
    }

    #[test]
    fn more_than_a_chunk_is_read_in_pieces_without_losing_a_line() {
        let mut rig = rig(30, 5, 5000);
        look(&mut rig);
        for n in 0..900 {
            rig.0.feed(format!("line {n}\r\n").as_bytes());
        }
        let read = look(&mut rig);
        assert_eq!(read.lines.len(), 500, "capped; the rest is counted");
        assert_eq!(read.skipped, 400);
        assert_eq!(read.lines[0], "line 400");
        assert_eq!(read.lines[499], "line 899");
        let limits = TailLimits { lines: 1000, bytes: 1 << 20 };
        for n in 0..900 {
            rig.0.feed(format!("more {n}\r\n").as_bytes());
        }
        let read = rig.0.tail(&mut rig.1, limits, &mut rig.2);
        assert_eq!(read.lines.len(), 900);
        assert_eq!((read.lines[0].as_str(), read.lines[899].as_str()), ("more 0", "more 899"));
    }

    #[test]
    fn releasing_a_tail_gives_its_pin_back() {
        let mut rig = rig(30, 5, 100);
        look(&mut rig);
        assert!(rig.1.pin.is_some());
        rig.0.release(&mut rig.1);
        assert!(rig.1.pin.is_none());
    }
}
