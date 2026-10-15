//! alacritty_terminal 0.26: the engine Wormhole ran until its own replaced it, and the reference the
//! new one is measured against. A black box behind its public API.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, Rgb};

use wormhole_vt_bench::engine::{CELL_PIXELS, DEFAULT_COLORS};
use wormhole_vt_bench::engines::xterm_palette;
use wormhole_vt_bench::{Attrs, Cell, CellColor, Cursor, Engine};

/// What the terminal asked the host for while a chunk was parsed, in order. Color and size
/// requests are answered after the chunk, when the terminal can be asked what they are now.
enum Pending {
    Bytes(Vec<u8>),
    Color(usize, Arc<dyn Fn(Rgb) -> String + Send + Sync>),
    Size(Arc<dyn Fn(WindowSize) -> String + Send + Sync>),
}

#[derive(Default)]
struct Shared {
    pending: Vec<Pending>,
    title: Option<String>,
}

struct Listener(Rc<RefCell<Shared>>);

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let mut shared = self.0.borrow_mut();
        match event {
            Event::PtyWrite(text) => shared.pending.push(Pending::Bytes(text.into_bytes())),
            Event::ColorRequest(index, format) => shared.pending.push(Pending::Color(index, format)),
            Event::TextAreaSizeRequest(format) => shared.pending.push(Pending::Size(format)),
            Event::Title(title) => shared.title = Some(title),
            Event::ResetTitle => shared.title = None,
            _ => {}
        }
    }
}

pub struct AlacrittyEngine {
    term: Term<Listener>,
    processor: Processor,
    shared: Rc<RefCell<Shared>>,
    replies: Vec<u8>,
}

impl AlacrittyEngine {
    fn resolve_pending(&mut self) {
        let pending = std::mem::take(&mut self.shared.borrow_mut().pending);
        for request in pending {
            match request {
                Pending::Bytes(bytes) => self.replies.extend_from_slice(&bytes),
                Pending::Color(index, format) => {
                    let (r, g, b) = self.term.colors()[index]
                        .map(|rgb| (rgb.r, rgb.g, rgb.b))
                        .unwrap_or_else(|| default_color(index));
                    self.replies.extend_from_slice(format(Rgb { r, g, b }).as_bytes());
                }
                Pending::Size(format) => {
                    let window = WindowSize {
                        num_lines: self.term.screen_lines() as u16,
                        num_cols: self.term.columns() as u16,
                        cell_width: CELL_PIXELS.0 as u16,
                        cell_height: CELL_PIXELS.1 as u16,
                    };
                    self.replies.extend_from_slice(format(window).as_bytes());
                }
            }
        }
    }

    fn line_text(&self, line: Line) -> String {
        let mut text = self.line_text_raw(line);
        text.truncate(text.trim_end_matches(' ').len());
        text
    }

    fn line_text_raw(&self, line: Line) -> String {
        let row = &self.term.grid()[line];
        let mut text = String::new();
        for col in 0..self.term.columns() {
            let cell = &row[Column(col)];
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            text.push(shown(cell.c));
            if let Some(extra) = cell.zerowidth() {
                text.extend(extra.iter());
            }
        }
        text
    }
}

/// What a cell shows: alacritty stores a tab character in the first cell a tab crosses, but the
/// cell is blank on screen.
fn shown(c: char) -> char {
    if c == '\t' { ' ' } else { c }
}

fn default_color(index: usize) -> (u8, u8, u8) {
    match index {
        0..=255 => xterm_palette(index as u8),
        i if i == NamedColor::Foreground as usize => DEFAULT_COLORS[0],
        i if i == NamedColor::Background as usize => DEFAULT_COLORS[1],
        _ => DEFAULT_COLORS[2],
    }
}

fn cell_color(color: Color) -> CellColor {
    match color {
        Color::Spec(rgb) => CellColor::Rgb(rgb.r, rgb.g, rgb.b),
        Color::Indexed(i) => CellColor::Indexed(i),
        Color::Named(name) => match name as usize {
            // Alacritty keeps SGR 30-37 and 90-97 as named colors; they are palette slots 0-15.
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
        (Flags::INVERSE, Attrs::INVERSE),
        (Flags::HIDDEN, Attrs::HIDDEN),
        (Flags::STRIKEOUT, Attrs::STRIKE),
    ] {
        if flags.contains(flag) {
            out |= attr;
        }
    }
    out
}

impl Engine for AlacrittyEngine {
    const NAME: &'static str = "alacritty";

    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let shared = Rc::new(RefCell::new(Shared::default()));
        // The app's own settings: only the history length and the kitty keyboard protocol differ
        // from alacritty's defaults.
        let config = Config { scrolling_history: scrollback, kitty_keyboard: true, ..Config::default() };
        let term = Term::new(config, &TermSize::new(cols, rows), Listener(shared.clone()));
        AlacrittyEngine { term, processor: Processor::new(), shared, replies: Vec::new() }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
        // A synchronized update (mode 2026) holds the bytes back until it ends or times out; the
        // tools want the screen the bytes make, so end it.
        if self.processor.sync_timeout().sync_timeout().is_some() {
            self.processor.stop_sync(&mut self.term);
        }
        self.resolve_pending();
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.term.resize(TermSize::new(cols, rows));
    }

    fn size(&self) -> (usize, usize) {
        (self.term.columns(), self.term.screen_lines())
    }

    fn row_text(&self, row: usize) -> String {
        self.line_text(Line(row as i32))
    }

    fn row_text_raw(&self, row: usize) -> String {
        self.line_text_raw(Line(row as i32))
    }

    fn row_wrapped(&self, row: usize) -> bool {
        let last = self.term.columns() - 1;
        self.term.grid()[Line(row as i32)][Column(last)].flags.contains(Flags::WRAPLINE)
    }

    fn cell(&self, row: usize, col: usize) -> Cell {
        let cell = &self.term.grid()[Line(row as i32)][Column(col)];
        let spacer = cell.flags.intersects(Flags::WIDE_CHAR_SPACER);
        let mut text = String::new();
        if !spacer {
            text.push(shown(cell.c));
            if let Some(extra) = cell.zerowidth() {
                text.extend(extra.iter());
            }
        }
        Cell {
            text,
            width: if spacer {
                0
            } else if cell.flags.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            },
            fg: cell_color(cell.fg),
            bg: cell_color(cell.bg),
            underline_color: cell.underline_color().map(cell_color).unwrap_or_default(),
            attrs: attrs(cell.flags),
            link: cell.hyperlink().map(|link| link.uri().to_string()),
        }
    }

    fn cursor(&self) -> Cursor {
        let point = self.term.grid().cursor.point;
        Cursor {
            row: point.line.0.max(0) as usize,
            col: point.column.0,
            visible: self.term.mode().contains(TermMode::SHOW_CURSOR),
        }
    }

    fn scrollback_len(&self) -> usize {
        self.term.grid().history_size()
    }

    fn scrollback_text(&self, index: usize) -> String {
        let oldest = -(self.scrollback_len() as i32);
        self.line_text(Line(oldest + index as i32))
    }

    fn scrollback_text_raw(&self, index: usize) -> String {
        let oldest = -(self.scrollback_len() as i32);
        self.line_text_raw(Line(oldest + index as i32))
    }

    fn scrollback_wrapped(&self, index: usize) -> bool {
        let oldest = -(self.scrollback_len() as i32);
        let last = self.term.columns() - 1;
        self.term.grid()[Line(oldest + index as i32)][Column(last)].flags.contains(Flags::WRAPLINE)
    }

    fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    fn title(&self) -> Option<String> {
        self.shared.borrow().title.clone()
    }

    fn alt_screen(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::event::VoidListener;

    use super::*;

    /// The reason `feed` ends a synchronized update: left alone, the engine holds the bytes back
    /// and the screen the tools read would be behind what was fed.
    #[test]
    fn the_engine_holds_synchronized_output_back_until_it_ends() {
        let mut term = Term::new(Config::default(), &TermSize::new(10, 2), VoidListener);
        let mut processor: Processor = Processor::new();
        processor.advance(&mut term, b"\x1b[?2026hhi");
        assert_eq!(term.grid()[Line(0)][Column(0)].c, ' ');
        processor.stop_sync(&mut term);
        assert_eq!(term.grid()[Line(0)][Column(0)].c, 'h');
    }

    #[test]
    fn feed_shows_everything_it_was_given() {
        let mut engine = AlacrittyEngine::new(10, 2, 0);
        engine.feed(b"\x1b[?2026hhi");
        assert_eq!(engine.row_text(0), "hi");
    }

    #[test]
    fn replies_keep_the_order_they_were_asked_in() {
        let mut engine = AlacrittyEngine::new(10, 2, 0);
        // A plain reply, a reply the host formats (a color) and a plain one again.
        engine.feed(b"\x1b[5n\x1b]11;?\x07\x1b[6n");
        let replies = String::from_utf8(engine.take_replies()).unwrap_or_default();
        assert!(replies.starts_with("\x1b[0n\x1b]11;rgb:1e1e/1e1e/1e1e\x07\x1b[1;1R"), "{replies:?}");
        assert!(engine.take_replies().is_empty());
    }

    #[test]
    fn a_color_set_by_the_program_is_what_a_query_returns() {
        let mut engine = AlacrittyEngine::new(10, 2, 0);
        engine.feed(b"\x1b]4;1;rgb:12/34/56\x07\x1b]4;1;?\x07");
        let replies = String::from_utf8(engine.take_replies()).unwrap_or_default();
        assert_eq!(replies, "\x1b]4;1;rgb:1212/3434/5656\x07");
    }

    #[test]
    fn colors_are_reported_as_palette_slots_or_rgb() {
        let mut engine = AlacrittyEngine::new(10, 2, 0);
        engine.feed(b"\x1b[31ma\x1b[91mb\x1b[38;5;200mc\x1b[38;2;1;2;3md\x1b[0me");
        let fg = |col| engine.cell(0, col).fg;
        assert_eq!(fg(0), CellColor::Indexed(1));
        assert_eq!(fg(1), CellColor::Indexed(9));
        assert_eq!(fg(2), CellColor::Indexed(200));
        assert_eq!(fg(3), CellColor::Rgb(1, 2, 3));
        assert_eq!(fg(4), CellColor::Default);
    }

    #[test]
    fn a_tab_reads_as_blanks() {
        let mut engine = AlacrittyEngine::new(20, 2, 0);
        engine.feed(b"a\tb");
        assert_eq!(engine.row_text(0), "a       b");
    }

    #[test]
    fn history_is_read_oldest_first() {
        let mut engine = AlacrittyEngine::new(5, 2, 10);
        engine.feed(b"1\r\n2\r\n3\r\n4");
        assert_eq!(engine.scrollback_len(), 2);
        assert_eq!((engine.scrollback_text(0), engine.scrollback_text(1)), ("1".into(), "2".into()));
    }
}
