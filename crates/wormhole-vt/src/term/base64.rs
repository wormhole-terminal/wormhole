//! Base64 (RFC 4648, the standard alphabet) for OSC 52, which carries clipboard text in it.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Each byte's sextet, or `INVALID`: one load per input byte instead of a chain of range tests.
pub(super) const VALUES: [u8; 256] = {
    let mut table = [INVALID; 256];
    let mut i = 0;
    while i < 64 {
        table[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    table
};
pub(super) const INVALID: u8 = 0xFF;

/// The bytes `text` encodes, up to the first character that is neither of the alphabet nor
/// padding (RFC 4648 3.3 lets an implementation stop there; xterm does, and esctest2's OSC 52
/// test sends a stray quote after the data); `None` for a length that no encoding has. Missing
/// padding is accepted.
pub(super) fn decode(text: &[u8]) -> Option<Vec<u8>> {
    let end = text.iter().position(|&b| b != b'=' && VALUES[usize::from(b)] == INVALID).unwrap_or(text.len());
    // Nothing but garbage is not an empty clipboard.
    if end == 0 && !text.is_empty() {
        return None;
    }
    let text = &text[..end];
    let text = match text {
        [rest @ .., b'=', b'='] | [rest @ .., b'='] => rest,
        _ => text,
    };
    if text.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3 + 2);
    let (groups, tail) = text.as_chunks::<4>();
    let mut bad = 0u8;
    for group in groups {
        let [a, b, c, d] = group.map(|byte| VALUES[byte as usize]);
        // One test per four bytes: any invalid value has its top bits set, no valid one does.
        bad |= a | b | c | d;
        let bits =
            u32::from(a & 63) << 18 | u32::from(b & 63) << 12 | u32::from(c & 63) << 6 | u32::from(d & 63);
        out.extend_from_slice(&bits.to_be_bytes()[1..]);
    }
    if bad & 0xC0 != 0 {
        return None;
    }
    if !tail.is_empty() {
        let mut bits = 0u32;
        for &byte in tail {
            let value = VALUES[byte as usize];
            if value == INVALID {
                return None;
            }
            bits = bits << 6 | u32::from(value);
        }
        bits <<= 6 * (4 - tail.len());
        out.extend_from_slice(&bits.to_be_bytes()[1..tail.len()]);
    }
    Some(out)
}

/// `bytes` as base64 with padding.
pub(super) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let bits = chunk.iter().enumerate().fold(0u32, |acc, (i, &b)| acc | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(bits >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn rfc_4648_vectors() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), coded);
            assert_eq!(decode(coded.as_bytes()).as_deref(), Some(plain.as_bytes()));
        }
    }

    #[test]
    fn padding_is_optional_and_the_text_ends_at_garbage() {
        assert_eq!(decode(b"Zm9vYg").as_deref(), Some(&b"foob"[..]));
        assert_eq!(decode(b"Zm9vY"), None);
        assert_eq!(decode(b"Zm9v!").as_deref(), Some(&b"foo"[..]));
        assert_eq!(decode(b"Zm9vYg=='").as_deref(), Some(&b"foob"[..]));
        assert_eq!(decode(b"Zm 9v").as_deref(), Some(&b"f"[..]));
        assert_eq!(decode(b"!"), None);
        assert_eq!(decode(b""), Some(Vec::new()));
    }
}
