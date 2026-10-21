//! Bytes as hexadecimal text, the form some reports take (XTGETTCAP's, a title's in title mode 1).

/// Bytes as uppercase hex digits.
pub(in crate::terminal) struct Hex<'a>(pub(in crate::terminal) &'a [u8]);

impl std::fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02X}"))
    }
}
