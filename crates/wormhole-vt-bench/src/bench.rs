//! Throughput and allocation measurement of an engine on a corpus: what vtbench runs.
//!
//! The measurement code is here, not in the binary, so it can be tested; the binary supplies the
//! counting allocator (a library must not install a global allocator for everyone who links it).

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::corpus::Corpus;
use crate::engine::Engine;

/// Allocations made so far by the process: calls to the allocator and bytes asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllocCounts {
    pub allocs: u64,
    pub bytes: u64,
}

impl AllocCounts {
    fn since(self, before: AllocCounts) -> AllocCounts {
        AllocCounts { allocs: self.allocs - before.allocs, bytes: self.bytes - before.bytes }
    }
}

/// How the stream is fed and how often.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    /// The size of each `feed` call, as a PTY reader's buffer makes them.
    pub chunk: usize,
    /// Passes over the stream; the fastest counts.
    pub runs: usize,
    /// Lines of history the engine keeps.
    pub scrollback: usize,
}

/// What one engine did with one corpus.
#[derive(Debug, Clone)]
pub struct Measurement {
    pub corpus: String,
    pub engine: &'static str,
    pub bytes: usize,
    pub best: Duration,
    pub median: Duration,
    /// Allocator calls per megabyte over a whole first pass on a fresh engine (the screen and the
    /// history grow during it).
    pub allocs_per_mb: f64,
    pub alloc_bytes_per_mb: f64,
    /// The same for a second pass over the same engine, once the screen and history are full:
    /// what an engine in steady state costs. Zero is the goal.
    pub steady_allocs_per_mb: f64,
}

impl Measurement {
    /// Megabytes (10^6 bytes) per second of the fastest pass.
    pub fn mb_per_s(&self) -> f64 {
        mb(self.bytes) / self.best.as_secs_f64()
    }

    pub fn median_mb_per_s(&self) -> f64 {
        mb(self.bytes) / self.median.as_secs_f64()
    }
}

fn mb(bytes: usize) -> f64 {
    bytes as f64 / 1e6
}

fn feed_all<E: Engine>(engine: &mut E, bytes: &[u8], chunk: usize) {
    for piece in bytes.chunks(chunk) {
        engine.feed(piece);
    }
}

/// Feed `corpus` to a fresh `E` `settings.runs` times, timing each pass, and count the
/// allocations of the first pass and of one more pass over the warmed-up last engine.
pub fn measure<E: Engine>(corpus: &Corpus, settings: &Settings, probe: fn() -> AllocCounts) -> Measurement {
    let runs = settings.runs.max(1);
    let mut times = Vec::with_capacity(runs);
    let mut first = AllocCounts::default();
    let mut last_engine = None;
    for run in 0..runs {
        let mut engine = E::new(corpus.cols, corpus.rows, settings.scrollback);
        let before = probe();
        let start = Instant::now();
        feed_all(&mut engine, &corpus.bytes, settings.chunk);
        let elapsed = start.elapsed();
        let counts = probe().since(before);
        if run == 0 {
            first = counts;
        }
        times.push(elapsed);
        // Replies pile up in the adapter; they are not what is measured.
        engine.take_replies();
        last_engine = Some(engine);
    }
    let steady = match last_engine.as_mut() {
        Some(engine) => {
            let before = probe();
            feed_all(engine, &corpus.bytes, settings.chunk);
            probe().since(before)
        }
        None => AllocCounts::default(),
    };
    times.sort();
    let per_mb = |n: u64| n as f64 / mb(corpus.bytes.len());
    Measurement {
        corpus: corpus.name.to_string(),
        engine: E::NAME,
        bytes: corpus.bytes.len(),
        best: times[0],
        median: times[times.len() / 2],
        allocs_per_mb: per_mb(first.allocs),
        alloc_bytes_per_mb: per_mb(first.bytes),
        steady_allocs_per_mb: per_mb(steady.allocs),
    }
}

/// The results as a table, one block per corpus, the first engine in the list being the one the
/// others are compared to (`x` is their speed over its speed).
pub fn table(results: &[Measurement]) -> String {
    let mut corpora: Vec<&str> = Vec::new();
    for r in results {
        if !corpora.contains(&r.corpus.as_str()) {
            corpora.push(&r.corpus);
        }
    }
    let mut out = String::new();
    out.push_str(&format!(
        "{:<15} {:<10} {:>9} {:>9} {:>7} {:>12} {:>12}\n",
        "corpus", "engine", "MB/s", "median", "x", "allocs/MB", "steady/MB"
    ));
    for corpus in corpora {
        let rows: Vec<&Measurement> = results.iter().filter(|r| r.corpus == corpus).collect();
        let base = rows.first().map(|r| r.mb_per_s()).unwrap_or(1.0);
        for r in rows {
            out.push_str(&format!(
                "{:<15} {:<10} {:>9.1} {:>9.1} {:>6.2}x {:>12.1} {:>12.1}\n",
                r.corpus,
                r.engine,
                r.mb_per_s(),
                r.median_mb_per_s(),
                r.mb_per_s() / base,
                r.allocs_per_mb,
                r.steady_allocs_per_mb
            ));
        }
    }
    out
}

/// Facts about the run that make a stored result comparable.
#[derive(Debug, Clone)]
pub struct RunInfo {
    pub settings: Settings,
    pub size: usize,
    pub seed: u64,
    pub release: bool,
}

/// The results as JSON, for a baseline stored in the repository.
pub fn json(results: &[Measurement], info: &RunInfo) -> Value {
    let round = |v: f64| (v * 10.0).round() / 10.0;
    json!({
        "version": 1,
        "chunk_bytes": info.settings.chunk,
        "runs": info.settings.runs,
        "scrollback": info.settings.scrollback,
        "corpus_bytes": info.size,
        "seed": info.seed,
        "release_build": info.release,
        "machine": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        },
        "results": results.iter().map(|r| json!({
            "corpus": r.corpus,
            "engine": r.engine,
            "bytes": r.bytes,
            "best_ms": round(r.best.as_secs_f64() * 1e3),
            "median_ms": round(r.median.as_secs_f64() * 1e3),
            "mb_per_s": round(r.mb_per_s()),
            "median_mb_per_s": round(r.median_mb_per_s()),
            "allocs_per_mb": round(r.allocs_per_mb),
            "alloc_bytes_per_mb": round(r.alloc_bytes_per_mb),
            "steady_allocs_per_mb": round(r.steady_allocs_per_mb),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::corpus;
    use crate::engines::WormholeEngine;

    /// A probe that counts its own calls, standing in for the allocator.
    fn fake_probe() -> AllocCounts {
        static CALLS: AtomicU64 = AtomicU64::new(0);
        let n = CALLS.fetch_add(1, Ordering::Relaxed);
        AllocCounts { allocs: n * 10, bytes: n * 100 }
    }

    fn quick() -> Settings {
        Settings { chunk: 4096, runs: 2, scrollback: 100 }
    }

    #[test]
    fn a_measurement_has_a_speed_and_allocation_counts() {
        let corpus = corpus::generate("log-lines", 64 * 1024, 1).expect("a known corpus");
        let m = measure::<WormholeEngine>(&corpus, &quick(), fake_probe);
        assert_eq!((m.engine, m.corpus.as_str(), m.bytes), ("wormhole", "log-lines", corpus.bytes.len()));
        assert!(m.best <= m.median && m.mb_per_s() >= m.median_mb_per_s() && m.mb_per_s() > 0.0);
        // Two probe calls per pass, ten allocations apart.
        assert!(m.allocs_per_mb > 0.0 && m.steady_allocs_per_mb > 0.0);
    }

    #[test]
    fn the_table_compares_engines_to_the_first() {
        let m = |engine: &'static str, ms: u64| Measurement {
            corpus: "c".into(),
            engine,
            bytes: 1_000_000,
            best: Duration::from_millis(ms),
            median: Duration::from_millis(ms + 1),
            allocs_per_mb: 12.0,
            alloc_bytes_per_mb: 1.0,
            steady_allocs_per_mb: 0.0,
        };
        let text = table(&[m("a", 100), m("b", 50)]);
        assert!(text.contains("corpus") && text.contains("steady/MB"), "{text}");
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[1].contains("10.0") && lines[1].contains("1.00x"), "{text}");
        assert!(lines[2].contains("20.0") && lines[2].contains("2.00x"), "{text}");
    }

    #[test]
    fn json_carries_what_makes_a_baseline_comparable() {
        let corpus = corpus::generate("ascii-flood", 32 * 1024, 1).expect("a known corpus");
        let results = [measure::<WormholeEngine>(&corpus, &quick(), fake_probe)];
        let info = RunInfo { settings: quick(), size: 32 * 1024, seed: 1, release: false };
        let value = json(&results, &info);
        assert_eq!(value["version"], 1);
        assert_eq!(value["chunk_bytes"], 4096);
        assert_eq!(value["results"][0]["engine"], "wormhole");
        assert!(value["results"][0]["mb_per_s"].as_f64().is_some_and(|v| v > 0.0));
        assert!(value["machine"]["cpus"].as_u64().is_some_and(|c| c >= 1));
    }
}
