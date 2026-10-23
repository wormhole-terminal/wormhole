//! `Event`: what the host saw the program do that is not screen content, and `Backlog`, the
//! bounded list of them replayed to a client that attaches late. Events carry the stream offset
//! they happened at, so a client can order them against the bytes.

use crate::cursor::{Reader, Writer};
use crate::error::{EncodeError, WireError};
use crate::types::{ExitInfo, List, wire_enum};

/// The most events one `Backlog` may carry.
pub const MAX_BACKLOG: usize = 4096;

wire_enum! {
    /// An OSC 133 prompt mark.
    MarkKind: u8 {
        PromptStart = 1,
        CommandStart = 2,
        OutputStart = 3,
        CommandEnd = 4,
    }
}

wire_enum! {
    /// OSC 9;4 progress states.
    ProgressState: u8 {
        Remove = 0,
        Set = 1,
        Error = 2,
        Indeterminate = 3,
        Paused = 4,
    }
}

/// A prompt mark with the block it belongs to (ids agree with the host's command log).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    pub kind: MarkKind,
    pub block: u64,
    /// The command's exit status, on `CommandEnd` when the shell reported one.
    pub exit_code: Option<i32>,
    /// Milliseconds since the Unix epoch.
    pub at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind<'a> {
    Title(&'a str),
    ResetTitle,
    Bell,
    Cwd(&'a str),
    Mark(Mark),
    /// OSC 9, 99 or 777; `source` says which.
    Notify {
        source: u8,
        title: &'a str,
        body: &'a str,
    },
    Progress {
        state: ProgressState,
        percent: u8,
    },
    /// OSC 7501: the program says what it is doing.
    ProgramStatus(&'a str),
    /// OSC 52 write: `selection` is the selection letter (`c`, `p`, `s`).
    ClipboardStore {
        selection: u8,
        data: &'a [u8],
    },
    /// OSC 52 read: answer with `ClipboardReply{ticket}`; none in time means "empty".
    ClipboardLoad {
        ticket: u32,
        selection: u8,
    },
    /// An xterm window operation the program asked for (`CSI t`), as its code and two arguments.
    WindowOp {
        op: u8,
        a: u32,
        b: u32,
    },
    Exited(ExitInfo),
    /// RIS (`ESC c`): the program reset the terminal. The replica's own events are dropped, so
    /// this is how the client learns to drop what programs reported about themselves.
    FullReset,
    /// The counters VT has no sequence for, sent just before a `Repaint` (same offset): the client
    /// seeds its replica with them before parsing the repaint, so its block numbers, generated
    /// link ids and picture handles are the host's.
    RepaintSeeds {
        block: u32,
        link_seq: u64,
        image_handle: u64,
    },
    /// An event kind from a newer minor: skip it.
    Unknown(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event<'a> {
    /// The stream offset the event happened at.
    pub offset: u64,
    pub kind: EventKind<'a>,
}

mod k {
    pub(super) const TITLE: u8 = 1;
    pub(super) const RESET_TITLE: u8 = 2;
    pub(super) const BELL: u8 = 3;
    pub(super) const CWD: u8 = 4;
    pub(super) const MARK: u8 = 5;
    pub(super) const NOTIFY: u8 = 6;
    pub(super) const PROGRESS: u8 = 7;
    pub(super) const PROGRAM_STATUS: u8 = 8;
    pub(super) const CLIPBOARD_STORE: u8 = 9;
    pub(super) const CLIPBOARD_LOAD: u8 = 10;
    pub(super) const WINDOW_OP: u8 = 11;
    pub(super) const EXITED: u8 = 12;
    pub(super) const FULL_RESET: u8 = 13;
    pub(super) const REPAINT_SEEDS: u8 = 14;
}

impl<'a> Event<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u64(self.offset);
        match self.kind {
            EventKind::Title(t) => {
                w.u8(k::TITLE);
                w.str16("Event.title", t)?;
            }
            EventKind::ResetTitle => w.u8(k::RESET_TITLE),
            EventKind::Bell => w.u8(k::BELL),
            EventKind::Cwd(p) => {
                w.u8(k::CWD);
                w.str16("Event.cwd", p)?;
            }
            EventKind::Mark(m) => {
                w.u8(k::MARK);
                w.u8(m.kind.to_wire());
                w.u64(m.block);
                w.bool(m.exit_code.is_some());
                w.i32(m.exit_code.unwrap_or(0));
                w.u64(m.at_ms);
            }
            EventKind::Notify { source, title, body } => {
                w.u8(k::NOTIFY);
                w.u8(source);
                w.str16("Event.notify title", title)?;
                w.str16("Event.notify body", body)?;
            }
            EventKind::Progress { state, percent } => {
                w.u8(k::PROGRESS);
                w.u8(state.to_wire());
                w.u8(percent);
            }
            EventKind::ProgramStatus(s) => {
                w.u8(k::PROGRAM_STATUS);
                w.str16("Event.program status", s)?;
            }
            EventKind::ClipboardStore { selection, data } => {
                w.u8(k::CLIPBOARD_STORE);
                w.u8(selection);
                w.bytes32("Event.clipboard", data)?;
            }
            EventKind::ClipboardLoad { ticket, selection } => {
                w.u8(k::CLIPBOARD_LOAD);
                w.u32(ticket);
                w.u8(selection);
            }
            EventKind::WindowOp { op, a, b } => {
                w.u8(k::WINDOW_OP);
                w.u8(op);
                w.u32(a);
                w.u32(b);
            }
            EventKind::Exited(x) => {
                w.u8(k::EXITED);
                x.write(w);
            }
            EventKind::FullReset => w.u8(k::FULL_RESET),
            EventKind::RepaintSeeds { block, link_seq, image_handle } => {
                w.u8(k::REPAINT_SEEDS);
                w.u32(block);
                w.u64(link_seq);
                w.u64(image_handle);
            }
            // Re-encoding a kind we cannot read would invent a body for it.
            EventKind::Unknown(_) => return Err(EncodeError::BadValue("Event kind")),
        }
        Ok(())
    }

    pub(crate) fn decode(payload: &'a [u8]) -> Result<Self, WireError> {
        const WHAT: &str = "Event";
        let mut r = Reader::new(payload, WHAT);
        let offset = r.u64()?;
        let kind = match r.u8()? {
            k::TITLE => EventKind::Title(r.str16()?),
            k::RESET_TITLE => EventKind::ResetTitle,
            k::BELL => EventKind::Bell,
            k::CWD => EventKind::Cwd(r.str16()?),
            k::MARK => {
                let kind = MarkKind::from_wire(r.u8()?);
                let block = r.u64()?;
                let has_exit = r.bool()?;
                let code = r.i32()?;
                EventKind::Mark(Mark { kind, block, exit_code: has_exit.then_some(code), at_ms: r.u64()? })
            }
            k::NOTIFY => EventKind::Notify { source: r.u8()?, title: r.str16()?, body: r.str16()? },
            k::PROGRESS => EventKind::Progress { state: ProgressState::from_wire(r.u8()?), percent: r.u8()? },
            k::PROGRAM_STATUS => EventKind::ProgramStatus(r.str16()?),
            k::CLIPBOARD_STORE => EventKind::ClipboardStore { selection: r.u8()?, data: r.bytes32()? },
            k::CLIPBOARD_LOAD => EventKind::ClipboardLoad { ticket: r.u32()?, selection: r.u8()? },
            k::WINDOW_OP => EventKind::WindowOp { op: r.u8()?, a: r.u32()?, b: r.u32()? },
            k::EXITED => EventKind::Exited(ExitInfo::read(&mut r)?),
            k::FULL_RESET => EventKind::FullReset,
            k::REPAINT_SEEDS => {
                EventKind::RepaintSeeds { block: r.u32()?, link_seq: r.u64()?, image_handle: r.u64()? }
            }
            other => EventKind::Unknown(other),
        };
        // Whatever follows is the TLV tail of a newer minor; there are no tags to read yet.
        Ok(Self { offset, kind })
    }
}

/// Encode a backlog: `count: u16`, then each event as `len: u32` and its encoding.
pub(crate) fn encode_backlog(events: &[Event<'_>], w: &mut Writer<'_>) -> Result<(), EncodeError> {
    if events.len() > MAX_BACKLOG {
        return Err(EncodeError::TooMany("Backlog"));
    }
    w.u16(events.len() as u16);
    for e in events {
        let at = w.len();
        w.u32(0);
        e.encode(w)?;
        w.patch_u32(at, (w.len() - at - 4) as u32);
    }
    Ok(())
}

pub(crate) fn decode_backlog<'a>(payload: &'a [u8]) -> Result<List<'a, Event<'a>>, WireError> {
    let mut r = Reader::new(payload, "Backlog");
    let count = usize::from(r.u16()?);
    if count > MAX_BACKLOG {
        return Err(WireError::TooMany("Backlog"));
    }
    let mut events = Vec::with_capacity(count.min(r.remaining() / 13));
    for _ in 0..count {
        let entry = r.bytes32()?;
        events.push(Event::decode(entry)?);
    }
    Ok(List::Owned(events))
}
