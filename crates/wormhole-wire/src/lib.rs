//! The wire protocol between the Wormhole app and a terminal's session host (`wormhole-host`).
//!
//! Design: `../repo-docs/docs/session-host.md`, section 7. This crate is the codec and the rules
//! around it, with no I/O of its own: bytes in, messages out, and the other way round.
//!
//! * [`header`]: the 8-byte frame header (`len u32 | type u8 | flags u8 | channel u16`, little
//!   endian), the type ids and the size limits.
//! * [`Message`] and [`encode`]: every message, framed into a buffer the caller reuses.
//! * [`Decoder`]: incremental, bounded and panic-free, for a hostile byte stream.
//! * [`boot`] and [`version`]: the frozen `Hello`/`Welcome` bootstrap and the version rules.
//! * [`io`]: the same codec over any `Read` and `Write` (a Unix socket now, SSH stdio later).
//! * [`StreamOrder`]: checks that output arrives contiguously and repaints and resizes in-band.
//!
//! `#![forbid(unsafe_code)]`, no dependencies, no serde, no async runtime; it builds on Linux too.
#![forbid(unsafe_code)]

pub mod boot;
mod control;
mod cursor;
mod decoder;
mod error;
mod event;
mod header;
mod history;
pub mod io;
mod message;
pub mod samples;
mod stream;
mod types;
pub mod version;

pub use boot::{BOOT_FIXED, Hello, Incompatible, MAGIC, MAX_TICKET, Welcome};
pub use control::{
    Attach, ClipboardReply, Colors, ErrorMsg, Hook, Limits, MAX_PALETTE, MAX_SPAWN_ITEMS, Options, Resize,
    Spawn, error_code, option_flags,
};
pub use decoder::Decoder;
pub use error::{EncodeError, WireError};
pub use event::{Event, EventKind, MAX_BACKLOG, Mark, MarkKind, ProgressState};
pub use header::{
    CHANNEL_CONNECTION, CHANNEL_LOCAL, HEADER_LEN, Header, MAX_CONTROL, MAX_DATA, MAX_PAYLOAD, OUT_PREFIX,
    PAGE_PREFIX, REPAINT_PREFIX, flags, max_payload, ty,
};
pub use history::{
    FindDirection, FindMatch, FindResults, FindStart, MAX_MATCHES_PER_FRAME, Page, PageGet, find_flags,
};
pub use message::{Frame, Message, encode};
pub use stream::{OrderError, Out, Repaint, RepaintKind, ResizeApplied, StreamOrder};
pub use types::{
    AppVersion, ColorScheme, Counters, DetachReason, ExitInfo, Features, HostState, List, Rgb, Role, Size,
    StreamMode,
};
