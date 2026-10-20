//! `ansi_to_html`: a program's recorded output (colours, bold, underlines, links) as an HTML page.
//!
//! ```sh
//! ls --color=always -l | cargo run -q -p wormhole-vt --example ansi_to_html > ls.html
//! cargo run -q -p wormhole-vt --example ansi_to_html -- --cols 100 session.raw > session.html
//! ```
//!
//! The output is not scanned for sequences here: it is fed to a terminal, as a real one would read
//! it, and the page is made from the screen and the history that came out. That is what makes it a
//! small tool and not a regex: cursor movement, carriage returns, erases, wrapped lines, wide
//! characters, true colours, palette changes (OSC 4, 10, 11) and OSC 8 links all come out as the
//! program meant them. A program that redraws the screen (an editor, `top`) leaves its last screen.
//!
//! Options: `--cols N` is the width lines wrap at (default 100), `FILE` the recording (default:
//! standard input).
//!
//! Link targets are text the program chose, so only `http`, `https` and `mailto` ones become
//! links (the rule of the security chapter of the guide); everything is HTML-escaped.
#![allow(clippy::print_stdout)] // a command line tool: the page is the output

use std::fmt::Write as _;
use std::io::{self, Read, Write};

use wormhole_vt::{Cell, Color, Colors, Flags, Hyperlink, Line, NamedColor, Rgb, RowExtras, Terminal};

/// What the page shows where the program did not choose a colour (the dark theme most terminals
/// start with). A real tool would take these from the user.
const DEFAULT_FG: Rgb = Rgb { r: 0xd4, g: 0xd4, b: 0xd4 };
const DEFAULT_BG: Rgb = Rgb { r: 0x1e, g: 0x1e, b: 0x1e };

/// The sixteen ANSI colours (the "Tango" set).
const ANSI: [Rgb; 16] = [
    rgb(0x2e, 0x34, 0x36),
    rgb(0xcc, 0x00, 0x00),
    rgb(0x4e, 0x9a, 0x06),
    rgb(0xc4, 0xa0, 0x00),
    rgb(0x34, 0x65, 0xa4),
    rgb(0x75, 0x50, 0x7b),
    rgb(0x06, 0x98, 0x9a),
    rgb(0xd3, 0xd7, 0xcf),
    rgb(0x55, 0x57, 0x53),
    rgb(0xef, 0x29, 0x29),
    rgb(0x8a, 0xe2, 0x34),
    rgb(0xfc, 0xe9, 0x4f),
    rgb(0x72, 0x9f, 0xcf),
    rgb(0xad, 0x7f, 0xa8),
    rgb(0x34, 0xe2, 0xe2),
    rgb(0xee, 0xee, 0xec),
];

const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

fn main() -> io::Result<()> {
    let (cols, path) = parse_args()?;
    let mut input = Vec::new();
    match path {
        Some(path) => input = std::fs::read(path)?,
        None => {
            io::stdin().read_to_end(&mut input)?;
        }
    }
    let html = convert(&input, cols);
    io::stdout().write_all(html.as_bytes())
}

fn parse_args() -> io::Result<(usize, Option<String>)> {
    let invalid = |message: &str| io::Error::new(io::ErrorKind::InvalidInput, message.to_string());
    let (mut cols, mut path) = (100, None);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cols" => {
                cols = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(|| invalid("--cols needs a number"))?;
            }
            "-h" | "--help" => {
                println!("usage: ansi_to_html [--cols N] [FILE]   (reads standard input without FILE)");
                std::process::exit(0);
            }
            _ if arg.starts_with('-') => return Err(invalid(&format!("unknown option {arg}"))),
            _ => path = Some(arg),
        }
    }
    Ok((cols, path))
}

/// The page for `input`, as a terminal `cols` wide shows it.
pub fn convert(input: &[u8], cols: usize) -> String {
    // The screen is tall so that ordinary output stays on it; what scrolls off goes to the history,
    // which is kept in full (one terminal row of 100 columns costs about 800 bytes).
    let mut term = Terminal::builder().size(cols, 50).scrollback(1_000_000).build();
    for chunk in input.chunks(16 * 1024) {
        term.feed(chunk);
        // Nobody reads the replies of a recording; do not let them pile up.
        term.clear_replies();
        term.events().for_each(drop);
    }

    let colors = term.colors();
    let mut page = String::new();
    page.push_str("<!DOCTYPE html>\n<meta charset=\"utf-8\">\n<title>terminal output</title>\n");
    let _ = write!(
        page,
        "<style>body {{ margin: 0; background: {bg} }} pre {{ margin: 0; padding: 1em; color: {fg}; \
         background: {bg}; font: 14px/1.25 Menlo, Consolas, monospace; white-space: pre-wrap; \
         overflow-wrap: anywhere }} a {{ color: inherit }}</style>\n<pre>",
        bg = css(default_color(&colors, NamedColor::Background)),
        fg = css(default_color(&colors, NamedColor::Foreground)),
    );

    // The history, then the screen without the blank rows at its bottom.
    let first = -(term.history_len() as i32);
    let mut last = term.rows() as i32 - 1;
    while last >= 0 && term.row_text(Line(last), true).is_empty() {
        last -= 1;
    }
    let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
    for line in first..=last {
        term.copy_row(Line(line), &mut cells, Some(&mut extras));
        // A row that goes on in the next one (a soft wrap) is one line of text.
        let wrapped = term.is_wrapped(Line(line));
        row_html(&mut page, &cells, &extras, &colors, wrapped);
        if !wrapped {
            page.push('\n');
        }
    }
    page.push_str("</pre>\n");
    page
}

/// One row as HTML: runs of cells that look the same, in a `<span>` when they are not plain, and in
/// a link when the program put them under one.
fn row_html(out: &mut String, cells: &[Cell], extras: &RowExtras, colors: &Colors, wrapped: bool) {
    // Blanks at the end of a row carry no information, unless the line goes on.
    let mut end = cells.len();
    while !wrapped && end > 0 && is_plain_blank(&cells[end - 1], extras, end - 1) {
        end -= 1;
    }

    let mut run_start = 0;
    let mut run = String::new();
    for (col, cell) in cells[..end].iter().enumerate() {
        if col > run_start && !same_look(&cells[run_start], cell, extras, run_start, col) {
            flush(out, &run, &cells[run_start], extras, run_start, colors);
            run.clear();
            run_start = col;
        }
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue; // the second half of a wide character: the glyph was drawn once already
        }
        let ch = if cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) { ' ' } else { cell.c };
        escape_into(&mut run, ch);
        for mark in extras.zerowidth(col) {
            escape_into(&mut run, *mark); // accents, variation selectors, joiners
        }
    }
    flush(out, &run, cells.get(run_start).unwrap_or(&Cell::default()), extras, run_start, colors);
}

fn is_plain_blank(cell: &Cell, extras: &RowExtras, col: usize) -> bool {
    cell.c == ' '
        && cell.bg == Color::Named(NamedColor::Background)
        && !cell.flags.intersects(Flags::INVERSE | Flags::ALL_UNDERLINES | Flags::STRIKEOUT | Flags::OVERLINE)
        && extras.hyperlink(col).is_none()
}

/// Whether two cells are drawn alike, so that one `<span>` can cover both.
fn same_look(a: &Cell, b: &Cell, extras: &RowExtras, col_a: usize, col_b: usize) -> bool {
    let attributes =
        |cell: &Cell| cell.flags & !(Flags::WIDE_CHAR | Flags::WIDE_CHAR_SPACER | Flags::WRAPLINE);
    a.fg == b.fg
        && a.bg == b.bg
        && attributes(a) == attributes(b)
        && extras.underline_color(col_a) == extras.underline_color(col_b)
        && extras.hyperlink(col_a) == extras.hyperlink(col_b)
}

/// Write `text`, which was drawn with the look of `cell` (and the extras at `col`).
fn flush(out: &mut String, text: &str, cell: &Cell, extras: &RowExtras, col: usize, colors: &Colors) {
    if text.is_empty() {
        return;
    }
    let link = extras.hyperlink(col).and_then(safe_link);
    if let Some(uri) = link {
        let _ = write!(out, "<a href=\"{}\">", escape(uri));
    }
    match style(cell, extras.underline_color(col), colors) {
        Some(style) => {
            let _ = write!(out, "<span style=\"{style}\">{text}</span>");
        }
        None => out.push_str(text),
    }
    if link.is_some() {
        out.push_str("</a>");
    }
}

/// The target of a link if it is one a page may carry: a program chose it, and `javascript:` or
/// `data:` in an `href` would run code in the page.
fn safe_link(link: &Hyperlink) -> Option<&str> {
    let uri: &str = &link.uri;
    let scheme = uri.split_once(':')?.0.to_ascii_lowercase();
    matches!(scheme.as_str(), "http" | "https" | "mailto").then_some(uri)
}

/// The CSS of a cell's look, `None` for a plain one.
fn style(cell: &Cell, underline_color: Option<Color>, colors: &Colors) -> Option<String> {
    let flags = cell.flags;
    let (mut fg, mut bg) = (resolve(cell.fg, colors), resolve(cell.bg, colors));
    let default_fg = default_color(colors, NamedColor::Foreground);
    let default_bg = default_color(colors, NamedColor::Background);
    if flags.contains(Flags::INVERSE) {
        // Swapping with a default stands for the page's own colours.
        let (f, b) = (fg.unwrap_or(default_fg), bg.unwrap_or(default_bg));
        (fg, bg) = (Some(b), Some(f));
    }
    if flags.contains(Flags::HIDDEN) {
        fg = Some(bg.unwrap_or(default_bg));
    }

    let mut css = String::new();
    if let Some(fg) = fg {
        let _ = write!(css, "color:{};", self::css(fg));
    }
    if let Some(bg) = bg {
        let _ = write!(css, "background:{};", self::css(bg));
    }
    if flags.contains(Flags::BOLD) {
        css.push_str("font-weight:bold;");
    }
    if flags.contains(Flags::DIM) {
        css.push_str("opacity:.6;");
    }
    if flags.contains(Flags::ITALIC) {
        css.push_str("font-style:italic;");
    }
    let mut lines = Vec::new();
    if flags.intersects(Flags::ALL_UNDERLINES) {
        lines.push("underline");
        let shape = [
            (Flags::DOUBLE_UNDERLINE, "double"),
            (Flags::UNDERCURL, "wavy"),
            (Flags::DOTTED_UNDERLINE, "dotted"),
            (Flags::DASHED_UNDERLINE, "dashed"),
        ];
        if let Some((_, name)) = shape.iter().find(|(flag, _)| flags.contains(*flag)) {
            let _ = write!(css, "text-decoration-style:{name};");
        }
        if let Some(color) = underline_color.and_then(|color| resolve(color, colors)) {
            let _ = write!(css, "text-decoration-color:{};", self::css(color));
        }
    }
    if flags.contains(Flags::STRIKEOUT) {
        lines.push("line-through");
    }
    if flags.contains(Flags::OVERLINE) {
        lines.push("overline");
    }
    if !lines.is_empty() {
        let _ = write!(css, "text-decoration-line:{};", lines.join(" "));
    }
    (!css.is_empty()).then_some(css)
}

/// A cell's colour as RGB, `None` for "the default": the page's own.
fn resolve(color: Color, colors: &Colors) -> Option<Rgb> {
    match color {
        Color::Named(NamedColor::Foreground | NamedColor::Background | NamedColor::Cursor) => None,
        Color::Named(name) => Some(colors.get(name as usize).unwrap_or(ANSI[name as usize])),
        Color::Indexed(n) => Some(colors.get(n as usize).unwrap_or_else(|| indexed(n))),
        Color::Rgb(rgb) => Some(rgb),
    }
}

/// The page's default foreground or background: the one a program set (OSC 10, 11), else ours.
fn default_color(colors: &Colors, name: NamedColor) -> Rgb {
    let ours = if name == NamedColor::Foreground { DEFAULT_FG } else { DEFAULT_BG };
    colors.get(name as usize).unwrap_or(ours)
}

/// An entry of xterm's 256-colour palette: the sixteen, the 6x6x6 cube, the grey ramp.
fn indexed(n: u8) -> Rgb {
    match n {
        0..=15 => ANSI[n as usize],
        16..=231 => {
            let n = n - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            rgb(level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        232..=255 => {
            let v = 8 + 10 * (n - 232);
            rgb(v, v, v)
        }
    }
}

fn css(color: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    text.chars().for_each(|c| escape_into(&mut out, c));
    out
}

fn escape_into(out: &mut String, c: char) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&#39;"),
        // Controls other than the layout we make ourselves would be invisible or odd in a page.
        c if c.is_control() => out.push('\u{fffd}'),
        c => out.push(c),
    }
}
