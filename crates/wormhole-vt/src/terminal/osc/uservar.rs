//! iTerm2's user variables: `OSC 1337 ; SetUserVar=name=base64 ST`.
//!
//! Spec: iTerm2's proprietary escape codes
//! (<https://iterm2.com/documentation-escape-codes.html>, "Set User Variable"), which WezTerm
//! and Ghostty-adjacent tools also speak: the value is base64 and the name is the text up to the
//! first `=`. The engine decodes the pair so a host does not have to parse the rest of OSC 1337
//! to hear it; it reaches the host as [`Event::UserVar`](crate::terminal::Event::UserVar) and
//! not as the raw 1337 event. A name that is empty, longer than [`NAME_MAX`], not text or made of
//! anything but printable characters, a value that is not base64, or one that decodes to more than
//! [`VALUE_MAX`] bytes is dropped whole.

use crate::terminal::Terminal;
use crate::terminal::event::Event;
use crate::terminal::osc::base64::{INVALID, VALUES, decode};

/// The longest name kept, in bytes.
pub(in crate::terminal) const NAME_MAX: usize = 256;
/// The longest value kept, decoded, in bytes.
pub(in crate::terminal) const VALUE_MAX: usize = 64 * 1024;
/// The most bytes of values waiting in the event queue; later ones are dropped until the host
/// takes the events. A queue of events that carry 64 KiB each would otherwise hold a gigabyte.
pub(in crate::terminal) const QUEUED_MAX: usize = 1 << 20;
/// The longest base64 text of a value of [`VALUE_MAX`] bytes.
const ENCODED_MAX: usize = VALUE_MAX.div_ceil(3) * 4;

/// The name and the decoded value in `pair` (what follows `SetUserVar=`).
fn parse(pair: &[u8]) -> Option<(String, Vec<u8>)> {
    let split = pair.iter().position(|&b| b == b'=')?;
    let (name, encoded) = (&pair[..split], &pair[split + 1..]);
    let name = std::str::from_utf8(name).ok()?;
    if name.is_empty() || name.len() > NAME_MAX || name.chars().any(char::is_control) {
        return None;
    }
    // The clipboard's decoder stops at the first stray byte; a value is all or nothing.
    if encoded.len() > ENCODED_MAX || encoded.iter().any(|&b| b != b'=' && VALUES[usize::from(b)] == INVALID)
    {
        return None;
    }
    let value = decode(encoded)?;
    (value.len() <= VALUE_MAX).then(|| (name.to_string(), value))
}

impl Terminal {
    /// `SetUserVar=` followed by `pair`.
    pub(in crate::terminal) fn osc_user_var(&mut self, pair: &[u8]) {
        let Some((name, value)) = parse(pair) else { return };
        if self.queued_values.saturating_add(value.len()) > QUEUED_MAX {
            return;
        }
        self.queued_values += value.len();
        self.push_event(Event::UserVar { name, value });
    }
}

#[cfg(test)]
mod tests {
    use super::{NAME_MAX, VALUE_MAX, parse};

    #[test]
    fn a_name_and_its_base64_value() {
        assert_eq!(parse(b"user=Zm9v"), Some(("user".to_string(), b"foo".to_vec())));
        assert_eq!(parse(b"empty="), Some(("empty".to_string(), Vec::new())));
        assert_eq!(parse(b"caf\xc3\xa9=Yg=="), Some(("caf\u{e9}".to_string(), b"b".to_vec())));
        // The name ends at the first `=`; a value cannot contain one before its padding.
        assert_eq!(parse(b"a=b=Yg=="), None);
    }

    #[test]
    fn bad_pairs_are_dropped_whole() {
        for bad in [&b"=Yg=="[..], b"novalue", b"a=Y!g=", b"a=Y", b"a\x1b=Yg==", b"a\xff=Yg=="] {
            assert_eq!(parse(bad), None, "{:?}", String::from_utf8_lossy(bad));
        }
        let long = format!("{}=Yg==", "n".repeat(NAME_MAX + 1));
        assert_eq!(parse(long.as_bytes()), None);
        assert!(parse(format!("{}=Yg==", "n".repeat(NAME_MAX)).as_bytes()).is_some());
    }

    #[test]
    fn a_value_is_at_most_64_kib_decoded() {
        let encoded = |len: usize| super::super::base64::encode(&vec![b'x'; len]);
        let fits = format!("v={}", encoded(VALUE_MAX));
        assert_eq!(parse(fits.as_bytes()).map(|(_, value)| value.len()), Some(VALUE_MAX));
        let over = format!("v={}", encoded(VALUE_MAX + 1));
        assert_eq!(parse(over.as_bytes()), None);
    }
}
