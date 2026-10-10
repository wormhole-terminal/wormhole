//! kitty's graphics protocol: the control data of one APC `G` command, chunked transfers put
//! back together, and the terminal's answers.
//!
//! A command is `ESC _ G <key=value,…> ; <base64 payload> ESC \`. Only what this terminal does
//! is read: transmitting and displaying direct (`t=d`) data as RGB, RGBA or PNG (zlib or not), in
//! one piece or chunks (`m=1`), placing a transmitted image, deleting placements, and queries.
//! Files, shared memory, animation and relative placements are answered with an error, as the
//! protocol has the terminal do for what it doesn't support, and every file access answers the
//! same way so a program learns nothing about files it can't read.

use base64::Engine;
use base64::engine::general_purpose::GeneralPurpose;
use base64::engine::{DecodePaddingMode, GeneralPurposeConfig};

use super::MAX_PAYLOAD_BYTES;

/// Most control data one command may carry; kitty's keys fit in a fraction of this.
pub const MAX_CONTROL_BYTES: usize = 1024;

/// One command's keys, with the protocol's defaults where a key is absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    /// `a`: `t` transmit, `T` transmit and display, `p` put (display a transmitted image), `d`
    /// delete, `q` query, `f` / `a` animation.
    pub action: char,
    /// `f`: 24 RGB, 32 RGBA, 100 PNG.
    pub format: u32,
    /// `t`: `d` direct, `f` file, `t` temporary file, `s` shared memory.
    pub transmission: char,
    /// `s`, `v`: the pixel size, needed for RGB and RGBA data.
    pub width: u32,
    pub height: u32,
    /// `o=z`: the data is zlib-compressed.
    pub compressed: bool,
    /// `m=1`: more chunks follow.
    pub more: bool,
    /// `q`: 1 keeps OK answers back, 2 every answer.
    pub quiet: u8,
    /// `i`: the image id (0: none).
    pub id: u32,
    /// `I`: the image number (0: none); the terminal picks the id.
    pub number: u32,
    /// `p`: the placement id (0: none).
    pub placement: u32,
    /// `c`, `r`: the placement's size in cells (0: from the image).
    pub cols: u32,
    pub rows: u32,
    /// `x`, `y`, `w`, `h`: the part of the image to show, in pixels (0: all of it).
    pub source_x: u32,
    pub source_y: u32,
    pub source_w: u32,
    pub source_h: u32,
    /// `z`: the stacking order (negative: under the text).
    pub z: i32,
    /// `C=1`: the cursor stays where it was.
    pub cursor_stays: bool,
    /// `U=1`: a virtual placement; the program prints the placeholders itself.
    pub unicode_placeholders: bool,
    /// `d`: what a delete removes (upper case also frees the image).
    pub delete: char,
    /// `S`, `O`: the size and offset of a file or shared memory read (not supported; kept to
    /// answer about them).
    pub size: u32,
    pub offset: u32,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            action: 't',
            format: 32,
            transmission: 'd',
            width: 0,
            height: 0,
            compressed: false,
            more: false,
            quiet: 0,
            id: 0,
            number: 0,
            placement: 0,
            cols: 0,
            rows: 0,
            source_x: 0,
            source_y: 0,
            source_w: 0,
            source_h: 0,
            z: 0,
            cursor_stays: false,
            unicode_placeholders: false,
            delete: 'a',
            size: 0,
            offset: 0,
        }
    }
}

/// Why a command was refused, with the code the protocol names for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A key or value the protocol doesn't allow (`EINVAL`).
    Invalid(&'static str),
    /// More data than this terminal keeps (`EFBIG`).
    TooBig,
    /// No image with that id (`ENOENT`).
    NotFound,
    /// The data couldn't be decoded as the format says (`ENODATA`).
    BadData(String),
    /// A file, shared memory or a feature this terminal doesn't do (`EBADF`, as every file
    /// failure must answer alike).
    Unsupported(&'static str),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::Invalid(_) => "EINVAL",
            Error::TooBig => "EFBIG",
            Error::NotFound => "ENOENT",
            Error::BadData(_) => "ENODATA",
            Error::Unsupported(_) => "EBADF",
        }
    }

    fn message(&self) -> String {
        match self {
            Error::Invalid(what) => what.to_string(),
            Error::TooBig => "the image is bigger than this terminal keeps".to_string(),
            Error::NotFound => "no image with that id".to_string(),
            Error::BadData(why) => why.clone(),
            Error::Unsupported(what) => format!("{what} is not supported"),
        }
    }
}

/// The control data (`key=value,…`), the keys this terminal doesn't know passed over.
pub fn parse(control: &[u8]) -> Result<Control, Error> {
    if control.len() > MAX_CONTROL_BYTES {
        return Err(Error::Invalid("control data too long"));
    }
    let text = std::str::from_utf8(control).map_err(|_| Error::Invalid("control data isn't ASCII"))?;
    let mut out = Control::default();
    for pair in text.split(',').filter(|pair| !pair.is_empty()) {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(Error::Invalid("a key without a value"));
        };
        let number = || value.parse::<i64>().map_err(|_| Error::Invalid("a value that isn't a number"));
        let unsigned =
            || number().and_then(|n| u32::try_from(n).map_err(|_| Error::Invalid("a value out of range")));
        let letter = || {
            let mut chars = value.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => Ok(c),
                _ => Err(Error::Invalid("a value that isn't a letter")),
            }
        };
        match key {
            "a" => out.action = letter()?,
            "f" => out.format = unsigned()?,
            "t" => out.transmission = letter()?,
            "s" => out.width = unsigned()?,
            "v" => out.height = unsigned()?,
            "o" => out.compressed = letter()? == 'z',
            "m" => out.more = unsigned()? == 1,
            "q" => out.quiet = unsigned()?.min(2) as u8,
            "i" => out.id = unsigned()?,
            "I" => out.number = unsigned()?,
            "p" => out.placement = unsigned()?,
            "c" => out.cols = unsigned()?,
            "r" => out.rows = unsigned()?,
            "x" => out.source_x = unsigned()?,
            "y" => out.source_y = unsigned()?,
            "w" => out.source_w = unsigned()?,
            "h" => out.source_h = unsigned()?,
            "z" => out.z = number()?.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            "C" => out.cursor_stays = unsigned()? == 1,
            "U" => out.unicode_placeholders = unsigned()? == 1,
            "d" => out.delete = letter()?,
            "S" => out.size = unsigned()?,
            "O" => out.offset = unsigned()?,
            // Animation, relative placements, usage hints: read so they parse, not acted on.
            _ => {}
        }
    }
    if out.id != 0 && out.number != 0 {
        return Err(Error::Invalid("both an id and a number"));
    }
    Ok(out)
}

/// A payload as kitty sends it: standard base64, padded or not.
fn decode_base64(payload: &[u8]) -> Result<Vec<u8>, Error> {
    let engine = GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );
    engine.decode(payload).map_err(|_| Error::Invalid("the payload isn't base64"))
}

/// A transfer's chunks put back together: the first chunk says everything, the rest only `m`.
#[derive(Default)]
pub struct Transfers {
    pending: Option<(Control, Vec<u8>)>,
}

impl Transfers {
    /// Whether a transfer is waiting for more chunks.
    pub fn in_progress(&self) -> bool {
        self.pending.is_some()
    }

    /// One command with its payload. `Ok(None)` while more chunks are to come; else the command
    /// (the first chunk's keys) with all of its data. A payload over [`MAX_PAYLOAD_BYTES`] ends
    /// the transfer with `EFBIG`.
    pub fn feed(&mut self, control: Control, payload: &[u8]) -> Result<Option<(Control, Vec<u8>)>, Error> {
        let bytes = match decode_base64(payload) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.pending = None;
                return Err(err);
            }
        };
        match self.pending.take() {
            Some((first, mut data)) => {
                if data.len() + bytes.len() > MAX_PAYLOAD_BYTES {
                    return Err(Error::TooBig);
                }
                data.extend_from_slice(&bytes);
                if control.more {
                    self.pending = Some((first, data));
                    Ok(None)
                } else {
                    Ok(Some((first, data)))
                }
            }
            None if control.more => {
                if bytes.len() > MAX_PAYLOAD_BYTES {
                    return Err(Error::TooBig);
                }
                self.pending = Some((control, bytes));
                Ok(None)
            }
            None if bytes.len() > MAX_PAYLOAD_BYTES => Err(Error::TooBig),
            None => Ok(Some((control, bytes))),
        }
    }
}

/// The terminal's answer to a command, as the protocol writes it (`ESC _ G i=<id>;OK ESC \`), or
/// none: a command without an id or number gets no answer, and `q` keeps answers back.
/// `assigned` is the id the terminal chose for a numbered image.
pub fn response(control: &Control, assigned: Option<u32>, result: Result<(), &Error>) -> Option<Vec<u8>> {
    if control.id == 0 && control.number == 0 {
        return None;
    }
    let keep_back = match result {
        Ok(()) => control.quiet >= 1,
        Err(_) => control.quiet >= 2,
    };
    if keep_back {
        return None;
    }
    let mut out = String::from("\x1b_G");
    let id = assigned.unwrap_or(control.id);
    out.push_str(&format!("i={id}"));
    if control.number != 0 {
        out.push_str(&format!(",I={}", control.number));
    }
    out.push(';');
    match result {
        Ok(()) => out.push_str("OK"),
        Err(err) => out.push_str(&format!("{}:{}", err.code(), err.message())),
    }
    out.push_str("\x1b\\");
    Some(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::{Control, Error, Transfers, parse, response};
    use crate::terminal::images::MAX_PAYLOAD_BYTES;

    #[test]
    fn the_keys_read_with_their_defaults() {
        let c = parse(b"a=T,f=100,i=31,s=10,v=20,c=5,r=2,o=z,m=1,q=2,C=1,z=-3,X=4").expect("parses");
        assert_eq!(
            c,
            Control {
                action: 'T',
                format: 100,
                id: 31,
                width: 10,
                height: 20,
                cols: 5,
                rows: 2,
                compressed: true,
                more: true,
                quiet: 2,
                cursor_stays: true,
                z: -3,
                ..Control::default()
            }
        );
        assert_eq!(parse(b""), Ok(Control::default()));
        assert_eq!(parse(b"a=d,d=I,i=4").map(|c| (c.action, c.delete, c.id)), Ok(('d', 'I', 4)));
        for bad in [&b"a"[..], b"i=x", b"i=-1", b"a=TT", b"i=1,I=2", b"s=99999999999"] {
            assert!(matches!(parse(bad), Err(Error::Invalid(_))), "{}", String::from_utf8_lossy(bad));
        }
        let long = vec![b'x'; 2000];
        assert!(parse(&long).is_err());
    }

    #[test]
    fn chunks_are_put_back_together_under_the_cap() {
        let mut transfers = Transfers::default();
        let first = parse(b"a=T,f=24,s=2,v=1,m=1,i=9").expect("parses");
        assert_eq!(transfers.feed(first, b"AAEC"), Ok(None));
        assert!(transfers.in_progress());
        let more = parse(b"m=1").expect("parses");
        assert_eq!(transfers.feed(more, b"AwQ="), Ok(None));
        let last = parse(b"m=0").expect("parses");
        let done = transfers.feed(last, b"BQY").expect("decodes").expect("complete");
        assert_eq!((done.0.id, done.0.action), (9, 'T'), "the first chunk's keys");
        assert_eq!(done.1, [0, 1, 2, 3, 4, 5, 6]);
        assert!(!transfers.in_progress());
        // One piece, unpadded or padded alike.
        let one = transfers.feed(Control::default(), b"aGk").expect("decodes").expect("complete");
        assert_eq!(one.1, b"hi");
        assert!(matches!(transfers.feed(Control::default(), b"!!"), Err(Error::Invalid(_))));
        // Too much: the transfer ends with EFBIG and nothing is kept.
        // Two chunks of a little over half the cap each: the first fits, the second doesn't.
        let big = "A".repeat((MAX_PAYLOAD_BYTES / 6 + 1) * 4);
        let start = Control { more: true, ..Control::default() };
        assert_eq!(transfers.feed(start, big.as_bytes()), Ok(None));
        assert_eq!(
            transfers.feed(Control { more: true, ..Control::default() }, big.as_bytes()),
            Err(Error::TooBig)
        );
        assert!(!transfers.in_progress());
    }

    #[test]
    fn answers_name_the_id_and_the_code_and_keep_quiet_when_asked() {
        let ok = response(&Control { id: 3, ..Control::default() }, None, Ok(()));
        assert_eq!(ok.as_deref(), Some(&b"\x1b_Gi=3;OK\x1b\\"[..]));
        let numbered = response(&Control { number: 5, ..Control::default() }, Some(u32::MAX), Ok(()));
        assert_eq!(numbered.as_deref(), Some(&b"\x1b_Gi=4294967295,I=5;OK\x1b\\"[..]));
        let err = response(&Control { id: 3, ..Control::default() }, None, Err(&Error::NotFound));
        assert_eq!(err.as_deref(), Some(&b"\x1b_Gi=3;ENOENT:no image with that id\x1b\\"[..]));
        assert_eq!(response(&Control::default(), None, Ok(())), None, "nothing to answer to");
        let quiet = Control { id: 3, quiet: 1, ..Control::default() };
        assert_eq!(response(&quiet, None, Ok(())), None);
        assert!(
            response(&quiet, None, Err(&Error::TooBig)).is_some_and(|r| r.starts_with(b"\x1b_Gi=3;EFBIG"))
        );
        let silent = Control { id: 3, quiet: 2, ..Control::default() };
        assert_eq!(response(&silent, None, Err(&Error::TooBig)), None);
        assert_eq!(Error::Unsupported("a file").code(), "EBADF");
    }
}
