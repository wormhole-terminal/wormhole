//! Measurements for choosing the PTY layer's sizes and for comparing builds, run by hand with
//! `--ignored --nocapture`. They are tests only so they build with the code they measure.

use std::time::{Duration, Instant};

use wormhole_vt::parser::Parser;

use super::screen::Screen;
use super::{GridSize, Pty, PtyEvent, SpawnOptions};

/// `ttystorm bench` (`tools/bench`, built with `cargo build --release`) inside a terminal of
/// this crate, end to end: the PTY layer, the host and the parser, without a window.
/// `TTYSTORM` is the binary, `ONLY` the benchmarks (default the three `scripts/perf.sh` runs),
/// `RUNS` the runs per benchmark (default 3).
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
fn ttystorm_throughput() {
    let bin = std::env::var("TTYSTORM").expect("TTYSTORM=<path to ttystorm>");
    let only = std::env::var("ONLY").unwrap_or_else(|_| "logflood,colorflood,cursorstorm".into());
    let runs = std::env::var("RUNS").unwrap_or_else(|_| "3".into());
    let size = GridSize { cols: 200, rows: 60, cell_width: 8.0, cell_height: 16.0 };
    let args = ["bench", "--plain", "--only", only.as_str(), "--size", "16", "--runs", runs.as_str()];
    let (pty, events) = Pty::spawn_program(size, &SpawnOptions::default(), &bin, &args).expect("spawns");
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        match events.try_recv() {
            Some(PtyEvent::Exited(_)) => break,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(20)),
        }
        assert!(Instant::now() < deadline, "ttystorm did not finish");
    }
    eprintln!("{}", pty.scrollback().text(200).unwrap_or_default());
}

/// Deterministic bytes like what programs print: `kind` 0 is log lines, 1 truecolor cells
/// (`colorflood`), 2 text at cursor positions (`cursorstorm`), 3 SGR-heavy text.
fn corpus(kind: usize, len: usize) -> Vec<u8> {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let words = ["deploy", "upstream", "handler", "token", "trace", "chunk", "build", "pipeline", "worker"];
    let mut out = Vec::with_capacity(len + 4096);
    while out.len() < len {
        match kind {
            0 => {
                out.extend_from_slice(b"2026-10-13T16:39:24.668Z DEBUG ");
                for _ in 0..(3 + next() % 8) {
                    out.extend_from_slice(words[(next() % 9) as usize].as_bytes());
                    out.push(b' ');
                }
                out.extend_from_slice(b"\r\n");
            }
            1 => {
                for _ in 0..200 {
                    let v = next();
                    out.extend_from_slice(
                        format!(
                            "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                            v & 255,
                            v >> 8 & 255,
                            v >> 16 & 255,
                            v >> 24 & 255,
                            v >> 32 & 255,
                            v >> 40 & 255
                        )
                        .as_bytes(),
                    );
                }
                out.extend_from_slice(b"\x1b[0m\r\n");
            }
            2 => {
                let v = next();
                out.extend_from_slice(format!("\x1b[{};{}H", 1 + v % 60, 1 + (v >> 8) % 190).as_bytes());
                out.extend_from_slice(words[(v >> 16) as usize % 9].as_bytes());
            }
            _ => {
                let v = next();
                out.extend_from_slice(
                    format!("\x1b[{};{}m", [1, 3, 4, 7, 31, 32, 33, 90][(v % 8) as usize], 40 + (v >> 8) % 8)
                        .as_bytes(),
                );
                out.extend_from_slice(words[(v >> 16) as usize % 9].as_bytes());
                if v % 7 == 0 {
                    out.extend_from_slice(b"\x1b[0m\r\n");
                }
            }
        }
    }
    out
}

/// How long the parser holds the screen's lock per slice of each size, by what is parsed: the
/// longest a snapshot can wait behind the reader (not counting scheduling). Run with
/// `--ignored --nocapture`.
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
fn slice_times() {
    let size = GridSize { cols: 200, rows: 60, cell_width: 8.0, cell_height: 16.0 };
    for (name, kind) in [("log lines", 0), ("truecolor cells", 1), ("cursor moves", 2), ("SGR churn", 3)] {
        let bytes = corpus(kind, 32 << 20);
        for slice in [1 << 10, 4 << 10, 16 << 10, 32 << 10, 64 << 10, 256 << 10] {
            let mut screen = Screen::new(size, 10_000);
            let mut parser = Parser::new();
            let mut times = Vec::with_capacity(bytes.len() / slice + 1);
            let started = Instant::now();
            for chunk in bytes.chunks(slice) {
                let at = Instant::now();
                screen.term.advance(&mut parser, chunk);
                times.push(at.elapsed());
            }
            let total = started.elapsed();
            times.sort_unstable();
            let pct = |p: usize| times[(times.len() - 1) * p / 100];
            eprintln!(
                "{name:16} slice {:>4} KiB: {:6.1} MB/s, per slice p50 {:>7.1?} p99 {:>7.1?} max {:>8.1?}",
                slice >> 10,
                bytes.len() as f64 / total.as_secs_f64() / 1e6,
                pct(50),
                pct(99),
                times[times.len() - 1],
            );
        }
    }
}

/// What the UI pays to take the screen's lock while the program floods, asking again and again
/// (a frame takes it several times): the wait per request, and how fast the flood goes meanwhile.
/// Run with `--release --ignored --nocapture`.
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
fn ui_lock_under_a_flood() {
    let line = "The quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnopqrstuvwxyz";
    let script = format!("yes '{line}' | head -c 104857600; echo; echo flood-done");
    let size = GridSize { cols: 200, rows: 60, cell_width: 8.0, cell_height: 16.0 };
    let (pty, events) =
        Pty::spawn_program(size, &SpawnOptions::default(), "/bin/sh", &["-c", script.as_str()])
            .expect("spawns");
    let mut waits = Vec::new();
    let started = Instant::now();
    loop {
        let asked = Instant::now();
        let _ = pty.mode();
        waits.push(asked.elapsed());
        if let Some(PtyEvent::Exited(_)) = events.try_recv() {
            break;
        }
        // A frame's other work between its lock requests.
        let until = Instant::now() + Duration::from_micros(20);
        while Instant::now() < until {
            std::hint::spin_loop();
        }
        assert!(started.elapsed() < Duration::from_secs(120));
    }
    let took = started.elapsed();
    waits.sort_unstable();
    let mean = waits.iter().sum::<Duration>() / waits.len() as u32;
    let pct = |p: usize| waits[(waits.len() - 1) * p / 100];
    eprintln!(
        "{} requests: wait mean {mean:?} median {:?} p99 {:?} max {:?}; the 100 MiB flood took {took:?} ({:.0} MB/s)",
        waits.len(),
        pct(50),
        pct(99),
        waits[waits.len() - 1],
        104.857_6 / took.as_secs_f64(),
    );
}
