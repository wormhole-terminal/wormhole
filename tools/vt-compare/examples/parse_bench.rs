//! Parser throughput: `wormhole_vt::parser` against alacritty's `vte` on deterministic inputs.
//!
//! ```sh
//! cargo run --release --manifest-path tools/vt-compare/Cargo.toml --example parse_bench
//! cargo run --release --manifest-path tools/vt-compare/Cargo.toml --example parse_bench -- ascii sgr   # only these inputs
//! ```
//!
//! Each input is fed in 64 KiB chunks (what a PTY read looks like) to a fresh parser; the best of
//! several runs counts. Two kinds of handler: *counting* does the least work that cannot be
//! optimised away (it adds up cells, events and one parameter per sequence), which is what the
//! table is about; *no-op* is the empty handler, where a compiler may delete everything a
//! per-character callback would have done, so the vte column of it flatters vte on text.
//!
//! The vte side is used through its public API only.
#![allow(clippy::print_stdout)] // a command line tool: the table is the output

use alacritty_terminal::vte;
use std::hint::black_box;
use std::time::Instant;
use wormhole_vt::parser::{Csi, Handler, Parser};

const SIZE: usize = 4 << 20;
const CHUNK: usize = 64 << 10;
const RUNS: usize = 25;

/// `PARSE_BENCH_RUNS=5` for a quick look.
fn runs() -> usize {
    std::env::var("PARSE_BENCH_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(RUNS)
}

// ---- the handlers ----

#[derive(Default)]
struct OursCount {
    cells: u64,
    events: u64,
    sum: u64,
}

impl Handler for OursCount {
    fn print_ascii(&mut self, run: &[u8]) {
        self.cells += run.len() as u64;
    }
    fn print(&mut self, c: char) {
        self.cells += 1 + (c as u64 >> 20);
    }
    fn execute(&mut self, byte: u8) {
        self.events += 1;
        self.sum += byte as u64;
    }
    fn csi(&mut self, csi: &Csi) {
        self.events += 1;
        self.sum += csi.param(0, 1) as u64 + csi.final_byte() as u64;
    }
    fn esc(&mut self, _: &[u8], byte: u8) {
        self.events += 1;
        self.sum += byte as u64;
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.sum += data.len() as u64;
    }
    fn osc_end(&mut self, bel: bool) {
        self.events += 1;
        self.sum += bel as u64;
    }
    fn dcs_put(&mut self, data: &[u8]) {
        self.sum += data.len() as u64;
    }
    fn apc_put(&mut self, data: &[u8]) {
        self.sum += data.len() as u64;
    }
}

#[derive(Default)]
struct VteCount {
    cells: u64,
    events: u64,
    sum: u64,
}

impl vte::Perform for VteCount {
    fn put(&mut self, _: u8) {
        self.sum += 1;
    }
    fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn unhook(&mut self) {}
    fn print(&mut self, c: char) {
        self.cells += 1 + (c as u64 >> 20);
    }
    fn execute(&mut self, byte: u8) {
        self.events += 1;
        self.sum += byte as u64;
    }
    fn csi_dispatch(&mut self, params: &vte::Params, _: &[u8], _: bool, action: char) {
        self.events += 1;
        let first = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(0);
        self.sum += if first == 0 { 1 } else { first } as u64 + action as u64;
    }
    fn esc_dispatch(&mut self, _: &[u8], _: bool, byte: u8) {
        self.events += 1;
        self.sum += byte as u64;
    }
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        self.events += 1;
        self.sum += params.iter().map(|p| p.len() as u64 + 1).sum::<u64>() + bell_terminated as u64;
    }
}

struct OursNoop;
impl Handler for OursNoop {}

struct VteNoop;
impl vte::Perform for VteNoop {
    fn print(&mut self, _: char) {}
    fn execute(&mut self, _: u8) {}
    fn csi_dispatch(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
}

// ---- the inputs ----

#[path = "../../../crates/wormhole-vt/tests/common/rng.rs"]
mod rng;
use rng::Rng;

fn words(rng: &mut Rng, out: &mut Vec<u8>, n: usize) {
    let target = out.len() + n;
    while out.len() < target {
        for _ in 0..rng.range(2, 9) {
            out.push(b'a' + rng.below(26) as u8);
        }
        out.push(b' ');
    }
    out.truncate(target);
}

fn build(size: usize, mut piece: impl FnMut(&mut Rng, &mut Vec<u8>)) -> Vec<u8> {
    let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);
    let mut out = Vec::with_capacity(size + 256);
    while out.len() < size {
        piece(&mut rng, &mut out);
    }
    out
}

fn ascii() -> Vec<u8> {
    build(SIZE, |rng, out| words(rng, out, 4000))
}

fn lines() -> Vec<u8> {
    build(SIZE, |rng, out| {
        let n = rng.range(10, 110);
        words(rng, out, n);
        out.extend_from_slice(b"\r\n");
    })
}

fn sgr() -> Vec<u8> {
    build(SIZE, |rng, out| {
        match rng.below(6) {
            0 => out.extend_from_slice(b"\x1b[0m"),
            1 => out.extend_from_slice(b"\x1b[1m"),
            2 => out.extend_from_slice(
                format!("\x1b[48;2;{};{};{}m", rng.below(256), rng.below(256), rng.below(256)).as_bytes(),
            ),
            3 => out.extend_from_slice(format!("\x1b[38;5;{}m", rng.below(256)).as_bytes()),
            _ => out.extend_from_slice(
                format!("\x1b[38;2;{};{};{}m", rng.below(256), rng.below(256), rng.below(256)).as_bytes(),
            ),
        }
        let n = rng.range(1, 8);
        words(rng, out, n);
    })
}

fn sgr_fixed() -> Vec<u8> {
    build(SIZE, |rng, out| {
        out.extend_from_slice(
            format!("\x1b[38;2;{:03};{:03};{:03}m", rng.below(256), rng.below(256), rng.below(256))
                .as_bytes(),
        );
        out.extend_from_slice(b"abcd ");
    })
}

/// Inline images: iTerm2's one big OSC, and the kitty protocol's APC chunks of 4 KiB.
fn images() -> Vec<u8> {
    const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    build(SIZE, |rng, out| {
        out.extend_from_slice(b"\x1b]1337;File=inline=1;size=262144:");
        for _ in 0..(256 << 10) {
            out.push(BASE64[rng.below(64)]);
        }
        out.push(0x07);
        for chunk in 0..64 {
            let more = if chunk == 63 { 0 } else { 1 };
            out.extend_from_slice(format!("\x1b_Ga=T,f=100,m={more};").as_bytes());
            for _ in 0..4096 {
                out.push(BASE64[rng.below(64)]);
            }
            out.extend_from_slice(b"\x1b\\");
        }
        out.extend_from_slice(b"\r\n");
    })
}

fn cjk() -> Vec<u8> {
    build(SIZE, |rng, out| {
        for _ in 0..rng.range(5, 60) {
            let c = match rng.below(10) {
                0 => rng.range(0x1F300, 0x1F64F) as u32,
                1 => rng.range(0xAC00, 0xD7A3) as u32,
                2 => ' ' as u32,
                _ => rng.range(0x4E00, 0x9FFF) as u32,
            };
            if let Some(c) = char::from_u32(c) {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
        out.extend_from_slice(b"\r\n");
    })
}

fn cursor() -> Vec<u8> {
    build(SIZE, |rng, out| {
        match rng.below(8) {
            0 => out.extend_from_slice(b"\x1b[H"),
            1 => {
                out.extend_from_slice(format!("\x1b[{};{}H", rng.range(1, 50), rng.range(1, 200)).as_bytes())
            }
            2 => out.extend_from_slice(
                format!("\x1b[{}{}", rng.range(1, 20), ["A", "B", "C", "D"][rng.below(4)]).as_bytes(),
            ),
            3 => out.extend_from_slice(b"\x1b[K"),
            4 => out.extend_from_slice(b"\x1b[2K"),
            5 => out.extend_from_slice(b"\x1b[?25l\x1b[?25h"),
            6 => {
                out.extend_from_slice(format!("\x1b[{};{}r", rng.range(1, 10), rng.range(20, 60)).as_bytes())
            }
            _ => out.extend_from_slice(b"\x1b[1;1H\x1b[J"),
        }
        let n = rng.range(1, 6);
        words(rng, out, n);
    })
}

fn osc8() -> Vec<u8> {
    build(SIZE, |rng, out| {
        let id = rng.below(100_000);
        let end: &[u8] = if rng.below(4) == 0 { b"\x07" } else { b"\x1b\\" };
        out.extend_from_slice(format!("\x1b]8;id={id};https://example.com/some/path/{id}?q=1").as_bytes());
        out.extend_from_slice(end);
        let n = rng.range(3, 24);
        words(rng, out, n);
        out.extend_from_slice(b"\x1b]8;;");
        out.extend_from_slice(end);
        if rng.below(8) == 0 {
            out.extend_from_slice(format!("\x1b]0;title {id}\x07").as_bytes());
        }
        out.push(b' ');
    })
}

// ---- measuring ----

/// The best time of `runs()` passes over `data`, in MB/s.
fn best(data: &[u8], mut pass: impl FnMut(&[u8]) -> u64) -> f64 {
    let mut fastest = f64::MAX;
    for _ in 0..runs() {
        let start = Instant::now();
        black_box(pass(black_box(data)));
        fastest = fastest.min(start.elapsed().as_secs_f64());
    }
    data.len() as f64 / fastest / 1e6
}

fn ours_cells(data: &[u8]) -> u64 {
    let mut parser = Parser::new();
    let mut h = OursCount::default();
    for chunk in data.chunks(CHUNK) {
        parser.advance(&mut h, chunk);
    }
    black_box(h.events ^ h.sum);
    h.cells
}

fn vte_cells(data: &[u8]) -> u64 {
    let mut parser = vte::Parser::new();
    let mut h = VteCount::default();
    for chunk in data.chunks(CHUNK) {
        parser.advance(&mut h, chunk);
    }
    black_box(h.events ^ h.sum);
    h.cells
}

fn ours_noop(data: &[u8]) -> u64 {
    let mut parser = Parser::new();
    let mut h = OursNoop;
    for chunk in data.chunks(CHUNK) {
        parser.advance(&mut h, chunk);
    }
    parser.is_ground() as u64
}

fn vte_noop(data: &[u8]) -> u64 {
    let mut parser = vte::Parser::new();
    let mut h = VteNoop;
    for chunk in data.chunks(CHUNK) {
        parser.advance(&mut h, chunk);
    }
    0
}

fn main() {
    let only: Vec<String> = std::env::args().skip(1).collect();
    type Input = (&'static str, fn() -> Vec<u8>);
    let inputs: [Input; 8] = [
        ("ascii", ascii),
        ("lines", lines),
        ("sgr", sgr),
        ("sgrfix", sgr_fixed),
        ("cjk", cjk),
        ("cursor", cursor),
        ("osc8", osc8),
        ("image", images),
    ];
    println!("{:<8} {:>9}   {:>20}   {:>20}", "", "", "counting handler", "no-op handler");
    println!(
        "{:<8} {:>9}   {:>8} {:>8} {:>6}   {:>8} {:>8} {:>6}",
        "input", "MB", "ours", "vte", "x", "ours", "vte", "x"
    );
    for (name, make) in inputs {
        if !only.is_empty() && !only.iter().any(|o| o == name) {
            continue;
        }
        let data = make();
        // The two must have read the same text.
        assert_eq!(ours_cells(&data), vte_cells(&data), "{name}: the parsers disagree on the text");
        let (oc, vc) = (best(&data, ours_cells), best(&data, vte_cells));
        let (on, vn) = (best(&data, ours_noop), best(&data, vte_noop));
        println!(
            "{:<8} {:>9.1}   {:>8.0} {:>8.0} {:>5.2}x   {:>8.0} {:>8.0} {:>5.2}x",
            name,
            data.len() as f64 / 1e6,
            oc,
            vc,
            oc / vc,
            on,
            vn,
            on / vn,
        );
    }
}
