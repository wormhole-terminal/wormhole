//! Runs the tests against the terminal on the other end of the pty.
//!
//! A test writes its bytes in 64 KiB chunks, then asks where the cursor is. The terminal answers
//! only after it has parsed everything written before the question, so the time to the answer is
//! the time the terminal took to consume the whole payload, not the time the kernel took to buffer it.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use super::results::{TestResult, Unit};
use super::stats;
use super::workloads::{self, Payload};
use crate::term::{Input, Key, Size};

const CHUNK: usize = 64 * 1024;
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
        write!(self.out, "\x1b[1;1H\x1b[0m\x1b[2K\x1b[38;2;130;180;255m termbench \x1b[0m{text}")?;
        self.out.flush()
    }

    fn interrupted(&mut self) -> bool {
        self.input.keys().iter().any(|k| matches!(k, Key::CtrlC | Key::Esc | Key::Char('q')))
    }

    pub fn run_test(&mut self, name: &str, index: usize, total: usize) -> io::Result<Step> {
        let head = format!("{name} ({}/{total})", index + 1);
        self.reset()?;
        self.status(&format!("{head}  generating…"))?;
        if name == "latency" {
            return self.latency(&head);
        }
        let Some(main) = workloads::payload(name, self.size, self.bytes) else {
            return Err(io::Error::other(format!("no test called {name}")));
        };
        let warm = workloads::payload(name, self.size, WARMUP_BYTES)
            .unwrap_or(Payload { bytes: Vec::new(), frames: 0 });

        // Warm up: fonts, glyph caches and the pty buffers are cold on the first bytes.
        self.reset()?;
        if time_payload(self.input, &mut self.out, &warm.bytes, RUN_TIMEOUT)?.is_none() {
            return Ok(Step::Done(Box::new(timeout_result(name, &main))));
        }

        let mut rates = Vec::new();
        let mut times = Vec::new();
        for run in 0..self.runs {
            if self.interrupted() {
                return Ok(Step::Interrupted);
            }
            if !self.reset()? {
                return Ok(Step::Done(Box::new(timeout_result(name, &main))));
            }
            let Some(secs) = time_payload(self.input, &mut self.out, &main.bytes, RUN_TIMEOUT)? else {
                return Ok(Step::Done(Box::new(timeout_result(name, &main))));
            };
            let rate = rate(name, &main, secs);
            rates.push(rate);
            times.push(secs);
            let (unit, _) = unit_of(name);
            self.reset()?;
            self.status(&format!("{head}  run {}/{}  {:.1} {}", run + 1, self.runs, rate, unit.label()))?;
        }
        Ok(Step::Done(Box::new(throughput_result(name, &main, &rates, &times))))
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

/// Writes `payload` and returns the seconds until the terminal had parsed all of it, or `None` when
/// it took longer than `timeout`.
pub fn time_payload(
    input: &mut Input,
    out: &mut impl Write,
    payload: &[u8],
    timeout: Duration,
) -> io::Result<Option<f64>> {
    let start = Instant::now();
    let deadline = start + timeout;
    for chunk in payload.chunks(CHUNK) {
        out.write_all(chunk)?;
        if Instant::now() > deadline {
            // Let it drain, so the next test doesn't start inside this one's backlog.
            out.flush()?;
            let _ = input.sync(out, RECOVER)?;
            return Ok(None);
        }
    }
    out.flush()?;
    let left = deadline.saturating_duration_since(Instant::now()).max(Duration::from_secs(1));
    Ok(input.sync(out, left)?.map(|_| start.elapsed().as_secs_f64()))
}

fn unit_of(name: &str) -> (Unit, bool) {
    if name == "frames" { (Unit::Fps, true) } else { (Unit::MBps, false) }
}

/// The run's rate in the test's unit: megabytes per second, or frames per second.
fn rate(name: &str, p: &Payload, secs: f64) -> f64 {
    let secs = secs.max(1e-9);
    if unit_of(name).1 { p.frames as f64 / secs } else { p.bytes.len() as f64 / 1e6 / secs }
}

fn throughput_result(name: &str, p: &Payload, rates: &[f64], times: &[f64]) -> TestResult {
    let (unit, frames) = unit_of(name);
    let median = stats::median(rates);
    let (min, max) = stats::min_max(rates);
    let secs = stats::median(times);
    let mut extra = Vec::new();
    if frames {
        extra.push(("mbps".to_string(), p.bytes.len() as f64 / 1e6 / secs.max(1e-9)));
        extra.push(("frames".to_string(), p.frames as f64));
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
    }
}

fn timeout_result(name: &str, p: &Payload) -> TestResult {
    TestResult {
        name: name.to_string(),
        unit: unit_of(name).0,
        median: 0.0,
        min: 0.0,
        max: 0.0,
        secs: RUN_TIMEOUT.as_secs_f64(),
        bytes: p.bytes.len() as u64,
        timeout: true,
        extra: Vec::new(),
    }
}

fn latency_timeout() -> TestResult {
    TestResult {
        name: "latency".to_string(),
        unit: Unit::Micros,
        median: 0.0,
        min: 0.0,
        max: 0.0,
        secs: 0.0,
        bytes: 0,
        timeout: true,
        extra: Vec::new(),
    }
}

fn latency_result(micros: &[f64]) -> TestResult {
    let mut sorted = micros.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let (min, max) = stats::min_max(&sorted);
    TestResult {
        name: "latency".to_string(),
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
        let p = Payload { bytes: vec![0; 10_000_000], frames: 0 };
        let rates: Vec<f64> = [0.5, 0.25, 1.0].iter().map(|s| rate("ascii", &p, *s)).collect();
        let r = throughput_result("ascii", &p, &rates, &[0.5, 0.25, 1.0]);
        assert!((r.median - 20.0).abs() < 1e-9);
        assert_eq!((r.min, r.max), (10.0, 40.0));
        assert_eq!(r.secs, 0.5);
    }

    #[test]
    fn frames_count_frames_per_second() {
        let p = Payload { bytes: vec![0; 4_000_000], frames: 60 };
        assert!((rate("frames", &p, 0.5) - 120.0).abs() < 1e-9);
        let r = throughput_result("frames", &p, &[120.0], &[0.5]);
        assert_eq!(r.unit, Unit::Fps);
        assert!((r.extra("mbps").unwrap_or(0.0) - 8.0).abs() < 1e-9);
    }
}
