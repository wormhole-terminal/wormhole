//! What reading output costs, for the Swift app's `perf-profile` (the GPUI app's `perf/report.rs`
//! `Reader` and `ui_lock`): the terminal layer's counters (`wormhole_term::perf`), every terminal of the
//! process together, between a start and a stop.
//!
//! The counters exist only with this crate's `perf-counters` feature, which turns the terminal layer's on;
//! without it both calls are empty and `available` says so, and the shipped library is what it was.

use wormhole_term::perf::{self, LockUse};

/// One side's use of the screens' lock while counting, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct PerfLockUse {
    /// Times the lock was taken.
    pub locks: u64,
    pub wait_ns: u64,
    pub wait_max_ns: u64,
    pub hold_ns: u64,
    pub hold_max_ns: u64,
}

impl From<LockUse> for PerfLockUse {
    fn from(used: LockUse) -> Self {
        Self {
            locks: used.locks,
            wait_ns: used.wait_ns,
            wait_max_ns: used.wait_max_ns,
            hold_ns: used.hold_ns,
            hold_max_ns: used.hold_max_ns,
        }
    }
}

/// Everything counted between [`perf_counters_start`] and [`perf_counters_stop`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct PerfCounters {
    /// Whether this library counts at all (its `perf-counters` feature); false: every number is 0.
    pub available: bool,
    /// Bytes of output handed to the parser, and the slices they were parsed in.
    pub bytes: u64,
    pub slices: u64,
    /// Time inside the parser, under the lock.
    pub parse_ns: u64,
    /// The lock as the threads that parse output took it, and as every other thread did (the frames'
    /// snapshot, input, find).
    pub reader: PerfLockUse,
    pub ui: PerfLockUse,
}

/// Resets the terminal layer's counters and starts counting.
#[uniffi::export]
pub fn perf_counters_start() {
    perf::start();
}

/// Stops counting, and what was counted since the start.
#[uniffi::export]
pub fn perf_counters_stop() -> PerfCounters {
    let counted = perf::stop();
    PerfCounters {
        available: cfg!(feature = "perf-counters"),
        bytes: counted.bytes,
        slices: counted.slices,
        parse_ns: counted.parse_ns,
        reader: counted.reader.into(),
        ui: counted.ui.into(),
    }
}
