//! The output stream and its flow control: `Out`, `Repaint`, `ResizeApplied`, `Input`, the acks
//! and `Counters`, and [`StreamOrder`], which a receiver uses to check that the stream arrives in
//! the order the host promised.
//!
//! Every stream frame carries a **byte offset**: the count of PTY bytes the host had parsed when
//! it was made. `Out{offset, bytes}` covers `offset..offset+len`; a repaint and a resize are
//! stamped with the offset they are current as of, and are *in-band*: they arrive between exactly
//! the bytes they were applied between. The hot frames (`Out`, `Input`) have a fixed prefix and
//! then the data to the end of the payload, with no tail, so they encode and decode without a copy
//! beyond the data itself.

use crate::cursor::{Reader, Writer};
use crate::error::{EncodeError, WireError};
use crate::header::MAX_DATA;
use crate::types::{Size, wire_enum};

/// Output the program wrote: `bytes` are stream bytes `offset..offset + bytes.len()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Out<'a> {
    pub offset: u64,
    pub bytes: &'a [u8],
}

impl<'a> Out<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        check_data("Out", self.bytes)?;
        w.u64(self.offset);
        w.raw(self.bytes);
        Ok(())
    }

    pub(crate) fn decode(payload: &'a [u8]) -> Result<Self, WireError> {
        let mut r = Reader::new(payload, "Out");
        let offset = r.u64()?;
        Ok(Self { offset, bytes: r.rest() })
    }

    /// One past the last byte this frame covers.
    pub fn end(&self) -> u64 {
        self.offset.saturating_add(self.bytes.len() as u64)
    }
}

wire_enum! {
    /// How much a repaint redraws.
    RepaintKind: u8 {
        /// The whole terminal, history included up to the attach cap.
        Full = 0,
        /// Only the rows damaged since the last frame sent to this client.
        Rows = 1,
    }
}

const PART_FIRST: u8 = 1;
const PART_LAST: u8 = 2;

/// VT bytes that, parsed by a replica, bring it to the host's state at `offset`. A repaint may
/// be longer than one frame: it is then several `Repaint` frames with the same `epoch`, `offset`,
/// `kind` and `history_gap`, the first with `first`, the last with `last`; control frames may come
/// between the parts, `Out` may not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repaint<'a> {
    pub epoch: u32,
    pub offset: u64,
    pub kind: RepaintKind,
    /// The repaint skipped output: the replica's history before it is not contiguous.
    pub history_gap: bool,
    pub first: bool,
    pub last: bool,
    pub bytes: &'a [u8],
}

impl<'a> Repaint<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        check_data("Repaint", self.bytes)?;
        if !self.first && !self.last && self.bytes.is_empty() {
            // A middle part with nothing in it is a sender's bug, and indistinguishable from padding.
            return Err(EncodeError::BadValue("Repaint part"));
        }
        w.u32(self.epoch);
        w.u64(self.offset);
        w.u8(self.kind.to_wire());
        w.u8((u8::from(self.first) * PART_FIRST) | (u8::from(self.last) * PART_LAST));
        w.bool(self.history_gap);
        w.u8(0);
        w.raw(self.bytes);
        Ok(())
    }

    pub(crate) fn decode(payload: &'a [u8]) -> Result<Self, WireError> {
        let mut r = Reader::new(payload, "Repaint");
        let epoch = r.u32()?;
        let offset = r.u64()?;
        let kind = RepaintKind::from_wire(r.u8()?);
        let part = r.u8()?;
        let history_gap = r.bool()?;
        r.skip(1)?;
        if part & (PART_FIRST | PART_LAST) == 0 && r.remaining() == 0 {
            return Err(WireError::BadValue("Repaint"));
        }
        Ok(Self {
            epoch,
            offset,
            kind,
            history_gap,
            first: part & PART_FIRST != 0,
            last: part & PART_LAST != 0,
            bytes: r.rest(),
        })
    }
}

/// The host applied a resize between two slices of PTY output, at stream offset `offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResizeApplied {
    pub offset: u64,
    /// The history's numbering restarts at this epoch.
    pub epoch: u32,
    /// The requester's `Resize.req`; 0 when the program or the host resized.
    pub req: u32,
    pub size: Size,
}

impl ResizeApplied {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) {
        w.u64(self.offset);
        w.u32(self.epoch);
        w.u32(self.req);
        self.size.write(w);
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { offset: r.u64()?, epoch: r.u32()?, req: r.u32()?, size: Size::read(r)? })
    }
}

/// A chunk of terminal data (also `Input` and `Page`) is at most [`MAX_DATA`].
pub(crate) fn check_data(field: &'static str, bytes: &[u8]) -> Result<(), EncodeError> {
    if bytes.len() > MAX_DATA { Err(EncodeError::DataTooLarge(field)) } else { Ok(()) }
}

/// A frame arrived out of the order the host promised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderError {
    /// `Out` or a resize starts at `got`, but the stream was at `expected`: bytes are missing or repeated.
    Gap { expected: u64, got: u64 },
    /// A repaint is stamped before the point the stream has reached.
    Backwards { at: u64, got: u64 },
    /// `Out` (or a resize) arrived inside a repaint that has not ended.
    InsideRepaint,
    /// A repaint part that does not continue the one before it.
    BrokenRepaint,
}

/// Checks the order of one attachment's stream frames: `Out` contiguous, repaints and resizes
/// at the point the stream has reached, repaint parts whole. A receiver feeds it every `Out`,
/// `Repaint` and `ResizeApplied` it gets and treats an error as a lost stream (ask for a repaint
/// or reconnect). The first frame sets the starting point, because an attachment may begin at a
/// repaint, at a resume offset, or (headless) at a resize.
#[derive(Clone, Debug, Default)]
pub struct StreamOrder {
    next: Option<u64>,
    open: Option<(u32, u64, RepaintKind, bool)>,
}

impl StreamOrder {
    pub fn new() -> Self {
        Self::default()
    }

    /// The offset the next `Out` must start at, once known.
    pub fn next_offset(&self) -> Option<u64> {
        self.next
    }

    pub fn out(&mut self, o: &Out<'_>) -> Result<(), OrderError> {
        if self.open.is_some() {
            return Err(OrderError::InsideRepaint);
        }
        if let Some(expected) = self.next
            && expected != o.offset
        {
            return Err(OrderError::Gap { expected, got: o.offset });
        }
        self.next = Some(o.end());
        Ok(())
    }

    pub fn repaint(&mut self, r: &Repaint<'_>) -> Result<(), OrderError> {
        match (self.open, r.first) {
            (None, false) | (Some(_), true) => return Err(OrderError::BrokenRepaint),
            (Some(same), false) if same != (r.epoch, r.offset, r.kind, r.history_gap) => {
                return Err(OrderError::BrokenRepaint);
            }
            _ => {}
        }
        if r.first
            && let Some(at) = self.next
            && r.offset < at
        {
            return Err(OrderError::Backwards { at, got: r.offset });
        }
        if r.last {
            self.open = None;
            self.next = Some(r.offset);
        } else {
            self.open = Some((r.epoch, r.offset, r.kind, r.history_gap));
        }
        Ok(())
    }

    pub fn resize_applied(&mut self, a: &ResizeApplied) -> Result<(), OrderError> {
        if self.open.is_some() {
            return Err(OrderError::InsideRepaint);
        }
        if let Some(expected) = self.next
            && expected != a.offset
        {
            return Err(OrderError::Gap { expected, got: a.offset });
        }
        self.next = Some(a.offset);
        Ok(())
    }
}
