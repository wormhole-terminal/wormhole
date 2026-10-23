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

    /// Whether `screen` is held back by a synchronized update, so the UI keeps drawing the last
    /// frame and a redraw would show it unchanged.
    fn held_back(&self, _screen: &S) -> bool {
        false
    }
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
            let still_held = {
                let mut screen = self.screen.lock_as(Side::Reader);
                let was_held = self.feed.held_back(&screen);
                let parsing = perf::clock();
                self.feed.feed(&mut screen, slice);
                perf::parsed(slice.len(), parsing);
                was_held && self.feed.held_back(&screen)
            };
            // A slice inside a held update changes nothing the UI shows: it draws the frame from
            // before the update until the update ends (the slice that ends it wakes, as does the
            // timeout, see `expire_sync`). The slice that began the hold may carry output from
            // before it, so that one wakes too.
            if still_held {
                self.proxy.revise();
            } else {
                self.proxy.wakeup();
            }
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

#[cfg(test)]
mod held_tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{Feed, Flow, Sink};
    use crate::events::{EventProxy, PtyEvents};
    use crate::pty::FairLock;

    /// A synchronized update as bytes: `{` begins one (mode 2026 set), `}` ends it. The screen is
    /// just the flag the engine's `held_back` would be.
    struct Update {
        deadline: Option<Instant>,
    }

    impl Feed<bool> for Update {
        fn feed(&mut self, held: &mut bool, bytes: &[u8]) {
            for byte in bytes {
                match byte {
                    b'{' => *held = true,
                    b'}' => *held = false,
                    _ => {}
                }
            }
        }

        fn deadline(&self) -> Option<Instant> {
            self.deadline
        }

        fn expire(&mut self, held: &mut bool) {
            *held = false;
            self.deadline = None;
        }

        fn held_back(&self, held: &bool) -> bool {
            *held
        }
    }

    fn sink() -> (Sink<bool, Update>, PtyEvents, EventProxy) {
        let (proxy, events) = EventProxy::new();
        let sink = Sink::new(Arc::new(FairLock::new(false)), Update { deadline: None }, proxy.clone());
        (sink, events, proxy)
    }

    /// Parse `bytes` and say whether the UI was woken, taking the wakeup so the next one counts.
    fn woke(sink: &mut Sink<bool, Update>, events: &PtyEvents, bytes: &[u8]) -> bool {
        sink.parse(bytes, || Flow::Continue);
        events.try_recv().is_some()
    }

    #[test]
    fn output_outside_an_update_wakes_the_ui_every_time() {
        let (mut sink, events, _proxy) = sink();
        assert!(woke(&mut sink, &events, b"one"));
        assert!(woke(&mut sink, &events, b"two"));
    }

    #[test]
    fn a_held_update_wakes_the_ui_when_it_begins_and_when_it_ends_and_between_never() {
        let (mut sink, events, proxy) = sink();
        // The slice that begins the hold may carry output from before it.
        assert!(woke(&mut sink, &events, b"before{"));
        let revision = proxy.shared().revision();
        for part in [&b"a"[..], b"b", b"c"] {
            assert!(!woke(&mut sink, &events, part), "a slice inside the update wakes nobody");
        }
        assert_eq!(proxy.shared().revision(), revision + 3, "the revision still counts the output");
        assert!(woke(&mut sink, &events, b"d}"), "the slice that ends it");
        assert!(woke(&mut sink, &events, b"after"));
    }

    #[test]
    fn the_end_and_the_next_begin_in_one_slice_stay_quiet_like_the_frame_they_hold() {
        // The screen is held before and after, and the UI keeps the frame from before either
        // way: a repaint would show it again.
        let (mut sink, events, _proxy) = sink();
        assert!(woke(&mut sink, &events, b"{"));
        assert!(!woke(&mut sink, &events, b"}{"));
        assert!(woke(&mut sink, &events, b"}"));
    }

    #[test]
    fn repeated_begins_and_ends_wake_once_each_way() {
        let (mut sink, events, _proxy) = sink();
        assert!(woke(&mut sink, &events, b"{"));
        assert!(!woke(&mut sink, &events, b"{{"), "a begin inside an update");
        assert!(woke(&mut sink, &events, b"}"));
        assert!(woke(&mut sink, &events, b"}"), "an end without a begin is plain output");
        assert!(woke(&mut sink, &events, b"{x}"), "begun and ended in one slice");
    }

    #[test]
    fn one_big_read_in_slices_wakes_for_the_slices_that_begin_and_end_the_update_only() {
        let (mut sink, events, _proxy) = sink();
        let mut bytes = vec![b'{'];
        bytes.resize(3 * super::SLICE + 1, b'x');
        bytes.push(b'}');
        let mut woken = 0;
        sink.parse(&bytes, || {
            woken += usize::from(events.try_recv().is_some());
            Flow::Continue
        });
        assert_eq!(woken, 2, "four slices: the first begins the hold, the last ends it");
    }

    #[test]
    fn the_timeout_wakes_the_ui_once_and_shows_the_screen_again() {
        let (mut sink, events, _proxy) = sink();
        assert!(woke(&mut sink, &events, b"{"));
        assert!(!woke(&mut sink, &events, b"x"));

        sink.feed.deadline = Some(Instant::now() + Duration::from_secs(60));
        sink.expire_sync();
        assert_eq!(events.queued(), 0, "not yet");

        sink.feed.deadline = Some(Instant::now());
        sink.expire_sync();
        assert!(events.try_recv().is_some(), "the timeout wakes the UI");
        assert!(events.try_recv().is_none(), "once");
        assert!(!*sink.screen.lock());
        // Output after the timeout shows again, as before any update.
        assert!(woke(&mut sink, &events, b"y"));
    }

    #[test]
    fn a_change_made_under_the_lock_wakes_the_ui_even_while_held() {
        // A resize or a repaint changes what the replica shows.
        let (mut sink, events, _proxy) = sink();
        assert!(woke(&mut sink, &events, b"{"));
        sink.update(|_screen, _feed| {});
        assert!(events.try_recv().is_some());
    }
}
