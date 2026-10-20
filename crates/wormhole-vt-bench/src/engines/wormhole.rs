//! `wormhole-vt`'s terminal: the engine that replaces alacritty_terminal.

use wormhole_vt::grid::Style;
use wormhole_vt::{COLOR_COUNT, Color, Column, Flags, Line, NamedColor, Point, Rgb, TermMode, Terminal};

use super::xterm_palette;
use crate::engine::{Attrs, CELL_PIXELS, Cell, CellColor, Cursor, DEFAULT_COLORS, Engine};

pub struct WormholeEngine {
    term: Terminal,
    replies: Vec<u8>,
    /// The history the tools asked for. The terminal keeps at least this many lines (it drops
    /// whole pages), the adapter shows exactly this many.
    scrollback: usize,
}

fn cell_color(color: Color) -> CellColor {
    match color {
        Color::Rgb(Rgb { r, g, b }) => CellColor::Rgb(r, g, b),
        Color::Indexed(i) => CellColor::Indexed(i),
        // SGR 30-37 and 90-97 are palette slots 0-15.
        Color::Named(name) => match name as usize {
            i @ 0..=15 => CellColor::Indexed(i as u8),
            _ => CellColor::Default,
        },
    }
}

fn attrs(flags: Flags) -> Attrs {
    let mut out = Attrs::empty();
    for (flag, attr) in [
        (Flags::BOLD, Attrs::BOLD),
        (Flags::DIM, Attrs::DIM),
        (Flags::ITALIC, Attrs::ITALIC),
        (Flags::UNDERLINE, Attrs::UNDERLINE),
        (Flags::DOUBLE_UNDERLINE, Attrs::DOUBLE_UNDERLINE),
        (Flags::UNDERCURL, Attrs::CURLY_UNDERLINE),
        (Flags::DOTTED_UNDERLINE, Attrs::DOTTED_UNDERLINE),
        (Flags::DASHED_UNDERLINE, Attrs::DASHED_UNDERLINE),
        (Flags::BLINK, Attrs::BLINK),
        (Flags::INVERSE, Attrs::INVERSE),
        (Flags::HIDDEN, Attrs::HIDDEN),
        (Flags::STRIKEOUT, Attrs::STRIKE),
        (Flags::OVERLINE, Attrs::OVERLINE),
    ] {
        if flags.contains(flag) {
            out |= attr;
        }
    }
    out
}

/// A terminal set up the way the tools use it: xterm's palette and the fixed defaults for color
/// queries, and a cell of [`CELL_PIXELS`]. The bisimulation harness makes both of its terminals
/// this way, as a host would configure the engine and its replica alike.
pub fn host_terminal(cols: usize, rows: usize, scrollback: usize) -> Terminal {
    let mut palette = [Rgb::default(); COLOR_COUNT];
    for (slot, color) in palette.iter_mut().enumerate().take(256) {
        let (r, g, b) = xterm_palette(slot as u8);
        *color = Rgb { r, g, b };
    }
    for (slot, (r, g, b)) in
        [NamedColor::Foreground, NamedColor::Background, NamedColor::Cursor].into_iter().zip(DEFAULT_COLORS)
    {
        palette[slot as usize] = Rgb { r, g, b };
    }
    Terminal::builder()
        .size(cols, rows)
        .scrollback(scrollback)
        .palette(&palette)
        .cell_size(CELL_PIXELS.0 as u16, CELL_PIXELS.1 as u16)
        .build()
}

impl WormholeEngine {
    /// The line of history `index` (0 is the oldest the adapter shows).
    fn history_line(&self, index: usize) -> Line {
        Line(index as i32 - self.scrollback_len() as i32)
    }
}

impl Engine for WormholeEngine {
    const NAME: &'static str = "wormhole";

    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let term = host_terminal(cols, rows, scrollback);
        WormholeEngine { term, replies: Vec::new(), scrollback }
    }

    fn allow_screen_checksum(&mut self) {
        let mut policy = self.term.policy();
        policy.screen_checksum = true;
        self.term.set_policy(policy);
    }

    fn allow_window_ops(&mut self) {
        let mut policy = self.term.policy();
        policy.window_ops = true;
        self.term.set_policy(policy);
    }

    fn allow_title_report(&mut self) {
        let mut policy = self.term.policy();
        policy.title_report = true;
        self.term.set_policy(policy);
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.term.feed(bytes);
        self.replies.extend_from_slice(self.term.replies());
        self.term.clear_replies();
        // Events are for the host; the tools read the state.
        self.term.events().for_each(drop);
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.term.resize(cols, rows);
    }

    fn size(&self) -> (usize, usize) {
        (self.term.cols(), self.term.rows())
    }

    fn row_text(&self, row: usize) -> String {
        self.term.row_text(Line(row as i32), true)
    }

    fn row_text_raw(&self, row: usize) -> String {
        self.term.row_text(Line(row as i32), false)
    }

    fn row_wrapped(&self, row: usize) -> bool {
        self.term.is_wrapped(Line(row as i32))
    }

    fn cell(&self, row: usize, col: usize) -> Cell {
        let point = Point::new(Line(row as i32), Column(col));
        let grid = self.term.grid();
        let Some(cell) = grid.cell(point) else { return Cell::blank() };
        let style: Style = grid.style_at(point).unwrap_or_default();
        let mut text = String::new();
        if !cell.is_spacer_tail() {
            text.push(cell.character().unwrap_or(' '));
            text.extend(grid.marks_at(point));
        }
        Cell {
            text,
            width: if cell.is_spacer_tail() {
                0
            } else if cell.is_wide() {
                2
            } else {
                1
            },
            fg: cell_color(style.fg),
            bg: cell_color(style.bg),
            underline_color: style.underline_color.map(cell_color).unwrap_or_default(),
            attrs: attrs(style.attrs),
            link: grid.link_at(point).map(|link| link.uri.to_string()),
        }
    }

    fn cursor(&self) -> Cursor {
        let point = self.term.cursor_point();
        Cursor {
            row: point.line.0.max(0) as usize,
            col: point.column.0,
            visible: self.term.mode().contains(TermMode::SHOW_CURSOR),
        }
    }

    fn scrollback_len(&self) -> usize {
        self.term.history_len().min(self.scrollback)
    }

    fn scrollback_text(&self, index: usize) -> String {
        self.term.row_text(self.history_line(index), true)
    }

    fn scrollback_text_raw(&self, index: usize) -> String {
        self.term.row_text(self.history_line(index), false)
    }

    fn scrollback_wrapped(&self, index: usize) -> bool {
        self.term.is_wrapped(self.history_line(index))
    }

    fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    fn title(&self) -> Option<String> {
        self.term.title().map(str::to_string)
    }

    fn alt_screen(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }
}
