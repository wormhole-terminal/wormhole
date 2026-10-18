//! 8-bit C1 controls (ECMA-48 5.3): the byte 0x80 + n stands for ESC and the byte 0x40 + n.
//!
//! The parser reads them only when the handler asks for them (`Handler::eight_bit_controls`),
//! because a stray byte of 0x80..=0x9F is otherwise malformed UTF-8 and printed as U+FFFD, and a
//! program that does not know about the mode must not be able to start a sequence with it. A
//! character of UTF-8 text is read as a whole first, so its continuation bytes are never taken for
//! controls; inside a string (a title, a DCS payload) the bytes that continue a character are
//! counted off as they pass, across feeds, so an em dash (E2 80 94) is text and a lone 0x9C is ST.

use super::{Handler, Parser, State};

/// The index of the first C1 control in the string text `bytes`, or its length. A byte of
/// 0x80..=0x9F that continues a UTF-8 character is text; `due` is how many continuation bytes the
/// last lead byte still expects, kept from one call to the next.
pub(super) fn first_c1_in_text(bytes: &[u8], due: &mut u8) -> usize {
    for (i, &byte) in bytes.iter().enumerate() {
        if *due > 0 && (0x80..=0xBF).contains(&byte) {
            *due -= 1;
            continue;
        }
        *due = match byte {
            0xC2..=0xDF => 1,
            0xE0..=0xEF => 2,
            0xF0..=0xF4 => 3,
            0x80..=0x9F => return i,
            _ => 0,
        };
    }
    bytes.len()
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
