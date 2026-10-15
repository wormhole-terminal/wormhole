//! UTF-8 for the ground state.
//!
//! Well-formed sequences are those of the Unicode Standard §3.9.3, Table 3-7 (no overlong forms, no
//! surrogates, nothing above U+10FFFF). What is not well-formed is replaced the way §3.9.6
//! ("U+FFFD Substitution of Maximal Subparts", definition D93b) recommends, which the WHATWG
//! Encoding standard's UTF-8 decoder also does: one U+FFFD for each maximal subpart, that is for
//! the longest prefix of the bytes that begins a well-formed sequence, and for each other byte.

/// U+FFFD REPLACEMENT CHARACTER.
pub(super) const REPLACEMENT: char = '\u{FFFD}';

/// What the start of a byte string is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Decoded {
    /// A character and the number of bytes it took.
    Char(char, usize),
    /// A malformed sequence of this many bytes (at least 1): one U+FFFD.
    Invalid(usize),
    /// A proper prefix of a well-formed sequence: more bytes decide it.
    Incomplete,
}

/// Decode the character at the start of `bytes`, which must be non-empty and start with a byte of
/// 0x80 or more.
#[inline(always)]
pub(super) fn decode(bytes: &[u8]) -> Decoded {
    let b0 = bytes[0] as u32;
    // The complete, well-formed case first, with no state: the continuation bytes are all of the
    // form 10xxxxxx (`b ^ 0x80 < 0x40`), and a code point too small for its length (overlong) or in
    // the surrogate range (which `char::from_u32` refuses) falls through to the exact path.
    if (0xC2..0xE0).contains(&b0)
        && let Some(&[_, b1]) = bytes.first_chunk::<2>()
        && b1 ^ 0x80 < 0x40
    {
        let cp = (b0 & 0x1F) << 6 | (b1 as u32 & 0x3F);
        return Decoded::Char(from_valid(cp), 2);
    }
    if (0xE0..0xF0).contains(&b0)
        && let Some(&[_, b1, b2]) = bytes.first_chunk::<3>()
        && (b1 ^ 0x80) | (b2 ^ 0x80) < 0x40
    {
        let cp = (b0 & 0x0F) << 12 | (b1 as u32 & 0x3F) << 6 | (b2 as u32 & 0x3F);
        if cp >= 0x800
            && let Some(c) = char::from_u32(cp)
        {
            return Decoded::Char(c, 3);
        }
    }
    if (0xF0..0xF5).contains(&b0)
        && let Some(&[_, b1, b2, b3]) = bytes.first_chunk::<4>()
        && (b1 ^ 0x80) | (b2 ^ 0x80) | (b3 ^ 0x80) < 0x40
    {
        let cp = (b0 & 0x07) << 18 | (b1 as u32 & 0x3F) << 12 | (b2 as u32 & 0x3F) << 6 | (b3 as u32 & 0x3F);
        if cp >= 0x10000
            && let Some(c) = char::from_u32(cp)
        {
            return Decoded::Char(c, 4);
        }
    }
    exact(bytes)
}

/// `cp` is a two-byte code point (U+0080..=U+07FF), always a character.
#[inline(always)]
fn from_valid(cp: u32) -> char {
    char::from_u32(cp).unwrap_or(REPLACEMENT)
}

/// Table 3-7 byte by byte: the lead byte fixes the length and the range of the second byte, the
/// rest are 0x80..=0xBF. Slow but complete; it settles everything the fast path does not accept.
#[cold]
#[inline(never)]
fn exact(bytes: &[u8]) -> Decoded {
    let lead = bytes[0];
    // (length, lowest and highest second byte, bits of the lead)
    let (len, lo, hi, bits) = match lead {
        0xC2..=0xDF => (2, 0x80, 0xBF, lead & 0x1F),
        0xE0 => (3, 0xA0, 0xBF, lead & 0x0F),
        0xE1..=0xEC | 0xEE..=0xEF => (3, 0x80, 0xBF, lead & 0x0F),
        0xED => (3, 0x80, 0x9F, lead & 0x0F),
        0xF0 => (4, 0x90, 0xBF, lead & 0x07),
        0xF1..=0xF3 => (4, 0x80, 0xBF, lead & 0x07),
        0xF4 => (4, 0x80, 0x8F, lead & 0x07),
        // A continuation byte alone, 0xC0 and 0xC1 (only overlong forms), 0xF5.. (above U+10FFFF).
        _ => return Decoded::Invalid(1),
    };
    let mut cp = u32::from(bits);
    for k in 1..len {
        let Some(&byte) = bytes.get(k) else {
            return Decoded::Incomplete;
        };
        let (lo, hi) = if k == 1 { (lo, hi) } else { (0x80, 0xBF) };
        if !(lo..=hi).contains(&byte) {
            return Decoded::Invalid(k);
        }
        cp = cp << 6 | u32::from(byte & 0x3F);
    }
    Decoded::Char(char::from_u32(cp).unwrap_or(REPLACEMENT), len)
}
