//! WezTerm's terminal model, the `wezterm-term` crate (a git dependency: it is not on crates.io;
//! `Cargo.toml` pins the commit). The same model the WezTerm app draws from.
//!
//! The configuration is the crate's defaults (so, among other things, Unicode 9 widths, as the
//! app has by default) except the history length and the colors, set to the fixed values every
//! adapter uses. What it writes back to the program goes into a buffer `take_replies` empties.
//!
//! The model writes its answers from a thread of its own, some time after `advance_bytes`
//! returns. `take_replies` therefore types a key nobody else types and waits for it to come out
//! of the same writer: a key goes through no parser, so it works in the middle of an escape
//! sequence too, and the writer is a queue, so everything before it has come out when it does.

use std::cell::RefCell;
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use wezterm_surface::CursorVisibility;
use wezterm_term::color::{ColorAttribute, ColorPalette, Palette256, SrgbaTuple};
use wezterm_term::{
    Blink, Intensity, KeyCode, KeyModifiers, Terminal, TerminalConfiguration, TerminalSize, Underline,
    VisibleRowIndex,
};
use wormhole_vt_bench::engine::{CELL_PIXELS, DEFAULT_COLORS};
use wormhole_vt_bench::engines::xterm_palette;
use wormhole_vt_bench::{Attrs, Cell, CellColor, Cursor, Engine};

use super::Region;

#[derive(Debug)]
struct Config {
    scrollback: usize,
}

/// The key `take_replies` types to know the writer has caught up: a private-use character, which
/// no program's answer contains.
const MARKER: char = '\u{f8f0}';

fn rgb((r, g, b): (u8, u8, u8)) -> SrgbaTuple {
    SrgbaTuple::from((r, g, b))
}

impl TerminalConfiguration for Config {
    fn scrollback_size(&self) -> usize {
        self.scrollback
    }

    fn color_palette(&self) -> ColorPalette {
        ColorPalette {
            colors: Palette256(std::array::from_fn(|i| rgb(xterm_palette(i as u8)))),
            foreground: rgb(DEFAULT_COLORS[0]),
            background: rgb(DEFAULT_COLORS[1]),
            cursor_fg: rgb(DEFAULT_COLORS[1]),
            cursor_bg: rgb(DEFAULT_COLORS[2]),
            cursor_border: rgb(DEFAULT_COLORS[2]),
            ..ColorPalette::default()
        }
    }
}

/// Where the terminal's answers to queries go.
#[derive(Clone, Default)]
struct Replies(Arc<Mutex<Vec<u8>>>);

impl Write for Replies {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A `wezterm-term` terminal and the buffer its answers are written to.
pub struct WeztermEngine {
    // Reading a screen takes `&mut` in places the trait gives `&self`.
    term: RefCell<Terminal>,
    replies: Replies,
}

fn size(cols: usize, rows: usize) -> TerminalSize {
    TerminalSize { rows, cols, pixel_width: cols * CELL_PIXELS.0, pixel_height: rows * CELL_PIXELS.1, dpi: 0 }
}

fn color(color: ColorAttribute) -> CellColor {
    match color {
        ColorAttribute::Default => CellColor::Default,
        ColorAttribute::PaletteIndex(i) => CellColor::Indexed(i),
        ColorAttribute::TrueColorWithPaletteFallback(c, _)
        | ColorAttribute::TrueColorWithDefaultFallback(c) => {
            let (r, g, b, _) = c.to_srgb_u8();
            CellColor::Rgb(r, g, b)
        }
    }
}

impl WeztermEngine {
    /// The physical row of history line `index` (0 = oldest) or of screen row `index`. The
    /// history is the physical rows above the first visible one; `scrollback_rows` counts all of
    /// them, the visible ones too.
    fn phys_row(term: &Terminal, region: Region, index: usize) -> usize {
        match region {
            Region::History => index,
            Region::Screen => term.screen().phys_row(index as VisibleRowIndex),
        }
    }

    fn with_line<T>(
        &self,
        region: Region,
        index: usize,
        read: impl FnOnce(&wezterm_term::Line) -> T,
    ) -> Option<T> {
        let term = self.term.borrow();
        let row = Self::phys_row(&term, region, index);
        // The model keeps rows it has not been written to out of its list: a row past the end is
        // a blank one.
        if row >= term.screen().scrollback_rows() {
            return None;
        }
        // `lines_in_phys_range` copies the line. The borrowing `with_phys_lines` panics on a
        // range that straddles the end of the ring buffer the lines live in, which a screen that
        // has scrolled down (IL, RI, SD) has.
        let lines = term.screen().lines_in_phys_range(row..row + 1);
        lines.first().map(read)
    }

    fn line_text(line: &wezterm_term::Line) -> String {
        let mut text = Self::line_text_raw(line);
        text.truncate(text.trim_end_matches(' ').len());
        text
    }

    /// [`Self::line_text`] with the trailing blanks kept (only the cells the line stores).
    fn line_text_raw(line: &wezterm_term::Line) -> String {
        let mut text = String::new();
        let mut column = 0;
        for cell in line.visible_cells() {
            // Cells that were never written are not stored; they read as blanks.
            while column < cell.cell_index() {
                text.push(' ');
                column += 1;
            }
            let s = cell.str();
            text.push_str(if s.is_empty() { " " } else { s });
            column += cell.width().max(1);
        }
        text
    }
}

impl Engine for WeztermEngine {
    const NAME: &'static str = "wezterm";

    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let replies = Replies::default();
        let term = Terminal::new(
            size(cols, rows),
            Arc::new(Config { scrollback }),
            "wormhole-vt-compare",
            "0",
            Box::new(replies.clone()),
        );
        WeztermEngine { term: RefCell::new(term), replies }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.term.get_mut().advance_bytes(bytes);
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.term.get_mut().resize(size(cols, rows));
    }

    fn size(&self) -> (usize, usize) {
        let size = self.term.borrow().get_size();
        (size.cols, size.rows)
    }

    fn row_text(&self, row: usize) -> String {
        self.with_line(Region::Screen, row, Self::line_text).unwrap_or_default()
    }

    fn row_text_raw(&self, row: usize) -> String {
        self.with_line(Region::Screen, row, Self::line_text_raw).unwrap_or_default()
    }

    fn row_wrapped(&self, row: usize) -> bool {
        self.with_line(Region::Screen, row, |line| line.last_cell_was_wrapped()).unwrap_or(false)
    }

    fn cell(&self, row: usize, col: usize) -> Cell {
        self.with_line(Region::Screen, row, |line| {
            // The blank after a wide character is stored as a cell of its own.
            if col > 0 && line.get_cell(col - 1).is_some_and(|c| c.width() == 2) {
                return Cell { width: 0, text: String::new(), ..Cell::blank() };
            }
            let Some(cell) = line.get_cell(col) else { return Cell::blank() };
            let attrs_of = |a: &wezterm_term::CellAttributes| {
                let mut attrs = Attrs::empty();
                match a.intensity() {
                    Intensity::Bold => attrs |= Attrs::BOLD,
                    Intensity::Half => attrs |= Attrs::DIM,
                    Intensity::Normal => {}
                }
                match a.underline() {
                    Underline::None => {}
                    Underline::Single => attrs |= Attrs::UNDERLINE,
                    Underline::Double => attrs |= Attrs::DOUBLE_UNDERLINE,
                    Underline::Curly => attrs |= Attrs::CURLY_UNDERLINE,
                    Underline::Dotted => attrs |= Attrs::DOTTED_UNDERLINE,
                    Underline::Dashed => attrs |= Attrs::DASHED_UNDERLINE,
                }
                for (on, attr) in [
                    (a.italic(), Attrs::ITALIC),
                    (a.blink() != Blink::None, Attrs::BLINK),
                    (a.reverse(), Attrs::INVERSE),
                    (a.invisible(), Attrs::HIDDEN),
                    (a.strikethrough(), Attrs::STRIKE),
                    (a.overline(), Attrs::OVERLINE),
                ] {
                    if on {
                        attrs |= attr;
                    }
                }
                attrs
            };
            let attrs = cell.attrs();
            let text = cell.str();
            Cell {
                text: if text.is_empty() { " ".to_string() } else { text.to_string() },
                width: cell.width().clamp(1, 2) as u8,
                fg: color(attrs.foreground()),
                bg: color(attrs.background()),
                underline_color: color(attrs.underline_color()),
                attrs: attrs_of(attrs),
                link: attrs.hyperlink().map(|link| link.uri().to_string()),
            }
        })
        .unwrap_or_else(Cell::blank)
    }

    fn cursor(&self) -> Cursor {
        let position = self.term.borrow().cursor_pos();
        // A cursor waiting to wrap is at one past the last column in this model; on the screen it is
        // on the last column.
        let (cols, _) = self.size();
        Cursor {
            row: position.y.max(0) as usize,
            col: position.x.min(cols.saturating_sub(1)),
            visible: matches!(position.visibility, CursorVisibility::Visible),
        }
    }

    fn scrollback_len(&self) -> usize {
        self.term.borrow().screen().phys_row(0)
    }

    fn scrollback_text(&self, index: usize) -> String {
        self.with_line(Region::History, index, Self::line_text).unwrap_or_default()
    }

    fn scrollback_text_raw(&self, index: usize) -> String {
        self.with_line(Region::History, index, Self::line_text_raw).unwrap_or_default()
    }

    fn scrollback_wrapped(&self, index: usize) -> bool {
        self.with_line(Region::History, index, |line| line.last_cell_was_wrapped()).unwrap_or(false)
    }

    fn take_replies(&mut self) -> Vec<u8> {
        let mut marker = [0u8; 4];
        let marker = MARKER.encode_utf8(&mut marker).as_bytes().to_vec();
        if self.term.get_mut().key_down(KeyCode::Char(MARKER), KeyModifiers::NONE).is_err() {
            return Vec::new();
        }
        // The answers are written in order, so the marker coming out ends the wait; the limit is
        // for a writer that died.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            {
                let mut written = self.replies.0.lock().unwrap_or_else(PoisonError::into_inner);
                if written.ends_with(&marker) {
                    let len = written.len() - marker.len();
                    written.truncate(len);
                    return std::mem::take(&mut *written);
                }
            }
            if Instant::now() >= deadline {
                return std::mem::take(&mut *self.replies.0.lock().unwrap_or_else(PoisonError::into_inner));
            }
            std::thread::yield_now();
        }
    }

    fn title(&self) -> Option<String> {
        let term = self.term.borrow();
        let title = term.get_title();
        (!title.is_empty()).then(|| title.to_string())
    }

    fn alt_screen(&self) -> bool {
        self.term.borrow().is_alt_screen_active()
    }
}

#[cfg(test)]
mod tests {
    use wormhole_vt_bench::{Attrs, CellColor, Engine};

    use super::WeztermEngine;

    #[test]
    fn text_colors_and_cursor() {
        let mut engine = WeztermEngine::new(10, 3, 10);
        engine.feed(b"\x1b[1;31mhi\x1b[0m there");
        assert_eq!(engine.row_text(0), "hi there");
        let cell = engine.cell(0, 0);
        assert_eq!((cell.fg, cell.attrs), (CellColor::Indexed(1), Attrs::BOLD));
        let cursor = engine.cursor();
        assert_eq!((cursor.row, cursor.col, cursor.visible), (0, 8, true));
    }

    #[test]
    fn replies_are_taken_once_in_order() {
        let mut engine = WeztermEngine::new(10, 3, 0);
        engine.feed(b"\x1b[5n\x1b[6n");
        assert_eq!(engine.take_replies(), b"\x1b[0n\x1b[1;1R");
        assert!(engine.take_replies().is_empty());
    }

    #[test]
    fn history_is_read_oldest_first() {
        let mut engine = WeztermEngine::new(5, 2, 100);
        engine.feed(b"1\r\n2\r\n3\r\n4");
        assert_eq!(engine.scrollback_len(), 2);
        assert_eq!((engine.scrollback_text(0), engine.scrollback_text(1)), ("1".into(), "2".into()));
        assert_eq!((engine.row_text(0), engine.row_text(1)), ("3".into(), "4".into()));
    }

    #[test]
    fn a_wide_character_reads_once_and_links_come_back() {
        let mut engine = WeztermEngine::new(20, 2, 0);
        engine.feed("日\u{1b}]8;;http://x.y\u{7}ab\u{1b}]8;;\u{7}c".as_bytes());
        assert_eq!(engine.row_text(0), "日abc");
        assert_eq!((engine.cell(0, 0).width, engine.cell(0, 1).width), (2, 0));
        assert_eq!(engine.cell(0, 2).link.as_deref(), Some("http://x.y"));
        assert_eq!(engine.cell(0, 4).link, None);
    }
}
