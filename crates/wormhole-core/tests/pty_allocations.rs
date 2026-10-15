//! The PTY reader thread allocates nothing while it reads and parses output, whatever the
//! output: a counting allocator watches the thread named "PTY reader" through floods of plain
//! text, truecolor text, escapes cut by the reads wherever they fall and cursor moves, the host and
//! the parser included.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.
#![allow(unsafe_code)] // only the counting allocator below, which forwards to the system's

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use wormhole_core::shells::ShellCommand;
use wormhole_core::terminal::{GridSize, Pty, SpawnOptions};

/// Allocations made by the reader thread so far.
static READER_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// This thread is inside the allocator already (looking up its own name may allocate).
    static INSIDE: Cell<bool> = const { Cell::new(false) };
}

fn count_if_reader() {
    if INSIDE.try_with(|inside| inside.replace(true)).unwrap_or(true) {
        return;
    }
    if std::thread::current().name() == Some("PTY reader") {
        READER_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }
    let _ = INSIDE.try_with(|inside| inside.set(false));
}

struct Counting;

// SAFETY: every call is forwarded unchanged to `System`, which upholds `GlobalAlloc`'s contract;
// the counting touches no memory of the caller.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_if_reader();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_if_reader();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const SIZE: GridSize = GridSize { cols: 100, rows: 30, cell_width: 8.0, cell_height: 16.0 };

/// Run `script` in a terminal and say how many allocations the reader made between its 20,000th
/// and its 100,000th pass over the output (a pass is a read and its parsing). By the first the
/// buffers are allocated and the history has grown to its cap, which takes its pages as the
/// screen scrolls into it (a program that scrolls once in 400 bytes, as the cursor-moves flood
/// does, was still growing it at the 2,000th pass); the end of the program, with its exit event,
/// is still far off at the second.
fn reader_allocations(script: &str) -> usize {
    let options = SpawnOptions {
        shell: Some(ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            name: "sh".into(),
        }),
        ..SpawnOptions::default()
    };
    // The events stay unread: taking them would let the reader send more, and the channel
    // allocates a block of them now and then.
    let (pty, _events) = Pty::spawn(SIZE, &options).expect("starts");
    let deadline = Instant::now() + Duration::from_secs(120);
    let (mut from, mut to) = (None, None);
    while to.is_none() {
        assert!(pty.is_alive(), "the program ended before its 100,000th pass");
        assert!(Instant::now() < deadline, "the program did not get that far");
        match pty.output_revision() {
            0..20_000 => {}
            20_000..100_000 => from = from.or(Some(READER_ALLOCATIONS.load(Ordering::Relaxed))),
            _ => to = Some(READER_ALLOCATIONS.load(Ordering::Relaxed)),
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    to.unwrap_or_default() - from.unwrap_or_default()
}

/// One test, so the floods never run at the same time (the counter counts every reader).
#[test]
fn reading_and_parsing_allocates_nothing() {
    let line = "The quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnopqrstuvwxyz";
    let floods = [
        ("plain text", format!("yes '{line}' | head -c 41943040")),
        (
            "truecolor",
            awk(r#"printf "\033[38;2;%d;%d;%dmword\033[0m\r\n", i % 256, i * 7 % 256, i * 13 % 256"#),
        ),
        // No line ends: the program's writes and the terminal's reads cut the escapes anywhere.
        (
            "unbroken escapes",
            awk_times(8_000_000, r#"printf "\033[38;2;%d;%d;%dmx", i % 256, i * 7 % 256, i * 13 % 256"#),
        ),
        ("cursor moves", awk(r#"printf "\033[%d;%dHword\r\n", i % 30 + 1, i % 90 + 1"#)),
        (
            "both",
            awk(
                r#"printf "\033[38;2;%d;%d;%dm\033[%d;%dHword\033[0m\r\n", i % 256, i * 7 % 256, i * 13 % 256, i % 30 + 1, i % 90 + 1"#,
            ),
        ),
    ];
    for (name, script) in floods {
        let allocations = reader_allocations(&script);
        assert_eq!(allocations, 0, "{name}: allocations between pass 20,000 and 100,000");
    }
}

fn awk(print: &str) -> String {
    awk_times(1_000_000, print)
}

/// A program that runs `print` (an awk statement) for `i` from 0 to `times`.
fn awk_times(times: u32, print: &str) -> String {
    format!("awk 'BEGIN {{ for (i = 0; i < {times}; i++) {print} }}'")
}
