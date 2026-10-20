//! Micro-benchmarks of the engine's hot paths (divan), to see what one change did to one path.
//!
//! ```sh
//! cargo bench -p wormhole-vt                      # all of them, about a minute
//! cargo bench -p wormhole-vt -- parser            # the ones whose name has "parser"
//! cargo bench -p wormhole-vt -- --sample-count 200 feed/plain
//! ```
//!
//! Each line shows the time (fastest, slowest, median, mean), the throughput of the bytes fed, and
//! under it the allocations (`alloc` count and bytes, `dealloc`, `grow`) of one run, counted by
//! divan's allocator: a path that should not allocate shows 0 there. Inputs are generated here, from
//! a fixed seed, the same on every machine; they are made before the timed part. `vtbench` (in the
//! bench crate of the repository) measures whole corpora against other engines; these find which
//! function got slower. A debug build says nothing, and `cargo bench` builds the optimized one.

use std::hint::black_box;

use divan::counter::BytesCount;
use divan::{AllocProfiler, Bencher};
use wormhole_vt::parser::{Csi, Handler, Parser};
use wormhole_vt::{Cell, Line, RowExtras, Terminal};

#[path = "../tests/common/mod.rs"]
mod common;
use common::rng::Rng;
use common::streams::{
    ascii_run, hyperlinks, log_lines, osc_payload, scroll_region, sgr_heavy, utf8_mixed, wide_and_emoji, word,
};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

/// Bytes per generated stream: long enough to dwarf the call overhead, short enough that a run is
/// a few hundred microseconds to a few milliseconds.
const SIZE: usize = 256 * 1024;

// ---- the parser alone ----

/// Counts what the parser reports, so none of it can be optimised away; a real handler does more
/// per item, this is the parser's own cost.
#[derive(Default)]
struct Count {
    printed: usize,
    controls: usize,
    sequences: usize,
    payload: usize,
}

impl Handler for Count {
    fn print_ascii(&mut self, run: &[u8]) {
        self.printed += run.len();
    }
    fn print(&mut self, c: char) {
        self.printed += c.len_utf8();
    }
    fn execute(&mut self, _byte: u8) {
        self.controls += 1;
    }
    fn csi(&mut self, csi: &Csi) {
        self.sequences += csi.len();
    }
    fn esc(&mut self, _intermediates: &[u8], _byte: u8) {
        self.sequences += 1;
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.payload += data.len();
    }
    fn dcs_put(&mut self, data: &[u8]) {
        self.payload += data.len();
    }
    fn apc_put(&mut self, data: &[u8]) {
        self.payload += data.len();
    }
}

fn parse(bencher: Bencher, data: &[u8]) {
    bencher
        .counter(BytesCount::of_slice(data))
        .with_inputs(|| (Parser::new(), Count::default()))
        .bench_local_refs(|(parser, count)| {
            parser.advance(count, black_box(data));
            black_box(&count);
        });
}

mod parser {
    use super::*;

    #[divan::bench]
    fn ascii_run(bencher: Bencher) {
        parse(bencher, &super::ascii_run(SIZE));
    }

    #[divan::bench]
    fn utf8_mixed(bencher: Bencher) {
        parse(bencher, &super::utf8_mixed(SIZE));
    }

    #[divan::bench]
    fn sgr_heavy(bencher: Bencher) {
        parse(bencher, &super::sgr_heavy(SIZE));
    }

    #[divan::bench]
    fn osc_payload(bencher: Bencher) {
        parse(bencher, &super::osc_payload(SIZE));
    }
}

// ---- the terminal ----

fn feed(bencher: Bencher, data: &[u8], cols: usize, rows: usize) {
    bencher
        .counter(BytesCount::of_slice(data))
        .with_inputs(|| Terminal::new(cols, rows))
        .bench_local_refs(|term| term.feed(black_box(data)));
}

mod feed {
    use super::*;

    #[divan::bench]
    fn plain_text(bencher: Bencher) {
        feed(bencher, &log_lines(SIZE), 120, 40);
    }

    #[divan::bench]
    fn ascii_wrapping(bencher: Bencher) {
        feed(bencher, &ascii_run(SIZE), 80, 24);
    }

    #[divan::bench]
    fn utf8_mixed(bencher: Bencher) {
        feed(bencher, &super::utf8_mixed(SIZE), 120, 40);
    }

    #[divan::bench]
    fn sgr_heavy(bencher: Bencher) {
        feed(bencher, &super::sgr_heavy(SIZE), 120, 40);
    }

    #[divan::bench]
    fn scroll_region(bencher: Bencher) {
        feed(bencher, &super::scroll_region(SIZE), 80, 24);
    }

    /// Mode 2027: clusters are measured, so every character is looked up in the cluster tables.
    #[divan::bench]
    fn wide_and_emoji_2027(bencher: Bencher) {
        feed(bencher, &super::wide_and_emoji(SIZE / 2), 120, 40);
    }

    #[divan::bench]
    fn hyperlinks(bencher: Bencher) {
        feed(bencher, &super::hyperlinks(SIZE), 120, 40);
    }

    #[divan::bench]
    fn osc_payload(bencher: Bencher) {
        feed(bencher, &super::osc_payload(SIZE), 120, 40);
    }
}

// ---- reading and resizing ----

/// A terminal with 10,000 lines of history of long lines (so most of them are soft-wrapped).
fn full_history(cols: usize, rows: usize) -> Terminal {
    let mut rng = Rng::new(9);
    let mut term = Terminal::builder().size(cols, rows).scrollback(10_000).build();
    let mut out = Vec::new();
    for n in 0..10_000 + rows {
        for _ in 0..rng.range(2, 60) {
            out.extend_from_slice(word(&mut rng).as_bytes());
            out.push(b' ');
        }
        out.extend_from_slice(format!("{n}\r\n").as_bytes());
    }
    term.feed(&out);
    term
}

mod resize {
    use super::*;

    /// Narrower and back: every line of history is rewrapped twice.
    #[divan::bench(sample_count = 20)]
    fn reflow_full_history(bencher: Bencher) {
        bencher.with_inputs(|| full_history(120, 40)).bench_local_values(|mut term| {
            term.resize(80, 40);
            term.resize(120, 40);
            term
        });
    }

    /// Only the height changes: no rewrapping, rows move in and out of the history.
    #[divan::bench(sample_count = 20)]
    fn taller_and_shorter(bencher: Bencher) {
        bencher.with_inputs(|| full_history(120, 40)).bench_local_values(|mut term| {
            term.resize(120, 60);
            term.resize(120, 40);
            term
        });
    }
}

mod copy {
    use super::*;

    /// A screen of coloured, linked text for the readers to copy.
    fn screen() -> Terminal {
        let mut term = Terminal::new(120, 40);
        let mut data = sgr_heavy(40_000);
        data.extend(hyperlinks(40_000));
        term.feed(&data);
        term
    }

    /// The viewport row by row with `copy_row`, into buffers kept between rows (and between runs).
    #[divan::bench]
    fn row_by_row(bencher: Bencher) {
        let term = screen();
        let mut cells: Vec<Cell> = Vec::new();
        let mut extras = RowExtras::default();
        // The buffers grow on the first rows; a host keeps them, so that is not what is measured.
        for y in 0..40 {
            term.copy_row(Line(y), &mut cells, Some(&mut extras));
        }
        bencher.counter(BytesCount::new(120 * 40 * std::mem::size_of::<Cell>())).bench_local(|| {
            for y in 0..40 {
                black_box(term.copy_row(Line(y), &mut cells, Some(&mut extras)));
            }
            black_box((&cells, &extras));
        });
    }

    /// The whole viewport with `copy_viewport`, every row (what the first frame and a resize cost).
    #[divan::bench]
    fn viewport_forced(bencher: Bencher) {
        let mut term = screen();
        let mut rows = Vec::new();
        term.copy_viewport(&mut rows, true);
        bencher.bench_local(|| {
            term.copy_viewport(&mut rows, true);
            black_box(&rows);
        });
    }

    /// The viewport after one row changed: only that row is copied.
    #[divan::bench]
    fn viewport_one_row_changed(bencher: Bencher) {
        let mut term = screen();
        let mut rows = Vec::new();
        term.copy_viewport(&mut rows, true);
        bencher.bench_local(|| {
            term.feed(b"\x1b[20;1Hx");
            term.copy_viewport(&mut rows, false);
            black_box(&rows);
        });
    }
}
