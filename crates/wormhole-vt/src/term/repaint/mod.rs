//! The repaint: VT bytes that rebuild a terminal's state in another terminal of the same size.
//!
//! A host that owns the authoritative engine and a client that keeps a replica meet in this
//! writer: it reads the engine's state and writes the escape sequences that, parsed by a fresh
//! engine (or by one holding anything at all: the stream starts with RIS), leave it in the state
//! the first one is in. The client then parses the program's raw bytes from that point on and the
//! two stay equal (`wormhole-vt-bench`'s bisimulation test checks that, for every corpus, at many
//! cut points). It serves the attach, the resync after a client fell behind, the bounded-bandwidth
//! mode and the checkpoint a restart restores from.
//!
//! # Where a repaint can be made
//!
//! Only between sequences: [`Terminal::repaint_ready`] says whether the parser and the terminal
//! are. A host that is asked in the middle of one keeps parsing until it is (a sequence is short;
//! a picture sent in chunks takes as long as it takes), or repaints at the last point that was
//! ready and then sends the bytes since as they came.
//!
//! # What a repaint cannot say
//!
//! Every piece of state either has VT that sets it, or a [`Gaps`] bit that says it was left out
//! ([`Repainted::gaps`]). Nothing is approximated without saying so, and the bisimulation test
//! fails for what is left out: the state comparison sees it. Counters that no sequence can set (the
//! next block number, the next generated link id, the next picture handle) are [`Seeds`]: the
//! client applies them with [`Terminal::seed`] before it parses the repaint.
//!
//! # Shape
//!
//! `rows` writes the screens (history by scrolling it in, the screen by cursor addressing),
//! `pen` the renditions, links, protection and block marks the cells carry, `tables` everything
//! else, and `fields` lists every field of the engine's state and how each is covered, which
//! `tests/structure.rs` checks against the source so that a new field cannot be forgotten.
//!
//! The writer takes `&Terminal`: it changes nothing, damage included, and allocates nothing but
//! the growth of the caller's buffer, which the caller keeps between calls.

mod fields;
mod pen;
mod rows;
mod tables;
#[cfg(test)]
mod tests;

use bitflags::bitflags;

use crate::grid::Content;
use crate::parser::Parser;

use super::Terminal;
use super::modes::Modes;
pub use fields::{CELL_FLAGS, Coverage, FIELDS, MODES, StructFields};
use pen::Pen;

bitflags! {
    /// What a repaint left out because VT cannot say it. A repaint with any of these is still a
    /// good approximation of the screen; the bisimulation test is what tells how good, and it
    /// fails for each of these on purpose.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Gaps: u32 {
        /// Pictures are placed in the grid. Their pixels are the host's and the engine keeps no
        /// encoded source to send again; the placements are not written.
        const PLACEMENTS = 1;
        /// The engine holds pictures (ids, numbers, virtual placements) that are not written.
        const IMAGES = 1 << 1;
        /// DECCOLM made the screen 132 columns wide: the mode only sets by resizing.
        const COLUMNS_132 = 1 << 2;
        /// An XTSAVE entry that cannot be rebuilt (DECCOLM's, or a set one the engine would not
        /// report as set).
        const SAVED_MODES = 1 << 3;
        /// The conformance level is below VT300: the sequences that put cursor information and tab
        /// stops back are VT300 features, so the cursor's wrap, shifts and sets are not written.
        const LOW_LEVEL = 1 << 4;
        /// A wrapped line whose last cell is not text or whose next row does not begin with it
        /// (an edit cut the line): the wrap flag cannot be made without printing there.
        const WRAP = 1 << 5;
        /// A cell carries a block mark the stream cannot reach (an older block than a later row's,
        /// a block on a cell of the alternate screen, or the pen's block was reset).
        const BLOCKS = 1 << 6;
        /// A color VT cannot name (a "named" foreground that is not a default).
        const COLOR = 1 << 7;
        /// A title, icon name or folder holds bytes that would end the sequence that sets it.
        const TEXT = 1 << 8;
        /// `last_char` or the counters needed a free cell to be set in and the screen has none.
        const TOUCH = 1 << 9;
        /// A spacer head (the blank a wide character that did not fit left) with no wide
        /// character on the next row.
        const SPACER_HEAD = 1 << 10;
        /// The alternate screen's pen is in the middle of a prompt: the mark only sets on the
        /// main screen.
        const ALT_CONTENT = 1 << 11;
        /// A zero-width character on a cell with no character, in the last column.
        const MARK = 1 << 12;
        /// The last character printed can still be extended, and either insert mode is on, which
        /// printing it again to say so would act on, or the cluster is not what its cell holds
        /// (the engine keeps the head of the cluster one column short when autowrap is off and a
        /// character fills the last column).
        const CLUSTER = 1 << 13;
        /// No character was ever printed (`last_char` is none) but the screen holds text: it
        /// came from DECALN or DECFRA, and printing it in a repaint sets `last_char`.
        const LAST_CHAR = 1 << 14;
        /// The alternate screen holds lines above its screen, which a resize left there; VT has
        /// no way to put lines in a history that is not kept.
        const ALT_HISTORY = 1 << 15;
        /// The main screen's history cap is not the one a receiver of this size has (the engine
        /// left a stale one when it was resized on the alternate screen). The caps are the
        /// receiver's own setting, not state a stream can carry.
        const LIMIT = 1 << 16;
        /// A cell's characters make a cell of another width or shape in the receiver, whether it
        /// clusters characters or not.
        const CELL = 1 << 17;
        /// ED 3 was the last thing but cursor moves and erases, and the history is not empty (a
        /// resize put lines in it), so the "ED 3 just before" mark of the next ED 2 cannot be set.
        const HISTORY_CLEARED = 1 << 18;
    }
}

impl Gaps {
    /// The gaps whose left-out state steers what later bytes do: a receiver made from a repaint
    /// with one of these drifts from the sender as the stream goes on (a pending wrap that is not
    /// there, a cap on the history, a cluster that cannot grow). The others only leave a field
    /// different (pictures, `last_char`) or matter to a resize.
    pub const STICKY: Gaps = Gaps::LOW_LEVEL
        .union(Gaps::ALT_HISTORY)
        .union(Gaps::LIMIT)
        .union(Gaps::CLUSTER)
        .union(Gaps::COLUMNS_132)
        .union(Gaps::SAVED_MODES)
        .union(Gaps::WRAP)
        .union(Gaps::BLOCKS)
        .union(Gaps::COLOR)
        .union(Gaps::TEXT)
        .union(Gaps::SPACER_HEAD)
        .union(Gaps::MARK)
        .union(Gaps::CELL)
        .union(Gaps::HISTORY_CLEARED)
        .union(Gaps::ALT_CONTENT);
}

/// Counters that VT has no sequence for. Apply them to the replica with [`Terminal::seed`]
/// before parsing the repaint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seeds {
    /// The number of the last command block (OSC 133) the replica has issued; the repaint's own
    /// `OSC 133;A` marks count up from it to the blocks the screen holds. This is the "oldest
    /// retained block, minus one" the design's `seed_block_counter` takes.
    pub block: u32,
    /// How many links without an id of their own have been given one.
    pub link_seq: u64,
    /// The last picture handle given out.
    pub image_handle: u64,
}

/// What a repaint wrote, besides the bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Repainted {
    pub seeds: Seeds,
    pub gaps: Gaps,
    /// Lines of history written.
    pub history_lines: usize,
}

/// Why a repaint cannot be made now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotReady {
    /// The parser is inside an escape sequence, a control string or a character.
    Sequence,
    /// A picture is arriving in chunks (kitty), or an iTerm2 multipart image is.
    Picture,
}

impl Terminal {
    /// Whether a repaint can be made now: nothing is half-read, in the parser or in the terminal.
    pub fn repaint_ready(&self, parser: &Parser) -> Result<(), NotReady> {
        if !parser.is_ground() {
            return Err(NotReady::Sequence);
        }
        if self.graphics.in_transfer() {
            return Err(NotReady::Picture);
        }
        Ok(())
    }

    /// Append to `out` the bytes that rebuild this terminal's state in another of the same size
    /// (columns, rows and history limit are the receiving side's own, set before). The bytes begin
    /// with RIS. Apply [`Repainted::seeds`] to the receiver first (see [`Terminal::seed`]).
    ///
    /// `Err` when [`repaint_ready`](Self::repaint_ready) says no; nothing is written then.
    pub fn repaint(&self, parser: &Parser, out: &mut Vec<u8>) -> Result<Repainted, NotReady> {
        self.repaint_ready(parser)?;
        let mut w = Writer::new(self, out);
        w.write_all();
        Ok(w.finish())
    }

    /// Set the counters a repaint cannot; call before parsing it.
    pub fn seed(&mut self, seeds: &Seeds) {
        self.seed_block_counter(seeds.block);
        self.link_seq = seeds.link_seq;
        self.graphics.seed_handle(seeds.image_handle);
    }

    /// Set the number of the last command block (OSC 133) issued: the next `OSC 133;A` makes block
    /// `n + 1`. A replica seeds it so that the block numbers its screen carries are the host's.
    pub fn seed_block_counter(&mut self, n: u32) {
        self.block = n;
    }
}

/// The state of one repaint being written: the receiver as the bytes written so far leave it.
struct Writer<'a> {
    term: &'a Terminal,
    out: &'a mut Vec<u8>,
    gaps: Gaps,
    /// The receiver's pen.
    pen: Pen<'a>,
    /// The receiver's block counter, and where it started.
    seed_block: Option<u32>,
    counter: u32,
    /// Whether the receiver shows its alternate screen, and the mark its main screen's pen was
    /// in while the other shows (the switch puts it aside).
    alt: bool,
    stash: Content,
    history_lines: usize,
    /// Columns of the blank cells of the span being written that HT must mark.
    tabs: Vec<usize>,
    /// The modes the XTSAVE table's replay changed, which are written again.
    touched: Vec<u16>,
    /// The last cell of the row just written was a blank that had to be printed for the line to
    /// wrap (the background it must be erased to), to be made blank again once the line has
    /// wrapped.
    fix_wrap: Option<Option<crate::types::Color>>,
    /// Whether the receiver clusters characters (mode 2027), as a fresh one does.
    clusters: bool,
    /// Where the last cell printed ended in the output, and whether it was more than ASCII.
    text_end: usize,
    text_rich: bool,
    /// What that cell's look was: where its style and link are, its flags and its row's block.
    pen_key: (usize, u16, u32, usize),
}

impl<'a> Writer<'a> {
    fn new(term: &'a Terminal, out: &'a mut Vec<u8>) -> Writer<'a> {
        Writer {
            term,
            out,
            gaps: Gaps::empty(),
            pen: Pen::default(),
            seed_block: None,
            counter: 0,
            alt: false,
            stash: Content::None,
            history_lines: 0,
            tabs: Vec::new(),
            touched: Vec::new(),
            fix_wrap: None,
            clusters: true,
            text_end: usize::MAX,
            text_rich: false,
            pen_key: (0, 0, 0, 0),
        }
    }

    /// The order matters; each step says why.
    fn write_all(&mut self) {
        let t = self.term;
        // Everything the receiver held goes, and the level is set first: DECSCL is a hard reset,
        // and the sequences that follow (cursor information, locator, margins) depend on it.
        self.put(b"\x1bc");
        self.level();
        // Pictures: their pixels are the host's, and nothing here says where to get them.
        if t.grid.placement_count() + t.other.placement_count() > 0 {
            self.gaps |= Gaps::PLACEMENTS;
        }
        if t.graphics.has_images() {
            self.gaps |= Gaps::IMAGES;
        }
        // A resize on the alternate screen hands the history limit to the wrong grid: the
        // alternate one then keeps lines that scroll off, and the main one keeps a stale cap. A
        // receiver that was never resized there has neither.
        let (main, alt) = if t.on_alt_screen() { (&t.other, &t.grid) } else { (&t.grid, &t.other) };
        if alt.scrollback_limit() != 0 || alt.history_len() > 0 {
            self.gaps |= Gaps::ALT_HISTORY;
        }
        if main.scrollback_limit() != super::history_bytes(t.scrollback_lines, t.cols) {
            self.gaps |= Gaps::LIMIT;
        }
        // The contents. The main screen first, then the other one, both with the receiver's
        // modes at their defaults (autowrap on, no margins, no origin), which is what the cells
        // were written under as far as a repaint can tell.
        let (primary, hidden) = if t.on_alt_screen() { (&t.other, &t.grid) } else { (&t.grid, &t.other) };
        self.screen_rows(primary, false);
        if rows::has_cells(hidden) {
            self.show(true);
            self.screen_rows(hidden, true);
            self.show(false);
        }
        // The tables, then the counters and the cell that carries `last_char`.
        self.tables();
        self.touch_and_counters();
        // The state of the cursor, per screen, then the real one.
        self.saved_cursors();
        self.modes();
        self.final_state();
    }

    fn finish(self) -> Repainted {
        Repainted {
            seeds: Seeds {
                block: self.seed_block.unwrap_or(self.term.block),
                link_seq: self.term.link_seq,
                image_handle: self.term.graphics.last_handle(),
            },
            gaps: self.gaps,
            history_lines: self.history_lines,
        }
    }

    // ---- bytes ----

    #[inline]
    fn put(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
    }

    fn num(&mut self, n: usize) {
        let mut buf = [0u8; 20];
        let mut at = buf.len();
        let mut n = n;
        loop {
            at -= 1;
            buf[at] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        self.out.extend_from_slice(&buf[at..]);
    }

    /// `ESC [ n final`, with the count left out when it is 1 (the default for the movements).
    fn csi_count(&mut self, n: usize, final_byte: u8) {
        self.put(b"\x1b[");
        if n != 1 {
            self.num(n);
        }
        self.out.push(final_byte);
    }

    /// `ESC [ a ; b final`.
    fn csi2(&mut self, a: usize, b: usize, final_byte: u8) {
        self.put(b"\x1b[");
        self.num(a);
        self.out.push(b';');
        self.num(b);
        self.out.push(final_byte);
    }

    /// An OSC string ended by ST.
    fn osc(&mut self, body: &str) {
        self.put(b"\x1b]");
        self.put(body.as_bytes());
        self.put(b"\x1b\\");
    }

    /// The conformance level, if it is not the default.
    fn level(&mut self) {
        use super::conformance::Level;
        let level = self.term.level;
        if level == Level::Vt500 {
            return;
        }
        // 7-bit controls: the receiver's replies are not the repaint's business, `S8C1T` at the end
        // says what the sender had.
        self.put(b"\x1b[");
        self.num(usize::from(level.param()));
        self.put(b";1\"p");
    }

    /// Set the mode `n` (DEC private) on or off.
    fn private_mode(&mut self, n: u16, on: bool) {
        self.put(b"\x1b[?");
        self.num(usize::from(n));
        self.out.push(if on { b'h' } else { b'l' });
    }

    /// Show the alternate (`true`) or the main screen, without clearing anything.
    fn show(&mut self, alt: bool) {
        if self.alt != alt {
            self.private_mode(47, alt);
            self.alt = alt;
            // Entering puts the pen's mark aside and leaving brings it back.
            if alt {
                self.stash = std::mem::replace(&mut self.pen.content, Content::None);
            } else {
                self.pen.content = std::mem::replace(&mut self.stash, Content::None);
            }
        }
    }

    /// Whether the mode is set in the terminal being written.
    fn has(&self, mode: Modes) -> bool {
        self.term.modes.contains(mode)
    }
}
