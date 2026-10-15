//! vt-bench: how fast each terminal engine takes in each corpus (`crates/wormhole-vt-bench`'s
//! generators and recordings), and what that costs in allocations. The same method as `vtbench`,
//! over more engines: Ghostty (libghostty-vt), WezTerm's model, vt100 and alacritty_terminal.
//!
//! ```sh
//! cargo run --release --bin vt-bench                       # every engine, every corpus
//! cargo run --release --bin vt-bench -- --markdown         # tables for a document
//! cargo run --release --bin vt-bench -- --json > a.json
//! cargo run --release --bin vt-bench -- --engine ghostty --engine alacritty --corpus tui-redraw --runs 9
//! ```
//!
//! The speed is the fastest of `--runs` passes feeding the stream in `--chunk` pieces to a fresh
//! engine; `allocs/MB` counts allocator calls per megabyte over a whole first pass, `steady/MB`
//! over a second pass on the same engine once everything is full. For the Rust engines that is the
//! global allocator; for Ghostty it is the allocator libghostty-vt is given (it also maps its page
//! memory directly, which no allocator sees). A debug build says nothing about speed: use `--release`.
// A command line tool: its output is the product, on stdout.
#![allow(clippy::print_stdout)]

#[path = "../../../../crates/wormhole-vt-bench/src/bin/vtbench/alloc.rs"]
mod alloc;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vt_compare::engines::{engine_info, engine_names, for_each_engine, unavailable, visit_engine};
use wormhole_vt_bench::Engine;
use wormhole_vt_bench::bench::{self, AllocCounts, Measurement, RunInfo, Settings};
use wormhole_vt_bench::corpus::{self, Corpus};
use wormhole_vt_bench::engines::EngineVisitor;

#[global_allocator]
static ALLOCATOR: alloc::Counting = alloc::Counting;

const USAGE: &str = "\
usage: vt-bench [options]
  --engine <name>      measure only this engine (repeatable; default: all, see --list)
  --corpus <name>      measure only this corpus (repeatable; default: all, see --list)
  --size <MiB>         size of each generated corpus (default 8)
  --chunk <KiB>        size of each feed call (default 64)
  --runs <n>           passes per measurement, the fastest counts (default 5)
  --scrollback <n>     lines of history (default 10000)
  --seed <n>           seed of the generators (default 1)
  --recorded <dir>     also measure the *.raw recordings in this folder
                       (default: tools/vt-corpus when it exists)
  --no-recorded        do not measure recordings
  --markdown           print markdown tables (MB/s, then allocations) instead of vtbench's table
  --json               print the results as JSON on stdout
  --save <file>        also write the results as JSON to this file, whatever is printed
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
    markdown: bool,
    json: bool,
    save: Option<PathBuf>,
    list: bool,
}

fn parse_args() -> Result<Option<Options>, String> {
    let default_recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vt-corpus");
    let mut o = Options {
        engines: Vec::new(),
        corpora: Vec::new(),
        size_mib: 8,
        settings: Settings { chunk: 64 * 1024, runs: 5, scrollback: 10_000 },
        seed: 1,
        recorded: default_recorded.is_dir().then_some(default_recorded),
        markdown: false,
        json: false,
        save: None,
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
            "--markdown" => o.markdown = true,
            "--json" => o.json = true,
            "--save" => o.save = Some(PathBuf::from(value("a file")?)),
            "--list" => o.list = true,
            "-h" | "--help" => return Ok(None),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(Some(o))
}

/// The `*.raw` files of a folder as corpora, with the size from their `.meta` (100x30 without),
/// as vtbench reads them.
fn recorded(dir: &Path) -> Vec<Corpus> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut raws: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "raw"))
        .collect();
    raws.sort();
    raws.into_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let meta = std::fs::read_to_string(path.with_extension("meta")).unwrap_or_default();
            let field = |key: &str| {
                meta.lines()
                    .find_map(|l| l.strip_prefix(key)?.strip_prefix('=')?.trim().parse::<usize>().ok())
            };
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(Corpus {
                name: name.into(),
                description: "recorded with scripts/vt-record.sh",
                cols: field("cols").unwrap_or(100),
                rows: field("rows").unwrap_or(30),
                bytes,
            })
        })
        .collect()
}

/// Allocator calls so far: Rust's, plus the ones libghostty-vt made through its allocator.
fn counts() -> AllocCounts {
    let rust = alloc::counts();
    #[cfg(has_ghostty)]
    let (allocs, bytes) = vt_compare::engines::ghostty::alloc_counts();
    #[cfg(not(has_ghostty))]
    let (allocs, bytes) = (0, 0);
    AllocCounts { allocs: rust.allocs + allocs, bytes: rust.bytes + bytes }
}

struct Run<'a> {
    corpora: &'a [Corpus],
    settings: Settings,
    quiet: bool,
    results: Vec<Measurement>,
}

impl EngineVisitor for Run<'_> {
    fn visit<E: Engine>(&mut self) {
        for corpus in self.corpora {
            if !self.quiet {
                eprintln!("measuring {} on {} ...", E::NAME, corpus.name);
            }
            self.results.push(bench::measure::<E>(corpus, &self.settings, counts));
        }
    }
}

/// The names in the order they were measured, once each.
fn distinct<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for name in names {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Two markdown tables: MB/s per corpus and engine (the best of each row in bold, a last row with
/// the geometric mean of the speed relative to the first engine), and allocator calls per MB.
fn markdown(results: &[Measurement]) -> String {
    let engines = distinct(results.iter().map(|r| r.engine));
    let corpora = distinct(results.iter().map(|r| r.corpus.as_str()));
    let find = |corpus: &str, engine: &str| results.iter().find(|r| r.corpus == corpus && r.engine == engine);
    let header = |title: &str| {
        let mut out = format!("| {title} |");
        for e in &engines {
            out.push_str(&format!(" {e} |"));
        }
        out.push_str("\n|---|");
        for _ in &engines {
            out.push_str("--:|");
        }
        out.push('\n');
        out
    };

    let mut out = String::from("### Throughput, MB/s (the fastest of the passes)\n\n");
    out.push_str(&header("corpus"));
    let mut ratios: Vec<Vec<f64>> = vec![Vec::new(); engines.len()];
    for corpus in &corpora {
        let speeds: Vec<Option<f64>> =
            engines.iter().map(|e| find(corpus, e).map(Measurement::mb_per_s)).collect();
        let best = speeds.iter().flatten().copied().fold(0.0, f64::max);
        out.push_str(&format!("| {corpus} |"));
        for (i, speed) in speeds.iter().enumerate() {
            match speed {
                Some(s) if *s >= best => out.push_str(&format!(" **{s:.0}** |")),
                Some(s) => out.push_str(&format!(" {s:.0} |")),
                None => out.push_str(" – |"),
            }
            if let (Some(s), Some(Some(base))) = (speed, speeds.first()) {
                ratios[i].push(s / base);
            }
        }
        out.push('\n');
    }
    out.push_str("| **vs the first engine** |");
    for r in &ratios {
        if r.is_empty() {
            out.push_str(" – |");
        } else {
            let geo = (r.iter().map(|v| v.ln()).sum::<f64>() / r.len() as f64).exp();
            out.push_str(&format!(" {geo:.2}x |"));
        }
    }
    out.push_str("\n\n### Allocator calls per MB: the first pass / a second pass on the same engine\n\n");
    out.push_str(&header("corpus"));
    for corpus in &corpora {
        out.push_str(&format!("| {corpus} |"));
        for e in &engines {
            match find(corpus, e) {
                Some(m) => {
                    out.push_str(&format!(" {:.0} / {:.0} |", m.allocs_per_mb, m.steady_allocs_per_mb))
                }
                None => out.push_str(" – |"),
            }
        }
        out.push('\n');
    }
    out
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("vt-bench: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if options.list {
        for name in engine_names() {
            println!("engine: {name}: {}", engine_info(name));
        }
        for (name, why) in unavailable() {
            println!("engine: {name}: not built, {why}");
        }
        for name in corpus::names() {
            println!("corpus: {name}");
        }
        return ExitCode::SUCCESS;
    }
    for name in &options.engines {
        if !engine_names().contains(&name.as_str()) {
            eprintln!("vt-bench: no engine called '{name}' (engines: {})", engine_names().join(" "));
            return ExitCode::from(2);
        }
    }
    let size = options.size_mib * 1024 * 1024;
    let mut corpora: Vec<Corpus> = if options.corpora.is_empty() {
        corpus::all(size, options.seed)
    } else {
        let mut chosen = Vec::new();
        for name in &options.corpora {
            match corpus::generate(name, size, options.seed) {
                Some(c) => chosen.push(c),
                None => {
                    eprintln!("vt-bench: no corpus called '{name}' (see --list)");
                    return ExitCode::from(2);
                }
            }
        }
        chosen
    };
    if options.corpora.is_empty()
        && let Some(dir) = &options.recorded
    {
        corpora.extend(recorded(dir));
    }
    if cfg!(debug_assertions) && !options.json {
        eprintln!("vt-bench: this is a debug build; the speeds mean nothing (use --release)");
    }

    let mut run =
        Run { corpora: &corpora, settings: options.settings, quiet: options.json, results: Vec::new() };
    if options.engines.is_empty() {
        for_each_engine(&mut run);
    } else {
        for name in &options.engines {
            visit_engine(name, &mut run);
        }
    }

    let info =
        RunInfo { settings: options.settings, size, seed: options.seed, release: !cfg!(debug_assertions) };
    let mut value = bench::json(&run.results, &info);
    if let Some(object) = value.as_object_mut() {
        let engines = distinct(run.results.iter().map(|r| r.engine));
        object.insert(
            "engines".into(),
            engines.iter().map(|e| serde_json::json!({ "name": e, "what": engine_info(e) })).collect(),
        );
    }
    let json_text = match serde_json::to_string_pretty(&value) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("vt-bench: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(path) = &options.save
        && let Err(e) = std::fs::write(path, format!("{json_text}\n"))
    {
        eprintln!("vt-bench: cannot write {}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    if options.json {
        println!("{json_text}");
    } else if options.markdown {
        print!("{}", markdown(&run.results));
    } else {
        print!("{}", bench::table(&run.results));
    }
    ExitCode::SUCCESS
}
