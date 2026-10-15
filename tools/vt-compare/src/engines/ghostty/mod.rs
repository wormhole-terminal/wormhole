//! Ghostty's terminal, through libghostty-vt's C API (`tools/vt-compare/fetch-ghostty.sh` builds the
//! library; `ffi` is the only code that calls it, through `shim/ghostty_shim.c`, written against its
//! headers).
//!
//! libghostty-vt is Ghostty's own engine without the app around it: the same parser, screen, pages
//! and reflow. The host answers what a window would: the cell size, the device attributes and the
//! colors, set to the fixed values the other adapters use.

mod ffi;

use std::sync::OnceLock;

use wormhole_vt_bench::engine::DEFAULT_COLORS;
use wormhole_vt_bench::engines::xterm_palette;
use wormhole_vt_bench::{Attrs, Cell, CellColor, Cursor, Engine};

use super::Region;

pub use ffi::alloc_counts;

/// xterm's palette as flat RGB bytes, the form the shim hands to the library.
fn palette_bytes() -> &'static [u8; 768] {
    static PALETTE: OnceLock<[u8; 768]> = OnceLock::new();
    PALETTE.get_or_init(|| {
        let mut bytes = [0u8; 768];
        for i in 0..=255u8 {
            let (r, g, b) = xterm_palette(i);
            bytes[i as usize * 3..i as usize * 3 + 3].copy_from_slice(&[r, g, b]);
        }
        bytes
    })
}

fn rgb((r, g, b): (u8, u8, u8)) -> [u8; 3] {
    [r, g, b]
}

/// A Ghostty terminal. Reads go through the library's grid references, which look a cell up by
/// its place each time: slow for rendering, which is not what is measured here, and exact.
pub struct GhosttyEngine {
    term: ffi::Terminal,
    cols: usize,
    rows: usize,
}

impl GhosttyEngine {
    fn text_of(cell: &ffi::Cell) -> String {
        let n = (cell.n_codepoints as usize).min(ffi::MAX_CODEPOINTS);
        cell.codepoints[..n].iter().filter_map(|&c| char::from_u32(c)).collect()
    }

    /// A row of the screen or the history as text: wide characters once, no trailing blanks.
    fn line_text(&self, region: Region, y: usize) -> String {
        let mut text = self.line_text_raw(region, y);
        text.truncate(text.trim_end_matches(' ').len());
        text
    }

    /// [`Self::line_text`] with the trailing blanks kept.
    fn line_text_raw(&self, region: Region, y: usize) -> String {
        let mut text = String::new();
        for x in 0..self.cols {
            let Some(cell) = self.term.cell(region, x, y) else { continue };
            if cell.width == 0 || cell.spacer_head != 0 {
                continue;
            }
            if cell.n_codepoints == 0 {
                text.push(' ');
            } else {
                text.push_str(&Self::text_of(&cell));
            }
        }
        text
    }
}

fn color(kind: u8, rgb: [u8; 3]) -> CellColor {
    match kind {
        ffi::COLOR_PALETTE => CellColor::Indexed(rgb[0]),
        ffi::COLOR_RGB => CellColor::Rgb(rgb[0], rgb[1], rgb[2]),
        _ => CellColor::Default,
    }
}

fn attrs(bits: u16) -> Attrs {
    let mut out = Attrs::empty();
    for (bit, attr) in [
        (ffi::attr::BOLD, Attrs::BOLD),
        (ffi::attr::DIM, Attrs::DIM),
        (ffi::attr::ITALIC, Attrs::ITALIC),
        (ffi::attr::BLINK, Attrs::BLINK),
        (ffi::attr::INVERSE, Attrs::INVERSE),
        (ffi::attr::HIDDEN, Attrs::HIDDEN),
        (ffi::attr::STRIKE, Attrs::STRIKE),
        (ffi::attr::OVERLINE, Attrs::OVERLINE),
        (ffi::attr::UNDERLINE, Attrs::UNDERLINE),
        (ffi::attr::DOUBLE_UNDERLINE, Attrs::DOUBLE_UNDERLINE),
        (ffi::attr::CURLY_UNDERLINE, Attrs::CURLY_UNDERLINE),
        (ffi::attr::DOTTED_UNDERLINE, Attrs::DOTTED_UNDERLINE),
        (ffi::attr::DASHED_UNDERLINE, Attrs::DASHED_UNDERLINE),
    ] {
        if bits & bit != 0 {
            out |= attr;
        }
    }
    out
}

impl Engine for GhosttyEngine {
    const NAME: &'static str = "ghostty";

    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        // The shim reports the cell size the other adapters do (8 x 16 pixels).
        let term = ffi::Terminal::new(
            cols,
            rows,
            scrollback,
            palette_bytes(),
            rgb(DEFAULT_COLORS[0]),
            rgb(DEFAULT_COLORS[1]),
            rgb(DEFAULT_COLORS[2]),
        );
        let Some(term) = term else { panic!("libghostty-vt could not make a {cols}x{rows} terminal") };
        GhosttyEngine { term, cols, rows }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.term.feed(bytes);
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        if self.term.resize(cols, rows) {
            self.cols = cols;
            self.rows = rows;
        }
    }

    fn size(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    fn row_text(&self, row: usize) -> String {
        self.line_text(Region::Screen, row)
    }

    fn row_text_raw(&self, row: usize) -> String {
        self.line_text_raw(Region::Screen, row)
    }

    fn row_wrapped(&self, row: usize) -> bool {
        self.term.row_wrapped(Region::Screen, row)
    }

    fn cell(&self, row: usize, col: usize) -> Cell {
        let Some(cell) = self.term.cell(Region::Screen, col, row) else { return Cell::blank() };
        let text = if cell.width == 0 {
            String::new()
        } else if cell.n_codepoints == 0 {
            " ".to_string()
        } else {
            Self::text_of(&cell)
        };
        Cell {
            text,
            width: cell.width,
            fg: color(cell.fg_kind, cell.fg),
            bg: color(cell.bg_kind, cell.bg),
            underline_color: color(cell.ul_kind, cell.ul),
            attrs: attrs(cell.attrs),
            link: self.term.link(Region::Screen, col, row),
        }
    }

    fn cursor(&self) -> Cursor {
        let (col, row, visible) = self.term.cursor();
        Cursor { row, col, visible }
    }

    fn scrollback_len(&self) -> usize {
        self.term.scrollback_rows()
    }

    fn scrollback_text(&self, index: usize) -> String {
        self.line_text(Region::History, index)
    }

    fn scrollback_text_raw(&self, index: usize) -> String {
        self.line_text_raw(Region::History, index)
    }

    fn scrollback_wrapped(&self, index: usize) -> bool {
        self.term.row_wrapped(Region::History, index)
    }

    fn take_replies(&mut self) -> Vec<u8> {
        self.term.take_replies()
    }

    fn title(&self) -> Option<String> {
        self.term.title()
    }

    fn alt_screen(&self) -> bool {
        self.term.alt_screen()
    }
}

#[cfg(test)]
mod tests {
    use wormhole_vt_bench::{Attrs, CellColor, Engine};

    use super::GhosttyEngine;

    #[test]
    fn text_colors_and_the_cursor_come_back() {
        let mut engine = GhosttyEngine::new(10, 3, 100);
        engine.feed(b"\x1b[1;31mhi\x1b[0m\r\nthere");
        assert_eq!(engine.row_text(0), "hi");
        assert_eq!(engine.row_text(1), "there");
        let cell = engine.cell(0, 0);
        assert_eq!((cell.text.as_str(), cell.fg, cell.attrs), ("h", CellColor::Indexed(1), Attrs::BOLD));
        let cursor = engine.cursor();
        assert_eq!((cursor.row, cursor.col, cursor.visible), (1, 5, true));
    }

    #[test]
    fn replies_are_taken_once_in_order() {
        let mut engine = GhosttyEngine::new(10, 3, 0);
        engine.feed(b"\x1b[5n\x1b[6n");
        assert_eq!(engine.take_replies(), b"\x1b[0n\x1b[1;1R");
        assert!(engine.take_replies().is_empty());
    }

    #[test]
    fn history_is_read_oldest_first() {
        let mut engine = GhosttyEngine::new(5, 2, 100);
        engine.feed(b"1\r\n2\r\n3\r\n4");
        assert_eq!(engine.scrollback_len(), 2);
        assert_eq!((engine.scrollback_text(0), engine.scrollback_text(1)), ("1".into(), "2".into()));
        assert_eq!(engine.row_text(0), "3");
    }

    #[test]
    fn the_title_and_links_are_answered() {
        let mut engine = GhosttyEngine::new(20, 2, 0);
        assert_eq!(engine.title(), None);
        engine.feed(b"\x1b]2;hello\x07\x1b]8;;http://x.y\x07ab\x1b]8;;\x07c");
        assert_eq!(engine.title().as_deref(), Some("hello"));
        assert_eq!(engine.cell(0, 0).link.as_deref(), Some("http://x.y"));
        assert_eq!(engine.cell(0, 2).link, None);
    }

    #[test]
    fn the_allocator_counts_what_the_library_asks_for() {
        let (before, _) = super::alloc_counts();
        let mut engine = GhosttyEngine::new(80, 24, 1000);
        engine.feed(&b"line of text\r\n".repeat(2000));
        let (after, bytes) = super::alloc_counts();
        assert!(after > before && bytes > 0);
    }
}
