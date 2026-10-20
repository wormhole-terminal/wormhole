//! Everything observable about a terminal, as named text, for the tests that compare two terminals
//! (the repaint's bisimulation, `wormhole-vt-bench`): one field per piece of state, so that a
//! mismatch names the first thing that differs instead of saying two big values are unequal.
//!
//! Behind the `state-dump` feature: the engine ships without it. What goes in is whatever the
//! host, the renderer or a later sequence can read: the screens' cells with their styles, links,
//! clusters and flags, the cursor and the saved ones, the modes, tables, colors, titles and the
//! counters. What stays out is the host's own configuration (the default palette, the window it
//! reports, which answers are allowed), the queues the host drains (events, replies, the OSC 7
//! reports), the viewer's state (display offset, selection, pins) and bookkeeping that nothing can
//! observe (page ids, spare pages, scratch buffers). `terminal::repaint::FIELDS` says which is which
//! for every field of the engine, and a test keeps that list whole.
//!
//! One cell is written in a normal form: a cell nothing was written to that carries a style no
//! erase can make (it is what is left of a wide character when half of it is overwritten) reads as
//! a written space with that style. Every consumer of the grid treats the two alike (text copy, the
//! snapshot, the renderer); only HT's tab mark and "is this row blank" tell them apart, and a
//! repaint cannot make the first from VT.

use std::fmt::Write as _;

use crate::grid::{BlockPart, Grid, RowView, Style};
use crate::types::{Color, Line, LineSize, NamedColor};

use crate::terminal::Terminal;

/// The named parts of a terminal's state, in a fixed order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateDump {
    /// `(name, value)`; the names are stable (`cursor`, `modes`, `primary/row[-3]`, …).
    pub fields: Vec<(String, String)>,
}

impl StateDump {
    fn field(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.fields.push((name.into(), value.into()));
    }

    /// The first `limit` fields that differ between `self` and `other`, one per line: the name,
    /// then each side's value (cut to a readable length around the first difference).
    pub fn diff(&self, other: &StateDump, limit: usize) -> String {
        fn say(out: &mut String, shown: &mut usize, name: &str, left: &str, right: &str) {
            let _ = writeln!(
                out,
                "{name}\n    left : {}\n    right: {}",
                excerpt(left, right),
                excerpt(right, left)
            );
            *shown += 1;
        }
        let mut out = String::new();
        let mut shown = 0;
        let (mut a, mut b) = (self.fields.iter().peekable(), other.fields.iter().peekable());
        while shown < limit {
            match (a.peek(), b.peek()) {
                (None, None) => break,
                (Some((name, left)), Some((other_name, right))) if name == other_name => {
                    if left != right {
                        say(&mut out, &mut shown, name, left, right);
                    }
                    a.next();
                    b.next();
                }
                (Some((name, left)), Some((other_name, _))) if name < other_name => {
                    say(&mut out, &mut shown, name, left, "(missing)");
                    a.next();
                }
                (Some(_), Some((other_name, right))) => {
                    say(&mut out, &mut shown, other_name, "(missing)", right);
                    b.next();
                }
                (Some((name, left)), None) => {
                    say(&mut out, &mut shown, name, left, "(missing)");
                    a.next();
                }
                (None, Some((name, right))) => {
                    say(&mut out, &mut shown, name, "(missing)", right);
                    b.next();
                }
            }
        }
        out
    }

    /// Whether the two dumps hold the same fields with the same values.
    pub fn same(&self, other: &StateDump) -> bool {
        self == other
    }
}

/// `text` cut to about 160 characters around its first difference from `other`.
fn excerpt(text: &str, other: &str) -> String {
    const WIDTH: usize = 160;
    if text.len() <= WIDTH {
        return text.to_string();
    }
    let at = text.bytes().zip(other.bytes()).position(|(a, b)| a != b).unwrap_or(0);
    let mut from = at.saturating_sub(WIDTH / 2);
    while !text.is_char_boundary(from) {
        from -= 1;
    }
    let mut to = (from + WIDTH).min(text.len());
    while !text.is_char_boundary(to) {
        to += 1;
    }
    format!(
        "{}{}{}",
        if from > 0 { "…" } else { "" },
        &text[from..to],
        if to < text.len() { "…" } else { "" }
    )
}

fn color(c: Color) -> String {
    match c {
        Color::Named(NamedColor::Foreground) => "fg".into(),
        Color::Named(NamedColor::Background) => "bg".into(),
        Color::Named(NamedColor::Cursor) => "cur".into(),
        Color::Named(named) => format!("n{}", named as u16),
        Color::Indexed(i) => format!("i{i}"),
        Color::Rgb(rgb) => format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b),
    }
}

fn style(s: &Style) -> String {
    if s.is_default() {
        return String::new();
    }
    let ul = s.underline_color.map_or(String::new(), color);
    format!("{}/{}/{ul}/{:x}", color(s.fg), color(s.bg), s.attrs.bits())
}

/// Whether an erase can make a blank with this style: the default, or just a background.
fn erasable(s: &Style) -> bool {
    s.is_default() || *s == Style::erased(s.bg)
}

/// One cell as text, in the normal form the module docs describe.
fn cell_token(out: &mut String, view: &RowView<'_>, x: usize, link: Option<&crate::types::Hyperlink>) {
    let cell = view.cells()[x];
    let st = view.style(cell.style_id());
    let mut cp = cell.codepoint();
    let layout = cell.is_wide() || cell.is_spacer_tail() || cell.is_spacer_head();
    if cp == 0 && !layout && !cell.has_grapheme() && cell.flags() == 0 && !erasable(st) {
        cp = 0x20;
    }
    if cp == 0 {
        out.push('_');
    } else {
        let _ = write!(out, "{cp:x}");
    }
    for mark in view.marks(x) {
        let _ = write!(out, ".{:x}", *mark as u32);
    }
    let s = style(st);
    if !s.is_empty() {
        let _ = write!(out, " s{s}");
    }
    if cell.flags() != 0 {
        let _ = write!(out, " f{:x}", cell.flags());
    }
    if let Some(link) = link {
        let _ = write!(out, " l{}@{}", link.id, link.uri);
    }
}

/// A row: its wrap flag, its mark and its cells in runs of equal ones.
fn row_text(grid: &Grid, line: Line) -> String {
    let Some(view) = grid.row_view(line) else { return "(no row)".into() };
    let mark = match grid.row_mark(line) {
        Some(m) => format!("{}:{}{}{}", m.block, u8::from(m.prompt), u8::from(m.command), u8::from(m.output)),
        None => "-".into(),
    };
    let size = match view.line_size() {
        LineSize::DoubleWidth => "dwl ",
        LineSize::DoubleHeightTop => "dhl-top ",
        LineSize::DoubleHeightBottom => "dhl-bottom ",
        _ => "",
    };
    let mut out = format!("{}{size}{mark} ", if view.wrapped() { "wrap " } else { "" });
    let mut links = view.links().peekable();
    let (mut prev, mut run_start) = (String::new(), 0usize);
    let mut token = String::new();
    for x in 0..view.cells().len() {
        while links.peek().is_some_and(|(_, end, _)| *end <= x) {
            links.next();
        }
        let link = links.peek().filter(|(start, _, _)| *start <= x).map(|(_, _, link)| *link);
        token.clear();
        cell_token(&mut token, &view, x, link);
        if token != prev {
            if x > 0 {
                let _ = write!(out, "[{run_start}-{}]{prev} ", x - 1);
            }
            prev.clone_from(&token);
            run_start = x;
        }
    }
    let _ = write!(out, "[{run_start}-{}]{prev}", view.cells().len() - 1);
    out
}

fn dump_grid(dump: &mut StateDump, name: &str, grid: &Grid) {
    dump.field(
        format!("{name}/size"),
        format!("{}x{} history {}", grid.cols(), grid.rows(), grid.history_len()),
    );
    dump.field(format!("{name}/limit"), grid.scrollback_limit().to_string());
    let first = -(grid.history_len() as i32);
    for (n, y) in (first..grid.rows() as i32).enumerate() {
        dump.field(format!("{name}/row[{n:07}]"), format!("L{y} {}", row_text(grid, Line(y))));
    }
    for (i, placement) in grid.describe_placements().into_iter().enumerate() {
        dump.field(format!("{name}/placement[{i}]"), placement);
    }
}

fn content(c: BlockPart) -> &'static str {
    match c {
        BlockPart::None => "none",
        BlockPart::Prompt => "prompt",
        BlockPart::Command => "command",
        BlockPart::Output => "output",
    }
}

impl Terminal {
    /// Every observable piece of the terminal's state; see the module docs.
    pub fn dump_state(&self) -> StateDump {
        let mut d = StateDump::default();
        let alt = self.on_alt_screen();
        let (primary, other) = if alt { (&self.other, &self.grid) } else { (&self.grid, &self.other) };
        d.field("a/size", format!("{}x{}", self.cols, self.rows));
        d.field("a/screen", if alt { "alternate" } else { "primary" });
        d.field("a/cursor", format!("{:?}", self.cursor));
        d.field(
            "a/pen",
            format!(
                "{:?} link={:?} content={} block={} protected={} guarded={}",
                self.pen.style,
                self.pen.link,
                content(self.pen.content),
                self.pen.block,
                self.pen.protected,
                self.pen.guarded
            ),
        );
        d.field("a/modes", format!("{:?}", self.modes));
        d.field("a/recorded", format!("{:#x}", self.recorded));
        d.field("a/saved_modes", format!("{:?}", self.saved_modes));
        d.field("a/margins", format!("{:?}", self.margins));
        d.field("a/tabs", format!("{:?}", self.tabs.columns().collect::<Vec<_>>()));
        d.field("a/charsets", format!("{:?}", self.charsets));
        d.field("a/locator", format!("{:?}", self.locator.settings()));
        d.field("a/preferred", format!("{:?}", self.preferred));
        d.field("a/vt52", format!("{:?}", self.vt52));
        d.field("a/sgr_stack", format!("{:?}", self.sgr_stack));
        d.field("a/color_stack", format!("{:?}", self.color_stack));
        d.field("a/saved[0]", format!("{:?}", self.saved[0]));
        d.field("a/saved[1]", format!("{:?}", self.saved[1]));
        d.field("a/keys[0]", format!("{:?}", self.keys[0]));
        d.field("a/keys[1]", format!("{:?}", self.keys[1]));
        d.field(
            "a/cursor_shape",
            format!("{:?} blink {:?}", self.cursor_style.shape, self.cursor_style.blink),
        );
        d.field("a/title", format!("{:?}", self.titles.window));
        d.field("a/icon_name", format!("{:?}", self.titles.icon));
        d.field("a/title_modes", format!("{:?}", self.titles.modes));
        d.field("a/title_stack", format!("{:?}", self.titles.stack));
        d.field("a/cwd", format!("{:?}", self.folder.now));
        d.field("a/colors", format!("{:?}", self.palette.set));
        d.field("a/special_colors", format!("{:?}", self.palette.special));
        d.field("a/extra_colors", format!("{:?}", self.palette.extra));
        d.field("a/last_char", format!("{:?}", self.last_char));
        // A closed cluster's other fields are dead values: the next character overwrites them.
        d.field("a/cluster", if self.cluster.open { format!("{:?}", self.cluster) } else { "closed".into() });
        d.field("a/block", self.block.to_string());
        d.field("a/stashed_content", content(self.stashed_content));
        d.field("a/link_seq", self.link_seq.to_string());
        d.field("a/history_just_cleared", self.history_just_cleared.to_string());
        d.field("a/level", format!("{:?} eight_bit {}", self.level, self.eight_bit));
        d.field("a/kept", format!("{:?}", self.kept));
        d.field("a/parser", self.parser.essence());
        d.field("a/guard_used", self.guard_used.to_string());
        d.field("a/attr_extent", format!("{:?}", self.attr_extent));
        for (name, value) in self.graphics.describe() {
            d.field(name, value);
        }
        dump_grid(&mut d, "primary", primary);
        dump_grid(&mut d, "alternate", other);
        // The names sort the way a reader looks for them; the diff walks both lists in order.
        d.fields.sort_by(|(a, _), (b, _)| a.cmp(b));
        d
    }
}

#[cfg(test)]
mod tests {
    use crate::terminal::Terminal;

    /// Two terminals fed the same bytes in different pieces show the same state, also when a piece
    /// ended in the middle of a character: the parser's buffer for it keeps the old bytes.
    #[test]
    fn the_parser_shows_only_the_live_bytes_of_a_cut_character() {
        let text = "a€é😀b \x1b[1m€".as_bytes();
        let mut whole = Terminal::new(20, 3);
        whole.feed(text);
        let mut by_byte = Terminal::new(20, 3);
        for byte in text {
            by_byte.feed(&[*byte]);
        }
        assert_eq!(whole.dump_state().diff(&by_byte.dump_state(), 4), "");
        // Cut inside a character, the held bytes show.
        let mut cut = Terminal::new(20, 3);
        cut.feed(&text[..2]);
        let dump = cut.dump_state();
        let parser = dump.fields.iter().find(|(name, _)| name == "a/parser").map(|(_, value)| value.as_str());
        assert_eq!(parser, Some("Utf8 pending [226] string_utf8 0"));
    }
}
