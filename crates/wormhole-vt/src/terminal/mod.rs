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
//! | | `vt52` | VT52 mode (DECANM reset): its escape sequences, `ESC Y`'s two characters, the modes and sets it suspends |
//! | | `dcs` | device control strings and APC: collecting them and sending them on (sixel is read here; the strings a host asked for with a `Passthrough` are collected here and queued as events) |
//! | `text/` | `print` | text: ASCII runs, wide characters, marks, grapheme clusters (mode 2027), REP, IRM |
//! | | `charset` | the sets G0..G3 hold: national replacement sets, DEC Technical and Supplemental, Latin-1; and the shifts |
//! | | `sgr`, `sgr_stack` | SGR and the colors it sets; XTPUSHSGR, XTPOPSGR and XTREPORTSGR |
//! | `screen/` | `cursor` | where the cursor is, its movement, origin mode, DECSC/DECRC and the slot they use |
//! | | `edit` | ED, EL, ECH, ICH, DCH, IL, DL, DECIC, DECDC, DECSED, DECSEL |
//! | | `rect` | DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA, DECSACE |
//! | | `scroll` | the margins; IND, RI, NEL, SU, SD, DECSTBM, DECSLRM, DECBI, DECFI |
//! | | `line_size` | DECSWL, DECDWL, DECDHL: a row drawn double-width or as half of a double-height line, and the cursor limited to the half it shows |
//! | | `tabs` | the tab stops themselves |
//! | | `altscreen` | 47, 1047, 1048, 1049 |
//! | | `history` | how much history the main screen keeps: the host's line count as the grid's byte cap |
//! | | `resize`, `reset` | the window changing size, DECCOLM; RIS, DECSTR, DECALN |
//! | `modes/` | (`mod`) | the numbers that name the modes (the set itself, `Modes`, is a value type in `types/`) and the sets a soft reset and the keyboard flags are |
//! | | `apply` | SM/RM, DECSET/DECRST, XTSAVE/XTRESTORE and what setting each does; DECRQM |
//! | | `conformance` | DECSCL, S7C1T and S8C1T: the level and one-byte C1 controls |
//! | | `keyboard`, `modkeys`, `kept` | the kitty keyboard stacks; XTMODKEYS and XTQMODKEYS (modifyOtherKeys); numbers kept for hardware we do not have |
//! | `reports/` | (`mod`) | DSR, DA1/2/3, XTVERSION |
//! | | `queue` | the replies owed to the program: kept or dropped whole, the cap, one-byte controls, and the order behind a picture's answer |
//! | | `scheme`, `inband` | the host's light or dark scheme: `CSI ? 996 n`, mode 2031; the in-band resize reports of mode 2048 |
//! | | `state`, `presentation` | DECRQPSR (cursor information, tab stops), DECRQTSR, and DECRSPS and DECRSTS to put them back; DECRQDE; the user-preferred supplemental set |
//! | | `checksum` | DECRQCRA, and the policy that turns it on (off by default) |
//! | | `rqss`, `tcap`, `hex` | DECRQSS; XTGETTCAP; bytes as hex digits, for the reports that spell text so |
//! | | `window`, `title` | XTWINOPS: the window's reports and operations; the title, the icon name and their stack |
//! | | `locator` | the DEC locator: DECELR, DECSLE, DECRQLP, DECEFR and the reports the host's pointer events make |
//! | `osc/` | (`mod`), `dispatch`, `payload` | operating system commands: collecting the string; which command it is; the payload as text |
//! | | `cwd`, `link`, `clipboard`, `notify`, `marks` | OSC 7 (the folder), 8 (hyperlinks), 52 (clipboard), 9, 9;4, 99 and 777 (notifications, progress), 133 (shell marks) |
//! | | `colors`, `spec` | OSC 4, 5, 10 to 19 and their resets; the parser of color specs |
//! | | `color_stack` | XTPUSHCOLORS, XTPOPCOLORS and XTREPORTCOLORS: the colors programs set, pushed and popped |
//! | | `pointer`, `uservar` | OSC 22 (pointer shape) and iTerm2's `SetUserVar` (OSC 1337), as events |
//! | `graphics/` | | kitty's graphics protocol, Sixel and iTerm2's inline images, placed in the grid |
//! | `read/` | `snapshot` | what the host draws from |
//! | | `selection`, `search` | selecting, copying text, the text a search runs over, links and marks under a point |
//! | `repaint/` | | the VT bytes that rebuild this terminal in another one (a host's attach and resync), `fields` listing how each piece of state is covered |
//! | (top) | `event`, `policy`, `limits`, `builder` | what the host hears and the queue it waits in; the risky switches; the caps the host can lower; how a terminal is made |
//! | | `archive` | handing the host the history lines that leave the engine (opt-in; the copying is the grid's) |
//! | | `base64` | base64 for the clipboard, user variables and the pictures |
//! | | `watch` | `ModesChanged` and `ColorsChanged`: what changed since the host last took the events |
//! | | `media_copy` | media copy (`CSI Ps i`): recognised and ignored; no code, only the decision and its test |
//! | | `passthrough` | [`Passthrough`](passthrough::Passthrough): the OSC, DCS and APC strings a host claims, and the refusal of the ones the engine reads itself (public as `wormhole_vt::passthrough`) |
//! | | `check`, `dump` | the invariants check (tests, and feature `internals`); the whole state as named text, for the tests that compare two terminals (feature `internals`) |
//!
//! # Choices
//!
//! - In VT52 mode the parser reads an ESC and one byte only ([`Handler::vt52_mode`](
//!   crate::parser::Handler::vt52_mode)), and the terminal reads the VT52's table (`dispatch/vt52`).
//!   The modes a VT52 has no use for (autowrap, origin, insert, new line) are suspended while it
//!   lasts and come back with `ESC <`, as do the character sets, and `ESC <` leaves the terminal at
//!   VT100 level (as xterm and the VT510 manual do); the margins are not touched.
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
//! - Mode 1007 (the wheel sends arrow keys on the alternate screen) starts set, so that a wheel
//!   over a pager or an editor, where there is no history to scroll, moves it instead of doing
//!   nothing; the other modes start as xterm's do. DECSTR leaves autowrap on (ncurses sends
//!   it and expects lines to wrap). DECSCUSR 0 hands the cursor style back to the host.
//! - A title or folder that a program sends is kept here and announced by an event without a
//!   payload (`title()`, `cwd()`), so that a prompt that sets both costs no allocation; an empty
//!   title is no title.
//! - A hostile program cannot make the terminal grow without bound: strings are kept up to a limit
//!   per kind (a title 8 KiB, a clipboard 8 MiB, an image 64 MiB) and dropped whole beyond it,
//!   events and replies that nobody collects are capped, and every count is clamped to the screen.
//!   Pictures have their own caps (`graphics`). Each cap is a ceiling the host can lower
//!   ([`Limits`]), never raise.

mod archive;
mod base64;
mod builder;
#[cfg(any(test, feature = "internals"))]
mod check;
mod dispatch;
#[cfg(any(test, feature = "internals"))]
mod dump;
mod event;
mod graphics;
mod limits;
mod media_copy;
mod modes;
mod osc;
pub mod passthrough;
mod policy;
mod read;
pub mod repaint;
mod reports;
mod screen;
#[cfg(test)]
mod tests;
mod text;
mod watch;

use crate::grid::{BlockPart, Grid, Pen};
use crate::parser::Parser;

use modes::Modes;
use modes::conformance::Level;
use modes::keyboard::KeyboardFlags;
use read::damage::Seen;
use screen::cursor::{CursorPos, CursorStyle, Saved};
use screen::scroll::Margins;
use screen::tabs::TabStops;
use text::charset::{Charsets, Preferred};
use text::print::Cluster;

pub use builder::Builder;
#[cfg(any(test, feature = "internals"))]
pub use dump::StateDump;
pub use event::{Event, Progress, ProgressState, PromptKind, ShellMark, Terminator};
pub use graphics::{
    Compression, IMAGE_BUDGET_BYTES, ImageError, ImageFormat, ImageUpload, MAX_IMAGES, Placeholder,
    VirtualPlacement, placeholder_at, strip_placeholders,
};
pub use limits::Limits;
pub use osc::cwd::CWD_REPORTS_MAX;
pub use policy::Policy;
pub use read::damage::Damage;
pub use read::search::Span;
pub use read::selection::DEFAULT_WORD_SEPARATORS;
pub use read::snapshot::Cursor;
pub use reports::locator::{LocatorButton, Pointer};
pub use reports::window::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};

/// The terminal state; see the module docs.
pub struct Terminal {
    /// The screen being shown, and the other one, swapped by `altscreen`.
    grid: Grid,
    other: Grid,
    cols: usize,
    rows: usize,
    cursor: CursorPos,
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
    /// VT52 mode's state: `ESC Y` in progress, graphics mode, and what ANSI mode had.
    vt52: dispatch::vt52::Vt52,
    /// XTPUSHSGR's stack.
    sgr_stack: text::sgr_stack::SgrStack,
    /// XTPUSHCOLORS' stack.
    color_stack: osc::color_stack::ColorStack,
    /// The size the host was last told in band (mode 2048), to tell it only when it changes.
    inband: reports::inband::Reported,
    /// DECSC's slot, per screen (index 0: primary, 1: alternate).
    saved: [Saved; 2],
    keys: [KeyboardFlags; 2],
    cursor_style: CursorStyle,
    titles: reports::title::Titles,
    /// What programs may do besides print; see [`Policy`].
    policy: Policy,
    /// The caps a program's sequences are held to; see [`Limits`].
    limits: Limits,
    /// Where the host's window is; unknown until it says.
    window: Option<WindowGeometry>,
    folder: osc::cwd::Folder,
    palette: osc::colors::Palette,
    /// A cell in pixels, for XTWINOPS 14, 16 and XTSMGRAPHICS.
    cell_px: (u16, u16),
    /// The status line's type and whether it is the active display; see `screen::status_line`.
    status: screen::status_line::StatusLine,
    /// The graphic character printed last when nothing but text has come since, for REP: every
    /// control function forgets it (`end_text`).
    last_char: Option<char>,
    cluster: Cluster,
    /// The number of the last command block (OSC 133); 0 before any.
    block: u32,
    /// The pen's shell mark while the alternate screen shows (its cells are not part of a block).
    stashed_content: BlockPart,
    /// Hyperlinks without an id of their own get one from this count.
    link_seq: u64,
    /// The text of the hyperlinks opened lately, shared; see `osc::link`.
    link_text: osc::link::LinkText,
    /// What XTVERSION answers: the host's name and version for the terminal.
    identity: String,
    /// The parser that reads `feed`'s bytes; see [`Terminal::feed`].
    parser: Parser,
    osc: osc::OscString,
    dcs: dispatch::dcs::DcsString,
    /// The strings the host asked to be handed over; see [`crate::passthrough`].
    passthrough: passthrough::Passthrough,
    /// Pictures: kitty's, Sixel and iTerm2's; see `graphics`.
    graphics: graphics::Graphics,
    events: Vec<Event>,
    /// Bytes of `Event::UserVar` values waiting in `events`.
    queued_values: usize,
    /// Bytes of the other events' text and data waiting in `events` (`Event::payload_bytes`).
    queued_bytes: usize,
    /// The replies owed to the program, kept whole or dropped whole; see `reports::queue`.
    replies: reports::ReplyQueue,
    /// The lines of history asked for, see [`Builder::scrollback`].
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
    /// The selection and the word separators; see `selection`.
    select: read::selection::Select,
    /// Whether the host draws a light or a dark scheme, once it said (`set_color_scheme`).
    scheme: Option<crate::types::ColorScheme>,
    /// What the last `take_damage` saw of the cursor and the colours.
    seen: Seen,
    /// What the host was last told changed, see `watch`.
    told: watch::Told,
}

// A host parses on one thread and reads on another, usually under a lock: the terminal must stay
// `Send` (and `Sync`, for hosts that share it behind a read lock). A field that is not (an `Rc`, a
// raw pointer) fails the build here instead of in someone else's.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Terminal>();
};

impl Terminal {
    /// Read `bytes` of program output. They may be cut anywhere between calls, in the middle of
    /// a sequence or of a UTF-8 character: the terminal keeps its parser's state.
    pub fn feed(&mut self, bytes: &[u8]) {
        // The parser calls back into the terminal, so it cannot stay inside it while it runs.
        // It is a plain value of about a hundred bytes: moving it out and back costs less than
        // a call, once per feed.
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(&mut dispatch::handler::Sink(self), bytes);
        self.parser = parser;
        // Output scrolls rows out of the history; a selection on them is gone with them.
        self.prune_selection();
    }
}
