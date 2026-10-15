//! History pages and find, both executed on the host (from protocol minor 0, behind
//! [`crate::Features::HISTORY_PAGES`] and [`crate::Features::FIND`]).
//!
//! Lines are numbered from the oldest line the host retains, per `epoch`; a resize that reflows
//! the history starts a new epoch, and a request or result of an old one is stale.

use crate::cursor::{Reader, Writer};
use crate::error::{EncodeError, WireError};
use crate::stream::check_data;
use crate::types::{List, wire_enum};

/// Bytes before the data in a `Page` payload.
pub const PAGE_PREFIX: usize = 16;
/// The most matches one `FindResults` frame carries; more come in further frames (`more`).
pub const MAX_MATCHES_PER_FRAME: usize = 2048;

/// Ask for `count` history lines starting at line `from`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageGet {
    pub epoch: u32,
    pub from: u64,
    pub count: u32,
}

impl PageGet {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) {
        w.u32(self.epoch);
        w.u64(self.from);
        w.u32(self.count);
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { epoch: r.u32()?, from: r.u64()?, count: r.u32()? })
    }
}

/// The answer: `count` lines (possibly fewer than asked, so that they fit one frame) as VT bytes
/// with soft wraps joined, which the client parses into a scratch grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page<'a> {
    pub epoch: u32,
    pub from: u64,
    pub count: u32,
    pub bytes: &'a [u8],
}

impl<'a> Page<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        check_data("Page", self.bytes)?;
        w.u32(self.epoch);
        w.u64(self.from);
        w.u32(self.count);
        w.raw(self.bytes);
        Ok(())
    }

    pub(crate) fn decode(payload: &'a [u8]) -> Result<Self, WireError> {
        let mut r = Reader::new(payload, "Page");
        let (epoch, from, count) = (r.u32()?, r.u64()?, r.u32()?);
        Ok(Self { epoch, from, count, bytes: r.rest() })
    }
}

wire_enum! {
    /// Which way a search runs from its anchor.
    FindDirection: u8 {
        Backward = 0,
        Forward = 1,
    }
}

/// `FindStart.flags`.
pub mod find_flags {
    pub const CASE_SENSITIVE: u8 = 1 << 0;
    pub const REGEX: u8 = 1 << 1;
    pub const WHOLE_WORD: u8 = 1 << 2;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindStart<'a> {
    /// The client's id for this search; results and cancel carry it.
    pub id: u32,
    pub flags: u8,
    pub direction: FindDirection,
    pub epoch: u32,
    pub anchor_line: u64,
    pub anchor_col: u32,
    pub pattern: &'a str,
}

impl<'a> FindStart<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u32(self.id);
        w.u8(self.flags);
        w.u8(self.direction.to_wire());
        w.u32(self.epoch);
        w.u64(self.anchor_line);
        w.u32(self.anchor_col);
        w.str16("FindStart.pattern", self.pattern)
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        Ok(Self {
            id: r.u32()?,
            flags: r.u8()?,
            direction: FindDirection::from_wire(r.u8()?),
            epoch: r.u32()?,
            anchor_line: r.u64()?,
            anchor_col: r.u32()?,
            pattern: r.str16()?,
        })
    }
}

/// One match: where it starts, how many cells it covers and its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindMatch<'a> {
    pub line: u64,
    pub col: u32,
    pub cells: u32,
    pub text: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindResults<'a> {
    pub id: u32,
    pub epoch: u32,
    /// Further `FindResults` for this search follow.
    pub more: bool,
    pub matches: List<'a, FindMatch<'a>>,
}

impl<'a> FindResults<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        if self.matches.len() > MAX_MATCHES_PER_FRAME {
            return Err(EncodeError::TooMany("FindResults"));
        }
        w.u32(self.id);
        w.u32(self.epoch);
        w.bool(self.more);
        w.u16(self.matches.len() as u16);
        for m in self.matches.iter() {
            w.u64(m.line);
            w.u32(m.col);
            w.u32(m.cells);
            w.str16("FindResults.text", m.text)?;
        }
        Ok(())
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        let (id, epoch, more) = (r.u32()?, r.u32()?, r.bool()?);
        let count = usize::from(r.u16()?);
        if count > MAX_MATCHES_PER_FRAME {
            return Err(WireError::TooMany("FindResults"));
        }
        // Each match is at least 18 bytes, so a short payload cannot make this allocation large.
        let mut matches = Vec::with_capacity(count.min(r.remaining() / 18));
        for _ in 0..count {
            matches.push(FindMatch { line: r.u64()?, col: r.u32()?, cells: r.u32()?, text: r.str16()? });
        }
        Ok(Self { id, epoch, more, matches: List::Owned(matches) })
    }
}
