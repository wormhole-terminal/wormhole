//! Collecting an operating system command's string.
//!
//! The parser streams the payload in slices; here it is gathered into one buffer (reused, so a
//! steady stream of titles or links allocates nothing) up to a limit that depends on what the
//! command is: a title is a few kilobytes, an inline image is megabytes. The command number is
//! read as soon as its `;` arrives, and a command nothing here acts on is thrown away without
//! being stored. A string over its limit is dropped whole, never cut: half a clipboard or half an
//! image is worse than none.

use memchr::memchr;

/// Longest payload kept, by command (bytes, the number and its `;` included).
const SMALL: usize = 8 * 1024;
const CLIPBOARD: usize = 8 * 1024 * 1024;
const IMAGE: usize = 64 * 1024 * 1024;

/// How much of an OSC string with this command number is kept; `None` for a number we ignore.
fn limit_for(number: u32) -> Option<usize> {
    match number {
        0..=2 | 4 | 5 | 7..=19 | 99 | 104 | 105 | 110..=119 | 133 | 777 | 7501 => Some(SMALL),
        52 => Some(CLIPBOARD),
        1337 => Some(IMAGE),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    /// No string open.
    #[default]
    Idle,
    /// Open, the command number not yet known (its `;` has not come).
    Reading,
    /// Open and kept, up to `limit` bytes.
    Keeping,
    /// Open, and thrown away.
    Dropping,
}

#[derive(Default)]
pub(super) struct OscString {
    buf: Vec<u8>,
    state: State,
    limit: usize,
}

/// The command number at the start of `buf` (digits up to the first `;` or the end), if it has
/// at most five digits.
pub(super) fn split_number(buf: &[u8]) -> Option<(u32, &[u8])> {
    let end = memchr(b';', buf).unwrap_or(buf.len());
    let digits = &buf[..end];
    if digits.is_empty() || digits.len() > 5 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let number = digits.iter().fold(0u32, |n, &d| n * 10 + u32::from(d - b'0'));
    Some((number, buf.get(end + 1..).unwrap_or(&[])))
}

impl OscString {
    pub(super) fn start(&mut self) {
        self.buf.clear();
        self.state = State::Reading;
        self.limit = 0;
    }

    pub(super) fn cancel(&mut self) {
        self.state = State::Idle;
        self.buf.clear();
    }

    pub(super) fn put(&mut self, data: &[u8]) {
        match self.state {
            State::Idle | State::Dropping => {}
            State::Reading => {
                // Only the number matters until it is known; a long run of digits is not one.
                self.buf.extend_from_slice(&data[..data.len().min(8)]);
                if memchr(b';', &self.buf).is_some() || self.buf.len() >= 6 {
                    self.decide();
                    if self.state == State::Keeping {
                        // What came with the number beyond the probe.
                        let probe = data.len().min(8);
                        self.append(&data[probe..]);
                    }
                }
            }
            State::Keeping => self.append(data),
        }
    }

    fn append(&mut self, data: &[u8]) {
        if self.buf.len() + data.len() > self.limit {
            self.state = State::Dropping;
            self.buf = Vec::new();
        } else {
            self.buf.extend_from_slice(data);
        }
    }

    /// The number is in `buf`: keep the string or drop it.
    fn decide(&mut self) {
        match split_number(&self.buf).and_then(|(n, _)| limit_for(n)) {
            Some(limit) if self.buf.len() <= limit => {
                self.limit = limit;
                self.state = State::Keeping;
            }
            _ => {
                self.state = State::Dropping;
                self.buf = Vec::new();
            }
        }
    }

    /// The string ended: its bytes, if it is one to act on. The caller hands the buffer back with
    /// [`OscString::recycle`].
    pub(super) fn take(&mut self) -> Option<Vec<u8>> {
        if self.state == State::Reading {
            self.decide();
        }
        let keep = self.state == State::Keeping;
        self.state = State::Idle;
        keep.then(|| std::mem::take(&mut self.buf))
    }

    /// Give the buffer back to be reused, unless it grew large for a one-off payload.
    pub(super) fn recycle(&mut self, mut buf: Vec<u8>) {
        if buf.capacity() > 64 * 1024 {
            return;
        }
        buf.clear();
        self.buf = buf;
    }
}

#[cfg(test)]
mod tests {
    use super::{OscString, split_number};

    fn feed(parts: &[&[u8]]) -> Option<Vec<u8>> {
        let mut osc = OscString::default();
        osc.start();
        for part in parts {
            osc.put(part);
        }
        osc.take()
    }

    #[test]
    fn the_number_and_the_rest() {
        assert_eq!(split_number(b"8;;http://x"), Some((8, &b";http://x"[..])));
        assert_eq!(split_number(b"104"), Some((104, &b""[..])));
        assert_eq!(split_number(b";x"), None);
        assert_eq!(split_number(b"123456;x"), None);
        assert_eq!(split_number(b"1a;x"), None);
    }

    #[test]
    fn known_commands_are_kept_whatever_the_slicing() {
        assert_eq!(feed(&[b"2;title"]).as_deref(), Some(&b"2;title"[..]));
        assert_eq!(feed(&[b"2", b";ti", b"tle"]).as_deref(), Some(&b"2;title"[..]));
        assert_eq!(feed(&[b"133;A"]).as_deref(), Some(&b"133;A"[..]));
        assert_eq!(feed(&[b"1", b"3", b"3", b";", b"A"]).as_deref(), Some(&b"133;A"[..]));
        assert_eq!(feed(&[b"104"]).as_deref(), Some(&b"104"[..]));
    }

    #[test]
    fn unknown_commands_and_oversized_strings_are_dropped() {
        assert_eq!(feed(&[b"6;whatever"]), None);
        assert_eq!(feed(&[b"junk;whatever"]), None);
        let mut big = b"2;".to_vec();
        big.extend(std::iter::repeat_n(b'x', 9000));
        assert_eq!(feed(&[&big]), None);
        assert_eq!(feed(&[b"2;a", &big]), None);
    }
}
