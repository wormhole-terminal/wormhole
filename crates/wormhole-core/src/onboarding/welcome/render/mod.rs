//! The welcome's pages as a pure function: the model, the screen's size and the clock in, a grid of
//! styled cells and the lines that can be clicked out. Nothing here writes to a terminal, so every step
//! at every size is a snapshot test.
//!
//! The clock is how long the program has been drawing (the wait for the start animation is over).
//! Titles type out a few milliseconds a character, and the lines of a list appear one by one; under
//! Reduce Motion, after a key, or on a step seen before, the model hands the pages a clock of for ever
//! and everything is simply there.
//!
//! Layout: a left margin of two cells and a blank row on top, then the `>_` mark at [`MARK_CELL`] (or
//! the logo at [`LOGO`]), wherever the window is sized, because the start animation aims the ship at
//! that cell. Colours are the terminal's palette only.
//!
//! This file is the page machinery (the typing schedule, rows, hits, the footer); the pages are
//! `first` (the welcome page and how it looks) and `steps` (notifications, the sidebar and agents,
//! what else is here).

mod first;
mod import;
mod steps;

use std::time::Duration;

use super::ansi::{Grid, Style, color, fit, text_width, wrap};
use super::art::{MARGIN, TOP};
use super::flow::Step;
use super::keys::Key;
use super::model::Welcome;

/// The one line under the name.
pub const TAGLINE: &str = "A terminal for working with AI coding agents.";

/// How fast the name types, then the lines under it, a character; a list's rows each appear this long
/// after the one before; and the pause after a line.
pub(super) const TYPE_NAME: Duration = Duration::from_millis(28);
pub(super) const TYPE_TEXT: Duration = Duration::from_millis(6);
pub(super) const ROW_EVERY: Duration = Duration::from_millis(40);
pub(super) const LINE_PAUSE: Duration = Duration::from_millis(70);

/// The screen's size in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Size {
    pub cols: usize,
    pub rows: usize,
}

/// What a click on a span of a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// The cursor goes to this line of the list and Return is pressed on it.
    Row(usize),
    /// A key in the footer.
    Key(Key),
}

/// A clickable span: cells `from..to` of `row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Hit {
    pub row: usize,
    pub from: usize,
    pub to: usize,
    pub target: Target,
}

impl Hit {
    pub(super) fn contains(&self, row: usize, col: usize) -> bool {
        self.row == row && (self.from..self.to).contains(&col)
    }
}

/// A drawn page.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Frame {
    pub grid: Grid,
    pub hits: Vec<Hit>,
}

const TITLE: Style = Style::PLAIN.bold();
const DIM: Style = Style::PLAIN.dim();
const ACCENT: Style = Style::PLAIN.fg(color::BLUE).bold();
const GOOD: Style = Style::PLAIN.fg(color::GREEN);
const BAD: Style = Style::PLAIN.fg(color::RED);
const ATTENTION: Style = Style::PLAIN.fg(color::YELLOW);
const MARK: Style = Style::PLAIN.fg(color::BRIGHT_WHITE).bold();

/// Draw the welcome as it looks `clock` into the program.
pub(super) fn render(welcome: &Welcome, size: Size, clock: Duration) -> Frame {
    let mut page = Page {
        grid: Grid::new(size.cols, size.rows),
        hits: Vec::new(),
        clock: welcome.page_clock(clock),
        t: Duration::ZERO,
        turn: Duration::ZERO,
        size,
    };
    let hints = match welcome.step() {
        Step::Welcome => page.welcome(welcome),
        Step::Look => page.look(welcome),
        Step::Notify => page.notify(welcome),
        Step::Sidebar => page.sidebar(welcome),
        Step::Extras => page.extras(),
    };
    page.problem_and_tour(welcome);
    page.footer(&hints);
    Frame { grid: page.grid, hits: page.hits }
}

/// A footer hint: the key as it is written, what it does, and the key a click on it presses.
type Hint<'a> = (&'a str, &'a str, Option<Key>);

struct Page {
    grid: Grid,
    hits: Vec<Hit>,
    /// How far into the page's entrance we are.
    clock: Duration,
    /// Where the next line starts in the entrance's schedule, and where the last one did.
    t: Duration,
    turn: Duration,
    size: Size,
}

impl Page {
    /// How many of `len` characters are out at the schedule time `start`, `per` a character.
    fn out(&self, start: Duration, per: Duration, len: usize) -> usize {
        if self.clock == Duration::MAX {
            return len;
        }
        if self.clock < start {
            return 0;
        }
        let done = (self.clock - start).as_micros() / per.as_micros().max(1);
        (done as usize + 1).min(len)
    }

    /// Draw the first `n` characters of `parts` from (`row`, `col`); returns where the whole would end.
    fn draw(&mut self, row: usize, col: usize, parts: &[(&str, Style)], n: usize) -> usize {
        let (mut at, mut left) = (col, n);
        for (text, style) in parts {
            let take: String = text.chars().take(left).collect();
            left -= take.chars().count();
            self.grid.put(row, at, &take, *style);
            at += text_width(text);
        }
        at
    }

    /// A line that types out, `per` a character; the next waits for it.
    fn typed(&mut self, row: usize, col: usize, parts: &[(&str, Style)], per: Duration) -> usize {
        let total: usize = parts.iter().map(|(text, _)| text.chars().count()).sum();
        let n = self.out(self.t, per, total);
        self.turn = self.t;
        self.t += per * u32::try_from(total).unwrap_or(u32::MAX) + LINE_PAUSE;
        self.draw(row, col, parts, n)
    }

    /// A line that is there from its turn on, the next a little later.
    fn shown(&mut self, row: usize, col: usize, parts: &[(&str, Style)]) -> usize {
        let end = self.also(row, col, parts, self.t);
        self.turn = self.t;
        self.t += ROW_EVERY;
        end
    }

    /// More of the line whose turn `shown` just took (its second column): out with it, not after it.
    fn also(&mut self, row: usize, col: usize, parts: &[(&str, Style)], turn: Duration) -> usize {
        let total: usize = parts.iter().map(|(text, _)| text.chars().count()).sum();
        let n = self.out(turn, ROW_EVERY, 1) * total;
        self.draw(row, col, parts, n)
    }

    fn hit(&mut self, row: usize, from: usize, to: usize, target: Target) {
        self.hits.push(Hit { row, from, to, target });
    }

    /// The width a page's text may take.
    fn text_width(&self) -> usize {
        self.size.cols.saturating_sub(2 * MARGIN).max(8)
    }

    /// A step's heading and the lines under it; returns the row the step continues from.
    fn heading(&mut self, title: &str, note: &str) -> usize {
        self.typed(TOP, MARGIN, &[(title, TITLE)], TYPE_NAME);
        let mut row = TOP + 1;
        for line in wrap(note, self.text_width()) {
            self.typed(row, MARGIN, &[(&line, DIM)], TYPE_TEXT);
            row += 1;
        }
        row + 1
    }

    /// A paragraph of `style` from (`row`, `col`), wrapped to the page's width less the column, one
    /// line after another as they come in. Returns the row after it.
    fn paragraph(&mut self, row: usize, col: usize, text: &str, style: Style) -> usize {
        let room = self.text_width().saturating_sub(col - MARGIN);
        let mut row = row;
        for line in wrap(text, room) {
            self.shown(row, col, &[(&line, style)]);
            row += 1;
        }
        row
    }

    // ---------------------------------------------------------------- the pieces

    /// A line of a list: the marker, the parts, and a click anywhere on them. Returns where the
    /// parts end.
    fn item_row(
        &mut self,
        row: usize,
        col: usize,
        selected: bool,
        parts: &[(&str, Style)],
        line: usize,
    ) -> usize {
        let marker = if selected { "❯" } else { " " };
        let mut all = vec![(marker, ACCENT), (" ", Style::PLAIN)];
        all.extend_from_slice(parts);
        let end = self.shown(row, col, &all);
        self.hit(row, col, end.max(col + 2), Target::Row(line));
        end
    }

    /// How many lines a list that starts at `row` may take, leaving `below` lines after it and the
    /// footer (and a tour's note) at the bottom.
    fn list_height(&self, row: usize, below: usize) -> usize {
        self.size.rows.saturating_sub(row + below + 2).max(1)
    }

    /// The lines at the bottom: what went wrong with the last request, and a tour's one dim note.
    fn problem_and_tour(&mut self, w: &Welcome) {
        let last = self.size.rows.saturating_sub(1);
        if let Some(problem) = w.problem()
            && last >= 3
        {
            let line = fit(problem, self.text_width());
            self.grid.put(last - 2, MARGIN, &line, BAD);
        }
        // With the footer: the page is whole when the last line of it is out.
        if !w.live() && last >= 2 && self.out(self.t, ROW_EVERY, 1) > 0 {
            let note = "Read-only tour: not connected to Wormhole, so nothing here changes anything.";
            self.grid.put(last - 1, MARGIN, &fit(note, self.text_width()), DIM);
        }
    }

    /// A step's button (Continue, Start), kept above the footer however short the window is.
    fn button(&mut self, row: usize, selected: bool, label: &str, line: usize) {
        let row = row.min(self.size.rows.saturating_sub(2));
        // On a window too short for the page it takes the place of what it would have sat on.
        self.grid.put(row, 0, &" ".repeat(self.size.cols), Style::PLAIN);
        self.item_row(row, MARGIN, selected, &[(label, if selected { ACCENT } else { TITLE })], line);
    }

    /// The key hints along the bottom row; each is clickable.
    fn footer(&mut self, hints: &[Hint<'_>]) {
        let row = self.size.rows.saturating_sub(1);
        // A page that ran out of room has drawn into this row: the hints are what it holds.
        self.grid.put(row, 0, &" ".repeat(self.size.cols), Style::PLAIN);
        let mut col = MARGIN;
        // It comes in last, when the page above it is out.
        let out = self.out(self.t, ROW_EVERY, 1) > 0;
        for (key, label, press) in hints {
            let width = text_width(key) + 1 + text_width(label);
            if col + width > self.size.cols.saturating_sub(MARGIN) {
                break;
            }
            if out {
                self.grid.put(row, col, key, TITLE);
                self.grid.put(row, col + text_width(key) + 1, label, DIM);
            }
            if let Some(press) = press {
                self.hit(row, col, col + width, Target::Key(*press));
            }
            col += width + 2;
        }
    }
}

/// Where the value of an option starts, after its label.
const VALUE_AT: usize = 11;

const UP_DOWN: Hint<'static> = ("↑↓", "choose", None);
const BACK: Hint<'static> = ("←", "back", Some(Key::Left));
const ESC_SKIP: Hint<'static> = ("esc", "skip", Some(Key::Esc));

/// The first line of a window of `height` lines onto a list of `len`, the cursor kept inside it.
fn window_start(cursor: usize, len: usize, height: usize) -> usize {
    if len <= height {
        return 0;
    }
    cursor.saturating_sub(height / 2).min(len - height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_onto_a_list_keeps_the_cursor_inside() {
        assert_eq!(window_start(0, 3, 5), 0, "it all fits");
        assert_eq!(window_start(0, 10, 4), 0);
        assert_eq!(window_start(5, 10, 4), 3);
        assert_eq!(window_start(9, 10, 4), 6, "at the end the window ends there");
    }
}
