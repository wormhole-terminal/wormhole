//! 8-bit C1 controls (ECMA-48 5.3): the byte 0x80 + n stands for ESC and the byte 0x40 + n.
//!
//! The parser reads them only when the handler asks for them (`Handler::eight_bit_controls`),
//! because a stray byte of 0x80..=0x9F is otherwise malformed UTF-8 and printed as U+FFFD, and a
//! program that does not know about the mode must not be able to start a sequence with it. A
//! character of UTF-8 text is read as a whole first, so its continuation bytes are never taken for
//! controls; the cost is that a string closed by ST (0x9C) cannot contain a UTF-8 character with
//! that byte in it, as on any 8-bit terminal.

use super::{Handler, Parser, State};

/// The index of the first C1 byte of `bytes`, or its length.
pub(super) fn first_c1(bytes: &[u8]) -> usize {
    bytes.iter().position(|b| (0x80..=0x9F).contains(b)).unwrap_or(bytes.len())
}

impl Parser {
    /// A C1 `byte` is read as `ESC` and the byte 0x40 less. `st` says the byte comes where an ESC
    /// would end a string (or a DCS header), so that a 0x9C is the string's ST and nothing else.
    #[cold]
    pub(super) fn c1<H: Handler>(&mut self, h: &mut H, byte: u8, st: bool) {
        self.state = if st { State::EscapeSt } else { State::Escape };
        self.escape(h, &[byte - 0x40], 0);
    }
}
