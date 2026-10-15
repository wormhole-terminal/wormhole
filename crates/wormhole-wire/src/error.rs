//! The two error types: what a decoder can find wrong in a peer's bytes, and what an encoder
//! refuses to write. Every failure has its own variant; nothing is a string to match on.

use std::fmt;

/// Something is wrong with bytes that came from the peer.
///
/// Two kinds, told apart by [`WireError::is_fatal`]. A *fatal* error means the stream's framing can no
/// longer be trusted (a length past the maximum, a flag this side never negotiated): the decoder
/// refuses everything after it and the connection must be closed. Any other error is about one
/// frame's payload; the frame was consumed whole, the stream is still in step, and the caller
/// decides whether the peer deserves to stay connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    /// A header names a payload longer than the maximum for its type (or for the connection
    /// before its handshake). Fatal, and checked before any byte of the payload is buffered.
    FrameTooLarge { ty: u8, len: u32, max: u32 },
    /// The `COMPRESSED` flag, which this build never negotiates. Fatal.
    Compressed,
    /// The first four bytes of a bootstrap message are not `WHSP`.
    BadMagic,
    /// The payload ends before a field of the named message does.
    Truncated(&'static str),
    /// A string field of the named message is not UTF-8.
    BadUtf8(&'static str),
    /// A field of the named message holds a value its meaning forbids.
    BadValue(&'static str),
    /// A field the named message requires is absent.
    MissingField(&'static str),
    /// A field that may appear once appears twice in the named message.
    DuplicateField(&'static str),
    /// A list in the named message is longer than the protocol allows.
    TooMany(&'static str),
}

impl WireError {
    /// The framing is lost: close the connection, do not read on.
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::FrameTooLarge { .. } | Self::Compressed)
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooLarge { ty, len, max } => {
                write!(f, "frame type {ty:#04x} announces {len} bytes, at most {max} are allowed")
            }
            Self::Compressed => f.write_str("compressed frame, but compression was not negotiated"),
            Self::BadMagic => f.write_str("not a Wormhole session protocol message (bad magic)"),
            Self::Truncated(what) => write!(f, "{what}: payload ends inside a field"),
            Self::BadUtf8(what) => write!(f, "{what}: a string is not valid UTF-8"),
            Self::BadValue(what) => write!(f, "{what}: a field has an impossible value"),
            Self::MissingField(what) => write!(f, "{what}: a required field is missing"),
            Self::DuplicateField(what) => write!(f, "{what}: a field appears twice"),
            Self::TooMany(what) => write!(f, "{what}: a list is longer than the protocol allows"),
        }
    }
}

impl std::error::Error for WireError {}

/// A message the encoder would not write. The buffer is left as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// A string longer than its `u16` length prefix can say.
    StringTooLong(&'static str),
    /// A list longer than the protocol allows.
    TooMany(&'static str),
    /// A data field (output, input, a repaint, a page) over [`crate::MAX_DATA`].
    DataTooLarge(&'static str),
    /// The whole payload would pass the maximum for its type.
    PayloadTooLarge { ty: u8, len: usize, max: usize },
    /// A field whose value the protocol forbids (an empty version range, a zero channel count).
    BadValue(&'static str),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StringTooLong(what) => write!(f, "{what}: string longer than 65,535 bytes"),
            Self::TooMany(what) => write!(f, "{what}: too many items"),
            Self::DataTooLarge(what) => write!(f, "{what}: more than one frame of data"),
            Self::PayloadTooLarge { ty, len, max } => {
                write!(f, "frame type {ty:#04x}: payload of {len} bytes, at most {max}")
            }
            Self::BadValue(what) => write!(f, "{what}: impossible value"),
        }
    }
}

impl std::error::Error for EncodeError {}
