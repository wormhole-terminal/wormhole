//! Why a terminal was fast or slow: the timing of every write, boiled down per run into where the
//! time went (the writer blocked because the terminal stopped reading, or a tail after the last byte
//! because it was still parsing), the stalls, a throughput timeline, and what the terminal process
//! used. The raw per-write events can be saved as a trace.

use std::io::{self, Write};

use super::compose::Composition;
use super::json::{self, Value};
use super::procinfo::{Sample as ProcSample, Terminal};
use super::stats;

/// Version of the diagnostic fields in the results file; bumped when one changes meaning.
pub const DIAG_VERSION: u32 = 1;
/// A write that took longer than this was blocked: the terminal had stopped reading.
pub const STALL_MS: f64 = 20.0;
/// The timeline's resolution.
pub const BUCKET_MS: f64 = 100.0;
/// How many of the longest stalls a run keeps.
const KEEP_STALLS: usize = 8;

/// One write: when it finished (since the run started), how long it took and how big it was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WriteEvent {
    pub end_ns: u64,
    pub took_ns: u64,
    pub bytes: usize,
    /// How many of the input check's queries (`integrity`) the write carried inside its data.
    pub checkpoints: u32,
}

/// A timed run: how long until the terminal had answered, how much of that was waiting for the
/// answer after the last byte, and every write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timed {
    pub secs: f64,
    pub dsr_wait_s: f64,
    pub writes: Vec<WriteEvent>,
    /// What the input check's checkpoints in this run were drawn from (`integrity::Plan::seed`).
    pub seed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stall {
    /// Seconds into the run when the write started.
    pub at_s: f64,
    pub ms: f64,
}

/// What one run showed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunDiag {
    pub secs: f64,
    /// Seconds spent inside `write`: with a full PTY buffer, the terminal not reading.
    pub write_blocked_s: f64,
    /// Seconds from the last byte to the terminal's answer: the parse tail.
    pub dsr_wait_s: f64,
    pub writes: usize,
    pub write_p50_us: f64,
    pub write_p99_us: f64,
    pub write_max_us: f64,
    /// Writes that took longer than `STALL_MS`.
    pub stalls: usize,
    pub stalled_s: f64,
    /// The longest of them, by time into the run.
    pub longest: Vec<Stall>,
    /// MB/s the terminal accepted in each `BUCKET_MS`.
    pub timeline: Vec<f64>,
    /// The terminal process's CPU seconds during the run, when it could be read.
    pub cpu_s: Option<f64>,
    pub rss_start_kb: Option<u64>,
    pub rss_end_kb: Option<u64>,
    /// What the run's checkpoints were drawn from; `--verify-seed` replays them.
    pub seed: Option<u64>,
}

impl RunDiag {
    pub fn from_timed(t: &Timed, before: Option<ProcSample>, after: Option<ProcSample>) -> RunDiag {
        let ns = 1e-9;
        let mut micros: Vec<f64> = t.writes.iter().map(|w| w.took_ns as f64 / 1e3).collect();
        micros.sort_by(|a, b| a.total_cmp(b));
        let mut stalls: Vec<Stall> = t
            .writes
            .iter()
            .filter(|w| w.took_ns as f64 / 1e6 > STALL_MS)
            .map(|w| Stall {
                at_s: w.end_ns.saturating_sub(w.took_ns) as f64 * ns,
                ms: w.took_ns as f64 / 1e6,
            })
            .collect();
        let (count, stalled_s) = (stalls.len(), stalls.iter().map(|s| s.ms / 1e3).sum());
        stalls.sort_by(|a, b| b.ms.total_cmp(&a.ms));
        stalls.truncate(KEEP_STALLS);
        stalls.sort_by(|a, b| a.at_s.total_cmp(&b.at_s));
        RunDiag {
            secs: t.secs,
            write_blocked_s: t.writes.iter().map(|w| w.took_ns).sum::<u64>() as f64 * ns,
            dsr_wait_s: t.dsr_wait_s,
            writes: t.writes.len(),
            write_p50_us: stats::percentile(&micros, 50.0),
            write_p99_us: stats::percentile(&micros, 99.0),
            write_max_us: micros.last().copied().unwrap_or(0.0),
            stalls: count,
            stalled_s,
            longest: stalls,
            timeline: timeline(&t.writes, t.secs),
            cpu_s: before.zip(after).map(|(b, a)| (a.cpu_s - b.cpu_s).max(0.0)),
            rss_start_kb: before.map(|s| s.rss_kb),
            rss_end_kb: after.map(|s| s.rss_kb),
            seed: t.seed,
        }
    }

    /// The share of the run the write was blocked.
    pub fn blocked_share(&self) -> f64 {
        if self.secs > 0.0 { self.write_blocked_s / self.secs } else { 0.0 }
    }

    pub fn tail_share(&self) -> f64 {
        if self.secs > 0.0 { self.dsr_wait_s / self.secs } else { 0.0 }
    }

    /// CPU time over wall time: 1.0 is one core busy for the whole run.
    pub fn cpu_share(&self) -> Option<f64> {
        self.cpu_s.filter(|_| self.secs > 0.0).map(|c| c / self.secs)
    }

    pub fn rss_growth_kb(&self) -> Option<i64> {
        Some(self.rss_end_kb? as i64 - self.rss_start_kb? as i64)
    }
}

/// MB/s accepted per `BUCKET_MS`, from the start to `secs`. The last bucket is partial and is
/// measured over the time it covers.
pub fn timeline(writes: &[WriteEvent], secs: f64) -> Vec<f64> {
    let bucket_ns = (BUCKET_MS * 1e6) as u64;
    let count = ((secs * 1e3 / BUCKET_MS).ceil() as usize).max(1);
    let mut bytes = vec![0u64; count];
    for w in writes {
        let i = ((w.end_ns / bucket_ns) as usize).min(count - 1);
        bytes[i] += w.bytes as u64;
    }
    bytes
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let covered = (secs - i as f64 * BUCKET_MS / 1e3).clamp(0.001, BUCKET_MS / 1e3);
            *b as f64 / 1e6 / covered
        })
        .collect()
}

/// What a test showed: the payload, each run and which one is the median.
#[derive(Clone, Debug, PartialEq)]
pub struct TestDiag {
    pub composition: Composition,
    pub runs: Vec<RunDiag>,
    /// The index of the run whose rate is closest to the median.
    pub median_run: usize,
}

impl TestDiag {
    pub fn median(&self) -> Option<&RunDiag> {
        self.runs.get(self.median_run)
    }

    pub fn to_json(&self) -> String {
        let runs: Vec<String> = self.runs.iter().map(run_json).collect();
        format!(
            "{{\"composition\": {}, \"median_run\": {}, \"runs\": [{}]}}",
            self.composition.to_json(),
            self.median_run,
            runs.join(", ")
        )
    }

    pub fn from_value(v: &Value) -> Option<TestDiag> {
        let runs = v.get("runs")?.as_arr()?.iter().map(run_from_value).collect();
        Some(TestDiag {
            composition: Composition::from_value(v.get("composition")?),
            runs,
            median_run: v.get("median_run").and_then(Value::as_f64).unwrap_or(0.0) as usize,
        })
    }
}

fn opt(n: Option<f64>) -> String {
    n.map_or("null".to_string(), json::number)
}

fn run_json(r: &RunDiag) -> String {
    let longest: Vec<String> = r
        .longest
        .iter()
        .map(|s| format!("{{\"at_s\": {}, \"ms\": {}}}", json::number(s.at_s), json::number(s.ms)))
        .collect();
    let timeline: Vec<String> = r.timeline.iter().map(|x| json::number(*x)).collect();
    format!(
        "{{\"secs\": {}, \"write_blocked_s\": {}, \"dsr_wait_s\": {}, \"writes\": {}, \
         \"write_us\": {{\"p50\": {}, \"p99\": {}, \"max\": {}}}, \
         \"stalls\": {{\"over_ms\": {}, \"count\": {}, \"total_s\": {}, \"longest\": [{}]}}, \
         \"timeline\": {{\"bucket_ms\": {}, \"mb_per_s\": [{}]}}, \
         \"terminal\": {{\"cpu_s\": {}, \"rss_start_kb\": {}, \"rss_end_kb\": {}}}, \
         \"checkpoint_seed\": {}}}",
        json::number(r.secs),
        json::number(r.write_blocked_s),
        json::number(r.dsr_wait_s),
        r.writes,
        json::number(r.write_p50_us),
        json::number(r.write_p99_us),
        json::number(r.write_max_us),
        json::number(STALL_MS),
        r.stalls,
        json::number(r.stalled_s),
        longest.join(", "),
        json::number(BUCKET_MS),
        timeline.join(", "),
        opt(r.cpu_s),
        opt(r.rss_start_kb.map(|k| k as f64)),
        opt(r.rss_end_kb.map(|k| k as f64)),
        // A whole number: `opt` rounds to thousandths, which loses the low digits of a large seed.
        r.seed.map_or("null".to_string(), |n| n.to_string()),
    )
}

fn run_from_value(v: &Value) -> RunDiag {
    let n = |o: Option<&Value>, k: &str| o.and_then(|o| o.get(k)).and_then(Value::as_f64);
    let top = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let (wu, st, term) = (v.get("write_us"), v.get("stalls"), v.get("terminal"));
    RunDiag {
        secs: top("secs"),
        write_blocked_s: top("write_blocked_s"),
        dsr_wait_s: top("dsr_wait_s"),
        writes: top("writes") as usize,
        write_p50_us: n(wu, "p50").unwrap_or(0.0),
        write_p99_us: n(wu, "p99").unwrap_or(0.0),
        write_max_us: n(wu, "max").unwrap_or(0.0),
        stalls: n(st, "count").unwrap_or(0.0) as usize,
        stalled_s: n(st, "total_s").unwrap_or(0.0),
        longest: st
            .and_then(|s| s.get("longest"))
            .and_then(Value::as_arr)
            .map(|a| {
                a.iter()
                    .map(|s| Stall {
                        at_s: s.get("at_s").and_then(Value::as_f64).unwrap_or(0.0),
                        ms: s.get("ms").and_then(Value::as_f64).unwrap_or(0.0),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        timeline: v
            .get("timeline")
            .and_then(|t| t.get("mb_per_s"))
            .and_then(Value::as_arr)
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default(),
        cpu_s: n(term, "cpu_s"),
        rss_start_kb: n(term, "rss_start_kb").map(|k| k as u64),
        rss_end_kb: n(term, "rss_end_kb").map(|k| k as u64),
        seed: v.get("checkpoint_seed").and_then(Value::as_f64).map(|n| n as u64),
    }
}

/// The first line of a `--trace` file when the input check is on: the seed `--verify-seed` takes back and
/// the distance asked for, which the places of every run's checkpoints follow from.
pub fn write_trace_header(out: &mut impl Write, seed: u64, every_bytes: usize) -> io::Result<()> {
    writeln!(out, "{{\"event\":\"check\",\"verify_seed\":{seed},\"verify_every_bytes\":{every_bytes}}}")
}

/// Every write of a run as JSON Lines, then the wait for the answer: the `--trace` file.
pub fn write_trace(out: &mut impl Write, test: &str, run: usize, t: &Timed) -> io::Result<()> {
    if let Some(seed) = t.seed {
        let checkpoints: u32 = t.writes.iter().map(|w| w.checkpoints).sum();
        writeln!(
            out,
            "{{\"test\":{},\"run\":{run},\"event\":\"plan\",\"checkpoint_seed\":{seed},\"checkpoints\":{checkpoints}}}",
            json::quote(test),
        )?;
    }
    let mut cum = 0u64;
    for w in &t.writes {
        cum += w.bytes as u64;
        writeln!(
            out,
            "{{\"test\":{},\"run\":{run},\"event\":\"write\",\"t_ms\":{:.3},\"bytes\":{},\"write_ms\":{:.3},\"cum_bytes\":{cum},\"checkpoints\":{}}}",
            json::quote(test),
            w.end_ns as f64 / 1e6,
            w.bytes,
            w.took_ns as f64 / 1e6,
            w.checkpoints,
        )?;
    }
    writeln!(
        out,
        "{{\"test\":{},\"run\":{run},\"event\":\"dsr\",\"t_ms\":{:.3},\"wait_ms\":{:.3},\"cum_bytes\":{cum}}}",
        json::quote(test),
        t.secs * 1e3,
        t.dsr_wait_s * 1e3,
    )
}

/// A row of block characters for `values`, shrunk to at most `max` characters by averaging
/// neighbours. Empty for no values.
pub fn sparkline(values: &[f64], max: usize) -> String {
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if values.is_empty() || max == 0 {
        return String::new();
    }
    let per = values.len().div_ceil(max);
    let merged: Vec<f64> = values.chunks(per).map(|c| c.iter().sum::<f64>() / c.len() as f64).collect();
    let top = merged.iter().cloned().fold(0.0f64, f64::max);
    merged
        .iter()
        .map(
            |v| {
                if top <= 0.0 {
                    BLOCKS[0]
                } else {
                    BLOCKS[((v / top) * 7.0).round().clamp(0.0, 7.0) as usize]
                }
            },
        )
        .collect()
}

/// A few plain sentences on what the run shows. `fast_enough` is the thing to say last, when nothing
/// stood out.
pub fn findings(d: &RunDiag, comp: &Composition, process: Option<&Terminal>) -> Vec<String> {
    let mut out = Vec::new();
    if d.secs <= 0.0 {
        return out;
    }
    let (blocked, tail) = (d.blocked_share(), d.tail_share());
    let pct = |x: f64| (x * 100.0).round() as u32;

    if comp.avg_write() > 0.0 && comp.avg_write() < 256.0 {
        out.push(format!(
            "tiny writes: {} writes of {:.0} bytes on average ({:.0} a second); the writer spent {}% of the time in write(), which here is mostly \
             the cost of each call (kernel and the terminal's read loop), not bandwidth",
            d.writes,
            comp.avg_write(),
            d.writes as f64 / d.secs,
            pct(blocked)
        ));
    } else if blocked >= 0.6 {
        out.push(format!(
            "read-bound: the writer waited for the terminal {}% of the time and the answer came {:.1} ms after the \
             last byte, so reading sets the pace and the parser keeps up; to go faster, look at the PTY reader \
             (read size, wake-ups, copies) and the parser's cost per byte",
            pct(blocked),
            d.dsr_wait_s * 1e3
        ));
    }
    if tail >= 0.5 {
        out.push(format!(
            "parse-bound: {}% of the time ({:.0} ms) was spent after the last byte, waiting for the answer; the terminal \
             accepts input faster than it parses it",
            pct(tail),
            d.dsr_wait_s * 1e3
        ));
    } else if d.dsr_wait_s >= 0.1 && tail >= 0.1 {
        out.push(format!(
            "parse tail {:.0} ms after the last byte: the parser runs behind the reader",
            d.dsr_wait_s * 1e3
        ));
    }
    let big: Vec<&Stall> = d.longest.iter().filter(|s| s.ms >= 100.0).collect();
    if big.len() >= 3 {
        let at: Vec<String> = big.iter().take(5).map(|s| format!("{:.1} s", s.at_s)).collect();
        let more = if big.len() > 5 { ", ..." } else { "" };
        let periodic = if is_periodic(&big.iter().map(|s| s.at_s).collect::<Vec<_>>()) {
            " - periodic pauses"
        } else {
            ""
        };
        out.push(format!("{} stalls over 100 ms at {}{more}{periodic}", big.len(), at.join(", ")));
    } else if let Some(s) = big.iter().max_by(|a, b| a.ms.total_cmp(&b.ms)) {
        out.push(format!(
            "{} stall{} over 100 ms, the longest {:.0} ms at {:.1} s: the terminal stopped reading for a while (a render, GC or throttling pause)",
            big.len(),
            if big.len() == 1 { "" } else { "s" },
            s.ms,
            s.at_s
        ));
    }
    if d.write_p50_us > 0.0 && d.write_p99_us > d.write_p50_us * 20.0 && d.write_p99_us > 5_000.0 {
        out.push(format!(
            "write latency is spiky: p99 {:.1} ms against p50 {:.0} us",
            d.write_p99_us / 1e3,
            d.write_p50_us
        ));
    }
    if let Some(share) = d.cpu_share() {
        if share < 0.5 && (blocked > 0.3 || tail > 0.3) {
            out.push(format!(
                "CPU {}% of wall time while the writer waits: waiting rather than computing - check render gating, vsync or a throttled thread",
                pct(share)
            ));
        } else if share > 1.2 {
            out.push(format!("CPU {}% of wall time: more than one thread was busy (reader, parser and renderer in parallel)", pct(share)));
        }
    }
    if d.timeline.len() >= 8 {
        let inner = &d.timeline[1..d.timeline.len() - 1];
        let mid = stats::median(inner);
        if let Some((i, low)) = inner.iter().enumerate().min_by(|a, b| a.1.total_cmp(b.1))
            && mid > 0.0
            && *low < mid * 0.25
        {
            out.push(format!(
                "throughput dips to {low:.0} MB/s at {:.1} s (the typical 100 ms is {mid:.0} MB/s): a hiccup",
                (i + 1) as f64 * BUCKET_MS / 1e3
            ));
        }
    }
    if let Some(kb) = d.rss_growth_kb()
        && kb >= 50 * 1024
    {
        out.push(format!(
            "memory grew by {} MB during the run ({:.1} bytes per input byte): scrollback or caches",
            kb / 1024,
            kb as f64 * 1024.0 / comp.bytes.max(1) as f64
        ));
    }
    if let Some(note) = process.and_then(|p| p.note.as_deref()) {
        out.push(format!("the process measured is {note}, not the terminal: CPU and memory describe it"));
    }
    if out.is_empty() {
        out.push("no stalls and no parse tail: the terminal kept up with the writer".to_string());
    }
    out
}

/// Whether times are about evenly spaced (the gaps within a quarter of their mean).
fn is_periodic(times: &[f64]) -> bool {
    if times.len() < 3 {
        return false;
    }
    let gaps: Vec<f64> = times.windows(2).map(|w| w[1] - w[0]).collect();
    let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
    mean > 0.0 && gaps.iter().all(|g| (g - mean).abs() <= mean * 0.25)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(end_ms: f64, took_ms: f64, bytes: usize) -> WriteEvent {
        WriteEvent { end_ns: (end_ms * 1e6) as u64, took_ns: (took_ms * 1e6) as u64, bytes, checkpoints: 0 }
    }

    #[test]
    fn stalls_are_the_slow_writes_with_where_they_started() {
        let t = Timed {
            secs: 1.0,
            dsr_wait_s: 0.1,
            writes: vec![
                ev(10.0, 1.0, 100),
                ev(500.0, 300.0, 100),
                ev(520.0, 10.0, 100),
                ev(900.0, 60.0, 100),
            ],
            ..Timed::default()
        };
        let d = RunDiag::from_timed(&t, None, None);
        assert_eq!((d.writes, d.stalls), (4, 2));
        assert!((d.stalled_s - 0.36).abs() < 1e-9);
        // Sorted by when they started: 200 ms and 840 ms in.
        assert_eq!(d.longest.len(), 2);
        assert!((d.longest[0].at_s - 0.2).abs() < 1e-9 && (d.longest[0].ms - 300.0).abs() < 1e-9);
        assert!((d.longest[1].at_s - 0.84).abs() < 1e-9);
        assert!((d.write_blocked_s - 0.371).abs() < 1e-9);
        assert!((d.blocked_share() - 0.371).abs() < 1e-9 && (d.tail_share() - 0.1).abs() < 1e-9);
        assert_eq!(d.write_max_us, 300_000.0);
    }

    #[test]
    fn only_the_longest_stalls_are_kept() {
        let writes = (0..20).map(|i| ev(100.0 * (i + 1) as f64, 30.0 + i as f64, 10)).collect();
        let d = RunDiag::from_timed(&Timed { secs: 2.5, writes, ..Timed::default() }, None, None);
        assert_eq!((d.stalls, d.longest.len()), (20, KEEP_STALLS));
        assert!(d.longest.iter().all(|s| s.ms >= 42.0));
    }

    #[test]
    fn the_timeline_is_megabytes_per_second_per_bucket() {
        let writes = vec![
            ev(10.0, 1.0, 500_000),
            ev(90.0, 1.0, 500_000),
            ev(150.0, 1.0, 100_000),
            ev(260.0, 1.0, 1_000_000),
        ];
        let tl = timeline(&writes, 0.3);
        assert_eq!(tl.len(), 3);
        assert!((tl[0] - 10.0).abs() < 1e-9 && (tl[1] - 1.0).abs() < 1e-9 && (tl[2] - 10.0).abs() < 1e-9);
        // The last bucket is partial and measured over what it covers.
        let tl = timeline(&[ev(120.0, 1.0, 1_000_000)], 0.15);
        assert!((tl[1] - 20.0).abs() < 1e-6, "{tl:?}");
        // A run shorter than a bucket still has one.
        assert_eq!(timeline(&[], 0.0).len(), 1);
    }

    #[test]
    fn cpu_and_memory_come_from_two_samples() {
        let t = Timed { secs: 2.0, ..Timed::default() };
        let (a, b) = (ProcSample { cpu_s: 10.0, rss_kb: 1000 }, ProcSample { cpu_s: 13.0, rss_kb: 3000 });
        let d = RunDiag::from_timed(&t, Some(a), Some(b));
        assert_eq!((d.cpu_s, d.cpu_share(), d.rss_growth_kb()), (Some(3.0), Some(1.5), Some(2000)));
        let none = RunDiag::from_timed(&t, Some(a), None);
        assert_eq!((none.cpu_s, none.cpu_share(), none.rss_growth_kb()), (None, None, None));
    }

    fn comp(bytes: u64, writes: u64) -> Composition {
        Composition { bytes, writes, ..Composition::default() }
    }

    fn run(secs: f64, blocked: f64, tail: f64) -> RunDiag {
        RunDiag { secs, write_blocked_s: blocked, dsr_wait_s: tail, writes: 100, ..RunDiag::default() }
    }

    #[test]
    fn a_blocked_writer_is_read_bound() {
        let f = findings(&run(1.0, 0.82, 0.02), &comp(16_000_000, 250), None);
        assert!(f[0].starts_with("read-bound: the writer waited for the terminal 82% of the time"), "{f:?}");
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn a_long_tail_is_the_parser_running_behind() {
        let f = findings(&run(1.0, 0.1, 0.14), &comp(16_000_000, 250), None);
        assert!(f.iter().any(|s| s.starts_with("parse tail 140 ms after the last byte")), "{f:?}");
        let f = findings(&run(1.0, 0.1, 0.7), &comp(16_000_000, 250), None);
        assert!(f.iter().any(|s| s.starts_with("parse-bound: 70%")), "{f:?}");
    }

    #[test]
    fn low_cpu_while_waiting_means_waiting() {
        let mut d = run(1.0, 0.8, 0.0);
        d.cpu_s = Some(0.35);
        let f = findings(&d, &comp(16_000_000, 250), None);
        assert!(
            f.iter().any(|s| s
                .starts_with("CPU 35% of wall time while the writer waits: waiting rather than computing")),
            "{f:?}"
        );
        d.cpu_s = Some(2.4);
        let f = findings(&d, &comp(16_000_000, 250), None);
        assert!(f.iter().any(|s| s.starts_with("CPU 240% of wall time: more than one thread")), "{f:?}");
    }

    #[test]
    fn periodic_stalls_are_named() {
        let mut d = run(5.0, 1.0, 0.0);
        d.longest = [1.2, 2.5, 3.8].iter().map(|at| Stall { at_s: *at, ms: 150.0 }).collect();
        let f = findings(&d, &comp(16_000_000, 250), None);
        assert!(
            f.iter().any(|s| s == "3 stalls over 100 ms at 1.2 s, 2.5 s, 3.8 s - periodic pauses"),
            "{f:?}"
        );
        d.longest = [0.2, 0.3, 4.0].iter().map(|at| Stall { at_s: *at, ms: 150.0 }).collect();
        let f = findings(&d, &comp(16_000_000, 250), None);
        assert!(f.iter().any(|s| s.starts_with("3 stalls") && !s.contains("periodic")), "{f:?}");
        d.longest = vec![Stall { at_s: 2.0, ms: 400.0 }];
        let f = findings(&d, &comp(16_000_000, 250), None);
        assert!(f.iter().any(|s| s.starts_with("1 stall over 100 ms, the longest 400 ms at 2.0 s")), "{f:?}");
    }

    #[test]
    fn a_hiccup_in_the_timeline_is_found() {
        let mut d = run(1.0, 0.1, 0.0);
        d.timeline = vec![50.0, 100.0, 100.0, 5.0, 100.0, 100.0, 100.0, 100.0, 60.0];
        let f = findings(&d, &comp(16_000_000, 250), None);
        assert!(f.iter().any(|s| s.starts_with("throughput dips to 5 MB/s at 0.3 s")), "{f:?}");
    }

    #[test]
    fn tiny_writes_and_memory_and_tmux_have_their_say() {
        let mut d = run(2.0, 1.4, 0.0);
        d.rss_start_kb = Some(100_000);
        d.rss_end_kb = Some(400_000);
        let tmux = Terminal { pid: 1, command: "tmux".into(), note: Some("tmux".into()) };
        let f = findings(&d, &comp(2_000_000, 220_000), Some(&tmux));
        assert!(f[0].starts_with("tiny writes: 100 writes of 9 bytes"), "{f:?}");
        assert!(f.iter().any(|s| s.starts_with("memory grew by 292 MB")), "{f:?}");
        assert!(f.iter().any(|s| s.contains("is tmux, not the terminal")), "{f:?}");
    }

    #[test]
    fn a_clean_run_says_so() {
        let f = findings(&run(1.0, 0.05, 0.01), &comp(16_000_000, 250), None);
        assert_eq!(f, vec!["no stalls and no parse tail: the terminal kept up with the writer".to_string()]);
    }

    #[test]
    fn the_sparkline_scales_and_shrinks() {
        assert_eq!(sparkline(&[0.0, 5.0, 10.0], 10), "▁▅█");
        assert_eq!(sparkline(&[1.0; 100], 20).chars().count(), 20);
        assert_eq!(sparkline(&[], 10), "");
        assert_eq!(sparkline(&[0.0, 0.0], 10), "▁▁");
    }

    #[test]
    fn the_diagnostics_survive_the_file() {
        let t = Timed {
            secs: 1.0,
            dsr_wait_s: 0.1,
            writes: vec![ev(10.0, 1.0, 100), ev(500.0, 300.0, 100)],
            seed: Some(7_624_267_924_332_224),
        };
        let a = ProcSample { cpu_s: 1.0, rss_kb: 1000 };
        let d = TestDiag {
            composition: Composition { bytes: 200, writes: 2, sgr: 3, ..Composition::default() },
            runs: vec![RunDiag::from_timed(&t, Some(a), Some(a)), RunDiag::from_timed(&t, None, None)],
            median_run: 1,
        };
        let v = json::parse(&d.to_json()).expect("valid json");
        let back = TestDiag::from_value(&v).expect("reads back");
        // Numbers are written to three decimals, so compare what a second trip writes.
        assert_eq!(back.to_json(), d.to_json());
        assert_eq!(
            (back.composition.clone(), back.median_run, back.runs.len()),
            (d.composition.clone(), 1, 2)
        );
        // A seed is written as the whole number it is: rounding to thousandths, as for the other numbers,
        // turned this one into ...225.
        assert!(d.to_json().contains("\"checkpoint_seed\": 7624267924332224"), "{}", d.to_json());
        assert_eq!(back.runs[0].seed, Some(7_624_267_924_332_224));
    }

    #[test]
    fn the_trace_has_a_line_per_write_and_one_for_the_answer() {
        let t = Timed {
            secs: 0.5,
            dsr_wait_s: 0.1,
            writes: vec![ev(10.0, 2.0, 100), ev(30.0, 20.0, 50)],
            ..Timed::default()
        };
        let mut out = Vec::new();
        write_trace(&mut out, "logflood", 2, &t).expect("writes to a vec");
        let text = String::from_utf8_lossy(&out);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[1],
            "{\"test\":\"logflood\",\"run\":2,\"event\":\"write\",\"t_ms\":30.000,\"bytes\":50,\"write_ms\":20.000,\"cum_bytes\":150,\"checkpoints\":0}"
        );
        assert!(lines[2].contains("\"event\":\"dsr\"") && lines[2].contains("\"wait_ms\":100.000"));
        assert!(lines.iter().all(|l| json::parse(l).is_ok()));
    }

    #[test]
    fn the_trace_starts_with_the_seed_of_the_invocation() {
        let mut out = Vec::new();
        write_trace_header(&mut out, 7_624_267_924_332_224, 262_144).expect("writes to a vec");
        let text = String::from_utf8_lossy(&out);
        assert_eq!(
            text.trim_end(),
            "{\"event\":\"check\",\"verify_seed\":7624267924332224,\"verify_every_bytes\":262144}"
        );
        assert!(json::parse(&text).is_ok());
    }

    #[test]
    fn the_trace_says_which_checkpoints_a_run_had() {
        let mut writes = vec![ev(10.0, 2.0, 100), ev(30.0, 20.0, 50)];
        writes[1].checkpoints = 2;
        let t = Timed { secs: 0.5, dsr_wait_s: 0.1, writes, seed: Some(4242) };
        let mut out = Vec::new();
        write_trace(&mut out, "logflood", 1, &t).expect("writes to a vec");
        let text = String::from_utf8_lossy(&out);
        assert_eq!(
            text.lines().next(),
            Some(
                "{\"test\":\"logflood\",\"run\":1,\"event\":\"plan\",\"checkpoint_seed\":4242,\"checkpoints\":2}"
            )
        );
        assert!(text.lines().all(|l| json::parse(l).is_ok()));
    }
}
