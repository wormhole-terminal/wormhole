//! `profile_feed`: feeds a generated stream to a terminal over and over for a few seconds, so a
//! sampling profiler has something steady to look at. `scripts/profile-vt.sh` builds it with the
//! `profiling` profile (optimised, with line tables) and runs it under samply or Instruments;
//! by hand:
//!
//! ```sh
//! cargo run --release -p wormhole-vt --example profile_feed -- --list
//! cargo run --release -p wormhole-vt --example profile_feed -- --corpus sgr --seconds 5
//! cargo run --release -p wormhole-vt --example profile_feed -- --alloc          # no timing loop
//! ```
//!
//! Options: `--corpus NAME` (default `log`; `--list` shows them), `--seconds N` (default 10),
//! `--size KiB` of the stream (default 4096), `--chunk KiB` fed per call (default 64, the size of
//! a host's read), `--parser` to run the parser alone (with a handler that only counts), `--no-copy`
//! to leave out the reader's `copy_viewport` after each chunk (by default it runs, as a host's
//! frame would), `--alloc` to count instead of loop.
//!
//! `--alloc` prints, per phase, the allocator calls and the bytes still held after it: the parser
//! alone must show 0, a warm terminal feeding (the second pass) only what the history grows by,
//! and the reader's copy 0 once its buffers are the right size. Each call is the allocator's, from
//! a counting wrapper that is installed here for every mode.
#![allow(clippy::print_stdout)] // a command line tool: the report is the output

use std::hint::black_box;
use std::time::{Duration, Instant};

use wormhole_vt::Terminal;
use wormhole_vt::parser::{Csi, Handler, Parser};

#[path = "../tests/common/mod.rs"]
mod common;
use common::alloc::{Counting, allocations, live_bytes};
use common::streams;

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const USAGE: &str = "usage: profile_feed [--corpus NAME] [--seconds N] [--size KiB] [--chunk KiB] [--parser] [--no-copy] [--alloc] [--list]";

#[derive(Clone)]
struct Options {
    corpus: String,
    seconds: u64,
    size: usize,
    chunk: usize,
    parser: bool,
    copy: bool,
    alloc: bool,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut o = Options {
        corpus: "log".into(),
        seconds: 10,
        size: 4096 * 1024,
        chunk: 64 * 1024,
        parser: false,
        copy: true,
        alloc: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut number = |what: &str| -> Option<usize> {
            let n = args.next().and_then(|v| v.parse().ok()).filter(|&n| n > 0);
            if n.is_none() {
                eprintln!("{what} needs a number above 0");
            }
            n
        };
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            "--list" => {
                for (name, about, cols, rows) in streams::NAMES {
                    println!("{name:<8} {cols}x{rows}  {about}");
                }
                return 0;
            }
            "--corpus" => match args.next() {
                Some(name) => o.corpus = name,
                None => return 2,
            },
            "--seconds" => match number("--seconds") {
                Some(n) => o.seconds = n as u64,
                None => return 2,
            },
            "--size" => match number("--size") {
                Some(n) => o.size = n * 1024,
                None => return 2,
            },
            "--chunk" => match number("--chunk") {
                Some(n) => o.chunk = n * 1024,
                None => return 2,
            },
            "--parser" => o.parser = true,
            "--no-copy" => o.copy = false,
            "--alloc" => o.alloc = true,
            _ => {
                eprintln!("unknown argument {arg}\n{USAGE}");
                return 2;
            }
        }
    }
    let Some(&(_, _, cols, rows)) = streams::NAMES.iter().find(|(name, ..)| *name == o.corpus) else {
        eprintln!("no corpus {:?} (see --list)", o.corpus);
        return 2;
    };
    let Some(data) = streams::generate(&o.corpus, o.size) else { return 2 };
    if o.alloc {
        count_allocations(&o, &data, cols, rows);
    } else {
        spin(&o, &data, cols, rows);
    }
    0
}

/// Counts what the parser reports, so none of it is optimised away.
#[derive(Default)]
struct Count(usize);

impl Handler for Count {
    fn print_ascii(&mut self, run: &[u8]) {
        self.0 += run.len();
    }
    fn print(&mut self, c: char) {
        self.0 += c.len_utf8();
    }
    fn execute(&mut self, _byte: u8) {
        self.0 += 1;
    }
    fn csi(&mut self, csi: &Csi) {
        self.0 += csi.len();
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.0 += data.len();
    }
}

/// One pass of the stream through `term`, `chunk` bytes per call, with the reader after each.
fn pass(term: &mut Terminal, rows: &mut Vec<wormhole_vt::ViewRow>, data: &[u8], o: &Options) {
    for piece in data.chunks(o.chunk) {
        term.feed(black_box(piece));
        if o.copy {
            term.copy_viewport(rows, false);
        }
    }
    black_box(&*rows);
}

fn parse_pass(parser: &mut Parser, count: &mut Count, data: &[u8], chunk: usize) {
    for piece in data.chunks(chunk) {
        parser.advance(count, black_box(piece));
    }
}

/// The timing loop for a profiler: the same terminal takes the stream again and again (so the
/// history is full and the pages are reused, as in a long session) until the time is up.
fn spin(o: &Options, data: &[u8], cols: usize, rows: usize) {
    let mut term = Terminal::new(cols, rows);
    let mut view = Vec::new();
    let mut parser = Parser::new();
    let mut count = Count::default();
    let deadline = Duration::from_secs(o.seconds);
    let start = Instant::now();
    let mut passes = 0usize;
    while start.elapsed() < deadline {
        if o.parser {
            parse_pass(&mut parser, &mut count, data, o.chunk);
        } else {
            pass(&mut term, &mut view, data, o);
        }
        passes += 1;
    }
    black_box(&count);
    let secs = start.elapsed().as_secs_f64();
    let mb = (passes * data.len()) as f64 / 1e6;
    println!(
        "{} {}: {passes} passes of {} KiB in {secs:.1} s, {:.0} MB/s",
        if o.parser { "parser" } else { "feed" },
        o.corpus,
        data.len() / 1024,
        mb / secs
    );
}

struct Row {
    phase: &'static str,
    bytes: usize,
    secs: f64,
    calls: usize,
    held: isize,
}

fn measure(phase: &'static str, bytes: usize, mut f: impl FnMut()) -> Row {
    let (calls, held) = (allocations(), live_bytes() as isize);
    let start = Instant::now();
    f();
    let secs = start.elapsed().as_secs_f64();
    Row { phase, bytes, secs, calls: allocations() - calls, held: live_bytes() as isize - held }
}

fn count_allocations(o: &Options, data: &[u8], cols: usize, rows: usize) {
    let mut table = Vec::new();
    let mut parser = Parser::new();
    let mut count = Count::default();
    table.push(measure("parser alone", data.len(), || parse_pass(&mut parser, &mut count, data, o.chunk)));
    black_box(&count);

    let mut term = Terminal::new(cols, rows);
    let mut view = Vec::new();
    let no_copy = Options { copy: false, ..o.clone() };
    table.push(measure("feed, first pass (history fills)", data.len(), || {
        pass(&mut term, &mut view, data, &no_copy)
    }));
    table.push(measure("feed, second pass", data.len(), || pass(&mut term, &mut view, data, &no_copy)));

    // The reader: one forced copy to size the buffers, then the rows a frame copies after a chunk.
    term.copy_viewport(&mut view, true);
    let mut copies = 0;
    let mut row = measure("copy_viewport after each chunk", 0, || {
        for piece in data.chunks(o.chunk) {
            term.feed(piece);
            term.copy_viewport(&mut view, false);
            copies += 1;
        }
    });
    row.bytes = data.len();
    table.push(row);

    println!(
        "{} {cols}x{rows}, {} KiB in {} KiB pieces ({copies} copies)\n",
        o.corpus,
        data.len() / 1024,
        o.chunk / 1024
    );
    println!("{:<36} {:>9} {:>12} {:>12} {:>12}", "phase", "MB/s", "allocations", "per MB", "held after");
    for r in table {
        let mb = r.bytes as f64 / 1e6;
        println!(
            "{:<36} {:>9.0} {:>12} {:>12.1} {:>+10} B",
            r.phase,
            mb / r.secs.max(1e-9),
            r.calls,
            r.calls as f64 / mb,
            r.held
        );
    }
}
