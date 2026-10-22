//! What reading output costs, counted for the app's `perf-profile` (devctl): the bytes the reader
//! threads parsed and how long that took, and how long the screen's lock was waited for and held,
//! by the reader threads and by everyone else (the UI).
//!
//! The counters exist only with the `perf-counters` feature (the app's devctl builds turn it on);
//! without it every function here is empty and the lock's guard carries nothing. With it they
//! count only between [`start`] and [`stop`]: until then each call site pays one relaxed load of
//! a flag, per 16 KiB slice and per lock, against microseconds of parsing. The counters are the
//! whole process's: every terminal's readers add to the same ones.

use std::time::Instant;

/// Who takes the screen's lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// A thread that parses output into the screen (a local terminal's PTY reader, a hosted
    /// terminal's client thread).
    Reader,
    /// Everyone else: the UI's snapshot, input, selection, find.
    Ui,
}

/// One side's use of the lock while counting, in nanoseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LockUse {
    /// Times the lock was taken.
    pub locks: u64,
    pub wait_ns: u64,
    pub wait_max_ns: u64,
    pub hold_ns: u64,
    pub hold_max_ns: u64,
}

/// Everything counted since [`start`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Bytes of output handed to the parser.
    pub bytes: u64,
    /// The slices they were parsed in (16 KiB at most, one lock each).
    pub slices: u64,
    /// Time inside the parser (under the lock).
    pub parse_ns: u64,
    pub reader: LockUse,
    pub ui: LockUse,
}

/// Whether the counters exist in this build at all.
pub const AVAILABLE: bool = cfg!(feature = "perf-counters");

#[cfg(feature = "perf-counters")]
mod counting {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
    use std::time::Instant;

    use super::{Counters, LockUse, Side};

    pub(super) static ENABLED: AtomicBool = AtomicBool::new(false);

    pub(super) struct SideCounters {
        locks: AtomicU64,
        wait_ns: AtomicU64,
        wait_max_ns: AtomicU64,
        hold_ns: AtomicU64,
        hold_max_ns: AtomicU64,
    }

    impl SideCounters {
        const fn new() -> Self {
            Self {
                locks: AtomicU64::new(0),
                wait_ns: AtomicU64::new(0),
                wait_max_ns: AtomicU64::new(0),
                hold_ns: AtomicU64::new(0),
                hold_max_ns: AtomicU64::new(0),
            }
        }

        pub(super) fn waited(&self, ns: u64) {
            self.locks.fetch_add(1, Relaxed);
            self.wait_ns.fetch_add(ns, Relaxed);
            self.wait_max_ns.fetch_max(ns, Relaxed);
        }

        pub(super) fn held(&self, ns: u64) {
            self.hold_ns.fetch_add(ns, Relaxed);
            self.hold_max_ns.fetch_max(ns, Relaxed);
        }

        fn read(&self) -> LockUse {
            LockUse {
                locks: self.locks.load(Relaxed),
                wait_ns: self.wait_ns.load(Relaxed),
                wait_max_ns: self.wait_max_ns.load(Relaxed),
                hold_ns: self.hold_ns.load(Relaxed),
                hold_max_ns: self.hold_max_ns.load(Relaxed),
            }
        }

        fn reset(&self) {
            for counter in [&self.locks, &self.wait_ns, &self.wait_max_ns, &self.hold_ns, &self.hold_max_ns] {
                counter.store(0, Relaxed);
            }
        }
    }

    pub(super) static READER: SideCounters = SideCounters::new();
    pub(super) static UI: SideCounters = SideCounters::new();
    pub(super) static BYTES: AtomicU64 = AtomicU64::new(0);
    pub(super) static SLICES: AtomicU64 = AtomicU64::new(0);
    pub(super) static PARSE_NS: AtomicU64 = AtomicU64::new(0);

    pub(super) fn side(side: Side) -> &'static SideCounters {
        match side {
            Side::Reader => &READER,
            Side::Ui => &UI,
        }
    }

    pub(super) fn read() -> Counters {
        Counters {
            bytes: BYTES.load(Relaxed),
            slices: SLICES.load(Relaxed),
            parse_ns: PARSE_NS.load(Relaxed),
            reader: READER.read(),
            ui: UI.read(),
        }
    }

    pub(super) fn reset() {
        for counter in [&BYTES, &SLICES, &PARSE_NS] {
            counter.store(0, Relaxed);
        }
        READER.reset();
        UI.reset();
    }

    pub(super) fn nanos_since(at: Instant) -> u64 {
        u64::try_from(at.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// Reset the counters and start counting.
pub fn start() {
    #[cfg(feature = "perf-counters")]
    {
        counting::reset();
        counting::ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Stop counting, and what was counted since [`start`].
pub fn stop() -> Counters {
    #[cfg(feature = "perf-counters")]
    {
        counting::ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
        counting::read()
    }
    #[cfg(not(feature = "perf-counters"))]
    Counters::default()
}

/// What was counted so far, counting on (for the difference over one frame).
pub fn read() -> Counters {
    #[cfg(feature = "perf-counters")]
    {
        counting::read()
    }
    #[cfg(not(feature = "perf-counters"))]
    Counters::default()
}

/// The time now while counting, `None` otherwise (and always without the feature, which the
/// compiler then folds away at every call site).
#[inline]
pub(crate) fn clock() -> Option<Instant> {
    #[cfg(feature = "perf-counters")]
    if counting::ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Some(Instant::now());
    }
    None
}

/// A slice of `bytes` was parsed, starting at `since` (from [`clock`]).
#[inline]
pub(crate) fn parsed(bytes: usize, since: Option<Instant>) {
    #[cfg(feature = "perf-counters")]
    if let Some(since) = since {
        use std::sync::atomic::Ordering::Relaxed;
        counting::BYTES.fetch_add(bytes as u64, Relaxed);
        counting::SLICES.fetch_add(1, Relaxed);
        counting::PARSE_NS.fetch_add(counting::nanos_since(since), Relaxed);
    }
    #[cfg(not(feature = "perf-counters"))]
    let _ = (bytes, since);
}

/// Kept in the lock's guard: counts how long the lock was held when it is let go. Empty without
/// the feature.
pub(crate) struct Held {
    #[cfg(feature = "perf-counters")]
    since: Option<(Side, Instant)>,
}

impl Held {
    /// `side` got the lock now, having asked at `asked` (from [`clock`]; `None`: not counting).
    #[inline]
    pub(crate) fn new(side: Side, asked: Option<Instant>) -> Self {
        #[cfg(feature = "perf-counters")]
        {
            let since = asked.map(|asked| {
                let now = Instant::now();
                counting::side(side).waited(u64::try_from((now - asked).as_nanos()).unwrap_or(u64::MAX));
                (side, now)
            });
            Self { since }
        }
        #[cfg(not(feature = "perf-counters"))]
        {
            let _ = (side, asked);
            Self {}
        }
    }
}

#[cfg(feature = "perf-counters")]
impl Drop for Held {
    fn drop(&mut self) {
        if let Some((side, since)) = self.since {
            counting::side(side).held(counting::nanos_since(since));
        }
    }
}
