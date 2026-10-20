//! Making a terminal: [`Terminal::new`] for the defaults, [`Terminal::builder`] to choose.
//!
//! Everything a host decides once, before the first byte, is an option here; what can change while
//! the terminal runs (the size, the palette, the cell size, the word separators, the policy) also
//! has a setter on [`Terminal`]. The builder can grow without breaking anyone: a new option is a
//! new method with a default.

use super::{
    Cluster, CursorPos, CursorStyle, Event, Level, Limits, Margins, Parser, Policy, Saved, Terminal,
    dispatch, graphics, history_bytes, osc, read, reports, screen, text, watch,
};
use crate::grid::{BlockPart, Grid, Pen};
use crate::passthrough::{Passthrough, PassthroughError};
use crate::terminal::modes::Modes;
use crate::types::{COLOR_COUNT, ColorScheme, Rgb};

/// The history a terminal keeps unless the builder says otherwise, in lines.
const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

/// The longest name or version XTVERSION says, each, in bytes. They come from the host, but a
/// reply is a sequence the program reads: it must stay short and free of controls.
const IDENTITY_PART_MAX: usize = 64;

/// Options for a [`Terminal`], from [`Terminal::builder`].
///
/// ```
/// use wormhole_vt::{Policy, Terminal};
///
/// let mut policy = Policy::default();
/// policy.window_ops = true;
/// let term = Terminal::builder()
///     .size(120, 40)
///     .scrollback(50_000)
///     .identity("myterm", "1.2.3")
///     .policy(policy)
///     .cell_size(9, 18)
///     .build();
/// assert_eq!((term.cols(), term.rows()), (120, 40));
/// ```
#[derive(Clone, Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct Builder {
    cols: usize,
    rows: usize,
    scrollback: usize,
    identity: String,
    policy: Policy,
    limits: Limits,
    archive: bool,
    palette: Option<Box<[Rgb; COLOR_COUNT]>>,
    cell: (u16, u16),
    separators: Option<String>,
    passthrough: Passthrough,
    scheme: Option<ColorScheme>,
}

impl Default for Builder {
    fn default() -> Builder {
        Builder {
            cols: 80,
            rows: 24,
            scrollback: DEFAULT_SCROLLBACK_LINES,
            identity: identity_text("wormhole-vt", env!("CARGO_PKG_VERSION")),
            policy: Policy::default(),
            limits: Limits::default(),
            archive: false,
            palette: None,
            cell: (8, 16),
            separators: None,
            passthrough: Passthrough::new(),
            scheme: None,
        }
    }
}

/// `name version` as XTVERSION answers it, cut and with the control characters left out.
fn identity_text(name: &str, version: &str) -> String {
    let clean = |part: &str| {
        let mut out = String::new();
        for c in part.chars().filter(|c| !c.is_control()) {
            if out.len() + c.len_utf8() > IDENTITY_PART_MAX {
                break;
            }
            out.push(c);
        }
        out
    };
    format!("{} {}", clean(name), clean(version))
}

impl Builder {
    /// The size in columns and rows (80 by 24 unless set). What the grid cannot hold is clamped:
    /// 2 to 16,384 columns, 1 to 16,384 rows.
    pub fn size(mut self, cols: usize, rows: usize) -> Builder {
        self.cols = cols;
        self.rows = rows;
        self
    }

    /// The lines of history to keep (10,000 unless set; 0 for none). It is a floor: the grid
    /// frees whole pages, so up to a few hundred more may be kept, see [`Terminal::scrollback_lines`].
    pub fn scrollback(mut self, lines: usize) -> Builder {
        self.scrollback = lines;
        self
    }

    /// The name and version XTVERSION (`CSI > q`) answers, `wormhole-vt` and this crate's version
    /// unless set. Control characters are left out and each part is cut to 64 bytes.
    pub fn identity(mut self, name: &str, version: &str) -> Builder {
        self.identity = identity_text(name, version);
        self
    }

    /// What programs may do besides print; everything is off unless set, see [`Policy`].
    pub fn policy(mut self, policy: Policy) -> Builder {
        self.policy = policy;
        self
    }

    /// The caps on what a program's sequences make the terminal hold (the built-in ceilings unless
    /// set); each value can only be lowered, see [`Limits`].
    pub fn limits(mut self, limits: Limits) -> Builder {
        self.limits = limits.clamped();
        self
    }

    /// Hand the host the history lines that leave the engine as
    /// [`Event::HistoryDropped`](crate::Event::HistoryDropped) (off unless set); see
    /// [`Terminal::set_archive_history`].
    pub fn archive_history(mut self, on: bool) -> Builder {
        self.archive = on;
        self
    }

    /// The host's colours, which OSC 4, 10, 11 and 12 queries are answered with until a program
    /// changes the entry; see [`Terminal::set_default_colors`].
    pub fn palette(mut self, palette: &[Rgb; COLOR_COUNT]) -> Builder {
        self.palette = Some(Box::new(*palette));
        self
    }

    /// The size of a cell in pixels (8 by 16 unless set), for the window and graphics reports;
    /// see [`Terminal::set_cell_size`].
    pub fn cell_size(mut self, width: u16, height: u16) -> Builder {
        self.cell = (width, height);
        self
    }

    /// The characters that end a word for a double click ([`DEFAULT_WORD_SEPARATORS`](crate::DEFAULT_WORD_SEPARATORS) unless
    /// set); see [`Terminal::set_word_separators`].
    pub fn word_separators(mut self, separators: &str) -> Builder {
        self.separators = Some(separators.to_string());
        self
    }

    /// Whether the host draws a light or a dark scheme, for programs that ask (`CSI ? 996 n`);
    /// unknown, and so unanswered, unless set. See [`Terminal::set_color_scheme`], which also tells
    /// the programs that asked to hear of changes.
    pub fn color_scheme(mut self, scheme: ColorScheme) -> Builder {
        self.scheme = Some(scheme);
        self
    }

    /// The OSC, DCS and APC strings to hand to the host as events instead of dropping them (none
    /// unless set); see [`passthrough`](crate::passthrough). Refused, with the reason, when the
    /// engine reads one of them itself or one cannot match anything.
    pub fn passthrough(mut self, passthrough: Passthrough) -> Result<Builder, PassthroughError> {
        passthrough.check()?;
        self.passthrough = passthrough;
        Ok(self)
    }

    /// Make the terminal.
    pub fn build(self) -> Terminal {
        let grid = Grid::new(self.cols, self.rows, history_bytes(self.scrollback, self.cols));
        let (cols, rows) = (grid.cols(), grid.rows());
        let mut term = Terminal {
            other: Grid::new(cols, rows, 0),
            grid,
            cols,
            rows,
            cursor: CursorPos::default(),
            pen: Pen::default(),
            modes: Modes::INITIAL,
            recorded: 0,
            saved_modes: Vec::new(),
            margins: Margins::full(cols, rows),
            tabs: screen::tabs::TabStops::new(cols),
            charsets: text::charset::Charsets::default(),
            locator: Default::default(),
            preferred: text::charset::Preferred::default(),
            saved: [Saved::default(), Saved::default()],
            keys: Default::default(),
            cursor_style: CursorStyle::default(),
            titles: reports::title::Titles::default(),
            policy: self.policy,
            limits: self.limits,
            window: None,
            folder: osc::cwd::Folder::default(),
            palette: osc::colors::Palette::new(),
            cell_px: (self.cell.0.max(1), self.cell.1.max(1)),
            last_char: None,
            cluster: Cluster {
                open: false,
                head: 0,
                width: 0,
                last: ' ',
                state: crate::unicode::GraphemeState::new(),
            },
            block: 0,
            stashed_content: BlockPart::None,
            link_seq: 0,
            link_text: osc::link::LinkText::default(),
            identity: self.identity,
            parser: Parser::new(),
            osc: osc::OscString::default(),
            dcs: dispatch::dcs::DcsString::default(),
            passthrough: Passthrough::new(),
            graphics: graphics::Graphics::default(),
            events: Vec::<Event>::new(),
            queued_values: 0,
            replies: Vec::new(),
            reply_dropped: false,
            scrollback_lines: self.scrollback,
            history_just_cleared: false,
            kept: Default::default(),
            level: Level::Vt500,
            eight_bit: false,
            guard_used: false,
            attr_extent: screen::rect::AttrExtent::default(),
            select: read::selection::Select::default(),
            scheme: self.scheme,
            seen: read::damage::Seen::new(),
            told: watch::Told::new(),
        };
        if let Some(palette) = &self.palette {
            term.set_default_colors(palette);
        }
        if let Some(separators) = &self.separators {
            term.set_word_separators(separators);
        }
        term.osc.set_claims(self.passthrough.osc_caps());
        term.passthrough = self.passthrough;
        term.apply_limits();
        if self.archive {
            term.set_archive_history(true);
        }
        term
    }
}

impl Terminal {
    /// A blank terminal of `cols` by `rows` (clamped to what the grid allows) with the defaults:
    /// 10,000 lines of history, nothing a program could abuse switched on. Use
    /// [`builder`](Self::builder) to choose.
    pub fn new(cols: usize, rows: usize) -> Terminal {
        Builder::default().size(cols, rows).build()
    }

    /// Change which strings are handed to the host, see [`Builder::passthrough`]. A string being
    /// read at the time keeps the limit it started with. Refused, with nothing changed, when the
    /// engine reads one of them itself or one cannot match anything.
    pub fn set_passthrough(&mut self, passthrough: Passthrough) -> Result<(), PassthroughError> {
        passthrough.check()?;
        self.osc.set_claims(passthrough.osc_caps());
        self.passthrough = passthrough;
        Ok(())
    }

    /// Options for a terminal; see [`Builder`].
    pub fn builder() -> Builder {
        Builder::default()
    }
}
