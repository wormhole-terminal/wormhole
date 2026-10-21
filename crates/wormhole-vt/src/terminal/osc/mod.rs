//! Collecting an operating system command's string.
//!
//! The parser streams the payload in slices; here it is gathered into one buffer (reused, so a
//! steady stream of titles or links allocates nothing) up to a limit that depends on what the
//! command is: a title is a few kilobytes, an inline image is megabytes. The command number is
//! read as soon as its `;` arrives, and a command nothing here acts on is thrown away without
//! being stored. A string over its limit is dropped whole, never cut: half a clipboard or half an
//! image is worse than none. A number an extension claimed is kept up to the claim's limit
//! instead, whatever the engine would do with it. The limits below are ceilings: the host can lower
//! each (`Limits`).

pub(super) mod clipboard;
pub(super) mod color_stack;
pub(super) mod colors;
pub(super) mod cwd;
pub(super) mod dispatch;
pub(super) mod link;
pub(super) mod marks;
pub(super) mod notify;
pub(super) mod payload;
pub(super) mod pointer;
pub(super) mod spec;
pub(super) mod uservar;

use memchr::memchr;

/// Longest payload kept, by command (bytes, the number and its `;` included).
pub(in crate::terminal) const SMALL: usize = 8 * 1024;
pub(in crate::terminal) const CLIPBOARD: usize = 8 * 1024 * 1024;
pub(in crate::terminal) const IMAGE: usize = 64 * 1024 * 1024;
/// What of OSC 1337 is not an inline image (marks, the folder, …) is a line or two; the host gets
/// at most this much of it. User variables are bigger, see `USER_VAR`.
pub(in crate::terminal) const ITERM_OTHER: usize = 64 * 1024;
/// A `SetUserVar` string: 16 bytes of `1337;SetUserVar=`, a name, `=` and a value of
/// [`uservar::VALUE_MAX`] bytes in base64, with room to spare.
const USER_VAR: usize = 88 * 1024;
const USER_VAR_KIND: &[u8] = b"SetUserVar=";
/// OSC 1337 strings of these kinds are pictures and keep the picture limit; the longest prefix is
/// `1337;MultipartFile=`, which tells them from the rest once that many bytes are in.
const IMAGE_KINDS: [&[u8]; 3] = [b"File=", b"MultipartFile=", b"FilePart="];
const SNIFF: usize = "1337;MultipartFile=".len();

/// Which of the limits rules a command's string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Size {
    Small,
    Clipboard,
    Image,
}

/// How big an OSC string with this command number may be; `None` for a number we ignore.
fn size_of_command(number: u32) -> Option<Size> {
    match number {
        0..=2 | 4 | 5 | 7..=19 | 22 | 99 | 104 | 105 | 110..=119 | 133 | 777 | 7501 => Some(Size::Small),
        52 => Some(Size::Clipboard),
        1337 => Some(Size::Image),
        _ => None,
    }
}

/// The limits on OSC strings, by size class: the ceilings above unless the host lowered them. A
/// string being read keeps the ones it started with.
#[derive(Clone, Copy, Debug)]
pub(in crate::terminal) struct Caps {
    pub small: usize,
    pub clipboard: usize,
    pub image: usize,
    /// OSC 1337 that is not a picture.
    pub other: usize,
}

impl Default for Caps {
    fn default() -> Caps {
        Caps { small: SMALL, clipboard: CLIPBOARD, image: IMAGE, other: ITERM_OTHER }
    }
}

impl Caps {
    fn of(self, size: Size) -> usize {
        match size {
            Size::Small => self.small,
            Size::Clipboard => self.clipboard,
            Size::Image => self.image,
        }
    }
}

/// Whether the engine acts on (or hands the host, raw) OSC `number` itself: those cannot be
/// passed through.
pub(crate) fn is_builtin(number: u32) -> bool {
    size_of_command(number).is_some()
}

/// Whether an OSC 1337 string (the number and its `;` included) starts a picture.
fn is_picture(buf: &[u8]) -> bool {
    split_number(buf).is_some_and(|(_, rest)| IMAGE_KINDS.iter().any(|kind| rest.starts_with(kind)))
}

/// Whether an OSC 1337 string (the number and its `;` included) sets a user variable.
fn is_user_var(buf: &[u8]) -> bool {
    split_number(buf).is_some_and(|(_, rest)| rest.starts_with(USER_VAR_KIND))
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
pub(in crate::terminal) struct OscString {
    buf: Vec<u8>,
    state: State,
    limit: usize,
    /// The OSC numbers extensions claimed and their limits on the payload.
    claims: Vec<(u32, usize)>,
    /// An OSC 1337 string whose kind is not known yet: it is kept to the picture limit until
    /// `SNIFF` bytes tell, then to `Caps::other` unless it is a picture.
    sniff: bool,
    caps: Caps,
}

/// The command number at the start of `buf` (digits up to the first `;` or the end), if it has
/// at most five digits.
pub(in crate::terminal) fn split_number(buf: &[u8]) -> Option<(u32, &[u8])> {
    let end = memchr(b';', buf).unwrap_or(buf.len());
    let digits = &buf[..end];
    if digits.is_empty() || digits.len() > 5 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let number = digits.iter().fold(0u32, |n, &d| n * 10 + u32::from(d - b'0'));
    Some((number, buf.get(end + 1..).unwrap_or(&[])))
}

impl OscString {
    pub(in crate::terminal) fn start(&mut self) {
        self.buf.clear();
        self.state = State::Reading;
        self.limit = 0;
        self.sniff = false;
    }

    /// The numbers extensions claim, for the limit of the strings that follow.
    pub(in crate::terminal) fn set_claims(&mut self, claims: &[(u32, usize)]) {
        self.claims.clear();
        self.claims.extend_from_slice(claims);
    }

    /// The limits for the strings that follow.
    pub(in crate::terminal) fn set_caps(&mut self, caps: Caps) {
        self.caps = caps;
    }

    pub(in crate::terminal) fn cancel(&mut self) {
        self.state = State::Idle;
        self.buf.clear();
    }

    pub(in crate::terminal) fn put(&mut self, data: &[u8]) {
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
            return;
        }
        self.buf.extend_from_slice(data);
        if self.sniff && self.buf.len() >= SNIFF {
            self.sniff = false;
            if !is_picture(&self.buf) {
                self.limit = if is_user_var(&self.buf) { USER_VAR } else { self.caps.other };
                if self.buf.len() > self.limit {
                    self.state = State::Dropping;
                    self.buf = Vec::new();
                }
            }
        }
    }

    /// The number is in `buf`: keep the string or drop it.
    fn decide(&mut self) {
        let limit = split_number(&self.buf).and_then(|(number, rest)| {
            match self.claims.iter().find(|claim| claim.0 == number) {
                // The claim's limit counts the payload; the buffer also holds the number and `;`.
                Some(&(_, cap)) => Some((self.buf.len() - rest.len()).saturating_add(cap)),
                None => {
                    // What is not a picture is cut short once that is known, see `append`.
                    self.sniff = number == 1337;
                    size_of_command(number).map(|size| self.caps.of(size))
                }
            }
        });
        match limit {
            Some(limit) if self.buf.len() <= limit => {
                self.limit = limit;
                self.state = State::Keeping;
            }
            _ => {
                // Only the number's probe is in the buffer: it is kept for the next string.
                self.state = State::Dropping;
                self.buf.clear();
            }
        }
    }

    /// The string ended: its bytes, if it is one to act on. The caller hands the buffer back with
    /// [`OscString::recycle`].
    pub(in crate::terminal) fn take(&mut self) -> Option<Vec<u8>> {
        if self.state == State::Reading {
            self.decide();
        }
        let keep = self.state == State::Keeping;
        self.state = State::Idle;
        keep.then(|| std::mem::take(&mut self.buf))
    }

    /// Give the buffer back to be reused, unless it grew large for a one-off payload.
    pub(in crate::terminal) fn recycle(&mut self, mut buf: Vec<u8>) {
        if buf.capacity() > 64 * 1024 {
            return;
        }
        buf.clear();
        self.buf = buf;
    }
}

#[cfg(test)]
mod tests {
    use super::{Caps, OscString, split_number};

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
    fn a_cancelled_string_is_gone() {
        let mut osc = OscString::default();
        osc.start();
        osc.put(b"2;title");
        osc.cancel();
        assert_eq!(osc.take(), None);
    }

    #[test]
    fn a_run_of_digits_that_is_no_command_number_is_dropped_as_soon_as_it_is_seen() {
        // Without a `;` the number is only known to be too long once six digits have come: the
        // string must not be gathered while the program goes on sending them.
        let mut osc = OscString::default();
        osc.start();
        for _ in 0..10 {
            osc.put(b"1");
        }
        assert_eq!(osc.state, super::State::Dropping);
        assert!(osc.buf.is_empty());
    }

    #[test]
    fn what_is_not_a_picture_is_held_to_the_cap_for_it_once_the_kind_is_known() {
        let big = vec![b'v'; 70_000];
        // Neither a picture nor a user variable: the 64 KiB for the rest of OSC 1337.
        assert_eq!(feed(&[b"1337;Custom=x", &big]), None);
        // A user variable may be bigger, up to 88 KiB, and a picture much bigger still.
        assert!(feed(&[b"1337;SetUserVar=x=", &big]).is_some());
        assert_eq!(feed(&[b"1337;SetUserVar=x=", &vec![b'v'; 100_000]]), None);
        assert!(feed(&[b"1337;File=inline=1:", &big]).is_some());
        // The multipart form's name is cut in two here: the kind is known when the last of it is in.
        assert!(feed(&[b"1337;Multipa", b"rtFile=", &big]).is_some());
    }

    #[test]
    fn a_string_of_exactly_the_cap_for_what_is_not_a_picture_is_kept() {
        let mut osc = OscString::default();
        osc.set_caps(Caps { other: 30, ..Caps::default() });
        // 30 bytes: the number, `;`, `Custom=` and 17 more.
        let text = |len: usize| {
            let mut text = b"1337;Custom=".to_vec();
            text.resize(len, b'x');
            text
        };
        osc.start();
        osc.put(&text(30));
        assert_eq!(osc.take().map(|kept| kept.len()), Some(30));
        osc.start();
        osc.put(&text(31));
        assert_eq!(osc.take(), None);
    }

    #[test]
    fn a_command_without_a_payload_is_judged_by_the_cap_when_it_ends() {
        let mut osc = OscString::default();
        osc.set_caps(Caps { small: 2, ..Caps::default() });
        osc.start();
        osc.put(b"104");
        assert_eq!(osc.take(), None, "three bytes against a cap of two");
        osc.set_caps(Caps { small: 3, ..Caps::default() });
        osc.start();
        osc.put(b"104");
        assert_eq!(osc.take().as_deref(), Some(&b"104"[..]), "and against three");
    }

    #[test]
    fn a_buffer_given_back_is_kept_for_the_next_string_up_to_64_kib() {
        for (capacity, kept) in [(2000, true), (64 * 1024, true), (64 * 1024 + 1, false), (128 * 1024, false)]
        {
            let mut osc = OscString::default();
            osc.recycle(Vec::with_capacity(capacity));
            assert_eq!(osc.buf.capacity() >= capacity, kept, "capacity {capacity}");
        }
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
