//! The repaint's bisimulation over everything the tools have: every `.vtt` case cut at every byte,
//! every corpus at many cuts (inside escape sequences and UTF-8 characters among them), with and
//! without a resize at the cut. See `wormhole_vt_bench::bisim` for what is checked.
//!
//! Knobs, for a longer run or a closer look: `BISIM_CUTS=<n>` cuts per corpus (default 100),
//! `BISIM_SIZE=<KiB>` corpus size (default 128), `BISIM_FILTER=<text>` only the scenarios whose
//! name contains it, `BISIM_ALL=1` every resize variant on the `.vtt` cases too.

use std::path::Path;
use std::time::Instant;

use wormhole_vt_bench::bisim::{
    Failure, Plan, ResizeAtCut, Resolve, Scenario, Stats, check, every_byte, from_case, from_corpus, select,
    suffix,
};
use wormhole_vt_bench::corpus;
use wormhole_vt_bench::vtt;

fn env_number(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn filter() -> Option<String> {
    std::env::var("BISIM_FILTER").ok().filter(|f| !f.is_empty())
}

fn cases() -> Vec<Scenario> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../wormhole-vt/tests/conformance");
    let cases = match vtt::load_dir(&dir) {
        Ok(cases) => cases,
        Err(errors) => panic!("the conformance files do not parse:\n{}", errors.join("\n")),
    };
    cases.iter().map(from_case).filter(|s| filter().is_none_or(|f| s.name.contains(&f))).collect()
}

fn corpora() -> Vec<Scenario> {
    let size = env_number("BISIM_SIZE", 128) * 1024;
    let mut all: Vec<_> = corpus::all(size, 1);
    let recordings = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vt-corpus");
    all.extend(corpus::recorded(&recordings, None));
    all.iter()
        .map(|c| from_corpus(c, 2000))
        .filter(|s| filter().is_none_or(|f| s.name.contains(&f)))
        .collect()
}

/// Report failures compactly: how many of each kind, and the smallest example of the first few.
fn report(what: &str, failures: &[Box<Failure>]) {
    if failures.is_empty() {
        return;
    }
    let mut kinds: Vec<(String, usize)> = Vec::new();
    for failure in failures {
        let kind = failure.kind();
        match kinds.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, n)) => *n += 1,
            None => kinds.push((kind, 1)),
        }
    }
    kinds.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    let mut text = format!("{} runs of {what} diverged; by the first thing that differed:\n", failures.len());
    for (kind, n) in &kinds {
        text.push_str(&format!("  {n:6}  {kind}\n"));
    }
    for (kind, _) in kinds.iter().take(8) {
        let smallest = failures.iter().filter(|f| f.kind() == *kind).min_by_key(|f| f.at + f.what.len());
        if let Some(failure) = smallest {
            text.push_str(&format!("\n{failure}\n"));
        }
    }
    panic!("{text}");
}

fn summary(what: &str, count: usize, total: &Stats, start: Instant) {
    eprintln!(
        "{count} {what}: {} repaints compared ({} cuts moved off a sequence, {} with a gap that steers later \
         bytes, compared at the cut only), {} KiB of repaint, gaps seen {:?}, {:.1}s",
        total.repaints,
        total.moved,
        total.sticky,
        total.repaint_bytes / 1024,
        total.gaps,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn every_vtt_case_at_every_byte() {
    let start = Instant::now();
    let all = std::env::var_os("BISIM_ALL").is_some();
    let mut failures = Vec::new();
    let mut total = Stats::default();
    let scenarios = cases();
    for scenario in &scenarios {
        let cuts = every_byte(scenario.bytes.len());
        for resolve in [Resolve::Defer, Resolve::Replay] {
            for resize in [ResizeAtCut::None, ResizeAtCut::After, ResizeAtCut::Before] {
                if resize != ResizeAtCut::None && !all && scenario.bytes.len() > 120 {
                    continue;
                }
                let mut plan = Plan::new(cuts.clone(), resolve);
                plan.resize = resize;
                match check(scenario, &plan) {
                    Ok(stats) => total.add(stats),
                    Err(failure) => failures.push(failure),
                }
            }
        }
    }
    summary(".vtt cases", scenarios.len(), &total, start);
    report("the .vtt cases", &failures);
}

#[test]
fn every_corpus_at_many_cuts() {
    let start = Instant::now();
    let count = env_number("BISIM_CUTS", 100);
    let mut failures = Vec::new();
    let mut total = Stats::default();
    let scenarios = corpora();
    for scenario in &scenarios {
        let cuts = select(&scenario.bytes, count, 1);
        for resolve in [Resolve::Defer, Resolve::Replay] {
            for resize in [ResizeAtCut::None, ResizeAtCut::After, ResizeAtCut::Before] {
                let mut plan = Plan::new(cuts.clone(), resolve);
                plan.resize = resize;
                // The resizes reflow a lot of history: fewer cuts keep the run short.
                if resize != ResizeAtCut::None {
                    plan.cuts = select(&scenario.bytes, count / 4, 2);
                }
                match check(scenario, &plan) {
                    Ok(stats) => total.add(stats),
                    Err(failure) => failures.push(failure),
                }
            }
        }
    }
    summary("corpora", scenarios.len(), &total, start);
    report("the corpora", &failures);
}

#[test]
fn every_corpus_to_the_end_of_the_stream() {
    let start = Instant::now();
    let mut failures = Vec::new();
    let mut total = Stats::default();
    let scenarios = corpora();
    for scenario in &scenarios {
        let cuts = select(&scenario.bytes, 12, 3);
        for resolve in [Resolve::Defer, Resolve::Replay] {
            match suffix(scenario, &cuts, resolve) {
                Ok(stats) => total.add(stats),
                Err(failure) => failures.push(failure),
            }
        }
    }
    summary("corpora run to the end", scenarios.len(), &total, start);
    report("the corpora run to the end", &failures);
}
