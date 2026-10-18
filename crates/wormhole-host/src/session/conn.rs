//! One connection's state: where it is in the handshake, what it attached to, how far its stream
//! has gone, and the bytes waiting to be written to it.

use std::time::Instant;

use wormhole_term::hosted::{Cursor, HostEvent};
use wormhole_wire::{self as wire, CHANNEL_LOCAL, Decoder, Message, Role, StreamMode};

/// Stream bytes are not taken out of the ring while this much is waiting to be written: the rest
/// waits in the ring (bounded) instead of in the connection's buffer.
pub(super) const OUT_HIGH: usize = 256 * 1024;
/// A connection with this much unwritten is not reading at all: it is closed.
pub(super) const OUT_MAX: usize = 4 * 1024 * 1024;

/// A connection, as the driver names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    /// Waiting for `Hello` until then.
    Hello {
        deadline: Instant,
    },
    Ready {
        role: Role,
    },
}

/// A repaint on its way out in parts of at most a data frame each.
pub(super) struct PendingRepaint {
    pub bytes: Vec<u8>,
    pub pos: usize,
    pub epoch: u32,
    pub offset: u64,
}

/// A `Sync` attach that could not make its repaint yet.
#[derive(Clone, Copy, Debug)]
pub(super) struct Waiting {
    pub history: usize,
    /// The first try, and when to try again.
    pub since: Instant,
    pub retry: Instant,
}

/// What a connection receives of the terminal after `Attach`.
pub(super) struct Attachment {
    pub mode: StreamMode,
    /// `None` once it was cut loose (it attaches again to come back).
    pub cursor: Option<Cursor>,
    /// The byte window: stream bytes up to `acked + window` may be sent.
    pub window: u64,
    pub acked: u64,
    /// The stream offset after the last byte sent.
    pub sent: u64,
    pub repaint: Option<PendingRepaint>,
    /// A `Sync` attach waiting for the parser to be between sequences.
    pub waiting: Option<Waiting>,
    /// Since when the stream had bytes for it that could not go out.
    pub stalled_since: Option<Instant>,
    /// What the repaint cannot say (the title, folder, reports), told once the repaint is out.
    pub replay: Vec<HostEvent>,
    /// Tell it the program exited once the repaint is out (it attached after the exit).
    pub exited_after_repaint: bool,
    pub counters: Option<wire::Counters>,
    pub counters_at: Option<Instant>,
}

impl Attachment {
    pub fn new(mode: StreamMode, window: u64) -> Self {
        Self {
            mode,
            cursor: None,
            window,
            acked: 0,
            sent: 0,
            repaint: None,
            waiting: None,
            stalled_since: None,
            replay: Vec::new(),
            exited_after_repaint: false,
            counters: None,
            counters_at: None,
        }
    }

    /// The stream bytes it may still be sent.
    pub fn credit(&self) -> u64 {
        if self.mode == StreamMode::None { 0 } else { (self.acked + self.window).saturating_sub(self.sent) }
    }
}

pub(super) struct Conn {
    pub id: ConnId,
    pub phase: Phase,
    pub dec: Decoder,
    out: Vec<u8>,
    /// Bytes of `out` written already.
    out_pos: usize,
    pub attach: Option<Attachment>,
    /// Close once `out` is written.
    pub closing: bool,
    /// Close now: the peer broke the protocol or stopped reading.
    pub dead: bool,
    pub input_acked: u64,
}

impl Conn {
    pub fn new(id: ConnId, hello_deadline: Instant) -> Self {
        let mut dec = Decoder::new();
        dec.limit_to_handshake();
        Self {
            id,
            phase: Phase::Hello { deadline: hello_deadline },
            dec,
            out: Vec::new(),
            out_pos: 0,
            attach: None,
            closing: false,
            dead: false,
            input_acked: 0,
        }
    }

    pub fn role(&self) -> Option<Role> {
        match self.phase {
            Phase::Ready { role } => Some(role),
            Phase::Hello { .. } => None,
        }
    }

    /// Queue `msg`. A message the codec refuses (an event too large for a frame) is dropped and
    /// logged: it is the program's, not the client's, and must not cost the connection.
    pub fn send(&mut self, msg: &Message<'_>) {
        if self.dead {
            return;
        }
        if let Err(err) = wire::encode(&mut self.out, CHANNEL_LOCAL, 0, msg) {
            tracing::debug!(conn = self.id.0, ty = msg.ty(), "a frame was not sent: {err}");
        }
        if self.pending() > OUT_MAX {
            tracing::info!(
                conn = self.id.0,
                pending = self.pending(),
                "connection is not reading, closing it"
            );
            self.dead = true;
        }
    }

    /// Queue an error about a message of type `ref_type`.
    pub fn refuse(&mut self, ref_type: u8, code: u16, text: &'static str) {
        self.send(&Message::Error(wire::ErrorMsg { ref_type, code, text }));
    }

    /// Bytes queued and not yet written.
    pub fn pending(&self) -> usize {
        self.out.len() - self.out_pos
    }

    pub fn output(&self) -> &[u8] {
        &self.out[self.out_pos..]
    }

    /// `n` bytes of [`output`](Self::output) were written.
    pub fn wrote(&mut self, n: usize) {
        self.out_pos = (self.out_pos + n).min(self.out.len());
        if self.out_pos == self.out.len() {
            self.out.clear();
            self.out_pos = 0;
        } else if self.out_pos > OUT_HIGH {
            self.out.drain(..self.out_pos);
            self.out_pos = 0;
        }
    }
}
