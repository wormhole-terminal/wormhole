//! The output on its way into the screen: the parser, called in slices under the screen's lock.
//!
//! The parser and the screen behind it are one call, [`Feed::feed`], so the reader thread does
//! not know the engine it drives (the PTY layer's tests drive it with a stub).

use std::sync::Arc;
use std::time::Instant;

use super::lock::FairLock;
use crate::terminal::events::EventProxy;

/// How many bytes the parser gets per turn at the screen's lock, which is the longest the UI's
/// snapshot waits behind it. `measure::slice_times` parses 32 MiB of log lines, truecolor cells,
/// cursor moves and SGR churn in slices of each size (M5 Pro): at 16 KiB the slowest corpus
/// (SGR churn, 156 MB/s) holds the lock for 100 µs at the median, 175 µs at the 99th percentile
/// and 390 µs at worst, and every corpus parses within 3% of its speed with 256 KiB slices, whose
/// 99th percentile is 1.6 to 2.1 ms. Smaller slices cost throughput: at 1 KiB log lines parse
/// 35% slower, the price of a call into the parser.
const SLICE: usize = 16 * 1024;

/// A parser and the screen it fills, as the reader thread drives them.
pub(in crate::terminal) trait Feed<S>: Send + 'static {
    /// Parse `bytes` of the program's output into `screen`. A sequence may end in the next call.
    fn feed(&mut self, screen: &mut S, bytes: &[u8]);

    /// When a synchronized update (mode 2026) that was begun and not ended must be given up on:
    /// the reader wakes then and calls [`expire`](Self::expire). `None` when there is none.
    fn deadline(&self) -> Option<Instant> {
        None
    }

    /// The deadline passed: show what the update held back.
    fn expire(&mut self, _screen: &mut S) {}
}

/// Whether the reader goes on after it attended to the rest of the app between two slices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flow {
    /// Parse the next slice.
    Continue,
    /// The terminal was closed: drop the rest.
    Stop,
}

/// The screen, the parser that fills it, and the UI's wakeup.
pub(super) struct Sink<S, F> {
    screen: Arc<FairLock<S>>,
    feed: F,
    proxy: EventProxy,
}

impl<S, F: Feed<S>> Sink<S, F> {
    /// A sink that parses with `feed` into `screen` and tells `proxy` when it did.
    pub(super) fn new(screen: Arc<FairLock<S>>, feed: F, proxy: EventProxy) -> Self {
        Self { screen, feed, proxy }
    }

    /// Hand `bytes` to the parser in slices, the screen's lock taken for one slice at a time.
    /// `between` is called after each slice (the reader attends to input there) and says whether
    /// to go on.
    pub(super) fn parse(&mut self, mut bytes: &[u8], mut between: impl FnMut() -> Flow) {
        while !bytes.is_empty() {
            let (slice, rest) = bytes.split_at(bytes.len().min(SLICE));
            bytes = rest;
            {
                let mut screen = self.screen.lock();
                self.feed.feed(&mut screen, slice);
            }
            self.proxy.wakeup();
            if between() == Flow::Stop {
                return;
            }
        }
    }

    /// Tell the UI the program exited (with `code`, if it exited with one) and its output is parsed.
    pub(super) fn report_exit(&self, code: Option<i32>) {
        self.proxy.exited(code);
    }

    /// When a synchronized update has to be given up on (see [`Feed::deadline`]).
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.feed.deadline()
    }

    /// Give up on a synchronized update that has been going on for too long, if it has.
    pub(super) fn expire_sync(&mut self) {
        if self.feed.deadline().is_some_and(|at| at <= Instant::now()) {
            {
                let mut screen = self.screen.lock();
                self.feed.expire(&mut screen);
            }
            self.proxy.wakeup();
        }
    }
}
