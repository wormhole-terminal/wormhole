//! vt-matrix: the conformance cases (`crates/wormhole-vt/tests/conformance/*.vtt`) run on every
//! engine, and who passes what.
//!
//! ```sh
//! cargo run --release --bin vt-matrix                  # a text report
//! cargo run --release --bin vt-matrix -- --markdown    # the same for a document or a pull request
//! cargo run --release --bin vt-matrix -- --json > m.json
//! cargo run --release --bin vt-matrix -- --engine ghostty --engine alacritty --filter wrap
//! ```
//!
//! A case an engine's adapter cannot ask (replies from an engine with no output, hyperlinks from
//! one that has none) counts as n/a for it, not as a failure.
// A command line tool: its output is the product, on stdout.
#![allow(clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vt_compare::engines::{engine_info, engine_names, for_each_engine, unavailable, visit_engine};
use vt_compare::matrix::{Matrix, Runner, load_cases, quiet_panics};
use vt_compare::render::{self, Detail, Format, Style};

const USAGE: &str = "\
usage: vt-matrix [options]
  --engine <name>   run only this engine (repeatable; default: all, see --list)
  --filter <text>   run only the cases whose name contains the text
  --suite <dir>     the folder of .vtt files (default: crates/wormhole-vt/tests/conformance)
  --markdown        print markdown tables
  --json            print JSON
  --no-lists        leave out the per-engine lists of failed cases
  --list            list the engines
  -h, --help
";

struct Options {
    engines: Vec<String>,
    filter: Option<String>,
    suite: PathBuf,
    markdown: bool,
    json: bool,
    lists: bool,
    list: bool,
}

fn parse_args() -> Result<Option<Options>, String> {
    let mut o = Options {
        engines: Vec::new(),
        filter: None,
        suite: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/wormhole-vt/tests/conformance"),
        markdown: false,
        json: false,
        lists: true,
        list: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |what: &str| args.next().ok_or_else(|| format!("{arg} needs {what}"));
        match arg.as_str() {
            "--engine" => o.engines.push(value("a name")?),
            "--filter" => o.filter = Some(value("some text")?),
            "--suite" => o.suite = PathBuf::from(value("a folder")?),
            "--markdown" => o.markdown = true,
            "--json" => o.json = true,
            "--no-lists" => o.lists = false,
            "--list" => o.list = true,
            "-h" | "--help" => return Ok(None),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(Some(o))
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("vt-matrix: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if options.list {
        for name in engine_names() {
            println!("{name}: {}", engine_info(name));
        }
        for (name, why) in unavailable() {
            println!("{name}: not built, {why}");
        }
        return ExitCode::SUCCESS;
    }
    for name in &options.engines {
        if !engine_names().contains(&name.as_str()) {
            eprintln!("vt-matrix: no engine called '{name}' (engines: {})", engine_names().join(" "));
            return ExitCode::from(2);
        }
    }
    let mut cases = match load_cases(&options.suite) {
        Ok(cases) => cases,
        Err(errors) => {
            eprintln!("vt-matrix: the conformance files do not parse:\n{errors}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(filter) = &options.filter {
        cases.retain(|c| c.name.contains(filter.as_str()));
    }

    // A panicking engine is a failed case, reported; its message would only be noise on stderr.
    quiet_panics();
    let mut runner = Runner::new(&cases);
    if options.engines.is_empty() {
        for_each_engine(&mut runner);
    } else {
        for name in &options.engines {
            visit_engine(name, &mut runner);
        }
    }
    let matrix: Matrix = runner.finish();
    let info: Vec<String> = matrix.engines.iter().map(|n| engine_info(n)).collect();
    if options.json {
        match serde_json::to_string_pretty(&render::json(&matrix, &info)) {
            Ok(text) => println!("{text}"),
            Err(e) => {
                eprintln!("vt-matrix: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let style = Style {
            format: if options.markdown { Format::Markdown } else { Format::Text },
            detail: if options.lists { Detail::WithFailureLists } else { Detail::Tables },
        };
        print!("{}", render::report(&matrix, &info, &unavailable(), style));
    }
    ExitCode::SUCCESS
}
