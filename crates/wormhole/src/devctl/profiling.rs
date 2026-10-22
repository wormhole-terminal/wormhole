//! `perf-profile <seconds> [name]`: what this window's frames and every terminal's reading cost
//! over a period (`crate::perf`; `repo-docs/docs/performance.md`, "Profiling").
//!
//! It draws the window again and again for the period, as `frames` does (forced draws measure the
//! same whether or not the window can be seen), and records each draw: its time, the UI thread's
//! allocations, the terminal's parts. The terminal layer counts the readers and the lock meanwhile.
//! The report goes to `<dir>/<name>.json` and `<dir>/<name>.txt` (name `perf-profile` by default),
//! and its table to the results file once the period is over: the rest of the batch waits for it.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow};
use gpui_kit::*;
use wormhole_core::fsutil::{LinkPolicy, write_atomic};

use super::{Ctx, file_name};
use crate::perf::allocations::thread_counts;
use crate::perf::report::{Report, Sample};
use crate::workspace::Workspace;

/// The longest a profile may run.
const MAX_SECONDS: f64 = 600.0;
/// The pause after each draw, as `frames` makes: the app handles input and output between them.
const BREATHER: Duration = Duration::from_millis(2);

pub(super) fn perf_profile(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let seconds = match words.next() {
        None => 5.0,
        Some(n) => n.parse::<f64>().map_err(|_| anyhow!("expected the seconds to record, got {n:?}"))?,
    };
    if !(0.1..=MAX_SECONDS).contains(&seconds) {
        return Err(anyhow!("record between 0.1 and {MAX_SECONDS} seconds, not {seconds}"));
    }
    let name = file_name(words.next().unwrap_or(""), "perf-profile")?.to_string();
    let dir = ctx.dir.to_path_buf();
    let period = Duration::from_secs_f64(seconds);
    ctx.wait = Some(ctx.cx.spawn_in(ctx.window, async move |this, cx| {
        crate::perf::start();
        let recorded = record(this, cx, period).await;
        let counters = crate::perf::stop();
        match recorded.and_then(|(samples, elapsed)| {
            let report = Report::new(&samples, elapsed, counters);
            write(&report, &dir, &name)?;
            Ok(report.summary())
        }) {
            Ok(summary) => summary,
            Err(err) => format!("perf-profile failed: {err:#}\n"),
        }
    }));
    Ok(String::new())
}

/// Draw the window for `period`, one [`Sample`] a draw; and how long that took in all.
async fn record(
    this: WeakEntity<Workspace>,
    cx: &mut AsyncWindowContext,
    period: Duration,
) -> Result<(Vec<Sample>, Duration)> {
    let started = Instant::now();
    let mut samples = Vec::new();
    while started.elapsed() < period {
        this.update(cx, |_, cx| cx.notify()).map_err(|_| anyhow!("the window closed"))?;
        let sample = cx.update(|window, cx| {
            crate::perf::begin_frame();
            let waited = wormhole_core::terminal::perf::read().ui.wait_ns;
            let allocated = thread_counts();
            let start = Instant::now();
            let arena = window.draw(cx);
            arena.clear(cx);
            let draw = start.elapsed();
            let allocated = thread_counts().since(allocated);
            Sample {
                draw_ns: u64::try_from(draw.as_nanos()).unwrap_or(u64::MAX),
                allocations: allocated.allocations,
                alloc_bytes: allocated.bytes,
                lock_wait_ns: wormhole_core::terminal::perf::read().ui.wait_ns.saturating_sub(waited),
                parts: crate::perf::take_frame(),
            }
        });
        samples.push(sample.map_err(|_| anyhow!("the window closed"))?);
        cx.background_executor().timer(BREATHER).await;
    }
    Ok((samples, started.elapsed()))
}

/// The report as `<name>.json` and its table as `<name>.txt`, each written whole (a script waits
/// for the JSON to appear).
fn write(report: &Report, dir: &std::path::Path, name: &str) -> Result<()> {
    let json = serde_json::to_vec_pretty(report).context("the report as JSON")?;
    let path: PathBuf = dir.join(format!("{name}.json"));
    let text = dir.join(format!("{name}.txt"));
    write_atomic(&text, report.summary().as_bytes(), LinkPolicy::Follow)
        .with_context(|| format!("writing {}", text.display()))?;
    write_atomic(&path, &json, LinkPolicy::Follow).with_context(|| format!("writing {}", path.display()))
}
