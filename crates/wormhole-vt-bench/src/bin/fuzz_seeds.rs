//! fuzz-seeds: the starting inputs for the `cargo fuzz` targets of `wormhole-vt`, made from what the
//! repository already has: every `.vtt` case's input and the generated corpora (and recordings,
//! when `tools/vt-corpus` has them).
//!
//! ```sh
//! cargo run -p wormhole-vt-bench --bin fuzz-seeds -- crates/wormhole-vt/fuzz/corpus
//! ```
//!
//! Writes one file per seed into `<dir>/<target>/`, named by a hash of its content (so running it
//! again changes nothing). `scripts/fuzz.sh seeds` runs this. The formats are those of
//! `crates/wormhole-vt/fuzz/src/lib.rs` (the terminal script) and the targets' headers.
// A command line tool: its output is the product.
#![allow(clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wormhole_vt_bench::corpus::{self, fnv1a, recorded};
use wormhole_vt_bench::vtt::{self, StepKind};

/// Seeds longer than this are cut: the fuzzer's `-max_len` is the same, and long inputs mutate badly.
const MAX_SEED: usize = 3000;

/// A terminal script's byte for a literal 0xFF: doubled.
fn escaped(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().flat_map(|&b| if b == 0xFF { vec![0xFF, 0xFF] } else { vec![b] }).collect()
}

/// The header `fuzz::Run::new` reads: columns, rows and the flag byte.
fn script_header(cols: usize, rows: usize, history: usize, checksum: bool) -> Vec<u8> {
    let history = match history {
        0 => 0,
        1..=10 => 1,
        11..=500 => 2,
        _ => 3,
    };
    vec![(cols.max(1) - 1) as u8 % 120, (rows.max(1) - 1) as u8 % 40, history | (u8::from(checksum) << 2) | 8]
}

/// Where the next string (APC, DCS, or OSC 1337) in `bytes` starts and ends, ST or BEL included.
fn next_string(bytes: &[u8], from: usize) -> Option<(usize, usize, u8)> {
    let mut at = from;
    while at + 2 < bytes.len() {
        if bytes[at] == 0x1B {
            let kind = match bytes[at + 1] {
                b'_' => Some(0),
                b'P' => Some(2),
                b']' if bytes[at + 2..].starts_with(b"1337;") => Some(4),
                _ => None,
            };
            if let Some(kind) = kind {
                let end = (at + 2..bytes.len())
                    .find_map(|i| match bytes[i] {
                        0x07 => Some(i + 1),
                        0x1B if bytes.get(i + 1) == Some(&b'\\') => Some(i + 2),
                        _ => None,
                    })
                    .unwrap_or(bytes.len());
                return Some((at, end, kind));
            }
        }
        at += 1;
    }
    None
}

/// The `graphics` target's frames for a stream: each picture command with its framing taken off
/// (the target puts it back, and the fuzzer can then vary what is inside), the rest as raw text.
fn frames(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some((start, end, kind)) = next_string(bytes, from) {
        if start > from {
            out.extend([0xFF, 6]);
            out.extend(escaped(&bytes[from..start]));
        }
        let skip = if kind == 4 { 7 } else { 2 };
        let tail = if bytes[end - 1] == 0x07 {
            1
        } else if bytes[..end].ends_with(b"\x1b\\") {
            2
        } else {
            0
        };
        let mut body = &bytes[(start + skip).min(end)..end.saturating_sub(tail).max((start + skip).min(end))];
        if kind == 0 {
            body = body.strip_prefix(b"G").unwrap_or(body);
        }
        out.extend([0xFF, kind]);
        out.extend(escaped(body));
        from = end;
    }
    if from < bytes.len() {
        out.extend([0xFF, 6]);
        out.extend(escaped(&bytes[from..]));
    }
    out
}

struct Seeds {
    dir: PathBuf,
    count: usize,
}

impl Seeds {
    fn add(&mut self, target: &str, mut bytes: Vec<u8>) {
        bytes.truncate(MAX_SEED);
        let dir = self.dir.join(target);
        let name = format!("{:016x}", fnv1a(&bytes));
        if std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join(name), &bytes)).is_ok() {
            self.count += 1;
        }
    }

    /// One stream for every target.
    fn stream(&mut self, cols: usize, rows: usize, history: usize, checksum: bool, bytes: &[u8]) {
        let mut term = script_header(cols, rows, history, checksum);
        term.extend(escaped(bytes));
        self.add("term", term);
        let mut parser = vec![0];
        parser.extend(bytes);
        self.add("parser", parser);
        let mut split = vec![(cols.max(1) - 1) as u8 % 60, (rows.max(1) - 1) as u8 % 20, 2, 3, 40, 90, 130];
        split.extend(bytes);
        self.add("split", split);
        if bytes.windows(2).any(|w| w == b"\x1b_" || w == b"\x1bP" || w == b"\x1b]") {
            let mut graphics = script_header(cols, rows, history, checksum);
            graphics.extend(frames(bytes));
            self.add("graphics", graphics);
        }
    }
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: fuzz-seeds <corpus dir>");
        return ExitCode::FAILURE;
    };
    let conformance = Path::new(env!("CARGO_MANIFEST_DIR")).join("../wormhole-vt/tests/conformance");
    let cases = match vtt::load_dir(&conformance) {
        Ok(cases) => cases,
        Err(errors) => {
            eprintln!("the conformance files do not parse:\n{}", errors.join("\n"));
            return ExitCode::FAILURE;
        }
    };
    let mut seeds = Seeds { dir: PathBuf::from(dir), count: 0 };
    for case in &cases {
        let mut script =
            script_header(case.cols, case.rows, case.scrollback_limit, case.allow_screen_checksum);
        let mut all = Vec::new();
        for step in &case.steps {
            match &step.kind {
                StepKind::Feed(bytes) => {
                    script.extend(escaped(bytes));
                    all.extend(bytes);
                }
                StepKind::Resize { cols, rows } => {
                    script.extend([0xFF, 0, (*cols).max(1) as u8 - 1, (*rows).max(1) as u8 - 1])
                }
                _ => {}
            }
        }
        seeds.add("term", script);
        seeds.stream(case.cols, case.rows, case.scrollback_limit, case.allow_screen_checksum, &all);
    }
    for corpus in corpus::all(MAX_SEED * 4, 1) {
        // A few windows of each: the start, and later parts that begin mid-state.
        for part in corpus.bytes.chunks(MAX_SEED).take(4) {
            seeds.stream(corpus.cols, corpus.rows, 1000, false, part);
        }
    }
    let recordings = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vt-corpus");
    for corpus in recorded(&recordings, None) {
        for part in corpus.bytes.chunks(MAX_SEED).take(6) {
            seeds.stream(corpus.cols, corpus.rows, 1000, false, part);
        }
    }
    println!("{} seeds in {}", seeds.count, seeds.dir.display());
    ExitCode::SUCCESS
}
