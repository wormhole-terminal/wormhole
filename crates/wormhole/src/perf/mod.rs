//! What a frame costs, recorded for devctl's `perf-profile` (`repo-docs/docs/performance.md`,
//! "Profiling"): the terminal's renderer marks its parts with [`span`] and [`count`], the command
//! draws the window and collects them per frame ([`begin_frame`], [`take_frame`]), and `report`
//! turns the frames into the numbers it writes.
//!
//! Without the devctl feature a span is an empty value and every function here is empty, so the
//! shipped renderer compiles to what it was. With it, a span costs one relaxed load of a flag
//! until `perf-profile` records.

#[cfg(any(test, feature = "devctl"))]
pub(crate) mod allocations;
#[cfg(any(test, feature = "devctl"))]
pub(crate) mod report;

/// A part of the terminal's frame, as `report` lists them. `Prepare` holds `Snapshot`, `Rows` and
/// `Images`; `Rows` holds `Shape` and `Blur`; `Paint` is the second half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// The canvas's prepaint: lay out, snapshot, build rows, the cursor and the pictures.
    Prepare,
    /// Bringing the snapshot up to date under the terminal's lock (the wait for it included).
    Snapshot,
    /// Syncing the row cache: rebuilding the rows that changed.
    Rows,
    /// Shaping new words with the text system (inside `Rows`).
    Shape,
    /// Shaping the blurred lines under the title bar, whole lines at a time (inside `Rows`).
    Blur,
    /// The inline pictures' tiles and placements.
    Images,
    /// Painting the layers.
    Paint,
}

#[cfg(any(test, feature = "devctl"))]
impl Part {
    pub(crate) const ALL: [Part; 7] =
        [Part::Prepare, Part::Snapshot, Part::Rows, Part::Shape, Part::Blur, Part::Images, Part::Paint];

    /// The part's name in the report.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Part::Prepare => "prepare",
            Part::Snapshot => "snapshot",
            Part::Rows => "rows",
            Part::Shape => "shape",
            Part::Blur => "blur",
            Part::Images => "images",
            Part::Paint => "paint",
        }
    }
}

/// What the renderer counts besides time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Count {
    /// Rows rebuilt by the row cache.
    RowsBuilt,
    /// Words handed to the text system (each new word once, by text and fonts).
    WordsShaped,
}

/// One frame's terminal parts: time and allocations per [`Part`], summed over the terminals the
/// frame drew (a split draws several).
#[cfg(any(test, feature = "devctl"))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Parts {
    /// Terminal grids prepared in the frame (0: the terminal reused its last paint).
    pub terminals: u32,
    /// Nanoseconds per part, in [`Part::ALL`]'s order.
    pub nanos: [u64; Part::ALL.len()],
    /// Allocations per part, in the same order.
    pub allocations: [u64; Part::ALL.len()],
    pub rows_built: u64,
    pub words_shaped: u64,
}

#[cfg(feature = "devctl")]
mod recording {
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    use super::allocations::{self, thread_counts};
    use super::{Count, Part, Parts};

    /// Set while `perf-profile` records.
    static RECORDING: AtomicBool = AtomicBool::new(false);

    thread_local! {
        /// The frame being drawn (the UI thread draws them all).
        static FRAME: RefCell<Parts> = RefCell::new(Parts::default());
    }

    /// A part being timed, from [`span`] to [`Span::end`]; empty while nothing records.
    pub(crate) struct Span(Option<(Instant, u64)>);

    #[inline]
    pub(crate) fn span() -> Span {
        if RECORDING.load(Ordering::Relaxed) {
            Span(Some((Instant::now(), thread_counts().allocations)))
        } else {
            Span(None)
        }
    }

    impl Span {
        #[inline]
        pub(crate) fn end(self, part: Part) {
            let Some((at, allocated)) = self.0 else { return };
            let nanos = u64::try_from(at.elapsed().as_nanos()).unwrap_or(u64::MAX);
            let allocations = thread_counts().allocations.saturating_sub(allocated);
            FRAME.with_borrow_mut(|frame| {
                let i = part as usize;
                frame.nanos[i] += nanos;
                frame.allocations[i] += allocations;
                if part == Part::Prepare {
                    frame.terminals += 1;
                }
            });
        }
    }

    #[inline]
    pub(crate) fn count(what: Count, n: usize) {
        if RECORDING.load(Ordering::Relaxed) {
            FRAME.with_borrow_mut(|frame| match what {
                Count::RowsBuilt => frame.rows_built += n as u64,
                Count::WordsShaped => frame.words_shaped += n as u64,
            });
        }
    }

    /// Start recording: frames' parts, every thread's allocations and the terminal layer's
    /// counters (reset).
    pub(crate) fn start() {
        wormhole_core::terminal::perf::start();
        allocations::set_counting(true);
        RECORDING.store(true, Ordering::Relaxed);
    }

    /// Stop recording, and the terminal layer's counters since [`start`].
    pub(crate) fn stop() -> wormhole_core::terminal::perf::Counters {
        RECORDING.store(false, Ordering::Relaxed);
        allocations::set_counting(false);
        wormhole_core::terminal::perf::stop()
    }

    /// Forget what was recorded since the last frame (frames drawn between the ones measured).
    pub(crate) fn begin_frame() {
        FRAME.with_borrow_mut(|frame| *frame = Parts::default());
    }

    /// What the frame just drawn recorded.
    pub(crate) fn take_frame() -> Parts {
        FRAME.with_borrow_mut(std::mem::take)
    }
}

#[cfg(feature = "devctl")]
pub(crate) use recording::{begin_frame, count, span, start, stop, take_frame};

/// Without devctl: nothing, at no cost.
#[cfg(not(feature = "devctl"))]
pub(crate) struct Span;

#[cfg(not(feature = "devctl"))]
#[inline(always)]
pub(crate) fn span() -> Span {
    Span
}

#[cfg(not(feature = "devctl"))]
impl Span {
    #[inline(always)]
    pub(crate) fn end(self, _part: Part) {}
}

#[cfg(not(feature = "devctl"))]
#[inline(always)]
pub(crate) fn count(_what: Count, _n: usize) {}
