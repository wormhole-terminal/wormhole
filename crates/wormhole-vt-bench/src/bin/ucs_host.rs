//! ucs-host: Wormhole's engine as the terminal behind two pipes, for measuring it headlessly.
//!
//! Bytes a program wrote arrive on stdin and go into the engine; whatever the engine answers
//! (cursor position reports, mode queries, device attributes) is written to stdout. Nothing is
//! drawn and no window opens. `tools/vt-compare/ucs-headless.py` connects ucs-detect to it through
//! a pseudo-terminal:
//!
//! ```sh
//! tools/vt-compare/ucs-headless.py            # ucs-detect's default run against the engine
//! ```
//!
//! `ucs-host <columns> <rows> [on|off]`: the last argument sets mode 2027 before the first byte
//! (the engine's own default when left out).
use std::io::{Read, Write};
use std::process::ExitCode;

use wormhole_vt_bench::Engine;
use wormhole_vt_bench::engines::WormholeEngine;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cols = args.next().and_then(|a| a.parse().ok()).unwrap_or(80);
    let rows = args.next().and_then(|a| a.parse().ok()).unwrap_or(25);
    let mut engine = WormholeEngine::new(cols, rows, 1000);
    engine.allow_screen_checksum();
    match args.next().as_deref() {
        Some("on") => engine.feed(b"\x1b[?2027h"),
        Some("off") => engine.feed(b"\x1b[?2027l"),
        _ => {}
    }
    let (mut input, mut output) = (std::io::stdin().lock(), std::io::stdout().lock());
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = match input.read(&mut buf) {
            Ok(0) | Err(_) => return ExitCode::SUCCESS,
            Ok(n) => n,
        };
        engine.feed(&buf[..n]);
        let replies = engine.take_replies();
        if !replies.is_empty() && (output.write_all(&replies).is_err() || output.flush().is_err()) {
            return ExitCode::FAILURE;
        }
    }
}
