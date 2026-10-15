//! Find in scrollback: the history and the screen as text, searched for a query.
//!
//! The terminal's lock is held only to copy the grid's characters ([`GridText::copy`]); the
//! search itself runs on that copy, off the lock and off the UI thread, so a program printing
//! never waits for it. Soft-wrapped rows are one line, so a match may cross a wrap but never a
//! line break. Matches are in grid coordinates (`Line(0)` is the top of the live screen, the
//! history above it negative), which stay right until the next output scrolls the grid: the
//! view searches again when the terminal printed.

use regex::{Regex, RegexBuilder};
use wormhole_vt::types::{Cell, Column, Flags, Line, Point, RowExtras};

use super::screen::Screen;

/// The most matches one search keeps; the count then reads "10,000+".
pub const MAX_MATCHES: usize = 10_000;

/// The bar's three toggles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub match_case: bool,
    pub whole_word: bool,
    pub regex: bool,
}

/// One of [`FindOptions`]' toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindOption {
    MatchCase,
    WholeWord,
    Regex,
}

impl FindOption {
    pub const ALL: [FindOption; 3] = [FindOption::MatchCase, FindOption::WholeWord, FindOption::Regex];

    /// What its button's tooltip and VoiceOver call it.
    pub fn name(self) -> &'static str {
        match self {
            FindOption::MatchCase => "Match case",
            FindOption::WholeWord => "Whole word",
            FindOption::Regex => "Regular expression",
        }
    }
}

impl FindOptions {
    pub fn get(&self, option: FindOption) -> bool {
        match option {
            FindOption::MatchCase => self.match_case,
            FindOption::WholeWord => self.whole_word,
            FindOption::Regex => self.regex,
        }
    }

    pub fn toggle(&mut self, option: FindOption) {
        let flag = match option {
            FindOption::MatchCase => &mut self.match_case,
            FindOption::WholeWord => &mut self.whole_word,
            FindOption::Regex => &mut self.regex,
        };
        *flag = !*flag;
    }
}

/// What to look for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindQuery {
    pub text: String,
    pub options: FindOptions,
}

/// A query that isn't a pattern: a regular expression that doesn't parse (or is too big).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPattern;

impl FindQuery {
    /// The query as a regex; `None` for an empty one, which finds nothing.
    pub fn compile(&self) -> Result<Option<Regex>, InvalidPattern> {
        if self.text.is_empty() {
            return Ok(None);
        }
        let FindOptions { match_case, whole_word, regex } = self.options;
        let mut pattern = if regex { format!("(?:{})", self.text) } else { regex::escape(&self.text) };
        if whole_word {
            // Literal text gets a boundary only where it starts or ends with a word character, so
            // "--all" still finds itself after a space. A pattern can't be looked into: both ends.
            let word = |c: Option<char>| c.is_none_or(|c| c.is_alphanumeric() || c == '_');
            let (first, last) = (self.text.chars().next(), self.text.chars().next_back());
            let before = if regex || word(first) { r"\b" } else { "" };
            let after = if regex || word(last) { r"\b" } else { "" };
            pattern = format!("{before}{pattern}{after}");
        }
        RegexBuilder::new(&pattern)
            .case_insensitive(!match_case)
            .size_limit(1 << 22)
            .build()
            .map(Some)
            .map_err(|_| InvalidPattern)
    }
}

/// One match: its first and last cell (a wide character's last cell is its second), and the text
/// it matched, to tell whether the cells still hold it after the terminal printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindMatch {
    pub start: Point,
    pub end: Point,
    pub text: String,
}

impl FindMatch {
    /// The columns `[first, last]` this match covers on `line`, in a grid `cols` wide.
    pub fn columns_on(&self, line: Line, cols: usize) -> Option<(usize, usize)> {
        if line < self.start.line || line > self.end.line || cols == 0 {
            return None;
        }
        let first = if line == self.start.line { self.start.column.0 } else { 0 };
        let last = if line == self.end.line { self.end.column.0 } else { cols - 1 };
        (first <= last).then_some((first, last))
    }

    /// Whether the cells under the match still spell what it matched, the rows given by `row`
    /// (what a line holds now, in the match's coordinates); `None` when a row isn't at hand.
    pub fn holds<'a>(
        &self,
        row: impl Fn(Line) -> Option<(&'a [Cell], &'a RowExtras)>,
        cols: usize,
    ) -> Option<bool> {
        let mut text = String::with_capacity(self.text.len());
        for line in self.start.line.0..=self.end.line.0 {
            let (first, last) = self.columns_on(Line(line), cols)?;
            let (cells, extras) = row(Line(line))?;
            for (col, cell) in cells.get(first..=last)?.iter().enumerate().map(|(k, cell)| (first + k, cell))
            {
                if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                    continue;
                }
                text.push(if cell.c == '\0' { ' ' } else { cell.c });
                text.extend(extras.zerowidth(col));
            }
        }
        Some(text == self.text)
    }
}

/// What a search found, top to bottom.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindResults {
    pub matches: Vec<FindMatch>,
    /// More than [`MAX_MATCHES`] matched; only the first are kept.
    pub capped: bool,
}

impl FindResults {
    /// The matches that touch `line`, in order.
    pub fn on_line(&self, line: Line) -> impl Iterator<Item = (usize, &FindMatch)> {
        // Matches don't overlap, so their ends are in order too.
        let first = self.matches.partition_point(|m| m.end.line < line);
        self.matches[first..]
            .iter()
            .enumerate()
            .take_while(move |(_, m)| m.start.line <= line)
            .map(move |(ix, m)| (first + ix, m))
    }

    /// The match to start from with the view's bottom at `bottom`: the last one that starts at
    /// or above it (the newest output you can see or have scrolled past, as reading back up the
    /// history goes), else the first one below it.
    pub fn nearest(&self, bottom: Line) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        Some(self.matches.partition_point(|m| m.start.line <= bottom).saturating_sub(1))
    }
}

/// The view's offset into the history that shows `line` on a screen `rows` tall, when the
/// current `offset` doesn't: the line in the middle of the screen, as far as the history goes.
pub fn reveal_offset(line: Line, rows: usize, offset: usize, history: usize) -> Option<usize> {
    let row = line.0 + offset as i32;
    if (0..rows as i32).contains(&row) {
        return None;
    }
    Some((rows as i32 / 2 - line.0).clamp(0, history as i32) as usize)
}

/// One line of text: a row, or rows joined by soft wraps.
struct TextLine {
    /// The grid line of its first row.
    first: i32,
    text: String,
    /// Where each character starts in `text` and which cell it is in (counted across the rows).
    chars: Vec<CharAt>,
}

#[derive(Clone, Copy)]
struct CharAt {
    byte: u32,
    cell: u32,
    wide: bool,
}

/// The history and the screen as text, copied for a search.
pub struct GridText {
    cols: usize,
    /// The history's length when copied: output since moves the lines up by as many as it grew.
    history: usize,
    lines: Vec<TextLine>,
}

impl GridText {
    /// Copy the screen's characters (all of them: the history and the screen).
    pub(super) fn copy(screen: &Screen) -> Self {
        let cols = screen.columns();
        let (top, bottom) = (screen.topmost_line().0, screen.bottommost_line().0);
        let mut lines = Vec::new();
        let mut current = TextLine { first: top, text: String::new(), chars: Vec::new() };
        let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
        for line in top..=bottom {
            screen.copy_row(Line(line), &mut cells, Some(&mut extras));
            let base = (line - current.first) as usize * cols;
            for (col, cell) in cells.iter().enumerate() {
                if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                    continue;
                }
                current.chars.push(CharAt {
                    byte: current.text.len() as u32,
                    cell: (base + col) as u32,
                    wide: cell.flags.contains(Flags::WIDE_CHAR),
                });
                current.text.push(if cell.c == '\0' { ' ' } else { cell.c });
                current.text.extend(extras.zerowidth(col));
            }
            let wrapped = cells.last().is_some_and(|last| last.flags.contains(Flags::WRAPLINE));
            if !wrapped || line == bottom {
                let next = TextLine { first: line + 1, text: String::new(), chars: Vec::new() };
                let mut done = std::mem::replace(&mut current, next);
                trim_end(&mut done);
                lines.push(done);
            }
        }
        Self { cols, history: screen.history_size(), lines }
    }

    /// The history's length when this was copied.
    pub fn history(&self) -> usize {
        self.history
    }

    /// Every match of `regex`, top to bottom (at most [`MAX_MATCHES`]); empty matches don't count.
    pub fn find(&self, regex: &Regex) -> FindResults {
        let mut results = FindResults::default();
        for line in &self.lines {
            for found in regex.find_iter(&line.text).filter(|m| !m.is_empty()) {
                if results.matches.len() == MAX_MATCHES {
                    results.capped = true;
                    return results;
                }
                let first = line.char_at(found.start());
                let last = line.char_at(found.end() - 1);
                let end = last.cell + u32::from(last.wide);
                results.matches.push(FindMatch {
                    start: self.point(line.first, first.cell),
                    end: self.point(line.first, end),
                    text: found.as_str().to_string(),
                });
            }
        }
        results
    }

    fn point(&self, first: i32, cell: u32) -> Point {
        let cols = self.cols.max(1);
        let cell = cell as usize;
        Point::new(Line(first + (cell / cols) as i32), Column(cell % cols))
    }
}

impl TextLine {
    /// The character `byte` is in.
    fn char_at(&self, byte: usize) -> CharAt {
        let ix = self.chars.partition_point(|c| c.byte as usize <= byte).saturating_sub(1);
        self.chars[ix]
    }
}

/// Drop the blanks at the end of a line: a search for spaces shouldn't find the rest of the row.
fn trim_end(line: &mut TextLine) {
    let kept = line.text.trim_end_matches(' ').len();
    line.text.truncate(kept);
    let chars = line.chars.partition_point(|c| (c.byte as usize) < kept);
    line.chars.truncate(chars);
}

#[cfg(test)]
mod tests {
    use wormhole_vt::types::{Cell, Column, Line, Point, RowExtras};

    use super::{FindMatch, FindOptions, FindQuery, GridText, InvalidPattern, MAX_MATCHES};
    use crate::terminal::testing::TestTerm;

    fn term(cols: u16, rows: u16) -> TestTerm {
        TestTerm::new(cols, rows, 1000)
    }

    fn feed(term: &mut TestTerm, text: &str) {
        term.feed(text.as_bytes());
    }

    fn query(text: &str, options: FindOptions) -> FindQuery {
        FindQuery { text: text.into(), options }
    }

    /// (line, first column, line, last column) of every match.
    fn found(term: &TestTerm, query: &FindQuery) -> Vec<(i32, usize, i32, usize)> {
        let regex = query.compile().expect("valid").expect("not empty");
        let at = |p: Point| (p.line.0, p.column.0);
        GridText::copy(term.screen())
            .find(&regex)
            .matches
            .iter()
            .map(|m| {
                let ((l1, c1), (l2, c2)) = (at(m.start), at(m.end));
                (l1, c1, l2, c2)
            })
            .collect()
    }

    #[test]
    fn finds_every_match_in_the_history_and_on_the_screen() {
        let mut term = term(20, 3);
        feed(&mut term, "split one\r\nnothing\r\nsplit two split\r\nx\r\ny");
        // Five lines on three rows: the first two are in the history (lines -2 and -1).
        assert_eq!(
            found(&term, &query("split", FindOptions::default())),
            [(-2, 0, -2, 4), (0, 0, 0, 4), (0, 10, 0, 14)]
        );
    }

    #[test]
    fn case_is_ignored_unless_asked() {
        let mut term = term(20, 2);
        feed(&mut term, "Split split SPLIT");
        assert_eq!(found(&term, &query("split", FindOptions::default())).len(), 3);
        let case = FindOptions { match_case: true, ..FindOptions::default() };
        assert_eq!(found(&term, &query("split", case)), [(0, 6, 0, 10)]);
    }

    #[test]
    fn whole_words_and_literal_punctuation() {
        let mut term = term(30, 2);
        feed(&mut term, "split splits a.b axb --all");
        let words = FindOptions { whole_word: true, ..FindOptions::default() };
        assert_eq!(found(&term, &query("split", words)), [(0, 0, 0, 4)]);
        // Not a regex: the dot is a dot.
        assert_eq!(found(&term, &query("a.b", FindOptions::default())), [(0, 13, 0, 15)]);
        // Leading dashes aren't word characters, so they need no boundary before them.
        assert_eq!(found(&term, &query("--all", words)), [(0, 21, 0, 25)]);
    }

    #[test]
    fn regular_expressions() {
        let mut term = term(40, 2);
        feed(&mut term, "error[E0308] warning error[E0425]");
        let regex = FindOptions { regex: true, ..FindOptions::default() };
        assert_eq!(found(&term, &query(r"E\d{4}", regex)), [(0, 6, 0, 10), (0, 27, 0, 31)]);
        assert!(matches!(query("(unclosed", regex).compile(), Err(InvalidPattern)));
        assert!(matches!(query("", regex).compile(), Ok(None)), "an empty query finds nothing");
        // A pattern that can match nothing doesn't find every gap between characters.
        assert!(found(&term, &query("x*", regex)).is_empty());
    }

    #[test]
    fn a_match_across_a_soft_wrap_covers_both_rows() {
        let mut term = term(10, 3);
        feed(&mut term, "abcdefghsplit");
        // "split" starts at column 8 of the first row and ends on the next.
        assert_eq!(found(&term, &query("split", FindOptions::default())), [(0, 8, 1, 2)]);
        let m = FindMatch {
            start: Point::new(Line(0), Column(8)),
            end: Point::new(Line(1), Column(2)),
            text: "split".into(),
        };
        assert_eq!(m.columns_on(Line(0), 10), Some((8, 9)));
        assert_eq!(m.columns_on(Line(1), 10), Some((0, 2)));
        assert_eq!(m.columns_on(Line(2), 10), None);
    }

    #[test]
    fn wide_characters_keep_their_cells() {
        let mut term = term(20, 2);
        feed(&mut term, "日本 split 日本");
        assert_eq!(found(&term, &query("split", FindOptions::default())), [(0, 5, 0, 9)]);
        // A wide match ends on its last character's second cell.
        assert_eq!(found(&term, &query("日本", FindOptions::default())), [(0, 0, 0, 3), (0, 11, 0, 14)]);
    }

    #[test]
    fn blanks_at_the_end_of_a_row_are_not_text() {
        let mut term = term(10, 2);
        feed(&mut term, "a\r\nb");
        let regex = FindOptions { regex: true, ..FindOptions::default() };
        assert!(found(&term, &query("a ", regex)).is_empty());
    }

    #[test]
    fn too_many_matches_are_capped() {
        let mut term = term(100, 2);
        feed(&mut term, &"x".repeat(100));
        let regex = query("x", FindOptions::default()).compile().expect("valid").expect("not empty");
        let text = GridText::copy(term.screen());
        let results = text.find(&regex);
        assert_eq!(results.matches.len(), 100);
        assert!(!results.capped);
        let mut term = super::tests::term(200, 60);
        for _ in 0..60 {
            feed(&mut term, &format!("{}\r\n", "x".repeat(200)));
        }
        let results = GridText::copy(term.screen()).find(&regex);
        assert_eq!(results.matches.len(), MAX_MATCHES);
        assert!(results.capped);
    }

    #[test]
    fn a_match_knows_whether_its_cells_still_hold_it() {
        let mut term = term(10, 3);
        feed(&mut term, "abcdefghsplit 日");
        let regex = query("split", FindOptions::default()).compile().expect("valid").expect("not empty");
        let found = GridText::copy(term.screen()).find(&regex);
        let m = &found.matches[0];
        let rows = |term: &TestTerm| -> Vec<(Vec<Cell>, RowExtras)> {
            (0..term.screen_lines() as i32).map(|line| term.row(line)).collect()
        };
        let now = rows(&term);
        let row = |line: Line| now.get(line.0 as usize).map(|(cells, extras)| (cells.as_slice(), extras));
        assert_eq!(m.holds(row, 10), Some(true), "across the wrap");
        assert_eq!(m.holds(|_| None, 10), None, "rows out of reach");
        // The program redraws the line: the match is gone.
        feed(&mut term, "\x1b[1;1Habcdefghspl!t");
        let now = rows(&term);
        let row = |line: Line| now.get(line.0 as usize).map(|(cells, extras)| (cells.as_slice(), extras));
        assert_eq!(m.holds(row, 10), Some(false));
    }

    #[test]
    fn a_match_out_of_view_comes_to_the_middle() {
        use super::reveal_offset;
        // 10 rows, the live screen in view: a line on it needs no scrolling.
        assert_eq!(reveal_offset(Line(3), 10, 0, 100), None);
        // 40 lines up into the history: scrolled so that it is on row 5.
        assert_eq!(reveal_offset(Line(-40), 10, 0, 100), Some(45));
        // Near the top of the history: as far as it goes.
        assert_eq!(reveal_offset(Line(-99), 10, 0, 100), Some(100));
        // Scrolled up, a line on the live screen brings the view down again.
        assert_eq!(reveal_offset(Line(8), 10, 30, 100), Some(0));
        assert_eq!(reveal_offset(Line(-25), 10, 30, 100), None, "already in view");
    }

    #[test]
    fn the_nearest_match_and_the_matches_on_a_line() {
        let mut term = term(20, 4);
        feed(&mut term, "hit\r\nhit hit\r\n\r\nhit");
        let regex = query("hit", FindOptions::default()).compile().expect("valid").expect("not empty");
        let results = GridText::copy(term.screen()).find(&regex);
        assert_eq!(results.matches.len(), 4);
        assert_eq!(results.nearest(Line(3)), Some(3), "the bottom one when it is in view");
        assert_eq!(results.nearest(Line(2)), Some(2), "else the last one above the view's bottom");
        assert_eq!(results.nearest(Line(-5)), Some(0), "nothing above: the first below");
        let on_one: Vec<usize> = results.on_line(Line(1)).map(|(ix, _)| ix).collect();
        assert_eq!(on_one, [1, 2]);
        assert_eq!(results.on_line(Line(2)).count(), 0);
    }
}
