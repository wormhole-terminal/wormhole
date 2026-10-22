//! The 8-byte frame header, the message type ids and the size limits that go with them.
//!
//! ```text
//!  0       4     5      6        8
//!  +-------+-----+------+--------+----------------+
//!  | len   | ty  | flag | chan   | payload (len)  |   all little-endian
//!  +-------+-----+------+--------+----------------+
//! ```
//!
//! `len` counts the payload only. It is checked against a per-type maximum before the payload is
//! buffered, so a hostile length costs the receiver nothing.

/// Bytes in a frame header.
pub const HEADER_LEN: usize = 8;
/// The largest payload any frame may carry (1 MiB). A longer one closes the connection.
pub const MAX_PAYLOAD: usize = 1 << 20;
/// The largest chunk of terminal data (output, input, a repaint part, a page): 64 KiB, so that no
/// control frame ever waits behind more than this on the link.
pub const MAX_DATA: usize = 64 * 1024;
/// The largest payload of a small control message, and of anything before the handshake is done.
pub const MAX_CONTROL: usize = 64 * 1024;
/// Bytes before the data in an `Out` payload (`stream`).
pub const OUT_PREFIX: usize = 8;
/// Bytes before the data in a `Repaint` payload (`stream`).
pub const REPAINT_PREFIX: usize = 16;
/// Bytes before the data in a `Page` payload (`history`).
pub const PAGE_PREFIX: usize = 16;

/// Channel 0 is the connection itself (keep-alive between relays); 1 is the only terminal a local
/// socket carries; an SSH relay hands out the rest.
pub const CHANNEL_CONNECTION: u16 = 0;
/// The terminal channel of a local socket.
pub const CHANNEL_LOCAL: u16 = 1;

/// Frame flags. A flag that changes how a payload reads must be negotiated through
/// [`crate::Features`] before it is sent; the others are advice, and a bit this version does not
/// know is ignored.
pub mod flags {
    /// The payload is LZ4-compressed (negotiated; this build never sets or accepts it).
    pub const COMPRESSED: u8 = 1 << 0;
    /// Handle ahead of queued bulk data: keystrokes, acks, pings.
    pub const URGENT: u8 = 1 << 1;
}

/// A parsed header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub len: u32,
    pub ty: u8,
    pub flags: u8,
    pub channel: u16,
}

impl Header {
    pub fn parse(b: &[u8; HEADER_LEN]) -> Self {
        Self {
            len: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            ty: b[4],
            flags: b[5],
            channel: u16::from_le_bytes([b[6], b[7]]),
        }
    }

    pub fn to_bytes(self) -> [u8; HEADER_LEN] {
        let l = self.len.to_le_bytes();
        let c = self.channel.to_le_bytes();
        [l[0], l[1], l[2], l[3], self.ty, self.flags, c[0], c[1]]
    }
}

/// Message type ids. Never reused, never renumbered. `0x01..=0x0F` is the bootstrap and admin set,
/// frozen across majors so any version can talk to any other far enough to say "too old".
pub mod ty {
    pub const HELLO: u8 = 0x01;
    pub const WELCOME: u8 = 0x02;
    pub const INCOMPATIBLE: u8 = 0x03;
    pub const GET_INFO: u8 = 0x04;
    pub const INFO: u8 = 0x05;
    pub const KILL: u8 = 0x06;
    pub const DETACH: u8 = 0x07;
    pub const DETACHED: u8 = 0x08;
    pub const PING: u8 = 0x09;
    pub const PONG: u8 = 0x0A;
    pub const ERROR: u8 = 0x0B;

    pub const SPAWN: u8 = 0x10;
    pub const ATTACH: u8 = 0x11;
    pub const RESIZE: u8 = 0x12;
    pub const SET_COLORS: u8 = 0x13;
    pub const SET_LIMITS: u8 = 0x14;
    pub const SET_OPTIONS: u8 = 0x15;
    pub const CLOSE: u8 = 0x16;
    pub const CLIPBOARD_REPLY: u8 = 0x17;
    pub const HOOK: u8 = 0x18;
    pub const SET_COLOR_SCHEME: u8 = 0x19;

    pub const INPUT: u8 = 0x20;
    pub const INPUT_ACK: u8 = 0x21;
    pub const OUT: u8 = 0x22;
    pub const REPAINT: u8 = 0x23;
    pub const RESIZE_APPLIED: u8 = 0x24;
    pub const ACK: u8 = 0x25;
    pub const COUNTERS: u8 = 0x26;
    pub const EVENT: u8 = 0x27;
    pub const BACKLOG: u8 = 0x28;

    pub const PAGE_GET: u8 = 0x30;
    pub const PAGE: u8 = 0x31;
    pub const FIND_START: u8 = 0x32;
    pub const FIND_RESULTS: u8 = 0x33;
    pub const FIND_CANCEL: u8 = 0x34;
}

/// The longest payload a frame of type `t` may have, or `None` when this version does not know
/// the type (the frame is then skipped, up to [`MAX_PAYLOAD`]).
pub fn max_payload(t: u8) -> Option<usize> {
    Some(match t {
        ty::HELLO
        | ty::WELCOME
        | ty::INCOMPATIBLE
        | ty::GET_INFO
        | ty::INFO
        | ty::KILL
        | ty::DETACH
        | ty::DETACHED
        | ty::PING
        | ty::PONG
        | ty::ERROR
        | ty::ATTACH
        | ty::RESIZE
        | ty::SET_COLORS
        | ty::SET_LIMITS
        | ty::SET_OPTIONS
        | ty::SET_COLOR_SCHEME
        | ty::CLOSE
        | ty::INPUT_ACK
        | ty::RESIZE_APPLIED
        | ty::ACK
        | ty::COUNTERS
        | ty::PAGE_GET
        | ty::FIND_START
        | ty::FIND_CANCEL => MAX_CONTROL,
        ty::SPAWN | ty::CLIPBOARD_REPLY | ty::HOOK | ty::EVENT | ty::BACKLOG | ty::FIND_RESULTS => {
            MAX_PAYLOAD
        }
        // The data frames: a fixed prefix and then at most one chunk.
        ty::INPUT => MAX_DATA,
        ty::OUT => OUT_PREFIX + MAX_DATA,
        ty::REPAINT => REPAINT_PREFIX + MAX_DATA,
        ty::PAGE => PAGE_PREFIX + MAX_DATA,
        _ => return None,
    })
}
