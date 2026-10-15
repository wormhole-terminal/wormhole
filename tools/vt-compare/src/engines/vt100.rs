//! The `vt100` crate (crates.io, 0.16): a terminal parser that keeps the screen as cells, used by
//! tools that run other terminal programs (screen and tmux like).
//!
//! It has no output channel, so it answers no query; it has no strike, blink, overline, hyperlink
//! or underline style or color; and it keeps its history behind a scroll offset, which is moved to
//! read a line and put back.

use std::cell::RefCell;

use wormhole_vt_bench::{Attrs, Cell, CellColor, Cursor, Engine};

/// Collects what the screen alone cannot tell: the window title.
#[derive(Default)]
struct Title(Option<String>);

impl vt100::Callbacks for Title {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        let title = String::from_utf8_lossy(title).into_owned();
        self.0 = (!title.is_empty()).then_some(title);
    }
}

/// The `vt100` crate's parser and screen.
pub struct Vt100Engine {
    // Reading history moves the scroll offset, which takes `&mut` on a `&self` read.
    parser: RefCell<vt100::Parser<Title>>,
}

impl Vt100Engine {
    /// `read` on the screen as it is `offset` lines scrolled back into history, then put back.
    fn at_offset<T>(&self, offset: usize, read: impl FnOnce(&vt100::Screen) -> T) -> T {
        let mut parser = self.parser.borrow_mut();
        let screen = parser.screen_mut();
        let before = screen.scrollback();
        screen.set_scrollback(offset);
        let out = read(screen);
        screen.set_scrollback(before);
        out
    }

    fn history_len(&self) -> usize {
        self.at_offset(usize::MAX, |screen| screen.scrollback())
    }

    /// The text of screen row `row` as seen through `screen` (wide characters once).
    fn text_of_row(screen: &vt100::Screen, row: u16) -> String {
        let mut text = Self::text_of_row_raw(screen, row);
        text.truncate(text.trim_end_matches(' ').len());
        text
    }

    /// [`Self::text_of_row`] with the trailing blanks kept.
    fn text_of_row_raw(screen: &vt100::Screen, row: u16) -> String {
        let (_, cols) = screen.size();
        let mut text = String::new();
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else { continue };
            if cell.is_wide_continuation() {
                continue;
            }
            text.push_str(if cell.has_contents() { cell.contents() } else { " " });
        }
        text
    }
}

fn color(color: vt100::Color) -> CellColor {
    match color {
        vt100::Color::Default => CellColor::Default,
        vt100::Color::Idx(i) => CellColor::Indexed(i),
        vt100::Color::Rgb(r, g, b) => CellColor::Rgb(r, g, b),
    }
}

impl Engine for Vt100Engine {
    const NAME: &'static str = "vt100";

    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let parser =
            vt100::Parser::new_with_callbacks(rows as u16, cols as u16, scrollback, Title::default());
        Vt100Engine { parser: RefCell::new(parser) }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.parser.get_mut().process(bytes);
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.parser.get_mut().screen_mut().set_size(rows as u16, cols as u16);
    }

    fn size(&self) -> (usize, usize) {
        let (rows, cols) = self.parser.borrow().screen().size();
        (usize::from(cols), usize::from(rows))
    }

    fn row_text(&self, row: usize) -> String {
        Self::text_of_row(self.parser.borrow().screen(), row as u16)
    }

    fn row_text_raw(&self, row: usize) -> String {
        Self::text_of_row_raw(self.parser.borrow().screen(), row as u16)
    }

    fn row_wrapped(&self, row: usize) -> bool {
        self.parser.borrow().screen().row_wrapped(row as u16)
    }

    fn cell(&self, row: usize, col: usize) -> Cell {
        let parser = self.parser.borrow();
        let Some(cell) = parser.screen().cell(row as u16, col as u16) else { return Cell::blank() };
        let mut attrs = Attrs::empty();
        for (on, attr) in [
            (cell.bold(), Attrs::BOLD),
            (cell.dim(), Attrs::DIM),
            (cell.italic(), Attrs::ITALIC),
            (cell.underline(), Attrs::UNDERLINE),
            (cell.inverse(), Attrs::INVERSE),
        ] {
            if on {
                attrs |= attr;
            }
        }
        let wide_continuation = cell.is_wide_continuation();
        Cell {
            text: if wide_continuation {
                String::new()
            } else if cell.has_contents() {
                cell.contents().to_string()
            } else {
                " ".to_string()
            },
            width: if wide_continuation {
                0
            } else if cell.is_wide() {
                2
            } else {
                1
            },
            fg: color(cell.fgcolor()),
            bg: color(cell.bgcolor()),
            underline_color: CellColor::Default,
            attrs,
            link: None,
        }
    }

    fn cursor(&self) -> Cursor {
        let parser = self.parser.borrow();
        let screen = parser.screen();
        let (row, col) = screen.cursor_position();
        // A cursor waiting to wrap is at one past the last column in this model; on the screen it
        // is on the last column.
        let (_, cols) = screen.size();
        Cursor {
            row: usize::from(row),
            col: usize::from(col.min(cols.saturating_sub(1))),
            visible: !screen.hide_cursor(),
        }
    }

    fn scrollback_len(&self) -> usize {
        self.history_len()
    }

    fn scrollback_text(&self, index: usize) -> String {
        // Scrolled back by `n - index` lines, the top row of the screen is history line `index`.
        let offset = self.history_len().saturating_sub(index);
        self.at_offset(offset, |screen| Self::text_of_row(screen, 0))
    }

    fn scrollback_text_raw(&self, index: usize) -> String {
        let offset = self.history_len().saturating_sub(index);
        self.at_offset(offset, |screen| Self::text_of_row_raw(screen, 0))
    }

    fn scrollback_wrapped(&self, index: usize) -> bool {
        let offset = self.history_len().saturating_sub(index);
        self.at_offset(offset, |screen| screen.row_wrapped(0))
    }

    fn take_replies(&mut self) -> Vec<u8> {
        Vec::new()
    }

    fn title(&self) -> Option<String> {
        self.parser.borrow().callbacks().0.clone()
    }

    fn alt_screen(&self) -> bool {
        self.parser.borrow().screen().alternate_screen()
    }
}

#[cfg(test)]
mod tests {
    use wormhole_vt_bench::{CellColor, Engine};

    use super::Vt100Engine;

    #[test]
    fn text_colors_and_cursor() {
        let mut engine = Vt100Engine::new(10, 3, 10);
        engine.feed(b"\x1b[31mhi\x1b[0m there");
        assert_eq!(engine.row_text(0), "hi there");
        assert_eq!(engine.cell(0, 0).fg, CellColor::Indexed(1));
        let cursor = engine.cursor();
        assert_eq!((cursor.row, cursor.col, cursor.visible), (0, 8, true));
    }

    #[test]
    fn history_is_read_oldest_first_and_the_screen_is_left_alone() {
        let mut engine = Vt100Engine::new(5, 2, 10);
        engine.feed(b"1\r\n2\r\n3\r\n4");
        assert_eq!(engine.scrollback_len(), 2);
        assert_eq!((engine.scrollback_text(0), engine.scrollback_text(1)), ("1".into(), "2".into()));
        assert_eq!((engine.row_text(0), engine.row_text(1)), ("3".into(), "4".into()));
    }

    #[test]
    fn the_title_is_kept_and_a_wide_character_reads_once() {
        let mut engine = Vt100Engine::new(10, 2, 0);
        engine.feed("\u{1b}]2;hello\u{7}日本".as_bytes());
        assert_eq!(engine.title().as_deref(), Some("hello"));
        assert_eq!(engine.row_text(0), "日本");
        assert_eq!((engine.cell(0, 0).width, engine.cell(0, 1).width), (2, 0));
    }

    #[test]
    fn it_answers_nothing() {
        let mut engine = Vt100Engine::new(10, 2, 0);
        engine.feed(b"\x1b[5n\x1b[c");
        assert!(engine.take_replies().is_empty());
    }
}
