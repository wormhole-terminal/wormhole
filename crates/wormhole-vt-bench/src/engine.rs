//! The adapter every tool talks to a terminal engine through.
//!
//! An adapter wraps one engine (`wormhole-vt`'s terminal, or another terminal's in
//! `tools/vt-compare`) and answers the questions the conformance cases, the differential harness and
//! vtbench ask. It owns the whole path from bytes to screen: the parser, the screen and a place to
//! collect what the terminal replies, so a reply can be compared byte for byte.
//!
//! All coordinates are 0-based here (`row 0` is the top line); the `.vtt` format counts from 1 as
//! the escape sequences do and converts.

use bitflags::bitflags;

bitflags! {
    /// The attributes of a cell as a program asked for them (SGR), not as they are drawn:
    /// `INVERSE` is a flag, the colors are not swapped.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Attrs: u16 {
        const BOLD = 1 << 0;
        const DIM = 1 << 1;
        const ITALIC = 1 << 2;
        const UNDERLINE = 1 << 3;
        const DOUBLE_UNDERLINE = 1 << 4;
        const CURLY_UNDERLINE = 1 << 5;
        const DOTTED_UNDERLINE = 1 << 6;
        const DASHED_UNDERLINE = 1 << 7;
        const BLINK = 1 << 8;
        const INVERSE = 1 << 9;
        const HIDDEN = 1 << 10;
        const STRIKE = 1 << 11;
        const OVERLINE = 1 << 12;
    }
}

impl Attrs {
    /// Every name the `.vtt` format uses, in the order they are printed.
    pub const NAMES: [(&'static str, Attrs); 13] = [
        ("bold", Attrs::BOLD),
        ("dim", Attrs::DIM),
        ("italic", Attrs::ITALIC),
        ("underline", Attrs::UNDERLINE),
        ("double-underline", Attrs::DOUBLE_UNDERLINE),
        ("curly-underline", Attrs::CURLY_UNDERLINE),
        ("dotted-underline", Attrs::DOTTED_UNDERLINE),
        ("dashed-underline", Attrs::DASHED_UNDERLINE),
        ("blink", Attrs::BLINK),
        ("inverse", Attrs::INVERSE),
        ("hidden", Attrs::HIDDEN),
        ("strike", Attrs::STRIKE),
        ("overline", Attrs::OVERLINE),
    ];

    /// `bold,italic`, or `none`.
    pub fn describe(self) -> String {
        let names: Vec<&str> =
            Self::NAMES.iter().filter(|(_, a)| self.contains(*a)).map(|(n, _)| *n).collect();
        if names.is_empty() { "none".to_string() } else { names.join(",") }
    }
}

/// A color as the program set it: the palette slot or the RGB value, not what a theme makes of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CellColor {
    /// The terminal's default foreground, background (or underline: the text's own color).
    #[default]
    Default,
    /// Palette slot 0-255: SGR 30-37 are 0-7, 90-97 are 8-15, `38;5;n` is n.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl CellColor {
    /// `default`, `idx:4` or `rgb:0a141e`, as the `.vtt` format writes it.
    pub fn describe(self) -> String {
        match self {
            CellColor::Default => "default".to_string(),
            CellColor::Indexed(i) => format!("idx:{i}"),
            CellColor::Rgb(r, g, b) => format!("rgb:{r:02x}{g:02x}{b:02x}"),
        }
    }
}

/// One cell of the visible screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The cell's text: a blank is `" "`, a letter with combining marks (or an emoji sequence
    /// kept together) is the whole cluster, and the second half of a wide character is `""`.
    pub text: String,
    /// Columns the cell stands for: 1, 2 for the first half of a wide character, 0 for its
    /// second half.
    pub width: u8,
    pub fg: CellColor,
    pub bg: CellColor,
    pub underline_color: CellColor,
    pub attrs: Attrs,
    /// The OSC 8 hyperlink target, if the cell is part of one.
    pub link: Option<String>,
}

impl Cell {
    /// A blank cell with no attributes, as an untouched screen has.
    pub fn blank() -> Cell {
        Cell {
            text: " ".to_string(),
            width: 1,
            fg: CellColor::Default,
            bg: CellColor::Default,
            underline_color: CellColor::Default,
            attrs: Attrs::empty(),
            link: None,
        }
    }
}

/// Where the cursor is and whether it is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
}

/// The size in pixels an adapter reports a cell to have, for the replies that tell a program the
/// window's size (`CSI 14 t`, `CSI 16 t`).
pub const CELL_PIXELS: (usize, usize) = (8, 16);

/// The colors an adapter answers color queries (OSC 4, 10, 11, 12 with `?`) with when the
/// program has not changed them, so the replies are the same for every engine: foreground,
/// background, cursor. The palette is xterm's (`engines::xterm_palette`).
pub const DEFAULT_COLORS: [(u8, u8, u8); 3] = [(0xdd, 0xdd, 0xdd), (0x1e, 0x1e, 0x1e), (0xdd, 0xdd, 0xdd)];

/// A terminal engine under test. Created with a size, fed bytes, read back.
///
/// Rules every adapter follows:
/// - `feed` returns when every byte is applied: nothing stays buffered (synchronized output,
///   mode 2026, included), and any bytes the terminal wants to send back are queued for
///   [`Engine::take_replies`] in the order the engine produced them;
/// - requests the host would answer (color queries, the window size) are answered with fixed
///   values: [`DEFAULT_COLORS`], [`CELL_PIXELS`] (colors the program changed excepted); the answer
///   reflects the colors when `feed` returns, as some adapters can only know them then,
///   so a query followed by a change of the same color in one `feed` call is not portable;
/// - `row_text` is the row without trailing blanks, a wide character once (its second half
///   contributes nothing), a combining mark inside its cell's text;
/// - `row_text_raw` and `scrollback_text_raw` are the same text with the trailing blanks kept, so
///   a wrapped row can be joined to the next without losing a space at its end.
pub trait Engine: Sized {
    /// The name `known-fail:` lines in the `.vtt` files use.
    const NAME: &'static str;

    /// An engine with a `cols` x `rows` screen that keeps up to `scrollback` lines of history.
    fn new(cols: usize, rows: usize, scrollback: usize) -> Self;

    /// Turn on answering DECRQCRA, which engines that have it keep off until the host asks (it
    /// lets a program read the screen back). The default does nothing: an engine that has no
    /// such switch either answers or does not.
    fn allow_screen_checksum(&mut self) {}

    /// Turn on the window operations (move, resize, …), which engines that have them keep off
    /// until the host asks. The default does nothing.
    fn allow_window_ops(&mut self) {}

    /// Turn on answering the title and icon name reports (XTWINOPS 20 and 21), which engines that
    /// have them keep off until the host asks. The default does nothing.
    fn allow_title_report(&mut self) {}

    /// Tell the engine the host's color scheme (`dark`, else light), for the engines that answer
    /// `CSI ? 996 n`. The default does nothing.
    fn set_color_scheme(&mut self, _dark: bool) {}

    fn feed(&mut self, bytes: &[u8]);

    /// Resize the screen as a window resize does (the content reflows as the engine does).
    fn resize(&mut self, cols: usize, rows: usize);

    /// `(cols, rows)`.
    fn size(&self) -> (usize, usize);

    /// Row `row` of the visible screen (0 = top).
    fn row_text(&self, row: usize) -> String;

    /// [`Engine::row_text`] with the trailing blanks kept (a blank before a wrap is text the
    /// program wrote). The default is the trimmed text, so an adapter that does not override it
    /// loses such a blank when wrapped rows are joined; the cells cannot tell, as a spacer head
    /// (the blank a wide character left at a row's end) must contribute nothing.
    fn row_text_raw(&self, row: usize) -> String {
        self.row_text(row)
    }

    /// Whether row `row` continues in the next row because the text wrapped (not because the
    /// program ended the line).
    fn row_wrapped(&self, row: usize) -> bool;

    /// How big row `row` is drawn (DECSWL, DECDWL, DECDHL): `single`, `double-width`,
    /// `double-height-top` or `double-height-bottom`; `None` for an engine that has no line sizes.
    fn row_line_size(&self, _row: usize) -> Option<&'static str> {
        None
    }

    fn cell(&self, row: usize, col: usize) -> Cell;

    fn cursor(&self) -> Cursor;

    /// Lines in the history (not counting the visible screen).
    fn scrollback_len(&self) -> usize;

    /// History line `index`, 0 being the oldest; same rules as [`Engine::row_text`].
    fn scrollback_text(&self, index: usize) -> String;

    /// [`Engine::scrollback_text`] with the trailing blanks kept; the default is the trimmed text,
    /// with the same consequence as for [`Engine::row_text_raw`].
    fn scrollback_text_raw(&self, index: usize) -> String {
        self.scrollback_text(index)
    }

    /// Whether history line `index` continues in the next line (the one after it, or the first
    /// visible row for the newest) because the text wrapped.
    fn scrollback_wrapped(&self, index: usize) -> bool;

    /// The bytes the terminal sent to the program since the last call.
    fn take_replies(&mut self) -> Vec<u8>;

    /// The window title the program set (OSC 0 or 2), `None` when it never did or reset it.
    fn title(&self) -> Option<String>;

    /// Whether the alternate screen is showing.
    fn alt_screen(&self) -> bool;
}
