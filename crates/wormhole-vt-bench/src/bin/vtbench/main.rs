//! vtbench: how fast each terminal engine takes in each corpus, and what that costs in
//! allocations.
//!
//! ```sh
//! cargo run --release -p wormhole-vt-bench --bin vtbench                     # every engine, every corpus
//! cargo run --release -p wormhole-vt-bench --bin vtbench -- --json > a.json  # a baseline to store
//! cargo run --release -p wormhole-vt-bench --bin vtbench -- --corpus tui-redraw --runs 9
//! ```
//!
//! The speed is the fastest of `--runs` passes feeding the stream in `--chunk` pieces to a fresh
//! engine; `allocs/MB` counts allocator calls per megabyte over a whole first pass (the screen and
//! history grow during it), `steady/MB` over a second pass on the same engine, once everything is
//! full. A debug build says nothing about speed: use `--release`.
// A command line tool: its output is the product, on stdout.
#![allow(clippy::print_stdout)]

mod alloc;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wormhole_vt_bench::Engine;
use wormhole_vt_bench::bench::{self, Measurement, RunInfo, Settings};
use wormhole_vt_bench::corpus::{self, Corpus, recorded, recorded_names};
use wormhole_vt_bench::engines::{ENGINE_NAMES, EngineVisitor, for_each_engine, visit_engine};

#[global_allocator]
static ALLOCATOR: alloc::Counting = alloc::Counting;

const USAGE: &str = "\
usage: vtbench [options]
  --engine <name>      measure only this engine (repeatable; default: all)
  --corpus <name>      measure only this corpus (repeatable; default: all, see --list)
  --size <MiB>         size of each generated corpus (default 8)
  --chunk <KiB>        size of each feed call (default 64)
  --runs <n>           passes per measurement, the fastest counts (default 5)
  --scrollback <n>     lines of history (default 10000)
  --seed <n>           seed of the generators (default 1)
  --recorded <dir>     also measure the *.raw recordings in this folder
                       (default: tools/vt-corpus when it exists; see scripts/vt-record.sh)
  --no-recorded        do not measure recordings
  --json               print the results as JSON on stdout instead of a table
  --list               list the engines and corpora
  -h, --help
";

struct Options {
    engines: Vec<String>,
    corpora: Vec<String>,
    size_mib: usize,
    settings: Settings,
    seed: u64,
    recorded: Option<PathBuf>,
    json: bool,
    list: bool,
}

fn parse_args() -> Result<Option<Options>, String> {
    let default_recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vt-corpus");
    let mut o = Options {
        engines: Vec::new(),
        corpora: Vec::new(),
        size_mib: 8,
        settings: Settings { chunk: 64 * 1024, runs: 5, scrollback: 10_000 },
        seed: 1,
        recorded: default_recorded.is_dir().then_some(default_recorded),
        json: false,
        list: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |what: &str| args.next().ok_or_else(|| format!("{arg} needs {what}"));
        let number = |text: String, what: &str| {
            text.parse::<usize>().map_err(|_| format!("{what}: '{text}' is not a number"))
        };
        match arg.as_str() {
            "--engine" => o.engines.push(value("a name")?),
            "--corpus" => o.corpora.push(value("a name")?),
            "--size" => o.size_mib = number(value("a number of MiB")?, "--size")?.max(1),
            "--chunk" => o.settings.chunk = number(value("a number of KiB")?, "--chunk")?.max(1) * 1024,
            "--runs" => o.settings.runs = number(value("a number")?, "--runs")?.max(1),
            "--scrollback" => o.settings.scrollback = number(value("a number")?, "--scrollback")?,
            "--seed" => o.seed = number(value("a number")?, "--seed")? as u64,
            "--recorded" => o.recorded = Some(PathBuf::from(value("a folder")?)),
            "--no-recorded" => o.recorded = None,
            "--json" => o.json = true,
            "--list" => o.list = true,
            "-h" | "--help" => return Ok(None),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(Some(o))
}

struct Run<'a> {
    corpora: &'a [Corpus],
    settings: Settings,
    json: bool,
    results: Vec<Measurement>,
}

impl EngineVisitor for Run<'_> {
    fn visit<E: Engine>(&mut self) {
        for corpus in self.corpora {
            if !self.json {
                eprintln!("measuring {} on {} ...", E::NAME, corpus.name);
            }
            self.results.push(bench::measure::<E>(corpus, &self.settings, alloc::counts));
        }
    }
}

/// The corpora to measure: everything (generated, then recorded) without `--corpus`, otherwise
/// the named ones in the order given, from either kind.
fn choose(options: &Options, size: usize) -> Result<Vec<Corpus>, String> {
    let recordings = options.recorded.as_deref();
    if options.corpora.is_empty() {
        let mut all = corpus::all(size, options.seed);
        all.extend(recordings.map(|dir| recorded(dir, None)).unwrap_or_default());
        return Ok(all);
    }
    let mut from_recordings = recordings.map(|dir| recorded(dir, Some(&options.corpora))).unwrap_or_default();
    let mut chosen = Vec::new();
    for name in &options.corpora {
        if let Some(c) = corpus::generate(name, size, options.seed) {
            chosen.push(c);
        } else if let Some(at) = from_recordings.iter().position(|c| c.name == name.as_str()) {
            chosen.push(from_recordings.remove(at));
        } else {
            return Err(format!("no corpus called '{name}' (see --list)"));
        }
    }
    Ok(chosen)
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("vtbench: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if options.list {
        println!("engines: {}", ENGINE_NAMES.join(" "));
        for name in corpus::names() {
            println!("corpus: {name}");
        }
        for name in options.recorded.iter().flat_map(|dir| recorded_names(dir)) {
            println!("corpus: {name} (recorded)");
        }
        return ExitCode::SUCCESS;
    }
    for name in &options.engines {
        if !ENGINE_NAMES.contains(&name.as_str()) {
            eprintln!("vtbench: no engine called '{name}' (engines: {})", ENGINE_NAMES.join(" "));
            return ExitCode::from(2);
        }
    }
    let size = options.size_mib * 1024 * 1024;
    let corpora = match choose(&options, size) {
        Ok(corpora) => corpora,
        Err(message) => {
            eprintln!("vtbench: {message}");
            return ExitCode::from(2);
        }
    };
    if cfg!(debug_assertions) && !options.json {
        eprintln!("vtbench: this is a debug build; the speeds mean nothing (use --release)");
    }

    let mut run =
        Run { corpora: &corpora, settings: options.settings, json: options.json, results: Vec::new() };
    if options.engines.is_empty() {
        for_each_engine(&mut run);
    } else {
        for name in &options.engines {
            visit_engine(name, &mut run);
        }
    }

    if options.json {
        let info = RunInfo {
            settings: options.settings,
            size,
            seed: options.seed,
            release: !cfg!(debug_assertions),
        };
        match serde_json::to_string_pretty(&bench::json(&run.results, &info)) {
            Ok(text) => println!("{text}"),
            Err(e) => {
                eprintln!("vtbench: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        print!("{}", bench::table(&run.results));
    }
    ExitCode::SUCCESS
}
