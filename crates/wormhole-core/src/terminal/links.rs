//! Links in the terminal: OSC 8 hyperlinks that programs attach to text, and URLs
//! written out as plain text. ⌘-click opens one (see [`Openable`] for which).
//!
//! A link may run over several rows when a long line wraps, so the search
//! covers the whole wrapped line around the point (a bounded number of rows,
//! since this runs under the terminal's lock while the pointer moves).

use std::ops::Range;
use std::path::PathBuf;

use wormhole_vt::types::{Cell, Column, Flags, Line, Point, RowExtras};

use super::osc::percent_decode;
use super::procinfo;
use super::screen::Screen;

/// The most rows one wrapped line is searched over, each way.
const MAX_WRAPPED_ROWS: usize = 16;
/// Longer than any URL worth opening; bounds what a hostile program can make us hold.
const MAX_URI_LEN: usize = 8 * 1024;

/// A link at a point in the grid, with the cells it covers (`start..=end`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub uri: String,
    pub start: Point,
    pub end: Point,
    /// A program's own hyperlink (OSC 8) rather than a URL found in the text.
    pub explicit: bool,
}

impl Link {
    /// The columns `[first, end)` the link covers on grid line `line`, if any.
    pub fn columns_on(&self, line: Line, cols: usize) -> Option<Range<usize>> {
        if line < self.start.line || line > self.end.line {
            return None;
        }
        let first = if line == self.start.line { self.start.column.0 } else { 0 };
        let end = if line == self.end.line { self.end.column.0 + 1 } else { cols };
        (first < end).then_some(first..end)
    }
}

/// The grid line a search is looking at, copied once for as long as it stays on it.
struct Rows<'a> {
    screen: &'a Screen,
    line: Option<Line>,
    cells: Vec<Cell>,
    extras: RowExtras,
}

impl<'a> Rows<'a> {
    fn new(screen: &'a Screen) -> Self {
        Self { screen, line: None, cells: Vec::new(), extras: RowExtras::default() }
    }

    fn load(&mut self, line: Line) {
        if self.line != Some(line) {
            self.screen.copy_row(line, &mut self.cells, Some(&mut self.extras));
            self.line = Some(line);
        }
    }
}

/// The link at `point`: the program's hyperlink there, else a URL in the text around it.
pub(super) fn link_at(screen: &Screen, point: Point) -> Option<Link> {
    if point.line < screen.topmost_line() || point.line > screen.bottommost_line() {
        return None;
    }
    if point.column.0 >= screen.columns() {
        return None;
    }
    let mut rows = Rows::new(screen);
    hyperlink_at(&mut rows, point).or_else(|| url_at(&mut rows, point))
}

/// The cells around `point` that carry the same OSC 8 hyperlink.
fn hyperlink_at(rows: &mut Rows, point: Point) -> Option<Link> {
    // The shell integration's block marks are hyperlinks to the engine, but nothing to open (and
    // not among a row's hyperlinks): a URL written in a prompt or printed by a command is still
    // found in its text.
    rows.load(point.line);
    let link = rows.extras.hyperlink(point.column.0).cloned()?;
    let cols = rows.screen.columns();
    let budget = cols * MAX_WRAPPED_ROWS;
    let same = |rows: &mut Rows, p: Point| {
        rows.load(p.line);
        rows.extras.hyperlink(p.column.0) == Some(&link)
    };

    let mut start = point;
    for _ in 0..budget {
        let Some(prev) = step_back(rows, start) else { break };
        if !same(rows, prev) {
            break;
        }
        start = prev;
    }
    let mut end = point;
    for _ in 0..budget {
        let Some(next) = step_forward(rows, end) else { break };
        if !same(rows, next) {
            break;
        }
        end = next;
    }
    (link.uri.len() <= MAX_URI_LEN).then(|| Link { uri: link.uri.to_string(), start, end, explicit: true })
}

/// The cell before `point` on the same wrapped line.
fn step_back(rows: &mut Rows, point: Point) -> Option<Point> {
    if point.column.0 > 0 {
        return Some(Point::new(point.line, point.column - 1));
    }
    let above = Line(point.line.0 - 1);
    let last = rows.screen.last_column();
    (above >= rows.screen.topmost_line() && rows.screen.wraps(above)).then_some(Point::new(above, last))
}

/// The cell after `point` on the same wrapped line.
fn step_forward(rows: &mut Rows, point: Point) -> Option<Point> {
    let last = rows.screen.last_column();
    if point.column < last {
        return Some(Point::new(point.line, point.column + 1));
    }
    (point.line < rows.screen.bottommost_line() && rows.screen.wraps(point.line))
        .then_some(Point::new(Line(point.line.0 + 1), Column(0)))
}

/// A URL written in the text of the wrapped line around `point`.
fn url_at(rows: &mut Rows, point: Point) -> Option<Link> {
    let (top, bottom) = (rows.screen.topmost_line(), rows.screen.bottommost_line());
    let mut first = point.line;
    for _ in 0..MAX_WRAPPED_ROWS {
        let above = Line(first.0 - 1);
        if above < top || !rows.screen.wraps(above) {
            break;
        }
        first = above;
    }
    let mut final_line = point.line;
    for _ in 0..MAX_WRAPPED_ROWS {
        if final_line >= bottom || !rows.screen.wraps(final_line) {
            break;
        }
        final_line = Line(final_line.0 + 1);
    }

    // Each character with the cell it's in; wide characters' spacers are skipped.
    let mut chars: Vec<(char, Point)> = Vec::new();
    for line in first.0..=final_line.0 {
        let line = Line(line);
        rows.load(line);
        for (col, cell) in rows.cells.iter().enumerate() {
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            chars.push((cell.c, Point::new(line, Column(col))));
        }
    }
    let text: Vec<char> = chars.iter().map(|(c, _)| *c).collect();
    let at = chars.iter().position(|(_, p)| *p == point)?;
    let range = find_urls(&text).into_iter().find(|range| range.contains(&at))?;
    let uri: String = text[range.clone()].iter().collect();
    (uri.len() <= MAX_URI_LEN).then(|| Link {
        uri,
        start: chars[range.start].1,
        end: chars[range.end - 1].1,
        explicit: false,
    })
}

/// The schemes a URL written as text is recognized by.
const SCHEMES: [&str; 5] = ["https://", "http://", "file://", "mailto:", "ftp://"];

/// The character ranges of the URLs in `text`. A URL starts at a known scheme (not in the
/// middle of a word), runs to whitespace or a character URLs can't contain, and leaves out
/// punctuation that ends a sentence and closing brackets it didn't open.
pub(crate) fn find_urls(text: &[char]) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let starts_word = i == 0 || !text[i - 1].is_alphanumeric();
        let scheme = SCHEMES.iter().find(|scheme| matches_at(text, i, scheme));
        let Some(scheme) = scheme.filter(|_| starts_word) else {
            i += 1;
            continue;
        };
        let body = i + scheme.chars().count();
        let mut end = body;
        while end < text.len() && is_url_char(text[end]) {
            end += 1;
        }
        let end = trim_url_end(&text[i..end]) + i;
        if end > body {
            found.push(i..end);
        }
        i = end.max(i + 1);
    }
    found
}

fn matches_at(text: &[char], at: usize, pattern: &str) -> bool {
    (at..).zip(pattern.chars()).all(|(k, p)| text.get(k).is_some_and(|c| c.to_ascii_lowercase() == p))
}

fn is_url_char(c: char) -> bool {
    !c.is_whitespace()
        && !c.is_control()
        && !matches!(c, '\0' | '<' | '>' | '"' | '`' | '{' | '}' | '^' | '\\' | '|' | '⟨' | '⟩')
}

/// The length of `url` without what ends the sentence around it: trailing `.,:;!?'`, and
/// closing brackets beyond the ones the URL opened (so `(see https://x.org/a_(b))` keeps one).
fn trim_url_end(url: &[char]) -> usize {
    let mut end = url.len();
    loop {
        let Some(&last) = end.checked_sub(1).and_then(|k| url.get(k)) else { return end };
        let unbalanced = |open: char, close: char| {
            last == close && {
                let s = &url[..end];
                s.iter().filter(|&&c| c == close).count() > s.iter().filter(|&&c| c == open).count()
            }
        };
        if matches!(last, '.' | ',' | ':' | ';' | '!' | '?' | '\'')
            || unbalanced('(', ')')
            || unbalanced('[', ']')
        {
            end -= 1;
        } else {
            return end;
        }
    }
}

/// What a ⌘-click on a link may do. Links come from whatever program runs in the terminal
/// (or a file it printed), so only what can't run anything is allowed: web and mail links
/// open in their apps; a `file:` link on this Mac is shown in the Finder rather than opened,
/// since opening an app or a script would run it. Anything else (custom app schemes, which
/// can act on their own) is left alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Openable {
    Url(String),
    Reveal(PathBuf),
}

impl Openable {
    pub fn from_uri(uri: &str) -> Option<Self> {
        let (scheme, rest) = uri.split_once(':')?;
        match scheme.to_ascii_lowercase().as_str() {
            "http" | "https" if rest.starts_with("//") && rest.len() > 2 => Some(Self::Url(uri.into())),
            "mailto" if !rest.is_empty() => Some(Self::Url(uri.into())),
            "file" => {
                let rest = rest.strip_prefix("//")?;
                let (host, path) = rest.find('/').map(|at| rest.split_at(at))?;
                let path = path.split(['?', '#']).next().unwrap_or(path);
                if !procinfo::is_this_host(host) {
                    return None;
                }
                let path = percent_decode(path.as_bytes())?;
                (!path.chars().any(char::is_control)).then(|| Self::Reveal(PathBuf::from(path)))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wormhole_vt::types::{Column, Line, Point};

    use super::{Openable, find_urls, link_at};
    use crate::terminal::testing::TestTerm;

    fn urls(text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        find_urls(&chars).into_iter().map(|range| chars[range].iter().collect()).collect()
    }

    #[test]
    fn urls_end_where_the_sentence_resumes() {
        assert_eq!(urls("see https://example.com/a?b=1."), ["https://example.com/a?b=1"]);
        assert_eq!(
            urls("(https://en.wikipedia.org/wiki/Rust_(language))"),
            ["https://en.wikipedia.org/wiki/Rust_(language)"]
        );
        assert_eq!(urls("<http://a.io/x> and 'https://b.io'"), ["http://a.io/x", "https://b.io"]);
        assert_eq!(
            urls("mailto:me@example.com, file:///tmp/a%20b"),
            ["mailto:me@example.com", "file:///tmp/a%20b"]
        );
    }

    #[test]
    fn a_scheme_inside_a_word_or_alone_is_no_url() {
        assert!(urls("xhttps://example.com").is_empty());
        assert!(urls("https:// nothing").is_empty());
        assert!(urls("plain text").is_empty());
    }

    #[test]
    fn only_harmless_links_open() {
        let url = |s: &str| Some(Openable::Url(s.into()));
        assert_eq!(Openable::from_uri("https://example.com"), url("https://example.com"));
        assert_eq!(Openable::from_uri("HTTP://example.com"), url("HTTP://example.com"));
        assert_eq!(Openable::from_uri("mailto:a@b.c"), url("mailto:a@b.c"));
        assert_eq!(
            Openable::from_uri("file:///Users/me/a%20b.txt#L3"),
            Some(Openable::Reveal(PathBuf::from("/Users/me/a b.txt")))
        );
        assert_eq!(Openable::from_uri("file://localhost/tmp"), Some(Openable::Reveal(PathBuf::from("/tmp"))));
        for refused in [
            "file://some-other-host.example/etc/passwd",
            "file:///bad%zz",
            "javascript:alert(1)",
            "x-apple-systempreferences:com.apple",
            "ssh://host",
            "https:",
        ] {
            assert_eq!(Openable::from_uri(refused), None, "{refused}");
        }
    }

    fn term(cols: u16) -> TestTerm {
        TestTerm::new(cols, 4, 100)
    }

    fn feed(term: &mut TestTerm, bytes: &[u8]) {
        term.feed(bytes);
    }

    fn at(line: i32, col: usize) -> Point {
        Point::new(Line(line), Column(col))
    }

    #[test]
    fn a_hyperlink_covers_its_cells_across_a_wrap() {
        let mut term = term(10);
        feed(&mut term, b"ab\x1b]8;;https://example.com\x1b\\0123456789xyz\x1b]8;;\x1b\\ c");
        let link = link_at(term.screen(), at(1, 1)).expect("the hyperlink");
        assert_eq!(link.uri, "https://example.com");
        assert!(link.explicit);
        assert_eq!((link.start, link.end), (at(0, 2), at(1, 4)));
        assert_eq!(link.columns_on(Line(0), 10), Some(2..10));
        assert_eq!(link.columns_on(Line(1), 10), Some(0..5));
        assert_eq!(link_at(term.screen(), at(0, 1)), None, "before the link");
        assert_eq!(link_at(term.screen(), at(1, 6)), None, "after it");
    }

    #[test]
    fn a_prompt_mark_is_no_link_but_a_url_in_the_prompt_is() {
        // The engine keeps the shell's marks on the rows, not as hyperlinks.
        let mut term = TestTerm::new(40, 4, 100);
        feed(&mut term, b"\x1b]133;A\x07$ see https://a.io \x1b]133;B\x07");
        assert_eq!(link_at(term.screen(), at(0, 0)), None, "the prompt itself");
        let link = link_at(term.screen(), at(0, 10)).expect("the URL");
        assert_eq!((link.uri.as_str(), link.explicit), ("https://a.io", false));
    }

    #[test]
    fn a_url_in_the_text_is_found_across_a_wrap() {
        let mut term = term(12);
        feed(&mut term, b"go https://x.io/abcdef now");
        let link = link_at(term.screen(), at(1, 2)).expect("the URL");
        assert_eq!(link.uri, "https://x.io/abcdef");
        assert!(!link.explicit);
        assert_eq!((link.start, link.end), (at(0, 3), at(1, 9)));
        assert_eq!(link_at(term.screen(), at(0, 0)), None);
        assert_eq!(link_at(term.screen(), at(3, 0)), None, "an empty row");
        assert_eq!(link_at(term.screen(), at(9, 0)), None, "off the grid");
    }
}
