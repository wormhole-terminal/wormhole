//! The terminal allocates nothing in the steady state: text with wrap and scroll, colors, cursor
//! addressing, erases, scrolling regions, titles, marks, replies, marks on cells, switching
//! screens. A payload an event hands to the host owns its bytes and is the exception.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.
#![allow(unsafe_code)] // only the counting allocator below, which forwards to the system's

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use wormhole_vt::Terminal;

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
    steady_on(Terminal::builder().size(80, 24).scrollback(200).build(), name, stream, rounds)
}

/// [`steady`] on a terminal the caller made.
fn steady_on(mut term: Terminal, name: &str, stream: &[u8], rounds: usize) -> usize {
    let run = |term: &mut Terminal, times: usize| {
        for _ in 0..times {
            for piece in stream.chunks(4096) {
                term.feed(piece);
                term.events().for_each(drop);
                term.clear_replies();
            }
        }
    };
    run(&mut term, 40);
    let before = allocations();
    run(&mut term, rounds);
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

/// Lines leaving the history cost nothing when the host did not ask for them, with the caps at
/// their ceilings or lowered; and asking for them costs nothing while nothing leaves.
#[test]
fn history_leaving_allocates_nothing_unless_the_host_asked_for_it() {
    let line = "the quick brown fox jumps over the lazy dog, again and again and again and again, ";
    let stream = repeat(&format!("{line}\r\n"), 400);
    let mut limits = wormhole_vt::Limits::default();
    limits.image_bytes = 1 << 20;
    limits.title = 100;
    limits.archive_backlog = 1 << 20;
    let term = Terminal::builder().size(80, 24).scrollback(200).limits(limits).archive_history(false).build();
    assert_eq!(steady_on(term, "archive off, limits lowered", &stream, 20), 0);
    // Nothing leaves while the history grows (its pages are allocated either way): switching the
    // archive on adds nothing to that.
    let history =
        |archive| Terminal::builder().size(80, 24).scrollback(1_000_000).archive_history(archive).build();
    let without = steady_on(history(false), "growing history, archive off", &stream, 20);
    assert_eq!(steady_on(history(true), "growing history, archive on", &stream, 20), without);
}

/// What archiving costs per line that leaves: a vector of the row's cells, and a little for the batches.
#[test]
fn archiving_allocates_a_vector_per_line_and_no_more() {
    let line = "the quick brown fox jumps over the lazy dog, again and again and again and again, ";
    let stream = repeat(&format!("{line}\r\n"), 400);
    let term = Terminal::builder().size(80, 24).scrollback(200).archive_history(true).build();
    let rounds = 20;
    let count = steady_on(term, "archive on", &stream, rounds);
    // 400 lines of 82 characters are 800 rows a round, all of which leave once the history is full.
    let lines = 800 * rounds;
    assert!(count <= lines + lines / 10, "{count} allocations for {lines} lines");
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

/// A deterministic source of numbers for the streams below: the same seed gives the same bytes.
/// (`tests/common` has the corpora; these tests only need a few shapes, kept beside the assertions.)
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

/// Hyperlinks cost nothing once their text is known: the same link again, a listing of files
/// printed again and again (`ls --hyperlink`, a build log naming its sources), links with ids and
/// without. What the page keeps of a link (a run of cells and a count) lives in tables that are
/// reused with the page, and a short id (all the generated ones) is stored in the link itself.
#[test]
fn links_allocate_nothing_once_their_text_is_known() {
    let with_id = "\x1b]8;id=a1;file:///work/src/main.rs\x07main.rs:12\x1b]8;;\x07 text\r\n";
    assert_eq!(steady("one link with an id", &repeat(with_id, 200), 20), 0);
    // An id too long to be stored in the link is shared like the address.
    let long_id =
        "\x1b]8;id=a-link-id-of-more-than-twenty-two-bytes;file:///work/src/lib.rs\x07lib.rs\x1b]8;;\x07\r\n";
    assert_eq!(steady("one link with a long id", &repeat(long_id, 200), 20), 0);

    // The same files under 40 ids, in an order a program chose.
    let mut rng = Lcg(5);
    let mut mixed = String::new();
    for _ in 0..600 {
        let (id, file) = (rng.below(40), rng.below(300));
        mixed.push_str(&format!("\x1b]8;id=l{id};https://example.com/{file}\x1b\\word{id}\x1b]8;;\x1b\\ "));
    }
    assert_eq!(steady("mixed", mixed.as_bytes(), 20), 0);

    // No id: every link gets one of its own, and the same address is still shared.
    let without_id = "\x1b]8;;file:///work/src/main.rs\x07main.rs:12\x1b]8;;\x07 text\r\n";
    assert_eq!(steady("one link without an id", &repeat(without_id, 200), 20), 0);

    // A listing of 300 files, each a link of its own.
    let mut listing = String::new();
    for n in 0..300 {
        let target = format!("file:///work/crate{}/src/module{n}.rs", n % 7);
        listing.push_str(&format!("\x1b]8;;{target}\x1b\\file{n}.rs\x1b]8;;\x1b\\  "));
        if n % 6 == 5 {
            listing.push_str("\r\n");
        }
    }
    assert_eq!(steady("listing", listing.as_bytes(), 20), 0);
}

/// Text full of wide characters and emoji clusters under mode 2027: the zero-width characters of
/// a cell live in tables of the page that are reused with it.
#[test]
fn emoji_clusters_allocate_nothing_in_the_steady_state() {
    let pieces = [
        "日本語",
        "テキスト",
        "漢字",
        "한국어",
        "😀",
        "🚀",
        "👨\u{200d}👩\u{200d}👧",
        "🇯🇵",
        "👍🏽",
        "e\u{301}",
        "a\u{308}\u{323}",
        "❤\u{fe0f}",
        "1\u{fe0f}\u{20e3}",
        "word ",
        "text ",
    ];
    let mut rng = Lcg(7);
    let mut text = String::from("\x1b[?2027h");
    for _ in 0..300 {
        for _ in 0..(5 + rng.below(25)) {
            text.push_str(pieces[rng.below(pieces.len() as u64) as usize]);
        }
        text.push_str("\r\n");
    }
    assert_eq!(steady("emoji clusters", text.as_bytes(), 20), 0);
}

/// Asking what changed is part of every frame: once the `Damage` has seen the screen's size it
/// allocates nothing, whether the program scrolled, moved the cursor or changed the colours.
#[test]
fn taking_damage_allocates_nothing() {
    let mut term = Terminal::builder().size(80, 24).scrollback(200).build();
    let mut damage = wormhole_vt::Damage::new();
    let unit = "line of text\r\n\x1b[3;5Hx\x1b]4;1;#102030\x07\x1b[?25l\x1b[?25h\x1b]104\x07";
    let stream = repeat(unit, 50);
    let round = |term: &mut Terminal, damage: &mut wormhole_vt::Damage| {
        for piece in stream.chunks(300) {
            term.feed(piece);
            term.take_damage(damage);
            assert!(damage.rows().count() <= 24);
        }
    };
    for _ in 0..40 {
        round(&mut term, &mut damage);
    }
    let before = allocations();
    for _ in 0..20 {
        round(&mut term, &mut damage);
    }
    assert_eq!(allocations() - before, 0);
}

fn with_passthrough(passthrough: wormhole_vt::passthrough::Passthrough) -> Terminal {
    let Ok(builder) = Terminal::builder().size(80, 24).scrollback(200).passthrough(passthrough) else {
        panic!("the claims are fine")
    };
    builder.build()
}

/// Asking for strings costs nothing on the traffic nobody asked about: it is compared with the
/// claims only when a string ends.
#[test]
fn passthrough_allocates_nothing_for_what_it_was_not_asked_to_pass() {
    use wormhole_vt::passthrough::Passthrough;
    let term = with_passthrough(Passthrough::new().osc(51234, 1024).dcs(b"!", b'z', 64).apc(b'Q', 64));
    let unit = "text \x1b[1mbold\x1b[0m\x1b]51235;not asked\x07\x1bP!ynot asked\x1b\\\x1b_Rnot asked\x1b\\\
                \x1b]2;title\x07\x1b[5n more text\r\n";
    assert_eq!(steady_on(term, "unclaimed", &repeat(unit, 300), 20), 0);
}

/// A string that is passed through is an event, which owns its payload: the allocations are the
/// payload's and no more.
#[test]
fn a_string_that_is_passed_through_costs_its_payload() {
    use wormhole_vt::passthrough::Passthrough;
    let term = with_passthrough(Passthrough::new().osc(51234, 1024).dcs(b"!", b'z', 64).apc(b'Q', 64));
    let unit = "text\x1b]51234;payload\x07\x1bP!zdata\x1b\\\x1b_Qdata\x1b\\\r\n";
    let strings = 3 * 300 * 20;
    let count = steady_on(term, "passed through", &repeat(unit, 300), 20);
    // The collected buffer each (the event owns it, so the next string starts a new one), and for a
    // DCS also its header's numbers and intermediates.
    assert!(count <= 6 * 300 * 20, "{count} allocations for {strings} strings");
}
