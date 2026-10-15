//! A repaint into a buffer that has been used allocates nothing: the writer is called while the
//! host holds the screen's lock, and its cost must be the bytes it writes. Titles, colors, links,
//! wide characters, shell marks, the alternate screen and a long history are all in the terminal
//! being written.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.
#![allow(unsafe_code)] // only the counting allocator below, which forwards to the system's

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use wormhole_vt::parser::Parser;
use wormhole_vt::term::Terminal;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: every call is forwarded unchanged to `System`, which upholds `GlobalAlloc`'s contract;
// the counter is a thread-local `Cell` and touches no memory of the caller.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|n| n.set(n.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|n| n.set(n.get() + 1));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

fn busy_terminal() -> (Terminal, Parser) {
    let mut term = Terminal::new(100, 30, 2000);
    let mut parser = Parser::new();
    let mut stream = String::from("\x1b]2;a title\x07\x1b]7;file:///tmp/work\x07\x1b]4;1;rgb:ff/00/00\x07");
    for i in 0..3000 {
        stream.push_str(&format!(
            "\x1b]133;A\x07\x1b[32m$ \x1b]133;B\x07ls {i}\r\n\x1b]133;C\x07\x1b[1;38;2;{};10;200mline\x1b[m \
             \x1b]8;id=l{i};http://example.com/{i}\x07link\x1b]8;;\x07 日本語 e\u{301} \x1b[44mbg\x1b[K\x1b[m\r\n\
             \x1b]133;D;0\x07",
            i % 255
        ));
    }
    stream.push_str("\x1b[?1049h\x1b[5;5Halternate\x1b[?1049l\x1b[?1049h\x1b[3;3Hscreen");
    term.advance(&mut parser, stream.as_bytes());
    term.clear_replies();
    term.drain_events().for_each(drop);
    (term, parser)
}

#[test]
fn a_repaint_into_a_used_buffer_allocates_nothing() {
    let (term, parser) = busy_terminal();
    let mut out = Vec::new();
    // The first repaint grows the buffer; the second is the steady state.
    let first = term.repaint(&parser, &mut out).expect("ready");
    assert!(first.gaps.is_empty(), "{:?}", first.gaps);
    assert!(first.history_lines > 1000, "a long history is what costs");
    let size = out.len();
    out.clear();
    let before = allocations();
    let again = term.repaint(&parser, &mut out).expect("ready");
    let count = allocations() - before;
    eprintln!("a repaint of {size} bytes: {count} allocations");
    assert_eq!(out.len(), size, "the same terminal repaints the same");
    assert_eq!(again, first);
    assert_eq!(count, 0, "a repaint into a used buffer must not allocate");
}
