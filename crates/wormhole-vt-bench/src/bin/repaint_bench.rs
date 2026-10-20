//! What a repaint costs: the time to write it, its size, and the time a replica takes to parse it
//! (session-host.md, section 4 measured a naive writer; this is the real one). A terminal is
//! filled with log-like colored lines until its history holds N lines, then repainted.
//!
//!     cargo run --release -p wormhole-vt-bench --bin repaint-bench
//!     cargo run --release -p wormhole-vt-bench --bin repaint-bench -- --lines 10000 --cols 120 --runs 9
//!
//! With no `--lines` it runs the two sizes the design measured: 10,000 lines at 120 columns and
//! 200,000 lines at 200.

#![allow(clippy::print_stdout)]

use std::time::{Duration, Instant};

use wormhole_vt::Terminal;
use wormhole_vt_bench::corpus;

struct Row {
    lines: usize,
    cols: usize,
    write: Duration,
    parse: Duration,
    bytes: usize,
    history: usize,
}

fn best(times: &mut [Duration]) -> Duration {
    times.sort();
    times[0]
}

fn measure(lines: usize, cols: usize, runs: usize) -> Row {
    let rows = 40;
    let mut term = Terminal::builder().size(cols, rows).scrollback(lines).build();
    // Log-like lines (timestamps, colored levels, text) until the history holds `lines`.
    let mut seed = 1;
    while term.history_len() < lines {
        let chunk = corpus::generate("log-lines", 1 << 20, seed).expect("the log corpus");
        for piece in chunk.bytes.chunks(64 * 1024) {
            term.feed(piece);
            term.clear_replies();
            term.events().for_each(drop);
        }
        seed += 1;
    }
    let mut out = Vec::new();
    let mut writes = Vec::new();
    for _ in 0..runs {
        out.clear();
        let start = Instant::now();
        let done = term.repaint(&mut out).expect("between sequences");
        writes.push(start.elapsed());
        std::hint::black_box(done);
    }
    let mut parses = Vec::new();
    for _ in 0..runs {
        let mut replica = Terminal::builder().size(cols, rows).scrollback(lines).build();
        let start = Instant::now();
        replica.feed(&out);
        parses.push(start.elapsed());
        std::hint::black_box(&replica);
    }
    Row {
        lines,
        cols,
        write: best(&mut writes),
        parse: best(&mut parses),
        bytes: out.len(),
        history: term.history_len(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value =
        |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)?.parse::<usize>().ok());
    let runs = value("--runs").unwrap_or(5);
    let cases: Vec<(usize, usize)> = match (value("--lines"), value("--cols")) {
        (Some(lines), cols) => vec![(lines, cols.unwrap_or(120))],
        _ => vec![(10_000, 120), (200_000, 200)],
    };
    println!(
        "{:>9} {:>5} {:>10} {:>12} {:>9} {:>11} {:>10}",
        "lines", "cols", "write", "size", "B/line", "replica", "MB/s"
    );
    for (lines, cols) in cases {
        let row = measure(lines, cols, runs);
        println!(
            "{:>9} {:>5} {:>9.2}ms {:>10.2}MB {:>9.0} {:>9.2}ms {:>10.0}",
            row.lines,
            row.cols,
            row.write.as_secs_f64() * 1e3,
            row.bytes as f64 / 1e6,
            row.bytes as f64 / (row.history + 40) as f64,
            row.parse.as_secs_f64() * 1e3,
            row.bytes as f64 / 1e6 / row.parse.as_secs_f64()
        );
    }
}
