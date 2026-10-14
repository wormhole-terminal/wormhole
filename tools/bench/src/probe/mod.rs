//! `ttyprobe`: a feature check. It asks the terminal what it supports (XTVERSION, DA1, DECRQM, the
//! Kitty and iTerm2 protocols, colors, sizes), measures how many cells it draws a set of Unicode
//! texts in, shows a result screen worth a screenshot, and saves a file to compare terminals with.
//! Every exchange is kept so a surprising result can be traced; see `--details` and `--save`.

mod compare;
mod details;
mod features;
mod findings;
mod model;
mod query;
mod report;
mod sample;
mod widths;

use std::io::{self, Write};
use std::path::PathBuf;

use crate::bench::sysinfo;
use crate::term::{self, AltScreen, Input, RawMode};
use model::ProbeResults;
use query::Prober;

pub use compare::{compare, is_probe_file};

pub const ABOUT: &str = "what the terminal supports and how wide it draws text";

pub struct ProbeOptions {
    pub plain: bool,
    pub save: Option<PathBuf>,
    pub label: Option<String>,
    /// Print the sample page after the results.
    pub sample: bool,
    /// Print the transcript of every query and the findings after the results.
    pub details: bool,
    /// Dump every byte received, for debugging the probe itself.
    pub raw: Option<RawTarget>,
}

pub enum RawTarget {
    Stderr,
    File(PathBuf),
}

pub fn run(opts: &ProbeOptions) -> io::Result<()> {
    let terminal = term::terminal_name();
    let label = opts.label.clone().unwrap_or_else(|| terminal.clone());
    let machine = sysinfo::machine();
    let date = sysinfo::today();

    let (probed, widths, size, events, raw_log) = {
        let _raw = RawMode::enter()?;
        let _screen = AltScreen::enter()?;
        let mut input = Input::start();
        if opts.raw.is_some() {
            input.record_raw();
        }
        let size = term::size();
        let mut p = Prober::new(&mut input, io::stdout().lock());
        let probed = features::probe(&mut p, size)?;
        let widths = widths::measure(&mut p, probed.mode_2027)?;
        p.finish();
        let events = std::mem::take(&mut p.events);
        (probed, widths, size, events, input.raw_log().to_vec())
    };

    let mut results = ProbeResults {
        label,
        terminal,
        cols: size.cols,
        rows: size.rows,
        machine,
        date,
        version: env!("CARGO_PKG_VERSION").to_string(),
        features: probed.rows,
        widths,
        findings: Vec::new(),
        events,
    };
    results.findings = findings::findings(&results);

    if let Some(path) = &opts.save {
        std::fs::write(path, results.to_json())
            .map_err(|e| io::Error::other(format!("can't write {}: {e}", path.display())))?;
    }
    let saved = opts.save.as_ref().map(|p| p.display().to_string());
    let mut out = Vec::new();
    if opts.plain {
        out.extend_from_slice(report::results_plain(&results, saved.as_deref()).as_bytes());
    } else {
        report::results_panel(&results, report::panel_width(size.cols), saved.as_deref()).write(&mut out);
    }
    if opts.details {
        out.push(b'\n');
        out.extend_from_slice(details::details(&results).as_bytes());
    }
    if opts.sample && !opts.plain {
        out.extend_from_slice(sample::page().as_bytes());
    }
    io::stdout().write_all(&out)?;
    io::stdout().flush()?;

    match &opts.raw {
        Some(RawTarget::Stderr) => io::stderr().write_all(details::raw_dump(&raw_log).as_bytes())?,
        Some(RawTarget::File(path)) => std::fs::write(path, details::raw_dump(&raw_log))
            .map_err(|e| io::Error::other(format!("can't write {}: {e}", path.display())))?,
        None => {}
    }
    Ok(())
}
