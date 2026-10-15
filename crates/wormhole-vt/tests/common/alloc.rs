//! A counting allocator for the tests and examples that pin down how much memory the engine
//! asks for. The crate itself has no `unsafe`; this forwards to the system's allocator.
//!
//! A binary that uses it declares `#[global_allocator] static ALLOCATOR: Counting = Counting;`.
//! The allocator is global, so a test that must not be counted by another lives in a binary of its
//! own (`tests/*_allocations.rs`). The call counter is per thread (the test harness's other
//! threads do their own allocating); the live bytes are global.
#![allow(unsafe_code)] // only the forwarding below
#![allow(dead_code)] // every includer uses a part

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! {
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

static LIVE: AtomicUsize = AtomicUsize::new(0);

/// Allocations (and reallocations) this thread has made so far.
pub fn allocations() -> usize {
    CALLS.with(Cell::get)
}

/// Bytes allocated and not yet freed, all threads.
pub fn live_bytes() -> usize {
    LIVE.load(Ordering::Relaxed)
}

fn count_call() {
    CALLS.with(|n| n.set(n.get() + 1));
}

pub struct Counting;

// SAFETY: every call is forwarded unchanged to `System`, which upholds `GlobalAlloc`'s contract;
// the counters are a thread-local `Cell` and an atomic and touch no memory of the caller.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_call();
        LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_call();
        LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_call();
        LIVE.fetch_add(new_size, Ordering::Relaxed);
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
