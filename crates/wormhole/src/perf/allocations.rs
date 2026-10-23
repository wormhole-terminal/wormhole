//! The allocator of test and devctl builds: the system's, counting each thread's allocations and
//! the bytes they asked for. Tests count always (the frame benchmarks in `render/bench.rs` read
//! their own thread's count, so tests running in parallel don't disturb a measurement); a devctl
//! build counts only while `perf-profile` records, and pays one relaxed load of a flag per
//! allocation otherwise. The shipped build has none of this: it uses the system allocator directly.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set while `perf-profile` records.
static COUNTING: AtomicBool = AtomicBool::new(false);

thread_local! {
    // `const` and without a destructor: touching it never allocates.
    static COUNTS: Cell<Counts> = const { Cell::new(Counts { allocations: 0, bytes: 0 }) };
}

/// A thread's allocations (and reallocations) so far, and the bytes they asked for: a new block's
/// size, and what a reallocation grew by (a shrink adds none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub allocations: u64,
    pub bytes: u64,
}

impl Counts {
    /// What was counted between `before` and this.
    pub(crate) fn since(self, before: Counts) -> Counts {
        Counts {
            allocations: self.allocations.saturating_sub(before.allocations),
            bytes: self.bytes.saturating_sub(before.bytes),
        }
    }
}

/// Count every thread's allocations from now on, or stop (tests always count).
#[cfg_attr(not(feature = "devctl"), allow(dead_code))]
pub(crate) fn set_counting(on: bool) {
    COUNTING.store(on, Ordering::Relaxed);
}

/// This thread's counts so far.
pub(crate) fn thread_counts() -> Counts {
    COUNTS.try_with(Cell::get).unwrap_or_default()
}

/// Run `f` and leave this thread's counts as they were: for a check that is part of a debug or test build
/// and would otherwise be counted as the code it checks.
#[cfg(any(test, debug_assertions))]
pub(crate) fn uncounted<R>(f: impl FnOnce() -> R) -> R {
    let before = thread_counts();
    let out = f();
    let _ = COUNTS.try_with(|counts| counts.set(before));
    out
}

#[inline]
fn note(bytes: usize) {
    if cfg!(test) || COUNTING.load(Ordering::Relaxed) {
        let _ = COUNTS.try_with(|counts| {
            let Counts { allocations, bytes: total } = counts.get();
            counts.set(Counts { allocations: allocations + 1, bytes: total + bytes as u64 });
        });
    }
}

/// See the module docs.
struct Counting;

// SAFETY: every method forwards its arguments unchanged to `System`, which upholds the
// `GlobalAlloc` contract; counting touches only a flag and a thread-local without a destructor.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: the caller's `layout` obligations pass straight through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: as for `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System` with this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size.saturating_sub(layout.size()));
        // SAFETY: as for `alloc` and `dealloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[cfg(test)]
mod tests {
    use super::{Counts, thread_counts};

    #[test]
    fn a_threads_allocations_and_their_bytes_are_counted() {
        let before = thread_counts();
        let kept = std::hint::black_box(Vec::<u8>::with_capacity(1000));
        let counted = thread_counts().since(before);
        assert_eq!(counted.allocations, 1);
        assert!(counted.bytes >= 1000, "{counted:?}");
        drop(kept);
    }

    #[test]
    fn counts_since_never_go_below_zero() {
        let later = Counts { allocations: 3, bytes: 10 };
        assert_eq!(later.since(Counts { allocations: 5, bytes: 4 }), Counts { allocations: 0, bytes: 6 });
    }
}
