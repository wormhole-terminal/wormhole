//! A message without variables costs no allocation: `text` is an array index after one atomic read.
//! A counting allocator watches this thread. Its own test binary, because the allocator is global.
#![allow(unsafe_code)] // only the counting allocator below, which forwards to the system's

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use wormhole_text::keys::*;
use wormhole_text::{Language, text, with_language};

thread_local! {
    /// Allocations this thread has made. A `const` cell without a destructor: reading it allocates nothing.
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

fn count() {
    let _ = COUNT.try_with(|c| c.set(c.get() + 1));
}

// SAFETY: every call is forwarded unchanged to `System`, which upholds `GlobalAlloc`'s contract;
// the counting touches no memory of the caller.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations() -> usize {
    COUNT.with(Cell::get)
}

#[test]
fn a_plain_message_allocates_nothing() {
    // The first ask loads English; that is the one time it may allocate.
    let mut sink = text(SETTINGS_THEME_MODE).len();
    let before = allocations();
    for _ in 0..10_000 {
        sink += text(SETTINGS_THEME_MODE).len();
        sink += text(SETTINGS_THEME_MODE_DESCRIPTION).len();
        sink += text(SETTINGS_SECTION_SESSIONS).len();
    }
    assert_eq!(allocations() - before, 0, "text() allocated");
    assert!(sink > 0);
}

#[test]
fn a_plain_message_in_another_language_allocates_nothing_either() {
    with_language(Language::PSEUDO, || {
        let mut sink = text(SETTINGS_THEME_MODE).len();
        let before = allocations();
        for _ in 0..10_000 {
            sink += text(SETTINGS_THEME_MODE).len();
            sink += text(SETTINGS_SECTION_SESSIONS).len();
        }
        assert_eq!(allocations() - before, 0, "text() allocated under with_language");
        assert!(sink > 0);
    });
}

#[test]
fn a_message_with_variables_costs_a_few_small_allocations_not_a_load() {
    let _ = settings_font_count(2);
    let before = allocations();
    let _ = settings_font_count(2);
    let used = allocations() - before;
    assert!(used <= 8, "a formatted message took {used} allocations");
}
