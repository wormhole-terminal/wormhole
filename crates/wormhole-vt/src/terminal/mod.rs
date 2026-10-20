//! The terminal state: what the escape sequences the parser finds *mean*.
//!
//! [`Terminal`] owns the screen (a grid for the primary and one for the alternate screen), the
//! cursor and its pen, the modes, the scrolling region and margins, tab stops, character sets,
//! the palette a program changed, the kitty keyboard flags, the title and the parser that reads
//! the program's bytes. Feeding it is `terminal.feed(bytes)`: the parser keeps the state of a
//! half-read sequence, the terminal keeps the state of the screen, and neither is shared with a
//! thread or a lock here (the host decides how they are guarded).
//!
//! What the program asks for comes out in two queues: [`replies`](Terminal::replies), the bytes to
//! write to the PTY (DSR, DA, DECRQM, … in the order they were asked), and
//! [`events`](Terminal::events), the title, bell, notifications, clipboard requests, shell marks
//! and the pictures to decode. Neither allocates on the hot path.
//!
//! # Where each sequence is
//!
//! | folder | file | what |
//! |--------|------|------|
//! | `dispatch/` | `handler` | the private adapter the parser calls into |
//! | | `control`, `esc`, `csi` | the dispatch of controls, ESC and CSI finals |
//! | | `dcs` | device control strings and APC: collecting them and sending them on (sixel is read here) |
//! | `text/` | `print` | text: ASCII runs, wide characters, marks, grapheme clusters (mode 2027), REP, IRM |
//! | | `charset` | the sets G0..G3 hold: national replacement sets, DEC Technical and Supplemental, Latin-1; and the shifts |
//! | | `sgr` | SGR and the colors it sets |
//! | `screen/` | `cursor` | movement, margins, origin mode, tabs, DECSC/DECRC |
//! | | `edit` | ED, EL, ECH, ICH, DCH, IL, DL, DECIC, DECDC, DECSED, DECSEL |
//! | | `rect` | DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA, DECSACE |
//! | | `scroll` | IND, RI, NEL, SU, SD, DECSTBM, DECSLRM, DECBI, DECFI |
//! | | `tabs` | the tab stops themselves |
//! | | `altscreen` | 47, 1047, 1048, 1049 |
//! | | `resize`, `reset` | the window changing size, DECCOLM; RIS, DECSTR, DECALN |
//! | `modes/` | (`mod`) | the mode set and the numbers that name the modes |
//! | | `apply` | SM/RM, DECSET/DECRST, XTSAVE/XTRESTORE and what setting each does; DECRQM |
//! | | `conformance` | DECSCL, S7C1T and S8C1T: the level and one-byte C1 controls |
//! | | `keyboard`, `kept` | the kitty keyboard stacks; numbers kept for hardware we do not have |
//! | `reports/` | (`mod`) | DSR, DA1/2/3, XTVERSION |
//! | | `state`, `presentation` | DECRQPSR (cursor information, tab stops), DECRQTSR, and DECRSPS and DECRSTS to put them back; DECRQDE; the user-preferred supplemental set |
//! | | `checksum` | DECRQCRA, and the policy that turns it on (off by default) |
//! | | `rqss`, `tcap` | DECRQSS; XTGETTCAP |
//! | | `window`, `title` | XTWINOPS: the window's reports and operations; the title, the icon name and their stack |
//! | | `locator` | the DEC locator: DECELR, DECSLE, DECRQLP, DECEFR and the reports the host's pointer events make |
//! | `osc/` | (`mod`), `dispatch` | operating system commands: collecting the string; the dispatch and the text helpers |
//! | | `cwd`, `link`, `clipboard`, `notify`, `marks` | OSC 7 (the folder), 8 (hyperlinks), 52 (clipboard), 9, 9;4, 99 and 777 (notifications, progress), 133 (shell marks) |
//! | | `colors`, `spec` | OSC 4, 5, 10 to 19 and their resets; the parser of color specs |
//! | | `base64` | base64 for the clipboard and the pictures |
//! | `graphics/` | | kitty's graphics protocol, Sixel and iTerm2's inline images, placed in the grid |
//! | `read/` | `snapshot` | what the host draws from |
//! | | `selection`, `search` | selecting, copying text, the text a search runs over, links and marks under a point |
//! | `repaint/` | | the VT bytes that rebuild this terminal in another one (a host's attach and resync), `fields` listing how each piece of state is covered |
//! | (top) | `event`, `policy`, `builder` | what the host hears; the risky switches; how a terminal is made |
//! | | `check`, `dump` | the invariants check (tests, and feature `internals`); the whole state as named text, for the tests that compare two terminals (feature `internals`) |
//!
//! # Choices
//!
//! - The wrap after the last column is *delayed* (the cursor stays on the last column with a
//!   "wrap pending" flag); anything that moves the cursor clears it (terminfo's `xenl`), and so
//!   does anything that edits the line under it (xterm: "with the right part cleared, we can't
//!   wrap"). HT, which never wraps, is not a move where it has nowhere to go: the wrap stays due
//!   (xterm's mode 41 makes it wrap first).
//! - Erased cells take the pen's background and nothing else (`bce`), also when the alternate
//!   screen is cleared on entry.
//! - A scroll up of the whole screen, or of a region from the top row to the bottom margin that
//!   spans the full width, feeds the primary screen's history; any other scroll (DL, IL, SD, a
//!   region that starts lower) does not. ED 2 moves the lines that hold something into the
//!   history first (as ^L has always left them to scroll back to) unless ED 3 came just before
//!   (macOS `clear`).
//! - Modes belong to the terminal, not to a screen (IRM set on the main screen is still set on
//!   the alternate one); the cursor, pen, character sets, margins and tab stops are shared by
//!   both screens, DECSC's saved state and the kitty keyboard flags are per screen (xterm).
//!   DECSC saves the hyperlink with the rendition.
//! - Mode 1007 (the wheel sends arrow keys on the alternate screen) starts set, as the app has
//!   always had it; the other modes start as xterm's do. DECSTR leaves autowrap on (ncurses sends
//!   it and expects lines to wrap). DECSCUSR 0 hands the cursor style back to the host.
//! - A title or folder that a program sends is kept here and announced by an event without a
//!   payload (`title()`, `cwd()`), so that a prompt that sets both costs no allocation; an empty
//!   title is no title.
//! - A hostile program cannot make the terminal grow without bound: strings are kept up to a limit
//!   per kind (a title 8 KiB, a clipboard 8 MiB, an image 64 MiB) and dropped whole beyond it,
//!   events and replies that nobody collects are capped, and every count is clamped to the screen.
//!   Pictures have their own caps (`graphics`).

mod check;
mod dispatch;
#[cfg(any(test, feature = "state-dump"))]
mod dump;
mod event;
mod graphics;
mod modes;
mod osc;
mod read;
pub mod repaint;
mod reports;
mod screen;
#[cfg(test)]
mod tests;
mod text;

use crate::grid::{Content, Grid, Pen, Region};
use crate::parser::Parser;
use crate::types::{COLOR_COUNT, Colors, CursorShape, Rgb};

use modes::conformance::Level;
use modes::keyboard::KeyboardFlags;
use screen::tabs::TabStops;
use text::charset::{Charsets, Preferred};

#[cfg(any(test, feature = "state-dump"))]
pub use dump::StateDump;
pub use event::{Event, Progress, ProgressState, PromptKind, ShellMark, Terminator};
pub use graphics::{
    Compression, IMAGE_BUDGET_BYTES, ImageError, ImageFormat, ImageUpload, MAX_IMAGES, PLACEHOLDER,
    Placeholder, VirtualPlacement, placeholder_at, strip_placeholders,
};
pub use modes::Modes;
pub use read::search::PinnedSpan;
pub use read::selection::DEFAULT_WORD_SEPARATORS;
pub use read::snapshot::TerminalCursor;
pub use reports::checksum::ScreenChecksum;
pub use reports::locator::{LocatorButton, Pointer};
pub use reports::window::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp, WindowOps};

/// The cursor's place. `pending` is the delayed wrap: the last character filled the row, the next
/// one wraps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Cursor {
    x: usize,
    y: usize,
    pending: bool,
}

/// The scrolling region (DECSTBM) and the left and right margins (DECSLRM), always inside the
/// screen and covering it whole until a program says otherwise. Edges included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Margins {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

impl Margins {
    fn full(cols: usize, rows: usize) -> Margins {
        Margins { top: 0, bottom: rows - 1, left: 0, right: cols - 1 }
    }
}

/// What DECSC saves (and DECRC brings back): VT510 DECSC lists the position, the rendition, the
/// character sets, the origin mode, the selective erase attribute and the wrap flag.
#[derive(Clone, Debug)]
struct Saved {
    cursor: Cursor,
    style: crate::grid::Style,
    protected: bool,
    origin: bool,
    charsets: Charsets,
    link: Option<crate::types::Hyperlink>,
}

impl Default for Saved {
    /// The state a terminal starts with: home, normal rendition, ASCII.
    fn default() -> Saved {
        Saved {
            cursor: Cursor::default(),
            style: crate::grid::Style::DEFAULT,
            protected: false,
            origin: false,
            charsets: Charsets::default(),
            link: None,
        }
    }
}

/// The cursor's look as DECSCUSR and mode 12 asked for it; `None` leaves each to the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CursorStyle {
    shape: Option<CursorShape>,
    blink: Option<bool>,
}

/// A grapheme cluster being built in mode 2027: where the last printed character stands, how wide
/// the cluster is so far, and the state of the UAX #29 automaton.
#[derive(Clone, Copy, Debug)]
struct Cluster {
    /// Whether the character before the cursor was printed by the last operation and can still
    /// be extended. Any sequence that is not text closes it.
    open: bool,
    /// The column of the cluster's first cell.
    head: usize,
    width: u8,
    last: char,
    state: crate::unicode::GraphemeState,
}

/// The longest title kept, in bytes. A program that sends more gets it cut (at a character).
const TITLE_MAX: usize = 4096;
/// The longest folder kept, in bytes.
const CWD_MAX: usize = 4096;
/// The title stack's depth (xterm keeps 10).
const TITLE_STACK: usize = 16;
/// How many recent hyperlinks are remembered to share their text.
const RECENT_LINKS: usize = 8;
/// How many OSC 7 reports are kept for a host that is not looking.
pub const CWD_REPORTS_MAX: usize = 16;
/// Events kept for a host that is not looking.
const EVENTS_MAX: usize = 16_384;

/// The terminal state; see the module docs.
pub struct Terminal {
    /// The screen being shown, and the other one, swapped by `altscreen`.
    grid: Grid,
    other: Grid,
    cols: usize,
    rows: usize,
    cursor: Cursor,
    /// The rendition new text gets, the hyperlink, the shell mark and DECSCA.
    pen: Pen,
    modes: Modes,
    /// The modes that are only kept (`modes::RECORDED`), one bit each.
    recorded: u32,
    /// XTSAVE's table: DEC private mode number and the state it was in.
    saved_modes: Vec<(u16, modes::Switch)>,
    margins: Margins,
    tabs: TabStops,
    charsets: Charsets,
    /// The DEC locator (DECELR) and what it knows of the pointer.
    locator: reports::locator::Locator,
    /// What `<` designates (DECAUPSS).
    preferred: Preferred,
    /// DECSC's slot, per screen (index 0: primary, 1: alternate).
    saved: [Saved; 2],
    keys: [KeyboardFlags; 2],
    cursor_style: CursorStyle,
    titles: reports::title::Titles,
    /// Whether programs may change the window and read its title.
    window_ops: WindowOps,
    /// Where the host's window is; unknown until it says.
    window: Option<WindowGeometry>,
    folder: osc::cwd::Folder,
    palette: osc::colors::Palette,
    /// A cell in pixels, for XTWINOPS 14, 16 and XTSMGRAPHICS.
    cell_px: (u16, u16),
    /// The last graphic character printed, for REP.
    last_char: Option<char>,
    cluster: Cluster,
    /// The number of the last command block (OSC 133); 0 before any.
    block: u32,
    /// The pen's shell mark while the alternate screen shows (its cells are not part of a block).
    stashed_content: Content,
    /// Hyperlinks without an id of their own get one from this count.
    link_seq: u64,
    /// The last hyperlinks opened, most recent first; see `intern_link`.
    recent_links: Vec<crate::types::Hyperlink>,
    osc: osc::OscString,
    dcs: dispatch::dcs::DcsString,
    /// Pictures: kitty's, Sixel and iTerm2's; see `graphics`.
    graphics: graphics::Graphics,
    events: Vec<Event>,
    replies: Vec<u8>,
    /// The reply being written went past the cap: its other pieces go too (see `reply`).
    reply_dropped: bool,
    /// The lines of history asked for, see [`Terminal::new`].
    scrollback_lines: usize,
    /// ED 3 ran and nothing but cursor moves and erases came since: an ED 2 now does not save
    /// the screen into the history it was just asked to empty.
    history_just_cleared: bool,
    /// DECSCL's level, and whether the C1 controls are one byte (S8C1T).
    level: Level,
    eight_bit: bool,
    /// Numbers programs set for hardware we do not have, reported back by DECRQSS.
    kept: modes::kept::Kept,
    /// A program has used SPA: ED, EL and ECH must look for guarded cells.
    guard_used: bool,
    /// DECSACE: which cells DECCARA and DECRARA reach.
    attr_extent: screen::rect::AttrExtent,
    /// Whether DECRQCRA is answered; the host chooses (off by default).
    screen_checksum: ScreenChecksum,
    /// The selection and the word separators; see `selection`.
    select: read::selection::Select,
}

/// Cells are 8 bytes and a row's record 16; a history of N lines is capped at about N rows of
/// the current width, plus this slack: the grid drops whole pages, so it keeps up to three pages
/// (128 KiB each) fewer lines than the cap allows, and the slack buys them back.
const HISTORY_SLACK: usize = 3 * 128 * 1024;

fn history_bytes(lines: usize, cols: usize) -> usize {
    lines.saturating_mul(cols * 8 + 16).saturating_add(HISTORY_SLACK)
}

impl Terminal {
    /// A blank terminal of `cols` by `rows` (clamped to what the grid allows) that keeps at least
    /// `scrollback_lines` lines of history (it may keep up to a few hundred more, see
    /// [`Terminal::scrollback_lines`]).
    pub fn new(cols: usize, rows: usize, scrollback_lines: usize) -> Terminal {
        let grid = Grid::new(cols, rows, history_bytes(scrollback_lines, cols));
        let (cols, rows) = (grid.cols(), grid.rows());
        Terminal {
            other: Grid::new(cols, rows, 0),
            grid,
            cols,
            rows,
            cursor: Cursor::default(),
            pen: Pen::default(),
            modes: Modes::INITIAL,
            recorded: 0,
            saved_modes: Vec::new(),
            margins: Margins::full(cols, rows),
            tabs: TabStops::new(cols),
            charsets: Charsets::default(),
            locator: Default::default(),
            preferred: Preferred::default(),
            saved: [Saved::default(), Saved::default()],
            keys: [KeyboardFlags::default(), KeyboardFlags::default()],
            cursor_style: CursorStyle::default(),
            titles: reports::title::Titles::default(),
            window_ops: WindowOps::Deny,
            window: None,
            folder: osc::cwd::Folder::default(),
            palette: osc::colors::Palette::new(),
            cell_px: (8, 16),
            last_char: None,
            cluster: Cluster {
                open: false,
                head: 0,
                width: 0,
                last: ' ',
                state: crate::unicode::GraphemeState::new(),
            },
            block: 0,
            stashed_content: Content::None,
            link_seq: 0,
            recent_links: Vec::with_capacity(RECENT_LINKS),
            osc: osc::OscString::default(),
            dcs: dispatch::dcs::DcsString::default(),
            graphics: graphics::Graphics::default(),
            events: Vec::new(),
            replies: Vec::new(),
            reply_dropped: false,
            scrollback_lines,
            history_just_cleared: false,
            kept: Default::default(),
            level: Level::Vt500,
            eight_bit: false,
            guard_used: false,
            attr_extent: screen::rect::AttrExtent::default(),
            screen_checksum: ScreenChecksum::default(),
            select: read::selection::Select::default(),
        }
    }

    /// Feed `bytes` of program output. The parser may be cut anywhere between calls; keep one
    /// parser per terminal.
    pub fn advance(&mut self, parser: &mut Parser, bytes: &[u8]) {
        parser.advance(self, bytes);
        // Output scrolls rows out of the history; a selection on them is gone with them.
        self.prune_selection();
    }

    /// The events since the last call, oldest first.
    pub fn drain_events(&mut self) -> std::vec::Drain<'_, Event> {
        self.events.drain(..)
    }

    /// The bytes the program is waiting to read, in the order it asked. The host writes them to
    /// the PTY and calls [`clear_replies`](Self::clear_replies).
    pub fn replies(&self) -> &[u8] {
        &self.replies
    }

    /// Forget the replies, after writing them out; the buffer is kept.
    pub fn clear_replies(&mut self) {
        self.replies.clear();
    }

    /// Take the replies, leaving none.
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    /// The window title the program set, `None` if it never did or reset it.
    pub fn title(&self) -> Option<&str> {
        self.titles.window.as_deref()
    }

    /// The shell's folder as it said (OSC 7: `file://host/path`, percent-encoded), `None` before it
    /// said anything.
    pub fn cwd(&self) -> Option<&str> {
        (!self.folder.now.is_empty()).then_some(self.folder.now.as_str())
    }

    /// The history the terminal was asked to keep (at least this many lines; see
    /// [`Terminal::new`]).
    pub fn scrollback_lines(&self) -> usize {
        self.scrollback_lines
    }

    /// Change how much history is kept; lines over it are dropped now.
    pub fn set_scrollback_lines(&mut self, lines: usize) {
        self.scrollback_lines = lines;
        self.apply_history_limit();
    }

    /// Give the main screen the history the configuration asks for at the current width. The
    /// alternate screen never has any, so the grid that is not showing is the main one while it shows.
    fn apply_history_limit(&mut self) {
        let main = if self.on_alt_screen() { &mut self.other } else { &mut self.grid };
        main.set_scrollback_limit(history_bytes(self.scrollback_lines, self.cols));
    }

    /// The modes as programs set them.
    pub fn modes(&self) -> Modes {
        self.modes
    }

    /// The palette entries programs changed (OSC 4, 10, 11, 12) and have not reset.
    pub fn colors(&self) -> Colors {
        self.palette.set
    }

    /// The host's colors, which OSC 4, 10, 11 and 12 queries are answered with unless the program
    /// changed the entry. Without a call the palette is xterm's with a light-gray foreground on
    /// black. Entry `i` is `NamedColor`'s slot (0..=255 the palette, then foreground, background
    /// and cursor).
    pub fn set_default_colors(&mut self, palette: &[Rgb; COLOR_COUNT]) {
        self.palette.defaults.copy_from_slice(palette);
    }

    /// The size of a cell in pixels, which XTWINOPS 14 and 16 and the sixel geometry report say.
    pub fn set_cell_size(&mut self, width: u16, height: u16) {
        self.cell_px = (width.max(1), height.max(1));
    }

    /// The scrolling region as the grid wants it.
    #[inline]
    fn region(&self) -> Region {
        Region {
            top: self.margins.top,
            bottom: self.margins.bottom,
            left: self.margins.left,
            right: self.margins.right,
        }
    }

    /// The color erased and scrolled-in cells get: the pen's background.
    #[inline]
    fn erase_color(&self) -> crate::types::Color {
        self.pen.style.bg
    }

    /// Queue an event. A host that never drains them does not grow without bound: past the cap
    /// new ones are dropped, and a run of the same event (a bell, a title, a folder said twice, a
    /// reset) is one. A picture's pixels, their release and a reset are never dropped: the host
    /// would keep pixels nobody can show, or show a picture without them. They stay bounded past
    /// the cap: `add_image` takes no new picture there, a picture is freed once, and resets in a
    /// row are one.
    fn push_event(&mut self, event: Event) {
        let kept = matches!(event, Event::ImageData(_) | Event::ImageFree(_) | Event::FullReset);
        if (self.events.len() >= EVENTS_MAX && !kept)
            || (matches!(event, Event::Bell | Event::Title | Event::Cwd | Event::FullReset)
                && self.events.last() == Some(&event))
        {
            return;
        }
        self.events.push(event);
    }
}
