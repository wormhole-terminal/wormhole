//! A stand-in for a terminal, for the tests of the check and of the runner: what it answers is a
//! function of the bytes it is written, and it can be made to lose some of them.

use std::io;
use std::ops::Range;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use super::plan::QUERY;
use crate::bench::workloads::Payload;
use crate::term::Input;

/// What a terminal that is all right answers to DSR 5.
pub(crate) const REPLY: &[u8] = b"\x1b[0n";
pub(crate) const CPR_QUERY: &[u8] = b"\x1b[6n";
pub(crate) const DA1_QUERY: &[u8] = b"\x1b[c";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Answers everything, at once.
    Faithful,
    /// Never answers DSR 5 (answers the rest).
    NoDsr5,
    /// Answers only the first `n` DSR 5 queries it is ever sent.
    Rationed(u64),
    /// Answers every DSR 5 twice.
    Twice,
    /// Holds its DSR 5 answers back until after the next cursor report.
    Late,
    /// Throws away all input except the queries, and answers those: what a terminal written to pass
    /// would do. The check cannot tell it from a faithful one.
    PicksQueriesOut,
    /// Answers like a faithful terminal, each answer `ms` milliseconds late (after the probe).
    Delayed { ms: u64 },
    /// Parses only the last 4 KiB of each of the big calls (over 32 KiB) numbered `from..to` since it
    /// was made, and everything else: a terminal that has a bad moment.
    BadMoment { from: usize, to: usize },
    /// Parses only the last `keep` bytes of every call to `write` or `writev`: it reads in the
    /// writer's units and keeps the end of each.
    TailOfEachCall { keep: usize },
    /// Reads in ticks of `tick` bytes of the stream and parses only the last `keep` of each: a ring
    /// overwritten between ticks, whose ticks have nothing to do with the writer's writes.
    TailOfEachTick { tick: usize, keep: usize },
}

/// A terminal that is a function of the bytes written to it. Bytes at offsets in `lost` never reach
/// its parser, as if a ring buffer had been overwritten.
pub(crate) struct Fake {
    pub(crate) tx: Sender<Vec<u8>>,
    pub(crate) mode: Mode,
    pub(crate) lost: Range<usize>,
    pub(crate) offset: usize,
    pub(crate) window: Vec<u8>,
    pub(crate) held: Vec<u8>,
    /// What its parser got.
    pub(crate) parsed: Vec<u8>,
    /// How many bytes it was sent and how many reached its parser.
    pub(crate) sent: usize,
    pub(crate) kept: usize,
    pub(crate) dsr5_seen: u64,
    /// Calls to the kernel: `write` and `writev` count alike.
    pub(crate) calls: usize,
    /// The probe is over (its DA1 was answered).
    armed: bool,
    /// Big calls so far.
    big: usize,
    /// The current call is one of a bad moment.
    bad: bool,
}

impl Fake {
    pub(crate) fn new(mode: Mode, lost: Range<usize>) -> (Input, Fake) {
        let (tx, rx) = mpsc::channel();
        let fake = Fake {
            tx,
            mode,
            lost,
            offset: 0,
            window: Vec::new(),
            held: Vec::new(),
            parsed: Vec::new(),
            sent: 0,
            kept: 0,
            dsr5_seen: 0,
            calls: 0,
            armed: false,
            big: 0,
            bad: false,
        };
        (Input::with_receiver(rx), fake)
    }

    fn send(&self, bytes: &[u8]) {
        let _ = self.tx.send(bytes.to_vec());
    }

    fn answer_dsr5(&mut self) {
        self.dsr5_seen += 1;
        if let Mode::Delayed { ms } = self.mode
            && self.armed
        {
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(ms));
                let _ = tx.send(REPLY.to_vec());
            });
            return;
        }
        let copies = match self.mode {
            Mode::NoDsr5 => 0,
            Mode::Rationed(n) if self.dsr5_seen > n => 0,
            Mode::Twice => 2,
            _ => 1,
        };
        for _ in 0..copies {
            if self.mode == Mode::Late {
                self.held.extend_from_slice(REPLY);
            } else {
                self.send(REPLY);
            }
        }
    }

    /// Sends what was held back.
    fn release(&mut self) {
        let held = std::mem::take(&mut self.held);
        if !held.is_empty() {
            self.send(&held);
        }
    }

    /// Whether the byte at `at` in this call (of `len` bytes) reaches the parser.
    fn reaches_parser(&self, at: usize, len: usize) -> bool {
        let offset = self.offset + at;
        if self.lost.contains(&offset) {
            return false;
        }
        match self.mode {
            Mode::TailOfEachCall { keep } => at + keep >= len,
            Mode::BadMoment { .. } => !self.bad || at + 4096 >= len,
            Mode::TailOfEachTick { tick, keep } => offset % tick + keep >= tick,
            _ => true,
        }
    }

    /// One call to the kernel with these bytes.
    fn take(&mut self, buf: &[u8]) {
        self.calls += 1;
        // The question that follows the payload is the newest thing it was sent: a terminal that
        // keeps the end of what it reads keeps that.
        let newest = buf == CPR_QUERY;
        if let Mode::BadMoment { from, to } = self.mode {
            if buf.len() > 32 * 1024 {
                self.bad = (from..to).contains(&self.big);
                self.big += 1;
            } else {
                self.bad = false;
            }
        }
        self.sent += buf.len();
        for (at, &b) in buf.iter().enumerate() {
            if newest || self.reaches_parser(at, buf.len()) {
                self.window.push(b);
                self.kept += 1;
            }
        }
        self.offset += buf.len();
        let mut i = 0;
        while i < self.window.len() {
            let rest = &self.window[i..];
            if rest.starts_with(QUERY) {
                self.answer_dsr5();
                i += QUERY.len();
            } else if rest.starts_with(CPR_QUERY) {
                self.send(b"\x1b[24;80R");
                self.release();
                i += CPR_QUERY.len();
            } else if rest.starts_with(DA1_QUERY) {
                self.send(b"\x1b[?62c");
                self.armed = true;
                self.release();
                i += DA1_QUERY.len();
            } else {
                // A byte nothing is waiting on, unless it begins a query that isn't all here yet.
                let pending =
                    [QUERY, CPR_QUERY, DA1_QUERY].iter().any(|q| q.starts_with(rest) && rest.len() < q.len());
                if pending {
                    break;
                }
                if self.mode != Mode::PicksQueriesOut {
                    self.parsed.push(self.window[i]);
                }
                i += 1;
            }
        }
        self.window.drain(..i);
    }
}

impl io::Write for Fake {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.take(buf);
        Ok(buf.len())
    }

    /// Like the kernel's `writev`: everything in one call, in order.
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        let all: Vec<u8> = bufs.iter().flat_map(|b| b.iter().copied()).collect();
        self.take(&all);
        Ok(all.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn text(len: usize) -> Payload {
    Payload {
        bytes: b"2026-10-28 INFO request id=42\r\n".iter().copied().cycle().take(len).collect(),
        ..Payload::default()
    }
}
