//! The escape-sequence parser: bytes from a program in, text, controls and sequences out.
//!
//! Written from the published specifications alone: the state machine is Paul Williams' "A parser
//! for DEC's ANSI-compatible video terminals" (vt100.net/emu/dec_ansi_parser), the sequence
//! formats are ECMA-48 (5th ed., §5) and ECMA-35, the extra parameter syntax is ITU-T T.416
//! §13.1.8, and what xterm accepts beyond the standards is its ctlseqs document. The places where
//! this parser departs from Williams' tables, and why:
//!
//! - **UTF-8.** The ground state decodes UTF-8 (Unicode §3.9.3, Table 3-7; malformed input is
//!   replaced by U+FFFD per maximal subpart, §3.9.6); the bytes 0x80..=0x9F are never controls there (Williams' "anywhere"
//!   rows for them describe 8-bit terminals, and xterm's ctlseqs warns that a C1 byte "can be
//!   mistaken for badly-formed UTF-8"). Inside a sequence, bytes of 0x80 and more are ignored: the
//!   tables' GR = GL rule would make them part of it.
//! - **DEL** (0x7F) is ignored in the ground state: ECMA-35 6.2.1 lets it be inserted or removed
//!   "without affecting its information content", and Williams leaves the choice to the character set
//!   mapped into GL. It is dropped from sequences and from DCS and APC data as in his tables, and
//!   kept in OSC text, where his table has it as `osc_put`.
//! - **`:`** (0x3A) is a sub-parameter separator in CSI and DCS parameters (ECMA-48 5.4.2 b,
//!   T.416 §13.1.8; xterm's ctlseqs writes SGR colours as `38:2::r:g:b`), where Williams sends it
//!   to the ignoring state.
//! - **BEL** ends an OSC (xterm ctlseqs: "XTerm accepts either BEL or ST").
//! - **ST** is `ESC \`. An ESC inside a string ends it (the handler hears the end) and starts a new
//!   sequence; when that sequence is just `\` it was the string's terminator and nothing is
//!   reported, where Williams' tables would dispatch `ESC \`. The same holds for a DCS whose
//!   header is not finished: it has begun a control string (ECMA-48 5.6), which ST closes.
//! - **CAN and SUB** abort a sequence or cancel a string (the handler hears `*_cancel`) and are
//!   then executed, where Williams' tables also execute them but keep no record of the string.
//! - **OSC, DCS and APC payloads** are passed on in pieces as they arrive, never buffered. C0
//!   controls other than BEL, CAN, SUB and ESC are dropped from an OSC (his table ignores them),
//!   and pass through DCS and APC data (`put` takes 0x00..=0x17, 0x19, 0x1C..=0x7E). PM and SOS
//!   strings are consumed and never reported.
//! - **VT52 mode.** A handler that says so ([`Handler::vt52_mode`]) gets an ESC and its one next
//!   byte as an [`esc`](Handler::esc) with no intermediates, whatever the byte is: the VT52 has no
//!   CSI, no strings and no intermediates (VT100 User Guide, "Valid VT52 Mode Control Sequences"),
//!   so `ESC [` and `ESC ]` begin nothing. Asked once per ESC, so a handler that switches mode in
//!   the middle of the input is followed at the next ESC.
//! - **Limits.** Williams stores a fixed number of parameters and lets `collect` flag a sequence
//!   with too many intermediates. Here that is [`MAX_PARAMS`] values and [`MAX_INTERMEDIATES`]
//!   bytes; a CSI over either is still reported with [`Csi::overflow`] set, an ESC sequence over the
//!   intermediates limit is dropped. A value saturates at 65535.
//!
//! The fast paths are in `scan` (runs of printable ASCII and the ends of payloads, a word at a
//! time) and `utf8` (whole characters at once); everything else is a plain state machine that
//! keeps only a few bytes of state between calls, so a sequence may be cut anywhere.

mod c1;
mod csi;
mod escape;
mod ground;
mod handler;
mod scan;
mod sequence;
mod string;
mod utf8;

#[cfg(test)]
mod tests;

pub use csi::{Csi, Groups, MAX_INTERMEDIATES, MAX_PARAMS};
pub use handler::Handler;

/// Where the parser is between two bytes. Williams' states, with his CSI and DCS "entry" states
/// folded into the parameter states (a private marker is only valid before anything else, which
/// `Csi::at_start` tells), and two more: `Utf8` for a character cut by the end of the input and
/// `EscapeSt` for the ESC that ended a string.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Ground,
    /// The start of a UTF-8 character is held in `Parser::pending`.
    Utf8,
    Escape,
    /// An ESC that ended an OSC, DCS, APC, PM or SOS: a `\` next is its ST.
    EscapeSt,
    EscapeIntermediate,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    DcsParam,
    DcsIntermediate,
    DcsData,
    Osc,
    Apc,
    /// A string whose data nobody wants, skipped to its end: PM, SOS, or a DCS with a malformed
    /// header (Williams' DCS Ignore; the same bytes end it).
    StringIgnored,
}

/// The state machine. It holds no heap buffers (about a hundred bytes, the sequence being read
/// among them), and [`advance`](Parser::advance) allocates nothing.
#[derive(Clone, Debug)]
pub struct Parser {
    state: State,
    /// The parameter being read, saturated at 65535, and whether any digit was seen (an empty one
    /// is a default, ECMA-48 5.4.2 e).
    cur: u32,
    digits: bool,
    /// The sequence being read; also holds the intermediates of an ESC sequence.
    csi: Csi,
    /// The bytes of a UTF-8 character the previous input ended in the middle of.
    pending: [u8; 3],
    pending_len: u8,
    /// With 8-bit controls: the continuation bytes a character in a string still has to come, so
    /// that they are not read as C1 controls.
    string_utf8: u8,
}

impl Default for Parser {
    fn default() -> Parser {
        Parser::new()
    }
}

impl Parser {
    pub const fn new() -> Parser {
        Parser {
            state: State::Ground,
            cur: 0,
            digits: false,
            csi: Csi::new(),
            pending: [0; 3],
            pending_len: 0,
            string_utf8: 0,
        }
    }

    /// True between sequences: not inside an escape sequence, a control string, or the middle of a
    /// UTF-8 character.
    pub fn is_ground(&self) -> bool {
        self.state == State::Ground
    }

    /// The parts of the state that decide what the parser does next, as text for `dump_state`: the
    /// state it is in, the bytes of a cut character it holds and the continuation bytes a string
    /// still owes. The buffer for a cut character keeps old bytes once the character is finished,
    /// and past `pending_len`; they are never read, so they are left out and two parsers that were
    /// fed the same bytes in different pieces show the same.
    #[cfg(any(test, feature = "internals"))]
    pub(crate) fn essence(&self) -> String {
        let held =
            if self.state == State::Utf8 { &self.pending[..usize::from(self.pending_len)] } else { &[] };
        format!("{:?} pending {held:?} string_utf8 {}", self.state, self.string_utf8)
    }

    /// Read `bytes`, calling `handler` for what they hold. The input may end anywhere, in the
    /// middle of a sequence or of a character; the next call carries on.
    pub fn advance<H: Handler>(&mut self, handler: &mut H, bytes: &[u8]) {
        let mut at = 0;
        while at < bytes.len() {
            at = match self.state {
                State::Ground if handler.wants_raw() => {
                    at + handler.raw(&bytes[at..]).clamp(1, bytes.len() - at)
                }
                State::Ground => self.ground(handler, bytes, at),
                State::Utf8 => self.resume_utf8(handler, bytes, at),
                State::Escape | State::EscapeSt => self.escape(handler, bytes, at),
                State::EscapeIntermediate => self.escape_intermediate(handler, bytes, at),
                State::CsiParam => self.params::<H, false>(handler, bytes, at),
                State::CsiIntermediate => self.intermediates::<H, false>(handler, bytes, at),
                State::CsiIgnore => self.csi_ignore(handler, bytes, at),
                State::DcsParam => self.params::<H, true>(handler, bytes, at),
                State::DcsIntermediate => self.intermediates::<H, true>(handler, bytes, at),
                State::DcsData => self.dcs_data(handler, bytes, at),
                State::Osc => self.osc(handler, bytes, at),
                State::Apc => self.apc(handler, bytes, at),
                State::StringIgnored => self.string_ignored(handler, bytes, at),
            };
        }
    }

    /// An ESC begins a sequence (and ends any other): forget what was collected.
    #[inline(always)]
    fn begin(&mut self) {
        self.csi.clear();
        self.cur = 0;
        self.digits = false;
    }
}
