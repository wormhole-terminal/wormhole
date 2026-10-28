//! Throughput and latency benchmarks, the same bytes sent to any terminal, with a result screen
//! worth a screenshot and a file to compare terminals with. See `run` and `compare`.

mod compare;
mod compose;
mod details;
mod diag;
mod flood;
mod integrity;
pub(crate) mod json;
mod procinfo;
mod report;
mod results;
mod runner;
mod stats;
pub(crate) mod sysinfo;
pub(crate) mod ui;
mod workloads;

use std::io::{self, Write};
use std::path::PathBuf;

use crate::term::{self, AltScreen, Input, RawMode};
use results::Results;
use runner::{Runner, Step};

pub use flood::{FloodOptions, run as flood};
pub use workloads::{NAMES, about, current_name};

pub struct BenchOptions {
    pub size_mb: f64,
    pub only: Vec<String>,
    pub save: Option<PathBuf>,
    pub label: Option<String>,
    /// Runs per throughput test; the median is reported.
    pub runs: usize,
    /// No status line during the run and the results as plain text, not the painted panel.
    pub plain: bool,
    /// After the results, a text section per test on where the time went and what to look at
    /// (`--details`).
    pub details: bool,
    /// Every write of every measured run, as JSON Lines (`--trace <file>`).
    pub trace: Option<PathBuf>,
    /// About how many bytes apart the input check's checkpoints are (`--verify-every <KiB>`); `None`
    /// turns the check off (`--no-verify`).
    pub verify_every: Option<usize>,
    /// Where the checkpoints' random places start (`--verify-seed <n>`); `None` draws one from the
    /// clock. The same seed with the same options puts them in the same places.
    pub verify_seed: Option<u64>,
}

/// The input check's default distance between checkpoints, in bytes.
pub const DEFAULT_VERIFY_EVERY: usize = integrity::DEFAULT_EVERY;
/// The largest `--verify-seed`.
pub const MAX_VERIFY_SEED: u64 = integrity::MAX_SEED;

/// The tests in a named group (`realistic`, `stress` or `all`), in running order.
pub fn suite(name: &str) -> Option<Vec<&'static str>> {
    workloads::suite(name)
}

/// The tests to run, in running order, or why the request is wrong. `only` takes test names and the
/// groups `realistic`, `stress` and `all`.
fn selected(only: &[String]) -> Result<Vec<&'static str>, String> {
    let mut wanted: Vec<&str> = Vec::new();
    for o in only {
        match suite(o) {
            Some(group) => wanted.extend(group),
            None if workloads::NAMES.contains(&o.as_str()) => wanted.push(o),
            None => {
                return Err(format!(
                    "no test called `{o}` (they are: {}; or a group: realistic, stress, all)",
                    workloads::NAMES.join(", ")
                ));
            }
        }
    }
    Ok(workloads::NAMES.iter().copied().filter(|n| only.is_empty() || wanted.contains(n)).collect())
}

/// Standard output without std's line buffering: each write of a workload reaches the pty as one
/// write, as the workload meant it (a token-sized write with a newline in it would otherwise be two).
pub(crate) fn unbuffered_stdout() -> io::Result<std::fs::File> {
    use std::os::fd::AsFd;
    Ok(std::fs::File::from(io::stdout().as_fd().try_clone_to_owned()?))
}

pub fn run(opts: &BenchOptions) -> io::Result<()> {
    if !(0.05..=4096.0).contains(&opts.size_mb) {
        return Err(io::Error::other("--size must be between 0.05 and 4096 MB"));
    }
    if !(1..=100).contains(&opts.runs) {
        return Err(io::Error::other("--runs must be between 1 and 100"));
    }
    let names = selected(&opts.only).map_err(io::Error::other)?;
    let terminal = term::terminal_name();
    let label = opts.label.clone().unwrap_or_else(|| terminal.clone());
    let machine = sysinfo::machine();
    let date = sysinfo::today();
    // Found before raw mode and the alternate screen, which change nothing about the process tree.
    let process = procinfo::find_terminal();
    let trace = match &opts.trace {
        Some(path) => Some(std::io::BufWriter::new(
            std::fs::File::create(path)
                .map_err(|e| io::Error::other(format!("can't write {}: {e}", path.display())))?,
        )),
        None => None,
    };

    let (tests, size, interrupted, check_seed) = {
        let _raw = RawMode::enter()?;
        let _screen = AltScreen::enter()?;
        let mut input = Input::start();
        let size = term::size();
        let mut runner = Runner {
            input: &mut input,
            out: unbuffered_stdout()?,
            size,
            bytes: (opts.size_mb * 1e6) as usize,
            runs: opts.runs,
            plain: opts.plain,
            process: process.clone(),
            trace,
            verifier: integrity::Verifier::new(opts.verify_every, opts.verify_seed),
        };
        if let (Some(trace), Some(seed), Some(every)) =
            (&mut runner.trace, runner.verifier.seed(), opts.verify_every)
        {
            diag::write_trace_header(trace, seed, every)?;
        }
        let mut tests = Vec::new();
        let mut interrupted = false;
        for (i, name) in names.iter().enumerate() {
            match runner.run_test(name, i, names.len())? {
                Step::Done(t) => tests.push(*t),
                Step::Interrupted => {
                    interrupted = true;
                    break;
                }
            }
        }
        if let Some(trace) = &mut runner.trace {
            trace.flush()?;
        }
        (tests, size, interrupted, runner.verifier.seed())
    };

    let results = Results {
        label,
        terminal,
        cols: size.cols,
        rows: size.rows,
        machine,
        date,
        version: env!("CARGO_PKG_VERSION").to_string(),
        size_mb: opts.size_mb,
        runs: opts.runs,
        check_every: opts.verify_every.map(|n| n as u64),
        check_seed,
        process,
        tests,
    };
    if let Some(path) = &opts.save {
        std::fs::write(path, results.to_json())
            .map_err(|e| io::Error::other(format!("can't write {}: {e}", path.display())))?;
    }
    if results.tests.is_empty() {
        return Err(io::Error::other("interrupted before the first test finished"));
    }
    let saved = opts.save.as_ref().map(|p| p.display().to_string());
    let mut out = Vec::new();
    if opts.plain {
        out.extend_from_slice(report::results_plain(&results, saved.as_deref()).as_bytes());
    } else {
        report::results_panel(&results, report::panel_width(size.cols), saved.as_deref()).write(&mut out);
    }
    if interrupted {
        out.extend_from_slice(b"  interrupted: only the finished tests are shown\n");
    }
    if opts.details {
        out.extend_from_slice(details::details(&results).as_bytes());
    }
    io::stdout().write_all(&out)?;
    io::stdout().flush()
}

/// The saved results side by side: the terminal panel, or with `markdown` a table to paste.
pub fn compare(files: &[PathBuf], markdown: bool) -> io::Result<()> {
    if files.len() < 2 {
        return Err(io::Error::other("compare needs at least two result files (from `bench --save`)"));
    }
    let mut all = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(f)
            .map_err(|e| io::Error::other(format!("can't read {}: {e}", f.display())))?;
        all.push(Results::from_json(&text).map_err(|e| io::Error::other(format!("{}: {e}", f.display())))?);
    }
    let mut out = Vec::new();
    if markdown {
        out.extend_from_slice(compare::compare_markdown(&all).as_bytes());
    } else {
        compare::compare_panel(&all, panel_width_for_compare()).write(&mut out);
    }
    io::stdout().write_all(&out)?;
    io::stdout().flush()
}

/// stdout may be a pipe (no size to ask): the default is then the screenshot width.
fn panel_width_for_compare() -> usize {
    report::panel_width(term::size().cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_picks_tests_in_running_order() {
        let all = selected(&[]).unwrap_or_default();
        assert_eq!(all.len(), workloads::NAMES.len());
        let some = selected(&["roundtrip".to_string(), "logflood".to_string()]).unwrap_or_default();
        assert_eq!(some, vec!["logflood", "roundtrip"]);
    }

    #[test]
    fn an_unknown_test_is_an_error_naming_the_choices() {
        let e = selected(&["asci".to_string()]).err().unwrap_or_default();
        assert!(e.contains("asci") && e.contains("scrollstorm") && e.contains("realistic"), "{e}");
    }

    #[test]
    fn only_takes_groups() {
        let realistic = selected(&["realistic".to_string()]).unwrap_or_default();
        assert_eq!(
            realistic,
            vec!["buildlog", "agentstream", "agentredraw", "agentscreen", "diffscroll", "editstorm"]
        );
        let stress = selected(&["stress".to_string()]).unwrap_or_default();
        assert!(
            stress.contains(&"logflood") && stress.contains(&"screenswap") && !stress.contains(&"roundtrip")
        );
        assert!(!stress.contains(&"buildlog"));
        assert_eq!(selected(&["all".to_string()]).unwrap_or_default().len(), workloads::NAMES.len());
        let mixed = selected(&["roundtrip".to_string(), "realistic".to_string()]).unwrap_or_default();
        assert_eq!(mixed.len(), 7);
        assert_eq!(mixed[6], "roundtrip");
    }

    #[test]
    fn the_suites_cover_every_test_once() {
        let mut all = suite("realistic").unwrap_or_default();
        all.extend(suite("stress").unwrap_or_default());
        all.push("roundtrip");
        all.sort_unstable();
        let mut names = NAMES.to_vec();
        names.sort_unstable();
        assert_eq!(all, names);
        assert!(suite("nope").is_none());
    }
}
