//! Reading one APC graphics command as it streams in: control data, then base64 payload.
//!
//! The parser hands a string over in slices of any size. The control data (up to the first `;`)
//! is gathered and parsed when it ends; the payload is decoded from base64 as it arrives, into
//! the bytes it stands for, so a 30 MB picture is never held twice. Past the payload cap the
//! rest is read and thrown away and the command fails with `EFBIG`.

use super::command::{CONTROL_MAX, Command};
use super::reply::Error;
use crate::terminal::base64::{INVALID, VALUES};

/// Most bytes one command's payload may decode to (a PNG of a screenshot is a few megabytes;
/// raw 4K RGBA is 33 MB).
pub const PAYLOAD_MAX: usize = 32 * 1024 * 1024;

/// What a finished command is.
#[derive(Debug)]
pub(super) struct Parsed {
    pub command: Command,
    /// The first thing wrong with the keys.
    pub error: Option<Error>,
    /// The decoded payload.
    pub payload: Vec<u8>,
    /// More than the payload cap (at most [`PAYLOAD_MAX`]) came.
    pub overflow: bool,
    /// The payload is not base64.
    pub bad_base64: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Control,
    Payload,
}

/// The command being read.
#[derive(Debug, Default)]
pub(super) struct ApcReader {
    stage: Stage,
    control: Vec<u8>,
    /// Set when the control data ends.
    command: Option<(Command, Option<Error>)>,
    payload: Vec<u8>,
    /// Up to three characters of a group of four not yet complete.
    carry: [u8; 4],
    carried: usize,
    padded: bool,
    overflow: bool,
    bad_base64: bool,
    /// Most bytes the payload may decode to: [`PAYLOAD_MAX`], or less when the host lowered it.
    max: usize,
}

fn sextet(byte: u8) -> Option<u8> {
    let value = VALUES[byte as usize];
    (value != INVALID).then_some(value)
}

impl ApcReader {
    /// A new string begins (its `G` already seen); its payload may decode to `max` bytes.
    pub fn begin(&mut self, max: usize) {
        let (mut payload, mut control) =
            (std::mem::take(&mut self.payload), std::mem::take(&mut self.control));
        payload.clear();
        control.clear();
        *self = ApcReader { payload, control, max: max.min(PAYLOAD_MAX), ..ApcReader::default() };
    }

    /// The next slice of the string after the `G`.
    pub fn put(&mut self, mut data: &[u8]) {
        if self.stage == Stage::Control {
            let end = data.iter().position(|&b| b == b';');
            let take = end.unwrap_or(data.len());
            if self.control.len() + take > CONTROL_MAX {
                // Not a command anyone sends; keep reading to the end, answer with an error.
                self.control.resize(CONTROL_MAX + 1, b' ');
            } else {
                self.control.extend_from_slice(&data[..take]);
            }
            let Some(end) = end else { return };
            self.finish_control();
            data = &data[end + 1..];
        }
        self.payload(data);
    }

    fn finish_control(&mut self) {
        self.command = Some(Command::parse(&self.control));
        self.stage = Stage::Payload;
    }

    fn payload(&mut self, data: &[u8]) {
        if self.overflow || self.bad_base64 || self.padded {
            return;
        }
        let mut data = data;
        while !data.is_empty() {
            // Whole groups of four sextets at once, which is nearly all of a payload: one test per
            // group instead of one per byte.
            while self.carried == 0 && data.len() >= 4 {
                let [a, b, c, d] = [data[0], data[1], data[2], data[3]].map(|byte| VALUES[byte as usize]);
                if (a | b | c | d) & 0xC0 != 0 {
                    break;
                }
                if self.payload.len() + 3 > self.max {
                    self.overflow = true;
                    self.payload = Vec::new();
                    return;
                }
                self.payload.extend_from_slice(&[a << 2 | b >> 4, b << 4 | c >> 2, c << 6 | d]);
                data = &data[4..];
            }
            let Some((&byte, rest)) = data.split_first() else { return };
            data = rest;
            if byte == b'=' {
                self.padded = true;
                return;
            }
            if matches!(byte, b'\n' | b'\r' | b' ') {
                continue;
            }
            let Some(value) = sextet(byte) else {
                self.bad_base64 = true;
                return;
            };
            self.carry[self.carried] = value;
            self.carried += 1;
            if self.carried == 4 {
                if self.payload.len() + 3 > self.max {
                    self.overflow = true;
                    self.payload = Vec::new();
                    return;
                }
                let [a, b, c, d] = self.carry;
                self.payload.extend_from_slice(&[a << 2 | b >> 4, b << 4 | c >> 2, c << 6 | d]);
                self.carried = 0;
            }
        }
    }

    /// The string ended.
    pub fn finish(&mut self) -> Parsed {
        if self.stage == Stage::Control {
            self.finish_control();
        }
        // A last group of two or three characters is one or two bytes; one character is no byte.
        match self.carried {
            _ if self.overflow => {}
            0 => {}
            1 => self.bad_base64 = true,
            n => {
                let [a, b, c, _] = self.carry;
                self.payload.push(a << 2 | b >> 4);
                if n == 3 {
                    self.payload.push(b << 4 | c >> 2);
                }
            }
        }
        let (command, error) = self.command.take().unwrap_or_default();
        Parsed {
            command,
            error,
            payload: std::mem::take(&mut self.payload),
            overflow: self.overflow,
            bad_base64: self.bad_base64,
        }
    }

    /// The string was cancelled: forget it.
    pub fn cancel(&mut self) {
        self.begin(self.max);
    }
}

#[cfg(test)]
mod tests {
    use super::{ApcReader, PAYLOAD_MAX};

    fn read(parts: &[&[u8]]) -> super::Parsed {
        let mut reader = ApcReader::default();
        reader.begin(PAYLOAD_MAX);
        for part in parts {
            reader.put(part);
        }
        reader.finish()
    }

    #[test]
    fn control_then_payload_whatever_the_slicing() {
        let whole = read(&[b"a=T,f=24,i=3;AAEC"]);
        assert_eq!((whole.command.id, whole.command.format), (3, 24));
        assert_eq!(whole.payload, [0, 1, 2]);
        for cut in 0..=17 {
            let text = b"a=T,f=24,i=3;AAEC";
            let (a, b) = text.split_at(cut);
            let p = read(&[a, b]);
            assert_eq!((p.command.id, p.payload.as_slice()), (3, &[0u8, 1, 2][..]), "cut at {cut}");
        }
        let bytewise: Vec<&[u8]> = b"i=9;aGVsbG8".chunks(1).collect();
        assert_eq!(read(&bytewise).payload, b"hello");
    }

    #[test]
    fn padding_is_optional_and_a_command_may_have_no_payload() {
        assert_eq!(read(&[b";aGk="]).payload, b"hi");
        assert_eq!(read(&[b";aGk"]).payload, b"hi");
        assert_eq!(read(&[b";aA=="]).payload, b"h");
        assert_eq!(read(&[b";aA"]).payload, b"h");
        let none = read(&[b"a=d,d=a"]);
        assert!(none.payload.is_empty() && !none.bad_base64);
        assert_eq!(read(&[b""]).command, super::Command::default());
    }

    #[test]
    fn what_is_not_base64_is_reported() {
        assert!(read(&[b"i=1;!!!"]).bad_base64);
        assert!(read(&[b"i=1;A"]).bad_base64, "a single character is no byte");
        assert!(read(&[b"i=1;AAEC\nAAEC"]).payload.len() == 6, "line breaks are skipped");
    }

    #[test]
    fn a_payload_over_the_cap_is_dropped_whole_and_reading_goes_on() {
        let mut reader = ApcReader::default();
        reader.begin(PAYLOAD_MAX);
        reader.put(b"i=2;");
        let block = vec![b'A'; 4 * 1024 * 1024];
        for _ in 0..(PAYLOAD_MAX / (3 * 1024 * 1024)) + 2 {
            reader.put(&block);
        }
        let parsed = reader.finish();
        assert!(parsed.overflow);
        assert!(parsed.payload.is_empty());
        assert_eq!(parsed.command.id, 2);
    }

    #[test]
    fn control_data_without_end_is_an_error_not_a_panic() {
        let long = vec![b'i'; 10_000];
        let parsed = read(&[&long, b";AAAA"]);
        assert!(parsed.error.is_some());
        let mut reader = ApcReader::default();
        reader.begin(PAYLOAD_MAX);
        reader.put(&long);
        assert!(reader.finish().error.is_some());
    }
}
