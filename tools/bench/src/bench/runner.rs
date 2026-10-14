//! Runs the tests against the terminal on the other end of the pty.
//!
//! A test writes its bytes in 64 KiB chunks (or the sizes its payload names), then asks where the
//! cursor is. The terminal answers only after it has parsed everything written before the question,
//! so the time to the answer is the time the terminal took to consume the whole payload, not the
//! time the kernel took to buffer it.
//!
//! Every write is timed too: a write that blocks is the terminal not reading, and the wait for the
//! answer after the last write is the terminal still parsing (see `diag`).

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};

use super::compose;
use super::diag::{self, RunDiag, TestDiag, Timed, WriteEvent};
use super::procinfo::{self, Terminal};
use super::results::{TestResult, Unit};
use super::stats;
use super::workloads::{self, Payload};
use crate::term::{Input, Key, Size};

/// A run that takes longer than this is reported as a timeout.
const RUN_TIMEOUT: Duration = Duration::from_secs(60);
/// How long a terminal gets to catch up after a timeout before we give up on it.
const RECOVER: Duration = Duration::from_secs(30);
const WARMUP_BYTES: usize = 256 * 1024;
const LATENCY_SAMPLES: usize = 1000;
const LATENCY_TIMEOUT: Duration = Duration::from_secs(5);

/// Back to a known screen: SGR, scroll region, origin mode, wrap, synchronized output, cleared.
const RESET: &[u8] = b"\x1b[0m\x1b[r\x1b[?6l\x1b[?7h\x1b[?2026l\x1b[?25l\x1b[2J\x1b[H";

pub struct Runner<'a, W: Write> {
    pub input: &'a mut Input,
    pub out: W,
    pub size: Size,
    pub bytes: usize,
    pub runs: usize,
    /// No status line on top of the workload (`--plain`).
    pub plain: bool,
    /// The terminal's process, whose CPU time and memory are read around each run.
    pub process: Option<Terminal>,
    /// Every write of every measured run, as JSON Lines (`--trace`).
    pub trace: Option<BufWriter<File>>,
}

pub enum Step {
    Done(Box<TestResult>),
    /// A key asked to stop.
    Interrupted,
}

impl<W: Write> Runner<'_, W> {
    fn reset(&mut self) -> io::Result<bool> {
        self.out.write_all(RESET)?;
        Ok(self.input.sync(&mut self.out, Duration::from_secs(10))?.is_some())
    }

    /// A line along the top of the screen; the next reset wipes it.
    fn status(&mut self, text: &str) -> io::Result<()> {
        if self.plain {
            return Ok(());
        }
        write!(self.out, "\x1b[1;1H\x1b[0m\x1b[2K\x1b[38;2;130;180;255m ttystorm \x1b[0m{text}")?;
        self.out.flush()
    }

    fn interrupted(&mut self) -> bool {
        self.input.keys().iter().any(|k| matches!(k, Key::CtrlC | Key::Esc | Key::Char('q')))
    }

    pub fn run_test(&mut self, name: &str, index: usize, total: usize) -> io::Result<Step> {
        let head = format!("{name} ({}/{total})", index + 1);
        self.reset()?;
        self.status(&format!("{head}  generating…"))?;
        if name == "roundtrip" {
            return self.latency(&head);
        }
        let Some(main) = workloads::payload(name, self.size, self.bytes) else {
            return Err(io::Error::other(format!("no test called {name}")));
        };
        let warm = workloads::payload(name, self.size, WARMUP_BYTES).unwrap_or_default();

        // Warm up: fonts, glyph caches and the pty buffers are cold on the first bytes.
        self.reset()?;
        if time_payload(self.input, &mut self.out, &warm, RUN_TIMEOUT)?.is_none() {
            return Ok(Step::Done(Box::new(timeout_result(name, &main))));
        }

        let composition = compose::analyze(&main.bytes, main.write_count());
        let mut rates = Vec::new();
        let mut times = Vec::new();
        let mut runs = Vec::new();
        for run in 0..self.runs {
            if self.interrupted() {
                return Ok(Step::Interrupted);
            }
            if !self.reset()? {
                return Ok(Step::Done(Box::new(timeout_result(name, &main))));
            }
            let before = self.process.as_ref().and_then(|p| procinfo::sample(p.pid));
            let Some(timed) = time_payload(self.input, &mut self.out, &main, RUN_TIMEOUT)? else {
                return Ok(Step::Done(Box::new(timeout_result(name, &main))));
            };
            let after = self.process.as_ref().and_then(|p| procinfo::sample(p.pid));
            if let Some(trace) = &mut self.trace {
                diag::write_trace(trace, name, run + 1, &timed)?;
            }
            let rate = rate(name, &main, timed.secs);
            rates.push(rate);
            times.push(timed.secs);
            runs.push(RunDiag::from_timed(&timed, before, after));
            let unit = unit_of(name);
            self.reset()?;
            self.status(&format!("{head}  run {}/{}  {:.1} {}", run + 1, self.runs, rate, unit.label()))?;
        }
        let diag = TestDiag { composition, runs, median_run: nearest(&rates, stats::median(&rates)) };
        Ok(Step::Done(Box::new(throughput_result(name, &main, &rates, &times, Some(diag)))))
    }

    fn latency(&mut self, head: &str) -> io::Result<Step> {
        self.reset()?;
        for _ in 0..100 {
            if self.input.sync(&mut self.out, LATENCY_TIMEOUT)?.is_none() {
                return Ok(Step::Done(Box::new(latency_timeout())));
            }
        }
        let mut micros = Vec::with_capacity(LATENCY_SAMPLES);
        for i in 0..LATENCY_SAMPLES {
            match self.input.sync(&mut self.out, LATENCY_TIMEOUT)? {
                Some(d) => micros.push(d.as_secs_f64() * 1e6),
                None => return Ok(Step::Done(Box::new(latency_timeout()))),
            }
            if i % 250 == 249 && self.interrupted() {
                return Ok(Step::Interrupted);
            }
        }
        let _ = head;
        Ok(Step::Done(Box::new(latency_result(&micros))))
    }
}

/// Writes `payload` and returns how long until the terminal had parsed all of it, and the timing of
/// every write, or `None` when it took longer than `timeout`.
///
/// Each write is flushed so it is one call to the kernel: that is what is timed, and on a PTY a
/// write only returns when the terminal has read enough to make room.
pub fn time_payload(
    input: &mut Input,
    out: &mut impl Write,
    payload: &Payload,
    timeout: Duration,
) -> io::Result<Option<Timed>> {
    let mut events = Vec::with_capacity(payload.write_count());
    let start = Instant::now();
    let deadline = start + timeout;
    let mut last = start;
    for chunk in payload.writes() {
        out.write_all(chunk)?;
        out.flush()?;
        let now = Instant::now();
        events.push(WriteEvent {
            end_ns: (now - start).as_nanos() as u64,
            took_ns: (now - last).as_nanos() as u64,
            bytes: chunk.len(),
        });
        last = now;
        if now > deadline {
            // Let it drain, so the next test doesn't start inside this one's backlog.
            let _ = input.sync(out, RECOVER)?;
            return Ok(None);
        }
    }
    let written = Instant::now();
    let left = deadline.saturating_duration_since(written).max(Duration::from_secs(1));
    Ok(input.sync(out, left)?.map(|_| {
        let done = Instant::now();
        Timed {
            secs: (done - start).as_secs_f64(),
            dsr_wait_s: (done - written).as_secs_f64(),
            writes: events,
        }
    }))
}

fn unit_of(name: &str) -> Unit {
    match name {
        "framerate" | "agentredraw" | "agentscreen" => Unit::Fps,
        "screenswap" => Unit::PerSec,
        _ => Unit::MBps,
    }
}

/// The run's rate in the test's unit: megabytes per second, or screens per second.
fn rate(name: &str, p: &Payload, secs: f64) -> f64 {
    let secs = secs.max(1e-9);
    if unit_of(name) == Unit::MBps { p.bytes.len() as f64 / 1e6 / secs } else { p.frames as f64 / secs }
}

/// The index of the value closest to `target` (the first on a tie).
fn nearest(values: &[f64], target: f64) -> usize {
    let mut best = (0, f64::INFINITY);
    for (i, v) in values.iter().enumerate() {
        if (v - target).abs() < best.1 {
            best = (i, (v - target).abs());
        }
    }
    best.0
}

fn throughput_result(
    name: &str,
    p: &Payload,
    rates: &[f64],
    times: &[f64],
    diag: Option<TestDiag>,
) -> TestResult {
    let unit = unit_of(name);
    let median = stats::median(rates);
    let (min, max) = stats::min_max(rates);
    let secs = stats::median(times);
    let mut extra = Vec::new();
    if unit != Unit::MBps {
        extra.push(("mbps".to_string(), p.bytes.len() as f64 / 1e6 / secs.max(1e-9)));
        extra.push((if unit == Unit::Fps { "frames" } else { "swaps" }.to_string(), p.frames as f64));
    }
    if p.chunks.is_some() {
        extra.push(("writes_per_s".to_string(), p.write_count() as f64 / secs.max(1e-9)));
        extra.push(("writes".to_string(), p.write_count() as f64));
    }
    TestResult {
        name: name.to_string(),
        unit,
        median,
        min,
        max,
        secs,
        bytes: p.bytes.len() as u64,
        timeout: false,
        extra,
        diag,
    }
}

fn timeout_result(name: &str, p: &Payload) -> TestResult {
    TestResult {
        name: name.to_string(),
        unit: unit_of(name),
        median: 0.0,
        min: 0.0,
        max: 0.0,
        secs: RUN_TIMEOUT.as_secs_f64(),
        bytes: p.bytes.len() as u64,
        timeout: true,
        extra: Vec::new(),
        diag: None,
    }
}

fn latency_timeout() -> TestResult {
    TestResult {
        name: "roundtrip".to_string(),
        unit: Unit::Micros,
        median: 0.0,
        min: 0.0,
        max: 0.0,
        secs: 0.0,
        bytes: 0,
        timeout: true,
        extra: Vec::new(),
        diag: None,
    }
}

fn latency_result(micros: &[f64]) -> TestResult {
    let mut sorted = micros.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let (min, max) = stats::min_max(&sorted);
    TestResult {
        name: "roundtrip".to_string(),
        unit: Unit::Micros,
        median: stats::percentile(&sorted, 50.0),
        min,
        max,
        secs: 0.0,
        bytes: 0,
        timeout: false,
        extra: vec![
            ("p90".to_string(), stats::percentile(&sorted, 90.0)),
            ("p99".to_string(), stats::percentile(&sorted, 99.0)),
            ("samples".to_string(), sorted.len() as f64),
        ],
        diag: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_reports_percentiles_of_the_samples() {
        let samples: Vec<f64> = (1..=100).map(f64::from).collect();
        let r = latency_result(&samples);
        assert_eq!(r.unit, Unit::Micros);
        assert!((r.median - 50.5).abs() < 1e-9);
        assert!((r.extra("p99").unwrap_or(0.0) - 99.01).abs() < 1e-9);
        assert_eq!((r.min, r.max), (1.0, 100.0));
        assert!(!r.timeout);
    }

    #[test]
    fn throughput_uses_the_median_run() {
        let p = Payload { bytes: vec![0; 10_000_000], ..Payload::default() };
        let rates: Vec<f64> = [0.5, 0.25, 1.0].iter().map(|s| rate("logflood", &p, *s)).collect();
        let r = throughput_result("logflood", &p, &rates, &[0.5, 0.25, 1.0], None);
        assert!((r.median - 20.0).abs() < 1e-9);
        assert_eq!((r.min, r.max), (10.0, 40.0));
        assert_eq!(r.secs, 0.5);
    }

    #[test]
    fn frames_count_frames_per_second() {
        let p = Payload { bytes: vec![0; 4_000_000], frames: 60, chunks: None };
        assert!((rate("framerate", &p, 0.5) - 120.0).abs() < 1e-9);
        let r = throughput_result("framerate", &p, &[120.0], &[0.5], None);
        assert_eq!(r.unit, Unit::Fps);
        assert!((r.extra("mbps").unwrap_or(0.0) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn swaps_count_per_second_and_tiny_writes_count_writes() {
        let p = Payload { bytes: vec![0; 1_000_000], frames: 100, chunks: None };
        assert!((rate("screenswap", &p, 0.5) - 200.0).abs() < 1e-9);
        let r = throughput_result("screenswap", &p, &[200.0], &[0.5], None);
        assert_eq!((r.unit, r.extra("swaps")), (Unit::PerSec, Some(100.0)));
        let p = Payload { bytes: vec![0; 1000], frames: 0, chunks: Some(vec![10; 100]) };
        let r = throughput_result("agentstream", &p, &[0.002], &[0.5], None);
        assert_eq!(r.unit, Unit::MBps);
        assert_eq!((r.extra("writes"), r.extra("writes_per_s")), (Some(100.0), Some(200.0)));
    }

    #[test]
    fn the_median_run_is_the_one_nearest_the_median() {
        assert_eq!(nearest(&[10.0, 30.0, 20.0], 20.0), 2);
        assert_eq!(nearest(&[], 1.0), 0);
    }

    #[test]
    fn writes_follow_the_payload_sizes() {
        let p = Payload { bytes: (0..10).collect(), frames: 0, chunks: Some(vec![3, 3, 4]) };
        let w: Vec<&[u8]> = p.writes().collect();
        assert_eq!(w.iter().map(|c| c.len()).collect::<Vec<_>>(), vec![3, 3, 4]);
        let big = Payload { bytes: vec![0; workloads::CHUNK * 2 + 5], ..Payload::default() };
        assert_eq!(big.writes().count(), 3);
        assert_eq!(big.write_count(), 3);
    }
}
