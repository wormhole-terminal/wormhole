//! Memory stays bounded: hundreds of megabytes of hostile input, built to make a terminal grow
//! (strings that never end, parameters without limit, pictures, floods nobody collects), leave it
//! holding no more than the caps in `docs/security.md` allow. The peak is measured at the allocator,
//! after every megabyte fed, so it counts everything the engine holds, the parser's and the queues'
//! buffers included, not only what the terminal can name.
//!
//! This file is a test binary of its own because the allocator is global: the cases wait for each
//! other (`LOCK`), and nothing else allocates while one runs.
//!
//! The default run feeds about 600 MB and takes a few seconds; `-- --ignored` feeds five times as
//! much (`cargo test -p wormhole-vt --test invariants_memory --release -- --ignored`).

use std::sync::Mutex;

use wormhole_vt::Terminal;

#[path = "common/mod.rs"]
mod common;
use common::alloc::{Counting, live_bytes};
use common::rng::Rng;

#[global_allocator]
static ALLOCATOR: Counting = Counting;

static LOCK: Mutex<()> = Mutex::new(());

const MIB: usize = 1024 * 1024;

/// What a case measured.
struct Run {
    /// The most live bytes above the start, seen after any megabyte.
    peak: usize,
    /// Events waiting at the end, nobody having collected them (when the case does not).
    events: usize,
    /// The terminal, for the checks that look inside it.
    term: Terminal,
}

/// Feed `megabytes` of `piece(n)`'s output (`n` counts the pieces, each about 1 MiB) to a terminal
/// of `cols` by `rows` with `scrollback` lines; `drain` takes the events and replies after each
/// piece, as a host does.
fn run(
    megabytes: usize,
    (cols, rows, scrollback): (usize, usize, usize),
    drain: bool,
    mut piece: impl FnMut(usize) -> Vec<u8>,
) -> Run {
    let mut term = Terminal::builder().size(cols, rows).scrollback(scrollback).build();
    let base = live_bytes();
    let mut peak = 0;
    for n in 0..megabytes {
        let bytes = piece(n);
        // A host reads the PTY a few tens of KiB at a time.
        for part in bytes.chunks(64 * 1024) {
            term.feed(part);
        }
        if drain {
            let _ = term.events().count();
            term.clear_replies();
        }
        peak = peak.max(live_bytes().saturating_sub(base + bytes.len()));
    }
    let events = if drain { 0 } else { term.events().count() };
    Run { peak, events, term }
}

/// `unit` repeated to about a mebibyte.
fn repeated(unit: &[u8]) -> Vec<u8> {
    unit.repeat(MIB / unit.len() + 1)
}

fn check_peak(case: &str, run: &Run, limit: usize) {
    eprintln!("{case}: peak {} KiB of {} KiB allowed", run.peak / 1024, limit / 1024);
    assert!(
        run.peak <= limit,
        "{case}: the terminal held {} MiB at its peak, the documented bound is {} MiB",
        run.peak / MIB,
        limit / MIB
    );
    if let Err(why) = run.term.check_invariants() {
        panic!("{case}: the terminal is inconsistent afterwards: {why}");
    }
}

/// The default and the heavy run of every case.
fn all_cases(scale: usize) {
    let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

    // A string that never ends costs the bytes of the cap, not of the stream: a title is kept to 8 KiB.
    let mut first = true;
    let titles = run(8 * scale, (80, 24, 100), true, |_| {
        if std::mem::take(&mut first) {
            let mut out = b"\x1b]2;".to_vec();
            out.extend(repeated(b"A"));
            out
        } else {
            repeated(b"A")
        }
    });
    check_peak("an OSC 2 that never ends", &titles, MIB);
    assert!(titles.term.title().is_none(), "an unfinished title is not set");

    // Strings that do end, but are over their caps: dropped whole. OSC 52 is kept to 8 MiB, OSC 1337 to
    // 64, so the stream is longer than that, however `scale` is.
    for (case, head, megabytes, limit) in
        [("OSC 52", "\x1b]52;c;", 12, 10 * MIB), ("OSC 1337", "\x1b]1337;File=inline=1:", 72, 72 * MIB)]
    {
        let long = run(megabytes * (scale / 4).max(1), (80, 24, 100), true, |n| {
            let mut out = if n == 0 { head.as_bytes().to_vec() } else { Vec::new() };
            out.extend(repeated(b"QUJD"));
            if n % megabytes == megabytes - 1 {
                out.extend_from_slice(b"\x07");
            }
            out
        });
        check_peak(case, &long, limit);
    }

    // Parameters without end: a CSI keeps 32 values and a few bytes, however many it is sent.
    for (case, unit) in [
        ("endless parameters", &b"1;"[..]),
        ("endless sub-parameters", b"1:2:3:"),
        ("endless intermediates", b"!!!!!!!!"),
        ("empty parameters", b";;;;;;;;;;;;;;;;;;;"),
    ] {
        let mut first = true;
        let params = run(6 * scale, (80, 24, 100), true, |_| {
            let mut out = if std::mem::take(&mut first) { b"\x1b[".to_vec() } else { Vec::new() };
            out.extend(repeated(unit));
            out
        });
        check_peak(case, &params, MIB);
    }

    // Strings that start and never finish, over and over: the parser holds one.
    let starts =
        run(4 * scale, (80, 24, 100), true, |_| repeated(b"\x1b]0;x\x1bP1q\x1b_Gm=1;\x1b^pm\x1bXsos"));
    check_peak("strings begun and abandoned", &starts, MIB);

    // Sixel garbage, ended every few megabytes: pictures cost what the picture budgets say (128 MiB
    // kept, 4 MiB of data a time), and a host that collects uploads holds none.
    let mut rng = Rng::new(7);
    let sixel = run(24 * scale, (100, 30, 100), true, |n| {
        let mut out = Vec::with_capacity(MIB + 64);
        if n % 4 == 0 {
            out.extend_from_slice(b"\x1bPq\"1;1;4000;4000#0;2;100;0;0");
        }
        while out.len() < MIB {
            match rng.below(6) {
                0 => out.extend_from_slice(b"!99999999~"),
                1 => out.extend_from_slice(
                    format!("#{};2;{};{};{}", rng.below(300), rng.below(120), rng.below(120), rng.below(120))
                        .as_bytes(),
                ),
                2 => out.extend_from_slice(b"-"),
                3 => out.extend_from_slice(b"$"),
                4 => out.extend_from_slice(
                    format!("!{}{}", rng.below(5000), rng.pick(&["~", "?", "@", "N"])).as_bytes(),
                ),
                _ => out.push(0x3f + rng.below(64) as u8),
            }
        }
        if n % 4 == 3 {
            out.extend_from_slice(b"\x1b\\");
        }
        out
    });
    check_peak("sixel garbage", &sixel, 140 * MIB);

    // A kitty transmission that never finishes: one transmission is kept to 32 MiB.
    let mut first = true;
    let kitty = run(48 * scale, (100, 30, 100), true, |_| {
        let mut out = if std::mem::take(&mut first) {
            b"\x1b_Ga=T,f=32,s=4000,v=4000,i=1,m=1;".to_vec()
        } else {
            Vec::new()
        };
        out.extend(repeated(b"QUJD"));
        out
    });
    check_peak("a kitty transmission that never ends", &kitty, 40 * MIB);

    // Events nobody collects: 16,384 are kept (plus pictures' own, which are bounded apart), each
    // holding up to the string cap of 8 KiB.
    let events = run(8 * scale, (80, 24, 100), false, |n| {
        let mut out = Vec::with_capacity(MIB + 8192);
        let mut i = 0;
        while out.len() < MIB {
            i += 1;
            out.extend_from_slice(format!("\x1b]9;{n} {i} {}\x07", "x".repeat(7_000)).as_bytes());
        }
        out
    });
    check_peak("events nobody collects", &events, 140 * MIB);
    assert!(events.events <= 16_384 + 16, "{} events were kept, the cap is 16,384", events.events);

    // Replies nobody collects: 1 MiB are kept, and no more are started.
    let replies = run(12 * scale, (80, 24, 100), false, |_| {
        repeated(b"\x1b[c\x1b[6n\x1b[>q\x1b]10;?\x07\x1b[?1$p\x1bP$qm\x1b\\")
    });
    check_peak("replies nobody collects", &replies, 8 * MIB);
    assert!(
        replies.term.replies().len() <= MIB + 4096,
        "{} bytes of replies kept, the cap is 1 MiB",
        replies.term.replies().len()
    );

    // Plain output into a small history: the history is what the host asked for, to the byte.
    let lines = 1_000;
    let flood = run(16 * scale, (120, 40, lines), true, |n| {
        let mut rng = Rng::new(n as u64);
        let mut out = Vec::with_capacity(MIB + 200);
        while out.len() < MIB {
            for _ in 0..rng.range(1, 30) {
                out.extend_from_slice(b"flood \x1b[38;5;");
                out.extend_from_slice(rng.below(256).to_string().as_bytes());
                out.extend_from_slice(b"mtext\x1b[0m ");
            }
            out.extend_from_slice(b"\r\n");
        }
        out
    });
    let (held, limit) = (flood.term.grid().memory_bytes(), flood.term.grid().scrollback_limit());
    assert!(
        held <= 2 * limit + 4 * MIB,
        "plain output: {held} bytes in pages for a history limit of {limit}"
    );
    check_peak("plain output", &flood, 8 * MIB);

    // Every cell a link of its own, 1 KiB each, into a history of 1,000 lines: the links count as much again as the cells.
    let mut serial = 0u64;
    let links = run(8 * scale, (120, 40, lines), true, |_| {
        let mut out = Vec::with_capacity(MIB + 2048);
        while out.len() < MIB {
            for _ in 0..120 {
                serial += 1;
                out.extend_from_slice(
                    format!("\x1b]8;;https://example.com/{serial}/{}\x1b\\x", "u".repeat(1000)).as_bytes(),
                );
            }
            out.extend_from_slice(b"\x1b]8;;\x1b\\\r\n");
        }
        out
    });
    check_peak("a link in every cell", &links, 16 * MIB);

    // Cells with eight combining marks each, the other thing that is cheap to send and dear to keep.
    let marks = run(8 * scale, (120, 40, lines), true, |_| {
        repeated(
            &format!("{}\r\n", "a\u{301}\u{302}\u{303}\u{304}\u{305}\u{306}\u{307}\u{308}".repeat(120))
                .into_bytes(),
        )
    });
    check_peak("combining marks", &marks, 16 * MIB);

    // Reflow of a big history to other widths and back builds the new pages beside the old ones: what
    // the terminal holds afterwards is about what it held before, never a multiple of it.
    let start = live_bytes();
    let mut term = Terminal::builder().size(200, 50).scrollback(20_000).build();
    for n in 0..3_000 {
        term.feed(format!("{n} {}\r\n", "word ".repeat(150)).as_bytes());
    }
    let held = live_bytes() - start;
    for width in [100, 200, 120, 160, 200] {
        term.resize(width, 50);
        let now = live_bytes() - start;
        eprintln!("reflow to {width} columns: {} KiB held, {} KiB before", now / 1024, held / 1024);
        assert!(now <= 2 * held + 8 * MIB, "reflow to {width} columns: {now} bytes held, {held} before");
    }
}

#[test]
fn hostile_streams_leave_memory_within_the_documented_caps() {
    all_cases(4);
}

#[test]
#[ignore = "five times the default input, about 3 GB fed: cargo test -p wormhole-vt --test invariants_memory --release -- --ignored"]
fn hostile_streams_leave_memory_within_the_documented_caps_at_length() {
    all_cases(20);
}
