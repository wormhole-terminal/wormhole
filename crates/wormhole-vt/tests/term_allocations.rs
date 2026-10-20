//! The terminal allocates nothing in the steady state: text with wrap and scroll, colors, cursor
//! addressing, erases, scrolling regions, titles, marks, replies, marks on cells, switching
//! screens. A payload an event hands to the host owns its bytes and is the exception.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.
#![allow(unsafe_code)] // only the counting allocator below, which forwards to the system's

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use wormhole_vt::parser::Parser;
use wormhole_vt::terminal::Terminal;

thread_local! {
    /// This thread's allocations (the test harness's other threads do their own).
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
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

/// Feed `stream` until the terminal is warm (its history full, its buffers grown), then count
/// the allocations of feeding it again `rounds` times, draining what the terminal hands out the
/// way a host does.
fn steady(name: &str, stream: &[u8], rounds: usize) -> usize {
    let mut term = Terminal::new(80, 24, 200);
    let mut parser = Parser::new();
    let run = |term: &mut Terminal, parser: &mut Parser, times: usize| {
        for _ in 0..times {
            for piece in stream.chunks(4096) {
                term.advance(parser, piece);
                term.drain_events().for_each(drop);
                term.clear_replies();
            }
        }
    };
    run(&mut term, &mut parser, 40);
    let before = allocations();
    run(&mut term, &mut parser, rounds);
    let count = allocations() - before;
    eprintln!("{name}: {count} allocations");
    count
}

fn repeat(unit: &str, times: usize) -> Vec<u8> {
    unit.repeat(times).into_bytes()
}

#[test]
fn text_with_wrap_and_scroll_allocates_nothing() {
    let line = "the quick brown fox jumps over the lazy dog, again and again and again and again, ";
    assert_eq!(steady("ascii", &repeat(line, 400), 20), 0);
    assert_eq!(steady("lines", &repeat(&format!("{line}\r\n"), 400), 20), 0);
}

#[test]
fn colors_and_attributes_allocate_nothing() {
    let unit = "\x1b[1;31mred\x1b[0m \x1b[38;5;208morange\x1b[48;2;1;2;3m rgb \x1b[4:3;58;2;9;9;9mcurl\x1b[0m plain text\r\n";
    assert_eq!(steady("sgr", &repeat(unit, 300), 20), 0);
}

#[test]
fn cursor_addressing_and_erasing_allocate_nothing() {
    let unit = "\x1b[3;7Hhello\x1b[K\x1b[10;1H\x1b[2K\x1b[20;5Hworld\x1b[1J\x1b[H\x1b[2J\x1b[5;20r\x1b[5;1H\n\n\x1bM\x1b[r\x1b[2@\x1b[3P\x1b[L\x1b[M\x1b[4X\x1b[?25l\x1b[?25h\x1b7\x1b8";
    assert_eq!(steady("tui", &repeat(unit, 300), 20), 0);
}

#[test]
fn titles_marks_links_and_bells_allocate_nothing_but_what_a_link_owns() {
    let unit = "\x1b]0;user@host: ~/work/app\x07\x1b]7;file://host/work/app\x07\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07out\r\n\x1b]133;D;0\x07\x07\x07";
    assert_eq!(steady("shell", &repeat(unit, 200), 20), 0);
}

#[test]
fn replies_allocate_nothing() {
    let unit = "\x1b[5n\x1b[6n\x1b[c\x1b[>c\x1b[?2004$p\x1b[4$p\x1b[18t\x1b[?u\x1b]11;?\x07\x1b]4;1;?\x07\x1bP$qm\x1b\\\x1b[>q";
    assert_eq!(steady("replies", &repeat(unit, 200), 20), 0);
}

#[test]
fn marks_on_cells_and_wide_characters_allocate_nothing() {
    let unit = "日本語 e\u{301} a\u{300}\u{323} 👨\u{200d}👩\u{200d}👧 🇯🇵 ❤\u{fe0f} ü ß\r\n";
    assert_eq!(steady("unicode", &repeat(unit, 300), 20), 0);
    let clusters = format!("\x1b[?2027h{}", unit.repeat(300));
    assert_eq!(steady("clusters", clusters.as_bytes(), 20), 0);
}

#[test]
fn rectangular_operations_allocate_nothing() {
    // Fill, copy (overlapping, across the screen), change and reverse attributes, erase, over
    // text with attributes and a link; DECRQCRA is off, as it starts.
    let unit = "\x1b[1;31mabc\x1b[0m link 日本語\r\n\
                \x1b[37;2;2;10;40$x\x1b[3;3;12;60;1;5;9;1$v\x1b[5;5;9;70;1;4;4;1$v\x1b[2;2;9;9;1;4;7$r\x1b[2*x\
                \x1b[2;2;9;9;7;5$t\x1b[0*x\x1b[1;1;3;70;0$t\x1b[4;4;8;8$z\x1b[10;10;20;30${\x1b[1;0;1;1;5;5*y";
    assert_eq!(steady("rectangles", &repeat(unit, 100), 20), 0);
}

#[test]
fn switching_screens_allocates_nothing() {
    let unit = "\x1b[?1049hfull screen\x1b[2J\x1b[H\x1b[1;1Hx\x1b[?1049l\x1b[?47h\x1b[?47l main \r\n";
    assert_eq!(steady("altscreen", &repeat(unit, 300), 20), 0);
}

/// A hyperlink owns its target and its id (two `Arc<str>`): that is all it allocates, however
/// many cells it covers or lines it scrolls past.
#[test]
fn a_hyperlink_allocates_what_it_owns_and_no_more() {
    let unit = "\x1b]8;;file:///work/src/main.rs\x07main.rs:12\x1b]8;;\x07 text\r\n";
    let links = 200 * 20;
    let count = steady("links", &repeat(unit, 200), 20);
    assert!(count <= 2 * links, "{count} allocations for {links} links");
    let with_id = "\x1b]8;id=a1;file:///work/src/main.rs\x07main.rs:12\x1b]8;;\x07 text\r\n";
    let count = steady("links with an id", &repeat(with_id, 200), 20);
    assert!(count <= 2 * links, "{count} allocations for {links} links");
}
