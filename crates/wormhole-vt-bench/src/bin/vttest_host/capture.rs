//! One screen as the text a golden file holds: the rows, then what the cells carry beside their
//! characters (attributes and colors, as runs), then the cursor.
//!
//! Built to be read and diffed by a person: a row is its text with trailing blanks cut, an attribute
//! is named, a color is its palette number. A character set maps characters before they reach the
//! screen, so the line-drawing glyphs and the national replacements are in the text already.

use std::fmt::Write;

use wormhole_vt::{Cell, Color, Column, Flags, Line, Modes, NamedColor, Point, Terminal};

/// The attributes a vttest screen shows, in the order a run names them. (Wide-character and wrap
/// flags describe the row's layout, not the cell's look: the text shows them.)
const ATTRS: [(Flags, &str); 8] = [
    (Flags::BOLD, "bold"),
    (Flags::DIM, "dim"),
    (Flags::ITALIC, "italic"),
    (Flags::ALL_UNDERLINES, "underline"),
    (Flags::BLINK, "blink"),
    (Flags::INVERSE, "inverse"),
    (Flags::HIDDEN, "hidden"),
    (Flags::STRIKEOUT, "strikeout"),
];

/// A cell's look apart from its character: what runs are made of.
#[derive(Clone, PartialEq, Eq)]
struct Look {
    attrs: Vec<&'static str>,
    fg: Option<String>,
    bg: Option<String>,
}

impl Look {
    fn of(cell: &Cell) -> Look {
        let attrs = ATTRS.iter().filter(|(flag, _)| cell.flags.intersects(*flag)).map(|(_, n)| *n).collect();
        Look { attrs, fg: color(cell.fg), bg: color(cell.bg) }
    }

    fn is_plain(&self) -> bool {
        self.attrs.is_empty() && self.fg.is_none() && self.bg.is_none()
    }

    fn words(&self) -> String {
        let mut words: Vec<String> = self.attrs.iter().map(|a| (*a).to_string()).collect();
        words.extend(self.fg.as_ref().map(|c| format!("fg={c}")));
        words.extend(self.bg.as_ref().map(|c| format!("bg={c}")));
        words.join(" ")
    }
}

/// The color's palette number or `#rrggbb`; `None` for the defaults.
fn color(color: Color) -> Option<String> {
    match color {
        Color::Named(NamedColor::Foreground | NamedColor::Background | NamedColor::Cursor) => None,
        Color::Named(named) => Some((named as u16).to_string()),
        Color::Indexed(n) => Some(n.to_string()),
        Color::Rgb(rgb) => Some(format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)),
    }
}

/// The cursor: its row and column (counted from 1, as vttest and the manuals do), whether it is hidden,
/// and the style a program chose with DECSCUSR (nothing when it left the host's own).
fn cursor(term: &Terminal) -> String {
    let cursor = term.cursor();
    let Point { line: Line(row), column: Column(col) } = cursor.point;
    let mut text = format!("row {} col {}", row.max(0) + 1, col + 1);
    if !cursor.visible {
        text.push_str(" (hidden)");
    }
    if cursor.shape.is_some() || cursor.blinking.is_some() {
        let shape = cursor.shape.map_or("default".to_string(), |shape| format!("{shape:?}").to_lowercase());
        let blink = match cursor.blinking {
            Some(true) => " blinking",
            Some(false) => " steady",
            None => "",
        };
        text.push_str(&format!(" style {shape}{blink}"));
    }
    text
}

/// The modes that differ from where vttest leaves a terminal when it starts (its own defaults on a
/// terminal at power on, with 80/132 switching allowed and auto repeat off): `+NAME` set, `-NAME`
/// reset.
fn modes(term: &Terminal) -> String {
    let mut fresh = Terminal::new(2, 1);
    fresh.feed(b"\x1b[?40h\x1b[?8l");
    let (now, start) = (term.modes(), fresh.modes());
    let named = |modes: Modes, sign: char| -> Vec<String> {
        modes.iter_names().map(|(name, _)| format!("{sign}{name}")).collect()
    };
    let mut changed = named(now.difference(start), '+');
    changed.extend(named(start.difference(now), '-'));
    if changed.is_empty() { "default".to_string() } else { changed.join(" ") }
}

/// Where a screen came from, for its header.
pub struct Meta<'a> {
    pub version: &'a str,
    /// The top-level menu's number and name.
    pub menu: &'a str,
    /// The path of choices that led to it, with the names vttest gives them.
    pub path: &'a str,
    /// Set when the screen is not at a `Push <RETURN>` (vttest was silent, waiting for something).
    pub state: Option<&'a str>,
}

/// The golden text of the terminal's screen.
pub fn render(term: &Terminal, meta: &Meta) -> String {
    let (cols, rows) = (term.cols(), term.rows());
    let mut out = String::new();
    let _ = writeln!(out, "vttest {} | menu {} | screen @@", meta.version, meta.menu);
    if let Some(state) = meta.state {
        let _ = writeln!(out, "state: {state}");
    }
    let _ = writeln!(out, "path: {}", meta.path);
    let _ = writeln!(out, "size: {cols}x{rows}  cursor: {}", cursor(term));
    let _ = writeln!(out, "modes: {}", modes(term));
    out.push_str("--- text\n");
    for row in 0..rows {
        let text = term.row_text(Line(row as i32), true);
        let _ = writeln!(out, "{:02}|{text}", row + 1);
    }
    let mut look_lines = String::new();
    for row in 0..rows {
        let mut runs: Vec<String> = Vec::new();
        let mut col = 0;
        while col < cols {
            let Some(cell) = term.cell(Point::new(Line(row as i32), Column(col))) else { break };
            let look = Look::of(&cell);
            let mut end = col;
            while end + 1 < cols {
                let next = term.cell(Point::new(Line(row as i32), Column(end + 1)));
                if next.as_ref().map(Look::of).as_ref() == Some(&look) {
                    end += 1;
                } else {
                    break;
                }
            }
            if !look.is_plain() {
                let range =
                    if end == col { format!("{}", col + 1) } else { format!("{}-{}", col + 1, end + 1) };
                runs.push(format!("{range} {}", look.words()));
            }
            col = end + 1;
        }
        if !runs.is_empty() {
            let _ = writeln!(look_lines, "{:02}| {}", row + 1, runs.join("; "));
        }
    }
    if look_lines.is_empty() {
        out.push_str("--- attributes\n(none)\n");
    } else {
        out.push_str("--- attributes (columns from 1)\n");
        out.push_str(&look_lines);
    }
    out
}

#[cfg(test)]
mod tests {
    use wormhole_vt::Terminal;

    use super::{Meta, render};

    fn shown(bytes: &[u8]) -> String {
        let mut term = Terminal::new(20, 3);
        // As vttest's own setup leaves a terminal.
        term.feed(b"\x1b[?40h\x1b[?8l");
        term.feed(bytes);
        let meta = Meta { version: "1", menu: "01-x", path: "1 A", state: None };
        render(&term, &meta)
    }

    #[test]
    fn a_screen_is_its_rows_then_the_looks_of_the_cells_that_have_one() {
        let text = shown(b"plain \x1b[1;4mboth\x1b[0m \x1b[31;44mred\x1b[0m");
        assert!(text.contains("01|plain both red\n"), "{text}");
        // Columns count from 1: "both" is 7 to 10, "red" 12 to 14.
        assert!(text.contains("01| 7-10 bold underline; 12-14 fg=1 bg=4\n"), "{text}");
    }

    #[test]
    fn the_modes_that_are_not_the_defaults_are_named() {
        let text = shown(b"\x1b[?5h\x1b[?7l\x1b[4h");
        assert!(text.contains("modes: +INSERT +REVERSE_VIDEO -AUTOWRAP\n"), "{text}");
    }

    #[test]
    fn a_cursor_style_a_program_chose_is_named() {
        let text = shown(b"\x1b[3 q");
        assert!(text.contains("cursor: row 1 col 1 style underline blinking\n"), "{text}");
        assert!(shown(b"").contains("cursor: row 1 col 1\n"));
    }

    #[test]
    fn a_screen_without_looks_says_so_and_a_hidden_cursor_is_named() {
        let mut term = Terminal::new(20, 3);
        term.feed(b"\x1b[?40h\x1b[?8lx\x1b[2;5H\x1b[?25l");
        let meta = Meta { version: "1", menu: "01-x", path: "1 A", state: Some("waiting") };
        let text = render(&term, &meta);
        assert!(
            text.contains("state: waiting\n") && text.contains("cursor: row 2 col 5 (hidden)\n"),
            "{text}"
        );
        assert!(text.ends_with("--- attributes\n(none)\n"), "{text}");
    }
}
