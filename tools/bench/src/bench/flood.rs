//! `ttystorm flood`: one benchmark workload written over and over, as fast as the terminal reads,
//! so something else (a profiler, `sample`, a stopwatch on the terminal's own draw time) can watch
//! the terminal under load. Unlike `bench` it never waits for the terminal to answer.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use super::workloads;
use crate::term::{self, AltScreen, Input, Key, RawMode};

/// One pass of the stream. Big enough that the terminal never idles between passes, small enough to
/// build in a fraction of a second.
const PASS_BYTES: usize = 8 * 1024 * 1024;

pub struct FloodOptions {
    /// The workload's name in `workloads::NAMES` (`logflood`, `colorflood`, `glyphstorm`, ...).
    pub kind: String,
    pub seconds: Option<f64>,
}

/// The workloads that make sense as an endless stream (`roundtrip` sends nothing).
pub fn kinds() -> Vec<&'static str> {
    workloads::NAMES.iter().copied().filter(|n| *n != "roundtrip").collect()
}

pub fn run(opts: &FloodOptions) -> io::Result<()> {
    if !kinds().contains(&opts.kind.as_str()) {
        return Err(io::Error::other(format!(
            "no flood kind `{}` (they are: {})",
            opts.kind,
            kinds().join(", ")
        )));
    }
    let _raw = RawMode::enter()?;
    let _screen = AltScreen::enter()?;
    let mut input = Input::start();
    let size = term::size();
    // Built before the clock starts and from the same seed as `bench`, so a flood is the same bytes
    // every time (at the same window size, for the workloads that depend on it).
    let Some(payload) = workloads::payload(&opts.kind, size, PASS_BYTES) else {
        return Err(io::Error::other(format!("no workload called {}", opts.kind)));
    };
    let started = Instant::now();
    let limit = opts.seconds.map(Duration::from_secs_f64);
    let mut out = super::unbuffered_stdout()?;
    loop {
        for chunk in payload.writes() {
            // A blocking write: the pty's buffer is full whenever the terminal is behind, so this is
            // exactly "as fast as the terminal reads".
            out.write_all(chunk)?;
            if payload.chunks.is_some() {
                // Tiny writes stay tiny: one call to the kernel each, as an agent's output would be.
                out.flush()?;
            }
            if input.keys().iter().any(|k| matches!(k, Key::CtrlC | Key::Esc | Key::Char('q'))) {
                return Ok(());
            }
            if limit.is_some_and(|l| started.elapsed() >= l) {
                return Ok(());
            }
        }
        out.flush()?;
    }
}
