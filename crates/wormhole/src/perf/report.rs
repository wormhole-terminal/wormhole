//! `perf-profile`'s numbers: the frames it drew and the terminal layer's counters, turned into the
//! report it writes as JSON (for `scripts/profile-app.sh` to compare with a baseline) and as a
//! short table (for a person). Pure: no window, no clock.

use std::fmt::Write as _;
use std::time::Duration;

use serde::Serialize;
use wormhole_core::terminal::perf::{Counters, LockUse};

use super::{Part, Parts};

/// The report's format, for whoever reads the JSON.
pub(crate) const FORMAT: &str = "wormhole-perf-profile";
pub(crate) const FORMAT_VERSION: u32 = 1;

/// The first frames warm caches (shaped words, icons); like devctl `frames`, they are left out.
pub(crate) const WARM_UP_FRAMES: usize = 3;

/// One frame the command drew.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Sample {
    /// The whole window's draw.
    pub draw_ns: u64,
    /// The UI thread's allocations during the draw, and the bytes they asked for.
    pub allocations: u64,
    pub alloc_bytes: u64,
    /// How long the UI waited for terminals' locks during the draw.
    pub lock_wait_ns: u64,
    /// The terminal's parts of it.
    pub parts: Parts,
}

/// A number's spread over the frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Spread {
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
}

impl Spread {
    /// The spread of `values` (nearest-rank percentiles); all zero without any.
    pub(crate) fn of(mut values: Vec<f64>) -> Spread {
        if values.is_empty() {
            return Spread::default();
        }
        values.sort_by(f64::total_cmp);
        let rank = |p: f64| values[((p * values.len() as f64).ceil() as usize).clamp(1, values.len()) - 1];
        Spread {
            avg: round(values.iter().sum::<f64>() / values.len() as f64),
            p50: round(rank(0.5)),
            p95: round(rank(0.95)),
            max: round(values[values.len() - 1]),
        }
    }
}

/// What the frames cost.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Frames {
    /// Frames measured (after the warm-up).
    pub count: usize,
    /// Of those, the ones that drew a terminal grid (the others reused its last paint).
    pub terminal_frames: usize,
    /// Frames drawn per second of the profile, a 2 ms pause after each included.
    pub per_second: f64,
    pub draw_ms: Spread,
    pub allocations: Spread,
    pub alloc_kib: Spread,
    pub lock_wait_ms: Spread,
    /// Per part, over the frames that drew a terminal: milliseconds, then allocations (a JSON
    /// object by part name, in [`Part::ALL`]'s order).
    #[serde(serialize_with = "as_map")]
    pub parts_ms: Vec<(&'static str, Spread)>,
    #[serde(serialize_with = "as_map")]
    pub parts_allocations: Vec<(&'static str, Spread)>,
    pub rows_built: Spread,
    pub words_shaped: Spread,
}

/// The lock as one side used it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Lock {
    pub locks: u64,
    pub wait_ms: f64,
    pub wait_max_ms: f64,
    pub hold_ms: f64,
    pub hold_max_ms: f64,
}

impl From<LockUse> for Lock {
    fn from(used: LockUse) -> Lock {
        Lock {
            locks: used.locks,
            wait_ms: ms(used.wait_ns),
            wait_max_ms: ms(used.wait_max_ns),
            hold_ms: ms(used.hold_ns),
            hold_max_ms: ms(used.hold_max_ns),
        }
    }
}

/// What reading output cost, every terminal of the process together, over the whole profile (the
/// warm-up frames' time included: the readers don't draw frames).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Reader {
    pub bytes: u64,
    /// Bytes parsed per second of the profile.
    pub mb_per_s: f64,
    /// Bytes parsed per second of parsing: the parser's own speed.
    pub parse_mb_per_s: f64,
    /// The share of the profile's time spent parsing (over 100 with several terminals busy).
    pub parse_busy_pct: f64,
    pub slices: u64,
    pub lock: Lock,
}

/// The window's terminals, by where their programs run. A hosted terminal's program is read and
/// parsed by its session host, another process this profile doesn't see; the app parses the
/// host's stream again into its replica, which is what `reader` counts. Two profiles compare only
/// when these match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Terminals {
    pub hosted: usize,
    pub local: usize,
}

/// Everything `perf-profile` writes.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Report {
    pub format: &'static str,
    pub format_version: u32,
    pub seconds: f64,
    pub terminals: Terminals,
    pub frames: Frames,
    pub reader: Reader,
    /// The terminals' locks as every other thread took them (the UI's snapshot, input, find; the
    /// threads that decode pictures).
    pub ui_lock: Lock,
}

impl Report {
    /// The report of `samples` drawn in `elapsed`, with the terminal layer's `counters`, for a
    /// window with these `terminals`.
    pub(crate) fn new(
        samples: &[Sample],
        elapsed: Duration,
        counters: Counters,
        terminals: Terminals,
    ) -> Report {
        let seconds = elapsed.as_secs_f64();
        let frames = &samples[samples.len().min(WARM_UP_FRAMES)..];
        let drew: Vec<&Sample> = frames.iter().filter(|s| s.parts.terminals > 0).collect();
        let spread = |f: &dyn Fn(&Sample) -> f64| Spread::of(frames.iter().map(f).collect());
        let terminal = |f: &dyn Fn(&Parts) -> f64| Spread::of(drew.iter().map(|s| f(&s.parts)).collect());
        let parse_seconds = counters.parse_ns as f64 / 1e9;
        Report {
            format: FORMAT,
            format_version: FORMAT_VERSION,
            seconds: round(seconds),
            terminals,
            frames: Frames {
                count: frames.len(),
                terminal_frames: drew.len(),
                per_second: round(per(samples.len() as f64, seconds)),
                draw_ms: spread(&|s| ms(s.draw_ns)),
                allocations: spread(&|s| s.allocations as f64),
                alloc_kib: spread(&|s| s.alloc_bytes as f64 / 1024.0),
                lock_wait_ms: spread(&|s| ms(s.lock_wait_ns)),
                parts_ms: Part::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, part)| (part.name(), terminal(&|p| ms(p.nanos[i]))))
                    .collect(),
                parts_allocations: Part::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, part)| (part.name(), terminal(&|p| p.allocations[i] as f64)))
                    .collect(),
                rows_built: terminal(&|p| p.rows_built as f64),
                words_shaped: terminal(&|p| p.words_shaped as f64),
            },
            reader: Reader {
                bytes: counters.bytes,
                mb_per_s: round(per(counters.bytes as f64 / 1e6, seconds)),
                parse_mb_per_s: round(per(counters.bytes as f64 / 1e6, parse_seconds)),
                parse_busy_pct: round(100.0 * per(parse_seconds, seconds)),
                slices: counters.slices,
                lock: counters.reader.into(),
            },
            ui_lock: counters.ui.into(),
        }
    }

    /// The report as a short table.
    pub(crate) fn summary(&self) -> String {
        let f = &self.frames;
        let mut out = format!(
            "perf-profile {:.1} s: {} frames ({:.1}/s), {} drew the terminal; {} hosted and {} local terminals\n",
            self.seconds,
            f.count,
            f.per_second,
            f.terminal_frames,
            self.terminals.hosted,
            self.terminals.local
        );
        let _ = writeln!(out, "{:<22}{:>10}{:>10}{:>10}{:>10}", "per frame", "avg", "p50", "p95", "max");
        let mut row = |name: &str, s: &Spread| {
            let _ = writeln!(out, "{name:<22}{:>10.3}{:>10.3}{:>10.3}{:>10.3}", s.avg, s.p50, s.p95, s.max);
        };
        row("draw ms", &f.draw_ms);
        for ((name, spread), part) in f.parts_ms.iter().zip(Part::ALL) {
            row(&format!("{}{name} ms", nested(part)), spread);
        }
        row("  lock wait ms", &f.lock_wait_ms);
        row("rows built", &f.rows_built);
        row("words shaped", &f.words_shaped);
        row("allocations", &f.allocations);
        for ((name, spread), part) in f.parts_allocations.iter().zip(Part::ALL) {
            row(&format!("{}{name}", nested(part)), spread);
        }
        row("allocated KiB", &f.alloc_kib);
        let r = &self.reader;
        let _ = writeln!(
            out,
            "reader: {:.1} MB ({:.1} MB/s), parser {:.1} MB/s, parsing {:.1}% of the time, {} slices",
            r.bytes as f64 / 1e6,
            r.mb_per_s,
            r.parse_mb_per_s,
            r.parse_busy_pct,
            r.slices
        );
        for (who, lock) in [("reader", &r.lock), ("ui", &self.ui_lock)] {
            let _ = writeln!(
                out,
                "{who} lock: {} taken, waited {:.3} ms (max {:.3}), held {:.3} ms (max {:.3})",
                lock.locks, lock.wait_ms, lock.wait_max_ms, lock.hold_ms, lock.hold_max_ms
            );
        }
        out
    }
}

/// Pairs as a JSON object, in their order.
fn as_map<S: serde::Serializer>(pairs: &[(&'static str, Spread)], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_map(pairs.iter().map(|(name, spread)| (*name, spread)))
}

/// The indent of a part's line in the summary: one step under the frame, one more per level.
fn nested(part: Part) -> String {
    "  ".repeat(part.depth() + 1)
}

fn ms(nanos: u64) -> f64 {
    round(nanos as f64 / 1e6)
}

/// `amount` per `seconds`, 0 for no time.
fn per(amount: f64, seconds: f64) -> f64 {
    if seconds > 0.0 { amount / seconds } else { 0.0 }
}

/// Four decimals: enough for a microsecond in milliseconds, and a diff of two reports stays legible.
fn round(value: f64) -> f64 {
    (value * 1e4).round() / 1e4
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wormhole_core::terminal::perf::{Counters, LockUse};

    use super::{Report, Sample, Spread, Terminals, WARM_UP_FRAMES};
    use crate::perf::{Part, Parts};

    fn frame(draw_ms: u64, terminal: bool) -> Sample {
        let mut parts = Parts::default();
        if terminal {
            parts.terminals = 1;
            parts.nanos[Part::Paint as usize] = 500_000;
            parts.allocations[Part::Rows as usize] = 7;
            parts.rows_built = 2;
        }
        Sample { draw_ns: draw_ms * 1_000_000, allocations: 10, alloc_bytes: 2048, lock_wait_ns: 0, parts }
    }

    #[test]
    fn a_spread_has_the_average_the_nearest_rank_percentiles_and_the_maximum() {
        let spread = Spread::of((1..=100).map(f64::from).collect());
        assert_eq!(spread, Spread { avg: 50.5, p50: 50.0, p95: 95.0, max: 100.0 });
        assert_eq!(Spread::of(vec![3.0]), Spread { avg: 3.0, p50: 3.0, p95: 3.0, max: 3.0 });
        assert_eq!(Spread::of(Vec::new()), Spread::default(), "no frames, all zero");
    }

    #[test]
    fn the_warm_up_frames_are_left_out_and_the_terminals_parts_count_only_where_it_drew() {
        let mut samples: Vec<Sample> = (0..WARM_UP_FRAMES).map(|_| frame(100, true)).collect();
        samples.extend([frame(2, true), frame(4, false), frame(2, true), frame(4, false)]);
        let report = Report::new(&samples, Duration::from_secs(1), Counters::default(), Terminals::default());
        let frames = &report.frames;
        assert_eq!((frames.count, frames.terminal_frames), (4, 2));
        assert_eq!(frames.per_second, samples.len() as f64, "every frame drawn counts for the rate");
        assert_eq!(frames.draw_ms.avg, 3.0);
        assert_eq!(frames.draw_ms.max, 4.0, "the warm-up's 100 ms are not in it");
        assert_eq!(frames.alloc_kib.avg, 2.0);
        let paint = frames.parts_ms.iter().find(|(name, _)| *name == "paint").expect("paint");
        assert_eq!(paint.1.avg, 0.5, "averaged over the two frames that drew the terminal");
        let rows = frames.parts_allocations.iter().find(|(name, _)| *name == "rows").expect("rows");
        assert_eq!(rows.1.avg, 7.0);
        assert_eq!(frames.rows_built.avg, 2.0);
    }

    #[test]
    fn the_readers_speeds_come_from_the_profiles_time_and_the_parsers_time() {
        let counters = Counters {
            bytes: 50_000_000,
            slices: 3052,
            parse_ns: 250_000_000,
            reader: LockUse {
                locks: 3052,
                wait_ns: 2_000_000,
                wait_max_ns: 40_000,
                hold_ns: 260_000_000,
                hold_max_ns: 300_000,
            },
            ui: LockUse { locks: 600, wait_ns: 1_500_000, ..LockUse::default() },
        };
        let report = Report::new(&[], Duration::from_secs(2), counters, Terminals::default());
        assert_eq!(report.reader.mb_per_s, 25.0);
        assert_eq!(report.reader.parse_mb_per_s, 200.0);
        assert_eq!(report.reader.parse_busy_pct, 12.5);
        assert_eq!((report.reader.lock.wait_ms, report.reader.lock.hold_max_ms), (2.0, 0.3));
        assert_eq!((report.ui_lock.locks, report.ui_lock.wait_ms), (600, 1.5));
    }

    #[test]
    fn the_json_names_the_format_and_the_terminals_and_the_summary_nests_every_part() {
        let report = Report::new(
            &[frame(2, true); 6],
            Duration::from_secs(1),
            Counters::default(),
            Terminals { hosted: 1, local: 0 },
        );
        let json = serde_json::to_value(&report).expect("serializes");
        assert_eq!(json["format"], "wormhole-perf-profile");
        assert_eq!(json["frames"]["draw_ms"]["avg"], 2.0);
        assert_eq!(json["frames"]["parts_ms"]["paint"]["avg"], 0.5);
        let summary = report.summary();
        for part in Part::ALL {
            assert!(summary.contains(&format!("{} ms", part.name())), "{summary}");
        }
        assert!(summary.contains("\n      shape ms"), "shaping sits under rows, under prepare: {summary}");
        assert_eq!(
            (json["terminals"]["hosted"].as_u64(), json["terminals"]["local"].as_u64()),
            (Some(1), Some(0))
        );
        assert!(summary.contains("1 hosted and 0 local terminals"), "{summary}");
        assert!(summary.starts_with("perf-profile 1.0 s: 3 frames"), "{summary}");
    }

    #[test]
    fn nothing_drawn_and_nothing_read_is_a_report_of_zeros_not_a_division_by_zero() {
        let report = Report::new(&[], Duration::ZERO, Counters::default(), Terminals::default());
        assert_eq!(report.frames.per_second, 0.0);
        assert_eq!(report.reader.parse_mb_per_s, 0.0);
        assert!(report.summary().contains("0 frames"));
    }
}
