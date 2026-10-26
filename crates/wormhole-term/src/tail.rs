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

use wormhole_vt::{Column, Line, Pin, Point};

use crate::ScreenReader;
use crate::capture::{CHUNK, ReadScratch, RowBuf, copy_rows};
use crate::screen::Screen;

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

impl Tail {
    /// A tail whose first look tells everything the terminal has printed (see [`Tail`]).
    pub fn from_top() -> Self {
        Self { from_top: true, ..Self::default() }
    }
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
    /// The first look starts at the top of the history instead of the cursor: a terminal first heard of
    /// by its output has printed nothing that was not meant to be told.
    from_top: bool,
    /// On the main screen's first unread row. The alternate screen has pins of its own, so this one
    /// is left alone while a program has that screen.
    pin: Option<Pin>,
    in_alt: bool,
    /// The cursor's line as last told.
    partial: String,
    /// The output revision of the last look, to tell whether the alternate screen was printed to.
    revision: u64,
    /// The screen the pin was made on (`Screen::epoch`).
    epoch: u64,
}

/// Rows of finished lines one look reads at most: what is further back would be cut by the limits
/// anyway, and a flood can have written as many as the history holds.
const ROWS_PER_LINE_BUDGET: usize = 4;

/// The rows a soft-wrapped line is walked back over to find where it starts, at most.
const LINE_ROWS: i32 = 4096;

/// The rows of the cursor's own line that are read for `partial`, at most.
const PARTIAL_ROWS: i32 = 64;

impl ScreenReader {
    /// What was printed since `tail` was last looked at, read in chunks under the lock. The first
    /// look only places the pin at the cursor's line: nothing printed before it is reported, only
    /// what that line shows.
    pub fn tail(&self, tail: &mut Tail, limits: TailLimits, scratch: &mut ReadScratch) -> TailRead {
        self.tail_between(tail, limits, scratch, || {})
    }

    /// [`tail`](Self::tail), calling `between` each time the lock is let go of between two chunks:
    /// a test's way to let output (or a program's switch of screens) arrive there.
    pub(crate) fn tail_between(
        &self,
        tail: &mut Tail,
        limits: TailLimits,
        scratch: &mut ReadScratch,
        mut between: impl FnMut(),
    ) -> TailRead {
        let mut term = self.term.lock();
        term.reap();
        let revision = self.shared.revision();
        if term.epoch() != tail.epoch {
            // A hosted terminal's replica was made again: every pin went with the old one.
            *tail = Tail { epoch: term.epoch(), from_top: tail.from_top, ..Tail::default() };
        }
        if term.on_alt_screen() {
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
        let (mut cursor_start, mut cursor_line, mut cursor_col) = cursor_lines(&term);
        let at = |line: i32| Point::new(Line(line), Column(0));
        if tail.from_top && tail.pin.is_none() {
            tail.from_top = false;
            let top = term.topmost_line().0;
            tail.pin = Some(term.pin(at(top)));
        }
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
        let mut lines = Joined::default();
        let mut partial = None;
        if cursor_line - next < CHUNK as i32 {
            // Everything, the cursor's line too, in one hold.
            let n = copy_rows(&term, next, cursor_line, scratch);
            term.move_pin(pin, at(cursor_start));
            drop(term);
            let finished = usize::try_from(cursor_start - next).unwrap_or(0).min(n);
            lines.add(&scratch.rows(n)[..finished]);
            let from = usize::try_from(cursor_start.max(cursor_line - PARTIAL_ROWS) - next).unwrap_or(0);
            partial = Some(partial_text(&scratch.rows(n)[from.min(n)..], cursor_col));
        } else {
            // The finished rows, a chunk at a time: only those, so a line of any length (its rows are
            // the cursor's own, not finished) is never walked over again. `mark` keeps where they end
            // while output renumbers the lines between two holds.
            let mark = term.pin(at(cursor_start));
            let epoch = term.epoch();
            let mut complete = true;
            while next < cursor_start {
                let chunk_end = (next + CHUNK as i32 - 1).min(cursor_start - 1);
                let n = copy_rows(&term, next, chunk_end, scratch);
                term.move_pin(pin, at(chunk_end + 1));
                drop(term);
                lines.add(scratch.rows(n));
                between();
                term = self.term.lock();
                if term.epoch() != epoch {
                    // Every pin went with the screen that was replaced.
                    *tail = Tail { epoch: term.epoch(), ..Tail::default() };
                    complete = false;
                    break;
                }
                if term.on_alt_screen() {
                    // A program took the screen between two holds: the mark is on the main grid, out of
                    // reach, and goes when it is back. This look ends here.
                    term.release_pin(mark);
                    return TailRead { lines: lines.finish(limits).0, ..TailRead::default() };
                }
                next = match term.pin_point(pin) {
                    Some(point) => point.line.0,
                    None => {
                        lost = true;
                        term.topmost_line().0
                    }
                };
                match term.pin_point(mark) {
                    Some(point) => cursor_start = point.line.0,
                    None => break,
                }
            }
            if complete {
                term.release_pin(mark);
                // The cursor's own line, as it is now.
                (cursor_start, cursor_line, cursor_col) = cursor_lines(&term);
                let from = cursor_start.max(cursor_line - PARTIAL_ROWS);
                let n = copy_rows(&term, from, cursor_line, scratch);
                drop(term);
                partial = Some(partial_text(scratch.rows(n), cursor_col));
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

    /// Let go of `tail`'s pin, so the grid stops tracking it through reflows. While a program has the
    /// alternate screen the pin belongs to the main one, out of reach: it is removed once that is back.
    pub fn release(&self, tail: &mut Tail) {
        let Some(pin) = tail.pin.take() else { return };
        let mut term = self.term.lock();
        if term.epoch() == tail.epoch {
            term.release_pin(pin);
        }
    }
}

/// Where the cursor's own line starts, where it is, and its column: the line is walked back over
/// soft wraps, at most [`LINE_ROWS`] rows (one that long is cut there).
fn cursor_lines(term: &Screen) -> (i32, i32, usize) {
    let cursor = term.grid_cursor();
    let (line, col) = (cursor.line.0, cursor.column.0);
    let mut start = line;
    while start > term.topmost_line().0 && line - start < LINE_ROWS && term.wraps(Line(start - 1)) {
        start -= 1;
    }
    (start, line, col)
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

    #[test]
    fn a_line_of_many_chunks_without_its_newline_is_told_once_and_the_look_ends() {
        let mut rig = rig(10, 5, 20_000);
        look(&mut rig);
        // 3000 rows of one line, then a few finished lines before it.
        rig.0.feed(b"before\r\n");
        rig.0.feed("x".repeat(30_000).as_bytes());
        let started = std::time::Instant::now();
        let read = look(&mut rig);
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "the look did not end");
        assert_eq!(lines(&read), ["before"]);
        assert!(
            read.partial.as_deref().is_some_and(|p| p.starts_with('x') && p.len() <= 65 * 10),
            "{read:?}"
        );
        // The next look has nothing new, and does not walk the line again.
        let again = look(&mut rig);
        assert!(again.lines.is_empty() && again.partial.is_none(), "{again:?}");
        // The line ends. It is longer than a look reads of finished rows, so its start is skipped, and
        // what follows it is told.
        rig.0.feed(b"\r\nnext\r\n");
        let done = look(&mut rig);
        assert_eq!(done.lines.last().map(String::as_str), Some("next"));
        assert!(done.skipped > 0, "{done:?}");
    }

    #[test]
    fn a_screen_swapped_between_chunks_ends_the_look_and_leaves_no_pin() {
        let mut rig = rig(30, 5, 5000);
        look(&mut rig);
        for n in 0..900 {
            rig.0.feed(format!("line {n}\r\n").as_bytes());
        }
        let before = rig.0.term.lock().pin_slots_for_test();
        let read = rig.0.tail_between(&mut rig.1, LIMITS, &mut rig.2, || rig.0.feed(b"\x1b[?1049h"));
        assert!(!read.alt_screen && read.partial.is_none());
        rig.0.feed(b"\x1b[?1049l");
        let back = look(&mut rig);
        assert!(!back.alt_screen);
        assert_eq!(rig.0.term.lock().pin_slots_for_test(), before, "the mark was reaped");
        rig.0.release(&mut rig.1);
        assert_eq!(rig.0.term.lock().pin_slots_for_test(), before - 1, "and the tail's own is given back");
    }

    #[test]
    fn a_screen_made_again_leaves_the_tail_without_a_pin_instead_of_unpinning_another() {
        let mut rig = rig(30, 5, 5000);
        look(&mut rig);
        let size = GridSize { cols: 30, rows: 5, cell_width: 8.0, cell_height: 16.0 };
        *rig.0.term.lock() = Screen::new(size, 5000);
        // A pin of the new screen's own, in the slot the old tail's pin had.
        let theirs = {
            use wormhole_vt::{Column, Line, Point};
            rig.0.term.lock().pin(Point::new(Line(0), Column(0)))
        };
        rig.0.release(&mut rig.1);
        assert!(rig.0.term.lock().pin_point(theirs).is_some(), "the new screen's pin is untouched");
        look(&mut rig);
        rig.0.feed(b"fresh\r\n");
        assert_eq!(lines(&look(&mut rig)), ["fresh"]);
    }
}
