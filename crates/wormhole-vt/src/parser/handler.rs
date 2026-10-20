//! What the parser reports. Every method has an empty default, so a handler listens only to what it
//! needs, and the calls the compiler can see are empty are removed.

use super::csi::Csi;

/// The receiver of everything [`Parser::advance`](super::Parser::advance) finds in the bytes.
///
/// Text and string payloads are never copied: a run of printable ASCII and a piece of an OSC, DCS or
/// APC payload arrive as slices of the input, cut wherever the input was cut.
pub trait Handler {
    /// Whether the bytes 0x80..=0x9F that are no part of a UTF-8 character are C1 controls
    /// (S8C1T, DECSCL's 8-bit mode; ECMA-48 5.3). In UTF-8 text they are never controls, so a
    /// handler says yes only once the program has asked for them. Asked rarely: on such a byte,
    /// and once per string payload.
    fn eight_bit_controls(&self) -> bool {
        false
    }

    /// Whether the terminal is in VT52 mode (DECANM reset), where an ESC is followed by one
    /// final byte and nothing else: `ESC [`, `ESC ]`, `ESC P`, `ESC _`, `ESC X` and `ESC ^` begin
    /// no sequence but are reported as [`esc`](Handler::esc) like any other final, with no
    /// intermediates (`ESC Y`'s two following characters arrive as text). Asked once per ESC.
    fn vt52_mode(&self) -> bool {
        false
    }

    /// Whether DEL (0x7F) in the ground state is a character, not nothing: it is the last position
    /// of a 96-character set in GL (ECMA-35 6.2.1: DEL is only ignorable where it is no part of the
    /// set). Asked at each DEL, so rarely. When it is, the parser hands the DEL over as a run of one
    /// byte through [`print_ascii`](Handler::print_ascii).
    fn del_is_graphic(&self) -> bool {
        false
    }

    /// A run of printable ASCII (0x20..=0x7E), as long as the input allows. Never empty. A run of
    /// the one byte 0x7F when [`del_is_graphic`](Handler::del_is_graphic) says so.
    fn print_ascii(&mut self, _run: &[u8]) {}

    /// One character of U+0080 or more: UTF-8 decoded, with every malformed part (one maximal
    /// subpart each) replaced by U+FFFD. The C1 code points (U+0080..=U+009F) arrive here too:
    /// in UTF-8 they are text, never controls (xterm's ctlseqs, "Control Bytes, Characters, and
    /// Sequences": a C1 byte "can be mistaken for badly-formed UTF-8").
    fn print(&mut self, _c: char) {}

    /// A C0 control (0x00..=0x1F) other than ESC. CAN and SUB arrive here too, after they have
    /// aborted the sequence or string they interrupted (ECMA-48 8.3.6, 8.3.148).
    fn execute(&mut self, _byte: u8) {}

    /// A complete control sequence: `ESC [` parameters, intermediates and a final byte.
    fn csi(&mut self, _csi: &Csi) {}

    /// A complete escape sequence: `ESC`, up to [`MAX_INTERMEDIATES`](super::MAX_INTERMEDIATES)
    /// intermediates (0x20..=0x2F) and a final byte (0x30..=0x7E). `ESC \` after a string is that
    /// string's terminator and is not reported.
    fn esc(&mut self, _intermediates: &[u8], _byte: u8) {}

    /// An operating system command begins (`ESC ]`).
    fn osc_start(&mut self) {}
    /// A piece of the OSC payload; never empty, and no byte below 0x20.
    fn osc_put(&mut self, _data: &[u8]) {}
    /// The OSC ended with BEL (`bel_terminated`) or with ST (`ESC \`, or an ESC that began another
    /// sequence).
    fn osc_end(&mut self, _bel_terminated: bool) {}
    /// CAN or SUB cancelled the OSC; `execute` follows with that byte.
    fn osc_cancel(&mut self) {}

    /// A device control string begins; `dcs` holds its parameters, intermediates and final byte.
    fn dcs_hook(&mut self, _dcs: &Csi) {}
    /// A piece of the DCS data; never empty and never DEL.
    fn dcs_put(&mut self, _data: &[u8]) {}
    /// The DCS ended with ST (or an ESC that began another sequence).
    fn dcs_unhook(&mut self) {}
    /// CAN or SUB cancelled the DCS; `execute` follows with that byte.
    fn dcs_cancel(&mut self) {}

    /// An application program command begins (`ESC _`; the kitty graphics protocol).
    fn apc_start(&mut self) {}
    /// A piece of the APC payload; never empty and never DEL.
    fn apc_put(&mut self, _data: &[u8]) {}
    /// The APC ended with ST (or an ESC that began another sequence).
    fn apc_end(&mut self) {}
    /// CAN or SUB cancelled the APC; `execute` follows with that byte.
    fn apc_cancel(&mut self) {}
}
