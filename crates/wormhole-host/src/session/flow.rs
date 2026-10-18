//! The stream to one attachment, each turn of the loop.
//!
//! In order: the attach repaint (a `ResizeApplied` and the seeds first, then the repaint in parts
//! of at most a data frame), then the stream from the cursor: the program's bytes as `Out` frames
//! of up to 64 KiB a turn, never past the client's credit (`acked + window`), with the resizes and
//! events in between exactly where they happened, and `Counters` when they changed, at most 60 a
//! second. The PTY thread never waits for any of it: the bytes wait in the stream's ring, which
//! gives up on a client that falls too far behind.
//!
//! **Cut loose.** A client that could not be sent what the stream has for it (no credit, or its
//! socket not taking more) for `Config::stall_limit` is cut from the stream and told
//! `Detached{TooSlow}`; the connection stays, and a new `Attach` brings it back with a repaint. A
//! hung app never holds the program up by more than a quarter second, and the host's memory for it
//! is bounded by the window and the ring.

use std::time::{Duration, Instant};

use wormhole_term::hosted::Next;
use wormhole_wire::{self as wire, DetachReason, MAX_DATA, Message, RepaintKind, StreamMode};

use super::conn::{Conn, OUT_HIGH, PendingRepaint};
use super::{Deadline, Session};
use crate::program::{Program, Spawner};

/// How soon a `Sync` attach tries again when the parser was inside a sequence: soon at first,
/// then slowly, for a program that stopped in the middle of one.
const RETRY_SOON: Duration = Duration::from_millis(5);
const RETRY_LATER: Duration = Duration::from_millis(100);
const RETRY_SOON_FOR: Duration = Duration::from_millis(200);

impl<S: Spawner> Session<S> {
    /// One connection's turn; whether it has more to send right now.
    pub(super) fn pump_conn(&mut self, at: usize, now: Instant, next: &mut Deadline) -> bool {
        if let Some(waiting) = self.conns[at].attach.as_ref().and_then(|a| a.waiting) {
            if now < waiting.retry {
                next.at(Some(waiting.retry));
            } else {
                self.try_repaint(at, now);
            }
        }
        let controller = self.controller == Some(self.conns[at].id);
        let stall_limit = self.config.stall_limit;
        let counters_every = self.config.counters_every;
        let Some(program) = &self.program else { return false };
        let conn = &mut self.conns[at];
        if controller {
            let acked = program.input_acked();
            if acked > conn.input_acked {
                conn.input_acked = acked;
                conn.send(&Message::InputAck(acked));
            }
        }
        let Some(attach) = &mut conn.attach else { return false };
        if let Some(waiting) = attach.waiting {
            next.at(Some(waiting.retry));
            return false;
        }

        // The repaint first: nothing of the stream passes it.
        if attach.repaint.is_some() {
            send_repaint(conn);
            let Some(attach) = &mut conn.attach else { return false };
            if attach.repaint.is_some() {
                let blocked = conn.pending() >= OUT_HIGH;
                return check_stall(conn, blocked, now, next, stall_limit) || !blocked;
            }
            if std::mem::take(&mut attach.exited_after_repaint)
                && let Some(exit) = program.exit()
            {
                let offset = attach.sent;
                conn.send(&Message::Event(wire::Event { offset, kind: wire::EventKind::Exited(exit) }));
            }
        }

        let scratch = &mut self.scratch;
        let mut budget = MAX_DATA;
        let mut more = false;
        loop {
            // A full buffer takes no more bytes, but the cursor is still asked: for the resizes and
            // events (small), and so that one the ring gave up on is let go at once.
            let room = conn.pending() < OUT_HIGH;
            let Some(attach) = &mut conn.attach else { break };
            let max = if !room || attach.mode == StreamMode::None {
                0
            } else {
                budget.min(attach.credit() as usize)
            };
            let Some(cursor) = &mut attach.cursor else { break };
            scratch.clear();
            match cursor.next(max, scratch) {
                Next::Idle => break,
                Next::Overrun => {
                    cut_loose(conn, "fell too far behind the stream");
                    return false;
                }
                Next::Bytes { offset } => {
                    attach.sent = offset + scratch.len() as u64;
                    budget -= scratch.len();
                    conn.send(&Message::Out(wire::Out { offset, bytes: scratch }));
                    if budget == 0 {
                        more = true;
                        break;
                    }
                }
                Next::Resize(mark) => conn.send(&Message::ResizeApplied(wire::ResizeApplied {
                    offset: mark.offset,
                    epoch: mark.epoch,
                    req: mark.req,
                    size: mark.size,
                })),
                Next::Event { offset, event } => {
                    // The clipboard is the user's: only the controller hears what a program puts
                    // there or asks for.
                    if controller || !event.controller_only() {
                        conn.send(&Message::Event(event.to_wire(offset)));
                    }
                }
            }
        }
        send_counters(conn, program.counters(), now, next, counters_every);
        let Some(attach) = &mut conn.attach else { return false };
        // Stalled means no progress, not behind: a client that keeps acknowledging under a flood
        // has a full window after every turn and must not be cut loose for it.
        if budget < MAX_DATA {
            attach.stalled_since = None;
        }
        let unsent =
            attach.mode != StreamMode::None && attach.cursor.as_ref().is_some_and(|c| c.unsent() > 0);
        let blocked = unsent && (attach.credit() == 0 || conn.pending() >= OUT_HIGH);
        more |= unsent && !blocked;
        check_stall(conn, blocked, now, next, stall_limit) || more
    }

    /// Make a `Sync` attach's repaint, if the parser is between sequences now; otherwise try again
    /// a little later.
    pub(super) fn try_repaint(&mut self, at: usize, now: Instant) {
        let Some(program) = &self.program else { return };
        let conn = &mut self.conns[at];
        let Some(attach) = &mut conn.attach else { return };
        let Some(mut waiting) = attach.waiting else { return };
        let mut bytes = Vec::new();
        let made = match program.repaint(waiting.history, &mut bytes, self.wake.clone()) {
            Ok(made) => made,
            Err(_) => {
                let soon = now.duration_since(waiting.since) < RETRY_SOON_FOR;
                waiting.retry = now + if soon { RETRY_SOON } else { RETRY_LATER };
                attach.waiting = Some(waiting);
                return;
            }
        };
        tracing::info!(
            conn = conn.id.0,
            bytes = bytes.len(),
            history = made.history_lines,
            gaps = made.gaps.bits(),
            "repaint made"
        );
        attach.waiting = None;
        attach.sent = made.offset;
        attach.acked = made.offset;
        attach.cursor = Some(made.cursor);
        attach.repaint = Some(PendingRepaint { bytes, pos: 0, epoch: made.epoch, offset: made.offset });
        // The replica takes the size first, then the counters VT cannot set, then the repaint.
        conn.send(&Message::ResizeApplied(wire::ResizeApplied {
            offset: made.offset,
            epoch: made.epoch,
            req: 0,
            size: made.size,
        }));
        let seeds = wire::EventKind::RepaintSeeds {
            block: made.block,
            link_seq: made.link_seq,
            image_handle: made.image_handle,
        };
        conn.send(&Message::Event(wire::Event { offset: made.offset, kind: seeds }));
    }
}

/// Queue repaint parts while the connection's buffer has room.
fn send_repaint(conn: &mut Conn) {
    // Out of the attachment while its parts are queued (`send` needs the whole connection).
    let Some(mut repaint) = conn.attach.as_mut().and_then(|a| a.repaint.take()) else { return };
    while conn.pending() < OUT_HIGH && repaint.pos < repaint.bytes.len() {
        let end = (repaint.pos + MAX_DATA).min(repaint.bytes.len());
        conn.send(&Message::Repaint(wire::Repaint {
            epoch: repaint.epoch,
            offset: repaint.offset,
            kind: RepaintKind::Full,
            history_gap: false,
            first: repaint.pos == 0,
            last: end == repaint.bytes.len(),
            bytes: &repaint.bytes[repaint.pos..end],
        }));
        repaint.pos = end;
    }
    if repaint.pos < repaint.bytes.len()
        && let Some(attach) = &mut conn.attach
    {
        attach.repaint = Some(repaint);
    }
}

/// Note whether the stream is blocked for this connection, and cut it loose when it has been for
/// `limit`. Returns false (a connection cut loose has nothing more to send now).
fn check_stall(conn: &mut Conn, blocked: bool, now: Instant, next: &mut Deadline, limit: Duration) -> bool {
    let Some(attach) = &mut conn.attach else { return false };
    if !blocked {
        attach.stalled_since = None;
        return false;
    }
    let since = *attach.stalled_since.get_or_insert(now);
    if now.duration_since(since) >= limit {
        cut_loose(conn, "stalled with its window full");
    } else {
        next.at(Some(since + limit));
    }
    false
}

/// Stop streaming to the connection; it comes back with a new `Attach`.
fn cut_loose(conn: &mut Conn, why: &'static str) {
    let Some(attach) = &mut conn.attach else { return };
    tracing::info!(conn = conn.id.0, sent = attach.sent, acked = attach.acked, "cut loose: {why}");
    attach.cursor = None;
    attach.repaint = None;
    attach.waiting = None;
    attach.stalled_since = None;
    conn.send(&Message::Detached(DetachReason::TooSlow));
}

/// Send the counters when they changed, at most every `every`.
fn send_counters(
    conn: &mut Conn,
    counters: wire::Counters,
    now: Instant,
    next: &mut Deadline,
    every: Duration,
) {
    let Some(attach) = &mut conn.attach else { return };
    if attach.cursor.is_none() || attach.counters == Some(counters) {
        return;
    }
    if let Some(at) = attach.counters_at
        && now < at + every
    {
        next.at(Some(at + every));
        return;
    }
    attach.counters = Some(counters);
    attach.counters_at = Some(now);
    conn.send(&Message::Counters(counters));
}
