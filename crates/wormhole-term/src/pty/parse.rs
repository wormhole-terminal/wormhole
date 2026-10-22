//! The output on its way into the screen: the parser, called in slices under the screen's lock.
//!
//! The parser and the screen behind it are one call, [`Feed::feed`], so the reader thread does
//! not know the engine it drives (the PTY layer's tests drive it with a stub).

use std::sync::Arc;
use std::time::Instant;

use super::lock::FairLock;
use crate::events::EventProxy;
use crate::perf::{self, Side};

/// How many bytes the parser gets per turn at the screen's lock, which is the longest the UI's
/// snapshot waits behind it. `measure::slice_times` parses 32 MiB of log lines, truecolor cells,
/// cursor moves and SGR churn in slices of each size (M5 Pro): at 16 KiB the slowest corpus
/// (SGR churn, 156 MB/s) holds the lock for 100 µs at the median, 175 µs at the 99th percentile
/// and 390 µs at worst, and every corpus parses within 3% of its speed with 256 KiB slices, whose
/// 99th percentile is 1.6 to 2.1 ms. Smaller slices cost throughput: at 1 KiB log lines parse
/// 35% slower, the price of a call into the parser.
const SLICE: usize = 16 * 1024;

/// A parser and the screen it fills, as the reader thread drives them.
pub(crate) trait Feed<S>: Send + 'static {
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
pub(crate) enum Flow {
    /// Parse the next slice.
    Continue,
    /// The terminal was closed: drop the rest.
    Stop,
}

/// The screen, the parser that fills it, and the UI's wakeup.
pub(crate) struct Sink<S, F> {
    screen: Arc<FairLock<S>>,
    feed: F,
    proxy: EventProxy,
}

impl<S, F: Feed<S>> Sink<S, F> {
    /// A sink that parses with `feed` into `screen` and tells `proxy` when it did.
    pub(crate) fn new(screen: Arc<FairLock<S>>, feed: F, proxy: EventProxy) -> Self {
        Self { screen, feed, proxy }
    }

    /// Hand `bytes` to the parser in slices, the screen's lock taken for one slice at a time.
    /// `between` is called after each slice (the reader attends to input there) and says whether
    /// to go on.
    pub(crate) fn parse(&mut self, mut bytes: &[u8], mut between: impl FnMut() -> Flow) {
        while !bytes.is_empty() {
            let (slice, rest) = bytes.split_at(bytes.len().min(SLICE));
            bytes = rest;
            {
                let mut screen = self.screen.lock_as(Side::Reader);
                let parsing = perf::clock();
                self.feed.feed(&mut screen, slice);
                perf::parsed(slice.len(), parsing);
            }
            self.proxy.wakeup();
            if between() == Flow::Stop {
                return;
            }
        }
    }

    /// Tell the UI the program exited (with `code`, if it exited with one, or the `signal` that
    /// ended it) and its output is parsed.
    pub(super) fn report_exit(&self, code: Option<i32>, signal: Option<i32>) {
        self.proxy.exited(code, signal);
    }

    /// When a synchronized update has to be given up on (see [`Feed::deadline`]).
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.feed.deadline()
    }

    /// Give up on a synchronized update that has been going on for too long, if it has.
    pub(crate) fn expire_sync(&mut self) {
        if self.feed.deadline().is_some_and(|at| at <= Instant::now()) {
            {
                let mut screen = self.screen.lock_as(Side::Reader);
                self.feed.expire(&mut screen);
            }
            self.proxy.wakeup();
        }
    }

    /// Change the screen and the parser together, under the screen's lock, and tell the UI (a
    /// hosted terminal's replica is reset for a repaint and resized when the host says so).
    pub(crate) fn update(&mut self, change: impl FnOnce(&mut S, &mut F)) {
        {
            let mut screen = self.screen.lock_as(Side::Reader);
            change(&mut screen, &mut self.feed);
        }
        self.proxy.wakeup();
    }
}

#[cfg(all(test, feature = "perf-counters"))]
mod tests {
    use std::sync::Arc;

    use super::{Feed, Flow, Sink};
    use crate::events::EventProxy;
    use crate::perf::{self, COUNTING_TESTS};
    use crate::pty::FairLock;

    /// Counts the bytes it was fed.
    struct Tally;

    impl Feed<usize> for Tally {
        fn feed(&mut self, screen: &mut usize, bytes: &[u8]) {
            *screen += bytes.len();
        }
    }

    /// Other tests' terminals may be reading meanwhile and add to the counters: lower bounds.
    #[test]
    fn a_profiled_sink_counts_its_bytes_slices_and_reader_locks() {
        let _turn = COUNTING_TESTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let screen = Arc::new(FairLock::new(0usize));
        let (proxy, _events) = EventProxy::new();
        let mut sink = Sink::new(screen.clone(), Tally, proxy);
        let bytes = vec![b'x'; 40 * 1024];
        perf::start();
        sink.parse(&bytes, || Flow::Continue);
        let counted = perf::stop();
        assert_eq!(*screen.lock(), bytes.len());
        assert!(counted.bytes >= bytes.len() as u64, "{counted:?}");
        assert!(counted.slices >= 3 && counted.reader.locks >= 3, "three 16 KiB slices: {counted:?}");
    }
}
