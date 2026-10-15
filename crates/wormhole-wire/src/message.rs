//! [`Message`], every message of the protocol, and [`encode`], which frames one into a buffer the
//! caller owns.
//!
//! A message borrows its strings and data from the buffer it was decoded from (or that the sender
//! points it at), so the hot frames cost nothing beyond the copy of their data. The few messages
//! that hold lists (`Spawn`, `Backlog`, `FindResults`) allocate when decoded; they are cold.
//!
//! Layout of a payload: a message's fixed fields, then (for everything except the bulk frames
//! `Input`, `Out`, `Repaint` and `Page`, whose data runs to the end) a tail of TLVs, which a reader
//! skips tag by tag. That is how a minor version adds a field: a new tag, never a new byte in the
//! middle.

use crate::boot::{Hello, Incompatible, Welcome};
use crate::control::{Attach, ClipboardReply, Colors, ErrorMsg, Hook, Limits, Options, Resize, Spawn};
use crate::cursor::{Reader, Writer};
use crate::error::{EncodeError, WireError};
use crate::event::{Event, decode_backlog, encode_backlog};
use crate::header::{HEADER_LEN, Header, MAX_DATA, flags as frame_flags, max_payload, ty};
use crate::history::{FindResults, FindStart, Page, PageGet};
use crate::stream::{Out, Repaint, ResizeApplied, check_data};
use crate::types::{Counters, DetachReason, List};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message<'a> {
    // Bootstrap and admin: frozen.
    Hello(Hello<'a>),
    Welcome(Welcome),
    Incompatible(Incompatible),
    GetInfo,
    /// The same fields as `Welcome`, for an `Admin` connection.
    Info(Welcome),
    Kill,
    Detach,
    Detached(DetachReason),
    Ping(u64),
    Pong {
        n: u64,
        /// A counter the host's PTY thread bumps: a stuck parser is a counter that stops.
        pty_progress: u64,
    },
    Error(ErrorMsg<'a>),

    // Setting up and steering.
    Spawn(Spawn<'a>),
    Attach(Attach),
    Resize(Resize),
    SetColors(Colors<'a>),
    SetLimits(Limits),
    SetOptions(Options),
    /// End the session: hang up the shell, wait `grace_ms` before killing it.
    Close {
        grace_ms: u32,
    },
    ClipboardReply(ClipboardReply<'a>),
    Hook(Hook<'a>),

    // The stream.
    /// Bytes for the program's input; at most 64 KiB.
    Input(&'a [u8]),
    /// The host has written this many input bytes in all to the PTY (cumulative).
    InputAck(u64),
    Out(Out<'a>),
    Repaint(Repaint<'a>),
    ResizeApplied(ResizeApplied),
    /// The client has consumed the stream up to this offset: the host may send up to this plus the window.
    Ack(u64),
    Counters(Counters),
    Event(Event<'a>),
    Backlog(List<'a, Event<'a>>),

    // History and find.
    PageGet(PageGet),
    Page(Page<'a>),
    FindStart(FindStart<'a>),
    FindResults(FindResults<'a>),
    FindCancel {
        id: u32,
    },
}

/// A decoded frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame<'a> {
    pub channel: u16,
    pub flags: u8,
    pub message: Message<'a>,
}

impl Message<'_> {
    /// The frame type id.
    pub fn ty(&self) -> u8 {
        match self {
            Self::Hello(_) => ty::HELLO,
            Self::Welcome(_) => ty::WELCOME,
            Self::Incompatible(_) => ty::INCOMPATIBLE,
            Self::GetInfo => ty::GET_INFO,
            Self::Info(_) => ty::INFO,
            Self::Kill => ty::KILL,
            Self::Detach => ty::DETACH,
            Self::Detached(_) => ty::DETACHED,
            Self::Ping(_) => ty::PING,
            Self::Pong { .. } => ty::PONG,
            Self::Error(_) => ty::ERROR,
            Self::Spawn(_) => ty::SPAWN,
            Self::Attach(_) => ty::ATTACH,
            Self::Resize(_) => ty::RESIZE,
            Self::SetColors(_) => ty::SET_COLORS,
            Self::SetLimits(_) => ty::SET_LIMITS,
            Self::SetOptions(_) => ty::SET_OPTIONS,
            Self::Close { .. } => ty::CLOSE,
            Self::ClipboardReply(_) => ty::CLIPBOARD_REPLY,
            Self::Hook(_) => ty::HOOK,
            Self::Input(_) => ty::INPUT,
            Self::InputAck(_) => ty::INPUT_ACK,
            Self::Out(_) => ty::OUT,
            Self::Repaint(_) => ty::REPAINT,
            Self::ResizeApplied(_) => ty::RESIZE_APPLIED,
            Self::Ack(_) => ty::ACK,
            Self::Counters(_) => ty::COUNTERS,
            Self::Event(_) => ty::EVENT,
            Self::Backlog(_) => ty::BACKLOG,
            Self::PageGet(_) => ty::PAGE_GET,
            Self::Page(_) => ty::PAGE,
            Self::FindStart(_) => ty::FIND_START,
            Self::FindResults(_) => ty::FIND_RESULTS,
            Self::FindCancel { .. } => ty::FIND_CANCEL,
        }
    }

    fn encode_payload(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        match self {
            Self::Hello(m) => m.encode(w)?,
            Self::Welcome(m) | Self::Info(m) => m.encode(w)?,
            Self::Incompatible(m) => m.encode(w),
            Self::GetInfo | Self::Kill | Self::Detach => {}
            Self::Detached(r) => w.u8(r.to_wire()),
            Self::Ping(n) => w.u64(*n),
            Self::Pong { n, pty_progress } => {
                w.u64(*n);
                w.u64(*pty_progress);
            }
            Self::Error(m) => m.encode(w)?,
            Self::Spawn(m) => m.encode(w)?,
            Self::Attach(m) => m.encode(w)?,
            Self::Resize(m) => m.encode(w),
            Self::SetColors(m) => m.write(w)?,
            Self::SetLimits(m) => m.encode(w),
            Self::SetOptions(m) => m.write(w),
            Self::Close { grace_ms } => w.u32(*grace_ms),
            Self::ClipboardReply(m) => m.encode(w)?,
            Self::Hook(m) => m.encode(w)?,
            Self::Input(bytes) => {
                check_data("Input", bytes)?;
                w.raw(bytes);
            }
            Self::InputAck(n) | Self::Ack(n) => w.u64(*n),
            Self::Out(m) => m.encode(w)?,
            Self::Repaint(m) => m.encode(w)?,
            Self::ResizeApplied(m) => m.encode(w),
            Self::Counters(m) => m.write(w),
            Self::Event(m) => m.encode(w)?,
            Self::Backlog(events) => encode_backlog(events, w)?,
            Self::PageGet(m) => m.encode(w),
            Self::Page(m) => m.encode(w)?,
            Self::FindStart(m) => m.encode(w)?,
            Self::FindResults(m) => m.encode(w)?,
            Self::FindCancel { id } => w.u32(*id),
        }
        Ok(())
    }
}

impl<'a> Message<'a> {
    /// Decode one frame's payload. `Ok(None)` is a type this version does not know (the stream
    /// decoder never gets here for those; it skips them unbuffered).
    pub fn decode(t: u8, payload: &'a [u8]) -> Result<Option<Self>, WireError> {
        let mut r = Reader::new(payload, type_name(t));
        let r = &mut r;
        Ok(Some(match t {
            ty::HELLO => Self::Hello(Hello::decode(payload)?),
            ty::WELCOME => Self::Welcome(Welcome::decode(payload, "Welcome")?),
            ty::INCOMPATIBLE => Self::Incompatible(Incompatible::decode(payload)?),
            ty::GET_INFO => Self::GetInfo,
            ty::INFO => Self::Info(Welcome::decode(payload, "Info")?),
            ty::KILL => Self::Kill,
            ty::DETACH => Self::Detach,
            ty::DETACHED => Self::Detached(DetachReason::from_wire(r.u8()?)),
            ty::PING => Self::Ping(r.u64()?),
            ty::PONG => Self::Pong { n: r.u64()?, pty_progress: r.u64()? },
            ty::ERROR => Self::Error(ErrorMsg::decode(r)?),
            ty::SPAWN => Self::Spawn(Spawn::decode(r)?),
            ty::ATTACH => Self::Attach(Attach::decode(r)?),
            ty::RESIZE => Self::Resize(Resize::decode(r)?),
            ty::SET_COLORS => Self::SetColors(Colors::read(r)?),
            ty::SET_LIMITS => Self::SetLimits(Limits::decode(r)?),
            ty::SET_OPTIONS => Self::SetOptions(Options::read(r)?),
            ty::CLOSE => Self::Close { grace_ms: r.u32()? },
            ty::CLIPBOARD_REPLY => Self::ClipboardReply(ClipboardReply::decode(r)?),
            ty::HOOK => Self::Hook(Hook::decode(r)?),
            ty::INPUT => {
                if payload.len() > MAX_DATA {
                    return Err(WireError::BadValue("Input"));
                }
                Self::Input(payload)
            }
            ty::INPUT_ACK => Self::InputAck(r.u64()?),
            ty::OUT => Self::Out(Out::decode(payload)?),
            ty::REPAINT => Self::Repaint(Repaint::decode(payload)?),
            ty::RESIZE_APPLIED => Self::ResizeApplied(ResizeApplied::decode(r)?),
            ty::ACK => Self::Ack(r.u64()?),
            ty::COUNTERS => Self::Counters(Counters::read(r)?),
            ty::EVENT => Self::Event(Event::decode(payload)?),
            ty::BACKLOG => Self::Backlog(decode_backlog(payload)?),
            ty::PAGE_GET => Self::PageGet(PageGet::decode(r)?),
            ty::PAGE => Self::Page(Page::decode(payload)?),
            ty::FIND_START => Self::FindStart(FindStart::decode(r)?),
            ty::FIND_RESULTS => Self::FindResults(FindResults::decode(r)?),
            ty::FIND_CANCEL => Self::FindCancel { id: r.u32()? },
            _ => return Ok(None),
        }))
    }
}

/// The message's name, for errors.
fn type_name(t: u8) -> &'static str {
    match t {
        ty::HELLO => "Hello",
        ty::WELCOME => "Welcome",
        ty::INCOMPATIBLE => "Incompatible",
        ty::GET_INFO => "GetInfo",
        ty::INFO => "Info",
        ty::KILL => "Kill",
        ty::DETACH => "Detach",
        ty::DETACHED => "Detached",
        ty::PING => "Ping",
        ty::PONG => "Pong",
        ty::ERROR => "Error",
        ty::SPAWN => "Spawn",
        ty::ATTACH => "Attach",
        ty::RESIZE => "Resize",
        ty::SET_COLORS => "SetColors",
        ty::SET_LIMITS => "SetLimits",
        ty::SET_OPTIONS => "SetOptions",
        ty::CLOSE => "Close",
        ty::CLIPBOARD_REPLY => "ClipboardReply",
        ty::HOOK => "Hook",
        ty::INPUT => "Input",
        ty::INPUT_ACK => "InputAck",
        ty::OUT => "Out",
        ty::REPAINT => "Repaint",
        ty::RESIZE_APPLIED => "ResizeApplied",
        ty::ACK => "Ack",
        ty::COUNTERS => "Counters",
        ty::EVENT => "Event",
        ty::BACKLOG => "Backlog",
        ty::PAGE_GET => "PageGet",
        ty::PAGE => "Page",
        ty::FIND_START => "FindStart",
        ty::FIND_RESULTS => "FindResults",
        ty::FIND_CANCEL => "FindCancel",
        _ => "unknown message",
    }
}

/// Append `msg` as one frame on `channel` to `buf`, which the caller keeps and reuses (clear it
/// after writing it out; it grows once to the largest frame and then stays). On error `buf` is as
/// it was. `flags` may carry `URGENT`; `COMPRESSED` is refused, because this build never
/// compresses.
pub fn encode(buf: &mut Vec<u8>, channel: u16, flags: u8, msg: &Message<'_>) -> Result<(), EncodeError> {
    if flags & frame_flags::COMPRESSED != 0 {
        return Err(EncodeError::BadValue("frame flags"));
    }
    let start = buf.len();
    buf.extend_from_slice(&[0; HEADER_LEN]);
    let result = msg.encode_payload(&mut Writer::new(buf));
    let t = msg.ty();
    let len = buf.len() - start - HEADER_LEN;
    let max = max_payload(t).unwrap_or(0);
    let outcome = match result {
        Err(e) => Err(e),
        Ok(()) if len > max => Err(EncodeError::PayloadTooLarge { ty: t, len, max }),
        Ok(()) => Ok(()),
    };
    if outcome.is_err() {
        buf.truncate(start);
        return outcome;
    }
    let header = Header { len: len as u32, ty: t, flags, channel };
    buf[start..start + HEADER_LEN].copy_from_slice(&header.to_bytes());
    Ok(())
}
