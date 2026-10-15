//! The differential harness, run on the engine against itself: the harness and the adapter agree
//! with themselves, and the engine shows the same screen however the bytes are cut. (Against
//! other engines it runs in `tools/vt-compare`.)

use wormhole_vt_bench::corpus;
use wormhole_vt_bench::diff::{self, Fuzz, Setup, Split, compare};
use wormhole_vt_bench::engines::WormholeEngine;
use wormhole_vt_bench::{Cell, Cursor, Engine};

const SETUP: fn() -> Setup = || Setup::new(40, 12);

#[test]
fn the_engine_agrees_with_itself_on_random_streams() {
    let run = Fuzz {
        streams: 2000,
        length: 1500,
        seed: 1,
        split_a: Split::Whole,
        split_b: Split::Whole,
        ascii_only: false,
    };
    if let Err(mismatch) = diff::fuzz::<WormholeEngine, WormholeEngine>(&run, &SETUP()) {
        panic!("{mismatch}");
    }
}

/// Parsers keep their state between `feed` calls: where a stream is cut must not matter, UTF-8
/// text included.
#[test]
fn the_engine_shows_the_same_screen_however_the_bytes_are_cut() {
    for (name, split) in [
        ("one byte at a time", Split::Bytes(1)),
        ("3 bytes", Split::Bytes(3)),
        ("random pieces", Split::Random { seed: 9, max: 17 }),
    ] {
        let run = Fuzz {
            streams: 600,
            length: 1500,
            seed: 2,
            split_a: Split::Whole,
            split_b: split,
            ascii_only: false,
        };
        if let Err(mismatch) = diff::fuzz::<WormholeEngine, WormholeEngine>(&run, &SETUP()) {
            panic!("cut as {name}: {mismatch}");
        }
    }
}

#[test]
fn the_engine_shows_the_same_screen_for_the_corpus_however_it_is_cut() {
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

/// An engine that goes wrong on one thing: it does not know reverse index.
struct NoReverseIndex(WormholeEngine);

impl Engine for NoReverseIndex {
    const NAME: &'static str = "no-ri";
    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        NoReverseIndex(WormholeEngine::new(cols, rows, scrollback))
    }
    fn feed(&mut self, bytes: &[u8]) {
        // Replace ESC M by ESC SP: a no-op escape (a space is an intermediate byte).
        let mut data = bytes.to_vec();
        for i in 1..data.len() {
            if data[i - 1] == 0x1b && data[i] == b'M' {
                data[i] = b' ';
            }
        }
        self.0.feed(&data);
    }
    fn resize(&mut self, cols: usize, rows: usize) {
        self.0.resize(cols, rows);
    }
    fn size(&self) -> (usize, usize) {
        self.0.size()
    }
    fn row_text(&self, row: usize) -> String {
        self.0.row_text(row)
    }
    fn row_wrapped(&self, row: usize) -> bool {
        self.0.row_wrapped(row)
    }
    fn cell(&self, row: usize, col: usize) -> Cell {
        self.0.cell(row, col)
    }
    fn cursor(&self) -> Cursor {
        self.0.cursor()
    }
    fn scrollback_len(&self) -> usize {
        self.0.scrollback_len()
    }
    fn scrollback_text(&self, index: usize) -> String {
        self.0.scrollback_text(index)
    }
    fn scrollback_wrapped(&self, index: usize) -> bool {
        self.0.scrollback_wrapped(index)
    }
    fn take_replies(&mut self) -> Vec<u8> {
        self.0.take_replies()
    }
    fn title(&self) -> Option<String> {
        self.0.title()
    }
    fn alt_screen(&self) -> bool {
        self.0.alt_screen()
    }
}

#[test]
fn the_harness_finds_a_difference_and_cuts_the_stream_down_to_it() {
    let run = Fuzz {
        streams: 2000,
        length: 1500,
        seed: 1,
        split_a: Split::Whole,
        split_b: Split::Whole,
        ascii_only: false,
    };
    let mismatch = diff::fuzz::<WormholeEngine, NoReverseIndex>(&run, &SETUP())
        .expect_err("an engine without reverse index must differ from one with it within 400 streams");
    assert_eq!((mismatch.engines.0, mismatch.engines.1), ("wormhole", "no-ri"));
    // The shortest stream that shows it is a few bytes: some text, then ESC M.
    assert!(mismatch.stream.len() <= 12, "{} bytes left: {mismatch}", mismatch.stream.len());
    assert!(mismatch.stream.windows(2).any(|w| w == b"\x1bM"), "{mismatch}");
    let text = mismatch.to_string();
    assert!(text.contains("wormhole and no-ri differ:") && text.contains("screen of no-ri"), "{text}");
}

#[test]
fn what_is_not_compared_does_not_count() {
    // The engines differ only in the screen's text after ESC M scrolled it; with every check
    // that sees the screen off, they agree.
    let stream = b"abc\r\ndef\x1b[1;1H\x1bMx";
    let setup = Setup::new(10, 4);
    assert!(compare::<WormholeEngine, NoReverseIndex>(stream, &setup, Split::Whole, Split::Whole).is_err());
    let mut blind = setup;
    blind.compare.text = false;
    blind.compare.cells = false;
    blind.compare.scrollback = false;
    blind.compare.cursor = false;
    assert!(compare::<WormholeEngine, NoReverseIndex>(stream, &blind, Split::Whole, Split::Whole).is_ok());
}
