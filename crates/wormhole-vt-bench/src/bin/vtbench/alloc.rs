//! A global allocator that counts, for the allocation figures of vtbench.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use wormhole_vt_bench::bench::AllocCounts;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

/// The system allocator with two counters beside it: calls that obtain memory (`alloc`,
/// `alloc_zeroed`, `realloc`) and the bytes they asked for. Freeing is not counted: a loop that
/// allocates and frees once per byte is the cost this exists to find.
pub struct Counting;

// SAFETY: every method forwards to `System` with the arguments it was given, so the contract of
// `GlobalAlloc` is `System`'s; the counters are atomics and take no part in it.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract for `layout`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: the caller upholds `GlobalAlloc::alloc_zeroed`'s contract for `layout`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator with `layout`, as the caller promises.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        // SAFETY: `ptr` and `layout` describe a live block of this allocator and `new_size` is
        // valid, as the caller promises.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// What has been allocated so far.
pub fn counts() -> AllocCounts {
    AllocCounts { allocs: ALLOCS.load(Ordering::Relaxed), bytes: BYTES.load(Ordering::Relaxed) }
}
