//! Throughput and latency benchmarks, the same bytes sent to any terminal, with a result screen
//! worth a screenshot and a file to compare terminals with. See `run` and `compare`.

mod compare;
mod flood;
mod json;
mod report;
mod results;
mod runner;
mod stats;
mod sysinfo;
mod ui;
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
}

/// The tests to run, in running order, or why the request is wrong.
fn selected(only: &[String]) -> Result<Vec<&'static str>, String> {
    if let Some(bad) = only.iter().find(|o| !workloads::NAMES.contains(&o.as_str())) {
        return Err(format!("no test called `{bad}` (they are: {})", workloads::NAMES.join(", ")));
    }
    Ok(workloads::NAMES.iter().copied().filter(|n| only.is_empty() || only.iter().any(|o| o == n)).collect())
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

    let (tests, size, interrupted) = {
        let _raw = RawMode::enter()?;
        let _screen = AltScreen::enter()?;
        let mut input = Input::start();
        let size = term::size();
        let mut runner = Runner {
            input: &mut input,
            out: io::stdout().lock(),
            size,
            bytes: (opts.size_mb * 1e6) as usize,
            runs: opts.runs,
            plain: opts.plain,
        };
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
        (tests, size, interrupted)
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
    io::stdout().write_all(&out)?;
    io::stdout().flush()
}

pub fn compare(files: &[PathBuf]) -> io::Result<()> {
    if files.len() < 2 {
        return Err(io::Error::other("compare needs at least two result files (from `bench --save`)"));
    }
    let mut all = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(f)
            .map_err(|e| io::Error::other(format!("can't read {}: {e}", f.display())))?;
        all.push(Results::from_json(&text).map_err(|e| io::Error::other(format!("{}: {e}", f.display())))?);
    }
    let width = panel_width_for_compare();
    let mut out = Vec::new();
    compare::compare_panel(&all, width).write(&mut out);
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
        assert!(e.contains("asci") && e.contains("scrollstorm"), "{e}");
    }
}
