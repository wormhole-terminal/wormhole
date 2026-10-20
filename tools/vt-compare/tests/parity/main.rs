//! Wormhole's terminal against alacritty on random streams: the differential harness with the
//! new engine in it.
//!
//! alacritty is the baseline, not the truth. Where it is wrong (the conformance suite lists it:
//! `known-fail: alacritty`) or where the two make different but equally valid choices, the stream
//! is changed before the comparison so that it does not use the feature (`sanitize`), or the
//! alacritty side is helped (`fixed`), or the comparison is told what not to mind (`compare`).
//! Everything else is a real difference, decided from the specs: fixed in the new engine, or
//! turned into a conformance case that says who is right.
//!
//! `cargo test --manifest-path tools/vt-compare/Cargo.toml --test parity explore -- --ignored --nocapture` lists the
//! differences in a run (`PARITY_STREAMS`, `PARITY_LENGTH`, `PARITY_SEED`, `PARITY_SHOW`,
//! `PARITY_COLS`, `PARITY_ROWS`, `PARITY_SPLIT`).

mod compare;
mod fixed;
mod sanitize;

use wormhole_vt_bench::diff::Split;
use wormhole_vt_bench::diff::random::StreamGen;
use wormhole_vt_bench::vtt::show_bytes;

use compare::differ;
use sanitize::sanitize;

fn env(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// How the new engine is fed: whole, or in pieces of `PARITY_SPLIT` bytes (0: whole).
fn split() -> Split {
    match env("PARITY_SPLIT", 0) {
        0 => Split::Whole,
        n => Split::Bytes(n),
    }
}

/// The stream cut down to what still differs, sanitized at every step (a cut that leaves a lone
/// continuation byte must not count).
fn shrink(stream: &[u8], size: (usize, usize), how: Split) -> Vec<u8> {
    let differs = |s: &[u8]| differ(&sanitize(s, size.1), size, how).is_some();
    let mut current = stream.to_vec();
    let mut chunk = current.len().div_ceil(2).max(1);
    loop {
        let mut start = 0;
        while start < current.len() {
            let end = (start + chunk).min(current.len());
            let mut candidate = current[..start].to_vec();
            candidate.extend_from_slice(&current[end..]);
            if !candidate.is_empty() && differs(&candidate) {
                current = candidate;
            } else {
                start += chunk;
            }
        }
        if chunk == 1 {
            return sanitize(&current, size.1);
        }
        chunk = chunk.div_ceil(2);
    }
}

fn run(streams: usize, length: usize, seed: u64, size: (usize, usize), how: Split, show: usize) -> usize {
    let mut generator = StreamGen::new(seed, size.0, size.1);
    let mut found: Vec<Vec<u8>> = Vec::new();
    let mut differing = 0;
    for _ in 0..streams {
        let stream = generator.stream(length);
        if differ(&sanitize(&stream, size.1), size, how).is_some() {
            differing += 1;
            if found.len() < show {
                let short = shrink(&stream, size, how);
                if !found.contains(&short) {
                    found.push(short);
                }
            }
        }
    }
    for short in &found {
        if let Some(what) = differ(short, size, how) {
            eprintln!("---- {}\n{what}", show_bytes(short));
        }
    }
    differing
}

#[test]
#[ignore = "an exploration tool: prints the differences it finds"]
fn explore() {
    let size = (env("PARITY_COLS", 20), env("PARITY_ROWS", 6));
    let streams = env("PARITY_STREAMS", 2000);
    let differing = run(
        streams,
        env("PARITY_LENGTH", 120),
        env("PARITY_SEED", 1) as u64,
        size,
        split(),
        env("PARITY_SHOW", 30),
    );
    eprintln!("{differing} of {streams} streams differ");
}

/// Streams of every length on screens of several sizes, whole and cut: no difference that the
/// sanitizer, the helper and the comparison do not explain.
#[test]
fn the_new_terminal_agrees_with_alacritty_on_random_streams() {
    for (size, length, seed) in [((20, 6), 120, 1), ((40, 12), 600, 2), ((10, 4), 60, 3), ((80, 24), 2000, 4)]
    {
        for how in [Split::Whole, Split::Bytes(1), Split::Random { seed: 5, max: 17 }] {
            let differing = run(800, length, seed, size, how, 3);
            assert_eq!(differing, 0, "{differing} streams differ at {size:?} cut as {how:?} (details above)");
        }
    }
}

/// `PARITY_STREAM='...'` (with `\e`, `\n`, `\r`): both screens, for looking at a difference.
#[test]
#[ignore = "an exploration tool"]
fn show() {
    use wormhole_vt_bench::Engine;
    let Ok(text) = std::env::var("PARITY_STREAM") else { return };
    let stream = text.replace("\\e", "\x1b").replace("\\n", "\n").replace("\\r", "\r").into_bytes();
    let size = (env("PARITY_COLS", 20), env("PARITY_ROWS", 6));
    let stream = sanitize(&stream, size.1);
    eprintln!("sanitized: {}", show_bytes(&stream));
    let mut a = fixed::FixedAlacritty::new(size.0, size.1, 50);
    let mut b = wormhole_vt_bench::engines::WormholeEngine::new(size.0, size.1, 50);
    a.feed(&stream);
    b.feed(&stream);
    for row in 0..size.1 {
        eprintln!(
            "{:>2} {:?} {} | {:?} {}",
            row + 1,
            a.row_text(row),
            a.row_wrapped(row),
            b.row_text(row),
            b.row_wrapped(row)
        );
    }
    eprintln!("cursor {:?} {:?}", a.cursor(), b.cursor());
}

/// The new terminal alone: a stream means the same wherever it is cut, UTF-8 text and the
/// sequences that hold state (strings, clusters, wrap) included. alacritty drops a character in
/// one such case; this one must not.
#[test]
fn the_new_terminal_shows_the_same_screen_however_the_bytes_are_cut() {
    use wormhole_vt_bench::corpus;
    use wormhole_vt_bench::diff::{self, Fuzz, Setup, compare};
    use wormhole_vt_bench::engines::WormholeEngine;

    for (name, split) in [
        ("one byte at a time", Split::Bytes(1)),
        ("3 bytes", Split::Bytes(3)),
        ("random pieces", Split::Random { seed: 9, max: 17 }),
    ] {
        let run = Fuzz {
            streams: 800,
            length: 1500,
            seed: 7,
            split_a: Split::Whole,
            split_b: split,
            ascii_only: false,
        };
        if let Err(mismatch) = diff::fuzz::<WormholeEngine, WormholeEngine>(&run, &Setup::new(40, 12)) {
            panic!("cut as {name}: {mismatch}");
        }
    }
    for c in corpus::all(96 * 1024, 3) {
        let setup = Setup::new(c.cols, c.rows);
        for split in [Split::Bytes(4096), Split::Random { seed: 4, max: 5000 }] {
            if let Err(mismatch) =
                compare::<WormholeEngine, WormholeEngine>(&c.bytes, &setup, Split::Whole, split)
            {
                panic!("{} cut as {split:?}: {mismatch}", c.name);
            }
        }
    }
}

/// Recorded-looking streams (the corpus generators: text floods, colors, CJK and emoji, a TUI, scroll
/// regions, links) end on the same screen in both.
#[test]
#[ignore = "explore"]
fn the_corpora_end_on_the_same_screen() {
    for c in wormhole_vt_bench::corpus::all(256 * 1024, 5) {
        match differ(&c.bytes, (c.cols, c.rows), Split::Whole) {
            None => eprintln!("{}: same", c.name),
            Some(what) => eprintln!("{}: {what}", c.name),
        }
    }
}

/// `PARITY_RAW_DIR=<dir>`: the recordings in it (`scripts/vt-record.sh` writes them) end on the
/// same screen in both, whole and cut.
#[test]
#[ignore = "explore"]
fn recordings() {
    let Ok(dir) = std::env::var("PARITY_RAW_DIR") else { return };
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "raw"))
        .collect();
    names.sort();
    for path in names {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let meta = std::fs::read_to_string(path.with_extension("meta")).unwrap_or_default();
        let field = |key: &str| {
            meta.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('=')?.parse::<usize>().ok())
        };
        let size = (field("cols").unwrap_or(100), field("rows").unwrap_or(30));
        for how in [Split::Whole, Split::Bytes(1), Split::Random { seed: 3, max: 40 }] {
            match differ(&bytes, size, how) {
                None => eprintln!("{} {how:?}: same", path.display()),
                Some(what) => eprintln!("{} {how:?}: {what}", path.display()),
            }
        }
    }
}

/// Valid-looking streams with the modes the other engine lacks (margins, 2027) and resizes in the
/// middle: the grid stays consistent, the cursor stays on the screen.
#[test]
fn the_terminal_keeps_its_grid_consistent_through_streams_and_resizes() {
    use wormhole_vt::Terminal;
    use wormhole_vt_bench::rng::Rng;

    let mut rng = Rng::new(77);
    for seed in 0..env("PARITY_STREAMS", 300) as u64 {
        let (cols, rows) = (rng.range(2, 40), rng.range(1, 14));
        let mut generator = StreamGen::new(seed, cols, rows);
        let mut term = Terminal::builder().size(cols, rows).scrollback(rng.range(0, 30)).build();
        // Modes the generator does not know, mixed in with its streams.
        term.feed(b"\x1b[?2027h\x1b[?69h\x1b[?40h");
        let stream = generator.stream(2000);
        for piece in stream.chunks(rng.range(1, 150)) {
            term.feed(piece);
            term.events().for_each(drop);
            term.clear_replies();
            if rng.one_in(8) {
                term.resize(rng.range(2, 50), rng.range(1, 20));
            }
            if rng.one_in(30) {
                term.feed(b"\x1b[?2027h\x1b[?69h\x1b[3;7s\x1b[?1049h");
            }
        }
        if let Err(why) = term.check_invariants() {
            panic!("seed {seed}: {why}");
        }
        let cursor = term.grid_cursor();
        assert!(
            (cursor.line.0 as usize) < term.screen_lines() && cursor.column.0 < term.columns(),
            "seed {seed}: cursor {cursor:?} on a {}x{} screen",
            term.columns(),
            term.screen_lines()
        );
    }
}
