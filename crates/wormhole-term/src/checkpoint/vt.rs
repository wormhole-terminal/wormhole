//! A terminal's primary screen as VT bytes, and the filter that lets such bytes back in.
//!
//! The checkpoint's screen is **content only**: the history and the visible rows of the primary screen as
//! text with its colors and attributes (SGR), one logical line at a time (soft-wrapped rows joined, so a
//! restore wraps them to its own width), and nothing else. No modes (mouse tracking, bracketed paste,
//! application cursor keys, …), no alternate screen, no tab stops, no title, no pictures, no cursor
//! position: those belong to a program that is no longer running, and a replay that set them would leave
//! the new shell in a state it never asked for.
//!
//! The engine's own repaint (`Terminal::repaint`) is not used: it starts with a hard reset, sets every
//! mode, and writes the alternate screen, and its bytes cannot be filtered back to content without
//! parsing them. The content is read from the grid through `Screen::copy_row`, as the saved text is.
//!
//! [`safe_replay`] is the reader's half: whatever a file holds (it was sealed, so it is what a Wormhole
//! wrote, but a bug there must not become a terminal that obeys its history), only printable text, tabs,
//! line ends and SGR sequences come out.

use wormhole_vt::{Cell, Color, Flags, Line, Modes, NamedColor, RowExtras};

use super::replay::is_divider_line;
use crate::screen::Screen;

/// How much of a terminal a checkpoint keeps.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The most lines, counted from the bottom of the screen.
    pub max_lines: usize,
    /// The most bytes of VT; the oldest lines go first, whole (a cut is always at a line start, so no escape
    /// sequence is ever split).
    pub max_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        // What a new host can be seeded with: the checkpoint is drawn through a `Spawn`.
        Self { max_lines: 5_000, max_bytes: wormhole_wire::SEED_MAX }
    }
}

/// How the screen is reached: under its lock, for as long as the closure runs. The checkpoint thread reads the
/// screen in slices of [`SLICE_ROWS`] rows, one look each, so the PTY thread that parses under the same lock is
/// never kept waiting for more than a slice.
pub(crate) trait ScreenAccess {
    fn with<R>(&self, read: impl FnOnce(&Screen) -> R) -> R;
}

/// A screen that is not shared: its own access.
impl ScreenAccess for Screen {
    fn with<R>(&self, read: impl FnOnce(&Screen) -> R) -> R {
        read(self)
    }
}

/// Rows copied out in one look at the screen.
pub(crate) const SLICE_ROWS: usize = 256;

/// The attributes a cell can carry that a replay sets: SGR's.
const ATTRIBUTES: Flags = Flags::BOLD
    .union(Flags::DIM)
    .union(Flags::ITALIC)
    .union(Flags::ALL_UNDERLINES)
    .union(Flags::BLINK)
    .union(Flags::INVERSE)
    .union(Flags::HIDDEN)
    .union(Flags::STRIKEOUT)
    .union(Flags::OVERLINE);

/// The look of a run of cells.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Pen {
    fg: Color,
    bg: Color,
    flags: Flags,
}

impl Pen {
    const DEFAULT: Pen = Pen {
        fg: Color::Named(NamedColor::Foreground),
        bg: Color::Named(NamedColor::Background),
        flags: Flags::empty(),
    };

    fn of(cell: &Cell) -> Self {
        Self { fg: cell.fg, bg: cell.bg, flags: cell.flags.intersection(ATTRIBUTES) }
    }

    fn is_default(&self) -> bool {
        *self == Self::DEFAULT
    }
}

fn push_color(out: &mut Vec<u8>, color: Color, base: u32) {
    match color {
        Color::Named(NamedColor::Foreground | NamedColor::Background | NamedColor::Cursor) => {}
        Color::Named(named) => {
            let n = named as u32;
            // The sixteen: 30–37 and 90–97 (40–47, 100–107 for a background).
            let code = if n < 8 { base + n } else { base + 60 + n - 8 };
            out.extend_from_slice(format!(";{code}").as_bytes());
        }
        Color::Indexed(i) => out.extend_from_slice(format!(";{};5;{i}", base + 8).as_bytes()),
        Color::Rgb(rgb) => {
            out.extend_from_slice(format!(";{};2;{};{};{}", base + 8, rgb.r, rgb.g, rgb.b).as_bytes());
        }
    }
}

/// `ESC[0;…m` for `pen`: a reset first, so a run never depends on the one before it.
fn push_pen(out: &mut Vec<u8>, pen: Pen) {
    out.extend_from_slice(b"\x1b[0");
    for (flag, code) in [
        (Flags::BOLD, "1"),
        (Flags::DIM, "2"),
        (Flags::ITALIC, "3"),
        (Flags::UNDERLINE, "4"),
        (Flags::DOUBLE_UNDERLINE, "21"),
        (Flags::UNDERCURL, "4:3"),
        (Flags::DOTTED_UNDERLINE, "4:4"),
        (Flags::DASHED_UNDERLINE, "4:5"),
        (Flags::BLINK, "5"),
        (Flags::INVERSE, "7"),
        (Flags::HIDDEN, "8"),
        (Flags::STRIKEOUT, "9"),
        (Flags::OVERLINE, "53"),
    ] {
        if pen.flags.contains(flag) {
            out.push(b';');
            out.extend_from_slice(code.as_bytes());
        }
    }
    push_color(out, pen.fg, 30);
    push_color(out, pen.bg, 40);
    out.push(b'm');
}

/// A character that is safe as text: no control characters, and nothing the cell grid keeps as a placeholder.
fn glyph(cell: &Cell) -> char {
    if cell.c.is_control() || cell.c == crate::images::PLACEHOLDER { ' ' } else { cell.c }
}

/// One row copied out of the grid: its cells and the zero-width characters some of them carry.
struct RowCopy {
    cells: Vec<Cell>,
    marks: Vec<(usize, Vec<char>)>,
}

/// The primary screen as VT bytes, newest [`Limits::max_lines`] lines and at most [`Limits::max_bytes`]; `None`
/// while the alternate screen shows (a full-screen program: the engine keeps the real screen out of reach, so
/// the checkpoint before it is the one to keep), and `None` for a screen that is blank.
///
/// The screen is looked at in slices ([`SLICE_ROWS`] rows a look) and encoded outside the lock. A program that
/// prints while it is read may scroll the grid between two looks; the rows are then followed by how far the
/// history grew, and if the history is full and drops lines a row can be missed or repeated: the next checkpoint
/// (a quiet moment later) is whole.
pub(crate) fn primary_screen<A: ScreenAccess>(access: &A, limits: Limits) -> Option<Vec<u8>> {
    let (alt, top, bottom, history) = access.with(|s| {
        (s.mode().contains(Modes::ALT_SCREEN), s.topmost_line().0, s.bottommost_line().0, s.history_size())
    });
    if alt {
        return None;
    }
    let start = (bottom - limits.max_lines.min(i32::MAX as usize) as i32 + 1).max(top);
    let mut rows: Vec<RowCopy> = Vec::with_capacity((bottom - start + 1).max(0) as usize);
    let mut at = start;
    while at <= bottom {
        let end = (at + SLICE_ROWS as i32 - 1).min(bottom);
        access.with(|s| {
            // Lines that scrolled into the history since the first look moved up by as much.
            let shift = s.history_size() as i32 - history as i32;
            let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
            for line in at..=end {
                let line = line - shift;
                if line < s.topmost_line().0 || line > s.bottommost_line().0 {
                    rows.push(RowCopy { cells: Vec::new(), marks: Vec::new() });
                    continue;
                }
                s.copy_row(Line(line), &mut cells, Some(&mut extras));
                let marks = (0..cells.len())
                    .filter_map(|col| {
                        let marks = extras.zerowidth(col);
                        (!marks.is_empty()).then(|| (col, marks.to_vec()))
                    })
                    .collect();
                rows.push(RowCopy { cells: std::mem::take(&mut cells), marks });
            }
        });
        at = end + 1;
    }
    encode(&rows, limits)
}

/// VT for the copied rows (see the module).
fn encode(rows: &[RowCopy], limits: Limits) -> Option<Vec<u8>> {
    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut line = Vec::new();
    let mut plain = String::new();
    // Where `line` ends after its last cell that shows something: what comes after is blanks to trim.
    let mut shown = 0usize;
    let mut pen = Pen::DEFAULT;
    // Whether every character of the line is dim and italic, which is how a restore's divider is drawn.
    let mut divider_look = true;
    let mut finish =
        |line: &mut Vec<u8>, plain: &mut String, shown: &mut usize, pen: &mut Pen, look: &mut bool| {
            line.truncate(*shown);
            // Dividers of earlier restores do not pile up across restarts.
            if !is_divider_line(plain, *look) {
                let mut done = std::mem::take(line);
                done.extend_from_slice(b"\x1b[0m\r\n");
                lines.push(done);
            } else {
                line.clear();
            }
            plain.clear();
            *shown = 0;
            *pen = Pen::DEFAULT;
            *look = true;
        };
    for row in rows {
        for (col, cell) in row.cells.iter().enumerate() {
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            let marks: &[char] =
                row.marks.iter().find(|(c, _)| *c == col).map_or(&[], |(_, marks)| marks.as_slice());
            let this = Pen::of(cell);
            let blank = glyph(cell) == ' ' && marks.is_empty();
            if this != pen {
                if this.is_default() {
                    line.extend_from_slice(b"\x1b[0m");
                } else {
                    push_pen(&mut line, this);
                }
                pen = this;
            }
            let c = glyph(cell);
            let mut buf = [0u8; 4];
            line.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            plain.push(c);
            for &mark in marks {
                let safe = if mark.is_control() { ' ' } else { mark };
                line.extend_from_slice(safe.encode_utf8(&mut buf).as_bytes());
                plain.push(safe);
            }
            if !blank {
                divider_look &= cell.flags.contains(Flags::DIM | Flags::ITALIC);
            }
            // A blank in the default look shows nothing; anything else does (a background, an underline).
            if !(blank && this.is_default()) {
                shown = line.len();
            }
        }
        let wrapped = row.cells.last().is_some_and(|last| last.flags.contains(Flags::WRAPLINE));
        if !wrapped {
            finish(&mut line, &mut plain, &mut shown, &mut pen, &mut divider_look);
        }
    }
    if !line.is_empty() {
        finish(&mut line, &mut plain, &mut shown, &mut pen, &mut divider_look);
    }
    // The blank rows under the last line of output are not history.
    while lines.last().is_some_and(|l| l.as_slice() == b"\x1b[0m\r\n") {
        lines.pop();
    }
    if lines.is_empty() {
        return None;
    }
    // The oldest lines go first when the bytes are over the limit; lines go whole, so the cut is at a line start
    // and no escape sequence is split (every line starts from the default look).
    let mut total: usize = lines.iter().map(Vec::len).sum();
    let mut skip = 0;
    while total > limits.max_bytes && skip + 1 < lines.len() {
        total -= lines[skip].len();
        skip += 1;
    }
    let mut out = Vec::with_capacity(total);
    for line in &lines[skip..] {
        out.extend_from_slice(line);
    }
    Some(out)
}

/// The longest SGR sequence let through.
const SGR_MAX: usize = 64;

/// `bytes` reduced to what a replay may obey: printable characters, tabs, line ends and SGR sequences
/// (`ESC [ digits ; : m`, at most [`SGR_MAX`] bytes). Everything else, an escape that starts anything
/// but that included, is dropped. Invalid UTF-8 is dropped too.
pub fn safe_replay(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\u{1b}' => {
                // Only `ESC [ <digits ; :> m`.
                let mut j = i + 1;
                if chars.get(j) == Some(&'[') {
                    j += 1;
                    let params_start = j;
                    while j < chars.len()
                        && j - params_start <= SGR_MAX
                        && matches!(chars[j], '0'..='9' | ';' | ':')
                    {
                        j += 1;
                    }
                    if chars.get(j) == Some(&'m') && j - params_start <= SGR_MAX {
                        out.extend_from_slice(b"\x1b[");
                        out.extend(chars[params_start..=j].iter().map(|c| *c as u8));
                        i = j + 1;
                        continue;
                    }
                }
                // Not an SGR: the escape is dropped, and what follows is judged on its own.
                i += 1;
            }
            '\n' | '\r' | '\t' => {
                out.push(c as u8);
                i += 1;
            }
            c if c.is_control() || c == '\u{fffd}' => i += 1,
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestTerm;

    fn vt_of(bytes: &[u8]) -> Option<Vec<u8>> {
        let mut term = TestTerm::new(20, 5, 100);
        term.feed(bytes);
        primary_screen(term.screen(), Limits::default())
    }

    fn text(vt: &[u8]) -> String {
        String::from_utf8_lossy(vt).into_owned()
    }

    #[test]
    fn colors_and_attributes_come_back_and_nothing_else() {
        let vt =
            vt_of(b"\x1b[1;31mred\x1b[0m plain \x1b[38;2;1;2;3mrgb\x1b[0m\r\n\x1b[4:3;48;5;200mcurl\x1b[0m")
                .unwrap();
        let t = text(&vt);
        assert!(t.contains("\x1b[0;1;31mred"), "{t:?}");
        assert!(t.contains("\x1b[0m plain "), "{t:?}");
        assert!(t.contains("\x1b[0;38;2;1;2;3mrgb"), "{t:?}");
        assert!(t.contains("\x1b[0;4:3;48;5;200mcurl"), "{t:?}");
        // Only SGR: nothing of a mode, a screen switch, a title.
        assert_eq!(safe_replay(&vt), vt, "the writer's own output passes the filter unchanged");
    }

    #[test]
    fn modes_the_alternate_screen_and_titles_are_not_in_it() {
        let vt = vt_of(b"\x1b[?1000h\x1b[?2004h\x1b[?1h\x1b]0;a title\x07\x1b[2 qhello\r\nworld").unwrap();
        let t = text(&vt);
        assert_eq!(t.replace("\x1b[0m", ""), "hello\r\nworld\r\n", "{t:?}");
        assert!(!t.contains("?1000") && !t.contains("2004") && !t.contains("a title"));
    }

    #[test]
    fn a_full_screen_program_has_no_primary_screen_to_save() {
        assert_eq!(vt_of(b"before\r\n\x1b[?1049hfull screen"), None);
    }

    #[test]
    fn soft_wrapped_rows_are_one_line_and_trailing_blanks_go() {
        let vt = vt_of(b"aaaaaaaaaaaaaaaaaaaaBBBBB\r\nnext   ").unwrap();
        assert_eq!(text(&vt).replace("\x1b[0m", ""), "aaaaaaaaaaaaaaaaaaaaBBBBB\r\nnext\r\n");
    }

    #[test]
    fn blank_rows_below_the_output_are_not_history_and_a_blank_screen_is_none() {
        assert_eq!(text(&vt_of(b"one\r\n\r\n\r\n").unwrap()).replace("\x1b[0m", ""), "one\r\n");
        assert_eq!(vt_of(b""), None);
    }

    #[test]
    fn a_background_colour_on_a_blank_is_kept() {
        let t = text(&vt_of(b"\x1b[44m   \x1b[0mx").unwrap());
        assert!(t.contains("\x1b[0;44m   "), "{t:?}");
    }

    #[test]
    fn a_restored_divider_does_not_pile_up_whatever_its_words() {
        for words in ["restored 2026-11-02 14:03", "wiederhergestellt 02.11.2026 14:03"] {
            let divider = super::super::replay::divider(words);
            let t = text(&vt_of(format!("old\r\n{divider}new\r\n").as_bytes()).unwrap());
            assert!(!t.contains("────") && !t.contains(words), "{t:?}");
            assert!(t.contains("old") && t.contains("new"));
        }
        // A rule someone drew themselves is theirs: it is not dim and italic, so it stays.
        let t = text(&vt_of(b"a\r\n\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80 mine \xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\r\n").unwrap());
        assert!(t.contains("mine"), "{t:?}");
    }

    /// The screen is read in slices, each under one look at the lock, and the lock is never held for the whole
    /// of a long history: the PTY thread that parses under it waits for a slice at the most.
    #[test]
    fn a_long_history_is_read_in_slices_and_no_look_is_long() {
        use std::cell::RefCell;
        use std::time::{Duration, Instant};

        struct Timed<'a> {
            screen: &'a Screen,
            holds: RefCell<Vec<Duration>>,
        }
        impl ScreenAccess for Timed<'_> {
            fn with<R>(&self, read: impl FnOnce(&Screen) -> R) -> R {
                let started = Instant::now();
                let out = read(self.screen);
                self.holds.borrow_mut().push(started.elapsed());
                out
            }
        }
        let mut term = TestTerm::new(80, 24, 6_000);
        for i in 0..5_000 {
            term.feed(
                format!("\x1b[3{}mline {i} of the long history with some text on it\x1b[0m\r\n", i % 8)
                    .as_bytes(),
            );
        }
        let timed = Timed { screen: term.screen(), holds: RefCell::new(Vec::new()) };
        let vt =
            primary_screen(&timed, Limits { max_lines: 5_000, max_bytes: usize::MAX }).expect("a screen");
        let holds = timed.holds.borrow();
        assert!(holds.len() >= 5_000 / SLICE_ROWS, "{} looks", holds.len());
        let longest = holds.iter().max().copied().unwrap();
        assert!(longest < Duration::from_millis(25), "one look held the lock for {longest:?}");
        let t = text(&vt);
        assert!(
            t.contains("line 4999 ") && t.contains("line 0 ") || t.contains("line 4999 "),
            "the newest line is in it"
        );
        // The same bytes as reading the screen in one look.
        assert_eq!(
            Some(vt),
            primary_screen(term.screen(), Limits { max_lines: 5_000, max_bytes: usize::MAX })
        );
    }

    /// The oldest lines go whole when the bytes are over the limit: the cut is at a line start, so no escape
    /// sequence is ever split, and what is left passes the replay filter unchanged.
    #[test]
    fn the_byte_cap_cuts_at_a_line_start_and_never_inside_an_escape() {
        let mut term = TestTerm::new(40, 10, 1_000);
        for i in 0..400 {
            term.feed(format!("\x1b[1;3{}mline {i:03}\x1b[0m\r\n", i % 8).as_bytes());
        }
        let vt = primary_screen(term.screen(), Limits { max_lines: 1_000, max_bytes: 700 }).unwrap();
        assert!(vt.len() <= 700 + 64, "{}", vt.len());
        let t = text(&vt);
        assert!(t.starts_with("\x1b[0;1;3") && t.contains("line 399"), "{:?}", &t[..40.min(t.len())]);
        assert_eq!(safe_replay(&vt), vt, "no sequence was cut");
    }

    #[test]
    fn the_newest_lines_stay_when_a_limit_is_hit() {
        let mut term = TestTerm::new(20, 5, 100);
        for i in 0..30 {
            term.feed(format!("line{i:02}\r\n").as_bytes());
        }
        let few = primary_screen(term.screen(), Limits { max_lines: 4, max_bytes: usize::MAX }).unwrap();
        assert!(text(&few).contains("line29") && !text(&few).contains("line20"));
        let small = primary_screen(term.screen(), Limits { max_lines: 100, max_bytes: 40 }).unwrap();
        assert!(
            small.len() <= 40 + 12 && text(&small).contains("line29") && !text(&small).contains("line00")
        );
    }

    #[test]
    fn the_filter_lets_only_text_and_sgr_through() {
        let hostile = b"ok\x1b[6n\x1b]52;c;?\x07\x1b]0;t\x07\x1bP1$r\x1b\\\x1b[?1000h\x1b[31mred\x1b[0m\x07\x00\x7f\xc2\x9b\r\n\tend\x1b[";
        let safe = safe_replay(hostile);
        assert_eq!(text(&safe), "ok[6n]52;c;?]0;tP1$r\\[?1000h\x1b[31mred\x1b[0m\r\n\tend[");
        // A very long parameter run is not an SGR.
        let long = format!("\x1b[{}m", "1;".repeat(100));
        assert!(!text(&safe_replay(long.as_bytes())).contains('\x1b'));
        // Invalid UTF-8 is dropped, not passed.
        assert_eq!(safe_replay(&[b'a', 0xff, 0xfe, b'b']), b"ab");
    }
}
