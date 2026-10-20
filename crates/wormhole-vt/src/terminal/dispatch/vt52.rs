//! VT52 mode: what an escape sequence means while DECANM is reset.
//!
//! Spec: the VT100 User Guide ("Valid VT52 Mode Control Sequences", chapter 3), the VT510
//! Programmer Reference (DECANM: `CSI ? 2 l` enters the mode, `ESC <` leaves it) and xterm's ctlseqs
//! ("VT52 Mode"). Entering it is DECRST 2; the table below is all the terminal reads then:
//!
//! | sequence | does |
//! |----------|------|
//! | `ESC A` `B` `C` `D` | cursor up, down, right, left, stopping at the margins |
//! | `ESC F` `G` | enter and leave graphics mode |
//! | `ESC H` | cursor home |
//! | `ESC I` | reverse line feed (a scroll down at the top margin) |
//! | `ESC J` `K` | erase to the end of the screen, to the end of the line |
//! | `ESC Y` row col | direct cursor address, each a character, the number plus 32 |
//! | `ESC Z` | identify: answered `ESC / Z`, "a VT52 emulated by a VT100" |
//! | `ESC =` `>` | keypad: application and numeric |
//! | `ESC <` | back to ANSI mode |
//!
//! Everything else is ignored, the VT52's printer controls (`ESC ]`, `V`, `W`, `X`, `^`, `_`) and
//! hold-screen (`ESC [`, `ESC \`) among it: the parser reads an ESC and one byte only
//! ([`Handler::vt52_mode`](crate::parser::Handler::vt52_mode)), so `ESC [ 1 m` is a hold-screen
//! request and the text `1 m`, never an SGR. The one thing beyond the manuals is `ESC c`, a full
//! reset: a program that dies in VT52 mode would otherwise leave a terminal that `reset` cannot
//! bring back.
//!
//! What the mode takes from the screen (xterm does the same): it has no wraparound, no origin mode,
//! no insert mode and no new-line mode, and its character sets are ASCII and the graphics set; the
//! terminal keeps what ANSI mode had and brings it back at `ESC <`, which also takes the terminal to
//! VT100 mode, the conformance level 61 with 7-bit controls: the VT510 manual says "Exit VT52 mode.
//! Enter VT100 mode.", and xterm sets level 1 there (its source: "the terminal does not recognize
//! DECRQSS for DECSCL (per vttest)"). The margins are not touched. The graphics set is DEC's special graphics (xterm's
//! choice); the VT100 User Guide says the VT52's own differs in some characters and does not
//! list them.
//!
//! `ESC Y` is the one sequence with parameters after its final byte, and they are plain
//! characters to the parser: the terminal holds how many it still wants and the next text goes to
//! it ([`Terminal::vt52_take_address`]). A control in between acts as usual and an ESC cancels it. A
//! coordinate past the screen leaves that axis where it was (vttest draws with such addresses).

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;
use crate::terminal::modes::conformance::Level;
use crate::terminal::screen::cursor::Landing;
use crate::terminal::screen::edit::{Erase, EraseExtent};
use crate::terminal::text::charset::{Charset, Charsets};

/// The modes VT52 mode does without while it lasts.
const SUSPENDED: Modes = Modes::AUTOWRAP.union(Modes::ORIGIN).union(Modes::INSERT).union(Modes::NEW_LINE);

/// Where `ESC Y` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Address {
    /// Not in it.
    None,
    /// The row is next.
    Row,
    /// The row came (as a number: the character minus 32); the column is next.
    Column(usize),
}

/// VT52 mode's own state. Only meaningful while [`Modes::VT52`] is set.
#[derive(Clone, Debug)]
pub(in crate::terminal) struct Vt52 {
    address: Address,
    /// Graphics mode (`ESC F`).
    graphics: bool,
    /// The suspended modes as ANSI mode had them, and its character sets.
    suspended: Modes,
    charsets: Charsets,
}

impl Default for Vt52 {
    fn default() -> Vt52 {
        Vt52 {
            address: Address::None,
            graphics: false,
            suspended: Modes::empty(),
            charsets: Charsets::default(),
        }
    }
}

impl Vt52 {
    /// Whether `ESC Y` waits for characters.
    #[inline]
    pub(in crate::terminal) fn addressing(&self) -> bool {
        self.address != Address::None
    }

    /// Graphics mode is on.
    pub(in crate::terminal) fn graphics(&self) -> bool {
        self.graphics
    }
}

impl Terminal {
    /// DECRST 2: enter VT52 mode. A reset of a mode that is reset does nothing.
    pub(in crate::terminal) fn enter_vt52(&mut self) {
        if self.modes.contains(Modes::VT52) {
            return;
        }
        self.vt52 = Vt52 {
            address: Address::None,
            graphics: false,
            suspended: self.modes & SUSPENDED,
            charsets: std::mem::take(&mut self.charsets),
        };
        self.modes.remove(SUSPENDED);
        self.modes.insert(Modes::VT52);
        self.cursor.pending = false;
        self.cluster.open = false;
    }

    /// `ESC <`: back to ANSI mode, with the modes and sets it had, in VT100 mode.
    fn leave_vt52(&mut self) {
        let was = std::mem::take(&mut self.vt52);
        self.modes.remove(Modes::VT52);
        self.modes.insert(was.suspended);
        self.charsets = was.charsets;
        // The VT510 manual (DECANM, `ESC <`): "Exit VT52 mode. Enter VT100 mode." VT100 mode has
        // 7-bit controls only.
        self.level = Level::Vt100;
        self.eight_bit = false;
        self.cursor.pending = false;
        self.cluster.open = false;
    }

    /// The modes ANSI mode had before VT52 mode took some away: what a repaint writes before it
    /// enters VT52 mode itself.
    pub(in crate::terminal) fn modes_before_vt52(&self) -> Modes {
        if self.modes.contains(Modes::VT52) {
            (self.modes | self.vt52.suspended) - Modes::VT52
        } else {
            self.modes
        }
    }

    /// The character sets ANSI mode had, for the same.
    pub(in crate::terminal) fn charsets_before_vt52(&self) -> &Charsets {
        if self.modes.contains(Modes::VT52) { &self.vt52.charsets } else { &self.charsets }
    }

    /// The escape sequence `ESC byte` while in VT52 mode.
    pub(in crate::terminal) fn dispatch_vt52(&mut self, byte: u8) {
        // An ESC ends a direct address that was waiting for its characters.
        self.vt52.address = Address::None;
        match byte {
            b'A' => self.cursor_up(1, Landing::Stay),
            b'B' => self.cursor_down(1, Landing::Stay),
            b'C' => self.cursor_forward(1),
            b'D' => self.cursor_back(1),
            b'F' => self.vt52_graphics(true),
            b'G' => self.vt52_graphics(false),
            b'H' => self.home(),
            b'I' => self.reverse_index(),
            b'J' => self.erase_display(EraseExtent::ToEnd, Erase::Everything),
            b'K' => self.erase_line(EraseExtent::ToEnd, Erase::Everything),
            b'Y' => self.vt52.address = Address::Row,
            b'Z' => self.reply(format_args!("\x1b/Z")),
            b'=' => self.modes.insert(Modes::APP_KEYPAD),
            b'>' => self.modes.remove(Modes::APP_KEYPAD),
            b'<' => self.leave_vt52(),
            b'c' => self.reset(),
            _ => {}
        }
    }

    /// ESC F and ESC G: the graphics set, or ASCII, in G0.
    fn vt52_graphics(&mut self, on: bool) {
        self.vt52.graphics = on;
        self.charsets.designate(0, if on { Charset::Graphics } else { Charset::Ascii });
    }

    /// Text that arrives while `ESC Y` waits for its row and column: takes the characters it needs
    /// from the front of `text` and returns the rest, which is printed.
    pub(in crate::terminal) fn vt52_take_address<'a>(&mut self, text: &'a [u8]) -> &'a [u8] {
        let mut rest = text;
        while let Some((&byte, tail)) = rest.split_first() {
            if !self.vt52.addressing() {
                break;
            }
            rest = tail;
            self.vt52_address_character(usize::from(byte.saturating_sub(32)));
        }
        rest
    }

    /// A character other than ASCII where `ESC Y` waits: it is a coordinate too, past any screen.
    pub(in crate::terminal) fn vt52_take_address_char(&mut self) {
        self.vt52_address_character(usize::MAX);
    }

    fn vt52_address_character(&mut self, n: usize) {
        match self.vt52.address {
            Address::None => {}
            Address::Row => self.vt52.address = Address::Column(n),
            Address::Column(row) => {
                self.vt52.address = Address::None;
                let y = if row < self.rows { row } else { self.cursor.y };
                let x = if n < self.cols { n } else { self.cursor.x };
                self.place(x, y);
            }
        }
    }
}
