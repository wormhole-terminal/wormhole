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

use crate::restore::is_marker;
use crate::screen::Screen;

/// How much of a terminal a checkpoint keeps.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The most lines, counted from the bottom of the screen.
    pub max_lines: usize,
    /// The most bytes of VT; the oldest lines go first.
    pub max_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_lines: 5_000, max_bytes: 4 * 1024 * 1024 }
    }
}

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

/// The primary screen of `screen` as VT bytes, newest [`Limits::max_lines`] lines and at most
/// [`Limits::max_bytes`]; `None` while the alternate screen shows (a full-screen program: the engine keeps the
/// real screen out of reach, so the checkpoint before it is the one to keep), and `None` for a screen that is
/// blank. The caller holds the screen's lock for as long as this takes.
pub(crate) fn primary_screen(screen: &Screen, limits: Limits) -> Option<Vec<u8>> {
    if screen.mode().contains(Modes::ALT_SCREEN) {
        return None;
    }
    let top = screen.topmost_line().0;
    let bottom = screen.bottommost_line().0;
    let start = (bottom - limits.max_lines.min(i32::MAX as usize) as i32 + 1).max(top);

    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut line = Vec::new();
    let mut plain = String::new();
    // Where `line` ends after its last cell that shows something: what comes after is blanks to trim.
    let mut shown = 0usize;
    let mut pen = Pen::DEFAULT;
    let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
    let mut finish = |line: &mut Vec<u8>, plain: &mut String, shown: &mut usize, pen: &mut Pen| {
        line.truncate(*shown);
        // Restore markers do not pile up across restarts.
        if !is_marker(&plain) {
            let mut done = std::mem::take(line);
            done.extend_from_slice(b"\x1b[0m\r\n");
            lines.push(done);
        } else {
            line.clear();
        }
        plain.clear();
        *shown = 0;
        *pen = Pen::DEFAULT;
    };
    for row in start..=bottom {
        screen.copy_row(Line(row), &mut cells, Some(&mut extras));
        for (col, cell) in cells.iter().enumerate() {
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            let this = Pen::of(cell);
            let blank = glyph(cell) == ' ' && extras.zerowidth(col).is_empty();
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
            for &mark in extras.zerowidth(col) {
                let safe = if mark.is_control() { ' ' } else { mark };
                line.extend_from_slice(safe.encode_utf8(&mut buf).as_bytes());
                plain.push(safe);
            }
            // A blank in the default look shows nothing; anything else does (a background, an underline).
            if !(blank && this.is_default()) {
                shown = line.len();
            }
        }
        let wrapped = cells.last().is_some_and(|last| last.flags.contains(Flags::WRAPLINE));
        if !wrapped {
            finish(&mut line, &mut plain, &mut shown, &mut pen);
        }
    }
    if !line.is_empty() {
        finish(&mut line, &mut plain, &mut shown, &mut pen);
    }
    // The blank rows under the last line of output are not history.
    while lines.last().is_some_and(|l| l.as_slice() == b"\x1b[0m\r\n") {
        lines.pop();
    }
    if lines.is_empty() {
        return None;
    }
    // The oldest lines go first when the bytes are over the limit.
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
    fn a_restored_marker_does_not_pile_up() {
        let t = text(
            &vt_of(format!("old\r\n{}\r\nnew\r\n", crate::restore::RESTORED_MARKER).as_bytes()).unwrap(),
        );
        assert!(!t.contains("restored"), "{t:?}");
        assert!(t.contains("old") && t.contains("new"));
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
